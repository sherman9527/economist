use crate::application::dto_mapper::{
    board_to_str, build_kline_payload_dto, fee_schedule_from_strategy, map_portfolio_overview_dto,
};
use crate::application::portfolio_seeder::current_cst_dates;
use crate::application::terminal_service::TerminalService;
use crate::domain::factor_engine::compute_initial_hard_stop_fen_with_band;
use crate::domain::portfolio_ledger::{
    suggest_position_shares_for_slot, BuyOrderCommand, SellOrderCommand,
};
use crate::domain::types::{Fen, Symbol};
use crate::infrastructure::market_eastmoney::fetch_eastmoney_klines;
use crate::infrastructure::market_tencent::{fetch_tencent_fallback_klines, SimpleBar};
use crate::interface::dto::{PlaceOrderInputDto, PortfolioOverviewDto, StockKlinePayloadDto};

impl TerminalService {
    pub async fn get_stock_kline(
        &self,
        symbol: &str,
        period: &str,
    ) -> Result<StockKlinePayloadDto, String> {
        let sym = Symbol::parse(symbol)?;
        let norm_period = if period.eq_ignore_ascii_case("30m") {
            "30m"
        } else {
            "1d"
        };
        let bundle = self.plugins.load_all()?;

        let (cached_bars, candidate_opt, quote_name, circ_mv_yi) = {
            let guard = self.scan_cache.read().await;
            if let Some(st) = guard.as_ref() {
                let bars = if norm_period == "1d" {
                    st.output.kline_cache_1d.get(&sym.canonical()).cloned()
                } else {
                    None
                };
                let cand = st
                    .output
                    .candidates
                    .iter()
                    .find(|c| c.symbol == sym.canonical())
                    .cloned();
                let q = st
                    .output
                    .raw_quotes
                    .iter()
                    .find(|q| q.symbol.canonical() == sym.canonical());
                (
                    bars,
                    cand,
                    q.map(|x| x.name_zh.clone()),
                    q.map(|x| x.circ_mv_yi).unwrap_or(50.0),
                )
            } else {
                (None, None, None, 50.0)
            }
        };

        let raw_bars: Vec<SimpleBar> = if let Some(b) = cached_bars.filter(|v| !v.is_empty()) {
            b
        } else {
            match fetch_eastmoney_klines(
                &self.http,
                &bundle.app_cfg.market_sources.eastmoney_kline_url,
                &sym,
                norm_period,
                96,
            )
            .await
            {
                Ok(b) if !b.is_empty() => b,
                _ => {
                    fetch_tencent_fallback_klines(
                        &self.http,
                        &bundle.app_cfg.market_sources.tencent_fqkline_url,
                        &bundle.app_cfg.market_sources.tencent_mkline_url,
                        &sym,
                        norm_period,
                        96,
                        circ_mv_yi,
                    )
                    .await?
                }
            }
        };

        let (today_cst, _, _) = current_cst_dates();
        let ledger = self.repo.load_ledger(
            Fen::from_yuan(bundle.strategy.initial_capital_yuan as f64),
            &today_cst,
            fee_schedule_from_strategy(&bundle.strategy),
        )?;
        let active_pos = ledger.positions.get(&sym.canonical());
        let active_stop_yuan = active_pos.map(|p| p.stop_loss_price_fen.to_yuan());
        let name_zh = candidate_opt
            .as_ref()
            .map(|c| c.name_zh.clone())
            .or(quote_name)
            .or_else(|| active_pos.map(|p| p.name_zh.clone()))
            .unwrap_or_else(|| format!("{}({})", board_to_str(sym.board), sym.digits));

        let stored_recs = self.repo.list_recommendations_for_symbol(&sym.canonical())?;
        Ok(build_kline_payload_dto(
            &sym,
            &name_zh,
            norm_period,
            &raw_bars,
            &stored_recs,
            candidate_opt.as_ref(),
            active_stop_yuan,
            &bundle,
        ))
    }

