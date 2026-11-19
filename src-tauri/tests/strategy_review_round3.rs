use economist_quant_terminal_lib::application::pit_observation_builder::build_pit_walk_forward_observations;
use economist_quant_terminal_lib::application::scan_pipeline::ScanExecutionOutput;
use economist_quant_terminal_lib::application::terminal_service::{
    CachedScanState, TerminalService,
};
use economist_quant_terminal_lib::domain::types::Symbol;
use economist_quant_terminal_lib::domain::version_control::{
    compare_versions_and_attribute, StrategyVersionSnapshot, VersionParamsSnapshot,
};
use economist_quant_terminal_lib::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};
use economist_quant_terminal_lib::infrastructure::plugin_store::PluginStore;
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::interface::dto::{
    UniverseFunnelSummaryDto, UpdatePluginInputDto,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn r3_01_shadow_arena_respects_per_version_turnover_gate_and_atr_tail_stop_physics() {
    let sandbox = PathBuf::from(".temp").join("test_r3_shadow_turnover_and_atr");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let store = PluginStore::new(paths);
    let bundle = store.load_all().unwrap();

    let mut kline_cache_1d: HashMap<String, Vec<SimpleBar>> = HashMap::new();
    let mut raw_quotes = Vec::new();

    // 4 steady-turnover stocks (turnover = 6.2%) rally +5.5% over T+1..T+5;
    // 4 climax-turnover stocks (turnover = 11.5%) have identical/slightly higher raw factor score on T,
    // then crash -12% over T+1..T+5!
    for s in 0..8usize {
        let sym = format!("002{:03}.SZ", 700 + s);
        let is_climax = s >= 4;
        let turnover = if is_climax { 11.5 } else { 6.2 };
        let mut bars = Vec::new();
        let mut p = 15.0_f64;

        for d in 0..24usize {
            let date = format!("2026-10-{:02}", d + 1);
            let step = if d >= 11 {
                if is_climax {
                    0.972 // -2.8% per day crash -> ~ -13% over 5 days!
                } else {
                    1.012 // +1.2% per day steady gain -> ~ +6% over 5 days!
                }
            } else {
                1.003
            };
            let open = p;
            let close = (p * step).max(1.0);
            let high = open.max(close) * 1.012;
            let low = open.min(close) * 0.988;
            bars.push(SimpleBar {
                time: date,
                open,
                high,
                low,
                close,
                volume: if is_climax { 260_000.0 } else { 130_000.0 },
                amount_yuan: (if is_climax { 260_000.0 } else { 130_000.0 }) * close * 100.0,
                turnover_pct: turnover,
            });
            p = close;
        }

        raw_quotes.push(RawTencentQuote {
            symbol: Symbol::parse(&sym).unwrap(),
            name_zh: format!("影子测试{}", s),
            latest_yuan: p,
            prev_close_yuan: p * 0.99,
            open_yuan: p,
            high_yuan: p * 1.01,
            low_yuan: p * 0.99,
            volume_lots: 130_000.0,
            amount_wan: 18_000.0,
            turnover_pct: turnover,
            circ_mv_yi: 48.0 + (s as f64) * 3.0,
            total_mv_yi: 60.0 + (s as f64) * 3.0,
            vol_ratio: 1.5,
            vwap_yuan: p,
            is_suspended: false,
            raw_ts: "20261024150000".to_string(),
        });
        kline_cache_1d.insert(sym, bars);
    }

    let scan = ScanExecutionOutput {
        funnel: UniverseFunnelSummaryDto {
            total_active_count: 5500,
            tradable_count: 5100,
            under_200_yi_count: 4200,
            consecutive_turnover_vol_count: 20,
            pitfall_cleared_count: 8,
            qualified_buy_count: 4,
            provider_name: "r3-test".to_string(),
            scan_rtt_ms: 8,
            scanned_at_iso: "2026-10-24T15:00:00+08:00".to_string(),
            market_session: "CLOSED".to_string(),
        },
        candidates: Vec::new(),
        raw_quotes,
        clist_meta: HashMap::new(),
        kline_cache_1d,
    };

    let (panel, _) = build_pit_walk_forward_observations(Some(&scan), &bundle);
    let oos_obs: Vec<_> = panel
        .into_iter()
        .filter(|o| o.trade_date.as_str() >= "2026-10-12")
        .collect();
    assert!(!oos_obs.is_empty());

    // Identical factor weights! ONLY strategy turnover gate & ATR stop multiplier differ!
    let mut equal_w = BTreeMap::new();
    for k in bundle.strategy.factor_weights.keys() {
        equal_w.insert(k.clone(), 100.0 / (bundle.strategy.factor_weights.len() as f64));
    }

    let v_old = StrategyVersionSnapshot {
        version_id: "v1.0.0".to_string(),
        parent_version_id: None,
        source_tag: "BASELINE".to_string(),
        created_at: "2026-10-01T08:00:00Z".to_string(),
        is_active: false,
        summary_zh: "稳健换手门槛5%+标准1.8x止损".to_string(),
        factor_weights: equal_w.clone(),
        params: VersionParamsSnapshot {
            min_intraday_projected_turnover_pct: 5.0,
            insider_sell_veto_pct: 1.0,
            holder_surge_veto_pct: 15.0,
            main_outflow_veto_ratio: 0.08,
            atr_stop_mult: 1.8,
        },
        train_rank_ic: 0.05,
        oos_rank_ic: 0.04,
        oos_ic_ir: 0.65,
        pbo_risk_pct: 18.0,
    };

    let v_new = StrategyVersionSnapshot {
        version_id: "v1.1.0".to_string(),
        parent_version_id: Some("v1.0.0".to_string()),
        source_tag: "MANUAL_SAVE".to_string(),
        created_at: "2026-10-20T08:00:00Z".to_string(),
        is_active: true,
        summary_zh: "盲目拉高换手门槛至9.5%+放宽3.2x止损".to_string(),
        factor_weights: equal_w,
        params: VersionParamsSnapshot {
            min_intraday_projected_turnover_pct: 9.5, // forces selection ONLY from the >=11.5% climax-crash stocks!
            insider_sell_veto_pct: 1.0,
            holder_surge_veto_pct: 15.0,
            main_outflow_veto_ratio: 0.08,
            atr_stop_mult: 3.2, // widens stop so crashing climax stocks suffer much deeper tail loss!
        },
        train_rank_ic: 0.05,
        oos_rank_ic: 0.02,
        oos_ic_ir: 0.30,
        pbo_risk_pct: 32.0,
    };

    let cmp = compare_versions_and_attribute(&v_old, &v_new, &oos_obs);
    assert!(
        cmp.underperforming && cmp.oos_net_return_lift_bps < -80.0,
        "Expected v1.1.0 (forced >=9.5% climax turnover + 3.2x loose stop) to significantly underperform v1.0.0, got lift={:.2} bps",
        cmp.oos_net_return_lift_bps
    );
    let categories: Vec<&str> = cmp.root_causes.iter().map(|r| r.category.as_str()).collect();
    assert!(
        categories.contains(&"TURNOVER_PITFALL_PARAM"),
        "Expected TURNOVER_PITFALL_PARAM root cause when turnover gate jumped 5.0% -> 9.5%, got {:?}",
        categories
    );
    assert!(
        categories.contains(&"STOP_LOSS_PARAM"),
        "Expected STOP_LOSS_PARAM root cause when ATR stop widened 1.8x -> 3.2x, got {:?}",
        categories
    );
}

#[tokio::test]
async fn r3_02_insufficient_sample_audit_log_preserves_distinct_action_and_valid_dto_severity() {
    let sandbox = PathBuf::from(".temp").join("test_r3_audit_action_and_severity");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    // Seed empty / tiny scan cache -> must record INSUFFICIENT_SAMPLE, NOT REJECTED_OVERFIT_GUARD!
    {
        let mut w = svc.scan_cache.write().await;
        *w = Some(CachedScanState {
            captured_at: Instant::now(),
            output: ScanExecutionOutput {
                funnel: UniverseFunnelSummaryDto {
                    total_active_count: 100,
                    tradable_count: 90,
                    under_200_yi_count: 80,
                    consecutive_turnover_vol_count: 5,
                    pitfall_cleared_count: 4,
                    qualified_buy_count: 2,
                    provider_name: "r3-short".to_string(),
                    scan_rtt_ms: 5,
                    scanned_at_iso: "2026-11-19T15:00:00+08:00".to_string(),
                    market_session: "CLOSED".to_string(),
                },
                candidates: Vec::new(),
                raw_quotes: Vec::new(),
                clist_meta: HashMap::new(),
                kline_cache_1d: HashMap::new(),
            },
        });
    }

    let dash = svc.run_anti_overfit_iteration("MANUAL_SHORT").await.unwrap();
    assert_eq!(
        dash.evolution_logs[0].action,
        "INSUFFICIENT_SAMPLE",
        "Insufficient trading dates must surface action == INSUFFICIENT_SAMPLE rather than REJECTED_OVERFIT_GUARD"
    );

    // Create a second version with aggressive turnover & tightened ATR so rootCauses are generated,
    // and verify all RootCauseFindingDto.severity values strictly match TypeScript ["INFO", "WARN", "CRITICAL"]!
    let base_bundle = svc.plugins.load_all().unwrap();
    svc.update_plugin_config(UpdatePluginInputDto {
        min_intraday_projected_turnover_pct: 9.2,
        insider_sell_veto_pct: 2.5,
        holder_surge_veto_pct: 25.0,
        main_outflow_veto_ratio: 0.15,
        atr_stop_mult: 1.25,
        market_polarity: base_bundle.strategy.market_polarity,
        factor_weights: base_bundle.strategy.factor_weights,
    })
    .await
    .unwrap();

    // Populate a 22-day panel where >=9.2% turnover underperforms so root_causes is non-empty
    let mut kline_cache_1d = HashMap::new();
    let mut raw_quotes = Vec::new();
    for s in 0..8usize {
        let sym = format!("600{:03}.SH", 300 + s);
        let is_climax = s >= 4;
        let turnover = if is_climax { 10.8 } else { 6.0 };
        let mut bars = Vec::new();
        let mut p = 12.0_f64;
        for d in 0..22usize {
            let step = if d >= 11 {
                if is_climax { 0.972 } else { 1.015 }
            } else {
                1.003
            };
            let open = p;
            let close = p * step;
            bars.push(SimpleBar {
                time: format!("2026-10-{:02}", d + 1),
                open,
                high: open.max(close) * 1.01,
                low: open.min(close) * 0.99,
                close,
                volume: 100_000.0,
                amount_yuan: 100_000.0 * close * 100.0,
                turnover_pct: turnover,
            });
            p = close;
        }
        raw_quotes.push(RawTencentQuote {
            symbol: Symbol::parse(&sym).unwrap(),
            name_zh: format!("测试{}", s),
            latest_yuan: p,
            prev_close_yuan: p,
            open_yuan: p,
            high_yuan: p,
            low_yuan: p,
            volume_lots: 100_000.0,
            amount_wan: 12_000.0,
            turnover_pct: turnover,
            circ_mv_yi: 50.0,
            total_mv_yi: 65.0,
            vol_ratio: 1.3,
            vwap_yuan: p,
            is_suspended: false,
            raw_ts: "20261022150000".to_string(),
        });
        kline_cache_1d.insert(sym, bars);
    }
    {
        let mut w = svc.scan_cache.write().await;
        *w = Some(CachedScanState {
            captured_at: Instant::now(),
            output: ScanExecutionOutput {
                funnel: UniverseFunnelSummaryDto {
                    total_active_count: 5000,
                    tradable_count: 4800,
                    under_200_yi_count: 4000,
                    consecutive_turnover_vol_count: 16,
                    pitfall_cleared_count: 8,
                    qualified_buy_count: 4,
                    provider_name: "r3-full".to_string(),
                    scan_rtt_ms: 5,
                    scanned_at_iso: "2026-10-22T15:00:00+08:00".to_string(),
                    market_session: "CLOSED".to_string(),
                },
                candidates: Vec::new(),
                raw_quotes,
                clist_meta: HashMap::new(),
                kline_cache_1d,
            },
        });
    }

    let dash2 = svc.get_version_control_dashboard(None, None).await.unwrap();
    let cmp = &dash2.comparison;
    assert!(!cmp.root_causes.is_empty());
    for rc in &cmp.root_causes {
        assert!(
            matches!(rc.severity.as_str(), "INFO" | "WARN" | "CRITICAL"),
            "RootCauseFindingDto.severity '{}' violated TypeScript contract ['INFO','WARN','CRITICAL']",
            rc.severity
        );
    }
}

#[tokio::test]
async fn r3_03_tencent_fqkline_legacy_host_auto_upgrades_to_web_subdomain() {
    use economist_quant_terminal_lib::infrastructure::market_tencent::fetch_tencent_fallback_klines;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .unwrap();
    let sym = Symbol::parse("300308.SZ").unwrap();

    // Passing legacy bare ifzq.gtimg.cn (which returns HTTP 501 HTML on fqkline) must auto-upgrade
    // to web.ifzq.gtimg.cn for daily bars and return valid K-line bars!
    let bars = fetch_tencent_fallback_klines(
        &client,
        "https://ifzq.gtimg.cn/appstock/app/fqkline/get",
        "https://ifzq.gtimg.cn/appstock/app/kline/mkline",
        &sym,
        "1d",
        40,
        1500.0,
    )
    .await
    .expect("Legacy ifzq.gtimg.cn fqkline host must auto-upgrade to web.ifzq.gtimg.cn and parse JSON cleanly");
    assert!(
        bars.len() >= 20,
        "Expected >=20 daily fallback bars for 300308.SZ, got {}",
        bars.len()
    );
}
