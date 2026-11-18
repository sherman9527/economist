# -*- coding: utf-8 -*-
"""
Goldman Sachs (GS GIR) & Morgan Stanley (MS Research) Visual Chart-First Equity Report Renderer.
Powered by Apache ECharts (local /reports/assets/echarts.min.js) + Real 40-Day Market OHLCV Data.

Visual Components:
  1. Main Interactive K-Line Terminal (Candlestick + MA5/MA20 + Volume Bar + Economist Catalyst Marker)
  2. Multi-Asset 40-Day Normalized Return Line Chart (LineChart)
  3. GS/MS 6-Sector Conviction & Real 5D Return Grouped Column Chart (ColumnChart)
  4. Top Conviction OW vs UW Diverging Bar Chart (BarChart)
  5. Cross-Market 4-Quadrant Impact Scatter Plot (ScatterChart)
  6. Per-Article 2-Column Visual Note Cards:
       - Left: 3 Crisp One-Line Key Takeaways + Stock Scorecard Table with Inline SVG Sparklines & Real 1D/5D%
       - Right: Embedded Interactive ECharts (Switchable between Stock K-Line & Multi-Stock Return Column Chart)
       - Bottom Drawer: Collapsible Deep Logic & Original Excerpt (Zero Wall-of-Text Clutter)
"""

import html
import json
import re
from datetime import datetime
from pathlib import Path

from ack_server import load_ack_state, register_report
from backtester import evaluate_backtest_and_attribution
from config import ACK_SERVER_HOST, ACK_SERVER_PORT, DATA_DIR, REPORTS_DIR
from gs_ms_tables import (
    build_sector_scorecard_rows,
    extract_all_structured_stocks,
    localize_zh_text,
    parse_ticker_entry,
    split_to_bullets,
)
from market_data import fetch_market_klines_for_report, resolve_qt_code


def _esc(val: any) -> str:
    if val is None:
        return ""
    return html.escape(str(val))


def _safe_script_json(val: any) -> str:
    """Serialize data to JSON safe for embedding directly inside HTML <script> tags."""
    raw = json.dumps(val, ensure_ascii=False)
    return (
        raw.replace("<", "\\u003c")
        .replace(">", "\\u003e")
        .replace("&", "\\u0026")
        .replace("\u2028", "\\u2028")
        .replace("\u2029", "\\u2029")
    )


def _safe_http_url(url: any) -> str:
    """Allow only http:// and https:// external URLs; block javascript:/data:/file: schemes."""
    s = str(url or "").strip()
    lower = s.lower()
    if lower.startswith("https://") or lower.startswith("http://"):
        return s
    return "#"


def _clean_sec_labels_zh(labels: list[str]) -> str:
    cleaned = [re.sub(r"\s*\([A-Za-z0-9 &_-]+\)", "", str(x)).strip() for x in (labels or [])]
    return " / ".join([c for c in cleaned if c])


def _short_sentence(text: str, max_len: int = 54) -> str:
    """Trim a long sentence to a crisp, scannable 1-line Chinese phrase."""
    s = localize_zh_text(re.sub(r"\s+", " ", (text or "").strip()))
    if not s:
        return ""
    # Take first clause if very long
    clauses = re.split(r"[；。！？]", s)
    first = clauses[0].strip()
    if len(first) < 18 and len(clauses) > 1:
        first = f"{first}，{clauses[1].strip()}"
    if len(first) > max_len:
        return first[: max_len - 1] + "…"
    return first


def _build_svg_sparkline(closes: list[float], width: int = 84, height: int = 26) -> str:
    """Generate a crisp inline SVG sparkline with area fill for 20-day stock price trend."""
    if not closes or len(closes) < 3:
        return '<span style="color:#94a3b8;font-size:11px;">--</span>'
    pts = closes[-20:]
    mn, mx = min(pts), max(pts)
    rng = (mx - mn) if mx != mn else 1.0
    coords = []
    for i, val in enumerate(pts):
        x = round(2 + (i / (len(pts) - 1)) * (width - 4), 1)
        y = round(height - 3 - ((val - mn) / rng) * (height - 6), 1)
        coords.append(f"{x},{y}")
    is_up = pts[-1] >= pts[0]
    stroke = "#16a34a" if is_up else "#e11d48"
    fill = "rgba(22,163,74,0.14)" if is_up else "rgba(225,29,72,0.14)"
    poly_pts = " ".join(coords)
    area_pts = f"2,{height-2} {poly_pts} {width-2},{height-2}"
    return (
        f'<svg width="{width}" height="{height}" viewBox="0 0 {width} {height}" style="vertical-align:middle;overflow:visible;">'
        f'<polygon points="{area_pts}" fill="{fill}" />'
        f'<polyline points="{poly_pts}" fill="none" stroke="{stroke}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />'
        f"</svg>"
    )


def _fmt_pct_badge(val: float) -> str:
    if val is None:
        return '<span class="pct-pill pct-flat">--</span>'
    cls = "pct-up" if val > 0.05 else ("pct-down" if val < -0.05 else "pct-flat")
    sign = "+" if val > 0 else ""
    return f'<span class="pct-pill {cls}">{sign}{val:.2f}%</span>'


def _dir_to_gs_badge(direction: str, score: int = 5) -> tuple[str, str]:
    d = str(direction or "")
    if "强烈利好" in d:
        return f"强烈利好 ({score}分)", "gs-rating-cbuy"
    if "利好" in d:
        return f"温和利好 ({score}分)", "gs-rating-ow"
    if "强烈利空" in d:
        return f"强烈利空 ({score}分)", "gs-rating-uw-strong"
    if "利空" in d:
        return f"温和利空 ({score}分)", "gs-rating-uw"
    return f"中性分化 ({score}分)", "gs-rating-ew"


def _dir_filter_tag(direction: str) -> str:
    d = str(direction or "")
    if "利好" in d:
        return "bull"
    if "利空" in d:
        return "bear"
    return "neutral"


