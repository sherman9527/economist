use economist_quant_terminal_lib::application::pit_observation_builder::build_pit_walk_forward_observations;
use economist_quant_terminal_lib::application::scan_pipeline::ScanExecutionOutput;
use economist_quant_terminal_lib::domain::anti_overfit::{
    evaluate_walk_forward_iteration, CrossSectionalFactorObservation, WalkForwardConfig,
};
use economist_quant_terminal_lib::domain::types::{Fen, Symbol};
use economist_quant_terminal_lib::domain::watchlist_tracker::{
    evaluate_watchlist_entry, WatchlistItemState, WatchlistOutcome,
};
use economist_quant_terminal_lib::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};
use economist_quant_terminal_lib::infrastructure::plugin_store::PluginStore;
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::interface::dto::UniverseFunnelSummaryDto;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[test]
fn r2_01_candidate_weights_never_peek_at_oos_holdout_window() {
    let fids = [
        "F1_ideal_reversal",
        "F2_concave_abn_turnover",
        "F3_amihud_compression",
        "F4_intraday_apm_vwap",
        "F5_main_inflow_ir",
        "F6_pv_efficiency",
    ];
    let mut prior = BTreeMap::new();
    for id in fids {
        prior.insert(id.to_string(), 100.0 / 6.0);
    }

    let build_panel = |oos_favored_idx: usize| -> Vec<CrossSectionalFactorObservation> {
        let mut obs = Vec::new();
        for d in 0..25usize {
            let date = format!("2026-10-{:02}", d + 1);
            for s in 0..12usize {
                let z_main = (s as f64 - 5.5) / 3.0;
                let z_alt_1 = if s % 2 == 0 { 1.1 } else { -1.1 } + (s as f64) * 0.08;
                let z_alt_2 = if s % 3 == 0 { 1.0 } else { -0.7 } + (s as f64) * 0.06;
                let mut fz = BTreeMap::new();
                for (k, id) in fids.iter().enumerate() {
                    let v = match k {
                        0 => z_main,
                        1 => z_alt_1,
                        2 => z_alt_2,
                        _ => 0.15 * z_main,
                    };
                    fz.insert((*id).to_string(), v);
                }
                // Train + Embargo (first 18 days) are 100% IDENTICAL in both runs!
                // Only OOS holdout days (d >= 18) differ in whether F2 or F3 gets boosted.
                let ret = if d < 18 {
                    120.0 * z_main + 40.0 * z_alt_1 + 35.0 * z_alt_2
                } else if oos_favored_idx == 1 {
                    95.0 * z_main + 180.0 * z_alt_1 + 10.0 * z_alt_2
                } else {
                    95.0 * z_main + 10.0 * z_alt_1 + 180.0 * z_alt_2
                };
                obs.push(CrossSectionalFactorObservation {
                    trade_date: date.clone(),
                    symbol: format!("002{:03}.SZ", 100 + s),
                    factor_z: fz,
                    forward_5d_net_ret_bps: ret,
                    upper_shadow_ratio: 0.15,
                    ..Default::default()
                });
            }
        }
        obs
    };

    let cfg = WalkForwardConfig::default();
    let res_a = evaluate_walk_forward_iteration(&build_panel(1), &prior, &BTreeMap::new(), &cfg);
    let res_b = evaluate_walk_forward_iteration(&build_panel(2), &prior, &BTreeMap::new(), &cfg);

    assert!(res_a.promoted && res_b.promoted);
    for id in fids {
        let wa = res_a.proposed_weights[id];
        let wb = res_b.proposed_weights[id];
        assert!(
            (wa - wb).abs() < 1e-9,
            "OOS snooping detected! Factor {} proposed weight changed from {:.6} to {:.6} when ONLY OOS holdout returns changed",
            id,
            wa,
            wb
        );
    }
}

#[test]
fn r2_02_embargo_never_shrinks_below_5_trading_days_and_zero_lift_is_rejected() {
    let mut prior = BTreeMap::new();
    prior.insert("F1".to_string(), 50.0);
    prior.insert("F2".to_string(), 50.0);

    // 12 dates (< 4 train + 5 embargo + 5 oos = 14): must NOT shrink embargo to 12/4 = 3 days!
    let mut short_obs = Vec::new();
    for d in 0..12usize {
        for s in 0..8usize {
            let z = (s as f64 - 3.5) / 2.0;
            let mut fz = BTreeMap::new();
            fz.insert("F1".to_string(), z);
            fz.insert("F2".to_string(), z);
            short_obs.push(CrossSectionalFactorObservation {
                trade_date: format!("2026-10-{:02}", d + 1),
                symbol: format!("600{:03}.SH", s + 1),
                factor_z: fz,
                forward_5d_net_ret_bps: z * 100.0,
                upper_shadow_ratio: 0.1,
                ..Default::default()
            });
        }
    }
    let cfg = WalkForwardConfig::default();
    let res_short = evaluate_walk_forward_iteration(&short_obs, &prior, &BTreeMap::new(), &cfg);
    assert!(
        !res_short.promoted && res_short.verdict == "INSUFFICIENT_SAMPLE",
        "12 dates cannot fit 5-day non-overlapping embargo without label leakage; got verdict={} embargo={}",
        res_short.verdict,
        res_short.embargo_dates
    );

    // Identical symmetric factors on 20 dates: candidate_weights == [50, 50] == prior_weights -> zero OOS lift -> must NOT promote duplicate version!
    let mut flat_obs = Vec::new();
    for d in 0..20usize {
        for s in 0..8usize {
            let z = (s as f64 - 3.5) / 2.0;
            let mut fz = BTreeMap::new();
            fz.insert("F1".to_string(), z);
            fz.insert("F2".to_string(), z);
            flat_obs.push(CrossSectionalFactorObservation {
                trade_date: format!("2026-10-{:02}", d + 1),
                symbol: format!("600{:03}.SH", s + 1),
                factor_z: fz,
                forward_5d_net_ret_bps: z * 100.0,
                upper_shadow_ratio: 0.1,
                ..Default::default()
            });
        }
    }
    let res_flat = evaluate_walk_forward_iteration(&flat_obs, &prior, &BTreeMap::new(), &cfg);
    assert!(
        !res_flat.promoted,
        "Zero OOS lift (candidate identical to prior) must not mint a redundant promoted version"
    );
}

