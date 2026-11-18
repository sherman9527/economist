use economist_quant_terminal_lib::domain::portfolio_ledger::{BuyOrderCommand, PortfolioLedger};
use economist_quant_terminal_lib::domain::types::{FeeSchedule, Fen, Symbol};
use economist_quant_terminal_lib::infrastructure::agent_supervisor::AgentSupervisor;
use economist_quant_terminal_lib::infrastructure::economist_bridge::EconomistBridge;
use economist_quant_terminal_lib::infrastructure::plugin_store::PluginStore;
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::infrastructure::sqlite_repo::SqliteRepository;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), dst_path).unwrap();
        }
    }
}

#[test]
fn r4_alert1_economist_bridge_rejects_malformed_or_xss_report_ids() {
    let sandbox = PathBuf::from("..")
        .join(".temp")
        .join("r4_sec_economist_ack");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox.clone()).unwrap();
    std::fs::create_dir_all(paths.data_dir()).unwrap();
    std::fs::create_dir_all(paths.reports_dir().join("2026/11/18")).unwrap();
    std::fs::write(
        paths
            .reports_dir()
            .join("2026/11/18/economist_report_2026-11-18.html"),
        "<!doctype html><html><head></head><body>ok</body></html>",
    )
    .unwrap();
    std::fs::write(
        paths.data_dir().join("ack_state.json"),
        r#"{"reports":{"2026-11-18":{"report_id":"2026-11-18","date":"2026-11-18","rel_path":"2026/11/18/economist_report_2026-11-18.html","acked":false}}}"#,
    )
    .unwrap();

    let repo = SqliteRepository::new(&paths.sqlite_db_path()).unwrap();
    let bridge = EconomistBridge::new(paths);

    for bad_id in [
        "</script><script>alert(1)</script>",
        "../2026-11-18",
        "2026-13-45",
        "2026-11-99",
        "9999-01-01",
    ] {
        let res = bridge.set_report_ack(&repo, bad_id, true);
        assert!(
            res.is_err(),
            "EconomistBridge::set_report_ack MUST reject untrusted/unknown report_id: {}",
            bad_id
        );
    }

    let ok_snap = bridge.set_report_ack(&repo, "2026-11-18", true).unwrap();
    assert_eq!(ok_snap.unacked_count, 0);
}

#[test]
fn r4_alert4_tauri_csp_and_embedded_report_offline_csp_are_enforced() {
    let conf_raw = std::fs::read_to_string("tauri.conf.json").unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf_raw).unwrap();
    let csp = conf["app"]["security"]["csp"]
        .as_str()
        .expect("tauri.conf.json app.security.csp MUST NOT be null");
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("script-src 'self'"));
    assert!(csp.contains("object-src 'none'"));

    let sandbox = PathBuf::from("..")
        .join(".temp")
        .join("r4_sec_embed_csp");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    std::fs::create_dir_all(paths.reports_dir().join("2026/11/18")).unwrap();
    std::fs::write(
        paths
            .reports_dir()
            .join("2026/11/18/economist_report_2026-11-18.html"),
        "<!doctype html><html><head><title>T</title></head><body>Report</body></html>",
    )
    .unwrap();
    let bridge = EconomistBridge::new(paths);
    let html = bridge
        .read_report_html("./reports/2026/11/18/economist_report_2026-11-18.html")
        .unwrap();
    assert!(
        html.contains("http-equiv=\"Content-Security-Policy\"")
            && html.contains("connect-src 'none'"),
        "Embedded report HTML MUST inject an offline CSP blocking external network egress"
    );
}

