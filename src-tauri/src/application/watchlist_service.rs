use crate::application::portfolio_seeder::current_cst_dates;
use crate::application::scan_pipeline::ScanExecutionOutput;
use crate::application::terminal_service::TerminalService;
use crate::domain::types::{Fen, Symbol};
use crate::domain::watchlist_tracker::{
    evaluate_watchlist_entry, summarize_watchlist_win_rate, WatchlistItemState,
    DEFAULT_ROUND_TRIP_COST_BPS,
};
use crate::infrastructure::market_tencent::RawTencentQuote;
use crate::interface::dto::round_2;
use crate::interface::evolution_dto::{
    AddWatchlistInputDto, WatchlistItemDto, WatchlistOverviewDto, WatchlistSummaryDto,
};
use std::collections::HashMap;

pub fn map_watchlist_dto(items: &[WatchlistItemState]) -> WatchlistOverviewDto {
    let s = summarize_watchlist_win_rate(items);
    let summary = WatchlistSummaryDto {
        total_count: s.total_count,
        active_tracking_count: s.active_tracking_count,
        settled_count: s.settled_count,
        win_count: s.win_count,
        loss_count: s.loss_count,
        target_hit_count: s.target_hit_count,
        stop_hit_count: s.stop_hit_count,
        settled_win_rate_pct: round_2(s.settled_win_rate_pct),
        live_positive_rate_pct: round_2(s.live_positive_rate_pct),
        wilson_low_pct: round_2(s.wilson_low_pct),
        wilson_high_pct: round_2(s.wilson_high_pct),
        low_sample_warning: s.low_sample_warning,
        profit_factor: round_2(s.profit_factor),
        expectancy_pct: round_2(s.expectancy_bps / 100.0),
        high_conviction_win_rate_pct: round_2(s.high_conviction_win_rate_pct),
        baseline_conviction_win_rate_pct: round_2(s.baseline_conviction_win_rate_pct),
        round_trip_cost_bps: DEFAULT_ROUND_TRIP_COST_BPS,
    };
    let dto_items = items
        .iter()
        .map(|i| WatchlistItemDto {
            symbol: i.symbol.clone(),
            name_zh: i.name_zh.clone(),
            board: i.board.clone(),
            industry_sw1: i.industry_sw1.clone(),
            added_trade_date: i.added_trade_date.clone(),
            added_at_iso: i.added_at_iso.clone(),
            entry_price_yuan: i.entry_price_fen.to_yuan(),
            stop_loss_price_yuan: i.stop_loss_price_fen.to_yuan(),
            target_price_yuan: i.target_price_fen.to_yuan(),
            latest_price_yuan: i.latest_price_fen.to_yuan(),
            peak_price_yuan: i.peak_price_fen.to_yuan(),
            trough_price_yuan: i.trough_price_fen.to_yuan(),
            conviction_score: round_2(i.conviction_score),
            source_decision: i.source_decision.clone(),
            version_id_at_entry: i.version_id_at_entry.clone(),
            rationale_zh: i.rationale_zh.clone(),
            holding_trading_days: i.holding_trading_days,
            net_return_pct: round_2(i.net_return_bps / 100.0),
            mfe_pct: round_2(i.mfe_bps / 100.0),
            mae_pct: round_2(i.mae_bps / 100.0),
            outcome: i.outcome.as_str().to_string(),
            settled_trade_date: i.settled_trade_date.clone(),
            settled_price_yuan: i.settled_price_fen.map(|f| f.to_yuan()),
        })
        .collect();
    WatchlistOverviewDto {
        summary,
        items: dto_items,
    }
}

impl TerminalService {
    pub fn sync_watchlist_with_scan(
        &self,
        scan: &ScanExecutionOutput,
        today_cst: &str,
    ) -> Result<WatchlistOverviewDto, String> {
        let mut items = self.repo.list_watchlist_items()?;
        if !items.is_empty() {
            let quote_map: HashMap<String, &RawTencentQuote> = scan
                .raw_quotes
                .iter()
                .map(|q| (q.symbol.canonical(), q))
                .collect();
            for item in &mut items {
                if let Some(bars) = scan.kline_cache_1d.get(&item.symbol) {
                    for b in bars {
                        let d: String = b.time.chars().take(10).collect();
                        if d > item.added_trade_date {
                            evaluate_watchlist_entry(
                                item,
                                &d,
                                Fen::from_yuan(b.close),
                                Fen::from_yuan(b.high),
                                Fen::from_yuan(b.low),
                            );
                        }
                    }
                }
                if let Some(q) = quote_map.get(&item.symbol) {
                    if !q.is_suspended && q.latest_yuan > 0.0 {
                        evaluate_watchlist_entry(
                            item,
                            today_cst,
                            Fen::from_yuan(q.latest_yuan),
                            Fen::from_yuan(q.high_yuan.max(q.latest_yuan)),
                            Fen::from_yuan(if q.low_yuan > 0.0 {
                                q.low_yuan.min(q.latest_yuan)
                            } else {
                                q.latest_yuan
                            }),
                        );
                    }
                }
                self.repo.upsert_watchlist_item(item)?;
            }
        }
        Ok(map_watchlist_dto(&items))
    }

