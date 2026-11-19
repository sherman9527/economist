use crate::application::terminal_service::TerminalService;
use crate::interface::dto::{
    AddWatchlistInputDto, AgentSessionDto, EconomistModuleDto, MarketScanResponseDto,
    PlaceOrderInputDto, PortfolioOverviewDto, StockKlinePayloadDto, StrategyPluginConfigDto,
    TelemetryDashboardDto, TerminalBootstrapDto, UpdatePluginInputDto, VersionControlDashboardDto,
    WatchlistOverviewDto,
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn bootstrap_terminal(
    svc: State<'_, Arc<TerminalService>>,
) -> Result<TerminalBootstrapDto, String> {
    svc.bootstrap_terminal().await
}

#[tauri::command]
pub async fn run_market_scan(
    force_refresh: bool,
    app: AppHandle,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<MarketScanResponseDto, String> {
    let resp = svc.run_market_scan(force_refresh).await?;
    if let Some(ref alert) = resp.alert {
        let _ = app.emit("quant://rotation-alert", alert.clone());
    }
    Ok(resp)
}

#[tauri::command]
pub async fn get_stock_kline(
    symbol: String,
    period: String,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<StockKlinePayloadDto, String> {
    svc.get_stock_kline(&symbol, &period).await
}

#[tauri::command]
pub async fn place_simulated_buy(
    input: PlaceOrderInputDto,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<PortfolioOverviewDto, String> {
    svc.place_simulated_buy(input).await
}

#[tauri::command]
pub async fn close_simulated_position(
    symbol: String,
    reason_zh: String,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<PortfolioOverviewDto, String> {
    svc.close_simulated_position(&symbol, &reason_zh).await
}

#[tauri::command]
pub async fn update_plugin_config(
    input: UpdatePluginInputDto,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<StrategyPluginConfigDto, String> {
    svc.update_plugin_config(input).await
}

#[tauri::command]
pub async fn ack_economist_report(
    report_id: String,
    acked: bool,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<EconomistModuleDto, String> {
    svc.ack_economist_report(&report_id, acked).await
}

#[tauri::command]
pub async fn get_economist_report_html(
    rel_path: String,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<String, String> {
    svc.get_economist_report_html(&rel_path).await
}

#[tauri::command]
pub async fn run_storage_hygiene(
    svc: State<'_, Arc<TerminalService>>,
) -> Result<TelemetryDashboardDto, String> {
    svc.run_storage_hygiene().await
}

#[tauri::command]
pub async fn wake_agent_session(
    agent_role: String,
    prompt: String,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<AgentSessionDto, String> {
    svc.wake_agent_session(&agent_role, &prompt).await
}

#[tauri::command]
pub async fn get_watchlist_overview(
    svc: State<'_, Arc<TerminalService>>,
) -> Result<WatchlistOverviewDto, String> {
    svc.get_watchlist_overview().await
}

#[tauri::command]
pub async fn add_to_watchlist(
    input: AddWatchlistInputDto,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<WatchlistOverviewDto, String> {
    svc.add_to_watchlist(input).await
}

#[tauri::command]
pub async fn remove_from_watchlist(
    symbol: String,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<WatchlistOverviewDto, String> {
    svc.remove_from_watchlist(&symbol).await
}

#[tauri::command]
pub async fn get_version_control_dashboard(
    base_version_id: Option<String>,
    target_version_id: Option<String>,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<VersionControlDashboardDto, String> {
    svc.get_version_control_dashboard(base_version_id, target_version_id)
        .await
}

#[tauri::command]
pub async fn switch_strategy_version(
    version_id: String,
    svc: State<'_, Arc<TerminalService>>,
) -> Result<VersionControlDashboardDto, String> {
    svc.switch_strategy_version(&version_id).await
}

#[tauri::command]
pub async fn run_anti_overfit_iteration(
    svc: State<'_, Arc<TerminalService>>,
) -> Result<VersionControlDashboardDto, String> {
    svc.run_anti_overfit_iteration("MANUAL_UI").await
}
