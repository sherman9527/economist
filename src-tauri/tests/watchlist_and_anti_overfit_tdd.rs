use economist_quant_terminal_lib::domain::anti_overfit::{
    evaluate_walk_forward_iteration, CrossSectionalFactorObservation, WalkForwardConfig,
};
use economist_quant_terminal_lib::domain::types::Fen;
use economist_quant_terminal_lib::domain::version_control::{
    compare_versions_and_attribute, evaluate_shadow_versions_on_oos, StrategyVersionSnapshot,
    VersionParamsSnapshot,
};
use economist_quant_terminal_lib::domain::watchlist_tracker::{
    evaluate_watchlist_entry, summarize_watchlist_win_rate, WatchlistItemState, WatchlistOutcome,
};
use proptest::prelude::*;
use std::collections::BTreeMap;

#[test]
fn t0_same_day_spike_never_settles_target_or_stop_due_to_ashare_t_plus_1() {
    let mut item = WatchlistItemState::new_tracking(
        "600103.SH",
        "青山纸业",
        "MAIN",
        "造纸印刷",
        "2026-11-19",
        "2026-11-19T03:00:00Z",
        Fen::from_yuan(10.00),
        Fen::from_yuan(9.20),
        Fen::from_yuan(11.20),
        73.2,
        "BUY",
        "v1.2.0",
        "双日换手>5%放量突破但暂观望验证胜率",
        BTreeMap::new(),
    );

    // Same trade date (T+0): price hits 11.50 (> target 11.20), must stay TRACKING!
    evaluate_watchlist_entry(
        &mut item,
        "2026-11-19",
        Fen::from_yuan(11.50),
        Fen::from_yuan(11.60),
        Fen::from_yuan(9.80),
    );
    assert_eq!(item.holding_trading_days, 0);
    assert_eq!(item.outcome, WatchlistOutcome::Tracking);
    assert_eq!(item.peak_price_fen, Fen::from_yuan(11.60));

    // Next trading day (T+1 = 2026-11-20): high hits target -> settles HIT_TARGET net of 25 bps cost
    evaluate_watchlist_entry(
        &mut item,
        "2026-11-20",
        Fen::from_yuan(11.30),
        Fen::from_yuan(11.40),
        Fen::from_yuan(10.90),
    );
    assert_eq!(item.holding_trading_days, 1);
    assert_eq!(item.outcome, WatchlistOutcome::HitTarget);
    // Gross target = +12.00% (+1200 bps), minus 25 bps round-trip friction = +1175 bps
    assert!((item.net_return_bps - 1175.0).abs() < 2.0);
}

#[test]
fn five_day_horizon_deducts_25bps_friction_and_computes_wilson_95_ci() {
    let mut tiny_gain_loss_after_fees = WatchlistItemState::new_tracking(
        "000819.SZ",
        "岳阳兴长",
        "MAIN",
        "石油石化",
        "2026-11-10",
        "2026-11-10T06:00:00Z",
        Fen::from_yuan(20.00),
        Fen::from_yuan(18.40),
        Fen::from_yuan(23.00),
        71.0,
        "WATCH",
        "v1.2.0",
        "观望跟踪5日净胜率",
        BTreeMap::new(),
    );
    // 5 trading days later: 2026-11-17, price is 20.03 (+15 bps gross, -10 bps net after 25 bps cost!)
    evaluate_watchlist_entry(
        &mut tiny_gain_loss_after_fees,
        "2026-11-17",
        Fen::from_yuan(20.03),
        Fen::from_yuan(20.40),
        Fen::from_yuan(19.60),
    );
    assert_eq!(tiny_gain_loss_after_fees.holding_trading_days, 5);
    assert_eq!(
        tiny_gain_loss_after_fees.outcome,
        WatchlistOutcome::Settled5dLoss
    );
    assert!(tiny_gain_loss_after_fees.net_return_bps < 0.0);

    let mut solid_win = WatchlistItemState::new_tracking(
        "603215.SH",
        "比依股份",
        "MAIN",
        "家电行业",
        "2026-11-10",
        "2026-11-10T06:00:00Z",
        Fen::from_yuan(15.00),
        Fen::from_yuan(13.80),
        Fen::from_yuan(17.50),
        72.8,
        "BUY",
        "v1.2.0",
        "高分观望验证",
        BTreeMap::new(),
    );
    evaluate_watchlist_entry(
        &mut solid_win,
        "2026-11-17",
        Fen::from_yuan(15.90),
        Fen::from_yuan(16.10),
        Fen::from_yuan(14.85),
    );
    assert_eq!(solid_win.outcome, WatchlistOutcome::Settled5dWin);

    let summary = summarize_watchlist_win_rate(&[tiny_gain_loss_after_fees, solid_win]);
    assert_eq!(summary.total_count, 2);
    assert_eq!(summary.settled_count, 2);
    assert_eq!(summary.win_count, 1);
    assert_eq!(summary.loss_count, 1);
    assert!((summary.settled_win_rate_pct - 50.0).abs() < 1e-6);
    assert!(summary.wilson_low_pct < 50.0 && summary.wilson_high_pct > 50.0);
    assert!(summary.low_sample_warning);
}

