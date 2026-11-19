use crate::domain::anti_overfit::{spearman_rank_ic, CrossSectionalFactorObservation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionParamsSnapshot {
    pub min_intraday_projected_turnover_pct: f64,
    pub insider_sell_veto_pct: f64,
    pub holder_surge_veto_pct: f64,
    pub main_outflow_veto_ratio: f64,
    pub atr_stop_mult: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyVersionSnapshot {
    pub version_id: String,
    pub parent_version_id: Option<String>,
    pub source_tag: String,
    pub created_at: String,
    pub is_active: bool,
    pub summary_zh: String,
    pub factor_weights: BTreeMap<String, f64>,
    pub params: VersionParamsSnapshot,
    pub train_rank_ic: f64,
    pub oos_rank_ic: f64,
    pub oos_ic_ir: f64,
    pub pbo_risk_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionShadowPerformance {
    pub version_id: String,
    pub parent_version_id: Option<String>,
    pub is_active: bool,
    pub source_tag: String,
    pub summary_zh: String,
    pub created_at: String,
    pub oos_evaluated_dates: usize,
    pub shadow_oos_rank_ic: f64,
    pub shadow_oos_ic_ir: f64,
    pub shadow_top_win_rate_pct: f64,
    pub shadow_cum_net_return_bps: f64,
    pub shadow_mean_net_return_bps: f64,
    pub shadow_max_drawdown_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorAttributionRow {
    pub factor_id: String,
    pub old_weight_pct: f64,
    pub new_weight_pct: f64,
    pub weight_delta_pct: f64,
    pub factor_oos_spread_bps: f64,
    pub attribution_drag_or_lift_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCauseFinding {
    pub category: String,
    pub severity: String,
    pub title_zh: String,
    pub detail_zh: String,
    pub action_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparisonReport {
    pub base_version_id: String,
    pub target_version_id: String,
    pub underperforming: bool,
    pub oos_win_rate_lift_pct: f64,
    pub oos_net_return_lift_bps: f64,
    pub oos_rank_ic_lift: f64,
    pub drawdown_delta_bps: f64,
    pub base_perf: VersionShadowPerformance,
    pub target_perf: VersionShadowPerformance,
    pub factor_attributions: Vec<FactorAttributionRow>,
    pub root_causes: Vec<RootCauseFinding>,
}

fn evaluate_single_version_shadow(
    v: &StrategyVersionSnapshot,
    by_date: &BTreeMap<String, Vec<&CrossSectionalFactorObservation>>,
) -> VersionShadowPerformance {
    let mut daily_ics = Vec::new();
    let mut daily_top_rets = Vec::new();
    let mut top_wins = 0usize;
    let mut top_picks_total = 0usize;

    for rows in by_date.values() {
        if rows.len() < 3 {
            continue;
        }
        let mut scored: Vec<(f64, f64)> = rows
            .iter()
            .map(|r| {
                let s: f64 = v
                    .factor_weights
                    .iter()
                    .map(|(fid, w)| (w / 100.0) * r.factor_z.get(fid).copied().unwrap_or(0.0))
                    .sum();
                (s, r.forward_5d_net_ret_bps)
            })
            .collect();

        let xs: Vec<f64> = scored.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = scored.iter().map(|p| p.1).collect();
        daily_ics.push(spearman_rank_ic(&xs, &ys));

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let k = (scored.len() / 3).max(1).min(3);
        let mut day_sum = 0.0;
        for &(_, mut ret_bps) in scored.iter().take(k) {
            // Reflect tighter ATR stop whipsaw penalty in volatile small-cap regimes
            if v.params.atr_stop_mult < 1.60 && ret_bps > -120.0 && ret_bps < 80.0 {
                ret_bps -= (1.60 - v.params.atr_stop_mult) * 90.0;
            }
            if ret_bps > 0.0 {
                top_wins += 1;
            }
            top_picks_total += 1;
            day_sum += ret_bps;
        }
        daily_top_rets.push(day_sum / (k as f64));
    }

    let n_days = daily_ics.len();
    let mean_ic = if n_days == 0 {
        v.oos_rank_ic
    } else {
        daily_ics.iter().sum::<f64>() / (n_days as f64)
    };
    let ic_ir = if n_days >= 2 {
        let var = daily_ics
            .iter()
            .map(|x| (x - mean_ic) * (x - mean_ic))
            .sum::<f64>()
            / ((n_days - 1) as f64);
        let sd = var.sqrt();
        if sd > 1e-9 {
            (mean_ic / sd).clamp(-5.0, 5.0)
        } else {
            v.oos_ic_ir
        }
    } else {
        v.oos_ic_ir
    };

    let mut cum_bps = 0.0;
    let mut peak_bps = 0.0_f64;
    let mut max_dd_bps = 0.0_f64;
    for &r in &daily_top_rets {
        cum_bps += r;
        peak_bps = peak_bps.max(cum_bps);
        max_dd_bps = max_dd_bps.max(peak_bps - cum_bps);
    }
    let mean_ret_bps = if daily_top_rets.is_empty() {
        0.0
    } else {
        cum_bps / (daily_top_rets.len() as f64)
    };
    let win_rate_pct = if top_picks_total == 0 {
        50.0
    } else {
        (top_wins as f64 / top_picks_total as f64) * 100.0
    };

    VersionShadowPerformance {
        version_id: v.version_id.clone(),
        parent_version_id: v.parent_version_id.clone(),
        is_active: v.is_active,
        source_tag: v.source_tag.clone(),
        summary_zh: v.summary_zh.clone(),
        created_at: v.created_at.clone(),
        oos_evaluated_dates: n_days,
        shadow_oos_rank_ic: mean_ic,
        shadow_oos_ic_ir: ic_ir,
        shadow_top_win_rate_pct: win_rate_pct,
        shadow_cum_net_return_bps: cum_bps,
        shadow_mean_net_return_bps: mean_ret_bps,
        shadow_max_drawdown_bps: max_dd_bps,
    }
}

/// Evaluates ALL historical and current versions side by side on the identical OOS cross-section
/// so legacy versions are continuously shadow-tracked after any upgrade.
pub fn evaluate_shadow_versions_on_oos(
    versions: &[StrategyVersionSnapshot],
    oos_observations: &[CrossSectionalFactorObservation],
) -> Vec<VersionShadowPerformance> {
    let mut by_date: BTreeMap<String, Vec<&CrossSectionalFactorObservation>> = BTreeMap::new();
    for obs in oos_observations {
        by_date.entry(obs.trade_date.clone()).or_default().push(obs);
    }
    versions
        .iter()
        .map(|v| evaluate_single_version_shadow(v, &by_date))
        .collect()
}

/// Compares `v_new` (target) against `v_old` (baseline) on the identical OOS sample,
/// quantifies the exact upgrade lift, and diagnoses root causes when `v_new` underperforms `v_old`.
pub fn compare_versions_and_attribute(
    v_old: &StrategyVersionSnapshot,
    v_new: &StrategyVersionSnapshot,
    oos_observations: &[CrossSectionalFactorObservation],
) -> VersionComparisonReport {
    let mut by_date: BTreeMap<String, Vec<&CrossSectionalFactorObservation>> = BTreeMap::new();
    for obs in oos_observations {
        by_date.entry(obs.trade_date.clone()).or_default().push(obs);
    }

    let base_perf = evaluate_single_version_shadow(v_old, &by_date);
    let target_perf = evaluate_single_version_shadow(v_new, &by_date);

    let oos_win_rate_lift_pct =
        target_perf.shadow_top_win_rate_pct - base_perf.shadow_top_win_rate_pct;
    let oos_net_return_lift_bps =
        target_perf.shadow_cum_net_return_bps - base_perf.shadow_cum_net_return_bps;
    let oos_rank_ic_lift = target_perf.shadow_oos_rank_ic - base_perf.shadow_oos_rank_ic;
    let drawdown_delta_bps =
        target_perf.shadow_max_drawdown_bps - base_perf.shadow_max_drawdown_bps;

    // Compute exact factor-level top-minus-bottom OOS spread (bps) & Brinson weight-shift attribution
    let mut all_fids: Vec<String> = v_old
        .factor_weights
        .keys()
        .chain(v_new.factor_weights.keys())
        .cloned()
        .collect();
    all_fids.sort();
    all_fids.dedup();

    let mut factor_attributions = Vec::with_capacity(all_fids.len());
    for fid in &all_fids {
        let mut spreads = Vec::new();
        for rows in by_date.values() {
            if rows.len() < 4 {
                continue;
            }
            let mut pairs: Vec<(f64, f64)> = rows
                .iter()
                .map(|r| {
                    (
                        r.factor_z.get(fid).copied().unwrap_or(0.0),
                        r.forward_5d_net_ret_bps,
                    )
                })
                .collect();
            pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let k = (pairs.len() / 3).max(1);
            let top_avg = pairs.iter().take(k).map(|p| p.1).sum::<f64>() / (k as f64);
            let bot_avg = pairs.iter().rev().take(k).map(|p| p.1).sum::<f64>() / (k as f64);
            spreads.push(top_avg - bot_avg);
        }
        let oos_spread_bps = if spreads.is_empty() {
            0.0
        } else {
            spreads.iter().sum::<f64>() / (spreads.len() as f64)
        };
        let w_o = v_old.factor_weights.get(fid).copied().unwrap_or(0.0);
        let w_n = v_new.factor_weights.get(fid).copied().unwrap_or(0.0);
        let dw = w_n - w_o;
        let contrib = (dw / 100.0) * oos_spread_bps;
        factor_attributions.push(FactorAttributionRow {
            factor_id: fid.clone(),
            old_weight_pct: w_o,
            new_weight_pct: w_n,
            weight_delta_pct: dw,
            factor_oos_spread_bps: oos_spread_bps,
            attribution_drag_or_lift_bps: contrib,
        });
    }
    factor_attributions.sort_by(|a, b| {
        a.attribution_drag_or_lift_bps
            .partial_cmp(&b.attribution_drag_or_lift_bps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let underperforming = oos_net_return_lift_bps < -0.5
        || oos_rank_ic_lift < -0.005
        || oos_win_rate_lift_pct < -1.0;

    let mut root_causes = Vec::new();
    if underperforming {
        // 1. Factor Weight Shift Drag Attribution
        if let Some(worst) = factor_attributions
            .iter()
            .find(|r| r.attribution_drag_or_lift_bps < -1.0)
        {
            let direction_zh = if worst.weight_delta_pct > 0.0 {
                format!(
                    "新版本将 `{}` 权重从 {:.1}% 上调至 {:.1}%（+{:.1}%），但该因子近期样本外多空收益差为 {:.1} bps（风格失效反转）",
                    worst.factor_id, worst.old_weight_pct, worst.new_weight_pct, worst.weight_delta_pct, worst.factor_oos_spread_bps
                )
            } else {
                format!(
                    "新版本将老版本核心保护因子 `{}` 从 {:.1}% 下调至 {:.1}%（{:.1}%），错失了该因子近期样本外 +{:.1} bps 的防守超额收益",
                    worst.factor_id, worst.old_weight_pct, worst.new_weight_pct, worst.weight_delta_pct, worst.factor_oos_spread_bps
                )
            };
            root_causes.push(RootCauseFinding {
                category: "FACTOR_WEIGHT_DRAG".to_string(),
                severity: "HIGH".to_string(),
                title_zh: format!("因子权重迁移拖累（单日均值 {:.1} bps）", worst.attribution_drag_or_lift_bps),
                detail_zh: direction_zh,
                action_zh: format!(
                    "建议将 `{}` 权重向老版本 {} ({:.1}%) 收缩回归，降低单因子暴露偏离",
                    worst.factor_id, v_old.version_id, worst.old_weight_pct
                ),
            });
        }

        // 2. Market Regime / Upper-Shadow Mean-Reversion Shift Diagnosis
        let avg_upper_shadow = if oos_observations.is_empty() {
            0.0
        } else {
            oos_observations
                .iter()
                .map(|o| o.upper_shadow_ratio)
                .sum::<f64>()
                / (oos_observations.len() as f64)
        };
        if avg_upper_shadow >= 0.34 {
            root_causes.push(RootCauseFinding {
                category: "MARKET_REGIME_SHIFT".to_string(),
                severity: "MEDIUM".to_string(),
                title_zh: "市场微观结构切换：冲高回落长上影线诱多频发".to_string(),
                detail_zh: format!(
                    "近期样本外小市值池平均上影线占比达 {:.1}%（>=34% 分化震荡阈值），高换手追涨动量出现短周期均值回归，老版本防守型权重更具韧性",
                    avg_upper_shadow * 100.0
                ),
                action_zh: "在震荡高上影线体制下恢复老版本低波与收盘强度保护权重".to_string(),
            });
        }

        // 3. Stop-Loss & Pitfall Parameter Diff Attribution
        if (v_new.params.atr_stop_mult - v_old.params.atr_stop_mult).abs() >= 0.15 {
            if v_new.params.atr_stop_mult < v_old.params.atr_stop_mult {
                root_causes.push(RootCauseFinding {
                    category: "STOP_LOSS_PARAM".to_string(),
                    severity: "HIGH".to_string(),
                    title_zh: "ATR 动态止损倍数过紧导致盘中洗盘误止损".to_string(),
                    detail_zh: format!(
                        "新版本将 ATR 止损倍数从 {:.2}x 收紧至 {:.2}x，在 <200亿 高弹性标的正常日内波动中过早触发止损摩擦损耗",
                        v_old.params.atr_stop_mult, v_new.params.atr_stop_mult
                    ),
                    action_zh: format!(
                        "将 ATR 止损倍数恢复至 {:.2}x 附近以避免 T+1 次日噪音洗盘",
                        v_old.params.atr_stop_mult
                    ),
                });
            } else {
                root_causes.push(RootCauseFinding {
                    category: "STOP_LOSS_PARAM".to_string(),
                    severity: "MEDIUM".to_string(),
                    title_zh: "ATR 止损放宽放大单笔尾部回撤".to_string(),
                    detail_zh: format!(
                        "新版本将 ATR 止损倍数从 {:.2}x 放宽至 {:.2}x，弱势破位票止损滞后导致回撤扩大 +{:.1} bps",
                        v_old.params.atr_stop_mult, v_new.params.atr_stop_mult, drawdown_delta_bps.max(0.0)
                    ),
                    action_zh: format!("收紧单票止损纪律回 {:.2}x", v_old.params.atr_stop_mult),
                });
            }
        }

        // 4. In-Sample Overfit Decay Check
        if v_new.train_rank_ic > 0.04
            && target_perf.shadow_oos_rank_ic < 0.45 * v_new.train_rank_ic
        {
            root_causes.push(RootCauseFinding {
                category: "OVERFIT_DECAY".to_string(),
                severity: "HIGH".to_string(),
                title_zh: "样本内拟合衰减（IS → OOS RankIC 断崖回落）".to_string(),
                detail_zh: format!(
                    "新版本训练期 RankIC 高达 {:.4}，但样本外实测回落至 {:.4}（PBO 风险 {:.1}%），属于对近期局部行情的过度调参",
                    v_new.train_rank_ic, target_perf.shadow_oos_rank_ic, v_new.pbo_risk_pct
                ),
                action_zh: format!(
                    "建议立即一键切回经过样本外验证的老版本 `{}` 并提高 James-Stein 收缩强度",
                    v_old.version_id
                ),
            });
        }

        if root_causes.is_empty() {
            root_causes.push(RootCauseFinding {
                category: "SAMPLE_VARIANCE".to_string(),
                severity: "LOW".to_string(),
                title_zh: "短期样本外截面波动导致微幅落后".to_string(),
                detail_zh: format!(
                    "新版本 `{}` 相较 `{}` 样本外净收益差 {:.1} bps、RankIC 差 {:.4}，建议继续并行影子观察至少 10 个交易日后再定夺",
                    v_new.version_id, v_old.version_id, oos_net_return_lift_bps, oos_rank_ic_lift
                ),
                action_zh: format!("可临时切回 `{}` 或维持双版本并行影子追踪", v_old.version_id),
            });
        }
    }

    VersionComparisonReport {
        base_version_id: v_old.version_id.clone(),
        target_version_id: v_new.version_id.clone(),
        underperforming,
        oos_win_rate_lift_pct,
        oos_net_return_lift_bps,
        oos_rank_ic_lift,
        drawdown_delta_bps,
        base_perf,
        target_perf,
        factor_attributions,
        root_causes,
    }
}
