use rusqlite::Connection;
use std::path::Path;

pub const DEFAULT_SESSIONS: &[(&str, &str, Option<&str>, &str)] = &[
    (
        "main_supervisor",
        "0626dc52-8683-4d1c-befc-16509212d024",
        None,
        "./memo.md",
    ),
    (
        "ui_designer",
        "cbdcae76-1934-41ac-8160-43be116e5b9b",
        Some("0626dc52-8683-4d1c-befc-16509212d024"),
        "./.github/agents/designer.agent.md",
    ),
    (
        "quant_researcher",
        "88ace910-c427-41db-a87f-46d8ae39735b",
        Some("0626dc52-8683-4d1c-befc-16509212d024"),
        "./.github/agents/quantitative-researcher.agent.md",
    ),
    (
        "risk_auditor",
        "b075d939-e095-482e-8424-4aba1882b97c",
        Some("0626dc52-8683-4d1c-befc-16509212d024"),
        "./.github/agents/ddd-tauri-architect.agent.md",
    ),
    (
        "economist_analyst",
        "a194e7c0-7f51-4e92-9b81-61f04c92e315",
        Some("0626dc52-8683-4d1c-befc-16509212d024"),
        "./scripts/analyzer.py",
    ),
];

const MIGRATION_DDL: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA temp_store = MEMORY;

