# -*- coding: utf-8 -*-
"""
Main CLI entrypoint for The Economist Daily Market Impact Analysis System.
Supports:
  --mode analyze      (08:00 AM scheduled task: fetch + checkpointed Gemini 4 Argon analysis + HTML archive)
  --mode open         (09:00 AM scheduled task: ensure today's report exists + start ACK server + open all un-ACKed reports)
  --mode both         (Run analyze then open)
  --mode backfill-3d  (Analyze & archive the last 3 days of reports and open in browser)
"""

import argparse
import subprocess
import sys
import time
import webbrowser
from datetime import datetime, timedelta
from pathlib import Path

from ack_server import (
    ensure_ack_server_running,
    get_unacked_reports,
    load_ack_state,
    record_report_opened,
)
from analyzer import run_checkpointed_analysis
from config import LOGS_DIR, REPORTS_DIR
from fetcher import (
    enrich_single_article,
    fetch_all_rss_articles,
    fetch_world_in_brief,
    filter_articles_by_window,
)
from renderer import render_archive_index, render_daily_report


def make_logger(report_date: str):
    log_file = LOGS_DIR / f"daily_{report_date}.log"

    def _log(msg: str):
        ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        line = f"[{ts}] {msg}"
        print(line, flush=True)
        try:
            with open(log_file, "a", encoding="utf-8") as f:
                f.write(line + "\n")
        except Exception:
            pass

    return _log


