use crate::application::dto_mapper::fee_schedule_from_strategy;
use crate::application::scan_pipeline::ScanExecutionOutput;
use crate::application::terminal_service::TerminalService;
use crate::domain::factor_engine::{
    compute_chandelier_stop_fen, compute_initial_hard_stop_fen_with_band,
};
use crate::domain::portfolio_ledger::{
    suggest_position_shares_for_slot, BuyOrderCommand, PortfolioLedger, SellOrderCommand,
};
use crate::domain::strategy_gate::is_st_or_delisting_name;
use crate::domain::trading_calendar::next_ashare_trading_date_str;
use crate::domain::types::{Fen, Symbol};
use crate::infrastructure::market_tencent::{fetch_tencent_full_universe, RawTencentQuote};
use crate::infrastructure::plugin_store::PluginBundle;
use crate::infrastructure::sqlite_repo::EquitySnapshotRow;
use crate::interface::dto::ScreenerCandidateDto;

pub fn current_cst_dates() -> (String, String, String) {
    let cst = chrono::FixedOffset::east_opt(8 * 3600).expect("valid CST");
    let now = chrono::Utc::now().with_timezone(&cst);
    let today = now.format("%Y-%m-%d").to_string();
    let next_str = next_ashare_trading_date_str(&today);
    (today, next_str, now.to_rfc3339())
}

impl TerminalService {
    pub(crate) async fn sync_and_seed_ledger_locked(
        &self,
        scan: &ScanExecutionOutput,
        bundle: &PluginBundle,
        today_cst: &str,
        next_cst: &str,
        now_iso: &str,
    ) -> Result<PortfolioLedger, String> {
        let _guard = self.ledger_lock.lock().await;
        let fee_sched = fee_schedule_from_strategy(&bundle.strategy);
        let init_cap = Fen::from_yuan(bundle.strategy.initial_capital_yuan as f64);
        let mut ledger = self.repo.load_ledger(init_cap, today_cst, fee_sched)?;
        ledger.max_single_weight_pct = bundle.strategy.max_single_weight_pct;
        ledger.limit_guard_pct = bundle.strategy.limit_up_guard_pct;
        self.refresh_open_positions_and_seed_if_empty(
            &mut ledger,
            scan,
            bundle,
            today_cst,
            next_cst,
            now_iso,
        )?;
        Ok(ledger)
    }

