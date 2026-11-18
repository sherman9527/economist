use economist_quant_terminal_lib::application::dto_mapper::map_portfolio_overview_dto;
use economist_quant_terminal_lib::application::rotation_scheduler::cst_30m_rotation_slot_bucket;
use economist_quant_terminal_lib::application::scan_pipeline::ScanExecutionOutput;
use economist_quant_terminal_lib::application::terminal_service::{
    CachedScanState, TerminalService,
};
use economist_quant_terminal_lib::domain::portfolio_ledger::{
    suggest_position_shares, BuyOrderCommand, PortfolioLedger, SellOrderCommand,
};
use economist_quant_terminal_lib::domain::types::{BoardKind, FeeSchedule, Fen, Symbol};
use economist_quant_terminal_lib::infrastructure::market_tencent::RawTencentQuote;
use economist_quant_terminal_lib::infrastructure::plugin_store::{
    FactorPluginManifest, FactorSpecEntry, PluginStore,
};
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::interface::dto::{PlaceOrderInputDto, UniverseFunnelSummaryDto};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn preserves_ratcheted_chandelier_stop_above_avg_cost_and_star_odd_lots() {
    let mut ledger = PortfolioLedger::new(
        Fen::INITIAL_CAPITAL_100W,
        "2026-11-18",
        FeeSchedule::default(),
    );
    let star_sym = Symbol::parse("688111.SH").unwrap();
    let suggested = suggest_position_shares(
        BoardKind::Star,
        Fen::INITIAL_CAPITAL_100W,
        Fen::INITIAL_CAPITAL_100W,
        Fen::from_yuan(45.0),
        Fen::from_yuan(41.5),
        12.0,
        0.85,
    );
    assert!(suggested >= 200);

    ledger
        .execute_buy(BuyOrderCommand {
            order_id: "ORD-STAR-235".to_string(),
            rec_id: None,
            symbol: star_sym.clone(),
            name_zh: "金山办公".to_string(),
            industry_sw1: "计算机".to_string(),
            quote_price_fen: Fen::from_yuan(45.0),
            prev_close_fen: Fen::from_yuan(44.0),
            is_st: false,
            shares: 235,
            stop_loss_price_fen: Fen::from_yuan(41.5),
            rationale_zh: "科创板零股235股建仓".to_string(),
            trade_date: "2026-11-18".to_string(),
            next_trade_date: "2026-11-19".to_string(),
            created_at_iso: "2026-11-18T10:00:00+08:00".to_string(),
        })
        .expect("STAR odd-lot buy >= 200 shares must succeed");

    ledger.advance_trading_day("2026-11-19");
    ledger.update_mark_price("688111.SH", Fen::from_yuan(56.0));
    if let Some(pos) = ledger.positions.get_mut("688111.SH") {
        pos.stop_loss_price_fen = Fen::from_yuan(49.80);
    }

    let dto = map_portfolio_overview_dto(&ledger, &[]);
    assert_eq!(dto.positions.len(), 1);
    assert!(
        (dto.positions[0].stop_loss_price_yuan - 49.80).abs() < 0.02,
        "Ratcheted Chandelier stop (49.80 > avgCost 45.0) must not be clamped down, got {}",
        dto.positions[0].stop_loss_price_yuan
    );

    ledger
        .execute_sell(SellOrderCommand {
            order_id: "ORD-STAR-EXIT-235".to_string(),
            symbol: star_sym,
            quote_price_fen: Fen::from_yuan(56.0),
            prev_close_fen: Fen::from_yuan(54.0),
            is_st: false,
            shares: 235,
            trigger_type: "REBALANCE_EXIT".to_string(),
            rationale_zh: "科创板零股全额止盈".to_string(),
            trade_date: "2026-11-19".to_string(),
            created_at_iso: "2026-11-19T14:00:00+08:00".to_string(),
        })
        .expect("STAR odd-lot full sell after T+1 unlock must succeed");
    assert_eq!(ledger.open_positions_count(), 0);
}

#[test]
fn rejects_invalid_factor_manifest_enums_and_buckets_30m_grace_windows() {
    assert_eq!(cst_30m_rotation_slot_bucket(0, 9, 32), Some("09:30"));
    assert_eq!(cst_30m_rotation_slot_bucket(3, 14, 59), None);
    assert_eq!(cst_30m_rotation_slot_bucket(3, 15, 2), Some("15:00"));

    let bad_manifest = FactorPluginManifest {
        plugin_set_id: "test_bad".to_string(),
        version: "1.0.0".to_string(),
        neutralization: "SW1".to_string(),
        factors: vec![FactorSpecEntry {
            factor_id: "F_BAD".to_string(),
            name_zh: "非法频率因子".to_string(),
            expr: "turnover_2d_mean".to_string(),
            bar_freq: "5m".to_string(),
            weight_pct: 100.0,
            rank_ic_mean: 0.05,
            ic_ir: 0.8,
            oos_win_rate_pct: 55.0,
            evidence_tag: "[OOS]".to_string(),
            status: "ACTIVE".to_string(),
            quantile_returns_pct: vec![0.0; 10],
            ic_decay: vec![0.05; 10],
        }],
    };
    assert!(PluginStore::validate_factor_manifest(&bad_manifest).is_err());
}

