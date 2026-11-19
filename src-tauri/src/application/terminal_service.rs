use crate::application::dto_mapper::{
    fee_schedule_from_strategy, map_agent_session_to_dto, map_economist_snapshot_to_dto,
    map_plugin_bundle_to_dto, map_portfolio_overview_dto, map_telemetry_dashboard_dto,
};
use crate::application::portfolio_seeder::current_cst_dates;
use crate::application::scan_pipeline::{execute_live_market_scan, ScanExecutionOutput};
use crate::domain::strategy_gate::{detect_market_session_cst, elapsed_trading_minutes_cst};
use crate::domain::types::Fen;
use crate::infrastructure::agent_supervisor::AgentSupervisor;
use crate::infrastructure::economist_bridge::EconomistBridge;
use crate::infrastructure::plugin_store::{PluginBundle, PluginStore};
use crate::infrastructure::portable_paths::PortablePaths;
use crate::infrastructure::sqlite_repo::{SqliteRepository, StoredRecommendation};
use crate::infrastructure::telemetry_cleaner::TelemetryCleaner;
use crate::interface::dto::{
    AgentSessionDto, EconomistModuleDto, MarketScanResponseDto, RotationAlertEventDto,
    ScreenerCandidateDto, StrategyPluginConfigDto, TelemetryDashboardDto, TerminalBootstrapDto,
    UpdatePluginInputDto,
};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

pub struct CachedScanState {
    pub output: ScanExecutionOutput,
    pub captured_at: Instant,
}

pub struct TerminalService {
    pub paths: PortablePaths,
    pub repo: SqliteRepository,
    pub plugins: PluginStore,
    pub economist: EconomistBridge,
    pub cleaner: TelemetryCleaner,
    pub supervisor: AgentSupervisor,
    pub http: reqwest::Client,
    pub scan_cache: RwLock<Option<CachedScanState>>,
    pub scan_singleflight: Mutex<()>,
    pub ledger_lock: Mutex<()>,
    pub evolution_lock: Mutex<()>,
}

impl TerminalService {
    pub fn initialize(paths: PortablePaths) -> Result<Self, String> {
        let plugins = PluginStore::new(paths.clone());
        let bundle = plugins.load_all()?;
        let repo = SqliteRepository::new(&paths.sqlite_db_path())?;
        repo.sync_plugins_to_sqlite(&bundle)?;

        let economist = EconomistBridge::new(paths.clone());
        let cleaner = TelemetryCleaner::new(
            paths.clone(),
            bundle.app_cfg.storage.retention_days,
            bundle.app_cfg.storage.build_soft_cap_bytes,
            bundle.app_cfg.storage.build_cap_bytes,
        );
        let supervisor = AgentSupervisor::new(
            paths.clone(),
            bundle.app_cfg.agents.cli_binary.clone(),
        );
        let http = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) EconomistQuantTerminal/1.0")
            .timeout(Duration::from_millis(
                bundle.app_cfg.market_sources.request_timeout_ms.max(2500),
            ))
            .build()
            .map_err(|e| format!("初始化HTTP客户端失败: {}", e))?;

        let _ = cleaner.record_event(
            &repo,
            "INFO",
            "UI",
            "terminal_initialized",
            Some(1),
            true,
            "量化指挥台与经济学人跨市场引擎便携式底座就绪",
        );

