# -*- coding: utf-8 -*-
"""
Local lightweight HTTP server & state manager for report ACK persistence.
Binds strictly to 127.0.0.1:4288 (or ECONOMIST_PORT).
Provides:
  - GET  /                    -> Redirects to /reports/index.html
  - GET  /reports/<path>      -> Serves archived HTML reports from ./reports
  - GET  /api/ack-state       -> Returns full ack_state.json + unacked reports list
  - POST /api/ack             -> Updates ACK status for a report_id in ack_state.json & HTML snapshot
  - GET  /api/health          -> Health check
"""

import json
import os
import re
import socket
import subprocess
import sys
import threading
import time
from datetime import datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, unquote, urlparse

from config import (
    ACK_SERVER_HOST,
    ACK_SERVER_PORT,
    ACK_STATE_FILE,
    BASE_DIR,
    DATA_DIR,
    LOGS_DIR,
    REPORTS_DIR,
    SCRIPTS_DIR,
    TEMP_DIR,
)

STATE_LOCK = threading.Lock()
REPORT_ID_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
MAX_POST_BODY_BYTES = 65_536


def _validate_report_id(report_id: str) -> str:
    rid = str(report_id or "").strip()
    if not REPORT_ID_RE.match(rid):
        raise ValueError(f"Invalid report_id format: {rid!r}")
    try:
        datetime.strptime(rid, "%Y-%m-%d")
    except ValueError as exc:
        raise ValueError(f"Invalid calendar date in report_id: {rid!r}") from exc
    return rid


def _resolve_inside_reports_dir(rel_path: str) -> Path | None:
    clean_rel = str(rel_path or "").replace("\\", "/").lstrip("/")
    if not clean_rel:
        return None
    raw_candidate = REPORTS_DIR / Path(clean_rel)
    base_resolved = REPORTS_DIR.resolve()
    try:
        # Reject symlinks along any path segment inside REPORTS_DIR
        cur = REPORTS_DIR
        for part in Path(clean_rel).parts:
            if part in ("", ".", ".."):
                if part == "..":
                    return None
                continue
            cur = cur / part
            if cur.exists() and cur.is_symlink():
                return None
        target = raw_candidate.resolve()
    except OSError:
        return None
    if target == base_resolved or not target.is_relative_to(base_resolved):
        return None
    return target


def _safe_inline_json(val: dict) -> str:
    return (
        json.dumps(val, ensure_ascii=False)
        .replace("<", "\\u003c")
        .replace(">", "\\u003e")
        .replace("&", "\\u0026")
        .replace("\u2028", "\\u2028")
        .replace("\u2029", "\\u2029")
    )


def load_ack_state() -> dict:
    """Load ACK state registry from disk with safe fallback."""
    with STATE_LOCK:
        if not ACK_STATE_FILE.exists():
            default_state = {"updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"), "reports": {}}
            save_ack_state_unlocked(default_state)
            return default_state
        try:
            data = json.loads(ACK_STATE_FILE.read_text(encoding="utf-8"))
            if "reports" not in data or not isinstance(data["reports"], dict):
                data["reports"] = {}
            return data
        except Exception:
            return {"updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"), "reports": {}}


def save_ack_state_unlocked(state: dict) -> None:
    state["updated_at"] = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    tmp_path = ACK_STATE_FILE.with_suffix(".json.tmp")
    tmp_path.write_text(json.dumps(state, ensure_ascii=False, indent=2), encoding="utf-8")
    tmp_path.replace(ACK_STATE_FILE)


def register_report(
    report_id: str,
    report_date: str,
    title: str,
    rel_path: str,
    article_count: int,
    core_count: int,
    window_desc: str,
    preserve_existing_ack: bool = True,
) -> dict:
    """Register or update a generated HTML report in ack_state.json."""
    state = load_ack_state()
    with STATE_LOCK:
        existing = state["reports"].get(report_id, {})
        acked = existing.get("acked", False) if preserve_existing_ack else False
        acked_at = existing.get("acked_at", None) if (preserve_existing_ack and acked) else None
        now_str = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        entry = {
            "report_id": report_id,
            "date": report_date,
            "title": title,
            "rel_path": rel_path.replace("\\", "/"),
            "article_count": article_count,
            "core_count": core_count,
            "window_desc": window_desc,
            "generated_at": existing.get("generated_at", now_str),
            "updated_at": now_str,
            "acked": bool(acked),
            "acked_at": acked_at,
            "open_count": existing.get("open_count", 0),
            "last_opened_at": existing.get("last_opened_at", None),
        }
        state["reports"][report_id] = entry
        save_ack_state_unlocked(state)
    return entry


