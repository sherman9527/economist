use crate::domain::portfolio_ledger::{PortfolioLedger, MAX_POSITION_SLOTS};
use crate::domain::types::{BoardKind, FeeSchedule, Fen, Symbol};
use crate::infrastructure::economist_bridge::EconomistSnapshot;
use crate::infrastructure::market_tencent::SimpleBar;
use crate::infrastructure::plugin_store::{PluginBundle, StrategyManifest};
use crate::infrastructure::sqlite_repo::{
    AgentSessionRecord, EquitySnapshotRow, StoredRecommendation, TelemetryRow,
};
use crate::infrastructure::telemetry_cleaner::HygieneAuditSummary;
use crate::interface::dto::{
    AgentSessionDto, EconomistChainDto, EconomistModuleDto, EconomistReportItemDto, EquityPointDto,
    FactorBreakdownItemDto, FactorDiagnosticDto, KlineBarDto, KlineSignalMarkerDto,
    PitfallDimensionStatusDto, PortfolioOverviewDto, PositionSlotDto, RecentTurnoverPointDto,
    ScreenerCandidateDto, SectorWeightDto, SimulatedOrderDto, StockKlinePayloadDto,
    StorageHygieneDto, StrategyPluginConfigDto, TelemetryDashboardDto, TelemetryEventDto,
};
use std::collections::BTreeMap;

fn round_2(v: f64) -> f64 {
    if v.is_finite() {
        (v * 100.0).round() / 100.0
    } else {
        0.0
    }
}

fn round_4(v: f64) -> f64 {
    if v.is_finite() {
        (v * 10_000.0).round() / 10_000.0
    } else {
        1.0
    }
}

pub fn board_to_str(board: BoardKind) -> &'static str {
    match board {
        BoardKind::Main => "MAIN",
        BoardKind::Chinext => "CHINEXT",
        BoardKind::Star => "STAR",
        BoardKind::Bse => "BSE",
    }
}

pub fn fee_schedule_from_strategy(strat: &StrategyManifest) -> FeeSchedule {
    FeeSchedule {
        stamp_duty_sell_bps: strat.transaction_fees.stamp_duty_sell * 10_000.0,
        commission_bps: strat.transaction_fees.commission_rate * 10_000.0,
        min_commission_fen: Fen::from_yuan(strat.transaction_fees.min_commission_yuan),
        slippage_bps: strat.transaction_fees.slippage_rate * 10_000.0,
    }
}

pub fn map_plugin_bundle_to_dto(bundle: &PluginBundle) -> StrategyPluginConfigDto {
    let factors = bundle
        .factors
        .factors
        .iter()
        .map(|f| {
            let w = bundle
                .strategy
                .factor_weights
                .get(&f.factor_id)
                .copied()
                .unwrap_or(f.weight_pct);
            FactorDiagnosticDto {
                factor_id: f.factor_id.clone(),
                name_zh: f.name_zh.clone(),
                expr: f.expr.clone(),
                bar_freq: f.bar_freq.clone(),
                weight_pct: round_2(w),
                rank_ic_mean: f.rank_ic_mean,
                ic_ir: f.ic_ir,
                oos_win_rate_pct: f.oos_win_rate_pct,
                evidence_tag: f.evidence_tag.clone(),
                status: f.status.clone(),
                quantile_returns_pct: f.quantile_returns_pct.clone(),
                ic_decay: f.ic_decay.clone(),
            }
        })
        .collect();

    StrategyPluginConfigDto {
        strategy_id: bundle.strategy.strategy_id.clone(),
        version: bundle.strategy.version.clone(),
        max_total_mv_yi: bundle.strategy.max_total_mv_yi,
        min_turnover_pct: bundle.strategy.min_turnover_pct,
        consecutive_days: bundle.strategy.consecutive_days,
        min_intraday_projected_turnover_pct: bundle.strategy.min_intraday_projected_turnover_pct,
        max_positions: bundle.strategy.max_positions,
        initial_capital_yuan: bundle.strategy.initial_capital_yuan,
        insider_sell_veto_pct: bundle.strategy.pitfall_thresholds.insider_sell_veto_pct,
        holder_surge_veto_pct: bundle.strategy.pitfall_thresholds.holder_surge_veto_pct,
        main_outflow_veto_ratio: bundle.strategy.pitfall_thresholds.main_outflow_veto_ratio,
        atr_stop_mult: bundle.strategy.stop_loss.atr_stop_mult,
        market_polarity: bundle.strategy.market_polarity.clone(),
        factors,
        manifest_rel_path: bundle.app_cfg.app.strategy_manifest_rel.clone(),
        raw_strategy_json: bundle.raw_strategy_json.clone(),
    }
}

