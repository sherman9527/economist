use economist_quant_terminal_lib::domain::portfolio_ledger::{
    BuyOrderCommand, PortfolioLedger, SellOrderCommand, MAX_POSITION_SLOTS,
};
use economist_quant_terminal_lib::domain::types::{FeeSchedule, Fen, Symbol};
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_portfolio_accounting_t1_and_10_slot_invariants(
        actions in prop::collection::vec(
            (0_u8..4, 0_usize..14, 1_i64..12, 500_i64..4000_i64),
            1..45
        )
    ) {
        let mut day_num = 1_u32;
        let fmt_day = |d: u32| format!("2026-11-{:02}", d.min(28));
        let mut ledger = PortfolioLedger::new(
            Fen::INITIAL_CAPITAL_100W,
            &fmt_day(day_num),
            FeeSchedule::default(),
        );

        for (kind, sym_idx, lots, price_fen) in actions {
            let shares = lots * 100;
            let sym = Symbol::parse(&format!("{:06}.SZ", 2001 + sym_idx)).unwrap();
            let cur_date = ledger.current_trade_date.clone();

            match kind {
                0 => {
                    let stop = Fen(((price_fen as f64) * 0.94).round() as i64);
                    let was_unlocked = ledger
                        .positions
                        .get(&sym.canonical())
                        .map(|p| p.sellable_qty)
                        .unwrap_or(0);
                    let cmd = BuyOrderCommand {
                        order_id: format!("B-{}-{}", sym_idx, ledger.orders.len()),
                        rec_id: None,
                        symbol: sym.clone(),
                        name_zh: format!("票{}", sym_idx),
                        industry_sw1: "制造业".to_string(),
                        quote_price_fen: Fen(price_fen),
                        prev_close_fen: Fen(price_fen),
                        is_st: false,
                        shares,
                        stop_loss_price_fen: stop,
                        rationale_zh: "量化信号买入".to_string(),
                        trade_date: cur_date.clone(),
                        next_trade_date: "2026-12-01".to_string(),
                        created_at_iso: format!("{}T10:00:00+08:00", cur_date),
                    };
                    if let Ok(ord) = ledger.execute_buy(cmd) {
                        prop_assert!(ord.stop_loss_price_fen.0 > 0);
                        prop_assert!(ord.stop_loss_price_fen < ord.entry_price_fen);
                        prop_assert!(ord.stop_loss_price_fen < ord.fill_price_fen);
                        let pos = ledger.positions.get(&sym.canonical()).unwrap();
                        prop_assert_eq!(pos.sellable_qty, was_unlocked);
                        prop_assert!(pos.locked_t0_qty >= shares);
                    }
                }
                1 => {
                    let sellable = ledger
                        .positions
                        .get(&sym.canonical())
                        .map(|p| p.sellable_qty)
                        .unwrap_or(0);
                    let attempt_shares = sellable + 100;
                    let res = ledger.execute_sell(SellOrderCommand {
                        order_id: format!("S0-{}", ledger.orders.len()),
                        symbol: sym,
                        quote_price_fen: Fen(price_fen),
                        prev_close_fen: Fen(price_fen),
                        is_st: false,
                        shares: attempt_shares,
                        trigger_type: "REBALANCE_EXIT".to_string(),
                        rationale_zh: "测试T+0拦截".to_string(),
                        trade_date: cur_date.clone(),
                        created_at_iso: format!("{}T13:30:00+08:00", cur_date),
                    });
                    prop_assert!(res.is_err());
                }
                2 => {
                    if day_num < 28 {
                        day_num += 1;
                        ledger.advance_trading_day(&fmt_day(day_num));
                    }
                    let cur_date2 = ledger.current_trade_date.clone();
                    if let Some(pos) = ledger.positions.get(&sym.canonical()) {
                        let qty = pos.sellable_qty.min(shares);
                        if qty >= 100 {
                            let _ = ledger.execute_sell(SellOrderCommand {
                                order_id: format!("S1-{}", ledger.orders.len()),
                                symbol: sym,
                                quote_price_fen: Fen(price_fen),
                                prev_close_fen: Fen(price_fen),
                                is_st: false,
                                shares: qty,
                                trigger_type: "TAKE_PROFIT".to_string(),
                                rationale_zh: "次日平仓".to_string(),
                                trade_date: cur_date2.clone(),
                                created_at_iso: format!("{}T10:30:00+08:00", cur_date2),
                            });
                        }
                    }
                }
                _ => {
                    ledger.update_mark_price(&sym.canonical(), Fen(price_fen));
                }
            }

            prop_assert!(ledger.open_positions_count() <= MAX_POSITION_SLOTS);
            prop_assert!(ledger.cash_fen().0 >= 0);
            prop_assert_eq!(
                ledger.cash_fen().0 + ledger.market_value_fen().0,
                ledger.total_equity_fen().0
            );
            for pos in ledger.positions.values() {
                prop_assert!(pos.slot_no >= 1 && pos.slot_no <= 10);
                prop_assert_eq!(pos.total_qty, pos.sellable_qty + pos.locked_t0_qty);
                prop_assert!(pos.stop_loss_price_fen.0 > 0);
            }
        }
    }
}
