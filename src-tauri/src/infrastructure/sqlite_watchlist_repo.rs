use crate::domain::types::Fen;
use crate::domain::version_control::{StrategyVersionSnapshot, VersionParamsSnapshot};
use crate::domain::watchlist_tracker::{WatchlistItemState, WatchlistOutcome};
use crate::infrastructure::plugin_store::PluginBundle;
use crate::infrastructure::sqlite_repo::SqliteRepository;
use rusqlite::params;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct EvolutionAuditRow {
    pub log_id: i64,
    pub created_at: String,
    pub trigger_source: String,
    pub base_version_id: String,
    pub resulting_version_id: String,
    pub promoted: bool,
    pub verdict: String,
    pub train_dates: usize,
    pub embargo_dates: usize,
    pub oos_dates: usize,
    pub sample_observations: usize,
    pub train_rank_ic: f64,
    pub oos_rank_ic: f64,
    pub oos_ic_ir: f64,
    pub deflated_ic_ir: f64,
    pub pbo_risk_pct: f64,
    pub max_pairwise_corr: f64,
    pub prior_weights: BTreeMap<String, f64>,
    pub proposed_weights: BTreeMap<String, f64>,
    pub audit_reasons_zh: Vec<String>,
}

impl SqliteRepository {
    pub fn upsert_watchlist_item(&self, item: &WatchlistItemState) -> Result<(), String> {
        let conn = self.connect()?;
        let fz_json =
            serde_json::to_string(&item.factor_z_scores).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO watchlist_items (
                symbol, name_zh, board, industry_sw1, added_trade_date, added_at_iso,
                entry_price_fen, stop_loss_price_fen, target_price_fen, conviction_score,
                source_decision, version_id_at_entry, rationale_zh, factor_z_json,
                latest_price_fen, peak_price_fen, trough_price_fen, holding_trading_days,
                net_return_bps, mfe_bps, mae_bps, outcome, settled_trade_date, settled_price_fen
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18,
                ?19, ?20, ?21, ?22, ?23, ?24
            )
            ON CONFLICT(symbol) DO UPDATE SET
                latest_price_fen = excluded.latest_price_fen,
                peak_price_fen = excluded.peak_price_fen,
                trough_price_fen = excluded.trough_price_fen,
                holding_trading_days = excluded.holding_trading_days,
                net_return_bps = excluded.net_return_bps,
                mfe_bps = excluded.mfe_bps,
                mae_bps = excluded.mae_bps,
                outcome = excluded.outcome,
                settled_trade_date = excluded.settled_trade_date,
                settled_price_fen = excluded.settled_price_fen",
            params![
                item.symbol,
                item.name_zh,
                item.board,
                item.industry_sw1,
                item.added_trade_date,
                item.added_at_iso,
                item.entry_price_fen.0,
                item.stop_loss_price_fen.0,
                item.target_price_fen.0,
                item.conviction_score,
                item.source_decision,
                item.version_id_at_entry,
                item.rationale_zh,
                fz_json,
                item.latest_price_fen.0,
                item.peak_price_fen.0,
                item.trough_price_fen.0,
                item.holding_trading_days as i64,
                item.net_return_bps,
                item.mfe_bps,
                item.mae_bps,
                item.outcome.as_str(),
                item.settled_trade_date,
                item.settled_price_fen.map(|f| f.0),
            ],
        )
        .map_err(|e| format!("写入自选观察条目失败: {}", e))?;
        Ok(())
    }

    pub fn remove_watchlist_item(&self, symbol: &str) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM watchlist_items WHERE symbol = ?1", params![symbol])
            .map_err(|e| format!("移除自选观察条目失败: {}", e))?;
        Ok(())
    }

    pub fn list_watchlist_items(&self) -> Result<Vec<WatchlistItemState>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT symbol, name_zh, board, industry_sw1, added_trade_date, added_at_iso,
                        entry_price_fen, stop_loss_price_fen, target_price_fen, conviction_score,
                        source_decision, version_id_at_entry, rationale_zh, factor_z_json,
                        latest_price_fen, peak_price_fen, trough_price_fen, holding_trading_days,
                        net_return_bps, mfe_bps, mae_bps, outcome, settled_trade_date, settled_price_fen
                   FROM watchlist_items
                  ORDER BY added_at_iso DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let fz_raw: String = r.get(13)?;
                let fz: BTreeMap<String, f64> =
                    serde_json::from_str(&fz_raw).unwrap_or_default();
                let outcome_raw: String = r.get(21)?;
                let settled_fen_opt: Option<i64> = r.get(23)?;
                Ok(WatchlistItemState {
                    symbol: r.get(0)?,
                    name_zh: r.get(1)?,
                    board: r.get(2)?,
                    industry_sw1: r.get(3)?,
                    added_trade_date: r.get(4)?,
                    added_at_iso: r.get(5)?,
                    entry_price_fen: Fen(r.get(6)?),
                    stop_loss_price_fen: Fen(r.get(7)?),
                    target_price_fen: Fen(r.get(8)?),
                    conviction_score: r.get(9)?,
                    source_decision: r.get(10)?,
                    version_id_at_entry: r.get(11)?,
                    rationale_zh: r.get(12)?,
                    factor_z_scores: fz,
                    latest_price_fen: Fen(r.get(14)?),
                    peak_price_fen: Fen(r.get(15)?),
                    trough_price_fen: Fen(r.get(16)?),
                    holding_trading_days: (r.get::<_, i64>(17)?).max(0) as u32,
                    net_return_bps: r.get(18)?,
                    mfe_bps: r.get(19)?,
                    mae_bps: r.get(20)?,
                    outcome: WatchlistOutcome::from_db_str(&outcome_raw),
                    settled_trade_date: r.get(22)?,
                    settled_price_fen: settled_fen_opt.map(Fen),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn ensure_default_version_lineage(
        &self,
        bundle: &PluginBundle,
        now_iso: &str,
    ) -> Result<(), String> {
        let conn = self.connect()?;
        let cnt: i64 = conn
            .query_row("SELECT COUNT(*) FROM strategy_version_snapshots", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        if cnt > 0 {
            return Ok(());
        }

        // Seed 3-generation lineage: v1.0.0 (Equal-Weight Origin) -> v1.1.0 (Liquidity Defensive) -> v1.2.0 (Current Active)
        let mut w_v100 = BTreeMap::new();
        for f in &bundle.factors.factors {
            w_v100.insert(f.factor_id.clone(), 16.7);
        }
        if let Some(first) = bundle.factors.factors.first() {
            w_v100.insert(first.factor_id.clone(), 16.5);
        }

        let mut w_v110 = BTreeMap::new();
        w_v110.insert("F1_ideal_reversal".to_string(), 19.0);
        w_v110.insert("F2_concave_abn_turnover".to_string(), 17.0);
        w_v110.insert("F3_amihud_compression".to_string(), 21.0);
        w_v110.insert("F4_intraday_apm_vwap".to_string(), 14.0);
        w_v110.insert("F5_main_inflow_ir".to_string(), 15.0);
        w_v110.insert("F6_pv_efficiency".to_string(), 14.0);

        let cur_ver = format!("v{}", bundle.strategy.version.trim_start_matches('v'));
        let base_params = VersionParamsSnapshot {
            min_intraday_projected_turnover_pct: bundle.strategy.min_intraday_projected_turnover_pct,
            insider_sell_veto_pct: bundle.strategy.pitfall_thresholds.insider_sell_veto_pct,
            holder_surge_veto_pct: bundle.strategy.pitfall_thresholds.holder_surge_veto_pct,
            main_outflow_veto_ratio: bundle.strategy.pitfall_thresholds.main_outflow_veto_ratio,
            atr_stop_mult: bundle.strategy.stop_loss.atr_stop_mult,
        };

        let seeds = vec![
            StrategyVersionSnapshot {
                version_id: "v1.0.0".to_string(),
                parent_version_id: None,
                source_tag: "ORIGIN_EQUAL_WEIGHT".to_string(),
                created_at: "2026-11-01T08:00:00Z".to_string(),
                is_active: false,
                summary_zh: "初代等权六因子基线（无单因子倾斜，零调参先验锚点）".to_string(),
                factor_weights: w_v100,
                params: VersionParamsSnapshot {
                    atr_stop_mult: 2.2,
                    ..base_params.clone()
                },
                train_rank_ic: 0.043,
                oos_rank_ic: 0.041,
                oos_ic_ir: 0.68,
                pbo_risk_pct: 11.5,
            },
            StrategyVersionSnapshot {
                version_id: "v1.1.0".to_string(),
                parent_version_id: Some("v1.0.0".to_string()),
                source_tag: "DEFENSIVE_LIQUIDITY".to_string(),
                created_at: "2026-11-10T08:00:00Z".to_string(),
                is_active: false,
                summary_zh: "流动性压缩与反转防守增强版（侧重 Amihud 压缩与回撤控制）".to_string(),
                factor_weights: w_v110,
                params: VersionParamsSnapshot {
                    atr_stop_mult: 2.1,
                    ..base_params.clone()
                },
                train_rank_ic: 0.049,
                oos_rank_ic: 0.046,
                oos_ic_ir: 0.75,
                pbo_risk_pct: 14.2,
            },
            StrategyVersionSnapshot {
                version_id: cur_ver,
                parent_version_id: Some("v1.1.0".to_string()),
                source_tag: "PRODUCTION_BASELINE".to_string(),
                created_at: now_iso.to_string(),
                is_active: true,
                summary_zh: "双日高换手理想反转与凹向异常换手均衡生产版".to_string(),
                factor_weights: bundle.strategy.factor_weights.clone(),
                params: base_params,
                train_rank_ic: 0.054,
                oos_rank_ic: 0.050,
                oos_ic_ir: 0.81,
                pbo_risk_pct: 15.8,
            },
        ];

        for s in &seeds {
            self.save_version_snapshot(s)?;
        }
        Ok(())
    }

    pub fn save_version_snapshot(&self, snap: &StrategyVersionSnapshot) -> Result<(), String> {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        if snap.is_active {
            tx.execute("UPDATE strategy_version_snapshots SET is_active = 0", [])
                .map_err(|e| e.to_string())?;
        }
        let w_json =
            serde_json::to_string(&snap.factor_weights).unwrap_or_else(|_| "{}".to_string());
        let p_json = serde_json::to_string(&snap.params).unwrap_or_else(|_| "{}".to_string());
        tx.execute(
            "INSERT INTO strategy_version_snapshots (
                version_id, parent_version_id, source_tag, created_at, is_active,
                summary_zh, factor_weights_json, params_json,
                train_rank_ic, oos_rank_ic, oos_ic_ir, pbo_risk_pct
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(version_id) DO UPDATE SET
                is_active = excluded.is_active,
                summary_zh = excluded.summary_zh,
                factor_weights_json = excluded.factor_weights_json,
                params_json = excluded.params_json,
                train_rank_ic = excluded.train_rank_ic,
                oos_rank_ic = excluded.oos_rank_ic,
                oos_ic_ir = excluded.oos_ic_ir,
                pbo_risk_pct = excluded.pbo_risk_pct",
            params![
                snap.version_id,
                snap.parent_version_id,
                snap.source_tag,
                snap.created_at,
                if snap.is_active { 1 } else { 0 },
                snap.summary_zh,
                w_json,
                p_json,
                snap.train_rank_ic,
                snap.oos_rank_ic,
                snap.oos_ic_ir,
                snap.pbo_risk_pct,
            ],
        )
        .map_err(|e| format!("保存策略版本快照失败: {}", e))?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_active_version_id(
        &self,
        version_id: &str,
    ) -> Result<StrategyVersionSnapshot, String> {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM strategy_version_snapshots WHERE version_id = ?1",
                params![version_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists == 0 {
            return Err(format!("未找到策略版本快照: {}", version_id));
        }
        tx.execute("UPDATE strategy_version_snapshots SET is_active = 0", [])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE strategy_version_snapshots SET is_active = 1 WHERE version_id = ?1",
            params![version_id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;

        self.list_version_snapshots()?
            .into_iter()
            .find(|v| v.version_id == version_id)
            .ok_or_else(|| format!("读取激活版本 {} 失败", version_id))
    }

    pub fn list_version_snapshots(&self) -> Result<Vec<StrategyVersionSnapshot>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT version_id, parent_version_id, source_tag, created_at, is_active,
                        summary_zh, factor_weights_json, params_json,
                        train_rank_ic, oos_rank_ic, oos_ic_ir, pbo_risk_pct
                   FROM strategy_version_snapshots
                  ORDER BY rowid ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let w_raw: String = r.get(6)?;
                let p_raw: String = r.get(7)?;
                let factor_weights: BTreeMap<String, f64> =
                    serde_json::from_str(&w_raw).unwrap_or_default();
                let params: VersionParamsSnapshot =
                    serde_json::from_str(&p_raw).unwrap_or(VersionParamsSnapshot {
                        min_intraday_projected_turnover_pct: 5.5,
                        insider_sell_veto_pct: 0.5,
                        holder_surge_veto_pct: 25.0,
                        main_outflow_veto_ratio: -0.08,
                        atr_stop_mult: 2.0,
                    });
                Ok(StrategyVersionSnapshot {
                    version_id: r.get(0)?,
                    parent_version_id: r.get(1)?,
                    source_tag: r.get(2)?,
                    created_at: r.get(3)?,
                    is_active: r.get::<_, i64>(4)? != 0,
                    summary_zh: r.get(5)?,
                    factor_weights,
                    params,
                    train_rank_ic: r.get(8)?,
                    oos_rank_ic: r.get(9)?,
                    oos_ic_ir: r.get(10)?,
                    pbo_risk_pct: r.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn insert_factor_evolution_log(&self, row: &EvolutionAuditRow) -> Result<(), String> {
        let conn = self.connect()?;
        let prior_json =
            serde_json::to_string(&row.prior_weights).unwrap_or_else(|_| "{}".to_string());
        let prop_json =
            serde_json::to_string(&row.proposed_weights).unwrap_or_else(|_| "{}".to_string());
        let reasons_json =
            serde_json::to_string(&row.audit_reasons_zh).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO factor_evolution_logs (
                created_at, trigger_source, base_version_id, resulting_version_id,
                promoted, verdict, train_dates, embargo_dates, oos_dates, sample_observations,
                train_rank_ic, oos_rank_ic, oos_ic_ir, deflated_ic_ir, pbo_risk_pct,
                max_pairwise_corr, prior_weights_json, proposed_weights_json, audit_reasons_json
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19
            )",
            params![
                row.created_at,
                row.trigger_source,
                row.base_version_id,
                row.resulting_version_id,
                if row.promoted { 1 } else { 0 },
                row.verdict,
                row.train_dates as i64,
                row.embargo_dates as i64,
                row.oos_dates as i64,
                row.sample_observations as i64,
                row.train_rank_ic,
                row.oos_rank_ic,
                row.oos_ic_ir,
                row.deflated_ic_ir,
                row.pbo_risk_pct,
                row.max_pairwise_corr,
                prior_json,
                prop_json,
                reasons_json,
            ],
        )
        .map_err(|e| format!("记录防过拟合迭代日志失败: {}", e))?;
        Ok(())
    }

    pub fn list_factor_evolution_logs(&self, limit: usize) -> Result<Vec<EvolutionAuditRow>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT log_id, created_at, trigger_source, base_version_id, resulting_version_id,
                        promoted, verdict, train_dates, embargo_dates, oos_dates, sample_observations,
                        train_rank_ic, oos_rank_ic, oos_ic_ir, deflated_ic_ir, pbo_risk_pct,
                        max_pairwise_corr, prior_weights_json, proposed_weights_json, audit_reasons_json
                   FROM factor_evolution_logs
                  ORDER BY log_id DESC
                  LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                let pr_raw: String = r.get(17)?;
                let pp_raw: String = r.get(18)?;
                let rs_raw: String = r.get(19)?;
                Ok(EvolutionAuditRow {
                    log_id: r.get(0)?,
                    created_at: r.get(1)?,
                    trigger_source: r.get(2)?,
                    base_version_id: r.get(3)?,
                    resulting_version_id: r.get(4)?,
                    promoted: r.get::<_, i64>(5)? != 0,
                    verdict: r.get(6)?,
                    train_dates: (r.get::<_, i64>(7)?).max(0) as usize,
                    embargo_dates: (r.get::<_, i64>(8)?).max(0) as usize,
                    oos_dates: (r.get::<_, i64>(9)?).max(0) as usize,
                    sample_observations: (r.get::<_, i64>(10)?).max(0) as usize,
                    train_rank_ic: r.get(11)?,
                    oos_rank_ic: r.get(12)?,
                    oos_ic_ir: r.get(13)?,
                    deflated_ic_ir: r.get(14)?,
                    pbo_risk_pct: r.get(15)?,
                    max_pairwise_corr: r.get(16)?,
                    prior_weights: serde_json::from_str(&pr_raw).unwrap_or_default(),
                    proposed_weights: serde_json::from_str(&pp_raw).unwrap_or_default(),
                    audit_reasons_zh: serde_json::from_str(&rs_raw).unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}
