use crate::domain::strategy_gate::{interpolate_u_curve_weight, MarketSessionState};
use crate::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};

#[derive(Debug, Clone)]
pub struct AlignedDailyBars<'a> {
    pub quote_date: String,
    pub completed_prior_bars: &'a [SimpleBar],
    pub today_bar: Option<&'a SimpleBar>,
    pub bar_t1: &'a SimpleBar,
    pub adv20_amount_yuan: f64,
}

pub fn extract_quote_trade_date(raw_ts: &str) -> Option<String> {
    use crate::domain::trading_calendar::effective_ashare_trade_date;

    let trimmed = raw_ts.trim();
    let candidate = if trimmed.len() >= 8 && trimmed.as_bytes()[..8].iter().all(|b| b.is_ascii_digit()) {
        Some(format!(
            "{}-{}-{}",
            &trimmed[0..4],
            &trimmed[4..6],
            &trimmed[6..8]
        ))
    } else if trimmed.len() >= 10
        && trimmed.as_bytes()[4] == b'-'
        && trimmed.as_bytes()[7] == b'-'
    {
        Some(trimmed[..10].to_string())
    } else {
        None
    }?;
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(&candidate, "%Y-%m-%d") {
        Some(effective_ashare_trade_date(parsed).format("%Y-%m-%d").to_string())
    } else {
        Some(candidate)
    }
}

pub fn align_daily_bars_to_quote<'a>(
    quote: &RawTencentQuote,
    bars: &'a [SimpleBar],
) -> Option<AlignedDailyBars<'a>> {
    if bars.len() < 2 {
        return None;
    }
    let quote_date = extract_quote_trade_date(&quote.raw_ts)
        .unwrap_or_else(|| bars[bars.len() - 1].time.chars().take(10).collect());

    let split_idx = bars
        .iter()
        .position(|b| b.time.as_str() >= quote_date.as_str())
        .unwrap_or(bars.len());

    let (completed_prior_bars, today_bar) = if split_idx == 0 {
        return None;
    } else {
        let today_opt = bars
            .get(split_idx..)
            .and_then(|tail| tail.iter().find(|b| b.time.starts_with(&quote_date)));
        (&bars[..split_idx], today_opt)
    };

    let bar_t1 = completed_prior_bars.last()?;
    let prior_20 = &completed_prior_bars[completed_prior_bars.len().saturating_sub(20)..];
    let adv20_amount_yuan = if !prior_20.is_empty() {
        prior_20.iter().map(|b| b.amount_yuan).sum::<f64>() / (prior_20.len() as f64)
    } else {
        bar_t1.amount_yuan
    };

    Some(AlignedDailyBars {
        quote_date,
        completed_prior_bars,
        today_bar,
        bar_t1,
        adv20_amount_yuan,
    })
}

pub fn project_shortlist_turnover_pct(
    realized_turnover_pct: f64,
    session_state: MarketSessionState,
    elapsed_minutes: u32,
    u_weights: &[(u16, f64)],
) -> f64 {
    if matches!(
        session_state,
        MarketSessionState::Trading | MarketSessionState::LunchBreak
    ) && elapsed_minutes < 240
    {
        let w = interpolate_u_curve_weight((elapsed_minutes as u16).min(240), u_weights);
        realized_turnover_pct / w.max(0.08)
    } else {
        realized_turnover_pct
    }
}

pub struct EnrichedIntermediate {
    pub quote: RawTencentQuote,
    pub industry_sw1: String,
    pub main_inflow_wan: f64,
    pub turnover_t0: f64,
    pub turnover_t1: f64,
    pub projected_turnover: f64,
    pub vol_expansion: f64,
    pub ma20: f64,
    pub atr14: f64,
    pub prev_low_yuan: f64,
    pub bar_date: String,
    pub gate_passed: bool,
    pub gate_reject_reason: Option<String>,
    pub pit_report: crate::domain::pitfall_veto::PitfallAuditReport,
}

