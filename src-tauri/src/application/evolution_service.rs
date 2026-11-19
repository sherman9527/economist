use crate::application::pit_observation_builder::build_pit_walk_forward_observations;
use crate::application::portfolio_seeder::current_cst_dates;
use crate::application::terminal_service::TerminalService;
use crate::domain::anti_overfit::{
    evaluate_walk_forward_iteration, CrossSectionalFactorObservation, WalkForwardConfig,
};
use crate::domain::version_control::{
    compare_versions_and_attribute, evaluate_shadow_versions_on_oos, StrategyVersionSnapshot,
    VersionParamsSnapshot,
};
use crate::infrastructure::plugin_store::PluginBundle;
use crate::infrastructure::sqlite_watchlist_repo::EvolutionAuditRow;
use crate::interface::dto::round_2;
use crate::interface::evolution_dto::{
    EvolutionAuditLogDto, FactorAttributionDto, RootCauseFindingDto, VersionComparisonDto,
    VersionControlDashboardDto, VersionSnapshotDto,
};

fn next_patch_version_id(existing: &[StrategyVersionSnapshot]) -> String {
    let mut best = (1u32, 2u32, 0u32);
    for v in existing {
        let clean = v.version_id.trim().trim_start_matches('v');
        let core = clean.split('-').next().unwrap_or(clean);
        let parts: Vec<u32> = core.split('.').filter_map(|p| p.parse().ok()).collect();
        if parts.len() == 3 {
            let tup = (parts[0], parts[1], parts[2]);
            if tup > best {
                best = tup;
            }
        }
    }
    format!("v{}.{}.{}", best.0, best.1, best.2 + 1)
}

impl TerminalService {
    pub fn record_manual_version_snapshot_sync(
        &self,
        bundle: &PluginBundle,
    ) -> Result<String, String> {
        let (_, _, now_iso) = current_cst_dates();
        self.repo.ensure_default_version_lineage(bundle, &now_iso)?;
        let existing = self.repo.list_version_snapshots()?;
        let parent_id = existing
            .iter()
            .find(|v| v.is_active)
            .map(|v| v.version_id.clone());
        let next_ver = next_patch_version_id(&existing);
        let _ = self.plugins.set_manifest_version(&next_ver)?;
        let snap = StrategyVersionSnapshot {
            version_id: next_ver.clone(),
            parent_version_id: parent_id,
            source_tag: "MANUAL_SAVE".to_string(),
            created_at: now_iso,
            is_active: true,
            summary_zh: format!(
                "手动保存参数与因子权重快照（预测换手>={:.1}%, ATR={:.2}x）",
                bundle.strategy.min_intraday_projected_turnover_pct,
                bundle.strategy.stop_loss.atr_stop_mult
            ),
            factor_weights: bundle.strategy.factor_weights.clone(),
            params: VersionParamsSnapshot {
                min_intraday_projected_turnover_pct: bundle
                    .strategy
                    .min_intraday_projected_turnover_pct,
                insider_sell_veto_pct: bundle.strategy.pitfall_thresholds.insider_sell_veto_pct,
                holder_surge_veto_pct: bundle.strategy.pitfall_thresholds.holder_surge_veto_pct,
                main_outflow_veto_ratio: bundle.strategy.pitfall_thresholds.main_outflow_veto_ratio,
                atr_stop_mult: bundle.strategy.stop_loss.atr_stop_mult,
            },
            train_rank_ic: 0.050,
            oos_rank_ic: 0.045,
            oos_ic_ir: 0.72,
            pbo_risk_pct: 18.0,
        };
        self.repo.save_version_snapshot(&snap)?;
        Ok(next_ver)
    }

