use crate::application::scan_factors::{build_factor_vars, compute_ma_and_atr14};
use crate::application::scan_pipeline::ScanExecutionOutput;
use crate::domain::anti_overfit::CrossSectionalFactorObservation;
use crate::domain::factor_engine::{
    eval_factor_ast, neutralize_and_score_cross_section, parse_factor_expr, CrossSectionSample,
    ExprNode,
};
use crate::domain::types::Symbol;
use crate::domain::watchlist_tracker::DEFAULT_ROUND_TRIP_COST_BPS;
use crate::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};
use crate::infrastructure::plugin_store::PluginBundle;
use std::collections::{BTreeMap, HashMap};

pub fn count_ast_nodes(node: &ExprNode) -> usize {
    match node {
        ExprNode::Literal(_) | ExprNode::Var(_) => 1,
        ExprNode::UnaryNeg(inner) => 1 + count_ast_nodes(inner),
        ExprNode::Binary { lhs, rhs, .. } => 1 + count_ast_nodes(lhs) + count_ast_nodes(rhs),
        ExprNode::Call { args, .. } => 1 + args.iter().map(count_ast_nodes).sum::<usize>(),
    }
}

pub fn build_pit_walk_forward_observations(
    scan: Option<&ScanExecutionOutput>,
    bundle: &PluginBundle,
) -> (Vec<CrossSectionalFactorObservation>, BTreeMap<String, usize>) {
    let mut parsed_factors: Vec<(String, ExprNode)> = Vec::new();
    let mut ast_complexity: BTreeMap<String, usize> = BTreeMap::new();
    for f in &bundle.factors.factors {
        if let Ok(ast) = parse_factor_expr(&f.expr) {
            ast_complexity.insert(f.factor_id.clone(), count_ast_nodes(&ast));
            parsed_factors.push((f.factor_id.clone(), ast));
        }
    }
    let Some(scan_out) = scan else {
        return (Vec::new(), ast_complexity);
    };
    if scan_out.kline_cache_1d.is_empty() || parsed_factors.is_empty() {
        return (Vec::new(), ast_complexity);
    }

    struct RawObs {
        trade_date: String,
        symbol: String,
        industry_sw1: String,
        pit_circ_mv_yi: f64,
        raw_factors: BTreeMap<String, f64>,
        fwd_5d_net_bps: f64,
        upper_shadow_ratio: f64,
        turnover_pct: f64,
        max_adverse_atr_excursion: f64,
        atr_pct_bps: f64,
    }
    let quote_lookup: HashMap<String, &RawTencentQuote> = scan_out
        .raw_quotes
        .iter()
        .map(|q| (q.symbol.canonical(), q))
        .collect();
    let mut by_date_raw: BTreeMap<String, Vec<RawObs>> = BTreeMap::new();

    for (sym_str, bars) in &scan_out.kline_cache_1d {
        if bars.len() < 18 {
            continue;
        }
        let Ok(sym_parsed) = Symbol::parse(sym_str) else {
            continue;
        };
        let canonical = sym_parsed.canonical();
        let q_meta = quote_lookup.get(&canonical).copied();
        let ind_sw1 = scan_out
            .clist_meta
            .get(&canonical)
            .map(|m| m.industry_sw1.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "UNCLASSIFIED".to_string());
        let min_hist = 10usize.min(bars.len().saturating_sub(6));
        for t in min_hist..bars.len().saturating_sub(5) {
            let cur_bar: &SimpleBar = &bars[t];
            let prev_bar: &SimpleBar = &bars[t - 1];
            let next_bar: &SimpleBar = &bars[t + 1];

            // Exclude unbuyable A-share T+1 one-word limit-up bars (`一字涨停板`)
            let is_one_word_limit_up = next_bar.high <= next_bar.low * 1.0002
                && next_bar.open >= cur_bar.close.max(0.01) * 1.094;
            if is_one_word_limit_up {
                continue;
            }

            let entry_open = next_bar.open.max(0.01);
            let exit_close = bars[t + 5].close.max(0.01);
            let fwd_5d_net_bps =
                ((exit_close - entry_open) / entry_open) * 10_000.0 - DEFAULT_ROUND_TRIP_COST_BPS;

            // Point-In-Time market cap scaling (`shares * close_t`) with zero future price look-ahead!
            let (pit_circ_mv_yi, pit_total_mv_yi) = if let Some(q) = q_meta {
                let scale = cur_bar.close / q.latest_yuan.max(0.01);
                (
                    (q.circ_mv_yi * scale).max(1.0),
                    (q.total_mv_yi * scale).max(1.0),
                )
            } else {
                (60.0, 75.0)
            };

            let hist_slice = &bars[..=t];
            let (ma20, atr14, vol_ma20) = compute_ma_and_atr14(hist_slice);
            let worst_low = bars[t + 1..=t + 5]
                .iter()
                .map(|b| b.low)
                .fold(entry_open, f64::min);
            let atr_safe = atr14.max(entry_open * 0.005);
            let max_adverse_atr_excursion = ((entry_open - worst_low).max(0.0)) / atr_safe;
            let atr_pct_bps = ((atr_safe / entry_open) * 10_000.0).max(40.0);
            let vol_exp = if vol_ma20 > 0.0 {
                cur_bar.volume / vol_ma20
            } else {
                1.0
            };
            let vwap_est = if cur_bar.volume > 0.0 && cur_bar.amount_yuan > 0.0 {
                (cur_bar.amount_yuan / (cur_bar.volume * 100.0)).max(0.01)
            } else {
                (cur_bar.high + cur_bar.low + cur_bar.close) / 3.0
            };
            let synth_quote = RawTencentQuote {
                symbol: sym_parsed.clone(),
                name_zh: String::new(),
                latest_yuan: cur_bar.close,
                prev_close_yuan: prev_bar.close.max(0.01),
                open_yuan: cur_bar.open,
                high_yuan: cur_bar.high,
                low_yuan: cur_bar.low,
                volume_lots: cur_bar.volume,
                amount_wan: cur_bar.amount_yuan / 10_000.0,
                turnover_pct: cur_bar.turnover_pct,
                circ_mv_yi: pit_circ_mv_yi,
                total_mv_yi: pit_total_mv_yi,
                vol_ratio: vol_exp,
                vwap_yuan: vwap_est,
                is_suspended: false,
                raw_ts: cur_bar.time.clone(),
            };
            let vars = build_factor_vars(
                &synth_quote,
                &bars[..t],
                cur_bar.turnover_pct,
                prev_bar.turnover_pct,
                ma20,
                atr14,
                vol_exp,
                0.0,
            );
            let mut rf = BTreeMap::new();
            for (fid, ast) in &parsed_factors {
                rf.insert(fid.clone(), eval_factor_ast(ast, &vars));
            }
            let range = (cur_bar.high - cur_bar.low).max(1e-6);
            let body_top = cur_bar.open.max(cur_bar.close);
            let upper_shadow_ratio = ((cur_bar.high - body_top).max(0.0) / range).clamp(0.0, 1.0);
            let trade_date: String = cur_bar.time.chars().take(10).collect();

            by_date_raw.entry(trade_date.clone()).or_default().push(RawObs {
                trade_date,
                symbol: canonical.clone(),
                industry_sw1: ind_sw1.clone(),
                pit_circ_mv_yi,
                raw_factors: rf,
                fwd_5d_net_bps,
                upper_shadow_ratio,
                turnover_pct: cur_bar.turnover_pct,
                max_adverse_atr_excursion,
                atr_pct_bps,
            });
        }
    }

    let mut equal_weights = BTreeMap::new();
    for (fid, _) in &parsed_factors {
        equal_weights.insert(fid.clone(), 1.0);
    }

    let mut panel = Vec::new();
    for (_date, rows) in by_date_raw {
        if rows.len() < 4 {
            continue;
        }
        let samples: Vec<CrossSectionSample> = rows
            .iter()
            .map(|r| CrossSectionSample {
                symbol: r.symbol.clone(),
                industry_sw1: r.industry_sw1.clone(),
                circ_mv_yi: r.pit_circ_mv_yi,
                raw_factors: r.raw_factors.clone(),
            })
            .collect();
        let scored = neutralize_and_score_cross_section(&samples, &equal_weights);

        for (r, sc) in rows.into_iter().zip(scored.into_iter()) {
            panel.push(CrossSectionalFactorObservation {
                trade_date: r.trade_date,
                symbol: r.symbol,
                factor_z: sc.factor_z_scores,
                forward_5d_net_ret_bps: r.fwd_5d_net_bps,
                upper_shadow_ratio: r.upper_shadow_ratio,
                turnover_pct: r.turnover_pct,
                max_adverse_atr_excursion: r.max_adverse_atr_excursion,
                atr_pct_bps: r.atr_pct_bps,
            });
        }
    }
    (panel, ast_complexity)
}
