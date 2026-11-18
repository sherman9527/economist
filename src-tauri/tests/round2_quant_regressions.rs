use economist_quant_terminal_lib::domain::factor_engine::{
    compute_chandelier_stop_fen, neutralize_and_score_cross_section, parse_factor_expr,
    winsorize_mad, CrossSectionSample,
};
use economist_quant_terminal_lib::domain::pitfall_veto::{
    audit_five_pitfalls, PitfallAuditInput, PitfallThresholds,
};
use economist_quant_terminal_lib::domain::portfolio_ledger::{
    suggest_position_shares, BuyOrderCommand, PortfolioLedger, SellOrderCommand,
};
use economist_quant_terminal_lib::domain::strategy_gate::{
    evaluate_strategy_gate, MarketSessionState, StrategyCandidateInput, StrategyGateParams,
};
use economist_quant_terminal_lib::domain::types::{
    compute_price_limits, BoardKind, FeeSchedule, Fen, Symbol,
};
use economist_quant_terminal_lib::infrastructure::market_eastmoney::parse_holder_notice_json;
use std::collections::BTreeMap;

#[test]
fn r2_chandelier_stop_never_ratchets_downward_when_price_crashes() {
    // Entry ¥20.00 (2000 fen), ATR ¥0.50 (50 fen), ratcheted stop at ¥20.40 (2040 fen) after ¥21.50 peak.
    // Price now crashes to ¥19.35 (1935 fen). Monotonic trailing stop MUST remain ¥20.40 (2040 fen).
    let next_stop = compute_chandelier_stop_fen(
        Fen(2000),
        Fen(2040),
        Fen(2150),
        Fen(1935),
        Fen(50),
        6.0,
        2.2,
    );
    assert_eq!(
        next_stop,
        Fen(2040),
        "Chandelier trailing stop must never ratchet downward during a crash"
    );
}

#[test]
fn r2_friday_buy_remains_t0_locked_over_weekend_and_national_day_holidays() {
    let mut ledger = PortfolioLedger::new(
        Fen::INITIAL_CAPITAL_100W,
        "2026-11-20", // Friday
        FeeSchedule::default(),
    );
    let sym = Symbol::parse("002463.SZ").unwrap();
    ledger
        .execute_buy(BuyOrderCommand {
            order_id: "ORD-FRI-1".to_string(),
            rec_id: None,
            symbol: sym.clone(),
            name_zh: "沪电股份".to_string(),
            industry_sw1: "元件".to_string(),
            quote_price_fen: Fen::from_yuan(20.0),
            prev_close_fen: Fen::from_yuan(19.5),
            is_st: false,
            shares: 1000,
            stop_loss_price_fen: Fen::from_yuan(18.8),
            rationale_zh: "周五尾盘建仓测试周末T+1锁仓".to_string(),
            trade_date: "2026-11-20".to_string(),
            next_trade_date: "2026-11-23".to_string(),
            created_at_iso: "2026-11-20T14:55:00+08:00".to_string(),
        })
        .unwrap();

    // Advance to Saturday 2026-11-21: MUST NOT unlock T+1!
    ledger.advance_trading_day("2026-11-21");
    let pos_sat = ledger.positions.get("002463.SZ").unwrap();
    assert_eq!(
        pos_sat.sellable_qty, 0,
        "Friday T+0 shares must stay locked on Saturday 2026-11-21"
    );
    assert_eq!(pos_sat.locked_t0_qty, 1000);

    // Advance to Monday 2026-11-23: now unlocks!
    ledger.advance_trading_day("2026-11-23");
    let pos_mon = ledger.positions.get("002463.SZ").unwrap();
    assert_eq!(pos_mon.sellable_qty, 1000);
    assert_eq!(pos_mon.locked_t0_qty, 0);
}