#[test]
fn walk_forward_anti_overfit_rejects_in_sample_mirage_when_oos_sign_flips() {
    let mut prior_weights = BTreeMap::new();
    for id in [
        "f_turnover_accel",
        "f_close_location",
        "f_intraday_vwap_strength",
        "f_smart_flow_ratio",
        "f_microcap_tilt",
        "f_low_atr_quality",
    ] {
        prior_weights.insert(id.to_string(), 100.0 / 6.0);
    }

    // Build 25 trading days: first 15 days (Train) f_turnover_accel strongly positive,
    // 5 days embargo, last 5 days (OOS) f_turnover_accel flips strongly negative (classic overfit trap!).
    let mut obs = Vec::new();
    for day_idx in 0..25 {
        let date = format!("2026-10-{:02}", day_idx + 1);
        for s_idx in 0..12 {
            let z = (s_idx as f64 - 5.5) / 3.0;
            let mut fz = BTreeMap::new();
            fz.insert("f_turnover_accel".to_string(), z);
            fz.insert("f_close_location".to_string(), z * 0.8);
            fz.insert("f_intraday_vwap_strength".to_string(), -0.1 * z);
            fz.insert("f_smart_flow_ratio".to_string(), 0.2 * z);
            fz.insert("f_microcap_tilt".to_string(), 0.1 * z);
            fz.insert("f_low_atr_quality".to_string(), -0.2 * z);

            let fwd_ret_bps = if day_idx < 15 {
                z * 180.0 // strong positive in Train
            } else {
                -z * 160.0 // flips negative in OOS!
            };
            obs.push(CrossSectionalFactorObservation {
                trade_date: date.clone(),
                symbol: format!("000{:03}.SZ", s_idx + 1),
                factor_z: fz,
                forward_5d_net_ret_bps: fwd_ret_bps,
                upper_shadow_ratio: if day_idx >= 15 { 0.48 } else { 0.15 },
            });
        }
    }

    let report = evaluate_walk_forward_iteration(
        &obs,
        &prior_weights,
        &BTreeMap::new(),
        &WalkForwardConfig::default(),
    );
    assert!(!report.promoted, "Overfitted IS-only signal MUST be vetoed");
    assert_eq!(report.verdict, "REJECTED_OVERFIT_GUARD");
    assert_eq!(report.proposed_weights, prior_weights);
    assert!(report.train_composite_rank_ic > 0.20);
    assert!(report.oos_composite_rank_ic < 0.0);
}