        Ok(Self {
            paths,
            repo,
            plugins,
            economist,
            cleaner,
            supervisor,
            http,
            scan_cache: RwLock::new(None),
            scan_singleflight: Mutex::new(()),
            ledger_lock: Mutex::new(()),
            evolution_lock: Mutex::new(()),
        })
    }

    async fn run_or_reuse_scan(
        &self,
        bundle: &PluginBundle,
        bearish_sectors: &[String],
        held_symbols: &[String],
        force_refresh: bool,
    ) -> Result<ScanExecutionOutput, String> {
        if !force_refresh {
            let guard = self.scan_cache.read().await;
            if let Some(cached) = guard.as_ref() {
                if cached.captured_at.elapsed() < Duration::from_secs(25) {
                    return Ok(cached.output.clone());
                }
            }
        }

        let _singleflight = self.scan_singleflight.lock().await;
        if !force_refresh {
            let guard = self.scan_cache.read().await;
            if let Some(cached) = guard.as_ref() {
                if cached.captured_at.elapsed() < Duration::from_secs(25) {
                    return Ok(cached.output.clone());
                }
            }
        }

        let quant_sess_id = self
            .repo
            .list_agent_sessions()
            .ok()
            .and_then(|list| {
                list.into_iter()
                    .find(|s| s.agent_role == "quant_researcher")
                    .map(|s| s.session_id)
            })
            .unwrap_or_else(|| "88ace910-c427-41db-a87f-46d8ae39735b".to_string());

        let (session_state, elapsed_min) = {
            use chrono::{Datelike, Timelike};
            let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            let now = chrono::Utc::now().with_timezone(&cst);
            let wd = now.weekday().number_from_monday() as u8;
            let h = now.hour() as u8;
            let m = now.minute() as u8;
            (
                detect_market_session_cst(wd, h, m),
                elapsed_trading_minutes_cst(h, m) as u32,
            )
        };

        let scan = execute_live_market_scan(
            &self.http,
            bundle,
            bearish_sectors,
            held_symbols,
            session_state,
            elapsed_min,
            &quant_sess_id,
        )
        .await?;
        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "MARKET",
            "ashare_universe_scan",
            Some(scan.funnel.scan_rtt_ms),
            true,
            &format!(
                "全市场扫描 {} 只活跃A股 -> <200亿 {} 只 -> 双日换手>5%放量 {} 只 -> 排雷绿灯 {} 只 (信号 {} 只)",
                scan.funnel.total_active_count,
                scan.funnel.under_200_yi_count,
                scan.funnel.consecutive_turnover_vol_count,
                scan.funnel.pitfall_cleared_count,
                scan.funnel.qualified_buy_count
            ),
        );

        {
            let repo_bg = self.repo.clone();
            let quotes_bg = scan.raw_quotes.clone();
            let meta_bg = scan.clist_meta.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = repo_bg.cache_universe_snapshot(&quotes_bg, &meta_bg);
            })
            .await;
        }

        let now_iso = chrono::Utc::now().to_rfc3339();
        let buy_recs: Vec<StoredRecommendation> = scan
            .candidates
            .iter()
            .filter(|c| c.decision == "BUY")
            .take(25)
            .map(|c| StoredRecommendation {
                rec_id: format!("REC-{}-{}", c.symbol.replace('.', ""), c.kline_bar_date),
                symbol: c.symbol.clone(),
                name_zh: c.name_zh.clone(),
                board: c.board.clone(),
                bar_time: c.kline_bar_date.clone(),
                entry_price_fen: Fen::from_yuan(c.latest_price_yuan).0,
                stop_loss_price_fen: Fen::from_yuan(c.suggested_stop_loss_yuan).0,
                target_price_fen: Some(Fen::from_yuan(c.suggested_target_yuan).0),
                conviction_score: c.conviction_score,
                decision: "BUY".to_string(),
                rationale_zh: c.rationale_zh.clone(),
                pitfalls_json: serde_json::to_string(&c.pitfalls)
                    .unwrap_or_else(|_| "[]".to_string()),
                factor_z_json: serde_json::to_string(&c.factor_z_scores)
                    .unwrap_or_else(|_| "{}".to_string()),
                agent_session_id: quant_sess_id.clone(),
                created_at: now_iso.clone(),
            })
            .collect();
        if !buy_recs.is_empty() {
            let _ = self.repo.upsert_recommendations(&buy_recs);
        }

        {
            let mut w = self.scan_cache.write().await;
            *w = Some(CachedScanState {
                output: scan.clone(),
                captured_at: Instant::now(),
            });
        }
        Ok(scan)
    }

    pub async fn bootstrap_terminal(&self) -> Result<TerminalBootstrapDto, String> {
        let bundle = self.plugins.load_all()?;
        let econ_snap = self.economist.load_and_sync(&self.repo)?;
        let (today_cst, next_cst, now_iso) = current_cst_dates();
        let fee_sched = fee_schedule_from_strategy(&bundle.strategy);
        let init_cap = Fen::from_yuan(bundle.strategy.initial_capital_yuan as f64);

        let held_syms: Vec<String> = {
            let _guard = self.ledger_lock.lock().await;
            let pre_ledger = self.repo.load_ledger(init_cap, &today_cst, fee_sched)?;
            pre_ledger.positions.keys().cloned().collect()
        };

        let scan = self
            .run_or_reuse_scan(
                &bundle,
                &econ_snap.bearish_risk_sectors_zh,
                &held_syms,
                false,
            )
            .await?;

        let ledger = self
            .sync_and_seed_ledger_locked(&scan, &bundle, &today_cst, &next_cst, &now_iso)
            .await?;

        let featured_symbol = scan
            .candidates
            .iter()
            .find(|c| c.decision == "BUY")
            .or_else(|| scan.candidates.first())
            .map(|c| c.symbol.clone())
            .or_else(|| ledger.positions.keys().next().cloned())
            .unwrap_or_else(|| "300308.SZ".to_string());

        let featured_kline = self.get_stock_kline(&featured_symbol, "1d").await?;
        let snaps = self.repo.list_equity_snapshots()?;
        let portfolio = map_portfolio_overview_dto(&ledger, &snaps);
        let plugin_config = map_plugin_bundle_to_dto(&bundle);
        let economist = map_economist_snapshot_to_dto(econ_snap);
        let telemetry = self.run_storage_hygiene().await?;
        let watchlist = self.sync_watchlist_with_scan(&scan, &today_cst)?;
        let version_control = self.get_version_control_dashboard(None, None).await?;

        Ok(TerminalBootstrapDto {
            funnel: scan.funnel,
            candidates: scan.candidates,
            portfolio,
            featured_kline,
            plugin_config,
            economist,
            telemetry,
            watchlist,
            version_control,
        })
    }

    pub async fn run_market_scan(
        &self,
        force_refresh: bool,
    ) -> Result<MarketScanResponseDto, String> {
        let bundle = self.plugins.load_all()?;
        let econ_snap = self.economist.load_and_sync(&self.repo)?;
        let (today_cst, next_cst, now_iso) = current_cst_dates();
        let fee_sched = fee_schedule_from_strategy(&bundle.strategy);
        let init_cap = Fen::from_yuan(bundle.strategy.initial_capital_yuan as f64);
        let held_syms: Vec<String> = {
            let _guard = self.ledger_lock.lock().await;
            let pre_ledger = self.repo.load_ledger(init_cap, &today_cst, fee_sched)?;
            pre_ledger.positions.keys().cloned().collect()
        };

        let scan = self
            .run_or_reuse_scan(
                &bundle,
                &econ_snap.bearish_risk_sectors_zh,
                &held_syms,
                force_refresh,
            )
            .await?;
        let _ = self
            .sync_and_seed_ledger_locked(&scan, &bundle, &today_cst, &next_cst, &now_iso)
            .await?;
        let watchlist = self.sync_watchlist_with_scan(&scan, &today_cst)?;

        let top_picks: Vec<ScreenerCandidateDto> = scan
            .candidates
            .iter()
            .filter(|c| c.decision == "BUY")
            .take(5)
            .cloned()
            .collect();

        let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let slot_ts = chrono::Utc::now()
            .with_timezone(&cst)
            .format("%Y-%m-%d %H:%M CST")
            .to_string();

        let alert = Some(RotationAlertEventDto {
            slot_timestamp_cst: slot_ts,
            qualified_count: scan.funnel.qualified_buy_count,
            top_picks,
        });

        Ok(MarketScanResponseDto {
            funnel: scan.funnel,
            candidates: scan.candidates,
            alert,
            watchlist,
        })
    }

    pub async fn update_plugin_config(
        &self,
        input: UpdatePluginInputDto,
    ) -> Result<StrategyPluginConfigDto, String> {
        let _evo_guard = self.evolution_lock.lock().await;
        let mut updated = self.plugins.update_strategy_and_weights(
            input.min_intraday_projected_turnover_pct,
            input.insider_sell_veto_pct,
            input.holder_surge_veto_pct,
            input.main_outflow_veto_ratio,
            input.atr_stop_mult,
            &input.market_polarity,
            &input.factor_weights,
        )?;
        let new_ver = self.record_manual_version_snapshot_sync(&updated)?;
        updated.strategy.version = new_ver.trim_start_matches('v').to_string();
        self.repo.sync_plugins_to_sqlite(&updated)?;
        {
            let mut w = self.scan_cache.write().await;
            *w = None;
        }
        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "QUANT",
            "plugin_manifest_hot_reloaded",
            Some(3),
            true,
            &format!(
                "热重载策略与因子插件 {} (预测换手阈值 {:.1}%, ATR倍数 {:.2})",
                updated.app_cfg.app.strategy_manifest_rel,
                updated.strategy.min_intraday_projected_turnover_pct,
                updated.strategy.stop_loss.atr_stop_mult
            ),
        );
        Ok(map_plugin_bundle_to_dto(&updated))
    }

    pub async fn ack_economist_report(
        &self,
        report_id: &str,
        acked: bool,
    ) -> Result<EconomistModuleDto, String> {
        let snap = self.economist.set_report_ack(&self.repo, report_id, acked)?;
        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "NEWS",
            "report_ack_updated",
            Some(1),
            true,
            &format!("研报 {} 已标记为 acked={}", report_id, acked),
        );
        Ok(map_economist_snapshot_to_dto(snap))
    }

    pub async fn get_economist_report_html(&self, rel_path: &str) -> Result<String, String> {
        self.economist.read_report_html(rel_path)
    }

    pub async fn run_storage_hygiene(&self) -> Result<TelemetryDashboardDto, String> {
        let cleaner_bg = self.cleaner.clone();
        let repo_bg = self.repo.clone();
        let (hygiene, sessions, events) = tokio::task::spawn_blocking(move || {
            let h = cleaner_bg.run_hygiene(&repo_bg)?;
            let s = repo_bg.list_agent_sessions()?;
            let e = repo_bg.list_recent_telemetry(60)?;
            Ok::<_, String>((h, s, e))
        })
        .await
        .map_err(|e| format!("存储巡检后台任务异常: {}", e))??;
        Ok(map_telemetry_dashboard_dto(sessions, hygiene, events))
    }

    pub async fn wake_agent_session(
        &self,
        agent_role: &str,
        prompt: &str,
    ) -> Result<AgentSessionDto, String> {
        let updated = self
            .supervisor
            .wake_session(&self.repo, agent_role, prompt)
            .await?;
        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "AGENT",
            "agent_session_woken",
            Some(5),
            true,
            &format!(
                "唤醒持久化会话 [{}] session={} (model={} effort={})",
                updated.agent_role, updated.session_id, updated.model, updated.reasoning_effort
            ),
        );
        Ok(map_agent_session_to_dto(updated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[tokio::test]
    async fn initializes_isolated_service_and_blocks_report_path_traversal() {
        let sandbox = PathBuf::from(".temp").join("test_terminal_service_init");
        let _ = std::fs::remove_dir_all(&sandbox);
        let paths = PortablePaths::from_root(sandbox).unwrap();
        let svc = TerminalService::initialize(paths).unwrap();
        assert!(svc
            .get_economist_report_html("../config/app.toml")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn simulated_buy_enforces_t_plus_1_lock_and_persists_in_100w_ledger() {
        use crate::domain::types::Symbol;
        use crate::infrastructure::market_tencent::RawTencentQuote;
        use crate::interface::dto::{PlaceOrderInputDto, UniverseFunnelSummaryDto};
        use std::collections::HashMap;

        let sandbox = PathBuf::from(".temp").join("test_terminal_service_buy_t1");
        let _ = std::fs::remove_dir_all(&sandbox);
        let paths = PortablePaths::from_root(sandbox).unwrap();
        let svc = TerminalService::initialize(paths).unwrap();

        {
            let mut w = svc.scan_cache.write().await;
            *w = Some(CachedScanState {
                output: ScanExecutionOutput {
                    funnel: UniverseFunnelSummaryDto {
                        total_active_count: 5576,
                        tradable_count: 5210,
                        under_200_yi_count: 4120,
                        consecutive_turnover_vol_count: 42,
                        pitfall_cleared_count: 28,
                        qualified_buy_count: 12,
                        provider_name: "unit-test-cache".to_string(),
                        scan_rtt_ms: 15,
                        scanned_at_iso: "2026-11-18T10:00:00+08:00".to_string(),
                        market_session: "TRADING".to_string(),
                    },
                    candidates: Vec::new(),
                    raw_quotes: vec![RawTencentQuote {
                        symbol: Symbol::parse("002463.SZ").unwrap(),
                        name_zh: "精密电子".to_string(),
                        latest_yuan: 28.50,
                        prev_close_yuan: 27.60,
                        open_yuan: 27.80,
                        high_yuan: 28.90,
                        low_yuan: 27.55,
                        volume_lots: 185_000.0,
                        amount_wan: 52_600.0,
                        turnover_pct: 8.4,
                        circ_mv_yi: 96.0,
                        total_mv_yi: 108.0,
                        vol_ratio: 1.65,
                        vwap_yuan: 28.35,
                        is_suspended: false,
                        raw_ts: "20261118100000".to_string(),
                    }],
                    clist_meta: HashMap::new(),
                    kline_cache_1d: HashMap::new(),
                },
                captured_at: Instant::now(),
            });
        }

        let overview = svc
            .place_simulated_buy(PlaceOrderInputDto {
                symbol: "002463.SZ".to_string(),
                lot_qty: Some(1000),
                rationale_zh: Some("小市值双日高换手突破MA20+排雷绿灯".to_string()),
                stop_loss_price_yuan: Some(26.80),
            })
            .await
            .unwrap();

        assert_eq!(overview.open_slots_count, 1);
        assert_eq!(overview.max_slots, 10);
        assert!((overview.initial_capital_yuan - 1_000_000.0).abs() < 0.01);
        assert_eq!(overview.positions[0].locked_t0_qty, 1000);
        assert_eq!(overview.positions[0].sellable_qty, 0);
        assert!(overview.positions[0].stop_loss_price_yuan < overview.positions[0].avg_cost_yuan);
        assert!(
            ((overview.cash_yuan + overview.market_value_yuan) - overview.total_equity_yuan).abs()
                < 0.05
        );

        // Same-day T+0 close must be rejected by strict A-share T+1 rule
        let sell_err = svc
            .close_simulated_position("002463.SZ", "尝试当日T+0平仓")
            .await
            .unwrap_err();
        assert!(sell_err.contains("T+1"));
    }

    #[tokio::test]
    async fn hot_reloads_plugin_weights_and_wakes_locked_agent_session() {
        let sandbox = PathBuf::from(".temp").join("test_terminal_service_plugin_agent");
        let _ = std::fs::remove_dir_all(&sandbox);
        let paths = PortablePaths::from_root(sandbox).unwrap();
        let svc = TerminalService::initialize(paths).unwrap();

        let mut weights = BTreeMap::new();
        weights.insert("F1_ideal_reversal".to_string(), 30.0);
        weights.insert("F2_concave_abn_turnover".to_string(), 30.0);
        weights.insert("F3_amihud_compression".to_string(), 10.0);
        weights.insert("F4_intraday_apm_vwap".to_string(), 10.0);
        weights.insert("F5_main_inflow_ir".to_string(), 10.0);
        weights.insert("F6_pv_efficiency".to_string(), 10.0);

        let updated = svc
            .update_plugin_config(UpdatePluginInputDto {
                min_intraday_projected_turnover_pct: 5.8,
                insider_sell_veto_pct: 0.45,
                holder_surge_veto_pct: 22.0,
                main_outflow_veto_ratio: -0.07,
                atr_stop_mult: 2.0,
                market_polarity: "intl-green-up".to_string(),
                factor_weights: weights,
            })
            .await
            .unwrap();

        assert_eq!(updated.max_total_mv_yi, 200.0);
        assert_eq!(updated.min_turnover_pct, 5.0);
        assert_eq!(updated.consecutive_days, 2);
        assert_eq!(updated.market_polarity, "intl-green-up");

        let woken = svc
            .wake_agent_session("quant_researcher", "校验新因子权重下IC_IR稳健性")
            .await
            .unwrap();
        assert_eq!(woken.model, "gemini-4-carbon");
        assert_eq!(woken.reasoning_effort, "high");
        assert!(woken.turn_count >= 1);
    }
}