pub fn build_screener_candidate_dto(
    item: EnrichedIntermediate,
    scored: Option<&crate::domain::factor_engine::CrossSectionScored>,
    stop_cfg: &crate::infrastructure::plugin_store::StopLossManifestSection,
) -> crate::interface::dto::ScreenerCandidateDto {
    use crate::domain::factor_engine::compute_initial_hard_stop_fen_with_band;
    use crate::domain::pitfall_veto::{PitfallDimension, PitfallSeverity};
    use crate::domain::types::{BoardKind, ExchangeCode, Fen};
    use crate::interface::dto::{round_2, PitfallDimensionStatusDto, ScreenerCandidateDto};

    let canonical = item.quote.symbol.canonical();
    let raw_conv = scored.map(|s| s.conviction_score).unwrap_or(60.0);
    let conviction_score =
        round_2((raw_conv - item.pit_report.warn_penalty_pts).clamp(5.0, 99.5));

    let decision = if item.pit_report.vetoed {
        "REJECTED_BY_MINE".to_string()
    } else if item.gate_passed && conviction_score >= 54.0 {
        "BUY".to_string()
    } else {
        "WATCH".to_string()
    };

    let entry_fen = Fen::from_yuan(item.quote.latest_yuan);
    let stop_fen = compute_initial_hard_stop_fen_with_band(
        entry_fen,
        Fen::from_yuan(item.prev_low_yuan),
        Fen::from_yuan(item.atr14),
        stop_cfg.atr_stop_mult,
        stop_cfg.hard_stop_min_ratio,
        stop_cfg.hard_stop_max_ratio,
    );
    let risk_fen = (entry_fen.0 - stop_fen.0).max(1);
    let target_fen = Fen(entry_fen.0 + (risk_fen as f64 * 2.4).round() as i64);

    let pct_chg = if item.quote.prev_close_yuan > 0.0 {
        round_2(
            ((item.quote.latest_yuan - item.quote.prev_close_yuan) / item.quote.prev_close_yuan)
                * 100.0,
        )
    } else {
        0.0
    };

    let pitfalls_dto: Vec<PitfallDimensionStatusDto> = item
        .pit_report
        .items
        .iter()
        .map(|p| PitfallDimensionStatusDto {
            dimension: match p.dimension {
                PitfallDimension::InsiderReduction => "INSIDER_REDUCTION",
                PitfallDimension::NegativeNews => "NEGATIVE_NEWS",
                PitfallDimension::HolderSurge => "HOLDER_SURGE",
                PitfallDimension::MainOutflow => "MAIN_OUTFLOW",
                PitfallDimension::TechBreakdown => "TECH_BREAKDOWN",
            }
            .to_string(),
            label_zh: p.label_zh.clone(),
            passed: p.passed,
            severity: match p.severity {
                PitfallSeverity::Clear => "CLEAR",
                PitfallSeverity::Warn => "WARN",
                PitfallSeverity::Veto => "VETO",
            }
            .to_string(),
            detail_zh: p.detail_zh.clone(),
        })
        .collect();

    let rationale_zh = if decision == "BUY" {
        format!(
            "总市值 {:.1}亿 (<200亿硬红线)，两日连续换手 {:.2}%→{:.2}% 放量 {:.2}x 站稳MA20({:.2})，五维排雷 {}/5 通过，多因子中性化得分 {:.1}，硬止损 ¥{:.2}。",
            item.quote.total_mv_yi,
            item.turnover_t1,
            item.turnover_t0,
            item.vol_expansion,
            item.ma20,
            item.pit_report.passed_count,
            conviction_score,
            stop_fen.to_yuan()
        )
    } else if decision == "REJECTED_BY_MINE" {
        let veto_msg = pitfalls_dto
            .iter()
            .find(|p| p.severity == "VETO")
            .map(|p| p.detail_zh.as_str())
            .unwrap_or("触发五维排雷一票否决");
        format!("排雷一票否决拦截：{}", veto_msg)
    } else {
        format!(
            "观察候选：{}（前日换手 {:.2}% / 今日换手 {:.2}%，多因子得分 {:.1}）",
            item.gate_reject_reason
                .as_deref()
                .unwrap_or("等待放量突破确认"),
            item.turnover_t1,
            item.turnover_t0,
            conviction_score
        )
    };

    let exchange = match item.quote.symbol.exchange {
        ExchangeCode::Sh => "SH",
        ExchangeCode::Sz => "SZ",
        ExchangeCode::Bj => "BJ",
    }
    .to_string();
    let board = match item.quote.symbol.board {
        BoardKind::Main => "MAIN",
        BoardKind::Chinext => "CHINEXT",
        BoardKind::Star => "STAR",
        BoardKind::Bse => "BSE",
    }
    .to_string();

    ScreenerCandidateDto {
        symbol: canonical,
        name_zh: item.quote.name_zh,
        exchange,
        board,
        industry_sw1: item.industry_sw1,
        latest_price_yuan: round_2(item.quote.latest_yuan),
        pct_chg,
        total_mv_yi: round_2(item.quote.total_mv_yi),
        circ_mv_yi: round_2(item.quote.circ_mv_yi),
        turnover_t0_pct: round_2(item.turnover_t0),
        turnover_t1_pct: round_2(item.turnover_t1),
        projected_turnover_pct: round_2(item.projected_turnover),
        vol_expansion_ratio: round_2(item.vol_expansion),
        main_net_inflow_wan: round_2(item.main_inflow_wan),
        ma20_price_yuan: round_2(item.ma20),
        atr14_yuan: round_2(item.atr14),
        conviction_score,
        suggested_stop_loss_yuan: stop_fen.to_yuan(),
        suggested_target_yuan: target_fen.to_yuan(),
        decision,
        pitfalls_passed_count: item.pit_report.passed_count,
        pitfalls: pitfalls_dto,
        factor_z_scores: scored
            .map(|s| s.factor_z_scores.clone())
            .unwrap_or_default(),
        rationale_zh,
        kline_bar_date: item.bar_date,
    }
}