    pub async fn get_watchlist_overview(&self) -> Result<WatchlistOverviewDto, String> {
        let (today_cst, _, _) = current_cst_dates();
        let cached = self.scan_cache.read().await.as_ref().map(|c| c.output.clone());
        if let Some(ref scan) = cached {
            return self.sync_watchlist_with_scan(scan, &today_cst);
        }
        let items = self.repo.list_watchlist_items()?;
        Ok(map_watchlist_dto(&items))
    }

    pub async fn add_to_watchlist(
        &self,
        input: AddWatchlistInputDto,
    ) -> Result<WatchlistOverviewDto, String> {
        let sym = Symbol::parse(&input.symbol)?;
        if !input.entry_price_yuan.is_finite()
            || !(0.02..=10_000.0).contains(&input.entry_price_yuan)
        {
            return Err("自选纳入基准价必须处于有限安全区间 [0.02, 10000.00] 元".to_string());
        }
        if !input.stop_loss_price_yuan.is_finite()
            || input.stop_loss_price_yuan < 0.0
            || input.stop_loss_price_yuan > 10_000.0
        {
            return Err("自选止损价必须为有限非负数值且不超过 10000.00 元".to_string());
        }
        if !input.target_price_yuan.is_finite()
            || input.target_price_yuan < 0.0
            || input.target_price_yuan > 20_000.0
        {
            return Err("自选目标价必须为有限非负数值且不超过 20000.00 元".to_string());
        }
        if !input.conviction_score.is_finite()
            || !(0.0..=100.0).contains(&input.conviction_score)
        {
            return Err("自选信念评分必须处于有限区间 [0, 100]".to_string());
        }
        if input.name_zh.chars().count() > 64
            || input.board.chars().count() > 32
            || input.industry_sw1.chars().count() > 64
            || input.source_decision.chars().count() > 32
            || input.rationale_zh.chars().count() > 512
        {
            return Err("自选文本字段长度超出安全上限".to_string());
        }
        if input.factor_z_scores.len() > 32 {
            return Err("自选因子暴露映射条目数超出上限 (<=32)".to_string());
        }
        for (fk, &fv) in &input.factor_z_scores {
            if fk.is_empty()
                || fk.chars().count() > 64
                || !fv.is_finite()
                || !(-20.0..=20.0).contains(&fv)
            {
                return Err(format!("因子暴露 '{}' 数值非法或超出 [-20, 20] 安全域", fk));
            }
        }
        let bundle = self.plugins.load_all()?;
        let (today_cst, _, now_iso) = current_cst_dates();
        self.repo.ensure_default_version_lineage(&bundle, &now_iso)?;
        let active_ver = self
            .repo
            .list_version_snapshots()?
            .into_iter()
            .find(|v| v.is_active)
            .map(|v| v.version_id)
            .unwrap_or_else(|| format!("v{}", bundle.strategy.version.trim_start_matches('v')));

        let stop_yuan = if input.stop_loss_price_yuan > 0.0
            && input.stop_loss_price_yuan < input.entry_price_yuan
        {
            input.stop_loss_price_yuan
        } else {
            input.entry_price_yuan * 0.93
        };
        let target_yuan = if input.target_price_yuan > input.entry_price_yuan {
            input.target_price_yuan
        } else {
            input.entry_price_yuan * 1.15
        };

        let canonical = sym.canonical();
        let existing_items = self.repo.list_watchlist_items()?;
        if existing_items.iter().any(|x| {
            x.symbol == canonical
                && x.outcome == crate::domain::watchlist_tracker::WatchlistOutcome::Tracking
        }) {
            return self.get_watchlist_overview().await;
        }

        let item = WatchlistItemState::new_tracking(
            &canonical,
            input.name_zh.trim(),
            input.board.trim(),
            input.industry_sw1.trim(),
            &today_cst,
            &now_iso,
            Fen::from_yuan(input.entry_price_yuan),
            Fen::from_yuan(stop_yuan),
            Fen::from_yuan(target_yuan),
            input.conviction_score,
            input.source_decision.trim(),
            &active_ver,
            if input.rationale_zh.trim().is_empty() {
                "盘中观望暂不建仓，加入自选验证样本外胜率"
            } else {
                input.rationale_zh.trim()
            },
            input.factor_z_scores,
        );
        self.repo.reset_or_insert_watchlist_item(&item)?;
        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "QUANT",
            "watchlist_stock_added",
            Some(1),
            true,
            &format!(
                "已将 {} ({}) 纳入观望自选胜率池 [版本={} 基准价=¥{:.2}]",
                item.name_zh, item.symbol, active_ver, input.entry_price_yuan
            ),
        );
        self.get_watchlist_overview().await
    }

    pub async fn remove_from_watchlist(
        &self,
        symbol: &str,
    ) -> Result<WatchlistOverviewDto, String> {
        let sym = Symbol::parse(symbol)?;
        self.repo.remove_watchlist_item(&sym.canonical())?;
        self.get_watchlist_overview().await
    }
}