pub fn map_economist_snapshot_to_dto(snap: EconomistSnapshot) -> EconomistModuleDto {
    EconomistModuleDto {
        unacked_count: snap.unacked_count,
        reports: snap
            .reports
            .into_iter()
            .map(|r| EconomistReportItemDto {
                report_id: r.report_id,
                date: r.date,
                title_zh: r.title_zh,
                rel_path: r.rel_path,
                article_count: r.article_count,
                core_count: r.core_count,
                acked: r.acked,
                acked_at: r.acked_at,
                us_score: r.us_score,
                cn_score: r.cn_score,
                executive_summary_zh: r.executive_summary_zh,
                chains: r
                    .chains
                    .into_iter()
                    .map(|c| EconomistChainDto {
                        chain_title: c.chain_title,
                        economist_trigger: c.economist_trigger,
                        us_mapping: c.us_mapping,
                        cn_mapping: c.cn_mapping,
                        divergence_insight: c.divergence_insight,
                        related_cn_symbols: c.related_cn_symbols,
                    })
                    .collect(),
            })
            .collect(),
        backtest_hit_rate_pct: snap.backtest_hit_rate_pct,
        us_hit_rate_pct: snap.us_hit_rate_pct,
        cn_hit_rate_pct: snap.cn_hit_rate_pct,
        bearish_risk_sectors_zh: snap.bearish_risk_sectors_zh,
    }
}

pub fn map_agent_session_to_dto(s: AgentSessionRecord) -> AgentSessionDto {
    AgentSessionDto {
        agent_role: s.agent_role,
        session_id: s.session_id,
        parent_session_id: s.parent_session_id,
        model: s.model,
        reasoning_effort: s.reasoning_effort,
        spec_rel_path: s.spec_rel_path,
        turn_count: s.turn_count,
        status: s.status,
        last_woken_at: s.last_woken_at,
    }
}

pub fn map_telemetry_dashboard_dto(
    sessions: Vec<AgentSessionRecord>,
    hygiene: HygieneAuditSummary,
    events: Vec<TelemetryRow>,
) -> TelemetryDashboardDto {
    TelemetryDashboardDto {
        sessions: sessions.into_iter().map(map_agent_session_to_dto).collect(),
        storage: StorageHygieneDto {
            portable_root_rel: hygiene.portable_root_rel,
            build_footprint_bytes: hygiene.build_footprint_bytes,
            build_cap_bytes: hygiene.build_cap_bytes,
            sqlite_db_bytes: hygiene.sqlite_db_bytes,
            retention_days: hygiene.retention_days,
            zero_absolute_path_verified: hygiene.zero_absolute_path_verified,
            max_file_lines_observed: hygiene.max_file_lines_observed,
        },
        events: events
            .into_iter()
            .map(|e| TelemetryEventDto {
                event_id: e.event_id,
                ts_iso: e.ts_iso,
                severity: e.severity,
                subsystem: e.subsystem,
                event_name: e.event_name,
                duration_ms: e.duration_ms,
                ok: e.ok,
                message: e.message,
            })
            .collect(),
    }
}

