use economist_quant_terminal_lib::application::rotation_scheduler::is_post_close_walk_forward_window_cst;
use economist_quant_terminal_lib::application::scan_pipeline::ScanExecutionOutput;
use economist_quant_terminal_lib::application::terminal_service::TerminalService;
use economist_quant_terminal_lib::domain::types::Symbol;
use economist_quant_terminal_lib::infrastructure::market_tencent::RawTencentQuote;
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::interface::dto::UniverseFunnelSummaryDto;
use economist_quant_terminal_lib::interface::evolution_dto::AddWatchlistInputDto;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[tokio::test]
async fn r5_01_watchlist_stale_quote_date_never_triggers_false_t1_exit_and_t0_ignores_preentry_extremes() {
    let sandbox = PathBuf::from(".temp").join("test_r5_watchlist_stale_quote");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    svc.add_to_watchlist(AddWatchlistInputDto {
        symbol: "300308.SZ".to_string(),
        name_zh: "中际旭创".to_string(),
        board: "CHINEXT".to_string(),
        industry_sw1: "通信".to_string(),
        entry_price_yuan: 100.0,
        stop_loss_price_yuan: 93.0,
        target_price_yuan: 115.0,
        conviction_score: 84.0,
        source_decision: "WATCH_OBSERVE".to_string(),
        rationale_zh: "周五尾盘纳入观望".to_string(),
        factor_z_scores: BTreeMap::new(),
    })
    .await
    .unwrap();

    // Pin added_trade_date to Friday 2026-11-20
    let mut items = svc.repo.list_watchlist_items().unwrap();
    items[0].added_trade_date = "2026-11-20".to_string();
    svc.repo.reset_or_insert_watchlist_item(&items[0]).unwrap();

    // Simulate Monday 08:30 CST pre-open: wall-clock today_cst is "2026-11-23",
    // while Tencent quote still carries Friday's raw_ts "20261120150003" where Friday morning low was 91.50 (<= stop 93.00)!
    let stale_friday_scan = ScanExecutionOutput {
        funnel: UniverseFunnelSummaryDto {
            total_active_count: 5500,
            tradable_count: 5100,
            under_200_yi_count: 4000,
            consecutive_turnover_vol_count: 20,
            pitfall_cleared_count: 10,
            qualified_buy_count: 5,
            provider_name: "r5-stale-preopen".to_string(),
            scan_rtt_ms: 10,
            scanned_at_iso: "2026-11-23T08:30:00+08:00".to_string(),
            market_session: "PRE_OPEN".to_string(),
        },
        candidates: Vec::new(),
        raw_quotes: vec![RawTencentQuote {
            symbol: Symbol::parse("300308.SZ").unwrap(),
            name_zh: "中际旭创".to_string(),
            latest_yuan: 100.5,
            prev_close_yuan: 98.0,
            open_yuan: 95.0,
            high_yuan: 102.0,
            low_yuan: 91.5, // Friday morning low BEFORE user added at 100.0!
            volume_lots: 120_000.0,
            amount_wan: 120_000.0,
            turnover_pct: 6.2,
            circ_mv_yi: 80.0,
            total_mv_yi: 95.0,
            vol_ratio: 1.5,
            vwap_yuan: 99.5,
            is_suspended: false,
            raw_ts: "20261120150003".to_string(),
        }],
        clist_meta: HashMap::new(),
        kline_cache_1d: HashMap::new(),
    };

    let dto = svc
        .sync_watchlist_with_scan(&stale_friday_scan, "2026-11-23")
        .unwrap();
    let tracked = &dto.items[0];
    assert_eq!(
        tracked.outcome, "TRACKING",
        "Stale Friday quote on Monday pre-open must NOT trigger false T+1 HIT_STOP!"
    );
    assert_eq!(
        tracked.holding_trading_days, 0,
        "Stale Friday quote must keep holding_trading_days == 0"
    );
    assert!(
        tracked.mae_pct >= -0.01,
        "T+0 pre-entry morning low (91.5) must NOT pollute post-entry MAE, got {}%",
        tracked.mae_pct
    );
}

#[tokio::test]
async fn r5_02_version_lineage_reconciles_external_manifest_edit_and_post_close_idempotency() {
    let sandbox = PathBuf::from(".temp").join("test_r5_lineage_and_postclose");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    let dash0 = svc.get_version_control_dashboard(None, None).await.unwrap();
    assert_eq!(dash0.active_version_id, "v1.2.0");

    // Simulate external hot-plugin edit on disk (Requirement #15): bump manifest version to 1.3.0
    svc.plugins.set_manifest_version("1.3.0").unwrap();

    let dash1 = svc.get_version_control_dashboard(None, None).await.unwrap();
    assert_eq!(
        dash1.active_version_id, "v1.3.0",
        "ensure_default_version_lineage must reconcile SQLite active version when on-disk manifest version advances to 1.3.0"
    );

    // Post-close walk-forward window & holiday / SQLite daily idempotency checks:
    let national_day = chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let thursday = chrono::NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
    assert!(
        !is_post_close_walk_forward_window_cst(national_day, 15, 10),
        "Statutory holiday must NOT open post-close walk-forward window"
    );
    assert!(
        !is_post_close_walk_forward_window_cst(thursday, 15, 2),
        "15:02 CST (before 15:05 closing auction settlement) must NOT trigger walk-forward"
    );
    assert!(
        is_post_close_walk_forward_window_cst(thursday, 15, 6),
        "15:06 CST on trading day MUST open post-close walk-forward window"
    );
    assert!(
        is_post_close_walk_forward_window_cst(thursday, 19, 30),
        "Evening launch (19:30 CST) on trading day MUST catch up post-close walk-forward if not yet run"
    );

    let (_, _, now_iso) = economist_quant_terminal_lib::application::portfolio_seeder::current_cst_dates();
    let today_prefix = &now_iso[..10];
    assert!(
        !svc.repo.has_post_close_evolution_for_date(today_prefix).unwrap(),
        "Before running post-close iteration, daily idempotency flag must be false"
    );
    let _ = svc.run_anti_overfit_iteration("POST_CLOSE_1505").await.unwrap();
    assert!(
        svc.repo.has_post_close_evolution_for_date(today_prefix).unwrap(),
        "After POST_CLOSE_1505 run, SQLite daily idempotency check must return true across restarts"
    );
}
