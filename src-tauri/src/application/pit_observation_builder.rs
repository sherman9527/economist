use crate::application::scan_factors::{build_factor_vars, compute_ma_and_atr14};
use crate::application::scan_pipeline::ScanExecutionOutput;
use crate::domain::anti_overfit::CrossSectionalFactorObservation;
use crate::domain::factor_engine::{eval_factor_ast, parse_factor_expr, winsorize_mad, ExprNode};
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
        raw_factors: BTreeMap<String, f64>,
        fwd_5d_net_bps: f64,
        upper_shadow_ratio: f64,
    }
    let mut by_date_raw: BTreeMap<String, Vec<RawObs>> = BTreeMap::new();

    for (sym_str, bars) in &scan_out.kline_cache_1d {
        if bars.len() < 18 {
            continue;
        }
        let Ok(sym_parsed) = Symbol::parse(sym_str) else {
            continue;
        };
        let min_hist = 10usize.min(bars.len().saturating_sub(6));
        for t in min_hist..bars.len().saturating_sub(5) {
            let cur_bar: &SimpleBar = &bars[t];
            let prev_bar: &SimpleBar = &bars[t - 1];
            let entry_open = bars[t + 1].open.max(0.01);
            let exit_close = bars[t + 5].close.max(0.01);
            let fwd_5d_net_bps =
                ((exit_close - entry_open) / entry_open) * 10_000.0 - DEFAULT_ROUND_TRIP_COST_BPS;

            let hist_slice = &bars[..=t];
            let (ma20, atr14, vol_ma20) = compute_ma_and_atr14(hist_slice);
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
                circ_mv_yi: 60.0,
                total_mv_yi: 75.0,
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
                symbol: sym_str.clone(),
                raw_factors: rf,
                fwd_5d_net_bps,
                upper_shadow_ratio,
            });
        }
    }

    let mut panel = Vec::new();
    for (_date, rows) in by_date_raw {
        if rows.len() < 4 {
            continue;
        }
        let mut z_by_fid: HashMap<String, Vec<f64>> = HashMap::new();
        for (fid, _) in &parsed_factors {
            let raw_vec: Vec<f64> = rows
                .iter()
                .map(|r| r.raw_factors.get(fid).copied().unwrap_or(0.0))
                .collect();
            let clipped = winsorize_mad(&raw_vec, 3.0);
            let mean = clipped.iter().sum::<f64>() / (clipped.len() as f64);
            let var = clipped
                .iter()
                .map(|v| (v - mean) * (v - mean))
                .sum::<f64>()
                / ((clipped.len() - 1).max(1) as f64);
            let sd = var.sqrt().max(1e-8);
            let zs: Vec<f64> = clipped.into_iter().map(|v| (v - mean) / sd).collect();
            z_by_fid.insert(fid.clone(), zs);
        }

        for (idx, r) in rows.into_iter().enumerate() {
            let mut fz = BTreeMap::new();
            for (fid, _) in &parsed_factors {
                let z = z_by_fid
                    .get(fid)
                    .and_then(|v| v.get(idx))
                    .copied()
                    .unwrap_or(0.0);
                fz.insert(fid.clone(), z);
            }
            panel.push(CrossSectionalFactorObservation {
                trade_date: r.trade_date,
                symbol: r.symbol,
                factor_z: fz,
                forward_5d_net_ret_bps: r.fwd_5d_net_bps,
                upper_shadow_ratio: r.upper_shadow_ratio,
            });
        }
    }
    (panel, ast_complexity)
}