def set_report_ack(report_id: str, acked: bool = True, note: str = "") -> dict:
    """Update ACK state for a validated existing report_id and patch its static HTML snapshot."""
    report_id = _validate_report_id(report_id)
    safe_note = str(note or "")[:500]
    state = load_ack_state()
    now_str = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    target_entry = None
    with STATE_LOCK:
        if report_id not in state["reports"]:
            default_rel = (
                f"{report_id[:4]}/{report_id[5:7]}/{report_id[8:10]}/"
                f"economist_report_{report_id}.html"
            )
            existing_html = _resolve_inside_reports_dir(default_rel)
            if existing_html is None or not existing_html.is_file():
                raise ValueError(f"Unknown report_id: {report_id}")
            state["reports"][report_id] = {
                "report_id": report_id,
                "date": report_id,
                "title": f"经济学人A股/美股深度投研总览 ({report_id})",
                "rel_path": default_rel,
                "generated_at": now_str,
                "acked": False,
                "acked_at": None,
            }
        entry = state["reports"][report_id]
        entry["acked"] = bool(acked)
        entry["acked_at"] = now_str if acked else None
        if safe_note:
            entry["note"] = safe_note
        entry["updated_at"] = now_str
        save_ack_state_unlocked(state)
        target_entry = dict(entry)

    # Also update embedded initial JSON inside the HTML file on disk so file:// viewing reflects ACK state
    rel_path = target_entry.get("rel_path", "")
    if rel_path:
        html_file = _resolve_inside_reports_dir(rel_path)
        if html_file is not None and html_file.is_file():
            html_text = html_file.read_text(encoding="utf-8")
            new_meta = _safe_inline_json(
                {
                    "report_id": report_id,
                    "acked": bool(acked),
                    "acked_at": target_entry["acked_at"],
                }
            )
            patched = re.sub(
                r"(/\*__ACK_SNAPSHOT_START__\*/).*?(/\*__ACK_SNAPSHOT_END__\*/)",
                lambda m: f"{m.group(1)}{new_meta}{m.group(2)}",
                html_text,
                flags=re.S,
            )
            if patched != html_text:
                html_file.write_text(patched, encoding="utf-8")

    # Refresh reports/index.html archive index if renderer is available
    try:
        from renderer import render_archive_index
        render_archive_index()
    except Exception:
        pass

    return target_entry


def record_report_opened(report_id: str) -> None:
    state = load_ack_state()
    now_str = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    with STATE_LOCK:
        if report_id in state["reports"]:
            state["reports"][report_id]["open_count"] = state["reports"][report_id].get("open_count", 0) + 1
            state["reports"][report_id]["last_opened_at"] = now_str
            save_ack_state_unlocked(state)


def get_unacked_reports() -> list[dict]:
    """Return all registered reports whose acked == False, sorted chronologically (oldest first, latest last)."""
    state = load_ack_state()
    unacked = []
    for rep_id, info in state.get("reports", {}).items():
        rel_path = info.get("rel_path", "")
        html_path = REPORTS_DIR / Path(rel_path) if rel_path else None
        if html_path and html_path.exists() and not info.get("acked", False):
            unacked.append(info)
    unacked.sort(key=lambda x: x.get("date", x.get("report_id", "")))
    return unacked


def is_server_running(host: str = ACK_SERVER_HOST, port: int = ACK_SERVER_PORT) -> bool:
    try:
        with socket.create_connection((host, port), timeout=1.0):
            return True
    except OSError:
        return False