#[test]
fn r2_singleton_industry_neutralization_is_zero_centered_and_mad_ignores_nans() {
    let mut weights = BTreeMap::new();
    weights.insert("F2_concave_abn_turnover".to_string(), 1.0);

    let mk = |sym: &str, ind: &str, mv: f64, f2: f64| {
        let mut raw = BTreeMap::new();
        raw.insert("F2_concave_abn_turnover".to_string(), f2);
        CrossSectionSample {
            symbol: sym.to_string(),
            industry_sw1: ind.to_string(),
            circ_mv_yi: mv,
            raw_factors: raw,
        }
    };
    let samples = vec![
        mk("688001.SH", "半导体", 60.0, 1.35),
        mk("688002.SH", "半导体", 65.0, 1.25),
        mk("300001.SZ", "专用设备", 62.0, 0.75), // Singleton industry with much weaker raw F2!
    ];
    let scored = neutralize_and_score_cross_section(&samples, &weights);
    let score_a = scored[0].conviction_score;
    let score_c = scored[2].conviction_score;
    assert!(
        score_a > score_c,
        "Stronger paired-industry stock ({}) must score higher than weaker singleton-industry stock ({})",
        score_a,
        score_c
    );

    let clipped = winsorize_mad(&[f64::NAN, f64::NAN, f64::NAN, 1.0, 2.0, 3.0, 100.0], 3.0);
    assert!(
        clipped[6] < 20.0,
        "MAD winsorizer must clip extreme outlier 100.0 even when NaNs are present, got {}",
        clipped[6]
    );
}

#[test]
fn r2_suggest_shares_accounts_for_friction_and_existing_holdings_and_bse_odd_lot_sell() {
    // 100W equity, quote ¥20.00, stop ¥19.20, 10.0% weight cap = ¥100,000 max post-friction cost.
    // Raw 5,000 shares * ¥20.03 fill + commission = ¥100,175.04 > ¥100,000 -> must suggest <= 4,900 shares!
    let suggested = suggest_position_shares(
        BoardKind::Main,
        Fen::INITIAL_CAPITAL_100W,
        Fen::INITIAL_CAPITAL_100W,
        Fen::from_yuan(20.0),
        Fen::from_yuan(19.2),
        10.0,
        0.8,
    );
    assert!(
        suggested <= 4900 && suggested >= 4800,
        "Suggested shares must stay <= 10.0% after 15bps slippage + commission, got {}",
        suggested
    );

    // BSE partial sell of 150 shares out of 300 sellable shares must succeed (BSE >=100, step 1)
    let mut ledger = PortfolioLedger::new(
        Fen::INITIAL_CAPITAL_100W,
        "2026-11-18",
        FeeSchedule::default(),
    );
    let bse_sym = Symbol::parse("920001.BJ").unwrap();
    ledger
        .execute_buy(BuyOrderCommand {
            order_id: "ORD-BSE-B".to_string(),
            rec_id: None,
            symbol: bse_sym.clone(),
            name_zh: "北证测试".to_string(),
            industry_sw1: "通用设备".to_string(),
            quote_price_fen: Fen::from_yuan(15.0),
            prev_close_fen: Fen::from_yuan(14.8),
            is_st: false,
            shares: 300,
            stop_loss_price_fen: Fen::from_yuan(14.0),
            rationale_zh: "北交所建仓".to_string(),
            trade_date: "2026-11-18".to_string(),
            next_trade_date: "2026-11-19".to_string(),
            created_at_iso: "2026-11-18T10:00:00+08:00".to_string(),
        })
        .unwrap();
    ledger.advance_trading_day("2026-11-19");
    let res = ledger.execute_sell(SellOrderCommand {
        order_id: "ORD-BSE-S".to_string(),
        symbol: bse_sym,
        quote_price_fen: Fen::from_yuan(15.5),
        prev_close_fen: Fen::from_yuan(15.0),
        is_st: false,
        shares: 150,
        trigger_type: "TAKE_PROFIT".to_string(),
        rationale_zh: "北交所部分止盈150股".to_string(),
        trade_date: "2026-11-19".to_string(),
        created_at_iso: "2026-11-19T10:30:00+08:00".to_string(),
    });
    assert!(
        res.is_ok(),
        "BSE partial sell of 150 shares (>=100, step 1) must succeed: {:?}",
        res
    );
}