    pub async fn place_simulated_buy(
        &self,
        input: PlaceOrderInputDto,
    ) -> Result<PortfolioOverviewDto, String> {
        let sym = Symbol::parse(&input.symbol)?;
        let bundle = self.plugins.load_all()?;
        let (today_cst, next_cst, now_iso) = current_cst_dates();
        let fee_sched = fee_schedule_from_strategy(&bundle.strategy);
        let init_cap = Fen::from_yuan(bundle.strategy.initial_capital_yuan as f64);

        let (name_zh, industry_sw1, quote_fen, prev_fen, atr_yuan, is_st) = self
            .resolve_symbol_quote_context(&sym, &bundle, input.stop_loss_price_yuan)
            .await;

        let (prev_low_fen_opt, adv20_fen_opt) = {
            let guard = self.scan_cache.read().await;
            if let Some(st) = guard.as_ref() {
                if let Some(bars) = st.output.kline_cache_1d.get(&sym.canonical()) {
                    let prior: Vec<_> = bars
                        .iter()
                        .filter(|b| b.time.as_str() < today_cst.as_str())
                        .collect();
                    let prev_low = prior.last().map(|b| Fen::from_yuan(b.low));
                    let slice = &prior[prior.len().saturating_sub(20)..];
                    let adv20 = if slice.is_empty() {
                        None
                    } else {
                        let avg =
                            slice.iter().map(|b| b.amount_yuan).sum::<f64>() / (slice.len() as f64);
                        Some(Fen::from_yuan(avg))
                    };
                    (prev_low, adv20)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        };

        let stop_fen = match input.stop_loss_price_yuan {
            Some(y) if y > 0.0 => Fen::from_yuan(y),
            _ => compute_initial_hard_stop_fen_with_band(
                quote_fen,
                prev_low_fen_opt.unwrap_or_else(|| Fen::from_yuan(quote_fen.to_yuan() * 0.955)),
                Fen::from_yuan(atr_yuan),
                bundle.strategy.stop_loss.atr_stop_mult,
                bundle.strategy.stop_loss.hard_stop_min_ratio,
                bundle.strategy.stop_loss.hard_stop_max_ratio,
            ),
        };
        let rationale_zh = input
            .rationale_zh
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                format!(
                    "{}({}) <200亿双日高换手放量模型入选，执行模拟建仓",
                    name_zh,
                    sym.canonical()
                )
            });

        let (ledger, filled) = {
            let _guard = self.ledger_lock.lock().await;
            let mut ledger = self.repo.load_ledger(init_cap, &today_cst, fee_sched)?;
            ledger.max_single_weight_pct = bundle.strategy.max_single_weight_pct;
            ledger.limit_guard_pct = bundle.strategy.limit_up_guard_pct;
            let existing_mv_fen = ledger
                .positions
                .get(&sym.canonical())
                .map(|p| Fen(quote_fen.0 * p.total_qty))
                .unwrap_or(Fen::ZERO);
            let suggested = suggest_position_shares_for_slot(
                sym.board,
                ledger.total_equity_fen(),
                ledger.cash_fen,
                existing_mv_fen,
                quote_fen,
                stop_fen,
                bundle.strategy.max_single_weight_pct,
                bundle.strategy.max_single_risk_pct,
                ledger.fee_schedule,
            );
            let default_shares = if suggested >= sym.board.min_lot_shares() {
                suggested
            } else {
                sym.board.min_lot_shares()
            };
            let shares = input.lot_qty.unwrap_or(default_shares);

            let order_id = format!(
                "ORD-BUY-{}-{}",
                sym.digits,
                chrono::Utc::now().timestamp_millis() % 1_000_000
            );
            let filled = ledger.execute_buy_with_adv20(
                BuyOrderCommand {
                    order_id,
                    rec_id: None,
                    symbol: sym.clone(),
                    name_zh: name_zh.clone(),
                    industry_sw1,
                    quote_price_fen: quote_fen,
                    prev_close_fen: prev_fen,
                    is_st,
                    shares,
                    stop_loss_price_fen: stop_fen,
                    rationale_zh,
                    trade_date: today_cst,
                    next_trade_date: next_cst,
                    created_at_iso: now_iso,
                },
                adv20_fen_opt,
            )?;
            self.repo.save_ledger(&ledger)?;
            (ledger, filled)
        };

        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "TRADING",
            "simulated_buy_filled",
            Some(2),
            true,
            &format!(
                "模拟买入 {} {} {}股 @ ¥{:.2} (止损 ¥{:.2}, T+1锁仓)",
                sym.canonical(),
                name_zh,
                filled.lot_qty,
                filled.fill_price_fen.to_yuan(),
                filled.stop_loss_price_fen.to_yuan()
            ),
        );

