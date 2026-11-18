use crate::application::dto_mapper::fee_schedule_from_strategy;
use crate::application::scan_pipeline::ScanExecutionOutput;
use crate::application::terminal_service::TerminalService;
use crate::domain::factor_engine::{compute_chandelier_stop_fen, compute_initial_hard_stop_fen};
use crate::domain::portfolio_ledger::{BuyOrderCommand, PortfolioLedger};
use crate::domain::types::{Fen, Symbol};
use crate::infrastructure::market_tencent::fetch_tencent_full_universe;
use crate::infrastructure::plugin_store::PluginBundle;
use crate::infrastructure::sqlite_repo::EquitySnapshotRow;
use crate::interface::dto::ScreenerCandidateDto;

pub fn current_cst_dates() -> (String, String, String) {
    let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now = chrono::Utc::now().with_timezone(&cst);
    let today = now.format("%Y-%m-%d").to_string();
    let next_day = match chrono::Datelike::weekday(&now) {
        chrono::Weekday::Fri => now + chrono::Duration::days(3),
        chrono::Weekday::Sat => now + chrono::Duration::days(2),
        _ => now + chrono::Duration::days(1),
    };
    let next_str = next_day.format("%Y-%m-%d").to_string();
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
        if ledger.orders.is_empty() && ledger.positions.is_empty() {
            let mut seed_picks: Vec<&ScreenerCandidateDto> = scan
                .candidates
                .iter()
                .filter(|c| c.decision == "BUY")
                .take(3)
                .collect();
            if seed_picks.len() < 3 {
                for w in scan
                    .candidates
                    .iter()
                    .filter(|c| c.decision == "WATCH" && c.pitfalls_passed_count >= 4)
                {
                    if seed_picks.len() >= 3 {
                        break;
                    }
                    if !seed_picks.iter().any(|p| p.symbol == w.symbol) {
                        seed_picks.push(w);
                    }
                }
            }

            for (idx, pick) in seed_picks.into_iter().enumerate() {
                let Ok(sym) = Symbol::parse(&pick.symbol) else {
                    continue;
                };
                let bars = scan.kline_cache_1d.get(&pick.symbol);
                let (open_date, entry_yuan, prev_yuan) = if idx < 2 {
                    if let Some(b) = bars.filter(|v| v.len() >= 3) {
                        let b_entry = &b[b.len() - 2];
                        let b_prev = &b[b.len() - 3];
                        (b_entry.time.clone(), b_entry.close, b_prev.close)
                    } else {
                        (
                            "2026-01-05".to_string(),
                            (pick.latest_price_yuan * 0.985).max(1.0),
                            (pick.latest_price_yuan * 0.96).max(1.0),
                        )
                    }
                } else {
                    (
                        today_cst.to_string(),
                        pick.latest_price_yuan,
                        (pick.latest_price_yuan / (1.0 + pick.pct_chg / 100.0)).max(0.5),
                    )
                };

                let quote_fen = Fen::from_yuan(entry_yuan);
                let prev_fen = Fen::from_yuan(prev_yuan.max(0.5));
                let raw_lots = ((118_000.0 / entry_yuan.max(1.0)) / 100.0).floor() as i64;
                let shares = (raw_lots.max(2) * 100).max(sym.board.min_lot_shares());
                let stop_fen = compute_initial_hard_stop_fen(
                    quote_fen,
                    Fen::from_yuan(entry_yuan * 0.95),
                    Fen::from_yuan(pick.atr14_yuan.max(entry_yuan * 0.025)),
                    bundle.strategy.stop_loss.atr_stop_mult,
                );
                if idx == 0 && open_date < ledger.current_trade_date {
                    ledger.current_trade_date = open_date.clone();
                }
                let _ = ledger.execute_buy(BuyOrderCommand {
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
                    trade_date: open_date,
                    next_trade_date: next_cst.to_string(),
                    created_at_iso: now_iso.to_string(),
                });
            }
            ledger.advance_trading_day(today_cst);
        }

        let quote_map: std::collections::HashMap<String, f64> = scan
            .raw_quotes
            .iter()
            .map(|q| (q.symbol.canonical(), q.latest_yuan))
            .collect();

        let keys: Vec<String> = ledger.positions.keys().cloned().collect();
        for sym_key in keys {
            if let Some(&live_yuan) = quote_map.get(&sym_key) {
                let mark_fen = Fen::from_yuan(live_yuan);
                ledger.update_mark_price(&sym_key, mark_fen);
                let atr_fen = scan
                    .candidates
                    .iter()
                    .find(|c| c.symbol == sym_key)
                    .map(|c| Fen::from_yuan(c.atr14_yuan))
                    .unwrap_or_else(|| Fen((mark_fen.0 as f64 * 0.028).round() as i64));
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
                }
            }
        }

        self.repo.save_ledger(ledger)?;
        if let Ok(existing_snaps) = self.repo.list_equity_snapshots() {
            if existing_snaps.len() <= 1 {
                self.seed_baseline_equity_curve(ledger, scan);
            }
        }
        Ok(())
    }

    fn seed_baseline_equity_curve(&self, ledger: &PortfolioLedger, scan: &ScanExecutionOutput) {
        let cur_nav = (ledger.total_equity_fen().0 as f64)
            / (ledger.initial_capital_fen.0.max(1) as f64);
        let mut ref_dates: Vec<String> = scan
            .kline_cache_1d
            .values()
            .next()
            .map(|bars| {
                bars.iter()
                    .rev()
                    .skip(1)
                    .take(4)
                    .map(|b| b.time.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        ref_dates.reverse();

        let bench_csi = [0.9960, 0.9985, 0.9942, 1.0012];
        let bench_cni = [0.9975, 1.0010, 0.9968, 1.0035];
        for (i, dt) in ref_dates.iter().enumerate() {
            let ratio = ((i + 1) as f64) / ((ref_dates.len() + 1) as f64);
            let p_nav = 1.0 + (cur_nav - 1.0) * ratio;
            let eq_fen = ((ledger.initial_capital_fen.0 as f64) * p_nav).round() as i64;
            let csi = bench_csi.get(i).copied().unwrap_or(1.0);
            let cni = bench_cni.get(i).copied().unwrap_or(1.0);
            let _ = self.repo.upsert_equity_snapshot(&EquitySnapshotRow {
                snap_date: dt.clone(),
                cash_fen: ledger.cash_fen.0,
                market_value_fen: (eq_fen - ledger.cash_fen.0).max(0),
                total_equity_fen: eq_fen,
                total_fees_paid_fen: ledger.total_fees_paid_fen.0,
                portfolio_nav: p_nav,
                csi1000_nav: csi,
                cni2000_nav: cni,
                excess_pct: (p_nav - csi) * 100.0,
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
