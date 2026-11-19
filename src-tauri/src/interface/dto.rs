use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitfallDimensionStatusDto {
    pub dimension: String,
    pub label_zh: String,
    pub passed: bool,
    pub severity: String,
    pub detail_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerCandidateDto {
    pub symbol: String,
    pub name_zh: String,
    pub exchange: String,
    pub board: String,
    #[serde(rename = "industrySw1")]
    pub industry_sw1: String,
    pub latest_price_yuan: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_close_yuan: Option<f64>,
    pub pct_chg: f64,
    pub total_mv_yi: f64,
    pub circ_mv_yi: f64,
    #[serde(rename = "turnoverT0Pct")]
    pub turnover_t0_pct: f64,
    #[serde(rename = "turnoverT1Pct")]
    pub turnover_t1_pct: f64,
    pub projected_turnover_pct: f64,
    pub vol_expansion_ratio: f64,
    pub main_net_inflow_wan: f64,
    #[serde(rename = "ma20PriceYuan")]
    pub ma20_price_yuan: f64,
    #[serde(rename = "atr14Yuan")]
    pub atr14_yuan: f64,
    pub conviction_score: f64,
    pub suggested_stop_loss_yuan: f64,
    pub suggested_target_yuan: f64,
    pub decision: String,
    pub pitfalls_passed_count: usize,
    pub pitfalls: Vec<PitfallDimensionStatusDto>,
    #[serde(rename = "factorZScores")]
    pub factor_z_scores: BTreeMap<String, f64>,
    pub rationale_zh: String,
    pub kline_bar_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniverseFunnelSummaryDto {
    pub total_active_count: usize,
    pub tradable_count: usize,
    #[serde(rename = "under200YiCount")]
    pub under_200_yi_count: usize,
    pub consecutive_turnover_vol_count: usize,
    pub pitfall_cleared_count: usize,
    pub qualified_buy_count: usize,
    pub provider_name: String,
    pub scan_rtt_ms: u64,
    pub scanned_at_iso: String,
    pub market_session: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KlineBarDto {
    pub time: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub turnover_pct: f64,
    #[serde(rename = "ma5")]
    pub ma5: Option<f64>,
    #[serde(rename = "ma20")]
    pub ma20: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KlineSignalMarkerDto {
    pub rec_id: String,
    pub symbol: String,
    pub bar_time: String,
    pub entry_price_yuan: f64,
    pub stop_loss_price_yuan: f64,
    pub target_price_yuan: Option<f64>,
    pub conviction_score: f64,
    pub rationale_zh: String,
    pub agent_session_id: String,
    pub pitfalls: Vec<PitfallDimensionStatusDto>,
    pub created_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTurnoverPointDto {
    pub date: String,
    pub turnover_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorBreakdownItemDto {
    pub factor_id: String,
    pub label_zh: String,
    pub z_score: f64,
    pub weight_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockKlinePayloadDto {
    pub symbol: String,
    pub name_zh: String,
    pub board: String,
    pub period: String,
    pub bars: Vec<KlineBarDto>,
    pub buy_markers: Vec<KlineSignalMarkerDto>,
    pub active_stop_loss_yuan: Option<f64>,
    pub recent_turnovers_pct: Vec<RecentTurnoverPointDto>,
    pub factor_breakdown: Vec<FactorBreakdownItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSlotDto {
    pub position_id: String,
    pub slot_no: usize,
    pub symbol: String,
    pub name_zh: String,
    pub board: String,
    pub total_qty: i64,
    pub sellable_qty: i64,
    #[serde(rename = "lockedT0Qty")]
    pub locked_t0_qty: i64,
    pub avg_cost_yuan: f64,
    pub mark_price_yuan: f64,
    pub stop_loss_price_yuan: f64,
    pub stop_distance_pct: f64,
    pub unrealized_pnl_yuan: f64,
    pub unrealized_pnl_pct: f64,
    pub entry_rationale_zh: String,
    pub opened_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatedOrderDto {
    pub order_id: String,
    pub rec_id: Option<String>,
    pub symbol: String,
    pub name_zh: String,
    pub side: String,
    pub trigger_type: String,
    pub status: String,
    pub lot_qty: i64,
    pub entry_price_yuan: f64,
    pub fill_price_yuan: f64,
    pub stop_loss_price_yuan: f64,
    pub commission_yuan: f64,
    pub stamp_duty_yuan: f64,
    pub slippage_yuan: f64,
    pub rationale_zh: String,
    pub trade_date: String,
    #[serde(rename = "tPlus1UnlockDate")]
    pub t_plus1_unlock_date: String,
    pub created_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityPointDto {
    pub date: String,
    pub portfolio_nav: f64,
    #[serde(rename = "csi1000Nav")]
    pub csi1000_nav: f64,
    #[serde(rename = "cni2000Nav")]
    pub cni2000_nav: f64,
    pub excess_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorWeightDto {
    pub name_zh: String,
    pub weight_pct: f64,
    pub value_yuan: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioOverviewDto {
    pub initial_capital_yuan: f64,
    pub cash_yuan: f64,
    pub market_value_yuan: f64,
    pub total_equity_yuan: f64,
    pub cum_return_pct: f64,
    pub daily_pnl_yuan: f64,
    pub daily_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub open_slots_count: usize,
    pub max_slots: usize,
    pub total_fees_paid_yuan: f64,
    pub positions: Vec<PositionSlotDto>,
    pub recent_orders: Vec<SimulatedOrderDto>,
    pub equity_curve: Vec<EquityPointDto>,
    pub sector_weights: Vec<SectorWeightDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactorDiagnosticDto {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPluginConfigDto {
    pub strategy_id: String,
    pub version: String,
    pub max_total_mv_yi: f64,
    pub min_turnover_pct: f64,
    pub consecutive_days: u32,
    pub min_intraday_projected_turnover_pct: f64,
    pub max_positions: usize,
    pub initial_capital_yuan: i64,
    pub insider_sell_veto_pct: f64,
    pub holder_surge_veto_pct: f64,
    pub main_outflow_veto_ratio: f64,
    pub atr_stop_mult: f64,
    pub market_polarity: String,
    pub factors: Vec<FactorDiagnosticDto>,
    pub manifest_rel_path: String,
    pub raw_strategy_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomistChainDto {
    pub chain_title: String,
    pub economist_trigger: String,
    pub us_mapping: String,
    pub cn_mapping: String,
    pub divergence_insight: String,
    pub related_cn_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomistReportItemDto {
    pub report_id: String,
    pub date: String,
    pub title_zh: String,
    pub rel_path: String,
    pub article_count: usize,
    pub core_count: usize,
    pub acked: bool,
    pub acked_at: Option<String>,
    pub us_score: f64,
    pub cn_score: f64,
    pub executive_summary_zh: String,
    pub chains: Vec<EconomistChainDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomistModuleDto {
    pub unacked_count: usize,
    pub reports: Vec<EconomistReportItemDto>,
    pub backtest_hit_rate_pct: f64,
    pub us_hit_rate_pct: f64,
    pub cn_hit_rate_pct: f64,
    pub bearish_risk_sectors_zh: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionDto {
    pub agent_role: String,
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub model: String,
    pub reasoning_effort: String,
    pub spec_rel_path: String,
    pub turn_count: i64,
    pub status: String,
    pub last_woken_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageHygieneDto {
    pub portable_root_rel: String,
    pub build_footprint_bytes: u64,
    pub build_cap_bytes: u64,
    pub sqlite_db_bytes: u64,
    pub retention_days: u32,
    pub zero_absolute_path_verified: bool,
    pub max_file_lines_observed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEventDto {
    pub event_id: i64,
    pub ts_iso: String,
    pub severity: String,
    pub subsystem: String,
    pub event_name: String,
    pub duration_ms: Option<u64>,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryDashboardDto {
    pub sessions: Vec<AgentSessionDto>,
    pub storage: StorageHygieneDto,
    pub events: Vec<TelemetryEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationAlertEventDto {
    pub slot_timestamp_cst: String,
    pub qualified_count: usize,
    pub top_picks: Vec<ScreenerCandidateDto>,
}

pub use crate::interface::evolution_dto::{
    AddWatchlistInputDto, EvolutionAuditLogDto, FactorAttributionDto, RootCauseFindingDto,
    VersionComparisonDto, VersionControlDashboardDto, VersionSnapshotDto, WatchlistItemDto,
    WatchlistOverviewDto, WatchlistSummaryDto,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketScanResponseDto {
    pub funnel: UniverseFunnelSummaryDto,
    pub candidates: Vec<ScreenerCandidateDto>,
    pub alert: Option<RotationAlertEventDto>,
    pub watchlist: WatchlistOverviewDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalBootstrapDto {
    pub funnel: UniverseFunnelSummaryDto,
    pub candidates: Vec<ScreenerCandidateDto>,
    pub portfolio: PortfolioOverviewDto,
    pub featured_kline: StockKlinePayloadDto,
    pub plugin_config: StrategyPluginConfigDto,
    pub economist: EconomistModuleDto,
    pub telemetry: TelemetryDashboardDto,
    pub watchlist: WatchlistOverviewDto,
    pub version_control: VersionControlDashboardDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrderInputDto {
    pub symbol: String,
    pub lot_qty: Option<i64>,
    pub rationale_zh: Option<String>,
    pub stop_loss_price_yuan: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePluginInputDto {
    pub min_intraday_projected_turnover_pct: f64,
    pub insider_sell_veto_pct: f64,
    pub holder_surge_veto_pct: f64,
    pub main_outflow_veto_ratio: f64,
    pub atr_stop_mult: f64,
    pub market_polarity: String,
    pub factor_weights: BTreeMap<String, f64>,
}

pub fn round_2(val: f64) -> f64 {
    if !val.is_finite() {
        return 0.0;
    }
    (val * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_digit_bearing_camel_case_fields_identically_to_typescript_contract() {
        let funnel = UniverseFunnelSummaryDto {
            total_active_count: 5931,
            tradable_count: 5120,
            under_200_yi_count: 3890,
            consecutive_turnover_vol_count: 42,
            pitfall_cleared_count: 26,
            qualified_buy_count: 10,
            provider_name: "Tencent+EastMoney".to_string(),
            scan_rtt_ms: 1420,
            scanned_at_iso: "2026-11-18T06:30:00Z".to_string(),
            market_session: "TRADING".to_string(),
        };
        let json = serde_json::to_value(&funnel).unwrap();
        assert!(json.get("under200YiCount").is_some());
        assert_eq!(round_2(12.3456), 12.35);
    }
}