CREATE TABLE IF NOT EXISTS agent_sessions (
    agent_role               TEXT PRIMARY KEY CHECK (agent_role IN (
                                 'main_supervisor', 'quant_researcher', 'ui_designer',
                                 'economist_analyst', 'risk_auditor'
                             )),
    session_id               TEXT NOT NULL UNIQUE CHECK (length(session_id) >= 32),
    parent_session_id        TEXT,
    model                    TEXT NOT NULL DEFAULT 'gemini-4-carbon'
                                 CHECK (model = 'gemini-4-carbon'),
    reasoning_effort         TEXT NOT NULL DEFAULT 'high'
                                 CHECK (reasoning_effort = 'high'),
    context_tier             TEXT NOT NULL DEFAULT 'long_context'
                                 CHECK (context_tier IN ('default', 'long_context')),
    cli_resume_flag          TEXT NOT NULL DEFAULT '--resume'
                                 CHECK (cli_resume_flag IN ('--session-id', '--resume')),
    resume_enabled           INTEGER NOT NULL DEFAULT 1 CHECK (resume_enabled IN (0, 1)),
    spec_rel_path            TEXT NOT NULL CHECK (instr(spec_rel_path, ':') = 0),
    spec_sha256              TEXT NOT NULL DEFAULT '',
    turn_count               INTEGER NOT NULL DEFAULT 1 CHECK (turn_count >= 0),
    status                   TEXT NOT NULL DEFAULT 'IDLE'
                                 CHECK (status IN ('IDLE', 'RUNNING', 'DEGRADED', 'RECOVERING')),
    last_woken_at            TEXT,
    last_completed_at        TEXT,
    last_error               TEXT,
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS stock_universe_cache (
    symbol                   TEXT PRIMARY KEY,
    name_zh                  TEXT NOT NULL,
    exchange                 TEXT NOT NULL,
    board                    TEXT NOT NULL,
    industry_sw1             TEXT NOT NULL DEFAULT '',
    listing_days             INTEGER NOT NULL DEFAULT 365,
    latest_price_fen         INTEGER NOT NULL,
    pct_chg                  REAL NOT NULL DEFAULT 0.0,
    turnover_pct             REAL NOT NULL DEFAULT 0.0,
    total_mv_yi              REAL NOT NULL DEFAULT 0.0,
    circ_mv_yi               REAL NOT NULL DEFAULT 0.0,
    main_net_inflow_wan      REAL NOT NULL DEFAULT 0.0,
    updated_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS factor_plugins (
    factor_id                TEXT PRIMARY KEY,
    name_zh                  TEXT NOT NULL,
    expr                     TEXT NOT NULL,
    bar_freq                 TEXT NOT NULL CHECK (bar_freq IN ('1d', '30m')),
    weight_pct               REAL NOT NULL,
    rank_ic_mean             REAL NOT NULL,
    ic_ir                    REAL NOT NULL,
    oos_win_rate_pct         REAL NOT NULL,
    evidence_tag             TEXT NOT NULL,
    status                   TEXT NOT NULL CHECK (status IN ('ACTIVE', 'CANDIDATE', 'DEPRECATED')),
    quantile_json            TEXT NOT NULL,
    ic_decay_json            TEXT NOT NULL,
    manifest_rel_path        TEXT NOT NULL CHECK (instr(manifest_rel_path, ':') = 0),
    updated_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS strategy_iterations (
    iteration_id             INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id              TEXT NOT NULL,
    version                  TEXT NOT NULL,
    manifest_rel_path        TEXT NOT NULL CHECK (instr(manifest_rel_path, ':') = 0),
    config_json              TEXT NOT NULL,
    promoted_by_role         TEXT NOT NULL DEFAULT 'quant_researcher',
    created_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recommendations (
    rec_id                   TEXT PRIMARY KEY,
    symbol                   TEXT NOT NULL,
    name_zh                  TEXT NOT NULL,
    board                    TEXT NOT NULL,
    bar_time                 TEXT NOT NULL,
    entry_price_fen          INTEGER NOT NULL CHECK (entry_price_fen > 0),
    stop_loss_price_fen      INTEGER NOT NULL CHECK (stop_loss_price_fen > 0 AND stop_loss_price_fen < entry_price_fen),
    target_price_fen         INTEGER,
    conviction_score         REAL NOT NULL,
    decision                 TEXT NOT NULL CHECK (decision IN ('BUY', 'WATCH', 'REJECTED_BY_MINE')),
    rationale_zh             TEXT NOT NULL CHECK (length(trim(rationale_zh)) > 0),
    pitfalls_json            TEXT NOT NULL,
    factor_z_json            TEXT NOT NULL,
    agent_session_id         TEXT NOT NULL,
    created_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS simulated_orders (
    order_id                 TEXT PRIMARY KEY,
    rec_id                   TEXT,
    symbol                   TEXT NOT NULL,
    name_zh                  TEXT NOT NULL,
    side                     TEXT NOT NULL CHECK (side IN ('BUY', 'SELL')),
    trigger_type             TEXT NOT NULL,
    status                   TEXT NOT NULL,
    lot_qty                  INTEGER NOT NULL CHECK (lot_qty > 0),
    entry_price_fen          INTEGER NOT NULL,
    fill_price_fen           INTEGER NOT NULL,
    stop_loss_price_fen      INTEGER NOT NULL CHECK (stop_loss_price_fen > 0),
    commission_fen           INTEGER NOT NULL,
    stamp_duty_fen           INTEGER NOT NULL,
    slippage_fen             INTEGER NOT NULL,
    rationale_zh             TEXT NOT NULL CHECK (length(trim(rationale_zh)) > 0),
    trade_date               TEXT NOT NULL,
    t_plus1_unlock_date      TEXT NOT NULL,
    created_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS positions (
    position_id              TEXT PRIMARY KEY,
    slot_no                  INTEGER NOT NULL UNIQUE CHECK (slot_no BETWEEN 1 AND 10),
    symbol                   TEXT NOT NULL UNIQUE,
    name_zh                  TEXT NOT NULL,
    board                    TEXT NOT NULL,
    industry_sw1             TEXT NOT NULL DEFAULT '',
    total_qty                INTEGER NOT NULL CHECK (total_qty > 0),
    sellable_qty             INTEGER NOT NULL CHECK (sellable_qty >= 0),
    locked_t0_qty            INTEGER NOT NULL CHECK (locked_t0_qty >= 0),
    avg_cost_fen             INTEGER NOT NULL,
    mark_price_fen           INTEGER NOT NULL,
    highest_price_fen        INTEGER NOT NULL,
    stop_loss_price_fen      INTEGER NOT NULL CHECK (stop_loss_price_fen > 0),
    entry_rationale_zh       TEXT NOT NULL,
    opened_trade_date        TEXT NOT NULL,
    last_buy_trade_date      TEXT NOT NULL,
    opened_at                TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS portfolio_snapshots (
    snap_date                TEXT PRIMARY KEY,
    cash_fen                 INTEGER NOT NULL CHECK (cash_fen >= 0),
    market_value_fen         INTEGER NOT NULL CHECK (market_value_fen >= 0),
    total_equity_fen         INTEGER NOT NULL CHECK (total_equity_fen >= 0),
    total_fees_paid_fen      INTEGER NOT NULL DEFAULT 0,
    portfolio_nav            REAL NOT NULL,
    csi1000_nav              REAL NOT NULL,
    cni2000_nav              REAL NOT NULL,
    excess_pct               REAL NOT NULL,
    updated_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS economist_news_index (
    report_id                TEXT PRIMARY KEY,
    report_date              TEXT NOT NULL,
    title_zh                 TEXT NOT NULL,
    rel_path                 TEXT NOT NULL CHECK (instr(rel_path, ':') = 0),
    article_count            INTEGER NOT NULL DEFAULT 0,
    core_count               INTEGER NOT NULL DEFAULT 0,
    acked                    INTEGER NOT NULL DEFAULT 0 CHECK (acked IN (0, 1)),
    acked_at                 TEXT,
    us_score                 REAL NOT NULL DEFAULT 50.0,
    cn_score                 REAL NOT NULL DEFAULT 50.0,
    executive_summary_zh     TEXT NOT NULL DEFAULT '',
    chains_json              TEXT NOT NULL DEFAULT '[]',
    updated_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS telemetry_events (
    event_id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_iso                   TEXT NOT NULL,
    epoch_secs               INTEGER NOT NULL,
    severity                 TEXT NOT NULL,
    subsystem                TEXT NOT NULL,
    event_name               TEXT NOT NULL,
    duration_ms              INTEGER,
    ok                       INTEGER NOT NULL CHECK (ok IN (0, 1)),
    message                  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recommendations_symbol ON recommendations(symbol, bar_time);
CREATE INDEX IF NOT EXISTS idx_orders_created_at ON simulated_orders(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_telemetry_epoch ON telemetry_events(epoch_secs DESC);
"#;

pub fn open_and_migrate_sqlite(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建SQLite目录失败: {}", e))?;
    }
    let conn = Connection::open(db_path)
        .map_err(|e| format!("打开Bundled SQLite数据库失败: {}", e))?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))
        .map_err(|e| format!("设置SQLite busy_timeout失败: {}", e))?;
    conn.execute_batch(MIGRATION_DDL)
        .map_err(|e| format!("执行10表领域Schema迁移失败: {}", e))?;
    let _ = conn.execute_batch(
        "UPDATE telemetry_events
            SET subsystem = CASE subsystem
                WHEN 'SYSTEM' THEN 'CLEANER'
                WHEN 'FACTOR' THEN 'QUANT'
                WHEN 'PORTFOLIO' THEN 'TRADING'
                WHEN 'ECONOMIST' THEN 'NEWS'
                ELSE subsystem
            END
          WHERE subsystem IN ('SYSTEM', 'FACTOR', 'PORTFOLIO', 'ECONOMIST');",
    );

    let existing_cnt: i64 = conn
        .query_row("SELECT COUNT(*) FROM agent_sessions", [], |r| r.get(0))
        .unwrap_or(0);
    if existing_cnt < (DEFAULT_SESSIONS.len() as i64) {
        let now_iso = chrono::Utc::now().to_rfc3339();
        for &(role, sess_id, parent_opt, spec_rel) in DEFAULT_SESSIONS {
            conn.execute(
                "INSERT INTO agent_sessions (
                    agent_role, session_id, parent_session_id, model, reasoning_effort,
                    context_tier, cli_resume_flag, resume_enabled, spec_rel_path,
                    spec_sha256, turn_count, status, last_woken_at, last_completed_at,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, 'gemini-4-carbon', 'high', 'long_context', '--resume',
                           1, ?4, '', 1, 'IDLE', ?5, ?5, ?5, ?5)
                 ON CONFLICT(agent_role) DO NOTHING",
                rusqlite::params![role, sess_id, parent_opt, spec_rel, now_iso],
            )
            .map_err(|e| format!("初始化 agent_sessions [{}] 失败: {}", role, e))?;
        }
    }
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn migrates_all_ten_tables_with_wal_and_seeds_gemini4_carbon_sessions() {
        let dir = PathBuf::from(".temp").join("test_sqlite_schema");
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("schema_test.db");
        let _ = std::fs::remove_file(&db);

        let conn = open_and_migrate_sqlite(&db).unwrap();
        let journal: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal.to_ascii_lowercase(), "wal");

        let expected = [
            "agent_sessions",
            "stock_universe_cache",
            "factor_plugins",
            "strategy_iterations",
            "recommendations",
            "simulated_orders",
            "positions",
            "portfolio_snapshots",
            "economist_news_index",
            "telemetry_events",
        ];
        for tbl in expected {
            let cnt: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [tbl],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(cnt, 1, "missing table {}", tbl);
        }

        let sess_cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_sessions WHERE model='gemini-4-carbon' AND reasoning_effort='high'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sess_cnt, 5);
    }
}