#[test]
fn version_comparison_pinpoints_exact_root_cause_when_new_version_underperforms_old() {
    let mut w_old = BTreeMap::new();
    w_old.insert("f_turnover_accel".to_string(), 18.0);
    w_old.insert("f_close_location".to_string(), 16.0);
    w_old.insert("f_intraday_vwap_strength".to_string(), 16.0);
    w_old.insert("f_smart_flow_ratio".to_string(), 16.0);
    w_old.insert("f_microcap_tilt".to_string(), 10.0);
    w_old.insert("f_low_atr_quality".to_string(), 24.0);

    // New version over-tilted into turnover_accel (+10%) & cut defensive low_atr_quality (-10%),
    // AND tightened ATR stop multiplier from 2.0 to 1.35 causing whipsaw stop-outs!
    let mut w_new = w_old.clone();
    w_new.insert("f_turnover_accel".to_string(), 28.0);
    w_new.insert("f_low_atr_quality".to_string(), 14.0);

    let v_old = StrategyVersionSnapshot {
        version_id: "v1.1.0".to_string(),
        parent_version_id: Some("v1.0.0".to_string()),
        source_tag: "BASELINE".to_string(),
        created_at: "2026-11-10T08:00:00Z".to_string(),
        is_active: false,
        summary_zh: "稳健低波防守均衡版".to_string(),
        factor_weights: w_old,
        params: VersionParamsSnapshot {
            min_intraday_projected_turnover_pct: 5.2,
            insider_sell_veto_pct: 1.0,
            holder_surge_veto_pct: 15.0,
            main_outflow_veto_ratio: 0.08,
            atr_stop_mult: 2.0,
        },
        train_rank_ic: 0.062,
        oos_rank_ic: 0.054,
        oos_ic_ir: 0.72,
        pbo_risk_pct: 14.0,
    };

    let v_new = StrategyVersionSnapshot {
        version_id: "v1.2.1".to_string(),
        parent_version_id: Some("v1.1.0".to_string()),
        source_tag: "MANUAL_SAVE".to_string(),
        created_at: "2026-11-18T08:00:00Z".to_string(),
        is_active: true,
        summary_zh: "激进换手追涨+收紧ATR止损实验版".to_string(),
        factor_weights: w_new,
        params: VersionParamsSnapshot {
            min_intraday_projected_turnover_pct: 5.2,
            insider_sell_veto_pct: 1.0,
            holder_surge_veto_pct: 15.0,
            main_outflow_veto_ratio: 0.08,
            atr_stop_mult: 1.35,
        },
        train_rank_ic: 0.095,
        oos_rank_ic: 0.018,
        oos_ic_ir: 0.21,
        pbo_risk_pct: 42.0,
    };

    // Build OOS regime where high turnover_accel reversed negatively while low_atr_quality protected alpha!
    let mut oos_obs = Vec::new();
    for day_idx in 0..8 {
        let date = format!("2026-11-{:02}", day_idx + 10);
        for s_idx in 0..10 {
            let defensive_z = (s_idx as f64 - 4.5) / 2.5;
            let chase_z = -defensive_z;
            let mut fz = BTreeMap::new();
            fz.insert("f_turnover_accel".to_string(), chase_z);
            fz.insert("f_close_location".to_string(), 0.1);
            fz.insert("f_intraday_vwap_strength".to_string(), 0.1);
            fz.insert("f_smart_flow_ratio".to_string(), 0.2 * defensive_z);
            fz.insert("f_microcap_tilt".to_string(), 0.0);
            fz.insert("f_low_atr_quality".to_string(), defensive_z);

            oos_obs.push(CrossSectionalFactorObservation {
                trade_date: date.clone(),
                symbol: format!("300{:03}.SZ", s_idx + 101),
                factor_z: fz,
                forward_5d_net_ret_bps: defensive_z * 140.0,
                upper_shadow_ratio: 0.46,
            });
        }
    }

    let shadow_board = evaluate_shadow_versions_on_oos(&[v_old.clone(), v_new.clone()], &oos_obs);
    assert_eq!(shadow_board.len(), 2);

    let diff = compare_versions_and_attribute(&v_old, &v_new, &oos_obs);
    assert!(diff.underperforming, "v1.2.1 must be flagged as underperforming v1.1.0");
    assert!(diff.oos_net_return_lift_bps < 0.0);
    assert!(!diff.root_causes.is_empty());
    // Must pinpoint factor weight shift drag (f_turnover_accel / f_low_atr_quality), regime shift, and tighter ATR stop!
    let joined_reasons = diff
        .root_causes
        .iter()
        .map(|r| format!("{}: {}", r.category, r.detail_zh))
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(joined_reasons.contains("f_turnover_accel") || joined_reasons.contains("f_low_atr_quality"));
    assert!(joined_reasons.contains("ATR") || joined_reasons.contains("止损"));
    assert!(joined_reasons.contains("上影线") || joined_reasons.contains("风格"));
}

