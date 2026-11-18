use crate::domain::types::{BoardKind, Fen};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketSessionState {
    PreOpen,
    Trading,
    LunchBreak,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyGateParams {
    pub max_total_mv_yi: f64,
    pub min_turnover_pct: f64,
    pub consecutive_days: u32,
    pub min_vol_ratio_t1_vs_ma20: f64,
    pub min_vol_ratio_t0_vs_t1: f64,
    pub min_listing_days: u32,
    pub limit_up_guard_pct: f64,
    pub min_intraday_projected_turnover_pct: f64,
    pub min_intraday_realized_turnover_pct: f64,
    pub u_curve_weights: Vec<(u16, f64)>,
}

impl Default for StrategyGateParams {
    fn default() -> Self {
        Self {
            max_total_mv_yi: 200.0,
            min_turnover_pct: 5.0,
            consecutive_days: 2,
            min_vol_ratio_t1_vs_ma20: 1.15,
            min_vol_ratio_t0_vs_t1: 1.02,
            min_listing_days: 60,
            limit_up_guard_pct: 0.20,
            min_intraday_projected_turnover_pct: 5.5,
            min_intraday_realized_turnover_pct: 2.5,
            u_curve_weights: vec![
                (30, 0.24),
                (60, 0.38),
                (90, 0.49),
                (120, 0.58),
                (150, 0.69),
                (180, 0.80),
                (210, 0.89),
                (240, 1.00),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrategyCandidateInput {
    pub name_zh: String,
    pub board: BoardKind,
    pub latest_price_fen: Fen,
    pub prev_close_fen: Fen,
    pub is_suspended: bool,
    pub listing_days: u32,
    pub total_mv_yi: f64,
    pub turnover_t0_pct: f64,
    pub turnover_t1_pct: f64,
    pub vol_t0: f64,
    pub vol_t1: f64,
    pub vol_ma20_t1: f64,
    pub session: MarketSessionState,
    pub elapsed_trading_minutes: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrategyGateVerdict {
    pub tradable: bool,
    pub under_200yi: bool,
    pub consecutive_turnover_vol_passed: bool,
    pub projected_turnover_pct: f64,
    pub vol_expansion_ratio: f64,
    pub reject_reason_zh: Option<String>,
}

pub fn detect_market_session_cst(weekday_mon1_sun7: u8, hour: u8, minute: u8) -> MarketSessionState {
    if weekday_mon1_sun7 == 0 || weekday_mon1_sun7 >= 6 {
        return MarketSessionState::Closed;
    }
    let total_min = (hour as u16) * 60 + (minute as u16);
    if (555..570).contains(&total_min) {
        MarketSessionState::PreOpen
    } else if (570..=690).contains(&total_min) || (780..=900).contains(&total_min) {
        MarketSessionState::Trading
    } else if (691..780).contains(&total_min) {
        MarketSessionState::LunchBreak
    } else {
        MarketSessionState::Closed
    }
}

pub fn elapsed_trading_minutes_cst(hour: u8, minute: u8) -> u16 {
    let total_min = (hour as u16) * 60 + (minute as u16);
    if total_min <= 570 {
        0
    } else if total_min <= 690 {
        total_min - 570
    } else if total_min < 780 {
        120
    } else if total_min <= 900 {
        120 + (total_min - 780)
    } else {
        240
    }
}

pub fn interpolate_u_curve_weight(elapsed_min: u16, weights: &[(u16, f64)]) -> f64 {
    if weights.is_empty() || elapsed_min >= 240 {
        return 1.0;
    }
    let clamped = elapsed_min.max(15);
    let mut prev_t = 0_u16;
    let mut prev_w = 0.0_f64;
    for &(t, w) in weights {
        if clamped <= t {
            let span = (t.saturating_sub(prev_t)).max(1) as f64;
            let frac = (clamped.saturating_sub(prev_t)) as f64 / span;
            return (prev_w + frac * (w - prev_w)).clamp(0.12, 1.0);
        }
        prev_t = t;
        prev_w = w;
    }
    1.0
}

pub fn is_st_or_delisting_name(name_zh: &str) -> bool {
    let upper = name_zh.trim().to_ascii_uppercase();
    upper.contains("ST") || name_zh.contains("退")
}

pub fn evaluate_strategy_gate(
    input: &StrategyCandidateInput,
    params: &StrategyGateParams,
) -> StrategyGateVerdict {
    use crate::domain::types::is_limit_up_locked;

    let is_st = is_st_or_delisting_name(&input.name_zh);
    let limit_locked = is_limit_up_locked(
        input.latest_price_fen,
        input.prev_close_fen,
        input.board,
        is_st,
        params.limit_up_guard_pct,
    );

    let intraday_mode = matches!(
        input.session,
        MarketSessionState::Trading | MarketSessionState::LunchBreak
    ) && input.elapsed_trading_minutes < 240;

    let u_weight = if intraday_mode {
        interpolate_u_curve_weight(input.elapsed_trading_minutes.max(15), &params.u_curve_weights)
    } else {
        1.0
    };
    let projected_turnover = (input.turnover_t0_pct / u_weight).max(input.turnover_t0_pct);
    let projected_vol_t0 = input.vol_t0 / u_weight;
    let vol_expansion_ratio = if input.vol_ma20_t1 > 0.0 {
        projected_vol_t0 / input.vol_ma20_t1
    } else if input.vol_t1 > 0.0 {
        projected_vol_t0 / input.vol_t1
    } else {
        1.0
    };

    if input.is_suspended || input.latest_price_fen.0 <= 0 {
        return StrategyGateVerdict {
            tradable: false,
            under_200yi: false,
            consecutive_turnover_vol_passed: false,
            projected_turnover_pct: projected_turnover,
            vol_expansion_ratio,
            reject_reason_zh: Some("停牌或无有效报价".to_string()),
        };
    }
    if is_st {
        return StrategyGateVerdict {
            tradable: false,
            under_200yi: false,
            consecutive_turnover_vol_passed: false,
            projected_turnover_pct: projected_turnover,
            vol_expansion_ratio,
            reject_reason_zh: Some("剔除ST/*ST/退市风险警示标的".to_string()),
        };
    }
    if input.listing_days < params.min_listing_days {
        return StrategyGateVerdict {
            tradable: false,
            under_200yi: false,
            consecutive_turnover_vol_passed: false,
            projected_turnover_pct: projected_turnover,
            vol_expansion_ratio,
            reject_reason_zh: Some(format!("上市未满{}个交易日", params.min_listing_days)),
        };
    }
    if limit_locked {
        return StrategyGateVerdict {
            tradable: false,
            under_200yi: false,
            consecutive_turnover_vol_passed: false,
            projected_turnover_pct: projected_turnover,
            vol_expansion_ratio,
            reject_reason_zh: Some("触及涨停锁死不可买入".to_string()),
        };
    }

    if !(input.total_mv_yi > 0.0 && input.total_mv_yi < params.max_total_mv_yi) {
        return StrategyGateVerdict {
            tradable: true,
            under_200yi: false,
            consecutive_turnover_vol_passed: false,
            projected_turnover_pct: projected_turnover,
            vol_expansion_ratio,
            reject_reason_zh: Some(format!(
                "总市值 {:.2} 亿超出 < {:.0} 亿硬门槛",
                input.total_mv_yi, params.max_total_mv_yi
            )),
        };
    }

    let t1_vol_ok = if input.vol_ma20_t1 > 0.0 {
        input.vol_t1 >= params.min_vol_ratio_t1_vs_ma20 * input.vol_ma20_t1
    } else {
        true
    };
    let t1_ok = input.turnover_t1_pct > params.min_turnover_pct && t1_vol_ok;

    let t0_vol_ok = if input.vol_t1 > 0.0 {
        projected_vol_t0 >= params.min_vol_ratio_t0_vs_t1 * input.vol_t1
    } else {
        true
    };
    let t0_ok = if intraday_mode {
        input.turnover_t0_pct >= params.min_intraday_realized_turnover_pct
            && projected_turnover >= params.min_intraday_projected_turnover_pct
            && t0_vol_ok
    } else {
        input.turnover_t0_pct > params.min_turnover_pct && t0_vol_ok
    };

    if !(t1_ok && t0_ok) {
        return StrategyGateVerdict {
            tradable: true,
            under_200yi: true,
            consecutive_turnover_vol_passed: false,
            projected_turnover_pct: projected_turnover,
            vol_expansion_ratio,
            reject_reason_zh: Some(
                "未满足连续2日换手率>5%且成交持续放量约束".to_string(),
            ),
        };
    }

    StrategyGateVerdict {
        tradable: true,
        under_200yi: true,
        consecutive_turnover_vol_passed: true,
        projected_turnover_pct: projected_turnover,
        vol_expansion_ratio,
        reject_reason_zh: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn sample_valid_eod_candidate() -> StrategyCandidateInput {
        StrategyCandidateInput {
            name_zh: "智算科技".to_string(),
            board: BoardKind::Chinext,
            latest_price_fen: Fen::from_yuan(24.50),
            prev_close_fen: Fen::from_yuan(23.00),
            is_suspended: false,
            listing_days: 320,
            total_mv_yi: 86.4,
            turnover_t0_pct: 8.2,
            turnover_t1_pct: 6.4,
            vol_t0: 185_000.0,
            vol_t1: 150_000.0,
            vol_ma20_t1: 110_000.0,
            session: MarketSessionState::Closed,
            elapsed_trading_minutes: 240,
        }
    }

    #[test]
    fn detects_cst_sessions_accurately() {
        assert_eq!(detect_market_session_cst(3, 9, 20), MarketSessionState::PreOpen);
        assert_eq!(detect_market_session_cst(3, 10, 0), MarketSessionState::Trading);
        assert_eq!(detect_market_session_cst(3, 12, 0), MarketSessionState::LunchBreak);
        assert_eq!(detect_market_session_cst(3, 14, 30), MarketSessionState::Trading);
        assert_eq!(detect_market_session_cst(3, 15, 5), MarketSessionState::Closed);
        assert_eq!(detect_market_session_cst(6, 10, 0), MarketSessionState::Closed);
    }

    #[test]
    fn passes_valid_smallcap_two_day_turnover_expansion() {
        let params = StrategyGateParams::default();
        let cand = sample_valid_eod_candidate();
        let verdict = evaluate_strategy_gate(&cand, &params);
        assert!(verdict.tradable);
        assert!(verdict.under_200yi);
        assert!(verdict.consecutive_turnover_vol_passed);
        assert!(verdict.reject_reason_zh.is_none());
    }

    #[test]
    fn projects_intraday_turnover_via_u_curve() {
        let params = StrategyGateParams::default();
        let mut cand = sample_valid_eod_candidate();
        cand.session = MarketSessionState::Trading;
        cand.elapsed_trading_minutes = 60; // weight = 0.38
        cand.turnover_t0_pct = 2.8; // projected = 2.8 / 0.38 = 7.37% >= 5.5%
        cand.vol_t0 = 60_000.0; // projected vol = 60_000 / 0.38 = 157_894 >= 1.02 * 150_000
        let verdict = evaluate_strategy_gate(&cand, &params);
        assert!(verdict.consecutive_turnover_vol_passed);
        assert!(verdict.projected_turnover_pct > 7.0);
    }

    proptest! {
        #[test]
        fn prop_rejects_any_cap_at_or_above_200yi_or_low_t1_turnover(
            mv_yi in 200.0_f64..5000.0_f64,
            low_tr in 0.0_f64..5.0_f64,
        ) {
            let params = StrategyGateParams::default();
            let mut over_cap = sample_valid_eod_candidate();
            over_cap.total_mv_yi = mv_yi;
            let v1 = evaluate_strategy_gate(&over_cap, &params);
            prop_assert!(!v1.under_200yi);
            prop_assert!(!v1.consecutive_turnover_vol_passed);

            let mut low_t1 = sample_valid_eod_candidate();
            low_t1.turnover_t1_pct = low_tr;
            let v2 = evaluate_strategy_gate(&low_t1, &params);
            prop_assert!(!v2.consecutive_turnover_vol_passed);
        }
    }
}
