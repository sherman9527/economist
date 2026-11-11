# -*- coding: utf-8 -*-
"""
Local lightweight HTTP server & state manager for report ACK persistence.
Binds strictly to 127.0.0.1:4288 (or ECONOMIST_PORT).
Provides:
  - GET  /                    -> Redirects to /reports/index.html
  - GET  /reports/<path>      -> Serves archived HTML reports from D:\\code\\economist\\reports
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
    LOGS_DIR,
    REPORTS_DIR,
    SCRIPTS_DIR,
    TEMP_DIR,
)

STATE_LOCK = threading.Lock()


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
            "abs_path": str((REPORTS_DIR / rel_path).resolve()),
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
    """Update ACK state for a specific report_id and patch its static HTML snapshot."""
    state = load_ack_state()
    now_str = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    target_entry = None
    with STATE_LOCK:
        if report_id not in state["reports"]:
            state["reports"][report_id] = {
                "report_id": report_id,
                "date": report_id[:10],
                "title": f"经济学人A股/美股深度投研总览 ({report_id})",
                "rel_path": f"{report_id[:4]}/{report_id[5:7]}/{report_id[8:10]}/economist_report_{report_id}.html",
                "generated_at": now_str,
                "acked": False,
                "acked_at": None,
            }
        entry = state["reports"][report_id]
        entry["acked"] = bool(acked)
        entry["acked_at"] = now_str if acked else None
        if note:
            entry["note"] = note
        entry["updated_at"] = now_str
        save_ack_state_unlocked(state)
        target_entry = dict(entry)

    # Also update embedded initial JSON inside the HTML file on disk so file:// viewing reflects ACK state
    try:
        rel_path = target_entry.get("rel_path", "")
        if rel_path:
            html_file = REPORTS_DIR / Path(rel_path)
            if html_file.exists():
                html_text = html_file.read_text(encoding="utf-8")
                new_meta = json.dumps(
                    {
                        "report_id": report_id,
                        "acked": bool(acked),
                        "acked_at": target_entry["acked_at"],
                    },
                    ensure_ascii=False,
                )
                patched = re.sub(
                    r"(/\*__ACK_SNAPSHOT_START__\*/).*?(/\*__ACK_SNAPSHOT_END__\*/)",
                    rf"\1{new_meta}\2",
                    html_text,
                    flags=re.S,
                )
                if patched != html_text:
                    html_file.write_text(patched, encoding="utf-8")
    except Exception:
        pass

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
    def _send_cors_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")

    def do_OPTIONS(self):
        self.send_response(204)
        self._send_cors_headers()
        self.end_headers()

    def do_GET(self):
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

        if path == "/api/ack-via-get":
            qs = parse_qs(parsed.query)
            report_id = (qs.get("report_id") or [""])[0].strip()
            acked_val = (qs.get("acked") or ["1"])[0].strip().lower() in ("1", "true", "yes")
            if not report_id:
                self._respond_json(400, {"status": "error", "message": "Missing report_id"})
                return
            entry = set_report_ack(report_id, acked=acked_val)
            self._respond_json(200, {"status": "ok", "entry": entry, "unacked_reports": get_unacked_reports()})
            return

        if path in ("/", "/index.html", "/reports", "/reports/"):
            self.send_response(302)
            self._send_cors_headers()
            self.send_header("Location", "/reports/index.html")
            self.end_headers()
            return

        if path.startswith("/reports/"):
            rel = path[len("/reports/") :].lstrip("/\\")
            target = (REPORTS_DIR / rel).resolve()
            if not str(target).startswith(str(REPORTS_DIR.resolve())) or not target.is_file():
                self.send_response(404)
                self._send_cors_headers()
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.end_headers()
                self.wfile.write(f"Report not found: {rel}".encode("utf-8"))
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
        parsed = urlparse(self.path)
        if parsed.path != "/api/ack":
            self._respond_json(404, {"status": "error", "message": "Not found"})
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
            raw_body = self.rfile.read(length).decode("utf-8") if length > 0 else "{}"
            payload = json.loads(raw_body)
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