#[tokio::test]
async fn close_position_emits_canonical_rebalance_trigger_and_trading_subsystem() {
    let sandbox = PathBuf::from(".temp").join("test_r1_close_trigger_contract");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox.clone()).unwrap();
    let svc = TerminalService::initialize(paths).unwrap();

    {
        let mut w = svc.scan_cache.write().await;
        *w = Some(CachedScanState {
            output: ScanExecutionOutput {
                funnel: UniverseFunnelSummaryDto {
                    total_active_count: 5576,
                    tradable_count: 5200,
                    under_200_yi_count: 4100,
                    consecutive_turnover_vol_count: 35,
                    pitfall_cleared_count: 20,
                    qualified_buy_count: 8,
                    provider_name: "unit-test".to_string(),
                    scan_rtt_ms: 10,
                    scanned_at_iso: "2026-11-18T10:00:00+08:00".to_string(),
                    market_session: "TRADING".to_string(),
                },
                candidates: Vec::new(),
                raw_quotes: vec![RawTencentQuote {
                    symbol: Symbol::parse("300308.SZ").unwrap(),
                    name_zh: "中际旭创".to_string(),
                    latest_yuan: 32.00,
                    prev_close_yuan: 31.20,
                    open_yuan: 31.50,
                    high_yuan: 32.40,
                    low_yuan: 31.10,
                    volume_lots: 120_000.0,
                    amount_wan: 38_400.0,
                    turnover_pct: 7.2,
                    circ_mv_yi: 88.0,
                    total_mv_yi: 95.0,
                    vol_ratio: 1.5,
                    vwap_yuan: 31.95,
                    is_suspended: false,
                    raw_ts: "20261118100000".to_string(),
                }],
                clist_meta: HashMap::new(),
                kline_cache_1d: HashMap::new(),
            },
            captured_at: Instant::now(),
        });
    }

    // Seed an already-unlocked T+1 position on an earlier trading day
    let mut prior_ledger = PortfolioLedger::new(
        Fen::INITIAL_CAPITAL_100W,
        "2020-01-02",
        FeeSchedule::default(),
    );
    prior_ledger
        .execute_buy(BuyOrderCommand {
            order_id: "ORD-PRIOR-1".to_string(),
            rec_id: None,
            symbol: Symbol::parse("300308.SZ").unwrap(),
            name_zh: "中际旭创".to_string(),
            industry_sw1: "通信".to_string(),
            quote_price_fen: Fen::from_yuan(30.00),
            prev_close_fen: Fen::from_yuan(29.50),
            is_st: false,
            shares: 500,
            stop_loss_price_fen: Fen::from_yuan(27.80),
            rationale_zh: "历史建仓已跨日解锁".to_string(),
            trade_date: "2020-01-02".to_string(),
            next_trade_date: "2020-01-03".to_string(),
            created_at_iso: "2020-01-02T10:00:00+08:00".to_string(),
        })
        .unwrap();
    svc.repo.save_ledger(&prior_ledger).unwrap();

    // Concurrent buy of a second stock before market scan sync must survive sync!
    let _ = svc
        .place_simulated_buy(PlaceOrderInputDto {
            symbol: "300308.SZ".to_string(),
            lot_qty: Some(200),
            rationale_zh: Some("盘中加仓测试".to_string()),
            stop_loss_price_yuan: Some(29.50),
        })
        .await
        .unwrap();

    // Closing the unlocked 500 shares must succeed, leaving 200 T+0 locked shares intact,
    // and emitting canonical REBALANCE_EXIT + TRADING subsystem!
    let after_close = svc
        .close_simulated_position("300308.SZ", "轮动减仓释放已解锁筹码")
        .await
        .unwrap();
    assert_eq!(after_close.positions.len(), 1);
    assert_eq!(after_close.positions[0].locked_t0_qty, 200);
    assert_eq!(after_close.positions[0].sellable_qty, 0);
    let sell_ord = after_close
        .recent_orders
        .iter()
        .find(|o| o.side == "SELL")
        .expect("SELL order recorded");
    assert_eq!(sell_ord.trigger_type, "REBALANCE_EXIT");

    let dash = svc.run_storage_hygiene().await.unwrap();
    for ev in &dash.events {
        assert!(matches!(
            ev.subsystem.as_str(),
            "MARKET" | "QUANT" | "TRADING" | "AGENT" | "NEWS" | "UI" | "CLEANER"
        ));
    }

    drop(svc);
    let _ = std::fs::remove_dir_all(&sandbox);
}