    pub(crate) fn refresh_open_positions_and_seed_if_empty(
        &self,
        ledger: &mut PortfolioLedger,
        scan: &ScanExecutionOutput,
        bundle: &PluginBundle,
        today_cst: &str,
        next_cst: &str,
        now_iso: &str,
    ) -> Result<(), String> {
        ledger.max_single_weight_pct = bundle.strategy.max_single_weight_pct;
        ledger.limit_guard_pct = bundle.strategy.limit_up_guard_pct;

        if ledger.orders.is_empty() && ledger.positions.is_empty() {
            let seed_picks: Vec<&ScreenerCandidateDto> = scan
                .candidates
                .iter()
                .filter(|c| c.decision == "BUY")
                .take(3)
                .collect();

            for (idx, pick) in seed_picks.into_iter().enumerate() {
                let Ok(sym) = Symbol::parse(&pick.symbol) else {
                    continue;
                };
                let bars = scan.kline_cache_1d.get(&pick.symbol);
                let prev_low_yuan = bars
                    .and_then(|v| {
                        v.iter()
                            .rev()
                            .find(|b| b.time.as_str() < pick.kline_bar_date.as_str())
                    })
                    .map(|b| b.low)
                    .unwrap_or(pick.latest_price_yuan * 0.955);
                let adv20_fen = bars.and_then(|v| {
                    let prior: Vec<_> = v
                        .iter()
                        .filter(|b| b.time.as_str() < pick.kline_bar_date.as_str())
                        .collect();
                    let slice = &prior[prior.len().saturating_sub(20)..];
                    if slice.is_empty() {
                        None
                    } else {
                        let avg_y =
                            slice.iter().map(|b| b.amount_yuan).sum::<f64>() / (slice.len() as f64);
                        Some(Fen::from_yuan(avg_y))
                    }
                });

                let quote_fen = Fen::from_yuan(pick.latest_price_yuan);
                let prev_yuan = (pick.latest_price_yuan / (1.0 + pick.pct_chg / 100.0)).max(0.5);
                let prev_fen = Fen::from_yuan(prev_yuan);
                let stop_fen = compute_initial_hard_stop_fen_with_band(
                    quote_fen,
                    Fen::from_yuan(prev_low_yuan),
                    Fen::from_yuan(pick.atr14_yuan.max(pick.latest_price_yuan * 0.025)),
                    bundle.strategy.stop_loss.atr_stop_mult,
                    bundle.strategy.stop_loss.hard_stop_min_ratio,
                    bundle.strategy.stop_loss.hard_stop_max_ratio,
                );
                let shares = suggest_position_shares_for_slot(
                    sym.board,
                    ledger.total_equity_fen(),
                    ledger.cash_fen,
                    Fen::ZERO,
                    quote_fen,
                    stop_fen,
                    bundle.strategy.max_single_weight_pct,
                    bundle.strategy.max_single_risk_pct,
                    ledger.fee_schedule,
                );
                if shares < sym.board.min_lot_shares() {
                    continue;
                }
                let _ = ledger.execute_buy_with_adv20(
                    BuyOrderCommand {
                        order_id: format!("ORD-SEED-{}-{}", idx + 1, sym.digits),
                        rec_id: Some(format!(
                            "REC-{}-{}",
                            pick.symbol.replace('.', ""),
                            pick.kline_bar_date
                        )),
                        symbol: sym,
                        name_zh: pick.name_zh.clone(),
                        industry_sw1: pick.industry_sw1.clone(),
                        quote_price_fen: quote_fen,
                        prev_close_fen: prev_fen,
                        is_st: false,
                        shares,
                        stop_loss_price_fen: stop_fen,
                        rationale_zh: pick.rationale_zh.clone(),
                        trade_date: today_cst.to_string(),
                        next_trade_date: next_cst.to_string(),
                        created_at_iso: now_iso.to_string(),
                    },
                    adv20_fen,
                );
            }
        }

        let quote_map: std::collections::HashMap<String, &RawTencentQuote> = scan
            .raw_quotes
            .iter()
            .map(|q| (q.symbol.canonical(), q))
            .collect();

        let keys: Vec<String> = ledger.positions.keys().cloned().collect();
        for sym_key in keys {
            if let Some(&q) = quote_map.get(&sym_key) {
                let mark_fen = Fen::from_yuan(q.latest_yuan);
                ledger.update_mark_price(&sym_key, mark_fen);
                let atr_fen = scan
                    .candidates
                    .iter()
                    .find(|c| c.symbol == sym_key)
                    .map(|c| Fen::from_yuan(c.atr14_yuan))
                    .unwrap_or_else(|| Fen((mark_fen.0 as f64 * 0.028).round() as i64));

                let mut auto_exit: Option<(Symbol, i64, Fen, Fen, bool, String, String)> = None;
                if let Some(pos) = ledger.positions.get_mut(&sym_key) {
                    pos.stop_loss_price_fen = compute_chandelier_stop_fen(
                        pos.avg_cost_fen,
                        pos.stop_loss_price_fen,
                        pos.highest_price_fen,
                        pos.mark_price_fen,
                        atr_fen,
                        bundle.strategy.stop_loss.chandelier_trigger_gain_pct,
                        bundle.strategy.stop_loss.chandelier_atr_mult,
                    );
                    if pos.sellable_qty > 0 && pos.mark_price_fen <= pos.stop_loss_price_fen {
                        let is_profit_lock = pos.stop_loss_price_fen > pos.avg_cost_fen;
                        let trig = if is_profit_lock {
                            "TAKE_PROFIT".to_string()
                        } else {
                            "STOP_LOSS".to_string()
                        };
                        let reason = if is_profit_lock {
                            format!(
                                "触及吊灯追踪止盈保护线 ¥{:.2}（现价 ¥{:.2}），自动锁定可卖仓位 {} 股利润",
                                pos.stop_loss_price_fen.to_yuan(),
                                pos.mark_price_fen.to_yuan(),
                                pos.sellable_qty
                            )
                        } else {
                            format!(
                                "跌破初始硬止损红线 ¥{:.2}（现价 ¥{:.2}），自动风控平仓 {} 股",
                                pos.stop_loss_price_fen.to_yuan(),
                                pos.mark_price_fen.to_yuan(),
                                pos.sellable_qty
                            )
                        };
                        auto_exit = Some((
                            pos.symbol.clone(),
                            pos.sellable_qty,
                            mark_fen,
                            Fen::from_yuan(q.prev_close_yuan.max(0.01)),
                            is_st_or_delisting_name(&q.name_zh),
                            trig,
                            reason,
                        ));
                    }
                }

                if let Some((sym, qty, q_fen, prev_fen, is_st, trig, reason)) = auto_exit {
                    let ord_id = format!(
                        "ORD-AUTO-{}-{}-{}",
                        trig,
                        sym.digits,
                        ledger.orders.len() + 1
                    );
                    if let Ok(filled) = ledger.execute_sell(SellOrderCommand {
                        order_id: ord_id,
                        symbol: sym.clone(),
                        quote_price_fen: q_fen,
                        prev_close_fen: prev_fen,
                        is_st,
                        shares: qty,
                        trigger_type: trig.clone(),
                        rationale_zh: reason,
                        trade_date: today_cst.to_string(),
                        created_at_iso: now_iso.to_string(),
                    }) {
                        let _ = self.cleaner.record_event(
                            &self.repo,
                            "WARN",
                            "TRADING",
                            "auto_stop_exit_filled",
                            Some(1),
                            true,
                            &format!(
                                "自动{}平仓 {} {}股 @ ¥{:.2}",
                                trig,
                                sym.canonical(),
                                filled.lot_qty,
                                filled.fill_price_fen.to_yuan()
                            ),
                        );
                    }
                }
            }
        }

        self.repo.save_ledger(ledger)?;
        if let Ok(existing_snaps) = self.repo.list_equity_snapshots() {
            if existing_snaps.is_empty() {
                self.seed_baseline_equity_curve(ledger, scan, today_cst);
            }
        }
        Ok(())
    }

