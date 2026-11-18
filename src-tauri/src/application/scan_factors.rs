use crate::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};
use std::collections::BTreeMap;

pub fn compute_ma_and_atr14(bars: &[SimpleBar]) -> (f64, f64, f64) {
    if bars.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let n = bars.len();
    let ma_slice = &bars[n.saturating_sub(20)..];
    let ma20 = ma_slice.iter().map(|b| b.close).sum::<f64>() / (ma_slice.len() as f64);
    let vol_ma20 = ma_slice.iter().map(|b| b.volume).sum::<f64>() / (ma_slice.len() as f64);

    let mut trs = Vec::with_capacity(14);
    let start = n.saturating_sub(14).max(1);
    for i in start..n {
        let cur = &bars[i];
        let prev_close = bars[i - 1].close;
        let tr = (cur.high - cur.low)
            .max((cur.high - prev_close).abs())
            .max((cur.low - prev_close).abs());
        trs.push(tr);
    }
    let atr14 = if !trs.is_empty() {
        trs.iter().sum::<f64>() / (trs.len() as f64)
    } else {
        (bars[n - 1].high - bars[n - 1].low).max(bars[n - 1].close * 0.025)
    };
    (ma20, atr14.max(0.02), vol_ma20)
}

#[allow(clippy::too_many_arguments)]
pub fn build_factor_vars(
    quote: &RawTencentQuote,
    completed_bars: &[SimpleBar],
    turnover_t0_effective: f64,
    turnover_t1: f64,
    ma20_price: f64,
    atr14_price: f64,
    vol_expansion_ratio: f64,
    main_net_inflow_wan: f64,
) -> BTreeMap<String, f64> {
    let mut vars = BTreeMap::new();
    let n = completed_bars.len();
    let window_start = n.saturating_sub(20);
    let mut ret_turn_pairs: Vec<(f64, f64)> = Vec::new();
    let mut amihud_vals: Vec<f64> = Vec::new();
    let mut turn_sum = 0.0_f64;
    let mut turn_cnt = 0.0_f64;

    for i in window_start..n {
        let b = &completed_bars[i];
        turn_sum += b.turnover_pct;
        turn_cnt += 1.0;
        if i > 0 && completed_bars[i - 1].close > 0.0 {
            let ret_pct =
                ((b.close - completed_bars[i - 1].close) / completed_bars[i - 1].close) * 100.0;
            ret_turn_pairs.push((b.turnover_pct, ret_pct));
            let amt_yi = (b.amount_yuan / 100_000_000.0).max(0.005);
            amihud_vals.push(ret_pct.abs() / amt_yi);
        }
    }

    if quote.prev_close_yuan > 0.0 && quote.latest_yuan > 0.0 && turnover_t0_effective > 0.0 {
        let live_ret_pct =
            ((quote.latest_yuan - quote.prev_close_yuan) / quote.prev_close_yuan) * 100.0;
        let proj_factor = if quote.turnover_pct > 0.0 {
            (turnover_t0_effective / quote.turnover_pct).clamp(1.0, 12.5)
        } else {
            1.0
        };
        let eff_amt_yi = ((quote.amount_wan * 10_000.0 * proj_factor) / 100_000_000.0).max(0.005);
        ret_turn_pairs.push((turnover_t0_effective, live_ret_pct));
        amihud_vals.push(live_ret_pct.abs() / eff_amt_yi);
    }

    ret_turn_pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let k = (ret_turn_pairs.len() / 3).clamp(1, 5);
    let ret_low_turn_5d = if !ret_turn_pairs.is_empty() {
        ret_turn_pairs.iter().take(k).map(|x| x.1).sum::<f64>() / (k as f64)
    } else {
        0.0
    };
    let ret_high_turn_5d = if !ret_turn_pairs.is_empty() {
        ret_turn_pairs.iter().rev().take(k).map(|x| x.1).sum::<f64>() / (k as f64)
    } else {
        0.0
    };

    let turnover_20d_mean = if turn_cnt > 0.0 {
        turn_sum / turn_cnt
    } else {
        turnover_t1.max(2.0)
    };
    let amihud_20d = if !amihud_vals.is_empty() {
        amihud_vals.iter().sum::<f64>() / (amihud_vals.len() as f64)
    } else {
        0.5
    };
    let amihud_5_slice = &amihud_vals[amihud_vals.len().saturating_sub(5)..];
    let amihud_5d = if !amihud_5_slice.is_empty() {
        amihud_5_slice.iter().sum::<f64>() / (amihud_5_slice.len() as f64)
    } else {
        amihud_20d * 0.85
    };

    let vwap = quote.vwap_yuan.max(0.01);
    let tail30m_ret_pct = ((quote.latest_yuan - vwap) / vwap) * 100.0;
    let main_inflow_ratio_pct = if quote.amount_wan > 0.0 {
        (main_net_inflow_wan / quote.amount_wan) * 100.0
    } else {
        0.0
    };
    let intraday_range_pct = if quote.prev_close_yuan > 0.0 {
        ((quote.high_yuan - quote.low_yuan).max(0.0) / quote.prev_close_yuan) * 100.0
    } else {
        2.0
    };

    vars.insert("ret_high_turn_5d".to_string(), ret_high_turn_5d);
    vars.insert("ret_low_turn_5d".to_string(), ret_low_turn_5d);
    vars.insert(
        "turnover_2d_mean".to_string(),
        0.5 * (turnover_t0_effective + turnover_t1),
    );
    vars.insert("turnover_20d_mean".to_string(), turnover_20d_mean);
    vars.insert("amihud_20d".to_string(), amihud_20d);
    vars.insert("amihud_5d".to_string(), amihud_5d);
    vars.insert("close_price".to_string(), quote.latest_yuan);
    vars.insert("vwap_price".to_string(), vwap);
    vars.insert("tail30m_ret_pct".to_string(), tail30m_ret_pct);
    vars.insert("main_inflow_ratio_pct".to_string(), main_inflow_ratio_pct);
    vars.insert("intraday_range_pct".to_string(), intraday_range_pct);
    vars.insert("ma20_price".to_string(), ma20_price);
    vars.insert("atr14_price".to_string(), atr14_price);
    vars.insert("vol_expansion_ratio".to_string(), vol_expansion_ratio);
    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_ma20_atr14_and_vol_ma20_from_daily_bars() {
        let bars: Vec<SimpleBar> = (0..22)
            .map(|i| SimpleBar {
                time: format!("2026-10-{:02}", i + 1),
                open: 20.0 + (i as f64) * 0.1,
                high: 20.8 + (i as f64) * 0.1,
                low: 19.6 + (i as f64) * 0.1,
                close: 20.2 + (i as f64) * 0.1,
                volume: 100_000.0 + (i as f64) * 1_000.0,
                amount_yuan: 20_000_000.0,
                turnover_pct: 6.2,
            })
            .collect();
        let (ma20, atr14, vol_ma20) = compute_ma_and_atr14(&bars);
        assert!(ma20 > 20.0);
        assert!(atr14 > 0.5);
        assert!(vol_ma20 > 100_000.0);
    }
}
