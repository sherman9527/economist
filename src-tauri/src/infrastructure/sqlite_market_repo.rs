use crate::domain::types::Fen;
use crate::infrastructure::economist_bridge::EconomistReportRow;
use crate::infrastructure::market_eastmoney::ClistEnrichmentMeta;
use crate::infrastructure::market_tencent::RawTencentQuote;
use crate::infrastructure::plugin_store::PluginBundle;
use crate::infrastructure::sqlite_repo::{SqliteRepository, StoredRecommendation, TelemetryRow};
use rusqlite::params;
use std::collections::HashMap;

impl SqliteRepository {
    pub fn upsert_recommendations(&self, recs: &[StoredRecommendation]) -> Result<(), String> {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for rec in recs {
            tx.execute(
                "INSERT OR REPLACE INTO recommendations (
                    rec_id, symbol, name_zh, board, bar_time, entry_price_fen,
                    stop_loss_price_fen, target_price_fen, conviction_score,
                    decision, rationale_zh, pitfalls_json, factor_z_json,
                    agent_session_id, created_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    rec.rec_id,
                    rec.symbol,
                    rec.name_zh,
                    rec.board,
                    rec.bar_time,
                    rec.entry_price_fen,
                    rec.stop_loss_price_fen,
                    rec.target_price_fen,
                    rec.conviction_score,
                    rec.decision,
                    rec.rationale_zh,
                    rec.pitfalls_json,
                    rec.factor_z_json,
                    rec.agent_session_id,
                    rec.created_at,
                ],
            )
            .map_err(|e| format!("保存推荐信号失败: {}", e))?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn list_recommendations_for_symbol(
        &self,
        symbol: &str,
    ) -> Result<Vec<StoredRecommendation>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT rec_id, symbol, name_zh, board, bar_time, entry_price_fen,
                        stop_loss_price_fen, target_price_fen, conviction_score,
                        decision, rationale_zh, pitfalls_json, factor_z_json,
                        agent_session_id, created_at
                 FROM recommendations WHERE symbol = ?1 AND decision = 'BUY'
                 ORDER BY bar_time ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([symbol], |r| {
                Ok(StoredRecommendation {
                    rec_id: r.get(0)?,
                    symbol: r.get(1)?,
                    name_zh: r.get(2)?,
                    board: r.get(3)?,
                    bar_time: r.get(4)?,
                    entry_price_fen: r.get(5)?,
                    stop_loss_price_fen: r.get(6)?,
                    target_price_fen: r.get(7)?,
                    conviction_score: r.get(8)?,
                    decision: r.get(9)?,
                    rationale_zh: r.get(10)?,
                    pitfalls_json: r.get(11)?,
                    factor_z_json: r.get(12)?,
                    agent_session_id: r.get(13)?,
                    created_at: r.get(14)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn sync_plugins_to_sqlite(&self, bundle: &PluginBundle) -> Result<(), String> {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let now_iso = chrono::Utc::now().to_rfc3339();
        for f in &bundle.factors.factors {
            let w = bundle
                .strategy
                .factor_weights
                .get(&f.factor_id)
                .copied()
                .unwrap_or(f.weight_pct);
            let q_json = serde_json::to_string(&f.quantile_returns_pct)
                .unwrap_or_else(|_| "[]".to_string());
            let d_json = serde_json::to_string(&f.ic_decay).unwrap_or_else(|_| "[]".to_string());
            tx.execute(
                "INSERT OR REPLACE INTO factor_plugins (
                    factor_id, name_zh, expr, bar_freq, weight_pct, rank_ic_mean,
                    ic_ir, oos_win_rate_pct, evidence_tag, status, quantile_json,
                    ic_decay_json, manifest_rel_path, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    f.factor_id,
                    f.name_zh,
                    f.expr,
                    f.bar_freq,
                    w,
                    f.rank_ic_mean,
                    f.ic_ir,
                    f.oos_win_rate_pct,
                    f.evidence_tag,
                    f.status,
                    q_json,
                    d_json,
                    &bundle.app_cfg.app.factors_manifest_rel,
                    now_iso,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "INSERT INTO strategy_iterations (
                strategy_id, version, manifest_rel_path, config_json, promoted_by_role, created_at
             ) VALUES (?1, ?2, ?3, ?4, 'quant_researcher', ?5)",
            params![
                bundle.strategy.strategy_id,
                bundle.strategy.version,
                bundle.app_cfg.app.strategy_manifest_rel,
                bundle.raw_strategy_json,
                now_iso,
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn cache_universe_snapshot(
        &self,
        quotes: &[RawTencentQuote],
        meta_map: &HashMap<String, ClistEnrichmentMeta>,
    ) -> Result<(), String> {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let now_iso = chrono::Utc::now().to_rfc3339();
        let mut stmt = tx
            .prepare(
                "INSERT INTO stock_universe_cache (
                    symbol, name_zh, exchange, board, industry_sw1, listing_days,
                    latest_price_fen, pct_chg, turnover_pct, total_mv_yi, circ_mv_yi,
                    main_net_inflow_wan, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
                 ON CONFLICT(symbol) DO UPDATE SET
                    name_zh = excluded.name_zh,
                    exchange = excluded.exchange,
                    board = excluded.board,
                    industry_sw1 = CASE
                        WHEN excluded.industry_sw1 != '' THEN excluded.industry_sw1
                        ELSE stock_universe_cache.industry_sw1
                    END,
                    listing_days = CASE
                        WHEN excluded.industry_sw1 != '' THEN excluded.listing_days
                        ELSE stock_universe_cache.listing_days
                    END,
                    latest_price_fen = excluded.latest_price_fen,
                    pct_chg = excluded.pct_chg,
                    turnover_pct = excluded.turnover_pct,
                    total_mv_yi = excluded.total_mv_yi,
                    circ_mv_yi = excluded.circ_mv_yi,
                    main_net_inflow_wan = CASE
                        WHEN excluded.industry_sw1 != '' THEN excluded.main_net_inflow_wan
                        WHEN substr(stock_universe_cache.updated_at, 1, 10) = substr(excluded.updated_at, 1, 10)
                            THEN stock_universe_cache.main_net_inflow_wan
                        ELSE 0.0
                    END,
                    updated_at = excluded.updated_at",
            )
            .map_err(|e| e.to_string())?;
        for q in quotes.iter().take(6000) {
            let ex = match q.symbol.exchange {
                crate::domain::types::ExchangeCode::Sh => "SH",
                crate::domain::types::ExchangeCode::Sz => "SZ",
                crate::domain::types::ExchangeCode::Bj => "BJ",
            };
            let bd = match q.symbol.board {
                crate::domain::types::BoardKind::Main => "MAIN",
                crate::domain::types::BoardKind::Chinext => "CHINEXT",
                crate::domain::types::BoardKind::Star => "STAR",
                crate::domain::types::BoardKind::Bse => "BSE",
            };
            let pct = if q.prev_close_yuan > 0.0 {
                ((q.latest_yuan - q.prev_close_yuan) / q.prev_close_yuan) * 100.0
            } else {
                0.0
            };
            let meta = meta_map.get(&q.symbol.digits);
            stmt.execute(params![
                q.symbol.canonical(),
                q.name_zh,
                ex,
                bd,
                meta.map(|m| m.industry_sw1.as_str()).unwrap_or(""),
                meta.map(|m| m.listing_days).unwrap_or(365),
                Fen::from_yuan(q.latest_yuan).0,
                pct,
                q.turnover_pct,
                q.total_mv_yi,
                q.circ_mv_yi,
                meta.map(|m| m.main_net_inflow_wan).unwrap_or(0.0),
                now_iso,
            ])
            .map_err(|e| e.to_string())?;
        }
        drop(stmt);
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn upsert_economist_index(&self, reports: &[EconomistReportRow]) -> Result<(), String> {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let now_iso = chrono::Utc::now().to_rfc3339();
        let mut stmt = tx
            .prepare(
                "INSERT OR REPLACE INTO economist_news_index (
                    report_id, report_date, title_zh, rel_path, article_count,
                    core_count, acked, acked_at, us_score, cn_score,
                    executive_summary_zh, chains_json, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            )
            .map_err(|e| e.to_string())?;
        for r in reports {
            let chains_json = serde_json::to_string(
                &r.chains
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "chainTitle": c.chain_title,
                            "economistTrigger": c.economist_trigger,
                            "usMapping": c.us_mapping,
                            "cnMapping": c.cn_mapping,
                            "divergenceInsight": c.divergence_insight,
                            "relatedCnSymbols": c.related_cn_symbols,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".to_string());
            stmt.execute(params![
                r.report_id,
                r.date,
                r.title_zh,
                r.rel_path,
                r.article_count as i64,
                r.core_count as i64,
                if r.acked { 1 } else { 0 },
                r.acked_at,
                r.us_score,
                r.cn_score,
                r.executive_summary_zh,
                chains_json,
                now_iso,
            ])
            .map_err(|e| e.to_string())?;
        }
        drop(stmt);
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn insert_telemetry(&self, row: &TelemetryRow) -> Result<i64, String> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO telemetry_events (
                ts_iso, epoch_secs, severity, subsystem, event_name, duration_ms, ok, message
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                row.ts_iso,
                row.epoch_secs,
                row.severity,
                row.subsystem,
                row.event_name,
                row.duration_ms.map(|d| d as i64),
                if row.ok { 1 } else { 0 },
                row.message,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_recent_telemetry(&self, limit: usize) -> Result<Vec<TelemetryRow>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT event_id, ts_iso, epoch_secs, severity, subsystem,
                        event_name, duration_ms, ok, message
                 FROM telemetry_events ORDER BY epoch_secs DESC, event_id DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit as i64], |r| {
                let dur: Option<i64> = r.get(6)?;
                let ok_int: i64 = r.get(7)?;
                Ok(TelemetryRow {
                    event_id: r.get(0)?,
                    ts_iso: r.get(1)?,
                    epoch_secs: r.get(2)?,
                    severity: r.get(3)?,
                    subsystem: r.get(4)?,
                    event_name: r.get(5)?,
                    duration_ms: dur.map(|d| d.max(0) as u64),
                    ok: ok_int != 0,
                    message: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn prune_telemetry_older_than(&self, cutoff_epoch_secs: i64) -> Result<usize, String> {
        let conn = self.connect()?;
        let deleted = conn
            .execute(
                "DELETE FROM telemetry_events WHERE epoch_secs < ?1",
                [cutoff_epoch_secs],
            )
            .map_err(|e| e.to_string())?;
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Symbol;
    use std::path::PathBuf;

    #[test]
    fn cache_universe_snapshot_preserves_industry_when_later_scan_lacks_meta() {
        let tmp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(".temp");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let db_path = tmp_dir.join(format!("test_univ_cache_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&db_path);

        let repo = SqliteRepository::new(&db_path).expect("init repo");
        let quote = RawTencentQuote {
            symbol: Symbol::parse("600103.SH").unwrap(),
            name_zh: "青山纸业".to_string(),
            latest_yuan: 3.26,
            prev_close_yuan: 3.33,
            open_yuan: 3.30,
            high_yuan: 3.34,
            low_yuan: 3.24,
            volume_lots: 641_000.0,
            amount_wan: 21_000.0,
            turnover_pct: 2.84,
            circ_mv_yi: 73.6,
            total_mv_yi: 73.6,
            vol_ratio: 0.92,
            vwap_yuan: 3.28,
            is_suspended: false,
            raw_ts: "20261120140000".to_string(),
        };

        let mut first_meta = HashMap::new();
        first_meta.insert(
            "600103".to_string(),
            ClistEnrichmentMeta {
                industry_sw1: "造纸".to_string(),
                listing_days: 7475,
                main_net_inflow_wan: -10_287.3,
            },
        );
        repo.cache_universe_snapshot(std::slice::from_ref(&quote), &first_meta)
            .expect("first snapshot");

        // Second scan without meta (e.g., outside top-800 turnover) must NOT wipe industry_sw1 to ""
        let empty_meta = HashMap::new();
        repo.cache_universe_snapshot(std::slice::from_ref(&quote), &empty_meta)
            .expect("second snapshot");

        let conn = repo.connect().unwrap();
        let (ind, days, flow): (String, i64, f64) = conn
            .query_row(
                "SELECT industry_sw1, listing_days, main_net_inflow_wan FROM stock_universe_cache WHERE symbol = '600103.SH'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        drop(conn);
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));

        assert_eq!(ind, "造纸");
        assert_eq!(days, 7475);
        assert!((flow - (-10_287.3)).abs() < 1e-3);
    }
}

