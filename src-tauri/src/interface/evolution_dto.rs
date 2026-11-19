use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddWatchlistInputDto {
    pub symbol: String,
    pub name_zh: String,
    pub board: String,
    pub industry_sw1: String,
    pub entry_price_yuan: f64,
    pub stop_loss_price_yuan: f64,
    pub target_price_yuan: f64,
    pub conviction_score: f64,
    pub source_decision: String,
    pub rationale_zh: String,
    pub factor_z_scores: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistItemDto {
    pub symbol: String,
    pub name_zh: String,
    pub board: String,
    pub industry_sw1: String,
    pub added_trade_date: String,
    pub added_at_iso: String,
    pub entry_price_yuan: f64,
    pub stop_loss_price_yuan: f64,
    pub target_price_yuan: f64,
    pub latest_price_yuan: f64,
    pub peak_price_yuan: f64,
    pub trough_price_yuan: f64,
    pub conviction_score: f64,
    pub source_decision: String,
    pub version_id_at_entry: String,
    pub rationale_zh: String,
    pub holding_trading_days: u32,
    pub net_return_pct: f64,
    pub mfe_pct: f64,
    pub mae_pct: f64,
    pub outcome: String,
    pub settled_trade_date: Option<String>,
    pub settled_price_yuan: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistSummaryDto {
    pub total_count: usize,
    pub active_tracking_count: usize,
    pub settled_count: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub target_hit_count: usize,
    pub stop_hit_count: usize,
    pub settled_win_rate_pct: f64,
    pub live_positive_rate_pct: f64,
    pub wilson_low_pct: f64,
    pub wilson_high_pct: f64,
    pub low_sample_warning: bool,
    pub profit_factor: f64,
    pub expectancy_pct: f64,
    pub high_conviction_win_rate_pct: f64,
    pub baseline_conviction_win_rate_pct: f64,
    pub round_trip_cost_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistOverviewDto {
    pub summary: WatchlistSummaryDto,
    pub items: Vec<WatchlistItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSnapshotDto {
    pub version_id: String,
    pub parent_version_id: Option<String>,
    pub source_tag: String,
    pub created_at: String,
    pub is_active: bool,
    pub summary_zh: String,
    pub factor_weights: BTreeMap<String, f64>,
    pub atr_stop_mult: f64,
    pub min_intraday_projected_turnover_pct: f64,
    pub train_rank_ic: f64,
    pub oos_rank_ic: f64,
    pub oos_ic_ir: f64,
    pub pbo_risk_pct: f64,
    pub shadow_oos_rank_ic: f64,
    pub shadow_oos_ic_ir: f64,
    pub shadow_top_win_rate_pct: f64,
    pub shadow_cum_net_return_bps: f64,
    pub shadow_max_drawdown_bps: f64,
    pub oos_evaluated_dates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorAttributionDto {
    pub factor_id: String,
    pub old_weight_pct: f64,
    pub new_weight_pct: f64,
    pub weight_delta_pct: f64,
    pub factor_oos_spread_bps: f64,
    pub attribution_drag_or_lift_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootCauseFindingDto {
    pub category: String,
    pub severity: String,
    pub title_zh: String,
    pub detail_zh: String,
    pub action_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionComparisonDto {
    pub base_version_id: String,
    pub target_version_id: String,
    pub underperforming: bool,
    pub oos_win_rate_lift_pct: f64,
    pub oos_net_return_lift_bps: f64,
    pub oos_rank_ic_lift: f64,
    pub drawdown_delta_bps: f64,
    pub factor_attributions: Vec<FactorAttributionDto>,
    pub root_causes: Vec<RootCauseFindingDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionAuditLogDto {
    pub log_id: i64,
    pub created_at: String,
    pub trigger_source: String,
    pub base_version_id: String,
    pub resulting_version_id: String,
    pub promoted: bool,
    pub verdict: String,
    pub train_dates: usize,
    pub embargo_dates: usize,
    pub oos_dates: usize,
    pub sample_observations: usize,
    pub train_rank_ic: f64,
    pub oos_rank_ic: f64,
    pub oos_ic_ir: f64,
    pub deflated_ic_ir: f64,
    pub pbo_risk_pct: f64,
    pub max_pairwise_corr: f64,
    pub prior_weights: BTreeMap<String, f64>,
    pub proposed_weights: BTreeMap<String, f64>,
    pub audit_reasons_zh: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionControlDashboardDto {
    pub active_version_id: String,
    pub versions: Vec<VersionSnapshotDto>,
    pub comparison: VersionComparisonDto,
    pub evolution_logs: Vec<EvolutionAuditLogDto>,
}