def open_url_in_browser(url: str, log_fn=print) -> None:
    """Open URL in default Windows browser reliably."""
    log_fn(f"[Browser] Opening: {url}")
    try:
        if sys.platform == "win32":
            subprocess.Popen(
                ["cmd.exe", "/c", "start", "", url],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return
    except Exception:
        pass
    webbrowser.open_new_tab(url)


def execute_single_report(
    report_date: str,
    days_window: int = 3,
    max_core: int = 24,
    batch_size: int = 6,
    pre_fetched: tuple = None,
    custom_articles: list = None,
    custom_window_desc: str = None,
    log_fn=print,
) -> Path:
    """Run fetch, enrichment, checkpointed Copilot analysis, and HTML rendering for a report_date."""
    log_fn(f"=== Starting Economist Analysis Pipeline for {report_date} (window={days_window}d) ===")

    if pre_fetched is not None:
        all_articles, max_dt = pre_fetched
    else:
        all_articles, max_dt = fetch_all_rss_articles(log_fn=log_fn)

    world_in_brief = fetch_world_in_brief(log_fn=log_fn)

    if custom_articles is not None:
        window_articles = custom_articles
    else:
        window_articles = filter_articles_by_window(
            all_articles, max_dt=max_dt, days=days_window, target_date=report_date
        )

    if not custom_window_desc:
        if window_articles:
            min_day = min(a.get("pub_day", report_date) for a in window_articles)
            max_day = max(a.get("pub_day", report_date) for a in window_articles)
            custom_window_desc = f"{min_day} 至 {max_day} (过去 {days_window} 天最新刊发)"
        else:
            custom_window_desc = f"截至 {report_date} (最近 {days_window} 天)"

    log_fn(f"[Pipeline] Selected {len(window_articles)} articles in window [{custom_window_desc}]")

    analysis_data = run_checkpointed_analysis(
        run_key=report_date,
        articles=window_articles,
        world_in_brief=world_in_brief,
        window_desc=custom_window_desc,
        max_core_articles=max_core,
        batch_size=batch_size,
        enrich_fn=lambda a: enrich_single_article(a, log_fn=log_fn),
        log_fn=log_fn,
    )

    html_path = render_daily_report(analysis_data, report_date=report_date)
    log_fn(f"[Renderer] ✓ Archived HTML report generated: {html_path}")
    return html_path


def open_pending_and_today_reports(target_date: str, days_window: int = 3, max_core: int = 24, log_fn=print) -> list[str]:
    """
    09:00 AM Open workflow:
      1. Ensure target_date's report exists (if 08:00 AM job didn't complete, resume from checkpoint and finish it).
      2. Ensure local ACK server (127.0.0.1:4288) is running.
      3. Find all un-ACKed reports in ack_state.json (including historical un-ACKed reports + today's report).
      4. Open all un-ACKed reports in browser tabs (or if today's is already ACKed, still open today's report).
    """
    state = load_ack_state()
    today_entry = state.get("reports", {}).get(target_date)
    today_html = (REPORTS_DIR / today_entry["rel_path"]) if (today_entry and today_entry.get("rel_path")) else None

    if not today_html or not today_html.exists():
        log_fn(f"[OpenMode] Report for {target_date} not found yet; running/resuming analysis first...")
        execute_single_report(
            report_date=target_date,
            days_window=days_window,
            max_core=max_core,
            log_fn=log_fn,
        )

    base_url = ensure_ack_server_running()
    render_archive_index()

    unacked = get_unacked_reports()
    state = load_ack_state()
    today_entry = state.get("reports", {}).get(target_date)

    to_open = list(unacked)
    if today_entry and all(r.get("report_id") != target_date for r in to_open):
        to_open.append(today_entry)

    opened_urls = []
    log_fn(f"[OpenMode] Found {len(unacked)} un-ACKed report(s). Opening {len(to_open)} tab(s) in browser...")
    for rep in to_open:
        rid = rep["report_id"]
        rel_path = rep["rel_path"]
        url = f"{base_url}/reports/{rel_path}"
        open_url_in_browser(url, log_fn=log_fn)
        record_report_opened(rid)
        opened_urls.append(url)
        time.sleep(0.4)

    return opened_urls


def run_backfill_3d(target_date: str, max_core: int = 22, batch_size: int = 6, log_fn=print):
    """
    Generate archived reports for the past 3 days (Day-2, Day-1, and Today's 3-Day Master Overview)
    so the YYYY/MM/DD archive and un-ACKed multi-report cascade are immediately populated and verifiable.
    """
    dt_end = datetime.strptime(target_date, "%Y-%m-%d")
    d_minus_2 = (dt_end - timedelta(days=2)).strftime("%Y-%m-%d")
    d_minus_1 = (dt_end - timedelta(days=1)).strftime("%Y-%m-%d")

    all_articles, max_dt = fetch_all_rss_articles(log_fn=log_fn)
    # Recent window articles (covering the weekly issue + weekend updates)
    recent_pool = filter_articles_by_window(all_articles, max_dt=max_dt, days=6)
    log_fn(f"[Backfill-3D] Total recent Economist articles in pool: {len(recent_pool)}")

    # Step 1: Run comprehensive deep analysis on the full recent pool for Today (target_date)
    # Because analyzer caches every article analysis in analyzed_cache.json + checkpoint,
    # running Today's comprehensive 3D report first analyzes all core articles in checkpointed batches!
    execute_single_report(
        report_date=target_date,
        days_window=4,
        max_core=max_core,
        batch_size=batch_size,
        pre_fetched=(all_articles, max_dt),
        custom_articles=recent_pool,
        custom_window_desc=f"最近 3~4 日《经济学人》全量最新文章深度总览 (截至 {target_date})",
        log_fn=log_fn,
    )

    # Step 2: Generate Day-2 Archive Report (focused on earlier half of the window: US Midterms & Macro Shock)
    d2_articles = [
        a for a in recent_pool if a.get("pub_day", "") <= "2026-11-05" and ("united-states" in a.get("sections", []) or "leaders" in a.get("sections", []) or "briefing" in a.get("sections", []))
    ][:14]
    if len(d2_articles) >= 4:
        execute_single_report(
            report_date=d_minus_2,
            days_window=2,
            max_core=10,
            batch_size=batch_size,
            pre_fetched=(all_articles, max_dt),
            custom_articles=d2_articles,
            custom_window_desc=f"{d_minus_2} 归档专题：美国中期选举冲击波与全球政经重估",
            log_fn=log_fn,
        )

    # Step 3: Generate Day-1 Archive Report (focused on latest weekend dispatches + Business/Tech/Commodities)
    d1_articles = [
        a for a in recent_pool if a.get("pub_day", "") >= "2026-11-06" or ("business" in a.get("sections", []) or "science-and-technology" in a.get("sections", []))
    ][:14]
    if len(d1_articles) >= 4:
        execute_single_report(
            report_date=d_minus_1,
            days_window=2,
            max_core=10,
            batch_size=batch_size,
            pre_fetched=(all_articles, max_dt),
            custom_articles=d1_articles,
            custom_window_desc=f"{d_minus_1} 归档专题：全球科技AI、大宗商品与商业并购追踪",
            log_fn=log_fn,
        )

    # Re-render today's report so its sidebar archive list includes d_minus_2 and d_minus_1
    execute_single_report(
        report_date=target_date,
        days_window=4,
        max_core=max_core,
        batch_size=batch_size,
        pre_fetched=(all_articles, max_dt),
        custom_articles=recent_pool,
        custom_window_desc=f"最近 3 日《经济学人》全量最新文章深度总览 (截至 {target_date})",
        log_fn=log_fn,
    )


def main():
    parser = argparse.ArgumentParser(description="Economist Daily A-Share / US-Stock Deep Analysis Runner")
    parser.add_argument(
        "--mode",
        choices=["analyze", "open", "both", "backfill-3d"],
        default="both",
        help="Execution mode: analyze (08:00), open (09:00), both, or backfill-3d",
    )
    parser.add_argument("--date", default=datetime.now().strftime("%Y-%m-%d"), help="Target report date YYYY-MM-DD")
    parser.add_argument("--days", type=int, default=3, help="Lookback window in days (default: 3)")
    parser.add_argument("--max-core", type=int, default=22, help="Max core articles for deep breakdown")
    parser.add_argument("--batch-size", type=int, default=6, help="Articles per Copilot CLI checkpoint batch")
    args = parser.parse_args()

    log_fn = make_logger(args.date)

    if args.mode == "analyze":
        execute_single_report(
            report_date=args.date,
            days_window=args.days,
            max_core=args.max_core,
            batch_size=args.batch_size,
            log_fn=log_fn,
        )
    elif args.mode == "open":
        open_pending_and_today_reports(
            target_date=args.date,
            days_window=args.days,
            max_core=args.max_core,
            log_fn=log_fn,
        )
    elif args.mode == "both":
        execute_single_report(
            report_date=args.date,
            days_window=args.days,
            max_core=args.max_core,
            batch_size=args.batch_size,
            log_fn=log_fn,
        )
        open_pending_and_today_reports(
            target_date=args.date,
            days_window=args.days,
            max_core=args.max_core,
            log_fn=log_fn,
        )
    elif args.mode == "backfill-3d":
        run_backfill_3d(
            target_date=args.date,
            max_core=args.max_core,
            batch_size=args.batch_size,
            log_fn=log_fn,
        )
        open_pending_and_today_reports(
            target_date=args.date,
            days_window=args.days,
            max_core=args.max_core,
            log_fn=log_fn,
        )


if __name__ == "__main__":
    main()
