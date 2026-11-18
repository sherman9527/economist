"""Round 4 security regression tests for ack_server.py and renderer_pkg."""
import http.client
import json
import shutil
import socket
import sys
import threading
import unittest
from http.server import ThreadingHTTPServer
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = ROOT_DIR / "scripts"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import ack_server  # noqa: E402
from renderer_pkg import archive_index, daily_body, daily_report, daily_ui_js  # noqa: E402


class Round4SecurityTests(unittest.TestCase):
    def setUp(self):
        self.sandbox = ROOT_DIR / ".temp" / "r4_py_sec_sandbox"
        shutil.rmtree(self.sandbox, ignore_errors=True)
        self.reports_dir = self.sandbox / "reports"
        self.sibling_dir = self.sandbox / "reports_backup"
        self.data_dir = self.sandbox / "data"
        (self.reports_dir / "2026" / "11" / "18").mkdir(parents=True, exist_ok=True)
        self.sibling_dir.mkdir(parents=True, exist_ok=True)
        self.data_dir.mkdir(parents=True, exist_ok=True)

        self.report_html = (
            self.reports_dir / "2026" / "11" / "18" / "economist_report_2026-11-18.html"
        )
        self.report_html.write_text(
            "<!doctype html><html><head></head><body><script>"
            'const INITIAL_ACK_SNAPSHOT = /*__ACK_SNAPSHOT_START__*/{"acked": false, "acked_at": null}/*__ACK_SNAPSHOT_END__*/;'
            "</script></body></html>",
            encoding="utf-8",
        )
        (self.sibling_dir / "secret.txt").write_text("TOP_SECRET", encoding="utf-8")
        self.ack_state_file = self.data_dir / "ack_state.json"
        self.ack_state_file.write_text(
            json.dumps(
                {
                    "updated_at": "2026-11-18 09:00:00",
                    "reports": {
                        "2026-11-18": {
                            "report_id": "2026-11-18",
                            "date": "2026-11-18",
                            "title": "Test Report",
                            "rel_path": "2026/11/18/economist_report_2026-11-18.html",
                            "acked": False,
                            "acked_at": None,
                        }
                    },
                }
            ),
            encoding="utf-8",
        )

        self._orig_reports_dir = ack_server.REPORTS_DIR
        self._orig_ack_file = ack_server.ACK_STATE_FILE
        ack_server.REPORTS_DIR = self.reports_dir
        ack_server.ACK_STATE_FILE = self.ack_state_file

        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(("127.0.0.1", 0))
        _, self.port = sock.getsockname()
        sock.close()

        self.httpd = ThreadingHTTPServer(("127.0.0.1", self.port), ack_server.AckRequestHandler)
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()

    def tearDown(self):
        self.httpd.shutdown()
        self.httpd.server_close()
        ack_server.REPORTS_DIR = self._orig_reports_dir
        ack_server.ACK_STATE_FILE = self._orig_ack_file
        shutil.rmtree(self.sandbox, ignore_errors=True)

    def _req(self, method: str, path: str, body: bytes | None = None, headers: dict | None = None):
        conn = http.client.HTTPConnection("127.0.0.1", self.port, timeout=5)
        hdrs = dict(headers or {})
        conn.request(method, path, body=body, headers=hdrs)
        resp = conn.getresponse()
        data = resp.read()
        resp_headers = {k.lower(): v for k, v in resp.getheaders()}
        conn.close()
        return resp.status, resp_headers, data

    def test_alert1_script_json_url_scheme_and_report_id_xss_hardening(self):
        # 1. archive_index.render_archive_index must escape </script> inside inline JSON
        self.ack_state_file.write_text(
            json.dumps(
                {
                    "reports": {
                        "2026-11-18": {
                            "report_id": "2026-11-18",
                            "date": "2026-11-18",
                            "title": "Breakout </script><script>alert(1)</script>",
                            "rel_path": "2026/11/18/economist_report_2026-11-18.html",
                            "acked": False,
                        }
                    }
                }
            ),
            encoding="utf-8",
        )
        out_index = self.reports_dir / "index.html"
        orig_index_dir = archive_index.REPORTS_DIR
        try:
            archive_index.REPORTS_DIR = self.reports_dir
            archive_index.render_archive_index()
            idx_html = out_index.read_text(encoding="utf-8")
            self.assertFalse(
                "</script><script>alert(1)" in idx_html,
                "raw </script> breakout present in archive index.html",
            )
        finally:
            archive_index.REPORTS_DIR = orig_index_dir
            self.ack_state_file.write_text(
                json.dumps(
                    {
                        "updated_at": "2026-11-18 09:00:00",
                        "reports": {
                            "2026-11-18": {
                                "report_id": "2026-11-18",
                                "date": "2026-11-18",
                                "title": "Test Report",
                                "rel_path": "2026/11/18/economist_report_2026-11-18.html",
                                "acked": False,
                                "acked_at": None,
                            }
                        },
                    }
                ),
                encoding="utf-8",
            )

        # 2. daily_body must escape </script> inside inline JSON constants
        body_html = daily_body.build_daily_body_and_vars(
            {
                "report_date": "2026-11-18",
                "window_desc": "2026-11-18",
                "generated_at": "2026-11-18 08:00:00",
                "kline_map": {"evil": "</script><script>alert(2)</script>"},
                "core_articles": [],
                "digest_articles": [],
                "unacked_banner_html": "",
                "toc_links_html": "",
                "archive_items_html": "",
                "headline_zh": "ok",
                "us_score": 55,
                "cn_score": 62,
                "spy_k": {},
                "qqq_k": {},
                "sh_k": {},
                "cyb_k": {},
                "exec_cards_html": "",
                "kline_tabs_html": "",
                "backtest_summary": {
                    "total_evaluated": 0,
                    "total_hits": 0,
                    "overall_hit_rate": 0.0,
                    "long_hit_rate": 0.0,
                    "long_hits": 0,
                    "long_total": 0,
                    "short_hit_rate": 0.0,
                    "short_hits": 0,
                    "short_total": 0,
                    "avg_long_ret_5d": 0.0,
                    "avg_short_ret_5d": 0.0,
                    "long_short_spread_5d": 0.0,
                    "us_hit_rate": 0.0,
                    "cn_hit_rate": 0.0,
                    "mkt_leader_note": "",
                },
                "bt_records": [],
                "bt_hit_count": 0,
                "bt_miss_count": 0,
                "backtest_rows_html": [],
                "sector_rows_html": [],
                "master_stock_rows_html": [],
                "pair_trade_rows_html": [],
                "article_cards_html": [],
                "digest_rows_html": [],
                "initial_ack_snapshot": '{"acked":false}',
                "stock_meta_by_sym": {},
                "sector_chart_categories": [],
                "sector_us_scores": [],
                "sector_cn_scores": [],
                "sector_real_5d": [],
                "diverging_items": [],
                "scatter_points": [],
                "per_article_chart_configs": {},
                "bt_chart_payload": {},
                "sector_scorecard": [],
                "all_stock_rows": [],
                "bull_stocks": [],
                "bear_stocks": [],
                "us_stocks": [],
                "cn_stocks": [],
                "chains": [],
            }
        )
        self.assertFalse(
            "</script><script>alert(2)" in body_html,
            "raw </script> breakout present in daily_body.py",
        )

        # 3. daily_ui_js must escape r.report_id in renderUnackedBanner and omit ack-via-get
        ui_js = daily_ui_js.DAILY_UI_JS
        self.assertNotIn("ack-via-get", ui_js)
        self.assertNotIn("quickAckOther('${r.report_id}')", ui_js)

        # 4. daily_report must block javascript: article links
        orig_rep_dir = daily_report.REPORTS_DIR
        orig_fetch_k = daily_report.fetch_market_klines_for_report
        orig_eval_bt = daily_report.evaluate_backtest_and_attribution
        try:
            daily_report.REPORTS_DIR = self.reports_dir
            daily_report.fetch_market_klines_for_report = lambda rows: {}
            daily_report.evaluate_backtest_and_attribution = (
                lambda rows, arts, kmap, report_date="": {
                    "total_evaluated": 0,
                    "total_hits": 0,
                    "overall_hit_rate": 0.0,
                    "long_hit_rate": 0.0,
                    "long_hits": 0,
                    "long_total": 0,
                    "short_hit_rate": 0.0,
                    "short_hits": 0,
                    "short_total": 0,
                    "avg_long_ret_5d": 0.0,
                    "avg_short_ret_5d": 0.0,
                    "long_short_spread_5d": 0.0,
                    "us_hit_rate": 0.0,
                    "cn_hit_rate": 0.0,
                    "mkt_leader_note": "",
                    "records": [],
                    "nav_dates": [],
                    "long_nav": [],
                    "short_nav": [],
                    "ls_nav": [],
                }
            )
            rep_path = daily_report.render_daily_report(
                {
                    "macro_synthesis": {"executive_summary": "summary"},
                    "core_articles": [
                        {
                            "zh_title": "测试文章",
                            "link": "javascript:alert(9)",
                            "us_impact": {"direction": "中性", "score": 5},
                            "cn_impact": {"direction": "中性", "score": 5},
                        }
                    ],
                    "digest_articles": [
                        {"title": "简讯", "link": "javascript:alert(10)"}
                    ],
                },
                "2026-11-18",
            )
            rep_html = rep_path.read_text(encoding="utf-8")
            self.assertFalse(
                'href="javascript:' in rep_html.lower(),
                "javascript: scheme link rendered in daily report",
            )
        finally:
            daily_report.REPORTS_DIR = orig_rep_dir
            daily_report.fetch_market_klines_for_report = orig_fetch_k
            daily_report.evaluate_backtest_and_attribution = orig_eval_bt

        # 5. set_report_ack must reject XSS / non-date / unknown report_id
        with self.assertRaises(ValueError):
            ack_server.set_report_ack("</script><script>alert(1)</script>", True)
        with self.assertRaises(ValueError):
            ack_server.set_report_ack("2099-01-01", True)

    def test_alert2_cors_csrf_host_and_body_size_confinement(self):
        # Wildcard CORS must never be emitted to untrusted external origins
        status, hdrs, _ = self._req(
            "GET",
            "/api/ack-state",
            headers={"Origin": "https://evil.example"},
        )
        self.assertNotEqual(hdrs.get("access-control-allow-origin"), "*")
        self.assertNotEqual(hdrs.get("access-control-allow-origin"), "https://evil.example")

        # State-mutating GET /api/ack-via-get must be removed
        status_get_ack, _, _ = self._req(
            "GET", "/api/ack-via-get?report_id=2026-11-18&acked=1"
        )
        self.assertIn(status_get_ack, (404, 405))
        state_after = ack_server.load_ack_state()
        self.assertFalse(state_after["reports"]["2026-11-18"]["acked"])

        # Cross-origin POST /api/ack from evil Origin must be rejected (403)
        status_csrf, _, _ = self._req(
            "POST",
            "/api/ack",
            body=b'{"report_id":"2026-11-18","acked":true}',
            headers={
                "Origin": "https://evil.example",
                "Content-Type": "application/json",
            },
        )
        self.assertEqual(status_csrf, 403)

        # Forged Host header (DNS rebinding) must be rejected (403)
        status_rebind, _, _ = self._req(
            "GET",
            "/api/ack-state",
            headers={"Host": "attacker-rebind.example"},
        )
        self.assertEqual(status_rebind, 403)

        # Oversized POST body (> 64 KiB) must be rejected (413)
        oversized = b'{"report_id":"2026-11-18","note":"' + (b"A" * 70_000) + b'"}'
        status_big, _, _ = self._req(
            "POST",
            "/api/ack",
            body=oversized,
            headers={"Content-Type": "application/json"},
        )
        self.assertEqual(status_big, 413)

        # Legitimate local POST /api/ack with application/json succeeds
        status_ok, hdrs_ok, _ = self._req(
            "POST",
            "/api/ack",
            body=b'{"report_id":"2026-11-18","acked":true}',
            headers={
                "Origin": f"http://127.0.0.1:{self.port}",
                "Content-Type": "application/json",
            },
        )
        self.assertEqual(status_ok, 200)
        self.assertEqual(
            hdrs_ok.get("access-control-allow-origin"),
            f"http://127.0.0.1:{self.port}",
        )

    def test_alert3_sibling_prefix_traversal_and_regex_backslash_safety(self):
        # Sibling directory `reports_backup` starts with `str(REPORTS_DIR)` when lacking boundary check!
        status, _, body = self._req("GET", "/reports/..%2Freports_backup%2Fsecret.txt")
        self.assertIn(status, (403, 404))
        self.assertNotIn(b"TOP_SECRET", body)

        status_win, _, body_win = self._req("GET", "/reports/..%5Creports_backup%5Csecret.txt")
        self.assertIn(status_win, (403, 404))
        self.assertNotIn(b"TOP_SECRET", body_win)

        # Backslashes in note must not trigger re.sub group-reference error or corrupt snapshot
        entry = ack_server.set_report_ack("2026-11-18", True, note=r"path\1\g<0>\99")
        self.assertTrue(entry["acked"])
        patched = self.report_html.read_text(encoding="utf-8")
        self.assertIn('"acked": true', patched)


if __name__ == "__main__":
    unittest.main()