#[test]
fn r2_exact_half_up_st_price_limits_0930_intraday_gate_economist_sector_and_sci_expr() {
    // 1. IEEE-754 tick rounding: prev_close = ¥1.10 (110 fen), ST 5% limit -> exact 104.5 fen -> half-up 105 fen (¥1.05)
    let (down, up) = compute_price_limits(Fen(110), BoardKind::Main, true);
    assert_eq!(down, Fen(105), "110 * 0.95 = 104.5 must round half-up to 105 fen");
    assert_eq!(up, Fen(116), "110 * 1.05 = 115.5 must round half-up to 116 fen");

    // 2. 09:30 opening slot (elapsed_trading_minutes = 0 during Trading) must engage U-curve intraday projection
    let params = StrategyGateParams::default();
    let verdict = evaluate_strategy_gate(
        &StrategyCandidateInput {
            name_zh: "晨盘突破".to_string(),
            board: BoardKind::Main,
            latest_price_fen: Fen::from_yuan(12.5),
            prev_close_fen: Fen::from_yuan(12.0),
            is_suspended: false,
            listing_days: 200,
            total_mv_yi: 85.0,
            turnover_t0_pct: 2.8, // At 09:30 opening slot, 2.8% realized projects well above 5.5%
            turnover_t1_pct: 7.2,
            vol_t0: 35_000.0,
            vol_t1: 90_000.0,
            vol_ma20_t1: 60_000.0,
            session: MarketSessionState::Trading,
            elapsed_trading_minutes: 0,
        },
        &params,
    );
    assert!(
        verdict.consecutive_turnover_vol_passed,
        "09:30 Trading slot with 2.8% realized turnover must use U-curve projection, got {:?}",
        verdict
    );

    // 3. Canonical Economist bearish prose must match EastMoney SW1 industry "房地产开发"
    let pit = audit_five_pitfalls(
        &PitfallAuditInput {
            net_insider_reduction_90d_pct: 0.0,
            announcement_titles_60d: vec![],
            industry_sw1: "房地产开发".to_string(),
            economist_bearish_sectors: vec![
                "A股/港股高负债民企地产开发商及纯内需传统建筑建材".to_string(),
            ],
            holder_num_change_qoq_pct: Some(-3.5),
            holder_notice_date: Some("2026-10-30".to_string()),
            main_net_inflow_ratio: 0.08,
            open_yuan: 10.0,
            high_yuan: 10.5,
            low_yuan: 9.95,
            close_yuan: 10.45,
            ma20_yuan: 9.8,
            pct_chg: 4.5,
        },
        &PitfallThresholds::default(),
    );
    assert!(
        pit.vetoed,
        "Real-estate SW1 '房地产开发' must be vetoed by Economist bearish prose '地产开发商'"
    );

    // 4. parse_holder_notice_json skips null HOLDER_NUM_RATIO in row 0 to read valid row 1
    let holder_json = r#"{"result":{"data":[{"HOLD_NOTICE_DATE":"2026-11-10 00:00:00","HOLDER_NUM_RATIO":null},{"HOLD_NOTICE_DATE":"2026-10-28 00:00:00","HOLDER_NUM_RATIO":48.5}]}}"#;
    let (ratio, dt) = parse_holder_notice_json(holder_json);
    assert_eq!(ratio, Some(48.5));
    assert_eq!(dt.as_deref(), Some("2026-10-28"));

    // 5. Factor parser supports scientific notation literals like 1e-4
    let ast = parse_factor_expr("(close_price - vwap_price) / max(vwap_price, 1e-4)");
    assert!(ast.is_ok(), "Scientific notation 1e-4 must parse cleanly: {:?}", ast);
}

#[test]
fn r2_tencent_star_fallback_shares_to_lots_and_pit_holder_notice_cutoff() {
    use economist_quant_terminal_lib::infrastructure::market_eastmoney::parse_holder_notice_json_as_of;
    use economist_quant_terminal_lib::infrastructure::market_tencent::parse_tencent_fallback_kline_json;

    // STAR 688 stock with ¥50亿 circ_mv, close ¥50.00 -> 100M float shares.
    // Raw row[5] = 6,000,000 shares (60,000 lots = 6.0% turnover, NOT 600% turnover!)
    let star_sym = Symbol::parse("688041.SH").unwrap();
    let payload = r#"{"code":0,"data":{"sh688041":{"qfqday":[["2026-11-18","48.50","50.00","51.20","48.00","6000000"]]}}}"#;
    let bars = parse_tencent_fallback_kline_json(payload, &star_sym, "1d", 50.0).unwrap();
    assert_eq!(bars.len(), 1);
    assert!(
        (bars[0].volume - 60_000.0).abs() < 1e-6,
        "STAR 688 raw share count 6,000,000 must normalize to 60,000 lots, got {}",
        bars[0].volume
    );
    assert!(
        (bars[0].turnover_pct - 6.0).abs() < 0.05,
        "STAR 688 turnover must be ~6.0% instead of 100x inflated 600%, got {}",
        bars[0].turnover_pct
    );

    // PIT cutoff: notice published 2026-11-15 must be ignored when as_of_date = "2026-11-10"
    let holder_pit = r#"{"result":{"data":[
        {"HOLD_NOTICE_DATE":"2026-11-15 00:00:00","HOLDER_NUM_RATIO":42.0},
        {"HOLD_NOTICE_DATE":"2026-10-25 00:00:00","HOLDER_NUM_RATIO":-6.5}
    ]}}"#;
    let (r_as_of, dt_as_of) = parse_holder_notice_json_as_of(holder_pit, Some("2026-11-10"));
    assert_eq!(r_as_of, Some(-6.5));
    assert_eq!(dt_as_of.as_deref(), Some("2026-10-25"));
}

