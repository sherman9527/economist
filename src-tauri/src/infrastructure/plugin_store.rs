use crate::infrastructure::portable_paths::PortablePaths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTomlConfig {
    pub app: AppSection,
    pub schedule: ScheduleSection,
    pub market_sources: MarketSourcesSection,
    pub storage: StorageSection,
    pub agents: AgentsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSection {
    pub name: String,
    pub version: String,
    pub portable_root: String,
    pub default_polarity: String,
    pub default_theme: String,
    pub strategy_manifest_rel: String,
    pub factors_manifest_rel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSection {
    pub timezone: String,
    pub weekdays_only: bool,
    pub trading_windows_cst: Vec<String>,
    pub rotation_interval_minutes: u32,
    pub telemetry_prune_interval_hours: u32,
    pub build_hygiene_interval_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSourcesSection {
    pub tencent_batch_url: String,
    pub tencent_mkline_url: String,
    pub tencent_fqkline_url: String,
    pub eastmoney_kline_url: String,
    pub eastmoney_clist_url: String,
    pub eastmoney_datacenter_url: String,
    pub eastmoney_notice_url: String,
    pub batch_chunk_size: usize,
    pub max_concurrency: usize,
    pub request_timeout_ms: u64,
    pub max_enrichment_candidates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSection {
    pub sqlite_rel_path: String,
    pub ack_state_rel_path: String,
    pub backtest_ledger_rel_path: String,
    pub runs_dir_rel: String,
    pub reports_dir_rel: String,
    pub logs_dir_rel: String,
    pub telemetry_dir_rel: String,
    pub webview_data_rel: String,
    pub temp_dir_rel: String,
    pub retention_days: u32,
    pub build_soft_cap_bytes: u64,
    pub build_cap_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsSection {
    pub locked_model: String,
    pub locked_reasoning_effort: String,
    pub agent_logs_rel: String,
    pub cli_binary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyManifest {
    pub strategy_id: String,
    pub version: String,
    pub description_zh: String,
    pub max_total_mv_yi: f64,
    pub min_turnover_pct: f64,
    pub consecutive_days: u32,
    pub min_vol_ratio_t1_vs_ma20: f64,
    pub min_vol_ratio_t0_vs_t1: f64,
    pub min_listing_days: u32,
    pub limit_up_guard_pct: f64,
    pub intraday_u_curve_weights: BTreeMap<String, f64>,
    pub min_intraday_projected_turnover_pct: f64,
    pub min_intraday_realized_turnover_pct: f64,
    pub pitfall_thresholds: PitfallManifestSection,
    pub initial_capital_yuan: i64,
    pub max_positions: usize,
    pub max_single_weight_pct: f64,
    pub max_single_risk_pct: f64,
    pub max_adv20_participation_pct: f64,
    pub transaction_fees: FeesManifestSection,
    pub stop_loss: StopLossManifestSection,
    pub market_polarity: String,
    pub factor_weights: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitfallManifestSection {
    pub insider_sell_veto_pct: f64,
    pub holder_surge_veto_pct: f64,
    pub holder_surge_warn_pct: f64,
    pub main_outflow_veto_ratio: f64,
    pub upper_shadow_veto_ratio: f64,
    pub regulatory_veto_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeesManifestSection {
    pub stamp_duty_sell: f64,
    pub commission_rate: f64,
    pub min_commission_yuan: f64,
    pub slippage_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossManifestSection {
    pub atr_stop_mult: f64,
    pub hard_stop_min_ratio: f64,
    pub hard_stop_max_ratio: f64,
    pub chandelier_trigger_gain_pct: f64,
    pub chandelier_atr_mult: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorPluginManifest {
    pub plugin_set_id: String,
    pub version: String,
    pub neutralization: String,
    pub factors: Vec<FactorSpecEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSpecEntry {
    pub factor_id: String,
    pub name_zh: String,
    pub expr: String,
    pub bar_freq: String,
    pub weight_pct: f64,
    pub rank_ic_mean: f64,
    pub ic_ir: f64,
    pub oos_win_rate_pct: f64,
    pub evidence_tag: String,
    pub status: String,
    pub quantile_returns_pct: Vec<f64>,
    pub ic_decay: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct PluginBundle {
    pub app_cfg: AppTomlConfig,
    pub strategy: StrategyManifest,
    pub raw_strategy_json: String,
    pub factors: FactorPluginManifest,
}

pub struct PluginStore {
    paths: PortablePaths,
}

const DEFAULT_APP_TOML: &str = include_str!("../../../config/app.toml");
const DEFAULT_STRATEGY_JSON: &str =
    include_str!("../../../plugins/strategies/smallcap_turnover_ignition.json");
const DEFAULT_FACTORS_JSON: &str =
    include_str!("../../../plugins/factors/core_factors_v1.json");

impl PluginStore {
    pub fn new(paths: PortablePaths) -> Self {
        Self { paths }
    }

    fn ensure_defaults_on_disk(&self) -> Result<(), String> {
        let app_toml_path = self.paths.config_dir().join("app.toml");
        if !app_toml_path.is_file() {
            std::fs::write(&app_toml_path, DEFAULT_APP_TOML)
                .map_err(|e| format!("写入默认 config/app.toml 失败: {}", e))?;
        }
        let strat_path = self
            .paths
            .plugins_dir()
            .join("strategies")
            .join("smallcap_turnover_ignition.json");
        if !strat_path.is_file() {
            std::fs::write(&strat_path, DEFAULT_STRATEGY_JSON)
                .map_err(|e| format!("写入默认策略插件失败: {}", e))?;
        }
        let fac_path = self
            .paths
            .plugins_dir()
            .join("factors")
            .join("core_factors_v1.json");
        if !fac_path.is_file() {
            std::fs::write(&fac_path, DEFAULT_FACTORS_JSON)
                .map_err(|e| format!("写入默认因子插件失败: {}", e))?;
        }
        Ok(())
    }

    pub fn load_all(&self) -> Result<PluginBundle, String> {
        use crate::domain::factor_engine::parse_factor_expr;
        use crate::infrastructure::portable_paths::normalize_rel_path;

        self.ensure_defaults_on_disk()?;
        let app_toml_raw = std::fs::read_to_string(self.paths.config_dir().join("app.toml"))
            .map_err(|e| format!("读取 ./config/app.toml 失败: {}", e))?;
        let app_cfg: AppTomlConfig =
            toml::from_str(&app_toml_raw).map_err(|e| format!("解析 app.toml 失败: {}", e))?;

        let strat_rel = normalize_rel_path(&app_cfg.app.strategy_manifest_rel)?;
        let fac_rel = normalize_rel_path(&app_cfg.app.factors_manifest_rel)?;

        let strat_path = self.paths.root().join(strat_rel.trim_start_matches("./"));
        let fac_path = self.paths.root().join(fac_rel.trim_start_matches("./"));

        let raw_strategy_json = std::fs::read_to_string(&strat_path)
            .map_err(|e| format!("读取策略插件 {} 失败: {}", strat_rel, e))?;
        let mut strategy: StrategyManifest = serde_json::from_str(&raw_strategy_json)
            .map_err(|e| format!("解析策略插件 JSON 失败: {}", e))?;

        // Enforce locked core strategy invariants (Req #20 & #21)
        strategy.max_total_mv_yi = 200.0;
        strategy.min_turnover_pct = 5.0;
        strategy.consecutive_days = 2;
        strategy.max_positions = 10;
        strategy.initial_capital_yuan = 1_000_000;

        let raw_factors_json = std::fs::read_to_string(&fac_path)
            .map_err(|e| format!("读取因子插件 {} 失败: {}", fac_rel, e))?;
        let factors: FactorPluginManifest = serde_json::from_str(&raw_factors_json)
            .map_err(|e| format!("解析因子插件 JSON 失败: {}", e))?;

        for f in &factors.factors {
            parse_factor_expr(&f.expr)
                .map_err(|e| format!("因子 {} 表达式安全校验失败: {}", f.factor_id, e))?;
        }

        Ok(PluginBundle {
            app_cfg,
            strategy,
            raw_strategy_json,
            factors,
        })
    }

    pub fn update_strategy_and_weights(
        &self,
        min_intraday_projected_turnover_pct: f64,
        insider_sell_veto_pct: f64,
        holder_surge_veto_pct: f64,
        main_outflow_veto_ratio: f64,
        atr_stop_mult: f64,
        market_polarity: &str,
        factor_weights: &BTreeMap<String, f64>,
    ) -> Result<PluginBundle, String> {
        let mut bundle = self.load_all()?;
        bundle.strategy.min_intraday_projected_turnover_pct =
            min_intraday_projected_turnover_pct.clamp(5.0, 25.0);
        bundle.strategy.pitfall_thresholds.insider_sell_veto_pct =
            insider_sell_veto_pct.clamp(0.05, 5.0);
        bundle.strategy.pitfall_thresholds.holder_surge_veto_pct =
            holder_surge_veto_pct.clamp(5.0, 100.0);
        bundle.strategy.pitfall_thresholds.main_outflow_veto_ratio =
            if main_outflow_veto_ratio > 0.0 {
                -main_outflow_veto_ratio.clamp(0.01, 0.40)
            } else {
                main_outflow_veto_ratio.clamp(-0.40, -0.01)
            };
        bundle.strategy.stop_loss.atr_stop_mult = atr_stop_mult.clamp(1.0, 4.0);
        bundle.strategy.market_polarity = if market_polarity == "intl-green-up" {
            "intl-green-up".to_string()
        } else {
            "cn-red-up".to_string()
        };

        if !factor_weights.is_empty() {
            let sum: f64 = factor_weights.values().map(|v| v.max(0.0)).sum();
            if sum > 1e-6 {
                for (k, v) in factor_weights {
                    let norm = ((v.max(0.0) / sum) * 1000.0).round() / 10.0;
                    bundle.strategy.factor_weights.insert(k.clone(), norm);
                }
                for f in &mut bundle.factors.factors {
                    if let Some(&w) = bundle.strategy.factor_weights.get(&f.factor_id) {
                        f.weight_pct = w;
                    }
                }
            }
        }

        let strat_rel = bundle
            .app_cfg
            .app
            .strategy_manifest_rel
            .trim_start_matches("./");
        let fac_rel = bundle
            .app_cfg
            .app
            .factors_manifest_rel
            .trim_start_matches("./");

        let strat_json = serde_json::to_string_pretty(&bundle.strategy)
            .map_err(|e| format!("序列化策略配置失败: {}", e))?;
        let fac_json = serde_json::to_string_pretty(&bundle.factors)
            .map_err(|e| format!("序列化因子配置失败: {}", e))?;

        std::fs::write(self.paths.root().join(strat_rel), &strat_json)
            .map_err(|e| format!("落盘策略配置失败: {}", e))?;
        std::fs::write(self.paths.root().join(fac_rel), &fac_json)
            .map_err(|e| format!("落盘因子配置失败: {}", e))?;

        bundle.raw_strategy_json = strat_json;
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_workspace_manifests_and_validates_all_six_factor_asts() {
        let paths = PortablePaths::resolve().unwrap();
        let store = PluginStore::new(paths);
        let bundle = store.load_all().unwrap();
        assert_eq!(bundle.strategy.strategy_id, "smallcap_turnover_ignition");
        assert_eq!(bundle.strategy.max_total_mv_yi, 200.0);
        assert_eq!(bundle.strategy.max_positions, 10);
        assert_eq!(bundle.factors.factors.len(), 6);
    }
}