#[test]
fn r2_03_watchlist_gap_down_below_stop_fills_at_realistic_traded_high_not_phantom_stop() {
    let mut item = WatchlistItemState::new_tracking(
        "300766.SZ",
        "每日互动",
        "CHINEXT",
        "计算机",
        "2026-11-18",
        "2026-11-18T06:00:00Z",
        Fen::from_yuan(20.00),
        Fen::from_yuan(18.60), // -7.0% stop
        Fen::from_yuan(23.00),
        72.0,
        "WATCH",
        "v1.2.0",
        "测试跳空跌破止损真实成交价",
        BTreeMap::new(),
    );

    // T+1 gaps down sharply: bar trades between [17.00, 17.50], NEVER reaching 18.60!
    evaluate_watchlist_entry(
        &mut item,
        "2026-11-19",
        Fen::from_yuan(17.20),
        Fen::from_yuan(17.50),
        Fen::from_yuan(17.00),
    );
    assert_eq!(item.outcome, WatchlistOutcome::HitStop);
    assert_eq!(
        item.settled_price_fen,
        Some(Fen::from_yuan(17.50)),
        "Gap-down bar [17.00, 17.50] below stop 18.60 cannot fill at unreached 18.60"
    );
    // Return at 17.50 on 20.00 entry = -1250 bps - 25 bps = -1275 bps
    assert!((item.net_return_bps - (-1275.0)).abs() < 2.0);
}

#[test]
fn r2_04_pit_builder_filters_unbuyable_t1_one_word_limit_up_bars() {
    let sandbox = PathBuf::from(".temp").join("test_r2_pit_limit_up_filter");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let store = PluginStore::new(paths);
    let bundle = store.load_all().unwrap();

    let mut kline_cache_1d: HashMap<String, Vec<SimpleBar>> = HashMap::new();
    let mut raw_quotes = Vec::new();

    for s in 0..6usize {
        let sym = format!("002{:03}.SZ", 600 + s);
        let mut bars = Vec::new();
        let mut p = 10.0_f64;
        for d in 0..22usize {
            let date = format!("2026-10-{:02}", d + 1);
            // Make s == 0 lock in a 10% one-word limit-up (open == high == low == close == prev * 1.10) on day 12 (t+1 for t=11)!
            if s == 0 && d == 12 {
                let locked = (p * 1.10 * 100.0).round() / 100.0;
                bars.push(SimpleBar {
                    time: date,
                    open: locked,
                    high: locked,
                    low: locked,
                    close: locked,
                    volume: 5_000.0,
                    amount_yuan: 5_000.0 * locked * 100.0,
                    turnover_pct: 0.4,
                });
                p = locked;
            } else {
                let open = p;
                let close = p * 1.005;
                bars.push(SimpleBar {
                    time: date,
                    open,
                    high: close * 1.01,
                    low: open * 0.99,
                    close,
                    volume: 100_000.0,
                    amount_yuan: 100_000.0 * close * 100.0,
                    turnover_pct: 6.2,
                });
                p = close;
            }
        }
        raw_quotes.push(RawTencentQuote {
            symbol: Symbol::parse(&sym).unwrap(),
            name_zh: format!("样本{}", s),
            latest_yuan: p,
            prev_close_yuan: p * 0.99,
            open_yuan: p * 0.995,
            high_yuan: p * 1.01,
            low_yuan: p * 0.985,
            volume_lots: 100_000.0,
            amount_wan: 10_000.0,
            turnover_pct: 6.2,
            circ_mv_yi: 40.0 + (s as f64) * 8.0,
            total_mv_yi: 50.0 + (s as f64) * 8.0,
            vol_ratio: 1.4,
            vwap_yuan: p,
            is_suspended: false,
            raw_ts: "20261022150000".to_string(),
        });
        kline_cache_1d.insert(sym, bars);
    }

    let scan = ScanExecutionOutput {
        funnel: UniverseFunnelSummaryDto {
            total_active_count: 5500,
            tradable_count: 5100,
            under_200_yi_count: 4200,
            consecutive_turnover_vol_count: 20,
            pitfall_cleared_count: 10,
            qualified_buy_count: 5,
            provider_name: "r2-test".to_string(),
            scan_rtt_ms: 8,
            scanned_at_iso: "2026-10-22T15:00:00+08:00".to_string(),
            market_session: "CLOSED".to_string(),
        },
        candidates: Vec::new(),
        raw_quotes,
        clist_meta: HashMap::new(),
        kline_cache_1d,
    };

    let (panel, _) = build_pit_walk_forward_observations(Some(&scan), &bundle);
    // On signal date 2026-10-12 (d=11), 002600.SZ has a one-word limit-up on T+1 (2026-10-13, d=12) -> must NOT appear!
    let has_unbuyable_entry = panel
        .iter()
        .any(|o| o.trade_date == "2026-10-12" && o.symbol == "002600.SZ");
    assert!(
        !has_unbuyable_entry,
        "Unbuyable T+1 one-word limit-up bar must be excluded from walk-forward training/OOS labels"
    );
}