#[test]
fn r2_daily_bar_alignment_and_unmasked_breached_stop_dto() {
    use economist_quant_terminal_lib::application::dto_mapper::map_portfolio_overview_dto;
    use economist_quant_terminal_lib::application::scan_alignment::align_daily_bars_to_quote;
    use economist_quant_terminal_lib::infrastructure::market_tencent::{RawTencentQuote, SimpleBar};

    // Quote on Saturday 2026-11-21 (raw_ts = 20261121100500) must map back to Friday 2026-11-20
    // so bar_t1 is Thursday 2026-11-19 rather than double-counting Friday!
    let quote = RawTencentQuote {
        symbol: Symbol::parse("300308.SZ").unwrap(),
        name_zh: "测试小盘".to_string(),
        latest_yuan: 25.0,
        prev_close_yuan: 24.0,
        open_yuan: 24.2,
        high_yuan: 25.4,
        low_yuan: 24.0,
        volume_lots: 120_000.0,
        amount_wan: 30_000.0,
        turnover_pct: 7.5,
        circ_mv_yi: 40.0,
        total_mv_yi: 45.0,
        vol_ratio: 1.8,
        vwap_yuan: 25.0,
        is_suspended: false,
        raw_ts: "20261121100500".to_string(),
    };
    let bars = vec![
        SimpleBar {
            time: "2026-11-18".to_string(),
            open: 22.0,
            high: 23.0,
            low: 21.8,
            close: 22.8,
            volume: 80_000.0,
            amount_yuan: 182_400_000.0,
            turnover_pct: 5.2,
        },
        SimpleBar {
            time: "2026-11-19".to_string(),
            open: 22.8,
            high: 24.2,
            low: 22.6,
            close: 24.0,
            volume: 100_000.0,
            amount_yuan: 240_000_000.0,
            turnover_pct: 6.4,
        },
        SimpleBar {
            time: "2026-11-20".to_string(),
            open: 24.2,
            high: 25.4,
            low: 24.0,
            close: 25.0,
            volume: 120_000.0,
            amount_yuan: 300_000_000.0,
            turnover_pct: 7.5,
        },
    ];
    let aligned = align_daily_bars_to_quote(&quote, &bars).unwrap();
    assert_eq!(aligned.quote_date, "2026-11-20");
    assert_eq!(aligned.bar_t1.time, "2026-11-19");
    assert!((aligned.bar_t1.turnover_pct - 6.4).abs() < 1e-6);

    // DTO mapper must NOT mask a breached stop when T+0 locked position drops below stop!
    let mut ledger = PortfolioLedger::new(
        Fen::INITIAL_CAPITAL_100W,
        "2026-11-20",
        FeeSchedule::default(),
    );
    ledger
        .execute_buy(BuyOrderCommand {
            order_id: "ORD-BREACH-1".to_string(),
            rec_id: None,
            symbol: Symbol::parse("002463.SZ").unwrap(),
            name_zh: "沪电测试".to_string(),
            industry_sw1: "元件".to_string(),
            quote_price_fen: Fen::from_yuan(20.0),
            prev_close_fen: Fen::from_yuan(19.6),
            is_st: false,
            shares: 1000,
            stop_loss_price_fen: Fen::from_yuan(19.0),
            rationale_zh: "测试T+0跌破止损后DTO真实展示负止损距离".to_string(),
            trade_date: "2026-11-20".to_string(),
            next_trade_date: "2026-11-23".to_string(),
            created_at_iso: "2026-11-20T10:00:00+08:00".to_string(),
        })
        .unwrap();
    ledger.update_mark_price("002463.SZ", Fen::from_yuan(18.5));
    let dto = map_portfolio_overview_dto(&ledger, &[]);
    assert_eq!(dto.positions[0].stop_loss_price_yuan, 19.0);
    assert!(
        dto.positions[0].stop_distance_pct < 0.0,
        "Breached stop must expose negative stop_distance_pct instead of clamping stop below mark, got {}",
        dto.positions[0].stop_distance_pct
    );
}
