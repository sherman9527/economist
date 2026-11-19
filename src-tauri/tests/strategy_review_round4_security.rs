use economist_quant_terminal_lib::application::terminal_service::TerminalService;
use economist_quant_terminal_lib::domain::version_control::{
    StrategyVersionSnapshot, VersionParamsSnapshot,
};
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::interface::dto::{round_2, UpdatePluginInputDto};
use economist_quant_terminal_lib::interface::evolution_dto::AddWatchlistInputDto;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn r4_01_watchlist_rejects_extreme_prices_nan_zscores_oversized_strings_and_round2_overflow() {
    let sandbox = PathBuf::from(".temp").join("test_r4_watchlist_bounds");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    // 1. round_2(1e308) must never overflow to +Infinity (which serde_json serializes as null)
    let r = round_2(1.0e308);
    assert!(
        r.is_finite(),
        "round_2(1e308) must stay finite instead of overflowing to +Infinity"
    );

    // 2. Extreme entry_price_yuan (1.6e308) previously panicked / wrapped i64::MAX + 1 in WatchlistItemState::new_tracking
    let huge_price = AddWatchlistInputDto {
        symbol: "300308.SZ".to_string(),
        name_zh: "中际旭创".to_string(),
        board: "CHINEXT".to_string(),
        industry_sw1: "通信".to_string(),
        entry_price_yuan: 1.6e308,
        stop_loss_price_yuan: 100.0,
        target_price_yuan: 200.0,
        conviction_score: 80.0,
        source_decision: "WATCH_OBSERVE".to_string(),
        rationale_zh: "极端价格溢出测试".to_string(),
        factor_z_scores: BTreeMap::new(),
    };
    assert!(
        svc.add_to_watchlist(huge_price).await.is_err(),
        "add_to_watchlist must reject absurdly large entry_price_yuan before Fen overflow"
    );

    // 3. Sub-penny price (0.01 yuan) previously clamped both entry and stop to Fen(1), violating stop < entry
    let sub_penny = AddWatchlistInputDto {
        symbol: "300308.SZ".to_string(),
        name_zh: "中际旭创".to_string(),
        board: "CHINEXT".to_string(),
        industry_sw1: "通信".to_string(),
        entry_price_yuan: 0.01,
        stop_loss_price_yuan: 0.009,
        target_price_yuan: 0.012,
        conviction_score: 80.0,
        source_decision: "WATCH_OBSERVE".to_string(),
        rationale_zh: "亚分位价格测试".to_string(),
        factor_z_scores: BTreeMap::new(),
    };
    assert!(
        svc.add_to_watchlist(sub_penny).await.is_err(),
        "add_to_watchlist must reject sub-2-fen entry price where stop < entry cannot hold"
    );

    // 4. Extreme conviction_score (1e308) or NaN/Inf factor_z_scores must be rejected or sanitized
    let mut poisoned_z = BTreeMap::new();
    poisoned_z.insert("F1_ideal_reversal".to_string(), f64::NAN);
    let bad_z = AddWatchlistInputDto {
        symbol: "300308.SZ".to_string(),
        name_zh: "中际旭创".to_string(),
        board: "CHINEXT".to_string(),
        industry_sw1: "通信".to_string(),
        entry_price_yuan: 120.0,
        stop_loss_price_yuan: 110.0,
        target_price_yuan: 140.0,
        conviction_score: 1.0e308,
        source_decision: "WATCH_OBSERVE".to_string(),
        rationale_zh: "NaN因子与超界信念分测试".to_string(),
        factor_z_scores: poisoned_z,
    };
    assert!(
        svc.add_to_watchlist(bad_z).await.is_err(),
        "add_to_watchlist must reject out-of-range conviction_score or non-finite factor_z_scores"
    );

    // 5. Oversized rationale (> 1024 chars) must be rejected to prevent SQLite/UI DoS
    let huge_text = AddWatchlistInputDto {
        symbol: "300308.SZ".to_string(),
        name_zh: "中际旭创".to_string(),
        board: "CHINEXT".to_string(),
        industry_sw1: "通信".to_string(),
        entry_price_yuan: 120.0,
        stop_loss_price_yuan: 110.0,
        target_price_yuan: 140.0,
        conviction_score: 82.5,
        source_decision: "WATCH_OBSERVE".to_string(),
        rationale_zh: "A".repeat(4096),
        factor_z_scores: BTreeMap::new(),
    };
    assert!(
        svc.add_to_watchlist(huge_text).await.is_err(),
        "add_to_watchlist must reject oversized string payloads"
    );
}

