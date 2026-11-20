use crate::domain::portfolio_ledger::{OrderRecord, PortfolioLedger, PositionRecord};
use crate::domain::types::{FeeSchedule, Fen, Symbol};
use crate::infrastructure::sqlite_schema::open_and_migrate_sqlite;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AgentSessionRecord {
    pub agent_role: String,
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub model: String,
    pub reasoning_effort: String,
    pub spec_rel_path: String,
    pub turn_count: i64,
    pub status: String,
    pub last_woken_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredRecommendation {
    pub rec_id: String,
    pub symbol: String,
    pub name_zh: String,
    pub board: String,
    pub bar_time: String,
    pub entry_price_fen: i64,
    pub stop_loss_price_fen: i64,
    pub target_price_fen: Option<i64>,
    pub conviction_score: f64,
    pub decision: String,
    pub rationale_zh: String,
    pub pitfalls_json: String,
    pub factor_z_json: String,
    pub agent_session_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct EquitySnapshotRow {
    pub snap_date: String,
    pub cash_fen: i64,
    pub market_value_fen: i64,
    pub total_equity_fen: i64,
    pub total_fees_paid_fen: i64,
    pub portfolio_nav: f64,
    pub csi1000_nav: f64,
    pub cni2000_nav: f64,
    pub excess_pct: f64,
}

#[derive(Debug, Clone)]
pub struct TelemetryRow {
    pub event_id: i64,
    pub ts_iso: String,
    pub epoch_secs: i64,
    pub severity: String,
    pub subsystem: String,
    pub event_name: String,
    pub duration_ms: Option<u64>,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SqliteRepository {
    db_path: PathBuf,
}

impl SqliteRepository {
    pub fn new(db_path: &Path) -> Result<Self, String> {
        let _conn = open_and_migrate_sqlite(db_path)?;
        Ok(Self {
            db_path: db_path.to_path_buf(),
        })
    }

    pub(crate) fn connect(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("连接SQLite失败: {}", e))?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| format!("设置SQLite超时失败: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA temp_store = MEMORY; PRAGMA foreign_keys = ON;")
            .map_err(|e| format!("设置SQLite PRAGMA失败: {}", e))?;
        Ok(conn)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn db_size_bytes(&self) -> u64 {
        std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    pub fn list_agent_sessions(&self) -> Result<Vec<AgentSessionRecord>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT agent_role, session_id, parent_session_id, model, reasoning_effort,
                        spec_rel_path, turn_count, status, last_woken_at
                 FROM agent_sessions ORDER BY agent_role ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(AgentSessionRecord {
                    agent_role: r.get(0)?,
                    session_id: r.get(1)?,
                    parent_session_id: r.get(2)?,
                    model: r.get(3)?,
                    reasoning_effort: r.get(4)?,
                    spec_rel_path: r.get(5)?,
                    turn_count: r.get(6)?,
                    status: r.get(7)?,
                    last_woken_at: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn update_agent_session_wake(
        &self,
        agent_role: &str,
        status: &str,
        woken_at_iso: &str,
        increment_turn: bool,
        last_error: Option<&str>,
    ) -> Result<AgentSessionRecord, String> {
        let conn = self.connect()?;
        let turn_inc: i64 = if increment_turn { 1 } else { 0 };
        let changed = conn
            .execute(
                "UPDATE agent_sessions
                 SET status = ?1,
                     last_woken_at = ?2,
                     last_completed_at = CASE WHEN ?1 = 'IDLE' THEN ?2 ELSE last_completed_at END,
                     turn_count = turn_count + ?3,
                     last_error = ?4,
                     updated_at = ?2
                 WHERE agent_role = ?5",
                params![status, woken_at_iso, turn_inc, last_error, agent_role],
            )
            .map_err(|e| format!("更新 agent_sessions 失败: {}", e))?;
        if changed == 0 {
            return Err(format!("未找到 Agent 角色会话: {}", agent_role));
        }
        self.list_agent_sessions()?
            .into_iter()
            .find(|s| s.agent_role == agent_role)
            .ok_or_else(|| format!("读取更新后的 Agent 会话失败: {}", agent_role))
    }

    pub fn load_ledger(
        &self,
        initial_capital_fen: Fen,
        current_trade_date: &str,
        fee_schedule: FeeSchedule,
    ) -> Result<PortfolioLedger, String> {
        let conn = self.connect()?;
        let mut orders_stmt = conn
            .prepare(
                "SELECT order_id, rec_id, symbol, name_zh, side, trigger_type, status,
                        lot_qty, entry_price_fen, fill_price_fen, stop_loss_price_fen,
                        commission_fen, stamp_duty_fen, slippage_fen, rationale_zh,
                        trade_date, t_plus1_unlock_date, created_at
                 FROM simulated_orders ORDER BY created_at ASC, rowid ASC",
            )
            .map_err(|e| e.to_string())?;
        let orders: Vec<OrderRecord> = orders_stmt
            .query_map([], |r| {
                let sym_str: String = r.get(2)?;
                let sym = Symbol::parse(&sym_str).unwrap_or(Symbol {
                    digits: "000001".to_string(),
                    exchange: crate::domain::types::ExchangeCode::Sz,
                    board: crate::domain::types::BoardKind::Main,
                });
                Ok(OrderRecord {
                    order_id: r.get(0)?,
                    rec_id: r.get(1)?,
                    symbol: sym,
                    name_zh: r.get(3)?,
                    side: r.get(4)?,
                    trigger_type: r.get(5)?,
                    status: r.get(6)?,
                    lot_qty: r.get(7)?,
                    entry_price_fen: Fen(r.get(8)?),
                    fill_price_fen: Fen(r.get(9)?),
                    stop_loss_price_fen: Fen(r.get(10)?),
                    commission_fen: Fen(r.get(11)?),
                    stamp_duty_fen: Fen(r.get(12)?),
                    slippage_fen: Fen(r.get(13)?),
                    rationale_zh: r.get(14)?,
                    trade_date: r.get(15)?,
                    t_plus1_unlock_date: r.get(16)?,
                    created_at_iso: r.get(17)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut pos_stmt = conn
            .prepare(
                "SELECT position_id, slot_no, symbol, name_zh, industry_sw1,
                        total_qty, sellable_qty, locked_t0_qty, avg_cost_fen,
                        mark_price_fen, highest_price_fen, stop_loss_price_fen,
                        entry_rationale_zh, opened_trade_date, last_buy_trade_date, opened_at
                 FROM positions ORDER BY slot_no ASC",
            )
            .map_err(|e| e.to_string())?;
        let mut positions = BTreeMap::new();
        let pos_rows = pos_stmt
            .query_map([], |r| {
                let sym_str: String = r.get(2)?;
                let sym = Symbol::parse(&sym_str).unwrap_or(Symbol {
                    digits: "000001".to_string(),
                    exchange: crate::domain::types::ExchangeCode::Sz,
                    board: crate::domain::types::BoardKind::Main,
                });
                Ok(PositionRecord {
                    position_id: r.get(0)?,
                    slot_no: r.get(1)?,
                    symbol: sym,
                    name_zh: r.get(3)?,
                    industry_sw1: r.get(4)?,
                    total_qty: r.get(5)?,
                    sellable_qty: r.get(6)?,
                    locked_t0_qty: r.get(7)?,
                    avg_cost_fen: Fen(r.get(8)?),
                    mark_price_fen: Fen(r.get(9)?),
                    highest_price_fen: Fen(r.get(10)?),
                    stop_loss_price_fen: Fen(r.get(11)?),
                    entry_rationale_zh: r.get(12)?,
                    opened_trade_date: r.get(13)?,
                    last_buy_trade_date: {
                        let d: String = r.get(14)?;
                        d
                    },
                    t_plus1_unlock_date: {
                        let d: String = r.get(14)?;
                        crate::domain::trading_calendar::next_ashare_trading_date_str(&d)
                    },
                    opened_at_iso: r.get(15)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in pos_rows {
            let p = row.map_err(|e| e.to_string())?;
            positions.insert(p.symbol.canonical(), p);
        }

        // Reconstruct exact cash & fees from orders or latest snapshot
        let latest_snap_cash: Option<(i64, i64)> = conn
            .query_row(
                "SELECT cash_fen, total_fees_paid_fen FROM portfolio_snapshots ORDER BY snap_date DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        let (cash_fen, total_fees_fen) = if orders.is_empty() && positions.is_empty() {
            (initial_capital_fen, Fen::ZERO)
        } else if let Some((c, f)) = latest_snap_cash {
            (Fen(c), Fen(f))
        } else {
            let mut cash = initial_capital_fen.0;
            let mut fees = 0_i64;
            for o in &orders {
                if o.status == "FILLED" {
                    let gross = o.fill_price_fen.0 * o.lot_qty;
                    fees += o.commission_fen.0 + o.stamp_duty_fen.0;
                    if o.side == "BUY" {
                        cash -= gross + o.commission_fen.0;
                    } else {
                        cash += gross - o.commission_fen.0 - o.stamp_duty_fen.0;
                    }
                }
            }
            (Fen(cash.max(0)), Fen(fees))
        };

        let mut ledger = PortfolioLedger {
            initial_capital_fen,
            cash_fen,
            total_fees_paid_fen: total_fees_fen,
            current_trade_date: current_trade_date.to_string(),
            fee_schedule,
            max_single_weight_pct: 10.0,
            limit_guard_pct: 0.25,
            positions,
            orders,
        };
        // Unlock T+1 positions only on valid A-share trading sessions on or after t_plus1_unlock_date
        if crate::domain::trading_calendar::is_ashare_trading_day_str(current_trade_date) {
            for pos in ledger.positions.values_mut() {
                if pos.locked_t0_qty > 0
                    && current_trade_date >= pos.t_plus1_unlock_date.as_str()
                {
                    pos.sellable_qty += pos.locked_t0_qty;
                    pos.locked_t0_qty = 0;
                }
            }
        }
        Ok(ledger)
    }

    pub fn save_ledger(&self, ledger: &PortfolioLedger) -> Result<(), String> {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        tx.execute("DELETE FROM positions", [])
            .map_err(|e| e.to_string())?;
        for pos in ledger.positions.values() {
            let board_str = match pos.symbol.board {
                crate::domain::types::BoardKind::Main => "MAIN",
                crate::domain::types::BoardKind::Chinext => "CHINEXT",
                crate::domain::types::BoardKind::Star => "STAR",
                crate::domain::types::BoardKind::Bse => "BSE",
            };
            tx.execute(
                "INSERT INTO positions (
                    position_id, slot_no, symbol, name_zh, board, industry_sw1,
                    total_qty, sellable_qty, locked_t0_qty, avg_cost_fen,
                    mark_price_fen, highest_price_fen, stop_loss_price_fen,
                    entry_rationale_zh, opened_trade_date, last_buy_trade_date, opened_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                params![
                    pos.position_id,
                    pos.slot_no,
                    pos.symbol.canonical(),
                    pos.name_zh,
                    board_str,
                    pos.industry_sw1,
                    pos.total_qty,
                    pos.sellable_qty,
                    pos.locked_t0_qty,
                    pos.avg_cost_fen.0,
                    pos.mark_price_fen.0,
                    pos.highest_price_fen.0,
                    pos.stop_loss_price_fen.0,
                    pos.entry_rationale_zh,
                    pos.opened_trade_date,
                    pos.last_buy_trade_date,
                    pos.opened_at_iso,
                ],
            )
            .map_err(|e| format!("保存持仓失败: {}", e))?;
        }

        for ord in &ledger.orders {
            tx.execute(
                "INSERT OR REPLACE INTO simulated_orders (
                    order_id, rec_id, symbol, name_zh, side, trigger_type, status,
                    lot_qty, entry_price_fen, fill_price_fen, stop_loss_price_fen,
                    commission_fen, stamp_duty_fen, slippage_fen, rationale_zh,
                    trade_date, t_plus1_unlock_date, created_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
                params![
                    ord.order_id,
                    ord.rec_id,
                    ord.symbol.canonical(),
                    ord.name_zh,
                    ord.side,
                    ord.trigger_type,
                    ord.status,
                    ord.lot_qty,
                    ord.entry_price_fen.0,
                    ord.fill_price_fen.0,
                    ord.stop_loss_price_fen.0,
                    ord.commission_fen.0,
                    ord.stamp_duty_fen.0,
                    ord.slippage_fen.0,
                    ord.rationale_zh,
                    ord.trade_date,
                    ord.t_plus1_unlock_date,
                    ord.created_at_iso,
                ],
            )
            .map_err(|e| format!("保存模拟订单失败: {}", e))?;
        }

        let nav = (ledger.total_equity_fen().0 as f64) / (ledger.initial_capital_fen.0.max(1) as f64);
        let (carry_csi, carry_cni): (f64, f64) = tx
            .query_row(
                "SELECT csi1000_nav, cni2000_nav
                 FROM portfolio_snapshots
                 WHERE snap_date <= ?1
                 ORDER BY snap_date DESC
                 LIMIT 1",
                params![ledger.current_trade_date],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((1.0, 1.0));
        let now_iso = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO portfolio_snapshots (
                snap_date, cash_fen, market_value_fen, total_equity_fen,
                total_fees_paid_fen, portfolio_nav, csi1000_nav, cni2000_nav, excess_pct, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(snap_date) DO UPDATE SET
                cash_fen = excluded.cash_fen,
                market_value_fen = excluded.market_value_fen,
                total_equity_fen = excluded.total_equity_fen,
                total_fees_paid_fen = excluded.total_fees_paid_fen,
                portfolio_nav = excluded.portfolio_nav,
                excess_pct = (excluded.portfolio_nav - portfolio_snapshots.csi1000_nav) * 100.0,
                updated_at = excluded.updated_at",
            params![
                ledger.current_trade_date,
                ledger.cash_fen.0,
                ledger.market_value_fen().0,
                ledger.total_equity_fen().0,
                ledger.total_fees_paid_fen.0,
                nav,
                carry_csi,
                carry_cni,
                (nav - carry_csi) * 100.0,
                now_iso,
            ],
        )
        .map_err(|e| format!("更新净值快照失败: {}", e))?;

        tx.commit().map_err(|e| e.to_string())
    }

    pub fn upsert_equity_snapshot(&self, snap: &EquitySnapshotRow) -> Result<(), String> {
        let conn = self.connect()?;
        let now_iso = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO portfolio_snapshots (
                snap_date, cash_fen, market_value_fen, total_equity_fen,
                total_fees_paid_fen, portfolio_nav, csi1000_nav, cni2000_nav, excess_pct, updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                snap.snap_date,
                snap.cash_fen,
                snap.market_value_fen,
                snap.total_equity_fen,
                snap.total_fees_paid_fen,
                snap.portfolio_nav,
                snap.csi1000_nav,
                snap.cni2000_nav,
                snap.excess_pct,
                now_iso,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_equity_snapshots(&self) -> Result<Vec<EquitySnapshotRow>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT snap_date, cash_fen, market_value_fen, total_equity_fen,
                        total_fees_paid_fen, portfolio_nav, csi1000_nav, cni2000_nav, excess_pct
                 FROM portfolio_snapshots ORDER BY snap_date ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(EquitySnapshotRow {
                    snap_date: r.get(0)?,
                    cash_fen: r.get(1)?,
                    market_value_fen: r.get(2)?,
                    total_equity_fen: r.get(3)?,
                    total_fees_paid_fen: r.get(4)?,
                    portfolio_nav: r.get(5)?,
                    csi1000_nav: r.get(6)?,
                    cni2000_nav: r.get(7)?,
                    excess_pct: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::portfolio_ledger::BuyOrderCommand;
    use crate::domain::types::Symbol;

    #[test]
    fn roundtrips_portfolio_ledger_and_prunes_28d_telemetry() {
        let dir = PathBuf::from(".temp").join("test_sqlite_repo");
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("repo_test.db");
        let _ = std::fs::remove_file(&db);

        let repo = SqliteRepository::new(&db).unwrap();
        let mut ledger = repo
            .load_ledger(
                Fen::INITIAL_CAPITAL_100W,
                "2026-11-18",
                FeeSchedule::default(),
            )
            .unwrap();
        assert_eq!(ledger.total_equity_fen(), Fen::INITIAL_CAPITAL_100W);

        ledger
            .execute_buy(BuyOrderCommand {
                order_id: "ORD-TEST-1".to_string(),
                rec_id: Some("REC-1".to_string()),
                symbol: Symbol::parse("300308.SZ").unwrap(),
                name_zh: "测试光联".to_string(),
                industry_sw1: "通信".to_string(),
                quote_price_fen: Fen::from_yuan(20.0),
                prev_close_fen: Fen::from_yuan(19.5),
                is_st: false,
                shares: 1000,
                stop_loss_price_fen: Fen::from_yuan(18.9),
                rationale_zh: "双日高换手放量突破".to_string(),
                trade_date: "2026-11-18".to_string(),
                next_trade_date: "2026-11-19".to_string(),
                created_at_iso: "2026-11-18T10:00:00+08:00".to_string(),
            })
            .unwrap();
        repo.save_ledger(&ledger).unwrap();

        let reloaded = repo
            .load_ledger(
                Fen::INITIAL_CAPITAL_100W,
                "2026-11-18",
                FeeSchedule::default(),
            )
            .unwrap();
        assert_eq!(reloaded.open_positions_count(), 1);
        assert_eq!(reloaded.orders.len(), 1);

        // Insert expired (40 days ago) and fresh telemetry events
        repo.insert_telemetry(&TelemetryRow {
            event_id: 0,
            ts_iso: "2026-10-01T00:00:00Z".to_string(),
            epoch_secs: 1_000,
            severity: "INFO".to_string(),
            subsystem: "MARKET".to_string(),
            event_name: "old_scan".to_string(),
            duration_ms: Some(120),
            ok: true,
            message: "expired".to_string(),
        })
        .unwrap();
        repo.insert_telemetry(&TelemetryRow {
            event_id: 0,
            ts_iso: "2026-11-18T00:00:00Z".to_string(),
            epoch_secs: 5_000_000,
            severity: "INFO".to_string(),
            subsystem: "MARKET".to_string(),
            event_name: "fresh_scan".to_string(),
            duration_ms: Some(980),
            ok: true,
            message: "fresh".to_string(),
        })
        .unwrap();

        let pruned = repo.prune_telemetry_older_than(2_000_000).unwrap();
        assert_eq!(pruned, 1);
        let remaining = repo.list_recent_telemetry(10).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].event_name, "fresh_scan");
    }
}