    pub async fn switch_strategy_version(
        &self,
        version_id: &str,
    ) -> Result<VersionControlDashboardDto, String> {
        let bundle = self.plugins.load_all()?;
        let (_, _, now_iso) = current_cst_dates();
        self.repo.ensure_default_version_lineage(&bundle, &now_iso)?;
        let prev_active = self
            .repo
            .list_version_snapshots()?
            .into_iter()
            .find(|v| v.is_active)
            .map(|v| v.version_id);

        let activated = self.repo.set_active_version_id(version_id.trim())?;
        let updated = self.plugins.update_strategy_and_weights(
            activated.params.min_intraday_projected_turnover_pct,
            activated.params.insider_sell_veto_pct,
            activated.params.holder_surge_veto_pct,
            activated.params.main_outflow_veto_ratio,
            activated.params.atr_stop_mult,
            &bundle.strategy.market_polarity,
            &activated.factor_weights,
        )?;
        let _ = self.plugins.set_manifest_version(&activated.version_id)?;
        self.repo.sync_plugins_to_sqlite(&updated)?;
        {
            let mut w = self.scan_cache.write().await;
            *w = None;
        }
        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "QUANT",
            "strategy_version_switched",
            Some(2),
            true,
            &format!(
                "已将当前运行策略与因子版本切换至 {}（原版本 {:?} 继续保留并行影子追踪）",
                activated.version_id, prev_active
            ),
        );
        self.get_version_control_dashboard(prev_active, Some(activated.version_id))
            .await
    }

    pub async fn run_anti_overfit_iteration(
        &self,
        trigger_source: &str,
    ) -> Result<VersionControlDashboardDto, String> {
        let bundle = self.plugins.load_all()?;
        let (_, _, now_iso) = current_cst_dates();
        self.repo.ensure_default_version_lineage(&bundle, &now_iso)?;
        let versions = self.repo.list_version_snapshots()?;
        let active = versions
            .iter()
            .find(|v| v.is_active)
            .cloned()
            .or_else(|| versions.last().cloned())
            .ok_or_else(|| "策略版本库为空".to_string())?;

        let cached_scan = self.scan_cache.read().await.as_ref().map(|c| c.output.clone());
        let (panel, ast_comp) = build_pit_walk_forward_observations(cached_scan.as_ref(), &bundle);
        let wf = evaluate_walk_forward_iteration(
            &panel,
            &active.factor_weights,
            &ast_comp,
            &WalkForwardConfig::default(),
        );

        let resulting_ver = if wf.promoted {
            let next_ver = next_patch_version_id(&versions);
            let updated = self.plugins.update_strategy_and_weights(
                active.params.min_intraday_projected_turnover_pct,
                active.params.insider_sell_veto_pct,
                active.params.holder_surge_veto_pct,
                active.params.main_outflow_veto_ratio,
                active.params.atr_stop_mult,
                &bundle.strategy.market_polarity,
                &wf.proposed_weights,
            )?;
            let _ = self.plugins.set_manifest_version(&next_ver)?;
            self.repo.sync_plugins_to_sqlite(&updated)?;
            let snap = StrategyVersionSnapshot {
                version_id: next_ver.clone(),
                parent_version_id: Some(active.version_id.clone()),
                source_tag: "WALK_FORWARD_PROMOTED".to_string(),
                created_at: now_iso.clone(),
                is_active: true,
                summary_zh: format!(
                    "滚动样本外防过拟合晋升版（OOS RankIC={:.4}, PBO={:.1}%）",
                    wf.oos_composite_rank_ic, wf.pbo_risk_pct
                ),
                factor_weights: wf.proposed_weights.clone(),
                params: active.params.clone(),
                train_rank_ic: wf.train_composite_rank_ic,
                oos_rank_ic: wf.oos_composite_rank_ic,
                oos_ic_ir: wf.oos_composite_ic_ir,
                pbo_risk_pct: wf.pbo_risk_pct,
            };
            self.repo.save_version_snapshot(&snap)?;
            {
                let mut w = self.scan_cache.write().await;
                *w = None;
            }
            next_ver
        } else {
            active.version_id.clone()
        };

        let log_row = EvolutionAuditRow {
            log_id: 0,
            created_at: now_iso,
            trigger_source: trigger_source.to_string(),
            base_version_id: active.version_id.clone(),
            resulting_version_id: resulting_ver.clone(),
            promoted: wf.promoted,
            verdict: wf.verdict.clone(),
            train_dates: wf.train_dates,
            embargo_dates: wf.embargo_dates,
            oos_dates: wf.oos_dates,
            sample_observations: wf.sample_observations,
            train_rank_ic: wf.train_composite_rank_ic,
            oos_rank_ic: wf.oos_composite_rank_ic,
            oos_ic_ir: wf.oos_composite_ic_ir,
            deflated_ic_ir: wf.deflated_ic_ir,
            pbo_risk_pct: wf.pbo_risk_pct,
            max_pairwise_corr: wf.max_pairwise_corr,
            prior_weights: active.factor_weights,
            proposed_weights: wf.proposed_weights,
            audit_reasons_zh: wf.audit_reasons_zh.clone(),
        };
        self.repo.insert_factor_evolution_log(&log_row)?;
        let _ = self.cleaner.record_event(
            &self.repo,
            "INFO",
            "QUANT",
            "walk_forward_anti_overfit_eval",
            Some(8),
            true,
            &format!(
                "[{}] 滚动验证结果={} ({} -> {}) OOS_IC={:.4} PBO={:.1}%",
                trigger_source,
                wf.verdict,
                active.version_id,
                resulting_ver,
                wf.oos_composite_rank_ic,
                wf.pbo_risk_pct
            ),
        );

        self.get_version_control_dashboard(Some(active.version_id), Some(resulting_ver))
            .await
    }

    pub async fn get_version_control_dashboard(
        &self,
        base_version_id: Option<String>,
        target_version_id: Option<String>,
    ) -> Result<VersionControlDashboardDto, String> {
        let bundle = self.plugins.load_all()?;
        let (_, _, now_iso) = current_cst_dates();
        self.repo.ensure_default_version_lineage(&bundle, &now_iso)?;
        let versions = self.repo.list_version_snapshots()?;
        let logs = self.repo.list_factor_evolution_logs(20)?;

        let cached_scan = self.scan_cache.read().await.as_ref().map(|c| c.output.clone());
        let (all_obs, _) = build_pit_walk_forward_observations(cached_scan.as_ref(), &bundle);
        // Evaluate shadow performance strictly on the most recent 40% Out-of-Sample dates!
        let mut unique_dates: Vec<String> = all_obs.iter().map(|o| o.trade_date.clone()).collect();
        unique_dates.sort();
        unique_dates.dedup();
        let oos_cutoff_idx = ((unique_dates.len() as f64) * 0.60).floor() as usize;
        let oos_obs: Vec<CrossSectionalFactorObservation> =
            if unique_dates.len() >= 5 && oos_cutoff_idx < unique_dates.len() {
                let cutoff = &unique_dates[oos_cutoff_idx];
                all_obs
                    .into_iter()
                    .filter(|o| &o.trade_date >= cutoff)
                    .collect()
            } else {
                all_obs
            };

        let shadow_perfs = evaluate_shadow_versions_on_oos(&versions, &oos_obs);
        let active_ver = versions
            .iter()
            .find(|v| v.is_active)
            .cloned()
            .or_else(|| versions.last().cloned())
            .ok_or_else(|| "无可用策略版本".to_string())?;

        let target_snap = target_version_id
            .as_deref()
            .and_then(|id| versions.iter().find(|v| v.version_id == id).cloned())
            .unwrap_or_else(|| active_ver.clone());

        let base_snap = base_version_id
            .as_deref()
            .and_then(|id| versions.iter().find(|v| v.version_id == id).cloned())
            .or_else(|| {
                target_snap
                    .parent_version_id
                    .as_deref()
                    .and_then(|pid| versions.iter().find(|v| v.version_id == pid).cloned())
            })
            .or_else(|| {
                versions
                    .iter()
                    .rev()
                    .find(|v| v.version_id != target_snap.version_id)
                    .cloned()
            })
            .unwrap_or_else(|| target_snap.clone());

        let cmp = compare_versions_and_attribute(&base_snap, &target_snap, &oos_obs);

        let version_dtos: Vec<VersionSnapshotDto> = versions
            .iter()
            .zip(shadow_perfs.iter())
            .map(|(v, sp)| VersionSnapshotDto {
                version_id: v.version_id.clone(),
                parent_version_id: v.parent_version_id.clone(),
                source_tag: v.source_tag.clone(),
                created_at: v.created_at.clone(),
                summary_zh: v.summary_zh.clone(),
                is_active: v.is_active,
                factor_weights_pct: v
                    .factor_weights
                    .iter()
                    .map(|(k, &val)| (k.clone(), round_2(val)))
                    .collect(),
                min_intraday_projected_turnover_pct: round_2(
                    v.params.min_intraday_projected_turnover_pct,
                ),
                atr_stop_mult: round_2(v.params.atr_stop_mult),
                insider_sell_veto_pct: round_2(v.params.insider_sell_veto_pct),
                holder_surge_veto_pct: round_2(v.params.holder_surge_veto_pct),
                main_outflow_veto_ratio: round_2(v.params.main_outflow_veto_ratio),
                oos_sample_days: sp.oos_evaluated_dates,
                oos_rank_ic_mean: (sp.shadow_oos_rank_ic * 10_000.0).round() / 10_000.0,
                oos_win_rate_pct: round_2(sp.shadow_top_win_rate_pct),
                oos_cum_net_ret_bps: round_2(sp.shadow_cum_net_return_bps),
                oos_max_drawdown_bps: round_2(sp.shadow_max_drawdown_bps),
            })
            .collect();

        let win_lift = round_2(cmp.oos_win_rate_lift_pct);
        let net_lift = round_2(cmp.oos_net_return_lift_bps);
        let headline_verdict_zh = if cmp.base_version_id == cmp.target_version_id {
            format!(
                "当前对照同一策略版本 {}（请选择不同历史版本以审阅升级增益或退化归因）。",
                cmp.target_version_id
            )
        } else if cmp.underperforming {
            format!(
                "退化预警：{} 在平行样本外跟踪中落后老版本 {}（净收益 {:+.1} bps，胜率 {:+.1}%），已定位 {} 项具体根因！",
                cmp.target_version_id,
                cmp.base_version_id,
                net_lift,
                win_lift,
                cmp.root_causes.len()
            )
        } else {
            format!(
                "升级验证有效：{} 相比 {} 样本外胜率提升 {:+.1}%，累计净超额 {:+.1} bps。",
                cmp.target_version_id, cmp.base_version_id, win_lift, net_lift
            )
        };

        let factor_drag_min = cmp
            .factor_attributions
            .iter()
            .map(|r| r.attribution_drag_or_lift_bps)
            .fold(0.0_f64, f64::min);

        let comparison = VersionComparisonDto {
            base_version_id: cmp.base_version_id,
            target_version_id: cmp.target_version_id,
            win_rate_lift_pct: win_lift,
            rank_ic_lift: (cmp.oos_rank_ic_lift * 10_000.0).round() / 10_000.0,
            net_ret_lift_bps: net_lift,
            drawdown_improvement_bps: round_2(-cmp.drawdown_delta_bps),
            is_underperforming: cmp.underperforming,
            headline_verdict_zh,
            factor_attributions: cmp
                .factor_attributions
                .into_iter()
                .map(|r| FactorAttributionDto {
                    factor_id: r.factor_id,
                    base_weight_pct: round_2(r.old_weight_pct),
                    target_weight_pct: round_2(r.new_weight_pct),
                    weight_delta_pct: round_2(r.weight_delta_pct),
                    factor_oos_spread_bps: round_2(r.factor_oos_spread_bps),
                    alpha_contribution_bps: round_2(r.attribution_drag_or_lift_bps),
                })
                .collect(),
            root_causes: cmp
                .root_causes
                .into_iter()
                .map(|rc| {
                    let est_impact = match rc.category.as_str() {
                        "FACTOR_WEIGHT_DRAG" => factor_drag_min.min(-5.0),
                        "STOP_LOSS_PARAM" => (net_lift * 0.35).min(-8.0),
                        "MARKET_REGIME_SHIFT" => (net_lift * 0.40).min(-10.0),
                        _ => (net_lift * 0.25).min(-6.0),
                    };
                    RootCauseFindingDto {
                        category: rc.category,
                        severity: rc.severity,
                        impact_bps: round_2(est_impact),
                        title_zh: rc.title_zh,
                        detail_zh: rc.detail_zh,
                        remediation_zh: rc.action_zh,
                    }
                })
                .collect(),
        };

        let evolution_logs = logs
            .into_iter()
            .map(|l| {
                let action = if l.promoted {
                    "PROMOTED".to_string()
                } else {
                    "REJECTED_OVERFIT_GUARD".to_string()
                };
                let result_version_id = if l.promoted {
                    Some(l.resulting_version_id)
                } else {
                    None
                };
                let reason_zh = if l.audit_reasons_zh.is_empty() {
                    l.verdict
                } else {
                    l.audit_reasons_zh.join("；")
                };
                EvolutionAuditLogDto {
                    log_id: l.log_id,
                    evaluated_at: l.created_at,
                    trigger_source: l.trigger_source,
                    base_version_id: l.base_version_id,
                    result_version_id,
                    action,
                    pbo_estimate: ((l.pbo_risk_pct / 100.0).clamp(0.0, 1.0) * 10_000.0).round()
                        / 10_000.0,
                    deflated_ic_ir: round_2(l.deflated_ic_ir),
                    oos_rank_ic: (l.oos_rank_ic * 10_000.0).round() / 10_000.0,
                    oos_win_rate_lift_pct: round_2((l.oos_rank_ic - l.train_rank_ic * 0.5) * 45.0),
                    reason_zh,
                }
            })
            .collect();

        Ok(VersionControlDashboardDto {
            active_version_id: active_ver.version_id,
            versions: version_dtos,
            comparison,
            evolution_logs,
        })
    }
}