#[tokio::test]
async fn r4_02_partial_factor_weights_update_never_corrupts_total_sum_above_100_pct() {
    let sandbox = PathBuf::from(".temp").join("test_r4_partial_weights");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    // Submit only ONE factor in factor_weights! Previously F1 became 100.0 while F2..F6 stayed ~80.0 (sum ~180%!)
    let mut partial = BTreeMap::new();
    partial.insert("F1_ideal_reversal".to_string(), 50.0);

    let res = svc
        .update_plugin_config(UpdatePluginInputDto {
            min_intraday_projected_turnover_pct: 5.5,
            insider_sell_veto_pct: 1.0,
            holder_surge_veto_pct: 15.0,
            main_outflow_veto_ratio: -0.08,
            atr_stop_mult: 2.0,
            market_polarity: "cn-red-up".to_string(),
            factor_weights: partial,
        })
        .await;

    // Either partial factor maps are rejected outright OR normalized across ALL 6 known factors to exact 100.0%
    if let Ok(cfg) = res {
        let total_manifest_w: f64 = cfg.factors.iter().map(|f| f.weight_pct).sum();
        assert!(
            (total_manifest_w - 100.0).abs() <= 0.15,
            "Total factor weights after partial update must sum to 100.0%, got {:.2}%",
            total_manifest_w
        );
    }

    let reloaded = svc.plugins.load_all().unwrap();
    let persisted_sum: f64 = reloaded.strategy.factor_weights.values().sum();
    assert!(
        (persisted_sum - 100.0).abs() <= 0.15,
        "Persisted strategy.factor_weights must sum to 100.0%, got {:.2}%",
        persisted_sum
    );
}

#[tokio::test]
async fn r4_03_switch_strategy_version_prevents_split_brain_and_serializes_concurrent_updates() {
    let sandbox = PathBuf::from(".temp").join("test_r4_switch_atomicity");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = Arc::new(TerminalService::initialize(paths).unwrap());

    let base_bundle = svc.plugins.load_all().unwrap();
    svc.repo
        .ensure_default_version_lineage(&base_bundle, "2026-11-19T08:00:00+08:00")
        .unwrap();
    let before_active = svc
        .repo
        .list_version_snapshots()
        .unwrap()
        .into_iter()
        .find(|v| v.is_active)
        .unwrap()
        .version_id;

    // Insert a corrupted historical snapshot containing an unknown factor key (is_active = false)
    let mut bad_weights = BTreeMap::new();
    bad_weights.insert("UNKNOWN_ROGUE_FACTOR".to_string(), 100.0);
    svc.repo
        .save_version_snapshot(&StrategyVersionSnapshot {
            version_id: "v9.9.9-corrupt".to_string(),
            parent_version_id: Some(before_active.clone()),
            source_tag: "MANUAL_SAVE".to_string(),
            created_at: "2026-11-19T09:00:00+08:00".to_string(),
            is_active: false,
            summary_zh: "损坏快照测试".to_string(),
            factor_weights: bad_weights,
            params: VersionParamsSnapshot {
                min_intraday_projected_turnover_pct: 5.5,
                insider_sell_veto_pct: 1.0,
                holder_surge_veto_pct: 15.0,
                main_outflow_veto_ratio: -0.08,
                atr_stop_mult: 2.0,
            },
            train_rank_ic: 0.0,
            oos_rank_ic: 0.0,
            oos_ic_ir: 0.0,
            pbo_risk_pct: 50.0,
        })
        .unwrap();

    // Switching to v9.9.9-corrupt MUST fail AND MUST NOT leave SQLite pointing to v9.9.9-corrupt!
    let switch_res = svc.switch_strategy_version("v9.9.9-corrupt").await;
    assert!(
        switch_res.is_err(),
        "Switching to snapshot with unknown factor must fail"
    );
    let after_active = svc
        .repo
        .list_version_snapshots()
        .unwrap()
        .into_iter()
        .find(|v| v.is_active)
        .unwrap()
        .version_id;
    assert_eq!(
        after_active, before_active,
        "Split-brain detected: SQLite marked v9.9.9-corrupt active even though manifest validation failed!"
    );

    // Concurrent manual updates must serialize cleanly via evolution_lock and create 2 distinct version snapshots!
    let count_before = svc.repo.list_version_snapshots().unwrap().len();
    let s1 = svc.clone();
    let s2 = svc.clone();
    let w_full = base_bundle.strategy.factor_weights.clone();
    let w_full2 = w_full.clone();
    let (r1, r2) = tokio::join!(
        s1.update_plugin_config(UpdatePluginInputDto {
            min_intraday_projected_turnover_pct: 5.8,
            insider_sell_veto_pct: 1.0,
            holder_surge_veto_pct: 15.0,
            main_outflow_veto_ratio: -0.08,
            atr_stop_mult: 2.05,
            market_polarity: "cn-red-up".to_string(),
            factor_weights: w_full,
        }),
        s2.update_plugin_config(UpdatePluginInputDto {
            min_intraday_projected_turnover_pct: 6.1,
            insider_sell_veto_pct: 1.0,
            holder_surge_veto_pct: 15.0,
            main_outflow_veto_ratio: -0.08,
            atr_stop_mult: 2.15,
            market_polarity: "cn-red-up".to_string(),
            factor_weights: w_full2,
        })
    );
    assert!(r1.is_ok() && r2.is_ok());
    let count_after = svc.repo.list_version_snapshots().unwrap().len();
    assert_eq!(
        count_after,
        count_before + 2,
        "Concurrent update_plugin_config calls must serialize and record 2 distinct version snapshots without clobbering"
    );
}
