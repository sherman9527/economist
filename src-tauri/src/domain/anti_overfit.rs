use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Computes exact Spearman Rank Information Coefficient `[-1.0, 1.0]` with mid-ranks for ties.
pub fn spearman_rank_ic(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len());
    if n < 3 {
        return 0.0;
    }
    let rank_vec = |vals: &[f64]| -> Vec<f64> {
        let mut idx: Vec<(usize, f64)> = vals[..n]
            .iter()
            .enumerate()
            .map(|(i, &v)| (i, if v.is_finite() { v } else { 0.0 }))
            .collect();
        idx.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut ranks = vec![0.0; n];
        let mut i = 0;
        while i < n {
            let mut j = i + 1;
            while j < n && (idx[j].1 - idx[i].1).abs() <= 1e-12 {
                j += 1;
            }
            let avg_rank = ((i + j - 1) as f64) * 0.5 + 1.0;
            for k in i..j {
                ranks[idx[k].0] = avg_rank;
            }
            i = j;
        }
        ranks
    };

    let rx = rank_vec(xs);
    let ry = rank_vec(ys);
    let mx = rx.iter().sum::<f64>() / (n as f64);
    let my = ry.iter().sum::<f64>() / (n as f64);
    let mut cov = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    for i in 0..n {
        let dx = rx[i] - mx;
        let dy = ry[i] - my;
        cov += dx * dy;
        vx += dx * dx;
        vy += dy * dy;
    }
    let denom = (vx * vy).sqrt();
    if denom <= 1e-12 {
        0.0
    } else {
        (cov / denom).clamp(-1.0, 1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSectionalFactorObservation {
    pub trade_date: String,
    pub symbol: String,
    pub factor_z: BTreeMap<String, f64>,
    pub forward_5d_net_ret_bps: f64,
    pub upper_shadow_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardConfig {
    pub train_ratio: f64,
    pub embargo_trading_days: usize,
    pub min_oos_rank_ic: f64,
    pub min_oos_ic_ir: f64,
    pub max_pbo_risk_pct: f64,
    pub max_collinearity_abs: f64,
    pub shrinkage_lambda: f64,
    pub max_step_pct: f64,
    pub min_weight_pct: f64,
    pub max_weight_pct: f64,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            train_ratio: 0.60,
            embargo_trading_days: 5,
            min_oos_rank_ic: 0.012,
            min_oos_ic_ir: 0.25,
            max_pbo_risk_pct: 35.0,
            max_collinearity_abs: 0.65,
            shrinkage_lambda: 0.25,
            max_step_pct: 3.0,
            min_weight_pct: 5.0,
            max_weight_pct: 35.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorWalkForwardMetric {
    pub factor_id: String,
    pub train_rank_ic: f64,
    pub oos_rank_ic: f64,
    pub oos_ic_ir: f64,
    pub sign_consistent: bool,
    pub collinearity_penalty: f64,
    pub complexity_penalty: f64,
    pub prior_weight_pct: f64,
    pub proposed_weight_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardIterationReport {
    pub promoted: bool,
    pub verdict: String,
    pub total_dates: usize,
    pub train_dates: usize,
    pub embargo_dates: usize,
    pub oos_dates: usize,
    pub sample_observations: usize,
    pub train_composite_rank_ic: f64,
    pub oos_composite_rank_ic: f64,
    pub oos_composite_ic_ir: f64,
    pub deflated_ic_ir: f64,
    pub pbo_risk_pct: f64,
    pub max_pairwise_corr: f64,
    pub proposed_weights: BTreeMap<String, f64>,
    pub factor_metrics: Vec<FactorWalkForwardMetric>,
    pub audit_reasons_zh: Vec<String>,
}

fn mean_and_ir(ics: &[f64]) -> (f64, f64) {
    if ics.is_empty() {
        return (0.0, 0.0);
    }
    let n = ics.len() as f64;
    let mean = ics.iter().sum::<f64>() / n;
    if ics.len() < 2 {
        return (mean, if mean > 0.0 { 0.5 } else { -0.5 });
    }
    let var = ics
        .iter()
        .map(|x| (x - mean) * (x - mean))
        .sum::<f64>()
        / (n - 1.0);
    let sd = var.sqrt();
    let ir = if sd < 1e-9 {
        if mean.abs() > 1e-6 {
            mean.signum() * 3.0
        } else {
            0.0
        }
    } else {
        (mean / sd).clamp(-5.0, 5.0)
    };
    (mean, ir)
}

fn daily_factor_ics(
    by_date: &BTreeMap<String, Vec<&CrossSectionalFactorObservation>>,
    dates: &[String],
    factor_id: &str,
) -> Vec<f64> {
    let mut out = Vec::new();
    for d in dates {
        let Some(rows) = by_date.get(d) else {
            continue;
        };
        if rows.len() < 4 {
            continue;
        }
        let mut xs = Vec::with_capacity(rows.len());
        let mut ys = Vec::with_capacity(rows.len());
        for r in rows {
            xs.push(r.factor_z.get(factor_id).copied().unwrap_or(0.0));
            ys.push(r.forward_5d_net_ret_bps);
        }
        out.push(spearman_rank_ic(&xs, &ys));
    }
    out
}

fn daily_composite_ics(
    by_date: &BTreeMap<String, Vec<&CrossSectionalFactorObservation>>,
    dates: &[String],
    weights: &BTreeMap<String, f64>,
) -> Vec<f64> {
    let mut out = Vec::new();
    for d in dates {
        let Some(rows) = by_date.get(d) else {
            continue;
        };
        if rows.len() < 4 {
            continue;
        }
        let mut xs = Vec::with_capacity(rows.len());
        let mut ys = Vec::with_capacity(rows.len());
        for r in rows {
            let score: f64 = weights
                .iter()
                .map(|(fid, w)| (w / 100.0) * r.factor_z.get(fid).copied().unwrap_or(0.0))
                .sum();
            xs.push(score);
            ys.push(r.forward_5d_net_ret_bps);
        }
        out.push(spearman_rank_ic(&xs, &ys));
    }
    out
}

fn project_bounded_simplex(
    raw: &BTreeMap<String, f64>,
    prior: &BTreeMap<String, f64>,
    min_w: f64,
    max_w: f64,
    max_step: f64,
) -> BTreeMap<String, f64> {
    let n = prior.len().max(1) as f64;
    let eff_min_w = min_w.min(100.0 / n);
    let eff_max_w = max_w.max((100.0 - (n - 1.0) * eff_min_w).min(100.0 / n + 15.0));

    let mut cur = BTreeMap::new();
    let mut bounds = BTreeMap::new();
    for (k, &p) in prior {
        let lo = (p - max_step).max(eff_min_w);
        let hi = (p + max_step).min(eff_max_w).max(lo);
        let v = raw.get(k).copied().unwrap_or(p).clamp(lo, hi);
        cur.insert(k.clone(), v);
        bounds.insert(k.clone(), (lo, hi));
    }

    // Iterative water-filling projection onto { sum(w) == 100.0, lo_i <= w_i <= hi_i }
    for _ in 0..24 {
        let sum: f64 = cur.values().sum();
        let diff = 100.0 - sum;
        if diff.abs() < 1e-9 {
            break;
        }
        let free_keys: Vec<String> = cur
            .iter()
            .filter(|(k, &v)| {
                let (lo, hi) = bounds[*k];
                if diff > 0.0 {
                    v < hi - 1e-9
                } else {
                    v > lo + 1e-9
                }
            })
            .map(|(k, _)| k.clone())
            .collect();
        if free_keys.is_empty() {
            break;
        }
        let delta = diff / (free_keys.len() as f64);
        for k in free_keys {
            let (lo, hi) = bounds[&k];
            if let Some(v) = cur.get_mut(&k) {
                *v = (*v + delta).clamp(lo, hi);
            }
        }
    }
    cur
}

pub fn evaluate_walk_forward_iteration(
    observations: &[CrossSectionalFactorObservation],
    prior_weights: &BTreeMap<String, f64>,
    factor_ast_complexity: &BTreeMap<String, usize>,
    cfg: &WalkForwardConfig,
) -> WalkForwardIterationReport {
    let mut by_date: BTreeMap<String, Vec<&CrossSectionalFactorObservation>> = BTreeMap::new();
    for obs in observations {
        by_date.entry(obs.trade_date.clone()).or_default().push(obs);
    }
    let all_dates: Vec<String> = by_date.keys().cloned().collect();
    let total_dates = all_dates.len();

    let embargo = cfg.embargo_trading_days.max(1);
    let min_required_dates = embargo + 9; // >= 5 Train + embargo + >= 4 OOS
    if total_dates < min_required_dates || prior_weights.is_empty() {
        return WalkForwardIterationReport {
            promoted: false,
            verdict: "INSUFFICIENT_SAMPLE".to_string(),
            total_dates,
            train_dates: 0,
            embargo_dates: embargo,
            oos_dates: 0,
            sample_observations: observations.len(),
            train_composite_rank_ic: 0.0,
            oos_composite_rank_ic: 0.0,
            oos_composite_ic_ir: 0.0,
            deflated_ic_ir: 0.0,
            pbo_risk_pct: 100.0,
            max_pairwise_corr: 0.0,
            proposed_weights: prior_weights.clone(),
            factor_metrics: Vec::new(),
            audit_reasons_zh: vec![format!(
                "有效交易截面仅 {} 日（低于 {} 日非重叠隔离带所需最低 {} 日门槛），严禁缩短隔离带偷看未来收益",
                total_dates, embargo, min_required_dates
            )],
        };
    }

    let usable = total_dates.saturating_sub(embargo);
    let train_len = ((usable as f64 * cfg.train_ratio).round() as usize)
        .max(5)
        .min(usable.saturating_sub(4));
    let oos_start = train_len + embargo;

    let train_dates = &all_dates[..train_len];
    let oos_dates = &all_dates[oos_start..];

    // Pairwise factor correlation on Train cross-section to penalize collinearity
    let factor_ids: Vec<String> = prior_weights.keys().cloned().collect();
    let mut max_pairwise_corr = 0.0_f64;
    let mut collinearity_penalties: BTreeMap<String, f64> = BTreeMap::new();

    for (i, fid_a) in factor_ids.iter().enumerate() {
        let mut excess_sum = 0.0_f64;
        let vec_a: Vec<f64> = train_dates
            .iter()
            .filter_map(|d| by_date.get(d))
            .flat_map(|rows| rows.iter())
            .map(|r| r.factor_z.get(fid_a).copied().unwrap_or(0.0))
            .collect();
        for (j, fid_b) in factor_ids.iter().enumerate() {
            if i == j {
                continue;
            }
            let vec_b: Vec<f64> = train_dates
                .iter()
                .filter_map(|d| by_date.get(d))
                .flat_map(|rows| rows.iter())
                .map(|r| r.factor_z.get(fid_b).copied().unwrap_or(0.0))
                .collect();
            let corr = spearman_rank_ic(&vec_a, &vec_b).abs();
            if corr > max_pairwise_corr {
                max_pairwise_corr = corr;
            }
            if corr > cfg.max_collinearity_abs {
                excess_sum += corr - cfg.max_collinearity_abs;
            }
        }
        collinearity_penalties.insert(fid_a.clone(), 1.0 / (1.0 + 1.5 * excess_sum));
    }

    // Score each factor: candidate target weights MUST be fitted 100% strictly on `train_dates`
    // (zero OOS snooping!). `oos_dates` is held out purely for validation & veto guards below.
    let mut raw_signal_scores: BTreeMap<String, f64> = BTreeMap::new();
    let mut metrics_draft = Vec::with_capacity(factor_ids.len());

    for fid in &factor_ids {
        let tr_ics = daily_factor_ics(&by_date, train_dates, fid);
        let oos_ics = daily_factor_ics(&by_date, oos_dates, fid);
        let (tr_mean, tr_ir) = mean_and_ir(&tr_ics);
        let (oos_mean, oos_ir) = mean_and_ir(&oos_ics);
        let sign_consistent = tr_mean > 0.0 && oos_mean > 0.0;
        let col_pen = collinearity_penalties.get(fid).copied().unwrap_or(1.0);
        let ast_ops = factor_ast_complexity.get(fid).copied().unwrap_or(4);
        let comp_pen = if ast_ops <= 8 {
            1.0
        } else {
            1.0 / (1.0 + 0.08 * ((ast_ops - 8) as f64))
        };

        // Strictly Train-only edge estimation (NO oos_mean / oos_ir allowed here!)
        let train_only_edge = if tr_mean >= 0.005 && tr_ir > 0.0 {
            (tr_ir.max(0.05) + tr_mean * 6.0) * col_pen * comp_pen
        } else {
            0.05 * col_pen * comp_pen
        };
        raw_signal_scores.insert(fid.clone(), train_only_edge);
        metrics_draft.push((
            fid.clone(),
            tr_mean,
            oos_mean,
            oos_ir,
            sign_consistent,
            col_pen,
            comp_pen,
        ));
    }

    let raw_sum: f64 = raw_signal_scores.values().sum::<f64>().max(1e-9);
    let mut shrunk_target = BTreeMap::new();
    for (fid, &prior_w) in prior_weights {
        let oos_target_w = (raw_signal_scores[fid] / raw_sum) * 100.0;
        let blended = (1.0 - cfg.shrinkage_lambda) * prior_w + cfg.shrinkage_lambda * oos_target_w;
        shrunk_target.insert(fid.clone(), blended);
    }

    let candidate_weights = project_bounded_simplex(
        &shrunk_target,
        prior_weights,
        cfg.min_weight_pct,
        cfg.max_weight_pct,
        cfg.max_step_pct,
    );

    // Evaluate composite IS & OOS RankIC of candidate weights vs prior weights
    let cand_tr_ics = daily_composite_ics(&by_date, train_dates, &candidate_weights);
    let cand_oos_ics = daily_composite_ics(&by_date, oos_dates, &candidate_weights);
    let prior_oos_ics = daily_composite_ics(&by_date, oos_dates, prior_weights);

    let (train_comp_ic, _) = mean_and_ir(&cand_tr_ics);
    let (oos_comp_ic, oos_comp_ir) = mean_and_ir(&cand_oos_ics);
    let (prior_oos_ic, _) = mean_and_ir(&prior_oos_ics);

    // Bailey & López de Prado multiple-testing haircut & PBO risk estimate
    let dof_haircut = ((2.0 * (factor_ids.len().max(2) as f64).ln()).sqrt())
        / ((oos_dates.len().max(2) as f64).sqrt());
    let deflated_ic_ir = oos_comp_ir - 0.35 * dof_haircut;

    let neg_oos_days = cand_oos_ics.iter().filter(|&&x| x <= 0.0).count();
    let sign_flip_factors = metrics_draft
        .iter()
        .filter(|(_, tr, oos, _, _, _, _)| (*tr > 0.01 && *oos < 0.0) || (*tr < -0.01 && *oos > 0.0))
        .count();
    let neg_day_ratio = if cand_oos_ics.is_empty() {
        1.0
    } else {
        neg_oos_days as f64 / cand_oos_ics.len() as f64
    };
    let flip_ratio = sign_flip_factors as f64 / (factor_ids.len().max(1) as f64);
    let pbo_risk_pct = ((0.55 * neg_day_ratio + 0.45 * flip_ratio) * 100.0).clamp(0.0, 100.0);

    let mut reasons = Vec::new();
    let mut vetoed = false;

    if train_comp_ic * oos_comp_ic <= 0.0 || oos_comp_ic < cfg.min_oos_rank_ic {
        vetoed = true;
        reasons.push(format!(
            "样本外复合 RankIC={:.4}（训练集={:.4}）未达 {:.4} 门槛或发生符号反转，判定存在过拟合风险",
            oos_comp_ic, train_comp_ic, cfg.min_oos_rank_ic
        ));
    }
    if oos_comp_ir < cfg.min_oos_ic_ir || deflated_ic_ir <= 0.0 {
        vetoed = true;
        reasons.push(format!(
            "多重检验惩罚后 Deflated IC_IR={:.2}（原始 OOS IC_IR={:.2}）统计显著性不足",
            deflated_ic_ir, oos_comp_ir
        ));
    }
    if pbo_risk_pct > cfg.max_pbo_risk_pct {
        vetoed = true;
        reasons.push(format!(
            "PBO 过拟合风险率 {:.1}% > 红线 {:.1}%，自动熔断晋升",
            pbo_risk_pct, cfg.max_pbo_risk_pct
        ));
    }
    let max_weight_shift = candidate_weights
        .iter()
        .map(|(k, &w)| (w - prior_weights.get(k).copied().unwrap_or(w)).abs())
        .fold(0.0_f64, f64::max);
    if max_weight_shift < 0.05 {
        vetoed = true;
        reasons.push("候选权重与现行版本几乎一致（最大权重偏移 < 0.05%），无需派生冗余新版本".to_string());
    } else if oos_comp_ic + 1e-6 < prior_oos_ic {
        vetoed = true;
        reasons.push(format!(
            "候选权重样本外 RankIC ({:.4}) 未优于现行基线 ({:.4})，坚守原版本",
            oos_comp_ic, prior_oos_ic
        ));
    }

    let final_weights = if vetoed {
        prior_weights.clone()
    } else {
        reasons.push(format!(
            "通过 60% 训练 + {} 日隔离带 + 40% 样本外五重护栏（OOS RankIC={:.4}, PBO={:.1}%），执行 λ={:.2} 收缩平滑升级",
            embargo, oos_comp_ic, pbo_risk_pct, cfg.shrinkage_lambda
        ));
        candidate_weights
    };

    let factor_metrics = metrics_draft
        .into_iter()
        .map(
            |(fid, tr_ic, oos_ic, oos_ir, sign_ok, col_pen, comp_pen)| FactorWalkForwardMetric {
                prior_weight_pct: prior_weights.get(&fid).copied().unwrap_or(0.0),
                proposed_weight_pct: final_weights.get(&fid).copied().unwrap_or(0.0),
                factor_id: fid,
                train_rank_ic: tr_ic,
                oos_rank_ic: oos_ic,
                oos_ic_ir: oos_ir,
                sign_consistent: sign_ok,
                collinearity_penalty: col_pen,
                complexity_penalty: comp_pen,
            },
        )
        .collect();

    WalkForwardIterationReport {
        promoted: !vetoed,
        verdict: if vetoed {
            "REJECTED_OVERFIT_GUARD".to_string()
        } else {
            "PROMOTED".to_string()
        },
        total_dates,
        train_dates: train_dates.len(),
        embargo_dates: embargo,
        oos_dates: oos_dates.len(),
        sample_observations: observations.len(),
        train_composite_rank_ic: train_comp_ic,
        oos_composite_rank_ic: oos_comp_ic,
        oos_composite_ic_ir: oos_comp_ir,
        deflated_ic_ir,
        pbo_risk_pct,
        max_pairwise_corr,
        proposed_weights: final_weights,
        factor_metrics,
        audit_reasons_zh: reasons,
    }
}