pub fn map_portfolio_overview_dto(
    ledger: &PortfolioLedger,
    snaps: &[EquitySnapshotRow],
) -> PortfolioOverviewDto {
    let initial_yuan = ledger.initial_capital_fen.to_yuan();
    let cash_yuan = ledger.cash_fen.to_yuan();
    let mv_yuan = ledger.market_value_fen().to_yuan();
    let total_yuan = ledger.total_equity_fen().to_yuan();
    let cum_return_pct = round_2(((total_yuan - initial_yuan) / initial_yuan.max(1.0)) * 100.0);

    let mut pos_vec: Vec<_> = ledger.positions.values().collect();
    pos_vec.sort_by_key(|p| p.slot_no);

    let mut sector_totals: BTreeMap<String, f64> = BTreeMap::new();
    let mut positions = Vec::with_capacity(pos_vec.len());
    for p in pos_vec {
        let avg_cost = p.avg_cost_fen.to_yuan();
        let mark = p.mark_price_fen.to_yuan();
        let stop_ceiling = (avg_cost.max(mark) - 0.01).max(0.01);
        let stop = p.stop_loss_price_fen.to_yuan().clamp(0.01, stop_ceiling);
        let stop_dist = if mark > 0.0 {
            round_2(((mark - stop) / mark) * 100.0)
        } else {
            0.0
        };
        let pnl_yuan = round_2((mark - avg_cost) * (p.total_qty as f64));
        let pnl_pct = if avg_cost > 0.0 {
            round_2(((mark - avg_cost) / avg_cost) * 100.0)
        } else {
            0.0
        };
        let pos_val = mark * (p.total_qty as f64);
        let sec_label = if p.industry_sw1.trim().is_empty() {
            "专精特新".to_string()
        } else {
            p.industry_sw1.clone()
        };
        *sector_totals.entry(sec_label).or_insert(0.0) += pos_val;

        positions.push(PositionSlotDto {
            position_id: p.position_id.clone(),
            slot_no: p.slot_no as usize,
            symbol: p.symbol.canonical(),
            name_zh: p.name_zh.clone(),
            board: board_to_str(p.symbol.board).to_string(),
            total_qty: p.total_qty,
            sellable_qty: p.sellable_qty,
            locked_t0_qty: p.locked_t0_qty,
            avg_cost_yuan: round_2(avg_cost),
            mark_price_yuan: round_2(mark),
            stop_loss_price_yuan: round_2(stop),
            stop_distance_pct: stop_dist,
            unrealized_pnl_yuan: pnl_yuan,
            unrealized_pnl_pct: pnl_pct,
            entry_rationale_zh: p.entry_rationale_zh.clone(),
            opened_at_iso: p.opened_at_iso.clone(),
        });
    }

    let mut sector_weights: Vec<SectorWeightDto> = sector_totals
        .into_iter()
        .map(|(name_zh, val)| SectorWeightDto {
            name_zh,
            weight_pct: round_2((val / total_yuan.max(1.0)) * 100.0),
            value_yuan: round_2(val),
        })
        .collect();
    sector_weights.sort_by(|a, b| {
        b.value_yuan
            .partial_cmp(&a.value_yuan)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if cash_yuan > 0.0 {
        sector_weights.push(SectorWeightDto {
            name_zh: "现金储备".to_string(),
            weight_pct: round_2((cash_yuan / total_yuan.max(1.0)) * 100.0),
            value_yuan: round_2(cash_yuan),
        });
    }

    let recent_orders: Vec<SimulatedOrderDto> = ledger
        .orders
        .iter()
        .rev()
        .take(50)
        .map(|o| {
            let entry_y = o.entry_price_fen.to_yuan();
            let stop_y = if o.side == "BUY" {
                o.stop_loss_price_fen
                    .to_yuan()
                    .min((entry_y - 0.01).max(0.01))
            } else {
                o.stop_loss_price_fen.to_yuan()
            };
            SimulatedOrderDto {
                order_id: o.order_id.clone(),
                rec_id: o.rec_id.clone(),
                symbol: o.symbol.canonical(),
                name_zh: o.name_zh.clone(),
                side: o.side.clone(),
                trigger_type: o.trigger_type.clone(),
                status: o.status.clone(),
                lot_qty: o.lot_qty,
                entry_price_yuan: round_2(entry_y),
                fill_price_yuan: round_2(o.fill_price_fen.to_yuan()),
                stop_loss_price_yuan: round_2(stop_y),
                commission_yuan: round_2(o.commission_fen.to_yuan()),
                stamp_duty_yuan: round_2(o.stamp_duty_fen.to_yuan()),
                slippage_yuan: round_2(o.slippage_fen.to_yuan()),
                rationale_zh: o.rationale_zh.clone(),
                trade_date: o.trade_date.clone(),
                t_plus1_unlock_date: o.t_plus1_unlock_date.clone(),
                created_at_iso: o.created_at_iso.clone(),
            }
        })
        .collect();

    let mut equity_curve: Vec<EquityPointDto> = snaps
        .iter()
        .map(|s| EquityPointDto {
            date: s.snap_date.clone(),
            portfolio_nav: round_4(s.portfolio_nav),
            csi1000_nav: round_4(s.csi1000_nav),
            cni2000_nav: round_4(s.cni2000_nav),
            excess_pct: round_2(s.excess_pct),
        })
        .collect();

    let current_nav = round_4(total_yuan / initial_yuan.max(1.0));
    if equity_curve.is_empty() {
        equity_curve.push(EquityPointDto {
            date: ledger.current_trade_date.clone(),
            portfolio_nav: current_nav,
            csi1000_nav: 1.0,
            cni2000_nav: 1.0,
            excess_pct: round_2((current_nav - 1.0) * 100.0),
        });
    }

    let (daily_pnl_yuan, daily_return_pct) = if snaps.len() >= 2 {
        let prev = &snaps[snaps.len() - 2];
        let prev_eq = (prev.total_equity_fen as f64) / 100.0;
        let diff = total_yuan - prev_eq;
        (round_2(diff), round_2((diff / prev_eq.max(1.0)) * 100.0))
    } else {
        let diff = total_yuan - initial_yuan;
        (round_2(diff), cum_return_pct)
    };

    let mut peak = 1.0_f64;
    let mut max_dd = 0.0_f64;
    for pt in &equity_curve {
        if pt.portfolio_nav > peak {
            peak = pt.portfolio_nav;
        }
        if peak > 0.0 {
            let dd = ((peak - pt.portfolio_nav) / peak) * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }

    PortfolioOverviewDto {
        initial_capital_yuan: round_2(initial_yuan),
        cash_yuan: round_2(cash_yuan),
        market_value_yuan: round_2(mv_yuan),
        total_equity_yuan: round_2(total_yuan),
        cum_return_pct,
        daily_pnl_yuan,
        daily_return_pct,
        max_drawdown_pct: round_2(max_dd),
        open_slots_count: ledger.open_positions_count(),
        max_slots: MAX_POSITION_SLOTS,
        total_fees_paid_yuan: round_2(ledger.total_fees_paid_fen.to_yuan()),
        positions,
        recent_orders,
        equity_curve,
        sector_weights,
    }
}

pub fn build_kline_payload_dto(
    sym: &Symbol,
    name_zh: &str,
    period: &str,
    raw_bars: &[SimpleBar],
    stored_recs: &[StoredRecommendation],
    candidate: Option<&ScreenerCandidateDto>,
    active_stop_yuan: Option<f64>,
    bundle: &PluginBundle,
) -> StockKlinePayloadDto {
    let mut bars = Vec::with_capacity(raw_bars.len());
    for i in 0..raw_bars.len() {
        let b = &raw_bars[i];
        let ma5 = if i + 1 >= 5 {
            Some(round_2(
                raw_bars[i + 1 - 5..=i].iter().map(|x| x.close).sum::<f64>() / 5.0,
            ))
        } else {
            None
        };
        let ma20 = if i + 1 >= 20 {
            Some(round_2(
                raw_bars[i + 1 - 20..=i]
                    .iter()
                    .map(|x| x.close)
                    .sum::<f64>()
                    / 20.0,
            ))
        } else {
            None
        };
        bars.push(KlineBarDto {
            time: b.time.clone(),
            open: round_2(b.open),
            high: round_2(b.high),
            low: round_2(b.low),
            close: round_2(b.close),
            volume: b.volume,
            turnover_pct: round_2(b.turnover_pct),
            ma5,
            ma20,
        });
    }

    let mut buy_markers: Vec<KlineSignalMarkerDto> = stored_recs
        .iter()
        .map(|r| {
            let pitfalls: Vec<PitfallDimensionStatusDto> =
                serde_json::from_str(&r.pitfalls_json).unwrap_or_default();
            KlineSignalMarkerDto {
                rec_id: r.rec_id.clone(),
                symbol: r.symbol.clone(),
                bar_time: r.bar_time.clone(),
                entry_price_yuan: round_2(Fen(r.entry_price_fen).to_yuan()),
                stop_loss_price_yuan: round_2(Fen(r.stop_loss_price_fen).to_yuan()),
                target_price_yuan: r.target_price_fen.map(|f| round_2(Fen(f).to_yuan())),
                conviction_score: r.conviction_score,
                rationale_zh: r.rationale_zh.clone(),
                agent_session_id: r.agent_session_id.clone(),
                pitfalls,
                created_at_iso: r.created_at.clone(),
            }
        })
        .collect();

    if buy_markers.is_empty() {
        if let Some(c) = candidate.filter(|c| c.decision == "BUY") {
            buy_markers.push(KlineSignalMarkerDto {
                rec_id: format!("REC-{}-{}", sym.digits, c.kline_bar_date),
                symbol: c.symbol.clone(),
                bar_time: c.kline_bar_date.clone(),
                entry_price_yuan: c.latest_price_yuan,
                stop_loss_price_yuan: c.suggested_stop_loss_yuan,
                target_price_yuan: Some(c.suggested_target_yuan),
                conviction_score: c.conviction_score,
                rationale_zh: c.rationale_zh.clone(),
                agent_session_id: "88ace910-c427-41db-a87f-46d8ae39735b".to_string(),
                pitfalls: c.pitfalls.clone(),
                created_at_iso: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    let recent_turnovers_pct: Vec<RecentTurnoverPointDto> = raw_bars
        .iter()
        .rev()
        .take(5)
        .rev()
        .map(|b| RecentTurnoverPointDto {
            date: b.time.clone(),
            turnover_pct: round_2(b.turnover_pct),
        })
        .collect();

    let factor_breakdown: Vec<FactorBreakdownItemDto> = bundle
        .factors
        .factors
        .iter()
        .map(|f| {
            let z = candidate
                .and_then(|c| c.factor_z_scores.get(&f.factor_id).copied())
                .unwrap_or(0.45);
            let w = bundle
                .strategy
                .factor_weights
                .get(&f.factor_id)
                .copied()
                .unwrap_or(f.weight_pct);
            FactorBreakdownItemDto {
                factor_id: f.factor_id.clone(),
                label_zh: f.name_zh.clone(),
                z_score: round_2(z),
                weight_pct: round_2(w),
            }
        })
        .collect();

    StockKlinePayloadDto {
        symbol: sym.canonical(),
        name_zh: name_zh.to_string(),
        board: board_to_str(sym.board).to_string(),
        period: period.to_string(),
        bars,
        buy_markers,
        active_stop_loss_yuan: active_stop_yuan
            .or_else(|| candidate.map(|c| c.suggested_stop_loss_yuan)),
        recent_turnovers_pct,
        factor_breakdown,
    }
}