    fn seed_baseline_equity_curve(
        &self,
        ledger: &PortfolioLedger,
        scan: &ScanExecutionOutput,
        today_cst: &str,
    ) {
        let prev_date = scan
            .kline_cache_1d
            .values()
            .next()
            .and_then(|bars| {
                bars.iter()
                    .rev()
                    .find(|b| b.time.as_str() < today_cst)
                    .map(|b| b.time.clone())
            });
        if let Some(dt) = prev_date {
            let init_fen = ledger.initial_capital_fen.0;
            let _ = self.repo.upsert_equity_snapshot(&EquitySnapshotRow {
                snap_date: dt,
                cash_fen: init_fen,
                market_value_fen: 0,
                total_equity_fen: init_fen,
                total_fees_paid_fen: 0,
                portfolio_nav: 1.0,
                csi1000_nav: 1.0,
                cni2000_nav: 1.0,
                excess_pct: 0.0,
            });
        }
    }

    pub(crate) async fn resolve_symbol_quote_context(
        &self,
        sym: &Symbol,
        bundle: &PluginBundle,
        fallback_stop_yuan: Option<f64>,
    ) -> (String, String, Fen, Fen, f64, bool) {
        {
            let guard = self.scan_cache.read().await;
            if let Some(st) = guard.as_ref() {
                if let Some(q) = st
                    .output
                    .raw_quotes
                    .iter()
                    .find(|q| q.symbol.canonical() == sym.canonical())
                {
                    let cand = st
                        .output
                        .candidates
                        .iter()
                        .find(|c| c.symbol == sym.canonical());
                    let ind = cand
                        .map(|c| c.industry_sw1.clone())
                        .or_else(|| {
                            st.output
                                .clist_meta
                                .get(&sym.digits)
                                .map(|m| m.industry_sw1.clone())
                        })
                        .unwrap_or_else(|| "专精特新".to_string());
                    let atr = cand
                        .map(|c| c.atr14_yuan)
                        .unwrap_or((q.latest_yuan * 0.028).max(0.05));
                    let is_st = q.name_zh.to_ascii_uppercase().contains("ST")
                        || q.name_zh.contains("退");
                    return (
                        q.name_zh.clone(),
                        ind,
                        Fen::from_yuan(q.latest_yuan),
                        Fen::from_yuan(q.prev_close_yuan.max(0.01)),
                        atr,
                        is_st,
                    );
                }
            }
        }

        let single_code = vec![sym.tencent_code()];
        if let Ok(mut live) = fetch_tencent_full_universe(
            &self.http,
            &bundle.app_cfg.market_sources.tencent_batch_url,
            Some(&single_code),
            10,
            1,
        )
        .await
        {
            if let Some(q) = live.pop() {
                let is_st =
                    q.name_zh.to_ascii_uppercase().contains("ST") || q.name_zh.contains("退");
                return (
                    q.name_zh,
                    "专精特新".to_string(),
                    Fen::from_yuan(q.latest_yuan),
                    Fen::from_yuan(q.prev_close_yuan.max(0.01)),
                    (q.latest_yuan * 0.028).max(0.05),
                    is_st,
                );
            }
        }

        let ref_yuan = fallback_stop_yuan
            .map(|s| (s / 0.94).max(1.0))
            .unwrap_or(20.0);
        (
            format!("模拟标的{}", sym.digits),
            "电子制造".to_string(),
            Fen::from_yuan(ref_yuan),
            Fen::from_yuan(ref_yuan * 0.99),
            (ref_yuan * 0.028).max(0.05),
            false,
        )
    }
}
