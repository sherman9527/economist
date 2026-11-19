use economist_quant_terminal_lib::application::scan_pipeline::ScanExecutionOutput;
use economist_quant_terminal_lib::application::terminal_service::{
    CachedScanState, TerminalService,
};
use economist_quant_terminal_lib::domain::types::{Fen, Symbol};
use economist_quant_terminal_lib::domain::version_control::{
    StrategyVersionSnapshot, VersionParamsSnapshot,
};
use economist_quant_terminal_lib::domain::watchlist_tracker::evaluate_watchlist_entry;
use economist_quant_terminal_lib::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::interface::dto::{
    AddWatchlistInputDto, UniverseFunnelSummaryDto, UpdatePluginInputDto,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Instant;

#[tokio::test]
async fn r1_01_rust_evolution_dtos_serialize_exact_typescript_camelcase_contract() {
    let sandbox = PathBuf::from(".temp").join("test_r1_ipc_camelcase_contract");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    // Run one walk-forward check so evolutionLogs has at least 1 row
    let _ = svc
        .run_anti_overfit_iteration("CONTRACT_CHECK")
        .await
        .unwrap();
    let vc = svc
        .get_version_control_dashboard(Some("v1.0.0".to_string()), Some("v1.2.0".to_string()))
        .await
        .unwrap();

    let json = serde_json::to_value(&vc).unwrap();

    // 1. VersionSnapshotDto exact keys expected by src/types/evolution.ts
    let v0 = &json["versions"][0];
    for key in [
        "versionId",
        "parentVersionId",
        "sourceTag",
        "createdAt",
        "summaryZh",
        "isActive",
        "factorWeightsPct",
        "minIntradayProjectedTurnoverPct",
        "atrStopMult",
        "insiderSellVetoPct",
        "holderSurgeVetoPct",
        "mainOutflowVetoRatio",
        "oosSampleDays",
        "oosRankIcMean",
        "oosWinRatePct",
        "oosCumNetRetBps",
        "oosMaxDrawdownBps",
    ] {
        assert!(
            v0.get(key).is_some(),
            "VersionSnapshotDto missing required TypeScript key: {}",
            key
        );
    }

    // 2. VersionComparisonDto exact keys
    let cmp = &json["comparison"];
    for key in [
        "baseVersionId",
        "targetVersionId",
        "winRateLiftPct",
        "rankIcLift",
        "netRetLiftBps",
        "drawdownImprovementBps",
        "isUnderperforming",
        "headlineVerdictZh",
        "factorAttributions",
        "rootCauses",
    ] {
        assert!(
            cmp.get(key).is_some(),
            "VersionComparisonDto missing required TypeScript key: {}",
            key
        );
    }

    if let Some(fa0) = cmp["factorAttributions"].as_array().and_then(|a| a.first()) {
        for key in [
            "factorId",
            "baseWeightPct",
            "targetWeightPct",
            "weightDeltaPct",
            "factorOosSpreadBps",
            "alphaContributionBps",
        ] {
            assert!(
                fa0.get(key).is_some(),
                "FactorAttributionDto missing required TypeScript key: {}",
                key
            );
        }
    }

    // 3. EvolutionAuditLogDto exact keys & pboEstimate scale [0.0, 1.0]
    let log0 = &json["evolutionLogs"][0];
    for key in [
        "logId",
        "evaluatedAt",
        "triggerSource",
        "baseVersionId",
        "resultVersionId",
        "action",
        "pboEstimate",
        "deflatedIcIr",
        "oosRankIc",
        "oosWinRateLiftPct",
        "reasonZh",
    ] {
        assert!(
            log0.get(key).is_some(),
            "EvolutionAuditLogDto missing required TypeScript key: {}",
            key
        );
    }
    let pbo_est = log0["pboEstimate"].as_f64().unwrap();
    assert!(
        (0.0..=1.0).contains(&pbo_est),
        "pboEstimate must be normalized to [0, 1] for UI percentage formatting, got {}",
        pbo_est
    );
}

#[tokio::test]
async fn r1_02_next_patch_version_advances_past_prerelease_tags() {
    let sandbox = PathBuf::from(".temp").join("test_r1_prerelease_semver");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();
    let bundle = svc.plugins.load_all().unwrap();
    svc.repo
        .ensure_default_version_lineage(&bundle, "2026-11-19T10:00:00+08:00")
        .unwrap();

    // Save an experimental version v1.3.0-exp
    svc.repo
        .save_version_snapshot(&StrategyVersionSnapshot {
            version_id: "v1.3.0-exp".to_string(),
            parent_version_id: Some("v1.2.0".to_string()),
            source_tag: "EXPERIMENTAL".to_string(),
            created_at: "2026-11-19T10:05:00+08:00".to_string(),
            is_active: true,
            summary_zh: "实验分支".to_string(),
            factor_weights: bundle.strategy.factor_weights.clone(),
            params: VersionParamsSnapshot {
                min_intraday_projected_turnover_pct: 5.8,
                insider_sell_veto_pct: 0.5,
                holder_surge_veto_pct: 25.0,
                main_outflow_veto_ratio: -0.08,
                atr_stop_mult: 1.8,
            },
            train_rank_ic: 0.04,
            oos_rank_ic: 0.03,
            oos_ic_ir: 0.5,
            pbo_risk_pct: 20.0,
        })
        .unwrap();

    let updated = svc
        .update_plugin_config(UpdatePluginInputDto {
            min_intraday_projected_turnover_pct: 5.6,
            insider_sell_veto_pct: 0.5,
            holder_surge_veto_pct: 25.0,
            main_outflow_veto_ratio: -0.08,
            atr_stop_mult: 2.0,
            market_polarity: "cn-red-up".to_string(),
            factor_weights: bundle.strategy.factor_weights.clone(),
        })
        .await
        .unwrap();

    assert_eq!(
        updated.version, "1.3.1",
        "Manual save after v1.3.0-exp must advance to 1.3.1 rather than regressing to 1.2.1"
    );
}

#[tokio::test]
async fn r1_03_watchlist_readd_preserves_active_tracking_but_resets_settled_items() {
    let sandbox = PathBuf::from(".temp").join("test_r1_watchlist_reentry");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    let _ = svc
        .add_to_watchlist(AddWatchlistInputDto {
            symbol: "300624.SZ".to_string(),
            name_zh: "万兴科技".to_string(),
            board: "CHINEXT".to_string(),
            industry_sw1: "计算机".to_string(),
            entry_price_yuan: 50.00,
            stop_loss_price_yuan: 46.50,
            target_price_yuan: 56.00,
            conviction_score: 74.0,
            source_decision: "WATCH".to_string(),
            rationale_zh: "首次观望".to_string(),
            factor_z_scores: BTreeMap::new(),
        })
        .await
        .unwrap();

    // Simulate 2 trading days of tracking in SQLite
    let mut items = svc.repo.list_watchlist_items().unwrap();
    items[0].added_trade_date = "2026-11-10".to_string();
    evaluate_watchlist_entry(
        &mut items[0],
        "2026-11-11",
        Fen::from_yuan(51.50),
        Fen::from_yuan(52.00),
        Fen::from_yuan(49.80),
    );
    evaluate_watchlist_entry(
        &mut items[0],
        "2026-11-12",
        Fen::from_yuan(52.40),
        Fen::from_yuan(52.80),
        Fen::from_yuan(51.10),
    );
    svc.repo.upsert_watchlist_item(&items[0]).unwrap();

    // Re-adding while still TRACKING must be idempotent (must NOT wipe 2 days of tracking!)
    let wl_mid = svc
        .add_to_watchlist(AddWatchlistInputDto {
            symbol: "300624.SZ".to_string(),
            name_zh: "万兴科技".to_string(),
            board: "CHINEXT".to_string(),
            industry_sw1: "计算机".to_string(),
            entry_price_yuan: 53.00,
            stop_loss_price_yuan: 49.00,
            target_price_yuan: 60.00,
            conviction_score: 80.0,
            source_decision: "BUY".to_string(),
            rationale_zh: "重复点击不应覆盖追踪天数".to_string(),
            factor_z_scores: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(wl_mid.items[0].holding_trading_days, 2);
    assert!((wl_mid.items[0].entry_price_yuan - 50.00).abs() < 0.01);

    // Now settle it via HIT_TARGET on 2026-11-13
    let mut items2 = svc.repo.list_watchlist_items().unwrap();
    evaluate_watchlist_entry(
        &mut items2[0],
        "2026-11-13",
        Fen::from_yuan(56.50),
        Fen::from_yuan(56.80),
        Fen::from_yuan(52.20),
    );
    svc.repo.upsert_watchlist_item(&items2[0]).unwrap();

    // Switch version to v1.1.0 and re-add on a new signal at ¥62.40 -> MUST reset entry price & version!
    let _ = svc.switch_strategy_version("v1.1.0").await.unwrap();
    let wl_reset = svc
        .add_to_watchlist(AddWatchlistInputDto {
            symbol: "300624.SZ".to_string(),
            name_zh: "万兴科技".to_string(),
            board: "CHINEXT".to_string(),
            industry_sw1: "计算机".to_string(),
            entry_price_yuan: 62.40,
            stop_loss_price_yuan: 57.70,
            target_price_yuan: 72.50,
            conviction_score: 88.0,
            source_decision: "BUY".to_string(),
            rationale_zh: "结案后新一轮放量重入自选".to_string(),
            factor_z_scores: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(wl_reset.items[0].outcome, "TRACKING");
    assert_eq!(wl_reset.items[0].holding_trading_days, 0);
    assert!((wl_reset.items[0].entry_price_yuan - 62.40).abs() < 0.01);
    assert_eq!(wl_reset.items[0].version_id_at_entry, "v1.1.0");
}

#[tokio::test]
async fn r1_04_walk_forward_promotion_invalidates_stale_scan_cache() {
    let sandbox = PathBuf::from(".temp").join("test_r1_wf_cache_invalidation");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    // Construct 40 trading days across 10 small-cap stocks where F2/F6 strongly predict +5d returns
    let mut kline_cache_1d: HashMap<String, Vec<SimpleBar>> = HashMap::new();
    let mut raw_quotes = Vec::new();
    for s in 0..10usize {
        let sym_str = format!("002{:03}.SZ", 500 + s);
        let mut bars = Vec::new();
        let mut price = 12.0 + (s as f64) * 0.5;
        let daily_ret = -0.008 + (s as f64) * 0.004;
        for d in 0..42usize {
            let month = if d < 21 { "09" } else { "10" };
            let day = (d % 21) + 1;
            let open = price;
            let close = (open * (1.0 + daily_ret)).max(2.0);
            let high = close.max(open) * 1.012;
            let low = close.min(open) * 0.992;
            bars.push(SimpleBar {
                time: format!("2026-{}-{:02}", month, day),
                open,
                high,
                low,
                close,
                volume: 80_000.0 + (s as f64) * 20_000.0,
                amount_yuan: (80_000.0 + (s as f64) * 20_000.0) * close * 100.0,
                turnover_pct: 4.5 + (s as f64) * 0.65,
            });
            price = close;
        }
        raw_quotes.push(RawTencentQuote {
            symbol: Symbol::parse(&sym_str).unwrap(),
            name_zh: format!("测试标的{}", s + 1),
            latest_yuan: price,
            prev_close_yuan: price / (1.0 + daily_ret),
            open_yuan: price * 0.99,
            high_yuan: price * 1.01,
            low_yuan: price * 0.985,
            volume_lots: 120_000.0,
            amount_wan: 3_600.0,
            turnover_pct: 4.5 + (s as f64) * 0.65,
            circ_mv_yi: 45.0 + (s as f64) * 3.0,
            total_mv_yi: 52.0 + (s as f64) * 3.0,
            vol_ratio: 1.3 + (s as f64) * 0.08,
            vwap_yuan: price * 0.996,
            is_suspended: false,
            raw_ts: "20261119143000".to_string(),
        });
        kline_cache_1d.insert(sym_str, bars);
    }

    {
        let mut w = svc.scan_cache.write().await;
        *w = Some(CachedScanState {
            output: ScanExecutionOutput {
                funnel: UniverseFunnelSummaryDto {
                    total_active_count: 5576,
                    tradable_count: 5200,
                    under_200_yi_count: 4300,
                    consecutive_turnover_vol_count: 25,
                    pitfall_cleared_count: 12,
                    qualified_buy_count: 6,
                    provider_name: "r1-test".to_string(),
                    scan_rtt_ms: 10,
                    scanned_at_iso: "2026-11-19T14:30:00+08:00".to_string(),
                    market_session: "TRADING".to_string(),
                },
                candidates: Vec::new(),
                raw_quotes,
                clist_meta: HashMap::new(),
                kline_cache_1d,
            },
            captured_at: Instant::now(),
        });
    }

    let wf_dash = svc
        .run_anti_overfit_iteration("POST_CLOSE_1500")
        .await
        .unwrap();
    let wf_json = serde_json::to_value(&wf_dash).unwrap();
    let action_str = wf_json["evolutionLogs"][0]["action"]
        .as_str()
        .unwrap_or_default();
    assert!(
        action_str == "PROMOTED" || action_str == "REJECTED_OVERFIT_GUARD",
        "evolutionLogs[0].action must be populated with TypeScript enum, got {:?}",
        wf_json["evolutionLogs"][0]
    );
    if wf_dash.active_version_id != "v1.2.0" {
        assert!(
            svc.scan_cache.read().await.is_none(),
            "scan_cache must be invalidated immediately when walk-forward promotes a new version"
        );
    }
}