proptest! {
    #[test]
    fn prop_shrinkage_step_cap_and_simplex_invariants_always_hold(
        signal_strength in -200.0f64..200.0f64,
        tilt_idx in 0usize..6usize,
    ) {
        let ids = [
            "f_turnover_accel",
            "f_close_location",
            "f_intraday_vwap_strength",
            "f_smart_flow_ratio",
            "f_microcap_tilt",
            "f_low_atr_quality",
        ];
        let mut prior = BTreeMap::new();
        for id in ids {
            prior.insert(id.to_string(), 100.0 / 6.0);
        }
        let mut obs = Vec::new();
        for d in 0..22 {
            let date = format!("2026-10-{:02}", d + 1);
            for s in 0..10 {
                let z = (s as f64 - 4.5) / 2.5;
                let mut fz = BTreeMap::new();
                for (k, id) in ids.iter().enumerate() {
                    fz.insert((*id).to_string(), if k == tilt_idx { z } else { 0.35 * z });
                }
                obs.push(CrossSectionalFactorObservation {
                    trade_date: date.clone(),
                    symbol: format!("600{:03}.SH", s + 1),
                    factor_z: fz,
                    forward_5d_net_ret_bps: z * signal_strength,
                    upper_shadow_ratio: 0.18,
                });
            }
        }
        let cfg = WalkForwardConfig::default();
        let res = evaluate_walk_forward_iteration(&obs, &prior, &BTreeMap::new(), &cfg);
        let sum: f64 = res.proposed_weights.values().sum();
        prop_assert!((sum - 100.0).abs() < 1e-4);
        for (id, &w) in &res.proposed_weights {
            let p = prior[id];
            prop_assert!((5.0 - 1e-4..=35.0 + 1e-4).contains(&w));
            // Single iteration drift must never exceed ±3.5% (3.0% step + simplex renormalization tolerance)
            prop_assert!((w - p).abs() <= 3.55);
        }
        if signal_strength <= 0.0 {
            prop_assert!(!res.promoted);
        }
    }
}

