use crate::domain::types::{BoardKind, FeeSchedule, Fen, Symbol};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_POSITION_SLOTS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRecord {
    pub position_id: String,
    pub slot_no: u8,
    pub symbol: Symbol,
    pub name_zh: String,
    pub industry_sw1: String,
    pub total_qty: i64,
    pub sellable_qty: i64,
    pub locked_t0_qty: i64,
    pub avg_cost_fen: Fen,
    pub mark_price_fen: Fen,
    pub highest_price_fen: Fen,
    pub stop_loss_price_fen: Fen,
    pub entry_rationale_zh: String,
    pub opened_trade_date: String,
    pub last_buy_trade_date: String,
    pub t_plus1_unlock_date: String,
    pub opened_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRecord {
    pub order_id: String,
    pub rec_id: Option<String>,
    pub symbol: Symbol,
    pub name_zh: String,
    pub side: String,
    pub trigger_type: String,
    pub status: String,
    pub lot_qty: i64,
    pub entry_price_fen: Fen,
    pub fill_price_fen: Fen,
    pub stop_loss_price_fen: Fen,
    pub commission_fen: Fen,
    pub stamp_duty_fen: Fen,
    pub slippage_fen: Fen,
    pub rationale_zh: String,
    pub trade_date: String,
    pub t_plus1_unlock_date: String,
    pub created_at_iso: String,
}

#[derive(Debug, Clone)]
pub struct BuyOrderCommand {
    pub order_id: String,
    pub rec_id: Option<String>,
    pub symbol: Symbol,
    pub name_zh: String,
    pub industry_sw1: String,
    pub quote_price_fen: Fen,
    pub prev_close_fen: Fen,
    pub is_st: bool,
    pub shares: i64,
    pub stop_loss_price_fen: Fen,
    pub rationale_zh: String,
    pub trade_date: String,
    pub next_trade_date: String,
    pub created_at_iso: String,
}

#[derive(Debug, Clone)]
pub struct SellOrderCommand {
    pub order_id: String,
    pub symbol: Symbol,
    pub quote_price_fen: Fen,
    pub prev_close_fen: Fen,
    pub is_st: bool,
    pub shares: i64,
    pub trigger_type: String,
    pub rationale_zh: String,
    pub trade_date: String,
    pub created_at_iso: String,
}

#[derive(Debug, Clone)]
pub struct PortfolioLedger {
    pub initial_capital_fen: Fen,
    pub cash_fen: Fen,
    pub total_fees_paid_fen: Fen,
    pub current_trade_date: String,
    pub fee_schedule: FeeSchedule,
    pub max_single_weight_pct: f64,
    pub limit_guard_pct: f64,
    pub positions: BTreeMap<String, PositionRecord>,
    pub orders: Vec<OrderRecord>,
}

impl PortfolioLedger {
    pub fn new(initial_capital_fen: Fen, trade_date: &str, fee_schedule: FeeSchedule) -> Self {
        Self {
            initial_capital_fen,
            cash_fen: initial_capital_fen,
            total_fees_paid_fen: Fen::ZERO,
            current_trade_date: trade_date.to_string(),
            fee_schedule,
            max_single_weight_pct: 10.0,
            limit_guard_pct: 0.25,
            positions: BTreeMap::new(),
            orders: Vec::new(),
        }
    }

    pub fn open_positions_count(&self) -> usize {
        self.positions.len()
    }

    pub fn cash_fen(&self) -> Fen {
        self.cash_fen
    }

    pub fn market_value_fen(&self) -> Fen {
        let sum: i64 = self
            .positions
            .values()
            .map(|p| p.mark_price_fen.0 * p.total_qty)
            .sum();
        Fen(sum)
    }

    pub fn total_equity_fen(&self) -> Fen {
        Fen(self.cash_fen.0 + self.market_value_fen().0)
    }

    pub fn advance_trading_day(&mut self, new_trade_date: &str) {
        use crate::domain::trading_calendar::is_ashare_trading_day_str;

        if new_trade_date > self.current_trade_date.as_str() {
            self.current_trade_date = new_trade_date.to_string();
        }
        if is_ashare_trading_day_str(new_trade_date) {
            for pos in self.positions.values_mut() {
                if pos.locked_t0_qty > 0 && new_trade_date >= pos.t_plus1_unlock_date.as_str() {
                    pos.sellable_qty += pos.locked_t0_qty;
                    pos.locked_t0_qty = 0;
                }
            }
        }
    }

    fn next_available_slot(&self) -> Option<u8> {
        let used: std::collections::BTreeSet<u8> =
            self.positions.values().map(|p| p.slot_no).collect();
        (1_u8..=(MAX_POSITION_SLOTS as u8)).find(|s| !used.contains(s))
    }

    pub fn execute_buy(&mut self, cmd: BuyOrderCommand) -> Result<OrderRecord, String> {
        self.execute_buy_with_adv20(cmd, None)
    }

    pub fn execute_buy_with_adv20(
        &mut self,
        cmd: BuyOrderCommand,
        adv20_amount_fen: Option<Fen>,
    ) -> Result<OrderRecord, String> {
        use crate::domain::trading_calendar::next_ashare_trading_date_str;
        use crate::domain::types::{is_limit_up_locked, validate_lot_size};

        if cmd.rationale_zh.trim().is_empty() {
            return Err("买入委托必须附带量化推荐理由 (rationale)".to_string());
        }
        validate_lot_size(cmd.symbol.board, cmd.shares)?;
        if is_limit_up_locked(
            cmd.quote_price_fen,
            cmd.prev_close_fen,
            cmd.symbol.board,
            cmd.is_st,
            self.limit_guard_pct,
        ) {
            return Err(format!("{} 涨停锁死无法买入成交", cmd.symbol.canonical()));
        }

        if cmd.trade_date > self.current_trade_date {
            self.advance_trading_day(&cmd.trade_date);
        }

        let cost = self.fee_schedule.compute_buy(cmd.quote_price_fen, cmd.shares)?;
        if cmd.stop_loss_price_fen.0 <= 0
            || cmd.stop_loss_price_fen >= cmd.quote_price_fen
            || cmd.stop_loss_price_fen >= cost.fill_price_fen
        {
            return Err(format!(
                "初始硬止损价({:.2})必须大于0且严格低于买入委托价({:.2})与成交价({:.2})",
                cmd.stop_loss_price_fen.to_yuan(),
                cmd.quote_price_fen.to_yuan(),
                cost.fill_price_fen.to_yuan()
            ));
        }

        if let Some(adv20) = adv20_amount_fen.filter(|a| a.0 > 0) {
            let max_adv_fen = ((adv20.0 as f64) * 0.015).round() as i64;
            if cost.gross_notional_fen.0 > max_adv_fen {
                return Err(format!(
                    "{} 委托金额 {:.2} 元超出 20日均成交额 1.5% 冲击红线 ({:.2} 元)",
                    cmd.symbol.canonical(),
                    cost.gross_notional_fen.to_yuan(),
                    Fen(max_adv_fen).to_yuan()
                ));
            }
        }

        let canonical = cmd.symbol.canonical();
        let is_new_slot = !self.positions.contains_key(&canonical);
        if is_new_slot && self.positions.len() >= MAX_POSITION_SLOTS {
            return Err(format!(
                "持仓席位已满 {}/{} 只上限，拒绝新开仓",
                self.positions.len(),
                MAX_POSITION_SLOTS
            ));
        }

        if self.max_single_weight_pct > 0.0 {
            let existing_mv_fen = self
                .positions
                .get(&canonical)
                .map(|p| cmd.quote_price_fen.0 * p.total_qty)
                .unwrap_or(0);
            let max_single_fen =
                ((self.total_equity_fen().0 as f64) * (self.max_single_weight_pct / 100.0)).round()
                    as i64;
            if existing_mv_fen + cost.total_cash_delta_fen.0 > max_single_fen {
                return Err(format!(
                    "{} 建仓后单票总敞口 {:.2} 元超出单票集中度上限 {:.1}% ({:.2} 元)",
                    canonical,
                    Fen(existing_mv_fen + cost.total_cash_delta_fen.0).to_yuan(),
                    self.max_single_weight_pct,
                    Fen(max_single_fen).to_yuan()
                ));
            }
        }

        if self.cash_fen < cost.total_cash_delta_fen {
            return Err(format!(
                "可用现金不足：需 {:.2} 元，当前现金 {:.2} 元",
                cost.total_cash_delta_fen.to_yuan(),
                self.cash_fen.to_yuan()
            ));
        }

        let unlock_date = {
            let calc = next_ashare_trading_date_str(&cmd.trade_date);
            if cmd.next_trade_date > calc {
                cmd.next_trade_date.clone()
            } else {
                calc
            }
        };

        self.cash_fen = Fen(self.cash_fen.0 - cost.total_cash_delta_fen.0);
        self.total_fees_paid_fen = Fen(
            self.total_fees_paid_fen.0 + cost.commission_fen.0 + cost.stamp_duty_fen.0,
        );

        if let Some(existing) = self.positions.get_mut(&canonical) {
            let prev_cost_notional = existing.avg_cost_fen.0 * existing.total_qty;
            let new_total_qty = existing.total_qty + cmd.shares;
            let blended_cost = (prev_cost_notional
                + cost.total_cash_delta_fen.0
                + new_total_qty.max(1) / 2)
                / new_total_qty.max(1);
            existing.total_qty = new_total_qty;
            existing.locked_t0_qty += cmd.shares;
            existing.avg_cost_fen = Fen(blended_cost);
            existing.mark_price_fen = cmd.quote_price_fen;
            existing.highest_price_fen = existing.highest_price_fen.max(cmd.quote_price_fen);
            existing.stop_loss_price_fen = existing.stop_loss_price_fen.max(cmd.stop_loss_price_fen);
            existing.last_buy_trade_date = cmd.trade_date.clone();
            existing.t_plus1_unlock_date = unlock_date.clone();
        } else {
            let slot_no = self
                .next_available_slot()
                .ok_or_else(|| "无可用持仓席位编号(1..10)".to_string())?;
            let avg_cost_fen =
                Fen((cost.total_cash_delta_fen.0 + cmd.shares.max(1) / 2) / cmd.shares.max(1));
            self.positions.insert(
                canonical,
                PositionRecord {
                    position_id: format!("POS-{}-{}", cmd.symbol.digits, slot_no),
                    slot_no,
                    symbol: cmd.symbol.clone(),
                    name_zh: cmd.name_zh.clone(),
                    industry_sw1: cmd.industry_sw1.clone(),
                    total_qty: cmd.shares,
                    sellable_qty: 0,
                    locked_t0_qty: cmd.shares,
                    avg_cost_fen,
                    mark_price_fen: cmd.quote_price_fen,
                    highest_price_fen: cmd.quote_price_fen.max(cost.fill_price_fen),
                    stop_loss_price_fen: cmd.stop_loss_price_fen,
                    entry_rationale_zh: cmd.rationale_zh.clone(),
                    opened_trade_date: cmd.trade_date.clone(),
                    last_buy_trade_date: cmd.trade_date.clone(),
                    t_plus1_unlock_date: unlock_date.clone(),
                    opened_at_iso: cmd.created_at_iso.clone(),
                },
            );
        }

        let order = OrderRecord {
            order_id: cmd.order_id,
            rec_id: cmd.rec_id,
            symbol: cmd.symbol,
            name_zh: cmd.name_zh,
            side: "BUY".to_string(),
            trigger_type: "QUANT_SIGNAL".to_string(),
            status: "FILLED".to_string(),
            lot_qty: cmd.shares,
            entry_price_fen: cmd.quote_price_fen,
            fill_price_fen: cost.fill_price_fen,
            stop_loss_price_fen: cmd.stop_loss_price_fen,
            commission_fen: cost.commission_fen,
            stamp_duty_fen: cost.stamp_duty_fen,
            slippage_fen: cost.slippage_fen,
            rationale_zh: cmd.rationale_zh,
            trade_date: cmd.trade_date,
            t_plus1_unlock_date: unlock_date,
            created_at_iso: cmd.created_at_iso,
        };
        self.orders.push(order.clone());
        Ok(order)
    }

    pub fn execute_sell(&mut self, cmd: SellOrderCommand) -> Result<OrderRecord, String> {
        use crate::domain::types::is_limit_down_locked;

        if cmd.shares <= 0 {
            return Err("卖出委托股数必须大于 0".to_string());
        }
        if cmd.trade_date > self.current_trade_date {
            self.advance_trading_day(&cmd.trade_date);
        }
        let canonical = cmd.symbol.canonical();
        let existing = self
            .positions
            .get(&canonical)
            .ok_or_else(|| format!("当前未持有 {}，无法平仓卖出", canonical))?;

        if existing.sellable_qty < cmd.shares {
            return Err(format!(
                "违反A股T+1交收锁仓约束：{} 当前可卖 {} 股（当日T+0锁仓 {} 股），请求卖出 {} 股",
                canonical, existing.sellable_qty, existing.locked_t0_qty, cmd.shares
            ));
        }
        let is_full_sellable_exit = cmd.shares == existing.sellable_qty;
        if !is_full_sellable_exit {
            crate::domain::types::validate_lot_size(cmd.symbol.board, cmd.shares)?;
        }

        if is_limit_down_locked(
            cmd.quote_price_fen,
            cmd.prev_close_fen,
            cmd.symbol.board,
            cmd.is_st,
            self.limit_guard_pct,
        ) {
            return Err(format!("{} 跌停封死流动性枯竭，延迟平仓", canonical));
        }

        let cost = self.fee_schedule.compute_sell(cmd.quote_price_fen, cmd.shares)?;
        let name_zh = existing.name_zh.clone();
        let stop_fen = existing.stop_loss_price_fen;

        self.cash_fen = Fen(self.cash_fen.0 + cost.total_cash_delta_fen.0);
        self.total_fees_paid_fen = Fen(
            self.total_fees_paid_fen.0 + cost.commission_fen.0 + cost.stamp_duty_fen.0,
        );

        let remove_pos = {
            let pos = self.positions.get_mut(&canonical).unwrap();
            pos.sellable_qty -= cmd.shares;
            pos.total_qty -= cmd.shares;
            pos.mark_price_fen = cmd.quote_price_fen;
            pos.total_qty == 0
        };
        if remove_pos {
            self.positions.remove(&canonical);
        }

        let order = OrderRecord {
            order_id: cmd.order_id,
            rec_id: None,
            symbol: cmd.symbol,
            name_zh,
            side: "SELL".to_string(),
            trigger_type: cmd.trigger_type,
            status: "FILLED".to_string(),
            lot_qty: cmd.shares,
            entry_price_fen: cmd.quote_price_fen,
            fill_price_fen: cost.fill_price_fen,
            stop_loss_price_fen: stop_fen,
            commission_fen: cost.commission_fen,
            stamp_duty_fen: cost.stamp_duty_fen,
            slippage_fen: cost.slippage_fen,
            rationale_zh: if cmd.rationale_zh.trim().is_empty() {
                "风控/轮动平仓".to_string()
            } else {
                cmd.rationale_zh
            },
            trade_date: cmd.trade_date.clone(),
            t_plus1_unlock_date: cmd.trade_date,
            created_at_iso: cmd.created_at_iso,
        };
        self.orders.push(order.clone());
        Ok(order)
    }

    pub fn update_mark_price(&mut self, canonical_symbol: &str, mark_price_fen: Fen) {
        if mark_price_fen.0 <= 0 {
            return;
        }
        if let Some(pos) = self.positions.get_mut(canonical_symbol) {
            pos.mark_price_fen = mark_price_fen;
            if mark_price_fen > pos.highest_price_fen {
                pos.highest_price_fen = mark_price_fen;
            }
        }
    }
}

pub fn suggest_position_shares(
    board: BoardKind,
    nav_fen: Fen,
    cash_fen: Fen,
    entry_price_fen: Fen,
    stop_loss_fen: Fen,
    max_single_weight_pct: f64,
    max_single_risk_pct: f64,
) -> i64 {
    suggest_position_shares_for_slot(
        board,
        nav_fen,
        cash_fen,
        Fen::ZERO,
        entry_price_fen,
        stop_loss_fen,
        max_single_weight_pct,
        max_single_risk_pct,
        FeeSchedule::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn suggest_position_shares_for_slot(
    board: BoardKind,
    nav_fen: Fen,
    cash_fen: Fen,
    existing_mv_fen: Fen,
    entry_price_fen: Fen,
    stop_loss_fen: Fen,
    max_single_weight_pct: f64,
    max_single_risk_pct: f64,
    fee_schedule: FeeSchedule,
) -> i64 {
    if entry_price_fen.0 <= 0 || cash_fen.0 <= 0 || nav_fen.0 <= 0 {
        return 0;
    }
    let single_cap_fen =
        (((nav_fen.0 as f64) * (max_single_weight_pct / 100.0)).round() as i64) - existing_mv_fen.0;
    if single_cap_fen <= 0 {
        return 0;
    }
    let max_budget_fen = single_cap_fen.min(((cash_fen.0 as f64) * 0.992).floor() as i64);
    let est_unit_cost_fen = (entry_price_fen.0 as f64)
        * (1.0 + (fee_schedule.slippage_bps + fee_schedule.commission_bps) / 10_000.0);
    let risk_per_share_fen = (entry_price_fen.0 - stop_loss_fen.0).max(1) as f64;
    let max_risk_budget_fen = (nav_fen.0 as f64) * (max_single_risk_pct / 100.0);

    let by_weight_shares = ((max_budget_fen as f64) / est_unit_cost_fen.max(1.0)).floor() as i64;
    let by_risk_shares = (max_risk_budget_fen / risk_per_share_fen).floor() as i64;
    let raw_shares = by_weight_shares.min(by_risk_shares);
    let step = if matches!(board, BoardKind::Star | BoardKind::Bse) {
        1_i64
    } else {
        100_i64
    };
    let mut candidate = if step == 1 {
        raw_shares
    } else {
        (raw_shares / 100) * 100
    };
    let min_lot = board.min_lot_shares();
    while candidate >= min_lot {
        if let Ok(cost) = fee_schedule.compute_buy(entry_price_fen, candidate) {
            if cost.total_cash_delta_fen.0 <= single_cap_fen
                && cost.total_cash_delta_fen <= cash_fen
            {
                return candidate;
            }
        }
        candidate -= step;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_buy(code_idx: usize, shares: i64, price_yuan: f64, date: &str) -> BuyOrderCommand {
        let sym = Symbol::parse(&format!("{:06}.SZ", 2001 + code_idx)).unwrap();
        let quote = Fen::from_yuan(price_yuan);
        let stop = Fen(((quote.0 as f64) * 0.945).round() as i64);
        BuyOrderCommand {
            order_id: format!("ORD-B-{}-{}", code_idx, date),
            rec_id: Some(format!("REC-{}", code_idx)),
            symbol: sym,
            name_zh: format!("测试标的{}", code_idx),
            industry_sw1: "电子".to_string(),
            quote_price_fen: quote,
            prev_close_fen: quote,
            is_st: false,
            shares,
            stop_loss_price_fen: stop,
            rationale_zh: "总市值68亿 + 连续两日换手>6.5%放量 + 五维排雷全清".to_string(),
            trade_date: date.to_string(),
            next_trade_date: "2026-11-19".to_string(),
            created_at_iso: format!("{}T10:00:00+08:00", date),
        }
    }

    #[test]
    fn enforces_t_plus_1_lock_and_10_slot_cap_and_mandatory_stop_loss() {
        let mut ledger = PortfolioLedger::new(
            Fen::INITIAL_CAPITAL_100W,
            "2026-11-18",
            FeeSchedule::default(),
        );

        // Missing rationale or invalid stop loss >= entry must fail
        let mut bad_stop = mk_buy(0, 500, 10.0, "2026-11-18");
        bad_stop.stop_loss_price_fen = Fen::from_yuan(10.5);
        assert!(ledger.execute_buy(bad_stop).is_err());

        let mut empty_rat = mk_buy(0, 500, 10.0, "2026-11-18");
        empty_rat.rationale_zh = "   ".to_string();
        assert!(ledger.execute_buy(empty_rat).is_err());

        // Fill 10 distinct slots
        for idx in 0..10 {
            ledger
                .execute_buy(mk_buy(idx, 500, 12.0, "2026-11-18"))
                .expect("slot 1..=10 buy should succeed");
        }
        assert_eq!(ledger.open_positions_count(), 10);

        // 11th distinct stock must fail due to 10-slot hard ceiling
        assert!(ledger.execute_buy(mk_buy(10, 500, 12.0, "2026-11-18")).is_err());

        // Same-day T+0 sell must ALWAYS fail
        let sym0 = Symbol::parse("002001.SZ").unwrap();
        let t0_sell = SellOrderCommand {
            order_id: "ORD-S-T0".to_string(),
            symbol: sym0.clone(),
            quote_price_fen: Fen::from_yuan(12.5),
            prev_close_fen: Fen::from_yuan(12.0),
            is_st: false,
            shares: 500,
            trigger_type: "TAKE_PROFIT".to_string(),
            rationale_zh: "尝试T+0卖出".to_string(),
            trade_date: "2026-11-18".to_string(),
            created_at_iso: "2026-11-18T14:30:00+08:00".to_string(),
        };
        assert!(ledger.execute_sell(t0_sell).is_err());

        // Advance to T+1 -> unlocks shares, sell succeeds!
        ledger.advance_trading_day("2026-11-19");
        let t1_sell = SellOrderCommand {
            order_id: "ORD-S-T1".to_string(),
            symbol: sym0,
            quote_price_fen: Fen::from_yuan(12.5),
            prev_close_fen: Fen::from_yuan(12.0),
            is_st: false,
            shares: 500,
            trigger_type: "TAKE_PROFIT".to_string(),
            rationale_zh: "T+1次日解锁止盈".to_string(),
            trade_date: "2026-11-19".to_string(),
            created_at_iso: "2026-11-19T10:00:00+08:00".to_string(),
        };
        assert!(ledger.execute_sell(t1_sell).is_ok());
        assert_eq!(ledger.open_positions_count(), 9);
    }
}
