use crate::domain::trading_calendar::is_ashare_trading_day;
use crate::domain::types::Fen;
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Realistic A-share small-cap round-trip friction:
/// 5 bps sell stamp duty + 5 bps two-way commission + 15 bps impact slippage = 25 bps.
pub const DEFAULT_ROUND_TRIP_COST_BPS: f64 = 25.0;
pub const WATCHLIST_HORIZON_TRADING_DAYS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchlistOutcome {
    Tracking,
    HitTarget,
    HitStop,
    Settled5dWin,
    Settled5dLoss,
}

impl WatchlistOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tracking => "TRACKING",
            Self::HitTarget => "HIT_TARGET",
            Self::HitStop => "HIT_STOP",
            Self::Settled5dWin => "SETTLED_5D_WIN",
            Self::Settled5dLoss => "SETTLED_5D_LOSS",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s.trim() {
            "HIT_TARGET" => Self::HitTarget,
            "HIT_STOP" => Self::HitStop,
            "SETTLED_5D_WIN" => Self::Settled5dWin,
            "SETTLED_5D_LOSS" => Self::Settled5dLoss,
            _ => Self::Tracking,
        }
    }

    pub fn is_settled(&self) -> bool {
        !matches!(self, Self::Tracking)
    }

    pub fn is_win(&self) -> bool {
        matches!(self, Self::HitTarget | Self::Settled5dWin)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistItemState {
    pub symbol: String,
    pub name_zh: String,
    pub board: String,
    pub industry_sw1: String,
    pub added_trade_date: String,
    pub added_at_iso: String,
    pub entry_price_fen: Fen,
    pub stop_loss_price_fen: Fen,
    pub target_price_fen: Fen,
    pub conviction_score: f64,
    pub source_decision: String,
    pub version_id_at_entry: String,
    pub rationale_zh: String,
    pub factor_z_scores: BTreeMap<String, f64>,
    pub latest_price_fen: Fen,
    pub peak_price_fen: Fen,
    pub trough_price_fen: Fen,
    pub holding_trading_days: u32,
    pub net_return_bps: f64,
    pub mfe_bps: f64,
    pub mae_bps: f64,
    pub outcome: WatchlistOutcome,
    pub settled_trade_date: Option<String>,
    pub settled_price_fen: Option<Fen>,
}

impl WatchlistItemState {
    #[allow(clippy::too_many_arguments)]
    pub fn new_tracking(
        symbol: &str,
        name_zh: &str,
        board: &str,
        industry_sw1: &str,
        added_trade_date: &str,
        added_at_iso: &str,
        entry_price_fen: Fen,
        stop_loss_price_fen: Fen,
        target_price_fen: Fen,
        conviction_score: f64,
        source_decision: &str,
        version_id_at_entry: &str,
        rationale_zh: &str,
        factor_z_scores: BTreeMap<String, f64>,
    ) -> Self {
        let safe_entry = Fen(entry_price_fen.0.clamp(2, 1_000_000_000));
        let safe_stop = Fen(stop_loss_price_fen.0.clamp(1, safe_entry.0 - 1));
        let safe_target = Fen(target_price_fen
            .0
            .max(safe_entry.0.saturating_add(1))
            .min(2_000_000_000));
        Self {
            symbol: symbol.to_string(),
            name_zh: name_zh.to_string(),
            board: board.to_string(),
            industry_sw1: industry_sw1.to_string(),
            added_trade_date: added_trade_date.to_string(),
            added_at_iso: added_at_iso.to_string(),
            entry_price_fen: safe_entry,
            stop_loss_price_fen: safe_stop,
            target_price_fen: safe_target,
            conviction_score,
            source_decision: source_decision.to_string(),
            version_id_at_entry: version_id_at_entry.to_string(),
            rationale_zh: rationale_zh.to_string(),
            factor_z_scores,
            latest_price_fen: safe_entry,
            peak_price_fen: safe_entry,
            trough_price_fen: safe_entry,
            holding_trading_days: 0,
            net_return_bps: -DEFAULT_ROUND_TRIP_COST_BPS,
            mfe_bps: 0.0,
            mae_bps: 0.0,
            outcome: WatchlistOutcome::Tracking,
            settled_trade_date: None,
            settled_price_fen: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistWinRateSummary {
    pub total_count: usize,
    pub active_tracking_count: usize,
    pub settled_count: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub target_hit_count: usize,
    pub stop_hit_count: usize,
    pub settled_win_rate_pct: f64,
    pub live_positive_rate_pct: f64,
    pub wilson_low_pct: f64,
    pub wilson_high_pct: f64,
    pub low_sample_warning: bool,
    pub profit_factor: f64,
    pub expectancy_bps: f64,
    pub high_conviction_win_rate_pct: f64,
    pub baseline_conviction_win_rate_pct: f64,
}

pub fn count_ashare_trading_days_after(start_date_str: &str, end_date_str: &str) -> u32 {
    let s_prefix: String = start_date_str.chars().take(10).collect();
    let e_prefix: String = end_date_str.chars().take(10).collect();
    let (Ok(start), Ok(end)) = (
        NaiveDate::parse_from_str(&s_prefix, "%Y-%m-%d"),
        NaiveDate::parse_from_str(&e_prefix, "%Y-%m-%d"),
    ) else {
        return 0;
    };
    if end <= start {
        return 0;
    }
    let mut count = 0u32;
    let mut cur = start + Duration::days(1);
    let max_guard = start + Duration::days(730);
    while cur <= end && cur <= max_guard {
        if is_ashare_trading_day(cur) {
            count += 1;
        }
        cur += Duration::days(1);
    }
    count
}

fn compute_net_return_bps(entry: Fen, exit_or_mark: Fen) -> f64 {
    let base = entry.0.max(1) as f64;
    let gross_bps = ((exit_or_mark.0 as f64 - base) / base) * 10_000.0;
    gross_bps - DEFAULT_ROUND_TRIP_COST_BPS
}

/// Evaluates a single bar / live quote against a Watchlist entry while strictly enforcing
/// mainland China A-share `T+1` settlement (`holding_trading_days >= 1` required to exit).
pub fn evaluate_watchlist_entry(
    item: &mut WatchlistItemState,
    bar_trade_date: &str,
    close_or_last_fen: Fen,
    high_fen: Fen,
    low_fen: Fen,
) {
    let valid_last = Fen(close_or_last_fen.0.max(1));
    let valid_high = Fen(high_fen.0.max(valid_last.0));
    let valid_low = Fen(if low_fen.0 > 0 {
        low_fen.0.min(valid_last.0)
    } else {
        valid_last.0
    });

    if item.outcome.is_settled() {
        // Once settled, keep settled return frozen; still update latest mark for reference.
        item.latest_price_fen = valid_last;
        return;
    }

    let elapsed = count_ashare_trading_days_after(&item.added_trade_date, bar_trade_date);
    item.holding_trading_days = item.holding_trading_days.max(elapsed);
    item.latest_price_fen = valid_last;
    item.peak_price_fen = Fen(item.peak_price_fen.0.max(valid_high.0));
    item.trough_price_fen = Fen(item.trough_price_fen.0.min(valid_low.0));

    let entry_f = item.entry_price_fen.0.max(1) as f64;
    item.mfe_bps = (((item.peak_price_fen.0 as f64 - entry_f) / entry_f) * 10_000.0).max(0.0);
    item.mae_bps = (((item.trough_price_fen.0 as f64 - entry_f) / entry_f) * 10_000.0).min(0.0);
    item.net_return_bps = compute_net_return_bps(item.entry_price_fen, valid_last);

    // Strict A-share T+1 rule: cannot exit on T+0 (elapsed == 0)!
    if item.holding_trading_days == 0 {
        return;
    }

    // Conservative execution order on daily/30m bar: check stop-loss BEFORE target,
    // and account for overnight gap-downs (fill <= valid_high) / gap-ups (fill >= valid_low)!
    if valid_low.0 <= item.stop_loss_price_fen.0 {
        let fill = Fen(item.stop_loss_price_fen.0.min(valid_high.0).max(1));
        item.outcome = WatchlistOutcome::HitStop;
        item.settled_price_fen = Some(fill);
        item.settled_trade_date = Some(bar_trade_date.chars().take(10).collect());
        item.net_return_bps = compute_net_return_bps(item.entry_price_fen, fill);
    } else if valid_high.0 >= item.target_price_fen.0 {
        let fill = Fen(item.target_price_fen.0.max(valid_low.0));
        item.outcome = WatchlistOutcome::HitTarget;
        item.settled_price_fen = Some(fill);
        item.settled_trade_date = Some(bar_trade_date.chars().take(10).collect());
        item.net_return_bps = compute_net_return_bps(item.entry_price_fen, fill);
    } else if item.holding_trading_days >= WATCHLIST_HORIZON_TRADING_DAYS {
        let net_bps = compute_net_return_bps(item.entry_price_fen, valid_last);
        item.net_return_bps = net_bps;
        item.settled_price_fen = Some(valid_last);
        item.settled_trade_date = Some(bar_trade_date.chars().take(10).collect());
        item.outcome = if net_bps > 0.0 {
            WatchlistOutcome::Settled5dWin
        } else {
            WatchlistOutcome::Settled5dLoss
        };
    }
}

/// Exact 95% Wilson score confidence interval `[low_pct, high_pct]` for binomial win rate.
pub fn wilson_95_interval_pct(wins: usize, n: usize) -> (f64, f64) {
    if n == 0 {
        return (0.0, 0.0);
    }
    let z = 1.959_963_984_540_054_f64;
    let z2 = z * z;
    let nf = n as f64;
    let phat = (wins.min(n) as f64) / nf;
    let denom = 1.0 + z2 / nf;
    let center = (phat + z2 / (2.0 * nf)) / denom;
    let half = (z / denom) * ((phat * (1.0 - phat) / nf + z2 / (4.0 * nf * nf)).max(0.0)).sqrt();
    let low = ((center - half).clamp(0.0, 1.0)) * 100.0;
    let high = ((center + half).clamp(0.0, 1.0)) * 100.0;
    (low, high)
}

pub fn summarize_watchlist_win_rate(items: &[WatchlistItemState]) -> WatchlistWinRateSummary {
    let total_count = items.len();
    let settled: Vec<&WatchlistItemState> = items.iter().filter(|i| i.outcome.is_settled()).collect();
    let active: Vec<&WatchlistItemState> =
        items.iter().filter(|i| !i.outcome.is_settled()).collect();

    let settled_count = settled.len();
    let active_tracking_count = active.len();
    let win_count = settled.iter().filter(|i| i.outcome.is_win()).count();
    let loss_count = settled_count.saturating_sub(win_count);
    let target_hit_count = settled
        .iter()
        .filter(|i| i.outcome == WatchlistOutcome::HitTarget)
        .count();
    let stop_hit_count = settled
        .iter()
        .filter(|i| i.outcome == WatchlistOutcome::HitStop)
        .count();

    // If settled samples exist, primary win rate uses strictly settled OOS samples;
    // otherwise evaluate live mark-to-market net return > 0 with explicit low_sample_warning.
    let (eval_wins, eval_n, settled_win_rate_pct) = if settled_count > 0 {
        let wr = (win_count as f64 / settled_count as f64) * 100.0;
        (win_count, settled_count, wr)
    } else if total_count > 0 {
        let live_wins = items.iter().filter(|i| i.net_return_bps > 0.0).count();
        let wr = (live_wins as f64 / total_count as f64) * 100.0;
        (live_wins, total_count, wr)
    } else {
        (0, 0, 0.0)
    };

    let live_positive_rate_pct = if total_count > 0 {
        (items.iter().filter(|i| i.net_return_bps > 0.0).count() as f64 / total_count as f64)
            * 100.0
    } else {
        0.0
    };

    let (wilson_low_pct, wilson_high_pct) = wilson_95_interval_pct(eval_wins, eval_n);

    let eval_slice: Vec<&WatchlistItemState> = if settled_count > 0 {
        settled
    } else {
        items.iter().collect()
    };
    let gross_gains: f64 = eval_slice
        .iter()
        .filter(|i| i.net_return_bps > 0.0)
        .map(|i| i.net_return_bps)
        .sum();
    let gross_losses_abs: f64 = eval_slice
        .iter()
        .filter(|i| i.net_return_bps < 0.0)
        .map(|i| i.net_return_bps.abs())
        .sum();
    let profit_factor = if gross_losses_abs > 1e-6 {
        (gross_gains / gross_losses_abs).min(99.9)
    } else if gross_gains > 0.0 {
        9.99
    } else {
        0.0
    };
    let expectancy_bps = if eval_slice.is_empty() {
        0.0
    } else {
        eval_slice.iter().map(|i| i.net_return_bps).sum::<f64>() / (eval_slice.len() as f64)
    };

    let high_bucket: Vec<&&WatchlistItemState> = eval_slice
        .iter()
        .filter(|i| i.conviction_score >= 68.0)
        .collect();
    let low_bucket: Vec<&&WatchlistItemState> = eval_slice
        .iter()
        .filter(|i| i.conviction_score < 68.0)
        .collect();
    let bucket_wr = |b: &[&&WatchlistItemState]| -> f64 {
        if b.is_empty() {
            0.0
        } else {
            let w = b
                .iter()
                .filter(|i| {
                    if i.outcome.is_settled() {
                        i.outcome.is_win()
                    } else {
                        i.net_return_bps > 0.0
                    }
                })
                .count();
            (w as f64 / b.len() as f64) * 100.0
        }
    };

    WatchlistWinRateSummary {
        total_count,
        active_tracking_count,
        settled_count,
        win_count,
        loss_count,
        target_hit_count,
        stop_hit_count,
        settled_win_rate_pct,
        live_positive_rate_pct,
        wilson_low_pct,
        wilson_high_pct,
        low_sample_warning: settled_count < 20,
        profit_factor,
        expectancy_bps,
        high_conviction_win_rate_pct: bucket_wr(&high_bucket),
        baseline_conviction_win_rate_pct: bucket_wr(&low_bucket),
    }
}