def ensure_ack_server_running() -> str:
    """Ensure the background ACK server is listening on 127.0.0.1:4288 and return base URL."""
    base_url = f"http://{ACK_SERVER_HOST}:{ACK_SERVER_PORT}"
    if is_server_running():
        return base_url

    log_file = LOGS_DIR / "ack_server.log"
    env = os.environ.copy()
    env["TEMP"] = str(TEMP_DIR)
    env["TMP"] = str(TEMP_DIR)
    env["PYTHONIOENCODING"] = "utf-8"
    env.pop("PYTHONHOME", None)
    env.pop("PYTHONPATH", None)

    creationflags = 0
    if sys.platform == "win32":
        creationflags = subprocess.CREATE_NEW_PROCESS_GROUP | subprocess.DETACHED_PROCESS

    with open(log_file, "a", encoding="utf-8") as lf:
        subprocess.Popen(
            [sys.executable, "-E", "-X", "utf8", str(SCRIPTS_DIR / "ack_server.py"), "--serve"],
            cwd=str(BASE_DIR),
            env=env,
            stdout=lf,
            stderr=lf,
            creationflags=creationflags,
        )

    for _ in range(25):
        if is_server_running():
            return base_url
        time.sleep(0.2)
    return base_url


class AckRequestHandler(BaseHTTPRequestHandler):
    def _allowed_origins(self) -> set[str]:
        port = self.server.server_port
        return {f"http://127.0.0.1:{port}", f"http://localhost:{port}"}

    def _is_valid_host(self) -> bool:
        raw_host = (self.headers.get("Host") or "").strip().lower()
        if not raw_host:
            return True
        port = self.server.server_port
        return raw_host in {
            "127.0.0.1",
            "localhost",
            f"127.0.0.1:{port}",
            f"localhost:{port}",
        }

    def _is_trusted_origin(self, allow_missing: bool = True) -> bool:
        origin = self.headers.get("Origin")
        if origin is None:
            return allow_missing
        return origin.strip().lower() in self._allowed_origins()

    def _send_cors_headers(self):
        origin = (self.headers.get("Origin") or "").strip()
        if origin.lower() in self._allowed_origins():
            self.send_header("Access-Control-Allow-Origin", origin)
            self.send_header("Vary", "Origin")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")

    def do_OPTIONS(self):
        if not self._is_valid_host() or not self._is_trusted_origin(allow_missing=True):
            self._respond_json(403, {"status": "error", "message": "Forbidden origin or host"})
            return
        self.send_response(204)
        self._send_cors_headers()
        self.end_headers()

    def do_GET(self):
        if not self._is_valid_host():
            self._respond_json(403, {"status": "error", "message": "Forbidden Host header"})
            return

        parsed = urlparse(self.path)
        path = unquote(parsed.path)

        if path == "/api/health":
            self._respond_json(200, {"status": "ok", "time": datetime.now().strftime("%Y-%m-%d %H:%M:%S")})
            return

        if path == "/api/ack-state":
            state = load_ack_state()
            unacked = get_unacked_reports()
            self._respond_json(
                200,
                {
                    "status": "ok",
                    "updated_at": state.get("updated_at"),
                    "reports": state.get("reports", {}),
                    "unacked_count": len(unacked),
                    "unacked_reports": unacked,
                },
            )
            return

        if path == "/api/audit-summary":
            audit_files = sorted(DATA_DIR.glob("full_retrospective_audit_*.json"), reverse=True)
            if audit_files:
                try:
                    audit_data = json.loads(audit_files[0].read_text(encoding="utf-8"))
                    self._respond_json(200, {"status": "ok", "audit": audit_data})
                    return
                except Exception as e:
                    self._respond_json(500, {"status": "error", "message": str(e)})
                    return
            self._respond_json(200, {"status": "ok", "audit": None})
            return

        if path == "/api/ack-via-get":
            self._respond_json(405, {"status": "error", "message": "State-mutating GET endpoint is disabled; use POST /api/ack"})
            return

        if path in ("/", "/index.html", "/reports", "/reports/"):
            self.send_response(302)
            self._send_cors_headers()
            self.send_header("Location", "/reports/index.html")
            self.end_headers()
            return

        if path.startswith("/reports/"):
            rel = path[len("/reports/") :]
            target = _resolve_inside_reports_dir(rel)
            if target is None or not target.is_file():
                self.send_response(404)
                self._send_cors_headers()
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.end_headers()
                self.wfile.write(b"Report not found")
                return

            content = target.read_bytes()
            self.send_response(200)
            self._send_cors_headers()
            suf = target.suffix.lower()
            if suf == ".html":
                ctype = "text/html; charset=utf-8"
            elif suf == ".js":
                ctype = "application/javascript; charset=utf-8"
            elif suf == ".css":
                ctype = "text/css; charset=utf-8"
            elif suf == ".json":
                ctype = "application/json; charset=utf-8"
            else:
                ctype = "application/octet-stream"
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            self.wfile.write(content)
            return

        self._respond_json(404, {"status": "error", "message": f"Unknown path: {path}"})

    def do_POST(self):
        if not self._is_valid_host() or not self._is_trusted_origin(allow_missing=True):
            self._respond_json(403, {"status": "error", "message": "Cross-origin or untrusted Host POST denied"})
            return

        parsed = urlparse(self.path)
        if parsed.path not in ("/api/ack", "/api/ack-batch"):
            self._respond_json(404, {"status": "error", "message": "Not found"})
            return

        ctype = (self.headers.get("Content-Type") or "").split(";")[0].strip().lower()
        if ctype != "application/json":
            self._respond_json(415, {"status": "error", "message": "Content-Type must be application/json"})
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            self._respond_json(400, {"status": "error", "message": "Invalid Content-Length"})
            return
        if length < 0:
            self._respond_json(400, {"status": "error", "message": "Negative Content-Length"})
            return
        if length > MAX_POST_BODY_BYTES:
            self._respond_json(413, {"status": "error", "message": "Payload exceeds 64 KiB limit"})
            return

        try:
            raw_body = self.rfile.read(length).decode("utf-8") if length > 0 else "{}"
            payload = json.loads(raw_body)
            if not isinstance(payload, dict):
                self._respond_json(400, {"status": "error", "message": "JSON body must be an object"})
                return

            if parsed.path == "/api/ack-batch":
                report_ids = payload.get("report_ids") or []
                if not isinstance(report_ids, list) or len(report_ids) > 500:
                    self._respond_json(400, {"status": "error", "message": "Invalid report_ids list"})
                    return
                acked = bool(payload.get("acked", True))
                updated_entries = []
                for rid in report_ids:
                    rid_s = str(rid).strip()
                    if rid_s:
                        updated_entries.append(set_report_ack(rid_s, acked=acked))
                state = load_ack_state()
                self._respond_json(
                    200,
                    {
                        "status": "ok",
                        "updated_count": len(updated_entries),
                        "reports": state.get("reports", {}),
                        "unacked_reports": get_unacked_reports(),
                    },
                )
                return

            report_id = str(payload.get("report_id", "")).strip()
            acked = bool(payload.get("acked", True))
            note = str(payload.get("note", "")).strip()
            if not report_id:
                self._respond_json(400, {"status": "error", "message": "report_id is required"})
                return
            entry = set_report_ack(report_id, acked=acked, note=note)
            state = load_ack_state()
            self._respond_json(
                200,
                {
                    "status": "ok",
                    "entry": entry,
                    "reports": state.get("reports", {}),
                    "unacked_reports": get_unacked_reports(),
                },
            )
        except (ValueError, json.JSONDecodeError) as e:
            self._respond_json(400, {"status": "error", "message": str(e)})
        except Exception as e:
            self._respond_json(500, {"status": "error", "message": str(e)})

    def _respond_json(self, status_code: int, data: dict):
        body = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(status_code)
        self._send_cors_headers()
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        log_file = LOGS_DIR / "ack_server.log"
        try:
            with open(log_file, "a", encoding="utf-8") as f:
                f.write(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {fmt % args}\n")
        except Exception:
            pass


def run_server():
    server = ThreadingHTTPServer((ACK_SERVER_HOST, ACK_SERVER_PORT), AckRequestHandler)
    print(f"ACK Server listening on http://{ACK_SERVER_HOST}:{ACK_SERVER_PORT}")
    server.serve_forever()


if __name__ == "__main__":
    if "--serve" in sys.argv:
        run_server()
    elif "--ensure" in sys.argv:
        url = ensure_ack_server_running()
        print(url)
    elif "--status" in sys.argv:
        print(json.dumps(load_ack_state(), ensure_ascii=False, indent=2))
    else:
        url = ensure_ack_server_running()
        print(f"ACK Server active at {url}")