#[tokio::test]
async fn r4_alert5_agent_supervisor_locks_tools_stdin_prompt_bounds_and_binary() {
    let args = AgentSupervisor::build_wake_args(
        "88ace910-c427-41db-a87f-46d8ae39735b",
        "SECRET_UNTRUSTED_PROMPT_TEXT",
    );
    assert!(
        args.windows(2).any(|w| w == ["--available-tools", ""]),
        "AgentSupervisor::build_wake_args MUST lock --available-tools to empty string"
    );
    assert!(
        !args.iter().any(|a| a.contains("SECRET_UNTRUSTED_PROMPT_TEXT")),
        "Untrusted prompt MUST be piped via stdin rather than exposed on process -p argv"
    );

    let sandbox = PathBuf::from("..")
        .join(".temp")
        .join("r4_sec_agent_wake");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let repo = SqliteRepository::new(&paths.sqlite_db_path()).unwrap();

    let sup = AgentSupervisor::new(paths.clone(), "copilot".to_string());
    let oversized = "A".repeat(4001);
    assert!(
        sup.wake_session(&repo, "quant_researcher", &oversized)
            .await
            .is_err(),
        "Oversized prompts (> 4000 chars) MUST be rejected"
    );
    assert!(
        sup.wake_session(&repo, "quant_researcher", "bad\x07bell")
            .await
            .is_err(),
        "ASCII control chars in prompt MUST be rejected"
    );

    let hijacked = AgentSupervisor::new(paths, "./evil_copilot.cmd".to_string());
    assert!(
        hijacked
            .wake_session(&repo, "quant_researcher", "正常复盘指令")
            .await
            .is_err(),
        "Relative/script cli_binary names MUST be rejected to prevent CWD/batch hijacking"
    );
}

#[test]
fn r4_alert6_buy_overflow_and_plugin_weight_corruption_are_prevented() {
    let fees = FeeSchedule::default();
    // Near i64::MAX notional where gross fits in i64::checked_mul but gross + comm overflows i64,
    // and absurd share counts exceeding A-share sanity cap.
    let overflow_res = fees.compute_buy(Fen(1048), 8_784_163_844_623_500);
    assert!(
        overflow_res.is_err(),
        "FeeSchedule::compute_buy MUST fail cleanly instead of overflowing i64 on gross + comm"
    );

    let mut ledger = PortfolioLedger::new(Fen(100_000_000), "2026-11-18", fees);
    let buy_res = ledger.execute_buy(BuyOrderCommand {
        order_id: "ORD-R4-OVERFLOW".to_string(),
        rec_id: None,
        symbol: Symbol::parse("002131.SZ").unwrap(),
        name_zh: "利欧股份".to_string(),
        industry_sw1: "传媒".to_string(),
        quote_price_fen: Fen(1048),
        prev_close_fen: Fen(1000),
        is_st: false,
        shares: 8_784_163_844_623_500,
        stop_loss_price_fen: Fen(950),
        rationale_zh: "overflow probe".to_string(),
        trade_date: "2026-11-18".to_string(),
        next_trade_date: "2026-11-19".to_string(),
        created_at_iso: "2026-11-18T10:00:00+08:00".to_string(),
    });
    assert!(buy_res.is_err(), "PortfolioLedger::execute_buy MUST reject overflow shares");

    let sandbox = PathBuf::from("..")
        .join(".temp")
        .join("r4_sec_plugin_weights");
    let _ = std::fs::remove_dir_all(&sandbox);
    let repo_root = PathBuf::from("..");
    copy_dir_recursive(&repo_root.join("config"), &sandbox.join("config"));
    copy_dir_recursive(&repo_root.join("plugins"), &sandbox.join("plugins"));
    let paths = PortablePaths::from_root(sandbox).unwrap();
    let store = PluginStore::new(paths);

    let mut huge_weights = BTreeMap::new();
    huge_weights.insert("F1_ideal_reversal".to_string(), 1e308);
    huge_weights.insert("F2_concave_turnover".to_string(), 1e308);
    assert!(
        store
            .update_strategy_and_weights(5.5, 0.5, 25.0, -0.08, 2.2, "cn-red-up", &huge_weights)
            .is_err(),
        "Factor weights that overflow or exceed [0, 100] finite bounds MUST be rejected instead of zeroing weights"
    );

    let mut unknown_key = BTreeMap::new();
    unknown_key.insert("__proto__".to_string(), 50.0);
    assert!(
        store
            .update_strategy_and_weights(5.5, 0.5, 25.0, -0.08, 2.2, "cn-red-up", &unknown_key)
            .is_err(),
        "Unknown factor IDs MUST be rejected rather than polluted into strategy JSON"
    );
}
