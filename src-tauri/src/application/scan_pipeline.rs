use crate::application::scan_factors::{build_factor_vars, compute_ma_and_atr14};
use crate::domain::strategy_gate::MarketSessionState;
use crate::infrastructure::market_eastmoney::ClistEnrichmentMeta;
use crate::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};
use crate::infrastructure::plugin_store::PluginBundle;
use crate::interface::dto::{ScreenerCandidateDto, UniverseFunnelSummaryDto};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ScanExecutionOutput {
    pub funnel: UniverseFunnelSummaryDto,
    pub candidates: Vec<ScreenerCandidateDto>,
    pub raw_quotes: Vec<RawTencentQuote>,
    pub clist_meta: HashMap<String, ClistEnrichmentMeta>,
    pub kline_cache_1d: HashMap<String, Vec<SimpleBar>>,
}

pub async fn execute_live_market_scan(
    http: &reqwest::Client,
    bundle: &PluginBundle,
    bearish_sectors: &[String],
    held_symbols: &[String],
    session_state: MarketSessionState,
    elapsed_minutes: u32,
    _agent_session_id: &str,
) -> Result<ScanExecutionOutput, String> {
    use crate::domain::factor_engine::{
        compute_initial_hard_stop_fen, eval_factor_ast, neutralize_and_score_cross_section,
        parse_factor_expr, CrossSectionSample,
    };
    use crate::domain::pitfall_veto::{
        audit_five_pitfalls, PitfallAuditInput, PitfallDimension, PitfallSeverity,
        PitfallThresholds,
    };
    use crate::domain::strategy_gate::{
        evaluate_strategy_gate, is_st_or_delisting_name, StrategyCandidateInput, StrategyGateParams,
    };
    use crate::domain::types::{is_limit_up_locked, BoardKind, ExchangeCode, Fen};
    use crate::infrastructure::market_eastmoney::{
        fetch_candidate_pitfall_signals, fetch_eastmoney_klines, fetch_top_turnover_clist_meta,
    };
    use crate::infrastructure::market_tencent::{
        fetch_tencent_fallback_klines, fetch_tencent_full_universe,
    };
    use crate::interface::dto::{round_2, PitfallDimensionStatusDto};
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    let started = std::time::Instant::now();
    let src = &bundle.app_cfg.market_sources;

    // 1. Fetch all 5,500+ (~5,931) active A-shares + EastMoney top-turnover clist metadata concurrently
    let (quotes_res, clist_meta) = tokio::join!(
        fetch_tencent_full_universe(
            http,
            &src.tencent_batch_url,
            None,
            src.batch_chunk_size,
            src.max_concurrency,
        ),
        fetch_top_turnover_clist_meta(http, &src.eastmoney_clist_url, 8)
    );
    let raw_quotes = quotes_res?;
    let total_active_count = raw_quotes.len();

    // Build strategy gate parameters from hot-reloadable plugin manifest
    let mut u_weights: Vec<(u16, f64)> = bundle
        .strategy
        .intraday_u_curve_weights
        .iter()
        .filter_map(|(k, &v)| k.parse::<u16>().ok().map(|m| (m, v)))
        .collect();
    u_weights.sort_by_key(|x| x.0);

    let gate_params = StrategyGateParams {
        max_total_mv_yi: 200.0,
        min_turnover_pct: 5.0,
        consecutive_days: 2,
        min_vol_ratio_t1_vs_ma20: bundle.strategy.min_vol_ratio_t1_vs_ma20,
        min_vol_ratio_t0_vs_t1: bundle.strategy.min_vol_ratio_t0_vs_t1,
        min_listing_days: bundle.strategy.min_listing_days,
        limit_up_guard_pct: bundle.strategy.limit_up_guard_pct,
        min_intraday_projected_turnover_pct: bundle.strategy.min_intraday_projected_turnover_pct,
        min_intraday_realized_turnover_pct: bundle.strategy.min_intraday_realized_turnover_pct,
        u_curve_weights: u_weights,
    };

    // 2. Full-universe Stage 1..3 funnel & shortlist top <200亿 high-turnover candidates
    let mut tradable_count = 0_usize;
    let mut under_200_yi_count = 0_usize;
    let mut pre_shortlist: Vec<&RawTencentQuote> = Vec::new();

    let min_t0_threshold = if matches!(
        session_state,
        MarketSessionState::Trading | MarketSessionState::LunchBreak
    ) && elapsed_minutes < 240
    {
        bundle.strategy.min_intraday_realized_turnover_pct.min(2.5)
    } else {
        5.0
    };

    for q in &raw_quotes {
        let is_st = is_st_or_delisting_name(&q.name_zh);
        let listing_days = clist_meta
            .get(&q.symbol.digits)
            .map(|m| m.listing_days)
            .unwrap_or(365);
        let latest_fen = Fen::from_yuan(q.latest_yuan);
        let prev_fen = Fen::from_yuan(q.prev_close_yuan);
        let limit_locked = is_limit_up_locked(
            latest_fen,
            prev_fen,
            q.symbol.board,
            is_st,
            gate_params.limit_up_guard_pct,
        );
        if q.is_suspended
            || is_st
            || latest_fen.0 <= 0
            || listing_days < gate_params.min_listing_days
            || limit_locked
        {
            continue;
        }
        tradable_count += 1;
        if q.total_mv_yi > 0.0 && q.total_mv_yi < 200.0 {
            under_200_yi_count += 1;
            if q.turnover_pct >= min_t0_threshold {
                pre_shortlist.push(q);
            }
        }
    }

    // Sort <200亿 high-turnover shortlist prioritizing 6%..24% healthy ignition band + volume ratio
    pre_shortlist.sort_by(|a, b| {
        let score_q = |q: &RawTencentQuote| -> f64 {
            let band_bonus = if (5.5..=22.0).contains(&q.turnover_pct) {
                20.0
            } else {
                0.0
            };
            band_bonus + q.turnover_pct.min(28.0) + q.vol_ratio.min(5.0) * 2.5
        };
        score_q(b)
            .partial_cmp(&score_q(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_enrich = src.max_enrichment_candidates.clamp(24, 64);
    let mut enrich_quotes: Vec<RawTencentQuote> = pre_shortlist
        .into_iter()
        .take(max_enrich)
        .cloned()
        .collect();

    // Ensure currently held portfolio symbols also get fresh K-lines cached
    for held_sym in held_symbols {
        if !enrich_quotes
            .iter()
            .any(|q| &q.symbol.canonical() == held_sym)
        {
            if let Some(q) = raw_quotes
                .iter()
                .find(|q| &q.symbol.canonical() == held_sym)
            {
                enrich_quotes.push(q.clone());
            }
        }
    }

    // 3. Concurrently fetch daily K-lines (EastMoney push2his f61 -> Tencent fallback) & 5D pitfall signals
    let sem = Arc::new(Semaphore::new(8));
    let mut set = JoinSet::new();
    for q in enrich_quotes {
        let c = http.clone();
        let em_kline = src.eastmoney_kline_url.clone();
        let tc_fq = src.tencent_fqkline_url.clone();
        let tc_mk = src.tencent_mkline_url.clone();
        let dc_url = src.eastmoney_datacenter_url.clone();
        let ann_url = src.eastmoney_notice_url.clone();
        let sem_c = sem.clone();
        set.spawn(async move {
            let _permit = sem_c.acquire_owned().await;
            let kline_fut = async {
                match fetch_eastmoney_klines(&c, &em_kline, &q.symbol, "1d", 45).await {
                    Ok(bars) if bars.len() >= 5 => Ok(bars),
                    _ => {
                        fetch_tencent_fallback_klines(
                            &c,
                            &tc_fq,
                            &tc_mk,
                            &q.symbol,
                            "1d",
                            45,
                            q.circ_mv_yi,
                        )
                        .await
                    }
                }
            };
            let pit_fut = fetch_candidate_pitfall_signals(&c, &dc_url, &ann_url, &q.symbol);
            let (bars_res, pit_signals) = tokio::join!(kline_fut, pit_fut);
            (q, bars_res, pit_signals)
        });
    }

    let parsed_asts: Vec<(String, crate::domain::factor_engine::ExprNode)> = bundle
        .factors
        .factors
        .iter()
        .filter_map(|f| parse_factor_expr(&f.expr).ok().map(|ast| (f.factor_id.clone(), ast)))
        .collect();

    let pit_thresholds = PitfallThresholds {
        insider_sell_veto_pct: bundle.strategy.pitfall_thresholds.insider_sell_veto_pct,
        holder_surge_veto_pct: bundle.strategy.pitfall_thresholds.holder_surge_veto_pct,
        holder_surge_warn_pct: bundle.strategy.pitfall_thresholds.holder_surge_warn_pct,
        main_outflow_veto_ratio: bundle.strategy.pitfall_thresholds.main_outflow_veto_ratio,
        upper_shadow_veto_ratio: bundle.strategy.pitfall_thresholds.upper_shadow_veto_ratio,
        regulatory_veto_keywords: bundle
            .strategy
            .pitfall_thresholds
            .regulatory_veto_keywords
            .clone(),
    };

    struct EnrichedIntermediate {
        quote: RawTencentQuote,
        industry_sw1: String,
        main_inflow_wan: f64,
        turnover_t0: f64,
        turnover_t1: f64,
        projected_turnover: f64,
        vol_expansion: f64,
        ma20: f64,
        atr14: f64,
        prev_low_yuan: f64,
        bar_date: String,
        gate_passed: bool,
        gate_reject_reason: Option<String>,
        pit_report: crate::domain::pitfall_veto::PitfallAuditReport,
    }

    let mut kline_cache_1d: HashMap<String, Vec<SimpleBar>> = HashMap::new();
    let mut intermediates: Vec<EnrichedIntermediate> = Vec::new();
    let mut cs_samples: Vec<CrossSectionSample> = Vec::new();
    let mut used_fallback = false;

    while let Some(joined) = set.join_next().await {
        let Ok((q, bars_res, pit_ext)) = joined else {
            continue;
        };
        let Ok(bars) = bars_res else {
            continue;
        };
        if bars.len() < 3 {
            continue;
        }
        let canonical = q.symbol.canonical();
        let n = bars.len();
        let bar_t0 = &bars[n - 1];
        let bar_t1 = &bars[n - 2];
        if bar_t0.turnover_pct <= 0.0 {
            used_fallback = true;
        }

        let turnover_t0 = if q.turnover_pct > 0.0 {
            q.turnover_pct
        } else {
            bar_t0.turnover_pct
        };
        let turnover_t1 = bar_t1.turnover_pct;
        let (ma20, atr14, vol_ma20) = compute_ma_and_atr14(&bars[..n - 1]);
        let meta = clist_meta.get(&q.symbol.digits);
        let industry_sw1 = meta
            .map(|m| m.industry_sw1.clone())
            .unwrap_or_else(|| "高端装备".to_string());
        let listing_days = meta.map(|m| m.listing_days).unwrap_or(365);
        let main_inflow_wan = meta.map(|m| m.main_net_inflow_wan).unwrap_or(0.0);

        let gate_verdict = evaluate_strategy_gate(
            &StrategyCandidateInput {
                name_zh: q.name_zh.clone(),
                board: q.symbol.board,
                latest_price_fen: Fen::from_yuan(q.latest_yuan),
                prev_close_fen: Fen::from_yuan(q.prev_close_yuan),
                is_suspended: q.is_suspended,
                listing_days,
                total_mv_yi: q.total_mv_yi,
                turnover_t0_pct: turnover_t0,
                turnover_t1_pct: turnover_t1,
                vol_t0: bar_t0.volume.max(q.volume_lots),
                vol_t1: bar_t1.volume,
                vol_ma20_t1: vol_ma20,
                session: session_state,
                elapsed_trading_minutes: (elapsed_minutes as u16).min(240),
            },
            &gate_params,
        );

        let pct_chg = if q.prev_close_yuan > 0.0 {
            ((q.latest_yuan - q.prev_close_yuan) / q.prev_close_yuan) * 100.0
        } else {
            0.0
        };
        let main_inflow_ratio = if q.amount_wan > 0.0 {
            main_inflow_wan / q.amount_wan
        } else {
            0.0
        };

        let pit_report = audit_five_pitfalls(
            &PitfallAuditInput {
                net_insider_reduction_90d_pct: pit_ext.net_insider_reduction_90d_pct,
                announcement_titles_60d: pit_ext.announcement_titles_60d,
                industry_sw1: industry_sw1.clone(),
                economist_bearish_sectors: bearish_sectors.to_vec(),
                holder_num_change_qoq_pct: pit_ext.holder_num_change_qoq_pct,
                holder_notice_date: pit_ext.holder_notice_date,
                main_net_inflow_ratio: main_inflow_ratio,
                open_yuan: if q.open_yuan > 0.0 { q.open_yuan } else { bar_t0.open },
                high_yuan: if q.high_yuan > 0.0 { q.high_yuan } else { bar_t0.high },
                low_yuan: if q.low_yuan > 0.0 { q.low_yuan } else { bar_t0.low },
                close_yuan: q.latest_yuan,
                ma20_yuan: ma20,
                pct_chg,
            },
            &pit_thresholds,
        );

        let vars = build_factor_vars(
            &q,
            &bars,
            turnover_t0,
            turnover_t1,
            ma20,
            atr14,
            gate_verdict.vol_expansion_ratio,
            main_inflow_wan,
        );
        let mut raw_factors = BTreeMap::new();
        for (fid, ast) in &parsed_asts {
            raw_factors.insert(fid.clone(), eval_factor_ast(ast, &vars));
        }
        cs_samples.push(CrossSectionSample {
            symbol: canonical.clone(),
            industry_sw1: industry_sw1.clone(),
            circ_mv_yi: q.circ_mv_yi,
            raw_factors,
        });

        let bar_date = bar_t0.time.clone();
        let prev_low_yuan = bar_t1.low;
        kline_cache_1d.insert(canonical, bars);

        intermediates.push(EnrichedIntermediate {
            quote: q,
            industry_sw1,
            main_inflow_wan,
            turnover_t0,
            turnover_t1,
            projected_turnover: gate_verdict.projected_turnover_pct,
            vol_expansion: gate_verdict.vol_expansion_ratio,
            ma20,
            atr14,
            prev_low_yuan,
            bar_date,
            gate_passed: gate_verdict.consecutive_turnover_vol_passed && gate_verdict.under_200yi,
            gate_reject_reason: gate_verdict.reject_reason_zh,
            pit_report,
        });
    }

    // 4. Cross-sectional 3*MAD winsorization + SW1 industry + ln(circ_mv) neutralization
    let scored_list =
        neutralize_and_score_cross_section(&cs_samples, &bundle.strategy.factor_weights);
    let scored_map: HashMap<String, _> = scored_list
        .into_iter()
        .map(|s| (s.symbol.clone(), s))
        .collect();

    let mut consecutive_turnover_vol_count = 0_usize;
    let mut pitfall_cleared_count = 0_usize;
    let mut qualified_buy_count = 0_usize;
    let mut candidates: Vec<ScreenerCandidateDto> = Vec::with_capacity(intermediates.len());

    for item in intermediates {
        // Skip held large-cap symbols from the <200亿 screener table while keeping their K-lines cached
        if item.quote.total_mv_yi >= 200.0 {
            continue;
        }
        let canonical = item.quote.symbol.canonical();
        let scored = scored_map.get(&canonical);
        let raw_conv = scored.map(|s| s.conviction_score).unwrap_or(60.0);
        let conviction_score =
            round_2((raw_conv - item.pit_report.warn_penalty_pts).clamp(5.0, 99.5));

        if item.gate_passed {
            consecutive_turnover_vol_count += 1;
            if !item.pit_report.vetoed {
                pitfall_cleared_count += 1;
            }
        }

        let decision = if item.pit_report.vetoed {
            "REJECTED_BY_MINE".to_string()
        } else if item.gate_passed && conviction_score >= 54.0 {
            qualified_buy_count += 1;
            "BUY".to_string()
        } else {
            "WATCH".to_string()
        };

        let entry_fen = Fen::from_yuan(item.quote.latest_yuan);
        let stop_fen = compute_initial_hard_stop_fen(
            entry_fen,
            Fen::from_yuan(item.prev_low_yuan),
            Fen::from_yuan(item.atr14),
            bundle.strategy.stop_loss.atr_stop_mult,
        );
        let risk_fen = (entry_fen.0 - stop_fen.0).max(1);
        let target_fen = Fen(entry_fen.0 + (risk_fen as f64 * 2.4).round() as i64);

        let pct_chg = if item.quote.prev_close_yuan > 0.0 {
            round_2(
                ((item.quote.latest_yuan - item.quote.prev_close_yuan)
                    / item.quote.prev_close_yuan)
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

        candidates.push(ScreenerCandidateDto {
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
        });
    }

    // Rank BUY first by convictionScore DESC, then WATCH, then REJECTED_BY_MINE
    candidates.sort_by(|a, b| {
        let rank = |d: &str| match d {
            "BUY" => 0,
            "WATCH" => 1,
            _ => 2,
        };
        rank(&a.decision)
            .cmp(&rank(&b.decision))
            .then_with(|| {
                b.conviction_score
                    .partial_cmp(&a.conviction_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let session_str = match session_state {
        MarketSessionState::PreOpen => "PRE_OPEN",
        MarketSessionState::Trading => "TRADING",
        MarketSessionState::LunchBreak => "LUNCH_BREAK",
        MarketSessionState::Closed => "CLOSED",
    }
    .to_string();

    let funnel = UniverseFunnelSummaryDto {
        total_active_count,
        tradable_count,
        under_200_yi_count,
        consecutive_turnover_vol_count,
        pitfall_cleared_count,
        qualified_buy_count,
        provider_name: if used_fallback {
            "Tencent GTimg 全市场扫描 + 腾讯前复权K线容灾源".to_string()
        } else {
            "Tencent GTimg 5900+全市场批扫 + EastMoney push2his/datacenter".to_string()
        },
        scan_rtt_ms: started.elapsed().as_millis() as u64,
        scanned_at_iso: chrono::Utc::now().to_rfc3339(),
        market_session: session_str,
    };

    Ok(ScanExecutionOutput {
        funnel,
        candidates,
        raw_quotes,
        clist_meta,
        kline_cache_1d,
    })
}