#[tokio::test]
async fn service_watchlist_version_switching_shadow_tracking_and_walk_forward_e2e() {
    use economist_quant_terminal_lib::application::scan_pipeline::ScanExecutionOutput;
    use economist_quant_terminal_lib::application::terminal_service::{
        CachedScanState, TerminalService,
    };
    use economist_quant_terminal_lib::domain::types::Symbol;
    use economist_quant_terminal_lib::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};
    use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
    use economist_quant_terminal_lib::interface::dto::{
        AddWatchlistInputDto, UniverseFunnelSummaryDto, UpdatePluginInputDto,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Instant;

    let sandbox = PathBuf::from(".temp").join("test_watchlist_version_evolution_e2e");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    // Seed 40 trading days across 8 small-cap stocks in scan_cache so PIT Walk-Forward has real bars
    let mut kline_cache_1d: HashMap<String, Vec<SimpleBar>> = HashMap::new();
    let mut raw_quotes = Vec::new();
    for s in 0..8usize {
        let sym_str = format!("002{:03}.SZ", 400 + s);
        let mut bars = Vec::new();
        let mut price = 15.0 + (s as f64);
        for d in 0..40usize {
            let drift = if s % 2 == 0 { 0.012 } else { -0.004 };
            let open = price;
            let close = (open * (1.0 + drift)).max(2.0);
            let high = close * 1.018;
            let low = open * 0.988;
            bars.push(SimpleBar {
                time: format!("2026-09-{:02}", (d % 28) + 1)
                    .replacen("2026-09-", if d < 20 { "2026-09-" } else { "2026-10-" }, 1),
                open,
                high,
                low,
                close,
                volume: 120_000.0 + (s as f64) * 8_000.0 + (d as f64) * 1_500.0,
                amount_yuan: 45_000_000.0,
                turnover_pct: if s % 2 == 0 { 7.2 } else { 5.4 },
            });
            price = close;
        }
        raw_quotes.push(RawTencentQuote {
            symbol: Symbol::parse(&sym_str).unwrap(),
            name_zh: format!("样本小盘{}", s + 1),
            latest_yuan: price,
            prev_close_yuan: price * 0.98,
            open_yuan: price * 0.985,
            high_yuan: price * 1.02,
            low_yuan: price * 0.975,
            volume_lots: 160_000.0,
            amount_wan: 5_200.0,
            turnover_pct: 7.5,
            circ_mv_yi: 65.0,
            total_mv_yi: 78.0,
            vol_ratio: 1.5,
            vwap_yuan: price * 0.994,
            is_suspended: false,
            raw_ts: "20261119110000".to_string(),
        });
        kline_cache_1d.insert(sym_str, bars);
    }

    {
        let mut w = svc.scan_cache.write().await;
        *w = Some(CachedScanState {
            output: ScanExecutionOutput {
                funnel: UniverseFunnelSummaryDto {
                    total_active_count: 5931,
                    tradable_count: 5300,
                    under_200_yi_count: 4532,
                    consecutive_turnover_vol_count: 21,
                    pitfall_cleared_count: 10,
                    qualified_buy_count: 8,
                    provider_name: "unit-test".to_string(),
                    scan_rtt_ms: 12,
                    scanned_at_iso: "2026-11-19T11:00:00+08:00".to_string(),
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

    // 1. Add stock to Watchlist ("观望加入自选")
    let wl = svc
        .add_to_watchlist(AddWatchlistInputDto {
            symbol: "002400.SZ".to_string(),
            name_zh: "样本小盘1".to_string(),
            board: "MAIN".to_string(),
            industry_sw1: "电子".to_string(),
            entry_price_yuan: 18.00,
            stop_loss_price_yuan: 16.56,
            target_price_yuan: 20.80,
            conviction_score: 73.5,
            source_decision: "WATCH".to_string(),
            rationale_zh: "双日高换手但暂不占用100W仓位，加入自选验证样本外胜率".to_string(),
            factor_z_scores: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(wl.items.len(), 1);
    assert_eq!(wl.items[0].symbol, "002400.SZ");
    assert!(!wl.items[0].version_id_at_entry.is_empty());

    // 2. Verify initial version control registry has >= 3 seeded lineage versions (v1.0.0 -> v1.1.0 -> v1.2.0)
    let vc_initial = svc.get_version_control_dashboard(None, None).await.unwrap();
    assert!(vc_initial.versions.len() >= 3);
    assert_eq!(vc_initial.active_version_id, "v1.2.0");

    // 3. Save a new manual version (auto-increments to v1.2.1) and verify old versions stay tracked!
    let mut custom_w = BTreeMap::new();
    custom_w.insert("F1_ideal_reversal".to_string(), 25.0);
    custom_w.insert("F2_concave_abn_turnover".to_string(), 22.0);
    custom_w.insert("F3_amihud_compression".to_string(), 15.0);
    custom_w.insert("F4_intraday_apm_vwap".to_string(), 15.0);
    custom_w.insert("F5_main_inflow_ir".to_string(), 13.0);
    custom_w.insert("F6_pv_efficiency".to_string(), 10.0);
    let _ = svc
        .update_plugin_config(UpdatePluginInputDto {
            min_intraday_projected_turnover_pct: 5.5,
            insider_sell_veto_pct: 0.5,
            holder_surge_veto_pct: 25.0,
            main_outflow_veto_ratio: -0.08,
            atr_stop_mult: 1.4,
            market_polarity: "cn-red-up".to_string(),
            factor_weights: custom_w,
        })
        .await
        .unwrap();

    let vc_after_save = svc
        .get_version_control_dashboard(Some("v1.1.0".to_string()), Some("v1.2.1".to_string()))
        .await
        .unwrap();
    assert_eq!(vc_after_save.active_version_id, "v1.2.1");
    assert!(vc_after_save.versions.iter().any(|v| v.version_id == "v1.1.0"));
    assert_eq!(vc_after_save.comparison.base_version_id, "v1.1.0");
    assert_eq!(vc_after_save.comparison.target_version_id, "v1.2.1");

    // 4. Switch active version back to legacy v1.1.0 and verify plugin config updates live!
    let switched = svc.switch_strategy_version("v1.1.0").await.unwrap();
    assert_eq!(switched.active_version_id, "v1.1.0");

    // 5. Run Purged Walk-Forward anti-overfitting iteration and verify audit persistence
    let wf_res = svc
        .run_anti_overfit_iteration("MANUAL_AUDIT")
        .await
        .unwrap();
    assert!(!wf_res.evolution_logs.is_empty());

    // 6. Remove watchlist item cleanly
    let wl_after_rm = svc.remove_from_watchlist("002400.SZ").await.unwrap();
    assert!(wl_after_rm.items.is_empty());
}
