"""Bootstrap portable SQLite database and persist Main & Expert Agent sessionIds.

Enforces Requirement #2 (zero absolute paths), Requirement #11 (SQLite sessionId
persistence for waking original agent contexts), and Requirement #13 (Gemini 4 Carbon
with high reasoning effort).
"""

from __future__ import annotations

from datetime import datetime, timezone
import hashlib
from pathlib import Path
import sqlite3

ROOT_DIR = Path(__file__).resolve().parent.parent
DB_REL_PATH = Path("data") / "economist_quant.db"


SCHEMA_SQL = """
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
"""

INITIAL_SESSIONS = [
    (
        "main_supervisor",
        "0626dc52-8683-4d1c-befc-16509212d024",
        None,
        "./memo.md",
    ),
    (
        "ui_designer",
        "cbdcae76-1934-41ac-8160-43be116e5b9b",
        "0626dc52-8683-4d1c-befc-16509212d024",
        "./.github/agents/designer.agent.md",
    ),
    (
        "quant_researcher",
        "88ace910-c427-41db-a87f-46d8ae39735b",
        "0626dc52-8683-4d1c-befc-16509212d024",
        "./.github/agents/quantitative-researcher.agent.md",
    ),
    (
        "risk_auditor",
        "b075d939-e095-482e-8424-4aba1882b97c",
        "0626dc52-8683-4d1c-befc-16509212d024",
        "./.github/agents/ddd-tauri-architect.agent.md",
    ),
    (
        "economist_analyst",
        "a194e7c0-7f51-4e92-9b81-61f04c92e315",
        "0626dc52-8683-4d1c-befc-16509212d024",
        "./scripts/analyzer.py",
    ),
]


def _file_sha256(rel_path: str) -> str:
    target = ROOT_DIR / rel_path.lstrip("./")
    if not target.exists():
        return ""
    return hashlib.sha256(target.read_bytes()).hexdigest()


def bootstrap_sessions() -> Path:
    db_path = ROOT_DIR / DB_REL_PATH
    db_path.parent.mkdir(parents=True, exist_ok=True)
    now_iso = datetime.now(timezone.utc).isoformat()
    with sqlite3.connect(db_path) as conn:
        conn.executescript(SCHEMA_SQL)
        for role, sess_id, parent_id, spec_rel in INITIAL_SESSIONS:
            sha = _file_sha256(spec_rel)
            conn.execute(
                """
                INSERT INTO agent_sessions (
                    agent_role, session_id, parent_session_id, model,
                    reasoning_effort, context_tier, cli_resume_flag,
                    resume_enabled, spec_rel_path, spec_sha256,
                    turn_count, status, last_woken_at, last_completed_at,
                    created_at, updated_at
                ) VALUES (?, ?, ?, 'gemini-4-carbon', 'high', 'long_context', '--resume',
                          1, ?, ?, 1, 'IDLE', ?, ?, ?, ?)
                ON CONFLICT(agent_role) DO UPDATE SET
                    session_id = excluded.session_id,
                    parent_session_id = excluded.parent_session_id,
                    spec_rel_path = excluded.spec_rel_path,
                    spec_sha256 = excluded.spec_sha256,
                    updated_at = excluded.updated_at
                """,
                (role, sess_id, parent_id, spec_rel, sha, now_iso, now_iso, now_iso, now_iso),
            )
        conn.commit()
    return db_path


if __name__ == "__main__":
    out = bootstrap_sessions()
    with sqlite3.connect(out) as conn:
        rows = conn.execute(
            "SELECT agent_role, session_id, model, reasoning_effort, spec_rel_path FROM agent_sessions"
        ).fetchall()
    for r in rows:
        print(f"[SQLite agent_sessions] {r[0]} -> {r[1]} ({r[2]} / {r[3]}) [{r[4]}]")
