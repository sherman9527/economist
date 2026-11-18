use serde::{Deserialize, Serialize};
use std::fmt;

/// Exact integer monetary unit in RMB Fen (1 Yuan = 100 Fen, 100W = 100_000_000 Fen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Fen(pub i64);

impl Fen {
    pub const ZERO: Fen = Fen(0);
    pub const INITIAL_CAPITAL_100W: Fen = Fen(100_000_000);

    pub fn from_yuan(yuan: f64) -> Self {
        if !yuan.is_finite() {
            return Fen::ZERO;
        }
        Fen((yuan * 100.0).round() as i64)
    }

    pub fn to_yuan(self) -> f64 {
        (self.0 as f64) / 100.0
    }

    pub fn checked_add(self, rhs: Fen) -> Option<Fen> {
        self.0.checked_add(rhs.0).map(Fen)
    }

    pub fn checked_sub(self, rhs: Fen) -> Option<Fen> {
        self.0.checked_sub(rhs.0).map(Fen)
    }
}

/// Exact basis points (1 bp = 0.01% = 1 / 10_000).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Bps(pub i32);

impl Bps {
    pub fn apply_fen(self, notional: Fen) -> Fen {
        let prod = (notional.0 as i128) * (self.0 as i128);
        let rounded = if prod >= 0 {
            (prod + 5_000) / 10_000
        } else {
            (prod - 5_000) / 10_000
        };
        Fen(rounded as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExchangeCode {
    Sh,
    Sz,
    Bj,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BoardKind {
    Main,
    Chinext,
    Star,
    Bse,
}

impl BoardKind {
    pub fn limit_pct(&self, is_st: bool) -> f64 {
        match self {
            BoardKind::Bse => 0.30,
            BoardKind::Chinext | BoardKind::Star => 0.20,
            BoardKind::Main => {
                if is_st {
                    0.05
                } else {
                    0.10
                }
            }
        }
    }

    pub fn min_lot_shares(&self) -> i64 {
        match self {
            BoardKind::Star => 200,
            BoardKind::Main | BoardKind::Chinext | BoardKind::Bse => 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub digits: String,
    pub exchange: ExchangeCode,
    pub board: BoardKind,
}

impl Symbol {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim().to_ascii_uppercase();
        if trimmed.is_empty() {
            return Err("证券代码不能为空".to_string());
        }
        let mut ex_hint: Option<ExchangeCode> = None;
        let mut core = trimmed.as_str();
        if let Some(rest) = core.strip_prefix("SH") {
            ex_hint = Some(ExchangeCode::Sh);
            core = rest;
        } else if let Some(rest) = core.strip_prefix("SZ") {
            ex_hint = Some(ExchangeCode::Sz);
            core = rest;
        } else if let Some(rest) = core.strip_prefix("BJ") {
            ex_hint = Some(ExchangeCode::Bj);
            core = rest;
        } else if let Some(rest) = core.strip_suffix(".SH") {
            ex_hint = Some(ExchangeCode::Sh);
            core = rest;
        } else if let Some(rest) = core.strip_suffix(".SZ") {
            ex_hint = Some(ExchangeCode::Sz);
            core = rest;
        } else if let Some(rest) = core.strip_suffix(".BJ") {
            ex_hint = Some(ExchangeCode::Bj);
            core = rest;
        } else if let Some((_, rhs)) = core.split_once('.') {
            if rhs.len() == 6 && rhs.chars().all(|c| c.is_ascii_digit()) {
                core = rhs;
            }
        }

        if core.len() != 6 || !core.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("无效的6位A股代码: {}", raw));
        }

        let (inferred_ex, board) = if core.starts_with("688") || core.starts_with("689") {
            (ExchangeCode::Sh, BoardKind::Star)
        } else if core.starts_with("600")
            || core.starts_with("601")
            || core.starts_with("603")
            || core.starts_with("605")
        {
            (ExchangeCode::Sh, BoardKind::Main)
        } else if core.starts_with("300") || core.starts_with("301") {
            (ExchangeCode::Sz, BoardKind::Chinext)
        } else if core.starts_with("000")
            || core.starts_with("001")
            || core.starts_with("002")
            || core.starts_with("003")
        {
            (ExchangeCode::Sz, BoardKind::Main)
        } else if core.starts_with("430")
            || core.starts_with("83")
            || core.starts_with("87")
            || core.starts_with("920")
        {
            (ExchangeCode::Bj, BoardKind::Bse)
        } else {
            return Err(format!("非沪深北A股代码段: {}", raw));
        };

        let exchange = ex_hint.unwrap_or(inferred_ex);
        Ok(Self {
            digits: core.to_string(),
            exchange,
            board,
        })
    }

    pub fn canonical(&self) -> String {
        let suffix = match self.exchange {
            ExchangeCode::Sh => "SH",
            ExchangeCode::Sz => "SZ",
            ExchangeCode::Bj => "BJ",
        };
        format!("{}.{}", self.digits, suffix)
    }

    pub fn tencent_code(&self) -> String {
        let prefix = match self.exchange {
            ExchangeCode::Sh => "sh",
            ExchangeCode::Sz => "sz",
            ExchangeCode::Bj => "bj",
        };
        format!("{}{}", prefix, self.digits)
    }

    pub fn eastmoney_secid(&self) -> String {
        let mkt = match self.exchange {
            ExchangeCode::Sh => "1",
            ExchangeCode::Sz | ExchangeCode::Bj => "0",
        };
        format!("{}.{}", mkt, self.digits)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical())
    }
}

pub fn compute_price_limits(prev_close: Fen, board: BoardKind, is_st: bool) -> (Fen, Fen) {
    let base = prev_close.0.max(1);
    let bps: i64 = match (board, is_st) {
        (BoardKind::Bse, _) => 3000,
        (BoardKind::Star | BoardKind::Chinext, _) => 2000,
        (BoardKind::Main, true) => 500,
        (BoardKind::Main, false) => 1000,
    };
    let down = ((base * (10_000 - bps) + 5_000) / 10_000).clamp(1, base);
    let up = ((base * (10_000 + bps) + 5_000) / 10_000).max(base);
    (Fen(down), Fen(up))
}

pub fn is_limit_up_locked(
    latest: Fen,
    prev_close: Fen,
    board: BoardKind,
    is_st: bool,
    guard_pct: f64,
) -> bool {
    if prev_close.0 <= 0 || latest.0 <= 0 {
        return false;
    }
    let (_, up) = compute_price_limits(prev_close, board, is_st);
    let guard_ratio = (guard_pct.max(0.0)) / 100.0;
    let threshold = ((up.0 as f64) * (1.0 - guard_ratio)).floor() as i64;
    latest.0 >= threshold
}

pub fn is_limit_down_locked(
    latest: Fen,
    prev_close: Fen,
    board: BoardKind,
    is_st: bool,
    guard_pct: f64,
) -> bool {
    if prev_close.0 <= 0 || latest.0 <= 0 {
        return false;
    }
    let (down, _) = compute_price_limits(prev_close, board, is_st);
    let guard_ratio = (guard_pct.max(0.0)) / 100.0;
    let threshold = ((down.0 as f64) * (1.0 + guard_ratio)).ceil() as i64;
    latest.0 <= threshold
}

pub fn validate_lot_size(board: BoardKind, shares: i64) -> Result<(), String> {
    let min_shares = board.min_lot_shares();
    if shares < min_shares {
        return Err(format!(
            "委托股数 {} 低于板块最低整手要求 {} 股",
            shares, min_shares
        ));
    }
    if !matches!(board, BoardKind::Star | BoardKind::Bse) && shares % 100 != 0 {
        return Err(format!("委托股数 {} 必须为 100 股整数倍", shares));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeeSchedule {
    pub stamp_duty_sell_bps: f64,
    pub commission_bps: f64,
    pub min_commission_fen: Fen,
    pub slippage_bps: f64,
}

impl Default for FeeSchedule {
    fn default() -> Self {
        Self {
            stamp_duty_sell_bps: 5.0,
            commission_bps: 2.5,
            min_commission_fen: Fen(500),
            slippage_bps: 15.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeCostBreakdown {
    pub fill_price_fen: Fen,
    pub gross_notional_fen: Fen,
    pub slippage_fen: Fen,
    pub commission_fen: Fen,
    pub stamp_duty_fen: Fen,
    pub total_cash_delta_fen: Fen,
}

impl FeeSchedule {
    pub fn compute_buy(&self, quote_price: Fen, shares: i64) -> Result<TradeCostBreakdown, String> {
        if quote_price.0 <= 0 || shares <= 0 {
            return Err("买入报价与股数必须大于 0".to_string());
        }
        let slip_ratio = self.slippage_bps.max(0.0) / 10_000.0;
        let fill_fen = (((quote_price.0 as f64) * (1.0 + slip_ratio)).ceil() as i64).max(quote_price.0);
        let gross = fill_fen
            .checked_mul(shares)
            .ok_or_else(|| "买入成交金额溢出".to_string())?;
        let slippage_fen = Fen((fill_fen - quote_price.0) * shares);
        let raw_comm = ((gross as f64) * (self.commission_bps.max(0.0) / 10_000.0)).round() as i64;
        let comm = raw_comm.max(self.min_commission_fen.0);
        let total = gross + comm;
        Ok(TradeCostBreakdown {
            fill_price_fen: Fen(fill_fen),
            gross_notional_fen: Fen(gross),
            slippage_fen,
            commission_fen: Fen(comm),
            stamp_duty_fen: Fen::ZERO,
            total_cash_delta_fen: Fen(total),
        })
    }

    pub fn compute_sell(&self, quote_price: Fen, shares: i64) -> Result<TradeCostBreakdown, String> {
        if quote_price.0 <= 0 || shares <= 0 {
            return Err("卖出报价与股数必须大于 0".to_string());
        }
        let slip_ratio = self.slippage_bps.max(0.0) / 10_000.0;
        let fill_fen = (((quote_price.0 as f64) * (1.0 - slip_ratio)).floor() as i64)
            .clamp(1, quote_price.0);
        let gross = fill_fen
            .checked_mul(shares)
            .ok_or_else(|| "卖出成交金额溢出".to_string())?;
        let slippage_fen = Fen((quote_price.0 - fill_fen).max(0) * shares);
        let raw_comm = ((gross as f64) * (self.commission_bps.max(0.0) / 10_000.0)).round() as i64;
        let comm = raw_comm.max(self.min_commission_fen.0);
        let stamp = (((gross as f64) * (self.stamp_duty_sell_bps.max(0.0) / 10_000.0)).round() as i64).max(1);
        let net_proceeds = gross - comm - stamp;
        Ok(TradeCostBreakdown {
            fill_price_fen: Fen(fill_fen),
            gross_notional_fen: Fen(gross),
            slippage_fen,
            commission_fen: Fen(comm),
            stamp_duty_fen: Fen(stamp),
            total_cash_delta_fen: Fen(net_proceeds),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn parses_sh_sz_bj_symbols_and_boards() {
        let s1 = Symbol::parse("600519").unwrap();
        assert_eq!(s1.canonical(), "600519.SH");
        assert_eq!(s1.board, BoardKind::Main);

        let s2 = Symbol::parse("sz300308").unwrap();
        assert_eq!(s2.canonical(), "300308.SZ");
        assert_eq!(s2.board, BoardKind::Chinext);

        let s3 = Symbol::parse("688111.SH").unwrap();
        assert_eq!(s3.board, BoardKind::Star);
        assert_eq!(s3.eastmoney_secid(), "1.688111");

        let s4 = Symbol::parse("bj920309").unwrap();
        assert_eq!(s4.canonical(), "920309.BJ");
        assert_eq!(s4.board, BoardKind::Bse);
    }

    #[test]
    fn enforces_star_200_share_minimum_and_1_share_step() {
        assert!(validate_lot_size(BoardKind::Main, 100).is_ok());
        assert!(validate_lot_size(BoardKind::Star, 100).is_err());
        assert!(validate_lot_size(BoardKind::Star, 199).is_err());
        assert!(validate_lot_size(BoardKind::Star, 200).is_ok());
        assert!(validate_lot_size(BoardKind::Star, 235).is_ok());
        assert!(validate_lot_size(BoardKind::Star, 300).is_ok());
        assert!(validate_lot_size(BoardKind::Chinext, 150).is_err());
    }

    #[test]
    fn computes_realistic_buy_and_sell_friction() {
        let fees = FeeSchedule::default();
        let buy = fees.compute_buy(Fen::from_yuan(20.0), 1000).unwrap();
        assert!(buy.fill_price_fen > Fen::from_yuan(20.0));
        assert_eq!(buy.stamp_duty_fen, Fen::ZERO);
        assert!(buy.commission_fen >= Fen(500));

        let sell = fees.compute_sell(Fen::from_yuan(20.0), 1000).unwrap();
        assert!(sell.fill_price_fen < Fen::from_yuan(20.0));
        assert!(sell.stamp_duty_fen > Fen::ZERO);
        assert!(sell.commission_fen >= Fen(500));
    }

    proptest! {
        #[test]
        fn prop_price_limits_bracket_prev_close(
            prev_fen in 100_i64..500_000_i64,
            board_idx in 0_usize..4,
            is_st in any::<bool>(),
        ) {
            let board = [BoardKind::Main, BoardKind::Chinext, BoardKind::Star, BoardKind::Bse][board_idx];
            let prev = Fen(prev_fen);
            let (down, up) = compute_price_limits(prev, board, is_st);
            prop_assert!(down.0 > 0);
            prop_assert!(down <= prev);
            prop_assert!(up >= prev);
        }

        #[test]
        fn prop_fee_invariants_hold(
            price_fen in 200_i64..100_000_i64,
            lots in 2_i64..200_i64,
        ) {
            let shares = lots * 100;
            let schedule = FeeSchedule::default();
            let buy = schedule.compute_buy(Fen(price_fen), shares).unwrap();
            let sell = schedule.compute_sell(Fen(price_fen), shares).unwrap();

            prop_assert!(buy.fill_price_fen.0 >= price_fen);
            prop_assert_eq!(buy.stamp_duty_fen, Fen::ZERO);
            prop_assert!(buy.commission_fen >= Fen(500));
            prop_assert_eq!(buy.total_cash_delta_fen.0, buy.gross_notional_fen.0 + buy.commission_fen.0);

            prop_assert!(sell.fill_price_fen.0 <= price_fen);
            prop_assert!(sell.stamp_duty_fen.0 > 0);
            prop_assert!(sell.commission_fen >= Fen(500));
            prop_assert_eq!(
                sell.total_cash_delta_fen.0,
                sell.gross_notional_fen.0 - sell.commission_fen.0 - sell.stamp_duty_fen.0
            );
        }
    }
}