        let snaps = self.repo.list_equity_snapshots()?;
        Ok(map_portfolio_overview_dto(&ledger, &snaps))
    }

    pub async fn close_simulated_position(
        &self,
        symbol: &str,
        reason_zh: &str,
    ) -> Result<PortfolioOverviewDto, String> {
        let sym = Symbol::parse(symbol)?;
        let bundle = self.plugins.load_all()?;
        let (today_cst, _, now_iso) = current_cst_dates();
        let fee_sched = fee_schedule_from_strategy(&bundle.strategy);
        let init_cap = Fen::from_yuan(bundle.strategy.initial_capital_yuan as f64);

        let pre_stop_yuan = {
            let _guard = self.ledger_lock.lock().await;
            let snapshot = self.repo.load_ledger(init_cap, &today_cst, fee_sched)?;
            snapshot
                .positions
                .get(&sym.canonical())
                .map(|p| p.stop_loss_price_fen.to_yuan())
                .ok_or_else(|| format!("持仓席位中不存在 {}", sym.canonical()))?
        };

        let (_, _, live_quote_fen, prev_fen, _, is_st) = self
            .resolve_symbol_quote_context(&sym, &bundle, Some(pre_stop_yuan))
            .await;

        let trigger_type = if reason_zh.contains("止损") {
            "STOP_LOSS"
        } else if reason_zh.contains("止盈") {
            "TAKE_PROFIT"
        } else {
            "REBALANCE_EXIT"
        }
        .to_string();

        let (ledger, filled) = {
            let _guard = self.ledger_lock.lock().await;
            let mut ledger = self.repo.load_ledger(init_cap, &today_cst, fee_sched)?;
            ledger.max_single_weight_pct = bundle.strategy.max_single_weight_pct;
            ledger.limit_guard_pct = bundle.strategy.limit_up_guard_pct;
            let existing = ledger
                .positions
                .get(&sym.canonical())
                .cloned()
                .ok_or_else(|| format!("持仓席位中不存在 {}", sym.canonical()))?;
            let quote_fen = if live_quote_fen.0 > 0 {
                live_quote_fen
            } else {
                existing.mark_price_fen
            };
            let shares_to_close = if existing.sellable_qty > 0 {
                existing.sellable_qty
            } else {
                existing.total_qty
            };
            let order_id = format!(
                "ORD-SELL-{}-{}",
                sym.digits,
                chrono::Utc::now().timestamp_millis() % 1_000_000
            );
            let filled = ledger.execute_sell(SellOrderCommand {
                order_id,
                symbol: sym.clone(),
                quote_price_fen: quote_fen,
                prev_close_fen: prev_fen,
                is_st,
                shares: shares_to_close,
                trigger_type,
                rationale_zh: reason_zh.to_string(),
                trade_date: today_cst,
                created_at_iso: now_iso,
            })?;
            self.repo.save_ledger(&ledger)?;
            (ledger, filled)
        };

        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "TRADING",
            "simulated_position_closed",
            Some(2),
            true,
            &format!(
                "模拟平仓 {} {}股 @ ¥{:.2} (印花税 ¥{:.2}, 佣金 ¥{:.2})",
                sym.canonical(),
                filled.lot_qty,
                filled.fill_price_fen.to_yuan(),
                filled.stamp_duty_fen.to_yuan(),
                filled.commission_fen.to_yuan()
            ),
        );

        let snaps = self.repo.list_equity_snapshots()?;
        Ok(map_portfolio_overview_dto(&ledger, &snaps))
    }
}
