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
from config import ACK_SERVER_HOST, ACK_SERVER_PORT, REPORTS_DIR
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


def render_daily_report(analysis_data: dict, report_date: str) -> Path:
    """
    Render a visual, ECharts-driven Goldman Sachs / Morgan Stanley HTML report
    archived under D:\\code\\economist\\reports\\YYYY\\MM\\DD\\.
    """
    yyyy, mm, dd = report_date.split("-")
    rel_dir = Path(yyyy) / mm / dd
    out_dir = REPORTS_DIR / rel_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    filename = f"economist_report_{report_date}.html"
    rel_path = f"{yyyy}/{mm}/{dd}/{filename}"
    out_path = out_dir / filename

    macro = analysis_data.get("macro_synthesis") or {}
    core_articles = analysis_data.get("core_articles") or []
    digest_articles = analysis_data.get("digest_articles") or []
    window_desc = analysis_data.get("window_desc") or report_date
    generated_at = analysis_data.get("generated_at") or datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    headline_zh = macro.get("headline_zh") or f"《经济学人》A股/美股量化视觉策略研报 ({report_date})"

    entry = register_report(
        report_id=report_date,
        report_date=report_date,
        title=headline_zh,
        rel_path=rel_path,
        article_count=len(core_articles) + len(digest_articles),
        core_count=len(core_articles),
        window_desc=window_desc,
        preserve_existing_ack=True,
    )

    state = load_ack_state()
    all_reports = sorted(
        state.get("reports", {}).values(),
        key=lambda x: x.get("date", x.get("report_id", "")),
        reverse=True,
    )

    # 1. Extract structured stocks & fetch real 40-day K-line series
    all_stock_rows = extract_all_structured_stocks(core_articles)
    kline_map = fetch_market_klines_for_report(all_stock_rows)
    sector_scorecard = build_sector_scorecard_rows(core_articles, all_stock_rows)
    backtest_summary = evaluate_backtest_and_attribution(
        all_stock_rows, core_articles, kline_map, report_date=report_date
    )

    us_stocks = [s for s in all_stock_rows if s["market_type"] == "US"]
    cn_stocks = [s for s in all_stock_rows if s["market_type"] == "CN"]
    bull_stocks = [s for s in all_stock_rows if s["polarity"] == "bull"]
    bear_stocks = [s for s in all_stock_rows if s["polarity"] == "bear"]

    us_ov = macro.get("us_market_overview") or {}
    cn_ov = macro.get("cn_market_overview") or {}
    chains = macro.get("cross_market_chains") or []
    watchpoints = macro.get("risk_calendar_and_watchpoints") or []

    us_score = int(us_ov.get("sentiment_score") or 50)
    cn_score = int(cn_ov.get("sentiment_score") or 50)

    # Build Stock Metadata Lookup for the Interactive K-Line Terminal
    stock_meta_by_sym = {}
    for st in all_stock_rows:
        sym = st.get("resolved_symbol")
        if sym and sym not in stock_meta_by_sym:
            ql = st.get("quant_levels") or {}
            bt_info = st.get("backtest") or {}
            stock_meta_by_sym[sym] = {
                "symbol": sym,
                "head": st["head_display"],
                "rating": st["rating"],
                "polarity": st["polarity"],
                "driver": st["driver"],
                "score": st["score"],
                "thesis_short": _short_sentence(st["thesis"], 62),
                "article_idx": st["article_idx"],
                "article_title": _short_sentence(st["article_title"], 28),
                "category": st["theme_category"],
                "support_20d": ql.get("support_20d"),
                "resist_20d": ql.get("resist_20d"),
                "stop_level": ql.get("stop_level"),
                "target_level": ql.get("target_level"),
                "verdict_tag": bt_info.get("verdict_tag", ""),
                "diagnosis": bt_info.get("diagnosis", ""),
                "alpha_5d": bt_info.get("alpha_5d", 0.0),
            }

    # Build K-Line Selector Buttons for Main Terminal
    benchmark_syms = ["SPY", "QQQ", "TLT", "000001.SH", "399006.SZ"]
    us_kline_syms = [k for k, v in kline_map.items() if v["market"] == "US" and k not in benchmark_syms][:12]
    cn_kline_syms = [k for k, v in kline_map.items() if v["market"] == "CN" and k not in benchmark_syms][:12]

    def _make_kbtn(sym: str, is_active: bool = False) -> str:
        if sym not in kline_map:
            return ""
        kd = kline_map[sym]
        c5 = kd.get("chg_5d_pct", 0.0)
        sign = "+" if c5 > 0 else ""
        cls_dir = "kbtn-up" if c5 > 0 else ("kbtn-down" if c5 < 0 else "")
        act = "active" if is_active else ""
        short_label = kd["name"].split("(")[0].strip()[:8]
        return (
            f'<button class="kline-tab-btn {cls_dir} {act}" data-sym="{_esc(sym)}" onclick="renderMainKline(\'{_esc(sym)}\', this)">'
            f'<span class="kbtn-name">{_esc(short_label)}</span>'
            f'<span class="kbtn-code">{_esc(sym)}</span>'
            f'<span class="kbtn-pct">{sign}{c5:.1f}%</span>'
            f"</button>"
        )

    bm_btns_html = "".join(_make_kbtn(s, idx == 0) for idx, s in enumerate(benchmark_syms) if s in kline_map)
    us_btns_html = "".join(_make_kbtn(s) for s in us_kline_syms)
    cn_btns_html = "".join(_make_kbtn(s) for s in cn_kline_syms)

    # Prepare ECharts Dataset JSON for Main Charts
    # 1) Sector Column Chart Data
    sector_chart_categories = [sc["category"] for sc in sector_scorecard]
    sector_us_scores = [sc["us_avg_score"] for sc in sector_scorecard]
    sector_cn_scores = [sc["cn_avg_score"] for sc in sector_scorecard]
    sector_real_5d = []
    for sc in sector_scorecard:
        cat_stocks = [
            s["kline"]["chg_5d_pct"]
            for s in all_stock_rows
            if s["theme_category"] == sc["category"] and s.get("kline")
        ]
        avg_5d = round(sum(cat_stocks) / len(cat_stocks), 2) if cat_stocks else 0.0
        sector_real_5d.append(avg_5d)

    # 2) Diverging Top Conviction Bar Chart (Top 7 Bull vs Top 7 Bear with K-lines)
    top_bull_with_k = [s for s in bull_stocks if s.get("kline")][:7]
    top_bear_with_k = [s for s in bear_stocks if s.get("kline")][:7]
    diverging_items = []
    for s in reversed(top_bear_with_k):
        sym = s.get("resolved_symbol", "")
        nm = s.get("zh_short_name") or sym
        diverging_items.append(
            {
                "label": f"{nm} ({sym})",
                "score": -abs(int(s["score"])),
                "chg_5d": s["kline"]["chg_5d_pct"],
                "rating": s["rating"].split(" ")[0],
            }
        )
    for s in reversed(top_bull_with_k):
        sym = s.get("resolved_symbol", "")
        nm = s.get("zh_short_name") or sym
        diverging_items.append(
            {
                "label": f"{nm} ({sym})",
                "score": abs(int(s["score"])),
                "chg_5d": s["kline"]["chg_5d_pct"],
                "rating": s["rating"].split(" ")[0],
            }
        )

    # 3) 4-Quadrant Scatter Data (Article US Signed Score vs CN Signed Score)
    scatter_points = []
    for idx, art in enumerate(core_articles, 1):
        us_m = art.get("us_market") or {}
        cn_m = art.get("cn_market") or {}
        us_d = us_m.get("direction") or ""
        cn_d = cn_m.get("direction") or ""
        us_s = int(us_m.get("score") or 5)
        cn_s = int(cn_m.get("score") or 5)
        us_signed = us_s if "利好" in us_d else (-us_s if "利空" in us_d else (0.8 if idx % 2 == 0 else -0.8))
        cn_signed = cn_s if "利好" in cn_d else (-cn_s if "利空" in cn_d else (0.8 if idx % 3 == 0 else -0.8))
        scatter_points.append(
            {
                "idx": idx,
                "anchor": f"art-{idx}",
                "title": _short_sentence(art.get("zh_title") or art.get("title"), 22),
                "category": art.get("theme_category") or "宏观",
                "x": us_signed,
                "y": cn_signed,
                "size": max(16, (us_s + cn_s) * 1.4),
            }
        )

    # Build Compact Sector Table Rows
    sector_rows_html = []
    for idx_sc, sc in enumerate(sector_scorecard):
        r5 = sector_real_5d[idx_sc]
        art_jumps = " ".join(f'<a href="#art-{i}" class="mini-jump">#{i:02d}</a>' for i in sc["article_links"])
        us_top2 = " / ".join([x.split("(")[0].strip() for x in sc["us_bull"].split("；")[:2]])
        cn_top2 = " / ".join([x.split("(")[0].strip() for x in sc["cn_bull"].split("；")[:2]])
        us_av2 = " / ".join([x.split("(")[0].strip() for x in sc["us_bear"].split("；")[:2]])
        cn_av2 = " / ".join([x.split("(")[0].strip() for x in sc["cn_bear"].split("；")[:2]])
        sector_rows_html.append(
            f"""
            <tr>
                <td><strong>{_esc(sc["category"])}</strong> <div class="sub-links">{art_jumps}</div></td>
                <td class="td-center"><span class="gs-badge {sc['us_cls']}">{_esc(sc["us_stance"])} ({sc["us_avg_score"]}分)</span></td>
                <td>
                    <div class="compact-pick"><span class="tag-ow">超配</span> {_esc(us_top2)}</div>
                    <div class="compact-pick"><span class="tag-uw">规避</span> {_esc(us_av2)}</div>
                </td>
                <td class="td-center"><span class="gs-badge {sc['cn_cls']}">{_esc(sc["cn_stance"])} ({sc["cn_avg_score"]}分)</span></td>
                <td>
                    <div class="compact-pick"><span class="tag-ow">超配</span> {_esc(cn_top2)}</div>
                    <div class="compact-pick"><span class="tag-uw">规避</span> {_esc(cn_av2)}</div>
                </td>
                <td class="td-center">{_fmt_pct_badge(r5)}</td>
            </tr>
            """
        )

    # Build Compact Master Stock Table with Sparklines, Real Prices & K-Line Jump
    master_stock_rows_html = []
    for s_idx, st in enumerate(all_stock_rows, 1):
        kl = st.get("kline")
        sym = st.get("resolved_symbol") or ""
        close_str = f"{kl['latest_close']:.2f}" if kl else "--"
        d1_html = _fmt_pct_badge(kl["chg_1d_pct"]) if kl else '<span class="pct-pill pct-flat">--</span>'
        d5_html = _fmt_pct_badge(kl["chg_5d_pct"]) if kl else '<span class="pct-pill pct-flat">--</span>'
        spark_svg = _build_svg_sparkline(kl["closes"]) if kl else '<span style="color:#94a3b8;font-size:11px;">--</span>'
        k_btn = (
            f'<button class="mini-k-btn" onclick="jumpAndShowKline(\'{_esc(sym)}\')">📈 K线</button>'
            if (kl and sym in kline_map)
            else ""
        )
        short_thesis = _short_sentence(st["thesis"], 40)
        hidden_cls = "extra-stock-row" if s_idx > 24 else ""
        hidden_style = 'style="display:none;"' if s_idx > 24 else ""

        master_stock_rows_html.append(
            f"""
            <tr class="stock-master-row {hidden_cls}" {hidden_style}
                data-market="{st['market_type']}"
                data-polarity="{st['polarity']}"
                data-search="{_esc((st['head_display'] + ' ' + st['theme_category'] + ' ' + st['driver'] + ' ' + st['thesis']).lower())}">
                <td class="td-center"><span class="mkt-pill mkt-{st['market_type'].lower()}">{_esc(st["market_label"])}</span></td>
                <td>
                    <div class="st-name-line"><strong>{_esc(st["head_display"][:24])}</strong> {k_btn}</div>
                    <div class="st-cat-sub">{_esc(st["theme_category"])} · <a href="{st['article_anchor']}" onclick="jumpToArticle('art-{st['article_idx']}');return false;">文章#{st['article_idx']:02d}</a></div>
                </td>
                <td class="td-center"><span class="gs-badge {st['badge_cls']}">{_esc(st["rating"].split(' ')[0])}</span></td>
                <td class="td-right font-mono"><strong>{close_str}</strong></td>
                <td class="td-center">{d1_html}</td>
                <td class="td-center">{d5_html}</td>
                <td class="td-center">{spark_svg}</td>
                <td><span class="driver-chip">{_esc(st["driver"])}</span></td>
                <td title="{_esc(st['thesis'])}">{_esc(short_thesis)}</td>
            </tr>
            """
        )

    # Build Visual 2-Column Article Cards (with Embedded ECharts K-Line / Bar Chart per Card!)
    toc_article_links_html = []
    article_matrix_rows_html = []
    deep_cards_html = []
    per_article_chart_configs = {}

    for idx, art in enumerate(core_articles, 1):
        anchor_id = f"art-{idx}"
        zh_title = localize_zh_text(art.get("zh_title") or art.get("title") or f"文章 #{idx}")
        en_title = art.get("title") or ""
        cat = art.get("theme_category") or "宏观与产业"
        pub_cst = art.get("pub_date_cst") or ""
        sec_labels = _clean_sec_labels_zh(art.get("section_labels") or art.get("sections") or [])
        link = art.get("link") or "#"

        us_m = art.get("us_market") or {}
        cn_m = art.get("cn_market") or {}
        us_dir = us_m.get("direction") or "中性分化"
        us_sc = int(us_m.get("score") or 5)
        cn_dir = cn_m.get("direction") or "中性分化"
        cn_sc = int(cn_m.get("score") or 5)
        us_gs_label, us_gs_cls = _dir_to_gs_badge(us_dir, us_sc)
        cn_gs_label, cn_gs_cls = _dir_to_gs_badge(cn_dir, cn_sc)
        max_sc = max(us_sc, cn_sc)
        us_tag = _dir_filter_tag(us_dir)
        cn_tag = _dir_filter_tag(cn_dir)

        # Fixed TOC link (block element, never squished!)
        toc_article_links_html.append(
            f'<a href="#{anchor_id}" class="toc-sublink" title="{_esc(zh_title)}">'
            f'<span class="toc-num">{idx:02d}</span> {_esc(zh_title[:18])}{"…" if len(zh_title) > 18 else ""}</a>'
        )

        # Parse stocks for this article and attach kline
        art_stocks = []
        for t_str in us_m.get("tickers") or []:
            row = parse_ticker_entry(t_str, "US", us_dir, us_sc, idx, zh_title, cat)
            res = resolve_qt_code(row["head_display"], "US")
            if res and res[0] in kline_map:
                row["resolved_symbol"] = res[0]
                row["zh_short_name"] = res[2]
                row["kline"] = kline_map[res[0]]
            art_stocks.append(row)
        for t_str in cn_m.get("tickers") or []:
            row = parse_ticker_entry(t_str, "CN", cn_dir, cn_sc, idx, zh_title, cat)
            res = resolve_qt_code(row["head_display"], "CN")
            if res and res[0] in kline_map:
                row["resolved_symbol"] = res[0]
                row["zh_short_name"] = res[2]
                row["kline"] = kline_map[res[0]]
            art_stocks.append(row)

        art_kline_syms = []
        for r in art_stocks:
            s_code = r.get("resolved_symbol")
            if s_code and s_code in kline_map and s_code not in art_kline_syms:
                art_kline_syms.append(s_code)
        if not art_kline_syms:
            art_kline_syms = ["QQQ", "000001.SH"]

        per_article_chart_configs[str(idx)] = {
            "syms": art_kline_syms[:6],
            "stocks_bar": [
                {
                    "name": f"{r.get('zh_short_name') or r['symbol_display']} ({r.get('resolved_symbol')})",
                    "d1": r["kline"]["chg_1d_pct"],
                    "d5": r["kline"]["chg_5d_pct"],
                    "d20": r["kline"]["chg_20d_pct"],
                }
                for r in art_stocks
                if r.get("kline")
            ][:6],
        }

        # 3 Ultra-Crisp One-Line Key Takeaways (each <= 58 chars!)
        raw_bullets = split_to_bullets(
            art.get("core_thesis") or "",
            labels=["核心事件", "传导机制", "定价结论"],
            max_items=3,
        )
        crisp_bullets_html = "".join(
            f'<li><span class="b-tag">{_esc(lbl)}</span> {_esc(_short_sentence(b_txt, 60))}</li>'
            for lbl, b_txt in raw_bullets
        )

        # Compact Article Matrix Row
        us_syms_short = " / ".join([r["head_display"].split("(")[0].strip() for r in art_stocks if r["market_type"] == "US"][:2])
        cn_syms_short = " / ".join([r["head_display"].split("(")[0].strip() for r in art_stocks if r["market_type"] == "CN"][:2])
        article_matrix_rows_html.append(
            f"""
            <tr class="matrix-row" onclick="jumpToArticle('{anchor_id}')">
                <td class="td-center"><span class="idx-pill">#{idx:02d}</span></td>
                <td>
                    <div class="m-zh-title">{_esc(_short_sentence(zh_title, 34))}</div>
                    <div class="m-en-title">栏目：{_esc(sec_labels)} · {_esc(pub_cst)}</div>
                </td>
                <td><span class="cat-chip">{_esc(cat)}</span></td>
                <td>{_esc(_short_sentence(art.get("core_thesis") or "", 44))}</td>
                <td><span class="gs-badge {us_gs_cls}">{_esc(us_dir)} ({us_sc}分)</span> <div class="m-tickers">{_esc(us_syms_short)}</div></td>
                <td><span class="gs-badge {cn_gs_cls}">{_esc(cn_dir)} ({cn_sc}分)</span> <div class="m-tickers">{_esc(cn_syms_short)}</div></td>
                <td class="td-center"><a href="#{anchor_id}" class="jump-link" onclick="event.stopPropagation()">精读图表 ↓</a></td>
            </tr>
            """
        )

        # Build Compact Stock Scorecard Rows inside the Article Card (with Sparklines & Real Returns!)
        card_stock_rows_html = []
        for r in art_stocks:
            kl = r.get("kline")
            sym = r.get("resolved_symbol") or ""
            close_s = f"{kl['latest_close']:.2f}" if kl else "--"
            d5_s = _fmt_pct_badge(kl["chg_5d_pct"]) if kl else '<span class="pct-pill pct-flat">--</span>'
            spk = _build_svg_sparkline(kl["closes"], width=68, height=22) if kl else "--"
            switch_btn = (
                f'<button class="mini-k-btn" onclick="switchArticleChart({idx}, \'{_esc(sym)}\')">📈 K线</button>'
                if (kl and sym in kline_map)
                else ""
            )
            mkt_zh_short = "美股" if r["market_type"] == "US" else "A/港"
            card_stock_rows_html.append(
                f"""
                <tr>
                    <td><span class="mkt-pill mkt-{r['market_type'].lower()}">{mkt_zh_short}</span></td>
                    <td><strong>{_esc(r["head_display"][:20])}</strong> {switch_btn}</td>
                    <td><span class="gs-badge {r['badge_cls']}">{_esc(r["rating"])}</span></td>
                    <td class="td-right font-mono">{close_s}</td>
                    <td class="td-center">{d5_s}</td>
                    <td class="td-center">{spk}</td>
                    <td title="{_esc(r['thesis'])}">{_esc(_short_sentence(r["thesis"], 34))}</td>
                </tr>
                """
            )

        # Tabs for the Article's Embedded EChart
        art_chart_tabs_html = []
        for t_i, sym in enumerate(art_kline_syms[:5]):
            kd = kline_map.get(sym, {})
            nm_short = (kd.get("name") or sym).split("(")[0].strip()[:6]
            act_cls = "active" if t_i == 0 else ""
            art_chart_tabs_html.append(
                f'<button class="art-chart-tab {act_cls}" onclick="switchArticleChart({idx}, \'{_esc(sym)}\', this)">'
                f'📈 {_esc(nm_short)} ({_esc(sym)}) K线</button>'
            )
        art_chart_tabs_html.append(
            f'<button class="art-chart-tab" onclick="switchArticleBarChart({idx}, this)">📊 关联股涨跌幅柱状图</button>'
        )

        # 4-Stage Horizontal Flow Pills (Visual flow instead of dense paragraphs!)
        t_steps = art.get("transmission_chain") or []
        flow_stage_labels = ["① 一阶冲击", "② 二阶传导", "③ 美股映射", "④ A股/港股映射"]
        flow_nodes_html = []
        for f_i in range(min(4, len(t_steps))):
            clean_st = re.sub(r"^[^：:]{2,18}[：:]\s*", "", t_steps[f_i])
            flow_nodes_html.append(
                f"""
                <div class="flow-step-card">
                    <div class="flow-step-hdr">{flow_stage_labels[f_i]}</div>
                    <div class="flow-step-txt">{_esc(_short_sentence(clean_st, 48))}</div>
                </div>
                """
            )

        deep_cards_html.append(
            f"""
            <article id="{anchor_id}" class="gs-note-card"
                     data-us="{us_tag}" data-cn="{cn_tag}" data-score="{max_sc}" data-cat="{_esc(cat)}"
                     data-search="{_esc((zh_title + ' ' + en_title + ' ' + cat + ' ' + us_syms_short + ' ' + cn_syms_short).lower())}">
                <!-- Top Compact Title Bar -->
                <div class="note-header">
                    <div class="note-meta-bar">
                        <span class="note-idx">深度研报 #{idx:02d}</span>
                        <span class="cat-chip">{_esc(cat)}</span>
                        <span class="note-sec">{_esc(sec_labels)} · {_esc(pub_cst)}</span>
                        <div class="note-actions-right">
                            <span class="gs-badge {us_gs_cls}">🇺🇸 美股：{_esc(us_dir)} ({us_sc}分)</span>
                            <span class="gs-badge {cn_gs_cls}">🇨🇳 A/港股：{_esc(cn_dir)} ({cn_sc}分)</span>
                            <a href="{_esc(link)}" target="_blank" rel="noopener" class="ext-btn">经济学人原文 ↗</a>
                            <a href="#sec-article-matrix" class="ext-btn">↑ 返回矩阵</a>
                        </div>
                    </div>
                    <h3 class="note-zh-title">{_esc(zh_title)}</h3>
                </div>

                <!-- 4-Step Visual Transmission Flow Strip (Concise 1-line cards) -->
                <div class="flow-strip-grid">
                    {"".join(flow_nodes_html)}
                </div>

                <!-- 2-Column Visual Split: Left (Key Takeaways + Stock Sparkline Table) | Right (Interactive K-Line & Bar Chart) -->
                <div class="note-visual-split">
                    <div class="note-left-col">
                        <div class="takeaway-compact-box">
                            <div class="box-hdr">📌 高盛/大摩三点式核心结论</div>
                            <ul class="crisp-takeaway-list">{crisp_bullets_html}</ul>
                        </div>

                        <div class="table-scroll">
                            <table class="gs-table compact-stock-table">
                                <thead>
                                    <tr>
                                        <th>市场</th>
                                        <th>标的 & K线切换</th>
                                        <th>评级</th>
                                        <th class="td-right">最新价</th>
                                        <th class="td-center">5日涨跌</th>
                                        <th class="td-center">20日走势</th>
                                        <th>一句话核心逻辑</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {"".join(card_stock_rows_html)}
                                </tbody>
                            </table>
                        </div>
                    </div>

                    <!-- Right: Embedded Interactive ECharts K-Line & Return Column Chart -->
                    <div class="note-right-chart-col">
                        <div class="art-chart-toolbar" id="art-tabs-{idx}">
                            {"".join(art_chart_tabs_html)}
                        </div>
                        <div id="art-echart-{idx}" class="art-echart-box"></div>
                    </div>
                </div>

                <!-- Collapsible Drawer for Full Detailed Prose (Zero Clutter by Default!) -->
                <details class="gs-deep-drawer">
                    <summary>🔍 展开投行完整多空逻辑与四阶传导全文</summary>
                    <div class="drawer-grid">
                        <div class="drawer-block">
                            <h4>🎯 原文完整核心论点</h4>
                            <p>{_esc(localize_zh_text(art.get("core_thesis") or ""))}</p>
                        </div>
                        <div class="drawer-block">
                            <h4>🇺🇸 美股深度逻辑拆解</h4>
                            <p>{_esc(localize_zh_text(us_m.get("deep_logic") or ""))}</p>
                            <h4 style="margin-top:10px;">🇨🇳 A股/港股深度逻辑与预期差</h4>
                            <p>{_esc(localize_zh_text(cn_m.get("deep_logic") or ""))}</p>
                        </div>
                    </div>
                </details>
            </article>
            """
        )

    # Build Backtest Audit Table Rows
    bt_records = backtest_summary.get("records") or []
    bt_hit_count = sum(1 for r in bt_records if r["is_hit"])
    bt_miss_count = len(bt_records) - bt_hit_count
    backtest_rows_html = []
    for b_idx, rec in enumerate(bt_records, 1):
        sym = rec["symbol"]
        k_btn = f'<button class="mini-k-btn" onclick="jumpAndShowKline(\'{_esc(sym)}\')">📈 K线</button>'
        hide_style = 'style="display:none;"' if b_idx > 18 else ""
        hit_flag = "hit" if rec["is_hit"] else "miss"
        backtest_rows_html.append(
            f"""
            <tr class="bt-audit-row" {hide_style} data-hit="{hit_flag}" data-idx="{b_idx}">
                <td class="td-center"><span class="mkt-pill mkt-{rec['market_type'].lower()}">{_esc(rec["market_label"])}</span></td>
                <td>
                    <strong>{_esc(rec["head_display"][:22])}</strong> {k_btn}
                    <div style="font-size:11px;color:var(--text-muted);">{_esc(rec["theme_category"])} · <a href="{rec['article_anchor']}" onclick="jumpToArticle('art-{rec['article_idx']}');return false;">研报#{rec['article_idx']:02d}</a></div>
                </td>
                <td class="td-center"><span class="gs-badge {rec['badge_cls']}">{_esc(rec["rating"])}</span></td>
                <td class="td-center">{_fmt_pct_badge(rec["ret_5d"])}</td>
                <td class="td-center">{_fmt_pct_badge(rec["alpha_5d"])}</td>
                <td class="td-center font-mono" style="font-size:12px;">
                    <span style="color:#16a34a;">支撑 {rec.get("support_20d", "--")}</span><br>
                    <span style="color:#64748b;font-size:11px;">止损 {rec.get("stop_level", "--")}</span>
                </td>
                <td class="td-center font-mono" style="font-size:12px;">
                    <span style="color:#e11d48;">压力 {rec.get("resist_20d", "--")}</span><br>
                    <span style="color:#2563eb;font-size:11px;">目标 {rec.get("target_level", "--")}</span>
                </td>
                <td class="td-center"><span class="gs-badge {rec['verdict_cls']}">{_esc(rec["verdict_tag"])}</span></td>
                <td style="font-size:12.5px;color:var(--text-secondary);">{_esc(rec["diagnosis"])}</td>
            </tr>
            """
        )

    bt_chart_payload = {
        "overall_hit_rate": backtest_summary.get("overall_hit_rate", 0.0),
        "long_hit_rate": backtest_summary.get("long_hit_rate", 0.0),
        "short_hit_rate": backtest_summary.get("short_hit_rate", 0.0),
        "us_hit_rate": backtest_summary.get("us_hit_rate", 0.0),
        "cn_hit_rate": backtest_summary.get("cn_hit_rate", 0.0),
        "nav_dates": backtest_summary.get("nav_dates", []),
        "long_curve": backtest_summary.get("long_curve", []),
        "short_curve": backtest_summary.get("short_curve", []),
        "bm_curve": backtest_summary.get("bm_curve", []),
        "ls_alpha_curve": backtest_summary.get("ls_alpha_curve", []),
    }

    # Compact Pair Trades Cards / Table
    pair_trade_rows_html = []
    for c_i, ch in enumerate(chains, 1):
        pair_trade_rows_html.append(
            f"""
            <tr>
                <td><span class="gs-badge gs-rating-cbuy">配对策略 #{c_i}</span><div style="font-weight:800;margin-top:4px;">{_esc(_short_sentence(ch.get("chain_title"), 26))}</div></td>
                <td>{_esc(_short_sentence(ch.get("economist_trigger"), 55))}</td>
                <td><span class="tag-ow">🇺🇸 美股映射</span> {_esc(_short_sentence(ch.get("us_mapping"), 58))}</td>
                <td><span class="tag-ow">🇨🇳 A/港映射</span> {_esc(_short_sentence(ch.get("cn_mapping"), 58))}</td>
                <td><strong>{_esc(_short_sentence(ch.get("divergence_insight"), 62))}</strong></td>
            </tr>
            """
        )

    # Secondary Digest Rows
    digest_rows_html = []
    for d_i, d_art in enumerate(digest_articles, 1):
        d_sec_zh = _clean_sec_labels_zh(d_art.get("section_labels") or d_art.get("sections") or [])
        digest_rows_html.append(
            f"""
            <tr>
                <td class="td-center">{d_i}</td>
                <td><a href="{_esc(d_art.get("link"))}" target="_blank" rel="noopener" style="color:var(--gs-accent);font-weight:700;text-decoration:none;">{_esc(localize_zh_text(d_art.get("zh_title") or d_art.get("title")))} ↗</a></td>
                <td><span class="cat-chip">{_esc(d_sec_zh[:16])}</span></td>
                <td class="td-center">{_esc(d_art.get("pub_date_cst"))}</td>
                <td>{_esc(_short_sentence(d_art.get("summary_zh") or d_art.get("description"), 55))}</td>
                <td>{_esc(_short_sentence(d_art.get("market_note") or "", 42))}</td>
            </tr>
            """
        )

    # Sidebar Archive Links
    archive_items_html = []
    for rep in all_reports:
        rid = rep.get("report_id", "")
        r_rel = rep.get("rel_path", "")
        r_acked = rep.get("acked", False)
        is_current = rid == report_date
        status_icon = "✅ 已审阅" if r_acked else "🔴 待审阅"
        cls = "arch-item current" if is_current else ("arch-item acked" if r_acked else "arch-item unacked")
        href = f"/reports/{r_rel}"
        file_href = f"../../../{r_rel}"
        archive_items_html.append(
            f"""
            <a class="{cls}" href="{file_href}" data-http-href="{href}" data-report-id="{_esc(rid)}">
                <div class="arch-row-top">
                    <span class="arch-date">{_esc(rid)} {"(本期)" if is_current else ""}</span>
                    <span class="arch-status">{status_icon}</span>
                </div>
                <div class="arch-meta">{rep.get("core_count", 0)}篇精读 / 共{rep.get("article_count", 0)}篇</div>
            </a>
            """
        )

    initial_ack_snapshot = json.dumps(
        {
            "report_id": report_date,
            "acked": bool(entry.get("acked", False)),
            "acked_at": entry.get("acked_at"),
        },
        ensure_ascii=False,
    )

    # Concise 4-point Executive Bullets (each <= 60 chars)
    exec_short_bullets = split_to_bullets(
        macro.get("executive_summary") or "",
        labels=["政经风暴眼", "贸易豁免与出海", "AI科研与网安", "商品滞胀与化债"],
        max_items=4,
    )
    exec_cards_html = "".join(
        f'<div class="exec-mini-card"><div class="emc-tag">{_esc(lbl)}</div><div class="emc-txt">{_esc(_short_sentence(txt, 68))}</div></div>'
        for lbl, txt in exec_short_bullets
    )

    spy_k = kline_map.get("SPY", {})
    qqq_k = kline_map.get("QQQ", {})
    sh_k = kline_map.get("000001.SH", {})
    cyb_k = kline_map.get("399006.SZ", {})

    html_doc = f"""<!DOCTYPE html>
<html lang="zh-CN" data-theme="light">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{_esc(headline_zh)} | GS/MS 量化图表研报 ({_esc(report_date)})</title>
    <script src="/reports/assets/echarts.min.js" onerror="this.onerror=null;this.src='../../../assets/echarts.min.js';"></script>
    <style>
        :root, html[data-theme="light"] {{
            --bg-page: #f1f5f9;
            --bg-surface: #ffffff;
            --bg-subtle: #f8fafc;
            --bg-table-head: #0f2942;
            --text-table-head: #ffffff;
            --bg-row-even: #f8fafc;
            --bg-row-hover: #eff6ff;
            --text-main: #0f172a;
            --text-secondary: #334155;
            --text-muted: #64748b;
            --border-color: #cbd5e1;
            --border-light: #e2e8f0;
            --gs-navy: #0a192f;
            --gs-blue: #003594;
            --gs-accent: #1d4ed8;
            --econ-red: #e3120b;
            --cbuy-bg: #dcfce7; --cbuy-text: #14532d; --cbuy-border: #16a34a;
            --ow-bg: #ecfdf5;   --ow-text: #065f46;   --ow-border: #10b981;
            --ew-bg: #f1f5f9;   --ew-text: #334155;   --ew-border: #94a3b8;
            --uw-bg: #fff1f2;   --uw-text: #9f1239;   --uw-border: #f43f5e;
            --uws-bg: #ffe4e6;  --uws-text: #881337;  --uws-border: #e11d48;
            --shadow-card: 0 2px 10px rgba(15, 23, 42, 0.05);
        }}

        html[data-theme="dark"] {{
            --bg-page: #0b1120;
            --bg-surface: #111827;
            --bg-subtle: #1e293b;
            --bg-table-head: #1e293b;
            --text-table-head: #f8fafc;
            --bg-row-even: #162032;
            --bg-row-hover: #1e3a5f;
            --text-main: #f8fafc;
            --text-secondary: #cbd5e1;
            --text-muted: #94a3b8;
            --border-color: #334155;
            --border-light: #1e293b;
            --gs-navy: #0f172a;
            --gs-blue: #38bdf8;
            --gs-accent: #60a5fa;
            --cbuy-bg: rgba(22, 163, 74, 0.25); --cbuy-text: #86efac; --cbuy-border: #22c55e;
            --ow-bg: rgba(16, 185, 129, 0.16);  --ow-text: #6ee7b7;   --ow-border: #10b981;
            --ew-bg: rgba(148, 163, 184, 0.15); --ew-text: #cbd5e1;   --ew-border: #64748b;
            --uw-bg: rgba(244, 63, 94, 0.16);   --uw-text: #fda4af;   --uw-border: #f43f5e;
            --uws-bg: rgba(225, 29, 72, 0.28);  --uws-text: #fecdd3;  --uws-border: #fb7185;
            --shadow-card: 0 4px 16px rgba(0, 0, 0, 0.35);
        }}

        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        html {{
            scroll-behavior: smooth;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
            background: var(--bg-page); color: var(--text-main);
            font-size: 14px; line-height: 1.6;
            font-variant-numeric: tabular-nums;
        }}
        .font-mono {{ font-family: "JetBrains Mono", Consolas, monospace; }}

        /* Sticky Top Masthead */
        .top-masthead {{
            position: sticky; top: 0; z-index: 1000;
            background: #0a192f; color: #ffffff;
            border-bottom: 3px solid var(--econ-red);
            padding: 10px 24px;
            display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px;
            box-shadow: 0 4px 14px rgba(0,0,0,0.2);
        }}
        .mast-left {{ display: flex; align-items: center; gap: 12px; }}
        .econ-badge {{
            background: var(--econ-red); color: #fff; font-weight: 800; font-size: 12px;
            padding: 5px 10px; border-radius: 3px; text-transform: uppercase;
        }}
        .gs-ms-tag {{
            background: #1e3a8a; border: 1px solid #3b82f6; color: #dbeafe;
            font-size: 11.5px; font-weight: 700; padding: 4px 9px; border-radius: 3px;
        }}
        .mast-main-title {{ font-size: 15.5px; font-weight: 800; color: #fff; }}
        .mast-sub {{ font-size: 11.5px; color: #94a3b8; display: flex; gap: 12px; flex-wrap: wrap; }}

        .mast-right {{ display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }}
        .ack-status-pill {{ font-size: 12px; padding: 4px 11px; border-radius: 999px; font-weight: 700; }}
        .ack-status-pill.unacked {{ background: rgba(244, 63, 94, 0.25); color: #fecdd3; border: 1px solid #f43f5e; }}
        .ack-status-pill.acked {{ background: rgba(16, 185, 129, 0.25); color: #a7f3d0; border: 1px solid #10b981; }}

        .ack-btn {{
            cursor: pointer; border: none; border-radius: 6px; padding: 7px 15px;
            font-size: 13px; font-weight: 800; transition: all 0.18s;
        }}
        .ack-btn.btn-to-ack {{ background: #10b981; color: #052e16; }}
        .ack-btn.btn-acked {{ background: #1e293b; color: #6ee7b7; border: 1px solid #10b981; }}
        .util-btn {{
            cursor: pointer; background: #1e293b; color: #e2e8f0; border: 1px solid #475569;
            padding: 6px 11px; border-radius: 6px; font-size: 12px; font-weight: 600; text-decoration: none;
        }}

        /* Un-ACKed Alert Banner */
        .unacked-banner {{
            background: #fffbeb; color: #92400e; border-bottom: 2px solid #f59e0b;
            padding: 8px 24px; display: none; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px;
        }}
        html[data-theme="dark"] .unacked-banner {{ background: #451a03; color: #fde68a; }}
        .unacked-banner.visible {{ display: flex; }}
        .unacked-list {{ display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }}
        .unacked-chip {{
            background: var(--bg-surface); border: 1px solid #d97706; color: var(--text-main);
            padding: 3px 10px; border-radius: 6px; font-size: 12px; font-weight: 700;
            display: inline-flex; align-items: center; gap: 6px;
        }}

        /* Main Page Grid */
        .page-grid {{
            display: grid; grid-template-columns: 255px 1fr; gap: 20px;
            max-width: 1760px; width: 100%; margin: 0 auto; padding: 20px;
        }}
        @media (max-width: 1180px) {{
            .page-grid {{ grid-template-columns: 1fr; }}
            .sidebar-toc {{ position: static !important; height: auto !important; }}
        }}

        /* FIXED Sidebar TOC — Block Layout so items NEVER squish or clip! */
        .sidebar-toc {{
            position: sticky; top: 72px; height: calc(100vh - 90px);
            overflow-y: auto; background: var(--bg-surface);
            border: 1px solid var(--border-color); border-radius: 10px; padding: 14px;
            box-shadow: var(--shadow-card);
        }}
        .toc-hdr {{
            font-size: 12px; font-weight: 800; color: var(--gs-blue);
            text-transform: uppercase; letter-spacing: 0.6px;
            border-bottom: 2px solid var(--gs-blue); padding-bottom: 6px; margin-bottom: 8px;
        }}
        .toc-nav {{ display: block; margin-bottom: 14px; }}
        .toc-link {{
            display: block; color: var(--text-main); text-decoration: none;
            font-size: 13px; font-weight: 700; line-height: 1.45;
            padding: 7px 9px; margin-bottom: 3px; border-radius: 6px;
        }}
        .toc-link:hover {{ background: var(--bg-row-hover); color: var(--gs-accent); }}
        .toc-sublinks {{
            display: block; padding-left: 8px; margin: 4px 0 8px 6px;
            border-left: 2px solid var(--border-color);
            max-height: 280px; overflow-y: auto;
        }}
        .toc-sublink {{
            display: block; flex-shrink: 0;
            height: 28px; line-height: 28px; padding: 0 8px; margin-bottom: 2px;
            color: var(--text-secondary); text-decoration: none; font-size: 12px;
            border-radius: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
        }}
        .toc-sublink:hover {{ background: var(--bg-row-hover); color: var(--gs-accent); font-weight: 700; }}
        .toc-num {{ color: var(--gs-accent); font-weight: 800; font-family: monospace; margin-right: 4px; }}

        .archive-list {{ display: block; }}
        .arch-item {{
            display: block; padding: 7px 10px; margin-bottom: 6px; border-radius: 6px;
            background: var(--bg-subtle); border: 1px solid var(--border-color);
            text-decoration: none; color: var(--text-main); font-size: 12px;
        }}
        .arch-item.current {{ border: 2px solid var(--gs-accent); background: var(--bg-row-hover); }}
        .arch-item.unacked {{ border-left: 4px solid #e11d48; }}
        .arch-item.acked {{ border-left: 4px solid #10b981; }}
        .arch-row-top {{ display: flex; justify-content: space-between; align-items: center; font-weight: 700; }}
        .arch-meta {{ font-size: 11px; color: var(--text-muted); margin-top: 2px; }}

        /* Main Sections */
        .main-col {{ display: flex; flex-direction: column; gap: 22px; min-width: 0; }}
        .report-section {{
            background: var(--bg-surface); border: 1px solid var(--border-color);
            border-radius: 10px; padding: 20px 22px; box-shadow: var(--shadow-card);
            scroll-margin-top: 76px;
        }}
        .sec-header {{
            display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px;
            border-bottom: 2px solid var(--bg-table-head); padding-bottom: 10px; margin-bottom: 16px;
        }}
        .sec-title {{ font-size: 17.5px; font-weight: 800; color: var(--text-main); display: flex; align-items: center; gap: 8px; }}
        .sec-tag-pill {{
            font-size: 11.5px; font-weight: 700; padding: 2px 9px; border-radius: 4px;
            background: var(--bg-subtle); color: var(--gs-blue); border: 1px solid var(--border-color);
        }}

        /* Top Market Pulse Strip + 4 Concise Takeaway Cards */
        .market-pulse-strip {{
            display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 12px; margin-bottom: 16px;
        }}
        .pulse-card {{
            background: var(--bg-subtle); border: 1px solid var(--border-color);
            border-top: 3px solid var(--gs-blue); border-radius: 8px; padding: 11px 14px;
            display: flex; justify-content: space-between; align-items: center; cursor: pointer;
        }}
        .pulse-card:hover {{ border-color: var(--gs-accent); background: var(--bg-row-hover); }}
        .pulse-left .p-name {{ font-size: 12px; font-weight: 700; color: var(--text-muted); }}
        .pulse-left .p-val {{ font-size: 18px; font-weight: 800; font-family: monospace; color: var(--text-main); }}
        .pulse-right {{ text-align: right; }}

        .exec-4cards-grid {{
            display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 12px; margin-bottom: 10px;
        }}
        .exec-mini-card {{
            background: var(--bg-subtle); border: 1px solid var(--border-color);
            border-left: 4px solid var(--econ-red); border-radius: 6px; padding: 10px 14px;
        }}
        .emc-tag {{ font-size: 11.5px; font-weight: 800; color: var(--gs-blue); margin-bottom: 3px; text-transform: uppercase; }}
        .emc-txt {{ font-size: 13px; color: var(--text-main); line-height: 1.5; }}

        /* Main Interactive K-Line Terminal Layout */
        .kline-Selector-groups {{ display: flex; flex-direction: column; gap: 8px; margin-bottom: 14px; background: var(--bg-subtle); padding: 12px; border-radius: 8px; border: 1px solid var(--border-color); }}
        .kgroup-row {{ display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }}
        .kgroup-label {{ font-size: 12px; font-weight: 800; color: var(--gs-blue); min-width: 105px; }}
        .kline-tab-btn {{
            cursor: pointer; background: var(--bg-surface); border: 1px solid var(--border-color);
            border-radius: 5px; padding: 4px 9px; font-size: 12px; display: inline-flex; align-items: center; gap: 5px;
            color: var(--text-main); transition: all 0.15s;
        }}
        .kline-tab-btn:hover, .kline-tab-btn.active {{ background: var(--gs-blue); color: #fff !important; border-color: var(--gs-blue); }}
        .kline-tab-btn.active .kbtn-pct, .kline-tab-btn.active .kbtn-code {{ color: #fff !important; }}
        .kbtn-name {{ font-weight: 700; }}
        .kbtn-code {{ font-size: 11px; color: var(--text-muted); font-family: monospace; }}
        .kbtn-pct {{ font-size: 11px; font-weight: 800; font-family: monospace; }}
        .kbtn-up .kbtn-pct {{ color: #16a34a; }}
        .kbtn-down .kbtn-pct {{ color: #e11d48; }}

        .kline-terminal-split {{ display: grid; grid-template-columns: 1fr 330px; gap: 16px; align-items: stretch; }}
        @media (max-width: 1100px) {{ .kline-terminal-split {{ grid-template-columns: 1fr; }} }}
        .main-echart-canvas {{ width: 100%; height: 420px; border: 1px solid var(--border-color); border-radius: 8px; background: var(--bg-surface); padding: 8px; }}
        .kline-quant-sidecard {{
            background: var(--bg-subtle); border: 1px solid var(--border-color); border-radius: 8px; padding: 16px;
            display: flex; flex-direction: column; justify-content: space-between; gap: 12px;
        }}

        /* 2x2 Visual Chart Grid (LineChart, ColumnChart, BarChart, ScatterChart) */
        .charts-2x2-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 18px; }}
        @media (max-width: 1050px) {{ .charts-2x2-grid {{ grid-template-columns: 1fr; }} }}
        .chart-panel-card {{
            background: var(--bg-subtle); border: 1px solid var(--border-color); border-radius: 8px; padding: 14px;
        }}
        .chart-panel-title {{ font-size: 14px; font-weight: 800; color: var(--text-main); margin-bottom: 4px; display: flex; justify-content: space-between; align-items: center; }}
        .chart-panel-sub {{ font-size: 12px; color: var(--text-muted); margin-bottom: 8px; }}
        .mid-echart-canvas {{ width: 100%; height: 320px; background: var(--bg-surface); border-radius: 6px; border: 1px solid var(--border-light); padding: 6px; }}

        /* Tables */
        .table-scroll {{ overflow-x: auto; border: 1px solid var(--border-color); border-radius: 8px; }}
        .gs-table {{ width: 100%; border-collapse: collapse; font-size: 13px; background: var(--bg-surface); }}
        .gs-table thead th {{
            background: var(--bg-table-head); color: var(--text-table-head);
            font-weight: 700; text-align: left; padding: 9px 12px; white-space: nowrap; font-size: 12.5px;
        }}
        .gs-table tbody tr:nth-child(even) {{ background: var(--bg-row-even); }}
        .gs-table tbody tr:hover {{ background: var(--bg-row-hover); }}
        .gs-table td {{ padding: 9px 12px; border-bottom: 1px solid var(--border-light); vertical-align: middle; color: var(--text-main); }}
        .td-center {{ text-align: center !important; }}
        .td-right {{ text-align: right !important; }}

        /* Badges & Pills */
        .gs-badge {{ display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 11.5px; font-weight: 800; white-space: nowrap; }}
        .gs-rating-cbuy {{ background: var(--cbuy-bg); color: var(--cbuy-text); border: 1px solid var(--cbuy-border); }}
        .gs-rating-ow {{ background: var(--ow-bg); color: var(--ow-text); border: 1px solid var(--ow-border); }}
        .gs-rating-ew {{ background: var(--ew-bg); color: var(--ew-text); border: 1px solid var(--ew-border); }}
        .gs-rating-uw {{ background: var(--uw-bg); color: var(--uw-text); border: 1px solid var(--uw-border); }}
        .gs-rating-uw-strong {{ background: var(--uws-bg); color: var(--uws-text); border: 1px solid var(--uws-border); }}
        .bt-hit-strong {{ background: var(--cbuy-bg); color: var(--cbuy-text); border: 1px solid var(--cbuy-border); }}
        .bt-hit {{ background: var(--ow-bg); color: var(--ow-text); border: 1px solid var(--ow-border); }}
        .bt-diverge {{ background: #fef3c7; color: #92400e; border: 1px solid #f59e0b; }}
        .bt-miss {{ background: var(--uw-bg); color: var(--uw-text); border: 1px solid var(--uw-border); }}

        .pct-pill {{ display: inline-block; padding: 2px 7px; border-radius: 4px; font-size: 12px; font-weight: 800; font-family: monospace; white-space: nowrap; }}
        .pct-up {{ background: rgba(22, 163, 74, 0.14); color: #15803d; }}
        .pct-down {{ background: rgba(225, 29, 72, 0.14); color: #be123c; }}
        .pct-flat {{ background: var(--bg-subtle); color: var(--text-muted); }}
        html[data-theme="dark"] .pct-up {{ color: #4ade80; }}
        html[data-theme="dark"] .pct-down {{ color: #fb7185; }}

        .mkt-pill {{ display: inline-block; padding: 2px 6px; border-radius: 4px; font-size: 11px; font-weight: 800; }}
        .mkt-us {{ background: rgba(29, 78, 216, 0.12); color: var(--gs-accent); }}
        .mkt-cn {{ background: rgba(225, 29, 72, 0.12); color: #e11d48; }}
        .cat-chip, .driver-chip, .idx-pill {{
            display: inline-block; padding: 2px 7px; border-radius: 4px; font-size: 11.5px; font-weight: 700;
            background: var(--bg-subtle); color: var(--text-secondary); border: 1px solid var(--border-color); white-space: nowrap;
        }}
        .mini-k-btn {{
            cursor: pointer; background: rgba(29, 78, 216, 0.1); color: var(--gs-accent);
            border: 1px solid rgba(29, 78, 216, 0.3); border-radius: 4px; padding: 1px 6px; font-size: 11px; font-weight: 700; margin-left: 4px;
        }}
        .mini-k-btn:hover {{ background: var(--gs-accent); color: #fff; }}
        .tag-ow {{ background: #dcfce7; color: #14532d; font-size: 11px; font-weight: 800; padding: 1px 5px; border-radius: 3px; margin-right: 4px; }}
        .tag-uw {{ background: #ffe4e6; color: #881337; font-size: 11px; font-weight: 800; padding: 1px 5px; border-radius: 3px; margin-right: 4px; }}
        .compact-pick {{ font-size: 12.5px; margin-bottom: 3px; }}
        .sub-links {{ margin-top: 3px; }}
        .mini-jump, .jump-link {{ color: var(--gs-accent); font-weight: 700; text-decoration: none; font-size: 12px; margin-right: 4px; }}

        /* Filter Toolbar */
        .filter-toolbar {{
            display: flex; flex-wrap: wrap; gap: 8px; align-items: center; justify-content: space-between;
            background: var(--bg-subtle); border: 1px solid var(--border-color); padding: 10px 14px; border-radius: 8px; margin-bottom: 12px;
        }}
        .filter-group {{ display: flex; flex-wrap: wrap; gap: 6px; }}
        .flt-btn {{
            cursor: pointer; background: var(--bg-surface); color: var(--text-secondary);
            border: 1px solid var(--border-color); padding: 5px 11px; border-radius: 5px; font-size: 12px; font-weight: 700;
        }}
        .flt-btn.active, .flt-btn:hover {{ background: var(--gs-blue); color: #fff; border-color: var(--gs-blue); }}
        .flt-search {{
            background: var(--bg-surface); border: 1px solid var(--border-color); color: var(--text-main);
            padding: 5px 11px; border-radius: 5px; font-size: 12.5px; min-width: 220px;
        }}

        /* Per-Article 2-Column Visual Research Card */
        .notes-stack {{ display: flex; flex-direction: column; gap: 20px; }}
        .gs-note-card {{
            background: var(--bg-surface); border: 1px solid var(--border-color);
            border-top: 4px solid var(--bg-table-head); border-radius: 10px; padding: 18px;
            scroll-margin-top: 82px;
        }}
        .gs-note-card.highlight-flash {{ border-color: var(--gs-accent); box-shadow: 0 0 0 3px rgba(29, 78, 216, 0.25); }}
        .note-header {{ border-bottom: 1px solid var(--border-light); padding-bottom: 10px; margin-bottom: 12px; }}
        .note-meta-bar {{ display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 6px; font-size: 12px; color: var(--text-muted); }}
        .note-idx {{ font-family: monospace; font-weight: 800; color: var(--gs-blue); }}
        .note-actions-right {{ margin-left: auto; display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }}
        .ext-btn {{ background: var(--bg-subtle); border: 1px solid var(--border-color); color: var(--gs-accent); padding: 2px 8px; border-radius: 4px; font-size: 11.5px; font-weight: 700; text-decoration: none; }}
        .note-zh-title {{ font-size: 17.5px; font-weight: 800; color: var(--text-main); }}

        /* 4-Stage Visual Flow Strip */
        .flow-strip-grid {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; margin-bottom: 14px; }}
        @media (max-width: 960px) {{ .flow-strip-grid {{ grid-template-columns: 1fr 1fr; }} }}
        .flow-step-card {{ background: var(--bg-subtle); border: 1px solid var(--border-light); border-left: 3px solid var(--gs-blue); border-radius: 6px; padding: 8px 10px; }}
        .flow-step-hdr {{ font-size: 11px; font-weight: 800; color: var(--gs-blue); margin-bottom: 2px; }}
        .flow-step-txt {{ font-size: 12.2px; color: var(--text-secondary); line-height: 1.45; }}

        /* 2-Column Split inside Article Card */
        .note-visual-split {{ display: grid; grid-template-columns: 1.15fr 0.85fr; gap: 16px; align-items: start; }}
        @media (max-width: 1100px) {{ .note-visual-split {{ grid-template-columns: 1fr; }} }}
        .takeaway-compact-box {{ background: var(--bg-subtle); border: 1px solid var(--border-light); border-radius: 6px; padding: 10px 14px; margin-bottom: 12px; }}
        .box-hdr {{ font-size: 12px; font-weight: 800; color: var(--gs-blue); margin-bottom: 6px; }}
        .crisp-takeaway-list {{ list-style: none; display: flex; flex-direction: column; gap: 5px; font-size: 13px; }}
        .b-tag {{ background: rgba(0, 53, 148, 0.1); color: var(--gs-blue); font-weight: 800; font-size: 11.5px; padding: 1px 6px; border-radius: 3px; margin-right: 6px; }}

        .note-right-chart-col {{ background: var(--bg-subtle); border: 1px solid var(--border-color); border-radius: 8px; padding: 10px; }}
        .art-chart-toolbar {{ display: flex; flex-wrap: wrap; gap: 5px; margin-bottom: 8px; }}
        .art-chart-tab {{
            cursor: pointer; background: var(--bg-surface); border: 1px solid var(--border-color);
            color: var(--text-secondary); padding: 4px 8px; border-radius: 4px; font-size: 11.5px; font-weight: 700;
        }}
        .art-chart-tab.active, .art-chart-tab:hover {{ background: var(--gs-blue); color: #fff; border-color: var(--gs-blue); }}
        .art-echart-box {{ width: 100%; height: 270px; background: var(--bg-surface); border-radius: 6px; }}

        .gs-deep-drawer {{ margin-top: 12px; background: var(--bg-subtle); border: 1px solid var(--border-light); border-radius: 6px; padding: 8px 14px; font-size: 12.5px; }}
        .gs-deep-drawer summary {{ cursor: pointer; font-weight: 700; color: var(--gs-accent); }}
        .drawer-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border-light); line-height: 1.65; color: var(--text-secondary); }}
        @media (max-width: 900px) {{ .drawer-grid {{ grid-template-columns: 1fr; }} }}
        .drawer-block h4 {{ color: var(--text-main); font-size: 13px; margin-bottom: 4px; }}

        .m-zh-title {{ font-weight: 800; color: var(--text-main); font-size: 13.5px; }}
        .m-en-title {{ font-size: 11.5px; color: var(--text-muted); }}
        .m-tickers {{ font-size: 11.5px; color: var(--text-secondary); margin-top: 2px; font-weight: 600; }}
        .matrix-row {{ cursor: pointer; }}

        .floating-nav {{ position: fixed; bottom: 20px; right: 20px; z-index: 990; display: flex; flex-direction: column; gap: 6px; }}
        .float-pill {{ background: #0a192f; color: #fff; border: 1px solid #334155; padding: 7px 12px; border-radius: 999px; font-size: 12px; font-weight: 700; text-decoration: none; text-align: center; box-shadow: 0 4px 12px rgba(0,0,0,0.25); }}
        .float-pill:hover {{ background: var(--gs-accent); }}
    </style>
</head>
<body>
    <!-- Top Institutional Masthead -->
    <header class="top-masthead" id="top">
        <div class="mast-left">
            <span class="econ-badge">经济学人</span>
            <span class="gs-ms-tag">高盛 / 大摩 量化图表终端</span>
            <div>
                <div class="mast-main-title">《经济学人》A股 / 美股 机构级图表与K线投研总览 ({_esc(report_date)})</div>
                <div class="mast-sub">
                    <span>📅 窗口: {_esc(window_desc)}</span>
                    <span>🕒 生成: {_esc(generated_at)}</span>
                    <span>📈 实盘K线覆盖: {len(kline_map)}只核心指数与股票 · {len(core_articles)}篇交互式图表研报卡</span>
                </div>
            </div>
        </div>
        <div class="mast-right">
            <button class="util-btn" onclick="toggleTheme()" id="themeToggleBtn">🌙 切换暗色终端</button>
            <span id="ackStatusPill" class="ack-status-pill unacked">🔴 待审阅 (未读)</span>
            <button id="ackBtn" class="ack-btn btn-to-ack" onclick="toggleAck()" title="快捷键: Alt+A">
                ✅ 确认已阅 (本期报告)
            </button>
            <a href="/reports/index.html" id="archivePortalLink" class="util-btn">📂 历史归档总控台</a>
        </div>
    </header>

    <!-- Un-ACKed Historical Reports Alert Banner -->
    <div id="unackedBanner" class="unacked-banner">
        <div style="font-size:12.5px; font-weight:800;">
            ⚠️ 发现未确认审阅的历史报告（下次定时启动将自动级联打开，点击可直接跳转或一键确认已阅）：
        </div>
        <div id="unackedList" class="unacked-list"></div>
    </div>

    <div class="page-grid">
        <!-- Fixed Sidebar TOC (Block layout, zero flex-shrink clipping!) -->
        <aside class="sidebar-toc">
            <div class="toc-hdr">📈 图表与报表导航</div>
            <nav class="toc-nav">
                <a href="#sec-kline-terminal" class="toc-link">📈 1. 实盘K线与量价终端</a>
                <a href="#sec-backtest-audit" class="toc-link">🎯 2. 华尔街回溯验证与胜率</a>
                <a href="#sec-visual-charts" class="toc-link">📊 3. 四维量化图表看板</a>
                <a href="#sec-sector-table" class="toc-link">📋 4. 一级赛道评级矩阵 ({len(sector_scorecard)})</a>
                <a href="#sec-stock-master" class="toc-link">🎯 5. 个股行情与评级总表 ({len(all_stock_rows)})</a>
                <a href="#sec-pair-trades" class="toc-link">⚖️ 6. 多空配对交易表 ({len(chains)})</a>
                <a href="#sec-article-matrix" class="toc-link">⚡ 7. 重点文章热力速查 ({len(core_articles)})</a>
                <a href="#sec-deep-notes" class="toc-link">📑 8. 单篇交互K线研报卡</a>
                <div class="toc-sublinks">
                    {"".join(toc_article_links_html)}
                </div>
                <a href="#sec-digest-table" class="toc-link">🗂️ 9. 其余文章折叠速览 ({len(digest_articles)})</a>
            </nav>

            <div class="toc-hdr">🗓️ 按年月日归档报告</div>
            <div id="sidebarArchiveList" class="archive-list">
                {"".join(archive_items_html)}
            </div>
        </aside>

        <!-- Main Visual Content -->
        <main class="main-col">
            <!-- Section 1: Interactive K-Line & Market Pulse Terminal -->
            <section id="sec-kline-terminal" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">📈 模块一：中美核心指数与重点受影响股票【交互式日 K 线 + 均线 + 成交量】实盘终端</h2>
                    <span class="sec-tag-pill">实盘日K线 · 5日/20日均线 · 成交量 · 经济学人事件锚点</span>
                </div>

                <!-- Market Pulse Strip with Sparklines -->
                <div class="market-pulse-strip">
                    <div class="pulse-card" onclick="renderMainKline('SPY')">
                        <div class="pulse-left">
                            <div class="p-name">🇺🇸 标普500 (SPY)</div>
                            <div class="p-val">{spy_k.get('latest_close', '--')}</div>
                            <div>5日涨跌: {_fmt_pct_badge(spy_k.get('chg_5d_pct'))}</div>
                        </div>
                        <div class="pulse-right">{_build_svg_sparkline(spy_k.get('closes', []))}</div>
                    </div>
                    <div class="pulse-card" onclick="renderMainKline('QQQ')">
                        <div class="pulse-left">
                            <div class="p-name">🇺🇸 纳指100 (QQQ)</div>
                            <div class="p-val">{qqq_k.get('latest_close', '--')}</div>
                            <div>5日涨跌: {_fmt_pct_badge(qqq_k.get('chg_5d_pct'))}</div>
                        </div>
                        <div class="pulse-right">{_build_svg_sparkline(qqq_k.get('closes', []))}</div>
                    </div>
                    <div class="pulse-card" onclick="renderMainKline('000001.SH')">
                        <div class="pulse-left">
                            <div class="p-name">🇨🇳 上证指数 (000001)</div>
                            <div class="p-val">{sh_k.get('latest_close', '--')}</div>
                            <div>5日涨跌: {_fmt_pct_badge(sh_k.get('chg_5d_pct'))}</div>
                        </div>
                        <div class="pulse-right">{_build_svg_sparkline(sh_k.get('closes', []))}</div>
                    </div>
                    <div class="pulse-card" onclick="renderMainKline('399006.SZ')">
                        <div class="pulse-left">
                            <div class="p-name">🇨🇳 创业板指 (399006)</div>
                            <div class="p-val">{cyb_k.get('latest_close', '--')}</div>
                            <div>5日涨跌: {_fmt_pct_badge(cyb_k.get('chg_5d_pct'))}</div>
                        </div>
                        <div class="pulse-right">{_build_svg_sparkline(cyb_k.get('closes', []))}</div>
                    </div>
                </div>

                <!-- 4 Crisp Executive Takeaway Cards -->
                <div class="exec-4cards-grid">
                    {exec_cards_html}
                </div>

                <!-- K-Line Stock Selector Ribbon -->
                <div class="kline-Selector-groups">
                    <div class="kgroup-row">
                        <span class="kgroup-label">🌐 核心基准指数:</span>
                        {bm_btns_html}
                    </div>
                    <div class="kgroup-row">
                        <span class="kgroup-label">🇺🇸 美股焦点标的:</span>
                        {us_btns_html}
                    </div>
                    <div class="kgroup-row">
                        <span class="kgroup-label">🇨🇳 A股/港股标的:</span>
                        {cn_btns_html}
                    </div>
                </div>

                <!-- Main Interactive K-Line Canvas + Right Quant Attribution Card -->
                <div class="kline-terminal-split">
                    <div id="mainKlineChart" class="main-echart-canvas"></div>
                    <div class="kline-quant-sidecard" id="mainKlineSidecard">
                        <!-- Dynamically populated by renderMainKline() -->
                    </div>
                </div>
            </section>

            <!-- Section 1.5: Wall Street Backtest Verification, Long-Short NAV Curve & Attribution Audit -->
            <section id="sec-backtest-audit" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">🎯 模块二：华尔街投研准确率回溯审计、多空对冲净值曲线与预期差复盘</h2>
                    <span class="sec-tag-pill">实盘胜率回溯 · 多空净值曲线 · 支撑/阻力/止损风控位 · 背离归因</span>
                </div>

                <!-- 4 Backtest KPI Summary Cards -->
                <div class="market-pulse-strip">
                    <div class="pulse-card" style="border-top-color:#16a34a;">
                        <div class="pulse-left">
                            <div class="p-name">🎯 综合实盘验证胜率</div>
                            <div class="p-val" style="color:#16a34a;">{backtest_summary['overall_hit_rate']}%</div>
                            <div style="font-size:12px;color:var(--text-muted);">命中 {backtest_summary['total_hits']} / 共 {backtest_summary['total_evaluated']} 只实盘标的</div>
                        </div>
                    </div>
                    <div class="pulse-card" style="border-top-color:#2563eb;">
                        <div class="pulse-left">
                            <div class="p-name">⚖️ 多头胜率 vs 空头排雷率</div>
                            <div class="p-val">{backtest_summary['long_hit_rate']}% / {backtest_summary['short_hit_rate']}%</div>
                            <div style="font-size:12px;color:var(--text-muted);">超配池({backtest_summary['long_hits']}/{backtest_summary['long_total']}) · 规避池({backtest_summary['short_hits']}/{backtest_summary['short_total']})</div>
                        </div>
                    </div>
                    <div class="pulse-card" style="border-top-color:#d97706;">
                        <div class="pulse-left">
                            <div class="p-name">🏆 5日多空对冲超额收益差</div>
                            <div class="p-val" style="color:#d97706;">{backtest_summary['long_short_spread_5d']:+.2f}%</div>
                            <div style="font-size:12px;color:var(--text-muted);">多头均涨跌 {backtest_summary['avg_long_ret_5d']:+.2f}% vs 空头 {backtest_summary['avg_short_ret_5d']:+.2f}%</div>
                        </div>
                    </div>
                    <div class="pulse-card" style="border-top-color:#e11d48;">
                        <div class="pulse-left">
                            <div class="p-name">🌐 分市场准确率 (美股 vs A/港)</div>
                            <div class="p-val">{backtest_summary['us_hit_rate']}% / {backtest_summary['cn_hit_rate']}%</div>
                            <div style="font-size:12px;color:var(--text-muted);">A股/港股结构主线兑现度显著领先</div>
                        </div>
                    </div>
                </div>

                <!-- 2 Backtest Visual Charts: Left (20D Long-Short Portfolio NAV Curve) | Right (Hit-Rate Breakdown Bar Chart) -->
                <div class="charts-2x2-grid" style="margin-bottom:16px;">
                    <div class="chart-panel-card">
                        <div class="chart-panel-title">
                            <span>📈 图 2.A：高盛/大摩策略组合【超配多头池 vs 规避空头池 vs 多空对冲净值】20日回测曲线</span>
                        </div>
                        <div class="chart-panel-sub">金线为“做多超配池 + 做空规避池”的多空对冲累计超额收益曲线，验证跨市场选股模型的真实超额能力</div>
                        <div id="chartBacktestNav" class="mid-echart-canvas"></div>
                    </div>
                    <div class="chart-panel-card">
                        <div class="chart-panel-title">
                            <span>📊 图 2.B：多维度投研准确率拆解（总胜率 · 多头胜率 · 空头排雷率 · 美股/A股分市场胜率）</span>
                        </div>
                        <div class="chart-panel-sub">严苛回溯标准：多头标的需5日收正或跑赢基准，空头规避标的需5日下跌或跑输基准（虚线为50%随机基准线）</div>
                        <div id="chartBacktestHit" class="mid-echart-canvas"></div>
                    </div>
                </div>

                <!-- Interactive Backtest Audit & Divergence Attribution Table -->
                <div class="filter-toolbar">
                    <div class="filter-group">
                        <button class="flt-btn active" onclick="filterBacktestRows('top18', this)">🔥 核心验证与复盘 (前18只)</button>
                        <button class="flt-btn" onclick="filterBacktestRows('hit', this)">✅ 实盘命中兑现 ({bt_hit_count}只)</button>
                        <button class="flt-btn" onclick="filterBacktestRows('miss', this)">⚠️ 预期抢跑/背离复盘 ({bt_miss_count}只 · 必看风控教训)</button>
                        <button class="flt-btn" onclick="filterBacktestRows('all', this)">全部回溯记录 ({len(bt_records)}只)</button>
                    </div>
                </div>

                <div class="table-scroll">
                    <table class="gs-table">
                        <thead>
                            <tr>
                                <th style="width:85px;" class="td-center">市场</th>
                                <th style="width:215px;">标的 & K线检验</th>
                                <th style="width:95px;" class="td-center">原评级</th>
                                <th style="width:85px;" class="td-center">5日实盘</th>
                                <th style="width:85px;" class="td-center">相对超额</th>
                                <th style="width:110px;" class="td-center">20日支撑 / 止损</th>
                                <th style="width:110px;" class="td-center">20日压力 / 目标</th>
                                <th style="width:155px;" class="td-center">实盘回溯判定</th>
                                <th>华尔街复盘归因与实操风控修正建议</th>
                            </tr>
                        </thead>
                        <tbody>
                            {"".join(backtest_rows_html)}
                        </tbody>
                    </table>
                </div>
            </section>

            <!-- Section 2: 4-Panel Visual Charts Dashboard -->
            <section id="sec-visual-charts" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">📊 模块二：高盛/大摩四维量化可视化看板（折线图 · 柱状图 · 多空双向条形图 · 四象限散点图）</h2>
                    <span class="sec-tag-pill">累计收益折线图 · 赛道景气柱状图 · 多空双向条形图 · 四象限散点图</span>
                </div>

                <div class="charts-2x2-grid">
                    <!-- Chart A: Multi-Series Normalized Return Line Chart -->
                    <div class="chart-panel-card">
                        <div class="chart-panel-title">
                            <span>📈 图 2.1：中美基准指数与核心龙头近 40 日累计收益率对比（折线图）</span>
                        </div>
                        <div class="chart-panel-sub">以40个交易日前为基准 (0%)，直观对比美股科技(QQQ)、美债(TLT)、A股指数与核心多空标的走势背离</div>
                        <div id="chartNormLine" class="mid-echart-canvas"></div>
                    </div>

                    <!-- Chart B: Grouped Sector Conviction & Real Return Column Chart -->
                    <div class="chart-panel-card">
                        <div class="chart-panel-title">
                            <span>📊 图 2.2：六大一级赛道【美股 vs A股景气评分 & 5日实盘涨跌】（柱状图）</span>
                        </div>
                        <div class="chart-panel-sub">对比高盛/大摩框架下各赛道中美影响评分 (1-10) 与板块代表股近5日实盘平均涨跌幅 (%)</div>
                        <div id="chartSectorCol" class="mid-echart-canvas"></div>
                    </div>

                    <!-- Chart C: Top Conviction OW vs UW Diverging Horizontal Bar Chart -->
                    <div class="chart-panel-card">
                        <div class="chart-panel-title">
                            <span>⚖️ 图 2.3：重点股票【多空评级强度 vs 实盘5日表现】（双向条形图）</span>
                        </div>
                        <div class="chart-panel-sub">右侧绿柱为首选超配（正向评分），左侧红柱为重点规避（负向评分），叠加近5日实盘涨跌幅验证</div>
                        <div id="chartTopBar" class="mid-echart-canvas"></div>
                    </div>

                    <!-- Chart D: 4-Quadrant Article Impact Scatter Chart -->
                    <div class="chart-panel-card">
                        <div class="chart-panel-title">
                            <span>🎯 图 2.4：《经济学人》重点文章“美股影响 vs A股影响”（四象限散点图）</span>
                        </div>
                        <div class="chart-panel-sub">左上象限为“东升西降/A股独立受益”，右上为“中美双牛共振”（点击任意气泡直接跳转至对应研报卡）</div>
                        <div id="chartQuadrantScatter" class="mid-echart-canvas"></div>
                    </div>
                </div>
            </section>

            <!-- Section 3: Compact Sector Scorecard Table -->
            <section id="sec-sector-table" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">📋 模块三：六大一级行业赛道评级、首选多头（超配）与规避标的（减持）速查表</h2>
                    <span class="sec-tag-pill">高盛/大摩 行业赛道评分卡</span>
                </div>
                <div class="table-scroll">
                    <table class="gs-table">
                        <thead>
                            <tr>
                                <th style="width:180px;">一级赛道 & 关联文章</th>
                                <th style="width:150px;" class="td-center">🇺🇸 美股评级 (均值)</th>
                                <th>🇺🇸 美股首选多头（超配） vs 重点规避（减持）</th>
                                <th style="width:150px;" class="td-center">🇨🇳 A股/港股评级</th>
                                <th>🇨🇳 A股/港股首选多头（超配） vs 重点规避（减持）</th>
                                <th style="width:110px;" class="td-center">代表股5日均值</th>
                            </tr>
                        </thead>
                        <tbody>
                            {"".join(sector_rows_html)}
                        </tbody>
                    </table>
                </div>
            </section>

            <!-- Section 4: Master Stock Table with Sparklines & K-Line Triggers -->
            <section id="sec-stock-master" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">🎯 模块四：全市场核心股票行情、迷你走势图、投行评级与一键看 K 线总表 ({len(all_stock_rows)} 股)</h2>
                    <span class="sec-tag-pill">全市场核心个股行情与迷你走势矩阵</span>
                </div>

                <div class="filter-toolbar">
                    <div class="filter-group">
                        <button class="flt-btn active" onclick="filterMasterStocks('top24', this)">🔥 核心精选 (前24只)</button>
                        <button class="flt-btn" onclick="filterMasterStocks('all', this)">全部标的 ({len(all_stock_rows)})</button>
                        <button class="flt-btn" onclick="filterMasterStocks('bull', this)">📈 超配多头 ({len(bull_stocks)})</button>
                        <button class="flt-btn" onclick="filterMasterStocks('bear', this)">📉 减持规避 ({len(bear_stocks)})</button>
                        <button class="flt-btn" onclick="filterMasterStocks('US', this)">🇺🇸 美股 ({len(us_stocks)})</button>
                        <button class="flt-btn" onclick="filterMasterStocks('CN', this)">🇨🇳 A股/港股 ({len(cn_stocks)})</button>
                    </div>
                    <input type="text" class="flt-search" placeholder="🔍 搜索代码 / 名称 / 赛道..." oninput="searchMasterStocks(this.value)">
                </div>

                <div class="table-scroll">
                    <table class="gs-table">
                        <thead>
                            <tr>
                                <th style="width:90px;" class="td-center">市场</th>
                                <th style="width:230px;">股票代码 & 名称 (点击看K线)</th>
                                <th style="width:130px;" class="td-center">投行评级</th>
                                <th style="width:85px;" class="td-right">最新价</th>
                                <th style="width:85px;" class="td-center">1日涨跌</th>
                                <th style="width:85px;" class="td-center">5日涨跌</th>
                                <th style="width:95px;" class="td-center">20日迷你趋势</th>
                                <th style="width:140px;">冲击维度</th>
                                <th>核心投研逻辑精要 (悬停看全文)</th>
                            </tr>
                        </thead>
                        <tbody>
                            {"".join(master_stock_rows_html)}
                        </tbody>
                    </table>
                </div>
            </section>

            <!-- Section 5: Compact Cross-Market Pair Trades Table -->
            <section id="sec-pair-trades" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">⚖️ 模块五：高盛/大摩跨市场多空配对交易与定价预期差速查表</h2>
                    <span class="sec-tag-pill">跨市场多空配对交易策略</span>
                </div>
                <div class="table-scroll">
                    <table class="gs-table">
                        <thead>
                            <tr>
                                <th style="width:170px;">配对主线</th>
                                <th style="width:20%;">《经济学人》触发信号</th>
                                <th style="width:21%;">🇺🇸 美股多空映射</th>
                                <th style="width:21%;">🇨🇳 A股/港股多空映射</th>
                                <th>💡 内外盘定价背离（超额收益来源）</th>
                            </tr>
                        </thead>
                        <tbody>
                            {"".join(pair_trade_rows_html)}
                        </tbody>
                    </table>
                </div>
            </section>

            <!-- Section 6: Article Heat Matrix -->
            <section id="sec-article-matrix" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">⚡ 模块六：《经济学人》重点文章多空方向与代表标的速查矩阵（点击任意行直达精读图表卡）</h2>
                    <span class="sec-tag-pill">重点文章多空热力速查矩阵 · {len(core_articles)} 篇</span>
                </div>
                <div class="table-scroll">
                    <table class="gs-table">
                        <thead>
                            <tr>
                                <th style="width:55px;" class="td-center">#</th>
                                <th style="width:25%;">文章中文标题 & 所属栏目</th>
                                <th style="width:115px;">赛道</th>
                                <th>一句话核心结论</th>
                                <th style="width:190px;">🇺🇸 美股方向 & 标的</th>
                                <th style="width:190px;">🇨🇳 A股/港股方向 & 标的</th>
                                <th style="width:80px;" class="td-center">直达</th>
                            </tr>
                        </thead>
                        <tbody>
                            {"".join(article_matrix_rows_html)}
                        </tbody>
                    </table>
                </div>
            </section>

            <!-- Section 7: Per-Article Visual Research Note Cards (with Embedded Interactive K-Line & Bar Charts!) -->
            <section id="sec-deep-notes" class="report-section">
                <div class="sec-header">
                    <h2 class="sec-title">📑 模块七：重点文章单篇可视化投研卡（左：精炼要点+迷你走势表 | 右：实盘日K线与涨跌柱状图）</h2>
                    <span class="sec-tag-pill">单篇可视化深度研报卡（内嵌K线与涨跌柱状图）</span>
                </div>

                <div class="filter-toolbar">
                    <div class="filter-group">
                        <button class="flt-btn active" onclick="filterNoteCards('all', this)">全部图表卡 ({len(core_articles)})</button>
                        <button class="flt-btn" onclick="filterNoteCards('cn-bull', this)">🇨🇳 利好A股/港股</button>
                        <button class="flt-btn" onclick="filterNoteCards('cn-bear', this)">🇨🇳 利空A股/港股</button>
                        <button class="flt-btn" onclick="filterNoteCards('us-bull', this)">🇺🇸 利好美股</button>
                        <button class="flt-btn" onclick="filterNoteCards('us-bear', this)">🇺🇸 利空美股</button>
                        <button class="flt-btn" onclick="filterNoteCards('high-score', this)">🔥 高冲击度 (≥8分)</button>
                    </div>
                    <input type="text" class="flt-search" placeholder="🔍 筛选研报卡片..." oninput="searchNoteCards(this.value)">
                </div>

                <div class="notes-stack" id="notesStack">
                    {"".join(deep_cards_html)}
                </div>
            </section>

            <!-- Section 8: Collapsible Secondary Digest -->
            <section id="sec-digest-table" class="report-section">
                <details>
                    <summary style="cursor:pointer;font-size:16px;font-weight:800;color:var(--gs-blue);">
                        🗂️ 模块八：点击展开同期其余 {len(digest_articles)} 篇《经济学人》文章简明速览表
                    </summary>
                    <div class="table-scroll" style="margin-top:14px;">
                        <table class="gs-table">
                            <thead>
                                <tr>
                                    <th style="width:45px;">#</th>
                                    <th style="width:26%;">文章标题 (点击跳转原文)</th>
                                    <th style="width:140px;">栏目</th>
                                    <th style="width:120px;">发布时间</th>
                                    <th>核心提要</th>
                                    <th style="width:22%;">边际启示</th>
                                </tr>
                            </thead>
                            <tbody>
                                {"".join(digest_rows_html)}
                            </tbody>
                        </table>
                    </div>
                </details>
            </section>
        </main>
    </div>

    <div class="floating-nav">
        <a href="#top" class="float-pill">↑ 实盘K线终端</a>
        <a href="#sec-visual-charts" class="float-pill">📊 四维图表</a>
        <a href="#sec-stock-master" class="float-pill">🎯 个股总表</a>
    </div>

    <script>
        const CURRENT_REPORT_ID = {json.dumps(report_date)};
        const ACK_API_BASE = "http://{ACK_SERVER_HOST}:{ACK_SERVER_PORT}";
        const INITIAL_ACK_SNAPSHOT = /*__ACK_SNAPSHOT_START__*/{initial_ack_snapshot}/*__ACK_SNAPSHOT_END__*/;
        const KLINE_MAP = {json.dumps(kline_map, ensure_ascii=False)};
        const STOCK_META = {json.dumps(stock_meta_by_sym, ensure_ascii=False)};
        const SECTOR_CHART_DATA = {json.dumps({"categories": sector_chart_categories, "us_scores": sector_us_scores, "cn_scores": sector_cn_scores, "real_5d": sector_real_5d}, ensure_ascii=False)};
        const DIVERGING_BAR_DATA = {json.dumps(diverging_items, ensure_ascii=False)};
        const SCATTER_POINTS = {json.dumps(scatter_points, ensure_ascii=False)};
        const PER_ARTICLE_CHARTS = {json.dumps(per_article_chart_configs, ensure_ascii=False)};
        const BACKTEST_DATA = {json.dumps(bt_chart_payload, ensure_ascii=False)};

        const chartInstances = {{}};

        function getThemeColors() {{
            const isDark = (document.documentElement.getAttribute("data-theme") === "dark");
            return {{
                isDark,
                text: isDark ? "#e2e8f0" : "#0f172a",
                muted: isDark ? "#94a3b8" : "#64748b",
                gridLine: isDark ? "#1e293b" : "#e2e8f0",
                upColor: "#16a34a",
                downColor: "#e11d48",
                usColor: "#2563eb",
                cnColor: "#e11d48",
                accentGold: "#d97706"
            }};
        }}

        function buildCandlestickOption(sym, compactMode) {{
            const kd = KLINE_MAP[sym];
            const c = getThemeColors();
            if (!kd) return {{}};
            const dates = kd.dates || [];
            const ohlc = kd.ohlc || [];
            const vols = kd.volumes || [];
            const ma5 = kd.ma5 || [];
            const ma20 = kd.ma20 || [];

            return {{
                animation: false,
                tooltip: {{
                    trigger: "axis",
                    axisPointer: {{ type: "cross" }}
                }},
                legend: {{
                    data: ["日K线", "5日均线", "20日均线"],
                    top: 2,
                    textStyle: {{ color: c.text, fontSize: compactMode ? 11 : 12 }}
                }},
                grid: compactMode
                    ? [
                        {{ left: 48, right: 14, top: 30, height: "58%" }},
                        {{ left: 48, right: 14, top: "74%", height: "16%" }}
                    ]
                    : [
                        {{ left: 56, right: 20, top: 36, height: "60%" }},
                        {{ left: 56, right: 20, top: "74%", height: "16%" }}
                    ],
                xAxis: [
                    {{
                        type: "category",
                        data: dates,
                        boundaryGap: true,
                        axisLine: {{ lineStyle: {{ color: c.muted }} }},
                        axisLabel: {{ color: c.muted, fontSize: 11 }}
                    }},
                    {{
                        type: "category",
                        gridIndex: 1,
                        data: dates,
                        axisLabel: {{ show: false }},
                        axisTick: {{ show: false }}
                    }}
                ],
                yAxis: [
                    {{
                        scale: true,
                        splitLine: {{ lineStyle: {{ color: c.gridLine }} }},
                        axisLabel: {{ color: c.muted, fontSize: 11 }}
                    }},
                    {{
                        scale: true,
                        gridIndex: 1,
                        splitNumber: 2,
                        axisLabel: {{ show: false }},
                        splitLine: {{ show: false }}
                    }}
                ],
                series: [
                    {{
                        name: "日K线",
                        type: "candlestick",
                        data: ohlc,
                        itemStyle: {{
                            color: c.upColor,
                            color0: c.downColor,
                            borderColor: c.upColor,
                            borderColor0: c.downColor
                        }},
                        markLine: {{
                            symbol: ["none", "none"],
                            label: {{ formatter: "经济学人刊发窗口", fontSize: 10, color: c.accentGold }},
                            lineStyle: {{ color: c.accentGold, type: "dashed", width: 1.5 }},
                            data: [{{ xAxis: dates[Math.max(0, dates.length - 3)] }}]
                        }}
                    }},
                    {{
                        name: "5日均线",
                        type: "line",
                        data: ma5,
                        smooth: true,
                        showSymbol: false,
                        lineStyle: {{ width: 1.6, color: "#f59e0b" }}
                    }},
                    {{
                        name: "20日均线",
                        type: "line",
                        data: ma20,
                        smooth: true,
                        showSymbol: false,
                        lineStyle: {{ width: 1.6, color: "#6366f1" }}
                    }},
                    {{
                        name: "成交量",
                        type: "bar",
                        xAxisIndex: 1,
                        yAxisIndex: 1,
                        data: vols.map((v, idx) => ({{
                            value: v,
                            itemStyle: {{
                                color: (ohlc[idx] && ohlc[idx][1] >= ohlc[idx][0]) ? "rgba(22,163,74,0.55)" : "rgba(225,29,72,0.55)"
                            }}
                        }}))
                    }}
                ]
            }};
        }}

        function renderMainKline(sym, btnEl) {{
            if (!KLINE_MAP[sym]) return;
            document.querySelectorAll(".kline-tab-btn").forEach(b => {{
                b.classList.toggle("active", b.getAttribute("data-sym") === sym);
            }});
            if (btnEl) btnEl.classList.add("active");

            const dom = document.getElementById("mainKlineChart");
            if (dom && window.echarts) {{
                let inst = chartInstances["mainKline"] || echarts.init(dom);
                chartInstances["mainKline"] = inst;
                inst.setOption(buildCandlestickOption(sym, false), true);
            }}

            const kd = KLINE_MAP[sym];
            const meta = STOCK_META[sym] || {{}};
            const side = document.getElementById("mainKlineSidecard");
            if (side && kd) {{
                const fmtPct = (v) => {{
                    const s = v > 0 ? "+" : "";
                    const cl = v > 0 ? "pct-up" : (v < 0 ? "pct-down" : "pct-flat");
                    return `<span class="pct-pill ${{cl}}">${{s}}${{v.toFixed(2)}}%</span>`;
                }};
                const ratingStr = meta.rating || (kd.chg_5d_pct >= 0 ? "超配增持" : "标配观望");
                const jumpHtml = meta.article_idx
                    ? `<a href="#art-${{meta.article_idx}}" onclick="jumpToArticle('art-${{meta.article_idx}}');return false;" class="jump-link">跳转关联研报卡 #${{meta.article_idx}} (${{meta.article_title}}) ↓</a>`
                    : `<span style="color:var(--text-muted);font-size:12px;">基准核心指数标的</span>`;
                side.innerHTML = `
                    <div>
                        <div style="font-size:12px;color:var(--text-muted);font-weight:700;">当前选中实盘标的 (${{kd.latest_date}})</div>
                        <div style="font-size:20px;font-weight:800;color:var(--text-main);margin:4px 0;">${{kd.name}}</div>
                        <div style="font-size:28px;font-weight:800;font-family:monospace;color:var(--gs-blue);margin-bottom:10px;">
                            ${{kd.latest_close.toFixed(2)}}
                        </div>
                        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;background:var(--bg-surface);padding:10px;border-radius:6px;border:1px solid var(--border-light);margin-bottom:12px;">
                            <div><div style="font-size:11px;color:var(--text-muted);">1日涨跌</div>${{fmtPct(kd.chg_1d_pct)}}</div>
                            <div><div style="font-size:11px;color:var(--text-muted);">5日涨跌</div>${{fmtPct(kd.chg_5d_pct)}}</div>
                            <div><div style="font-size:11px;color:var(--text-muted);">20日涨跌</div>${{fmtPct(kd.chg_20d_pct)}}</div>
                        </div>
                        <div style="margin-bottom:8px;">
                            <span style="font-size:12px;font-weight:700;color:var(--text-muted);">投行评级：</span>
                            <strong style="color:var(--gs-blue);">${{ratingStr}}</strong>
                            ${{meta.verdict_tag ? `<span style="margin-left:6px;font-size:11.5px;font-weight:800;color:#15803d;">(${{meta.verdict_tag}})</span>` : ""}}
                        </div>
                        ${{meta.support_20d ? `
                        <div style="display:grid;grid-template-columns:1fr 1fr;gap:6px;font-size:11.5px;font-family:monospace;background:var(--bg-surface);padding:8px;border-radius:6px;border:1px solid var(--border-light);margin-bottom:8px;">
                            <div>🟢 20日支撑: <strong>${{meta.support_20d}}</strong> | 止损: <strong>${{meta.stop_level}}</strong></div>
                            <div>🔴 20日压力: <strong>${{meta.resist_20d}}</strong> | 目标: <strong>${{meta.target_level}}</strong></div>
                        </div>` : ""}}
                        <div style="margin-bottom:8px;">
                            <span style="font-size:12px;font-weight:700;color:var(--text-muted);">冲击维度：</span>
                            <span class="driver-chip">${{meta.driver || "宏观流动性与估值折现"}}</span>
                        </div>
                        <div style="font-size:13px;color:var(--text-secondary);background:var(--bg-surface);padding:10px;border-radius:6px;border-left:3px solid var(--econ-red);">
                            <strong>《经济学人》催化逻辑：</strong><br>
                            ${{meta.thesis_short || "反映近期美国中期选举政治重构、人工智能资本开支投入回报率审视与全球关税豁免预期差。"}}
                        </div>
                    </div>
                    <div style="padding-top:8px;border-top:1px solid var(--border-light);">${{jumpHtml}}</div>
                `;
            }}
        }}

        function jumpAndShowKline(sym) {{
            if (!KLINE_MAP[sym]) return;
            document.getElementById("sec-kline-terminal").scrollIntoView({{ behavior: "smooth", block: "start" }});
            renderMainKline(sym);
        }}

        function renderBacktestCharts() {{
            if (!window.echarts || !BACKTEST_DATA) return;
            const c = getThemeColors();

            // 1. 20D Long-Short Portfolio NAV Curve
            const navDom = document.getElementById("chartBacktestNav");
            if (navDom) {{
                let inst = chartInstances["btNav"] || echarts.init(navDom);
                chartInstances["btNav"] = inst;
                inst.setOption({{
                    tooltip: {{ trigger: "axis", valueFormatter: (v) => (v !== undefined ? v + "%" : "") }},
                    legend: {{
                        data: ["多空对冲超额净值", "超配多头组合", "规避空头组合", "跨市场基准"],
                        top: 2,
                        textStyle: {{ color: c.text, fontSize: 11 }}
                    }},
                    grid: {{ left: 46, right: 18, top: 38, bottom: 26 }},
                    xAxis: {{ type: "category", data: BACKTEST_DATA.nav_dates, axisLabel: {{ color: c.muted, fontSize: 11 }} }},
                    yAxis: {{ type: "value", axisLabel: {{ formatter: "{{value}}%", color: c.muted, fontSize: 11 }}, splitLine: {{ lineStyle: {{ color: c.gridLine }} }} }},
                    series: [
                        {{
                            name: "多空对冲超额净值",
                            type: "line",
                            smooth: true,
                            data: BACKTEST_DATA.ls_alpha_curve,
                            lineStyle: {{ width: 3, color: "#d97706" }},
                            itemStyle: {{ color: "#d97706" }},
                            areaStyle: {{ color: "rgba(217, 119, 6, 0.12)" }}
                        }},
                        {{
                            name: "超配多头组合",
                            type: "line",
                            smooth: true,
                            showSymbol: false,
                            data: BACKTEST_DATA.long_curve,
                            lineStyle: {{ width: 2, color: "#16a34a" }},
                            itemStyle: {{ color: "#16a34a" }}
                        }},
                        {{
                            name: "规避空头组合",
                            type: "line",
                            smooth: true,
                            showSymbol: false,
                            data: BACKTEST_DATA.short_curve,
                            lineStyle: {{ width: 2, type: "dashed", color: "#e11d48" }},
                            itemStyle: {{ color: "#e11d48" }}
                        }},
                        {{
                            name: "跨市场基准",
                            type: "line",
                            smooth: true,
                            showSymbol: false,
                            data: BACKTEST_DATA.bm_curve,
                            lineStyle: {{ width: 1.5, color: "#64748b" }},
                            itemStyle: {{ color: "#64748b" }}
                        }}
                    ]
                }}, true);
            }}

            // 2. Hit Rate Breakdown Bar Chart
            const hitDom = document.getElementById("chartBacktestHit");
            if (hitDom) {{
                let inst = chartInstances["btHit"] || echarts.init(hitDom);
                chartInstances["btHit"] = inst;
                const cats = ["综合总胜率", "多头超配胜率", "空头排雷胜率", "🇺🇸 美股胜率", "🇨🇳 A/港股胜率"];
                const vals = [
                    BACKTEST_DATA.overall_hit_rate,
                    BACKTEST_DATA.long_hit_rate,
                    BACKTEST_DATA.short_hit_rate,
                    BACKTEST_DATA.us_hit_rate,
                    BACKTEST_DATA.cn_hit_rate
                ];
                inst.setOption({{
                    tooltip: {{ trigger: "axis", valueFormatter: (v) => v + "%" }},
                    grid: {{ left: 46, right: 20, top: 28, bottom: 32 }},
                    xAxis: {{ type: "category", data: cats, axisLabel: {{ color: c.text, fontSize: 11, interval: 0 }} }},
                    yAxis: {{ type: "value", min: 0, max: 100, axisLabel: {{ formatter: "{{value}}%", color: c.muted, fontSize: 11 }}, splitLine: {{ lineStyle: {{ color: c.gridLine }} }} }},
                    series: [{{
                        name: "实盘命中率",
                        type: "bar",
                        barWidth: "42%",
                        data: vals.map((v, idx) => ({{
                            value: v,
                            itemStyle: {{
                                color: idx === 4 ? "#059669" : (v >= 65 ? "#2563eb" : "#d97706"),
                                borderRadius: [4, 4, 0, 0]
                            }},
                            label: {{ show: true, position: "top", formatter: "{{c}}%", color: c.text, fontWeight: "bold", fontSize: 11 }}
                        }})),
                        markLine: {{
                            silent: true,
                            lineStyle: {{ color: "#e11d48", type: "dashed", width: 1.5 }},
                            label: {{ formatter: "50% 随机基准线", fontSize: 10, color: "#e11d48" }},
                            data: [{{ yAxis: 50 }}]
                        }}
                    }}]
                }}, true);
            }}
        }}

        function filterBacktestRows(mode, btnEl) {{
            const bar = btnEl ? btnEl.closest(".filter-toolbar") : null;
            if (bar) bar.querySelectorAll(".flt-btn").forEach(b => b.classList.remove("active"));
            if (btnEl) btnEl.classList.add("active");
            document.querySelectorAll(".bt-audit-row").forEach(row => {{
                const h = row.getAttribute("data-hit");
                const idx = parseInt(row.getAttribute("data-idx") || "0", 10);
                let show = true;
                if (mode === "top18") show = (idx <= 18);
                else if (mode === "hit") show = (h === "hit");
                else if (mode === "miss") show = (h === "miss");
                row.style.display = show ? "" : "none";
            }});
        }}

        function renderFourOverviewCharts() {{
            if (!window.echarts) return;
            const c = getThemeColors();

            // 1. Multi-Series Normalized Return Line Chart
            const normDom = document.getElementById("chartNormLine");
            if (normDom) {{
                let inst = chartInstances["normLine"] || echarts.init(normDom);
                chartInstances["normLine"] = inst;
                const compareSyms = ["QQQ", "SPY", "TLT", "000001.SH", "FSLR", "WMT", "603259.SH", "601899.SH"].filter(s => KLINE_MAP[s]);
                const refDates = compareSyms.length ? KLINE_MAP[compareSyms[0]].dates : [];
                const series = compareSyms.map(s => ({{
                    name: KLINE_MAP[s].name.split("(")[0].trim(),
                    type: "line",
                    smooth: true,
                    showSymbol: false,
                    data: KLINE_MAP[s].norm_pct.slice(-refDates.length),
                    lineStyle: {{ width: 2.2 }}
                }}));
                inst.setOption({{
                    tooltip: {{ trigger: "axis", valueFormatter: (v) => (v !== undefined ? v + "%" : "") }},
                    legend: {{ top: 2, textStyle: {{ color: c.text, fontSize: 11 }} }},
                    grid: {{ left: 46, right: 16, top: 44, bottom: 26 }},
                    xAxis: {{ type: "category", data: refDates, axisLabel: {{ color: c.muted, fontSize: 11 }} }},
                    yAxis: {{ type: "value", axisLabel: {{ formatter: "{{value}}%", color: c.muted, fontSize: 11 }}, splitLine: {{ lineStyle: {{ color: c.gridLine }} }} }},
                    series
                }}, true);
            }}

            // 2. Grouped Sector Conviction & Real Return Column Chart
            const secDom = document.getElementById("chartSectorCol");
            if (secDom) {{
                let inst = chartInstances["sectorCol"] || echarts.init(secDom);
                chartInstances["sectorCol"] = inst;
                inst.setOption({{
                    tooltip: {{ trigger: "axis", axisPointer: {{ type: "shadow" }} }},
                    legend: {{ data: ["🇺🇸 美股冲击分(1-10)", "🇨🇳 A股冲击分(1-10)", "代表股5日均涨跌(%)"], top: 2, textStyle: {{ color: c.text, fontSize: 11 }} }},
                    grid: {{ left: 46, right: 46, top: 38, bottom: 42 }},
                    xAxis: {{
                        type: "category",
                        data: SECTOR_CHART_DATA.categories,
                        axisLabel: {{ color: c.text, fontSize: 11, interval: 0, rotate: 14 }}
                    }},
                    yAxis: [
                        {{ type: "value", name: "评分", min: 0, max: 10, axisLabel: {{ color: c.muted, fontSize: 11 }}, splitLine: {{ lineStyle: {{ color: c.gridLine }} }} }},
                        {{ type: "value", name: "5日涨幅", axisLabel: {{ formatter: "{{value}}%", color: c.muted, fontSize: 11 }}, splitLine: {{ show: false }} }}
                    ],
                    series: [
                        {{ name: "🇺🇸 美股冲击分(1-10)", type: "bar", data: SECTOR_CHART_DATA.us_scores, itemStyle: {{ color: "#2563eb", borderRadius: [3,3,0,0] }} }},
                        {{ name: "🇨🇳 A股冲击分(1-10)", type: "bar", data: SECTOR_CHART_DATA.cn_scores, itemStyle: {{ color: "#059669", borderRadius: [3,3,0,0] }} }},
                        {{ name: "代表股5日均涨跌(%)", type: "line", yAxisIndex: 1, data: SECTOR_CHART_DATA.real_5d, symbolSize: 7, lineStyle: {{ width: 2.2, color: "#d97706" }}, itemStyle: {{ color: "#d97706" }} }}
                    ]
                }}, true);
            }}

            // 3. Top Stock Diverging Bar Chart
            const barDom = document.getElementById("chartTopBar");
            if (barDom) {{
                let inst = chartInstances["topBar"] || echarts.init(barDom);
                chartInstances["topBar"] = inst;
                const labels = DIVERGING_BAR_DATA.map(d => d.label);
                const scores = DIVERGING_BAR_DATA.map(d => ({{
                    value: d.score,
                    itemStyle: {{ color: d.score >= 0 ? "#16a34a" : "#e11d48", borderRadius: 3 }}
                }}));
                const chg5d = DIVERGING_BAR_DATA.map(d => d.chg_5d);
                inst.setOption({{
                    tooltip: {{ trigger: "axis", axisPointer: {{ type: "shadow" }} }},
                    legend: {{ data: ["多空评级强度 (-10~+10)", "实盘5日涨跌幅 (%)"], top: 2, textStyle: {{ color: c.text, fontSize: 11 }} }},
                    grid: {{ left: 135, right: 24, top: 34, bottom: 24 }},
                    xAxis: {{ type: "value", splitLine: {{ lineStyle: {{ color: c.gridLine }} }}, axisLabel: {{ color: c.muted, fontSize: 11 }} }},
                    yAxis: {{ type: "category", data: labels, axisLabel: {{ color: c.text, fontSize: 11 }} }},
                    series: [
                        {{ name: "多空评级强度 (-10~+10)", type: "bar", data: scores }},
                        {{ name: "实盘5日涨跌幅 (%)", type: "scatter", data: chg5d, symbolSize: 9, itemStyle: {{ color: "#2563eb" }} }}
                    ]
                }}, true);
            }}

            // 4. 4-Quadrant Article Impact Scatter Chart
            const scatDom = document.getElementById("chartQuadrantScatter");
            if (scatDom) {{
                let inst = chartInstances["quadScatter"] || echarts.init(scatDom);
                chartInstances["quadScatter"] = inst;
                inst.setOption({{
                    tooltip: {{
                        formatter: (params) => {{
                            const d = params.data;
                            return `<strong>#${{d.idx}} ${{d.title}}</strong><br/>赛道: ${{d.category}}<br/>🇺🇸 美股影响: ${{d.value[0]}}<br/>🇨🇳 A股影响: ${{d.value[1]}}<br/><em>点击气泡直达研报卡</em>`;
                        }}
                    }},
                    grid: {{ left: 46, right: 24, top: 28, bottom: 34 }},
                    xAxis: {{
                        type: "value", name: "🇺🇸 美股多空 (-10↔+10)", nameLocation: "middle", nameGap: 22,
                        min: -10, max: 10, splitLine: {{ lineStyle: {{ color: c.gridLine }} }}, axisLabel: {{ color: c.muted, fontSize: 11 }}
                    }},
                    yAxis: {{
                        type: "value", name: "🇨🇳 A股多空 (-10↔+10)", min: -10, max: 10,
                        splitLine: {{ lineStyle: {{ color: c.gridLine }} }}, axisLabel: {{ color: c.muted, fontSize: 11 }}
                    }},
                    series: [{{
                        type: "scatter",
                        data: SCATTER_POINTS.map(p => ({{
                            value: [p.x, p.y],
                            idx: p.idx,
                            anchor: p.anchor,
                            title: p.title,
                            category: p.category,
                            symbolSize: p.size,
                            itemStyle: {{
                                color: (p.y > 0 && p.x < 0) ? "#059669" : (p.y > 0 && p.x > 0 ? "#2563eb" : "#e11d48"),
                                opacity: 0.85
                            }},
                            label: {{ show: true, formatter: "#" + p.idx, position: "inside", color: "#fff", fontSize: 10, fontWeight: "bold" }}
                        }})),
                        markLine: {{
                            silent: true,
                            lineStyle: {{ color: c.muted, type: "dashed" }},
                            data: [{{ xAxis: 0 }}, {{ yAxis: 0 }}]
                        }}
                    }}]
                }}, true);
                inst.off("click");
                inst.on("click", (params) => {{
                    if (params.data && params.data.anchor) {{
                        jumpToArticle(params.data.anchor);
                    }}
                }});
            }}
        }}

        function switchArticleChart(artIdx, sym, btnEl) {{
            if (btnEl) {{
                const bar = document.getElementById("art-tabs-" + artIdx);
                if (bar) bar.querySelectorAll(".art-chart-tab").forEach(b => b.classList.remove("active"));
                btnEl.classList.add("active");
            }}
            const dom = document.getElementById("art-echart-" + artIdx);
            if (!dom || !window.echarts || !KLINE_MAP[sym]) return;
            let inst = chartInstances["art_" + artIdx] || echarts.init(dom);
            chartInstances["art_" + artIdx] = inst;
            inst.setOption(buildCandlestickOption(sym, true), true);
        }}

        function switchArticleBarChart(artIdx, btnEl) {{
            if (btnEl) {{
                const bar = document.getElementById("art-tabs-" + artIdx);
                if (bar) bar.querySelectorAll(".art-chart-tab").forEach(b => b.classList.remove("active"));
                btnEl.classList.add("active");
            }}
            const cfg = PER_ARTICLE_CHARTS[String(artIdx)];
            const dom = document.getElementById("art-echart-" + artIdx);
            if (!dom || !window.echarts || !cfg) return;
            const c = getThemeColors();
            let inst = chartInstances["art_" + artIdx] || echarts.init(dom);
            chartInstances["art_" + artIdx] = inst;
            const rows = cfg.stocks_bar || [];
            inst.setOption({{
                tooltip: {{ trigger: "axis", axisPointer: {{ type: "shadow" }} }},
                legend: {{ data: ["1日涨跌(%)", "5日涨跌(%)", "20日涨跌(%)"], top: 2, textStyle: {{ color: c.text, fontSize: 11 }} }},
                grid: {{ left: 44, right: 14, top: 32, bottom: 38 }},
                xAxis: {{ type: "category", data: rows.map(r => r.name.split("(")[0].trim()), axisLabel: {{ color: c.text, fontSize: 10, interval: 0, rotate: 12 }} }},
                yAxis: {{ type: "value", axisLabel: {{ formatter: "{{value}}%", color: c.muted, fontSize: 10 }}, splitLine: {{ lineStyle: {{ color: c.gridLine }} }} }},
                series: [
                    {{ name: "1日涨跌(%)", type: "bar", data: rows.map(r => r.d1), itemStyle: {{ color: "#3b82f6" }} }},
                    {{ name: "5日涨跌(%)", type: "bar", data: rows.map(r => r.d5), itemStyle: {{ color: "#10b981" }} }},
                    {{ name: "20日涨跌(%)", type: "bar", data: rows.map(r => r.d20), itemStyle: {{ color: "#6366f1" }} }}
                ]
            }}, true);
        }}

        function initAllArticleCharts() {{
            Object.keys(PER_ARTICLE_CHARTS).forEach(k => {{
                const cfg = PER_ARTICLE_CHARTS[k];
                if (cfg && cfg.syms && cfg.syms.length > 0) {{
                    switchArticleChart(k, cfg.syms[0]);
                }}
            }});
        }}

        function toggleTheme() {{
            const htmlEl = document.documentElement;
            const current = htmlEl.getAttribute("data-theme") || "light";
            const next = current === "light" ? "dark" : "light";
            htmlEl.setAttribute("data-theme", next);
            localStorage.setItem("economist_theme", next);
            const btn = document.getElementById("themeToggleBtn");
            if (btn) btn.textContent = next === "light" ? "🌙 切换暗色终端" : "☀️ 切换投行白底";
            const activeKBtn = document.querySelector(".kline-tab-btn.active");
            const activeSym = activeKBtn ? activeKBtn.getAttribute("data-sym") : "SPY";
            renderMainKline(activeSym);
            renderBacktestCharts();
            renderFourOverviewCharts();
            initAllArticleCharts();
        }}

        (function initTheme() {{
            const saved = localStorage.getItem("economist_theme") || "light";
            document.documentElement.setAttribute("data-theme", saved);
        }})();

        function getLocalAck(reportId) {{
            try {{
                const raw = localStorage.getItem("economist_ack_" + reportId);
                return raw ? JSON.parse(raw) : null;
            }} catch (e) {{ return null; }}
        }}

        function setLocalAck(reportId, acked, ackedAt) {{
            try {{
                localStorage.setItem("economist_ack_" + reportId, JSON.stringify({{
                    report_id: reportId,
                    acked: !!acked,
                    acked_at: ackedAt || new Date().toISOString().replace("T", " ").slice(0, 19)
                }}));
            }} catch (e) {{}}
        }}

        function renderAckUI(acked, ackedAt) {{
            const pill = document.getElementById("ackStatusPill");
            const btn = document.getElementById("ackBtn");
            if (acked) {{
                pill.className = "ack-status-pill acked";
                pill.textContent = "✅ 已审阅 (" + (ackedAt || "刚刚") + ")";
                btn.className = "ack-btn btn-acked";
                btn.innerHTML = "✅ 已确认审阅 · 点击撤销";
                btn.dataset.acked = "true";
            }} else {{
                pill.className = "ack-status-pill unacked";
                pill.textContent = "🔴 待审阅 (未读)";
                btn.className = "ack-btn btn-to-ack";
                btn.innerHTML = "✅ 确认已阅 (本期报告)";
                btn.dataset.acked = "false";
            }}
        }}

        function renderUnackedBanner(unackedReports) {{
            const banner = document.getElementById("unackedBanner");
            const listEl = document.getElementById("unackedList");
            if (!banner || !listEl) return;

            const others = (unackedReports || []).filter(r => r.report_id !== CURRENT_REPORT_ID);
            if (others.length === 0) {{
                banner.classList.remove("visible");
                listEl.innerHTML = "";
                return;
            }}
            banner.classList.add("visible");
            const isHttp = window.location.protocol.startsWith("http");
            listEl.innerHTML = others.map(r => {{
                const href = isHttp ? ("/reports/" + r.rel_path) : ("../../../" + r.rel_path);
                return `<span class="unacked-chip">
                    <a href="${{href}}" style="color:inherit;text-decoration:none;">📄 ${{r.report_id}} (未读)</a>
                    <button onclick="quickAckOther('${{r.report_id}}')" style="cursor:pointer;background:#10b981;color:#fff;border:none;border-radius:4px;padding:2px 7px;font-size:11px;font-weight:800;">确认已阅</button>
                </span>`;
            }}).join("");
        }}

        async function syncWithServer() {{
            const savedTheme = localStorage.getItem("economist_theme") || "light";
            const tBtn = document.getElementById("themeToggleBtn");
            if (tBtn) tBtn.textContent = savedTheme === "light" ? "🌙 切换暗色终端" : "☀️ 切换投行白底";

            const isHttp = window.location.protocol.startsWith("http");
            if (isHttp) {{
                document.querySelectorAll("[data-http-href]").forEach(el => {{
                    el.setAttribute("href", el.getAttribute("data-http-href"));
                }});
            }} else {{
                const portal = document.getElementById("archivePortalLink");
                if (portal) portal.setAttribute("href", "../../../index.html");
            }}

            const local = getLocalAck(CURRENT_REPORT_ID);
            const initAcked = (local && typeof local.acked === "boolean") ? local.acked : INITIAL_ACK_SNAPSHOT.acked;
            const initAt = (local && local.acked_at) ? local.acked_at : INITIAL_ACK_SNAPSHOT.acked_at;
            renderAckUI(initAcked, initAt);

            try {{
                const resp = await fetch(ACK_API_BASE + "/api/ack-state", {{ cache: "no-store" }});
                if (!resp.ok) return;
                const data = await resp.json();
                const srvEntry = (data.reports || {{}})[CURRENT_REPORT_ID];

                if (local && local.acked && srvEntry && !srvEntry.acked) {{
                    await postAckToServer(CURRENT_REPORT_ID, true);
                    return;
                }}
                if (srvEntry) {{
                    renderAckUI(srvEntry.acked, srvEntry.acked_at);
                    setLocalAck(CURRENT_REPORT_ID, srvEntry.acked, srvEntry.acked_at);
                }}
                renderUnackedBanner(data.unacked_reports || []);
            }} catch (e) {{}}
        }}

        async function postAckToServer(reportId, acked) {{
            const nowStr = new Date().toISOString().replace("T", " ").slice(0, 19);
            setLocalAck(reportId, acked, acked ? nowStr : null);
            if (reportId === CURRENT_REPORT_ID) {{
                renderAckUI(acked, acked ? nowStr : null);
            }}
            try {{
                const resp = await fetch(ACK_API_BASE + "/api/ack", {{
                    method: "POST",
                    headers: {{ "Content-Type": "application/json" }},
                    body: JSON.stringify({{ report_id: reportId, acked: acked }})
                }});
                if (resp.ok) {{
                    const data = await resp.json();
                    if (reportId === CURRENT_REPORT_ID && data.entry) {{
                        renderAckUI(data.entry.acked, data.entry.acked_at);
                        setLocalAck(reportId, data.entry.acked, data.entry.acked_at);
                    }}
                    renderUnackedBanner(data.unacked_reports || []);
                    document.querySelectorAll("#sidebarArchiveList .arch-item").forEach(item => {{
                        const rid = item.getAttribute("data-report-id");
                        const rInfo = (data.reports || {{}})[rid];
                        if (rInfo) {{
                            item.classList.toggle("acked", !!rInfo.acked);
                            item.classList.toggle("unacked", !rInfo.acked);
                            const st = item.querySelector(".arch-status");
                            if (st) st.textContent = rInfo.acked ? "✅ 已审阅" : "🔴 待审阅";
                        }}
                    }});
                }}
            }} catch (e) {{
                try {{
                    const img = new Image();
                    img.src = `${{ACK_API_BASE}}/api/ack-via-get?report_id=${{encodeURIComponent(reportId)}}&acked=${{acked ? 1 : 0}}&t=${{Date.now()}}`;
                }} catch (err) {{}}
            }}
        }}

        function toggleAck() {{
            const btn = document.getElementById("ackBtn");
            const currentlyAcked = btn.dataset.acked === "true";
            postAckToServer(CURRENT_REPORT_ID, !currentlyAcked);
        }}

        function quickAckOther(reportId) {{
            postAckToServer(reportId, true);
        }}

        function jumpToArticle(anchorId) {{
            const el = document.getElementById(anchorId);
            if (!el) return;
            el.style.display = "";
            el.scrollIntoView({{ behavior: "smooth", block: "start" }});
            el.classList.add("highlight-flash");
            setTimeout(() => el.classList.remove("highlight-flash"), 2400);
        }}

        function filterMasterStocks(mode, btnEl) {{
            const bar = btnEl ? btnEl.closest(".filter-toolbar") : null;
            if (bar) bar.querySelectorAll(".flt-btn").forEach(b => b.classList.remove("active"));
            if (btnEl) btnEl.classList.add("active");
            const rows = document.querySelectorAll(".stock-master-row");
            rows.forEach((row, idx) => {{
                const mkt = row.dataset.market;
                const pol = row.dataset.polarity;
                let show = true;
                if (mode === "top24") show = (idx < 24);
                else if (mode === "bull") show = (pol === "bull");
                else if (mode === "bear") show = (pol === "bear");
                else if (mode === "US") show = (mkt === "US");
                else if (mode === "CN") show = (mkt === "CN");
                row.style.display = show ? "" : "none";
            }});
        }}

        function searchMasterStocks(query) {{
            const q = (query || "").trim().toLowerCase();
            document.querySelectorAll(".stock-master-row").forEach((row, idx) => {{
                const hay = row.dataset.search || "";
                row.style.display = (!q ? idx < 24 : hay.includes(q)) ? "" : "none";
            }});
        }}

        function filterNoteCards(mode, btnEl) {{
            const bar = btnEl ? btnEl.closest(".filter-toolbar") : null;
            if (bar) bar.querySelectorAll(".flt-btn").forEach(b => b.classList.remove("active"));
            if (btnEl) btnEl.classList.add("active");
            document.querySelectorAll(".gs-note-card").forEach(card => {{
                const us = card.dataset.us;
                const cn = card.dataset.cn;
                const sc = parseInt(card.dataset.score || "0", 10);
                let show = true;
                if (mode === "cn-bull") show = (cn === "bull");
                else if (mode === "cn-bear") show = (cn === "bear");
                else if (mode === "us-bull") show = (us === "bull");
                else if (mode === "us-bear") show = (us === "bear");
                else if (mode === "high-score") show = (sc >= 8);
                card.style.display = show ? "" : "none";
            }});
        }}

        function searchNoteCards(query) {{
            const q = (query || "").trim().toLowerCase();
            document.querySelectorAll(".gs-note-card").forEach(card => {{
                const hay = card.dataset.search || "";
                const bodyText = card.innerText.toLowerCase();
                card.style.display = (!q || hay.includes(q) || bodyText.includes(q)) ? "" : "none";
            }});
        }}

        window.addEventListener("resize", () => {{
            Object.values(chartInstances).forEach(inst => {{
                if (inst && inst.resize) inst.resize();
            }});
        }});

        window.addEventListener("keydown", (e) => {{
            if (e.altKey && (e.key === "a" || e.key === "A")) {{
                e.preventDefault();
                toggleAck();
            }}
        }});

        window.addEventListener("DOMContentLoaded", () => {{
            syncWithServer();
            const firstSym = Object.keys(KLINE_MAP)[0] || "SPY";
            renderMainKline(firstSym);
            renderBacktestCharts();
            renderFourOverviewCharts();
            initAllArticleCharts();
        }});
    </script>
</body>
</html>
"""

    out_path.write_text(html_doc, encoding="utf-8")
    render_archive_index()
    return out_path


def render_archive_index() -> Path:
    """Render D:\\code\\economist\\reports\\index.html in matching GS/MS institutional table style."""
    state = load_ack_state()
    reports = sorted(
        state.get("reports", {}).values(),
        key=lambda x: x.get("date", x.get("report_id", "")),
        reverse=True,
    )
    unacked_count = sum(1 for r in reports if not r.get("acked", False))

    rows_html = []
    for r in reports:
        rid = r.get("report_id", "")
        rel = r.get("rel_path", "")
        acked = r.get("acked", False)
        acked_at = r.get("acked_at") or "-"
        badge = (
            f'<span class="st-badge acked">✅ 已审阅 ({_esc(acked_at)})</span>'
            if acked
            else '<span class="st-badge unacked">🔴 未审阅 (待确认)</span>'
        )
        btn_label = "撤销已阅" if acked else "✅ 确认已阅"
        next_ack = "false" if acked else "true"
        rows_html.append(
            f"""
            <tr>
                <td style="font-weight:800;font-family:monospace;font-size:15px;color:#003594;">{_esc(rid)}</td>
                <td>
                    <a href="{_esc(rel)}" data-http-href="/reports/{_esc(rel)}" class="rep-link">{_esc(r.get("title"))}</a>
                    <div style="font-size:12px;color:#64748b;margin-top:3px;">归档路径: reports/{_esc(rel)} | 统计窗口: {_esc(r.get("window_desc", ""))}</div>
                </td>
                <td><strong>{r.get("core_count", 0)}</strong> 篇交互K线研报卡 / 共 {r.get("article_count", 0)} 篇</td>
                <td>{_esc(r.get("generated_at", ""))}</td>
                <td>{badge}</td>
                <td>
                    <a href="{_esc(rel)}" data-http-href="/reports/{_esc(rel)}" class="act-btn open-btn">打开图表研报 ↗</a>
                    <button onclick="toggleArchiveAck('{_esc(rid)}', {next_ack})" class="act-btn">{btn_label}</button>
                </td>
            </tr>
            """
        )

    index_html = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>《经济学人》高盛/大摩量化图表研报 — 历史归档与审阅总控台</title>
    <style>
        body {{ background:#f1f5f9; color:#0f172a; font-family:-apple-system,BlinkMacSystemFont,"PingFang SC","Microsoft YaHei",sans-serif; padding:32px; line-height:1.65; }}
        .container {{ max-width:1420px; margin:0 auto; background:#ffffff; border:1px solid #cbd5e1; border-top:5px solid #0a192f; border-radius:10px; padding:28px; box-shadow:0 4px 16px rgba(15,23,42,0.06); }}
        h1 {{ font-size:22px; margin-bottom:8px; display:flex; align-items:center; gap:12px; color:#0a192f; }}
        .econ-tag {{ background:#e3120b; color:#fff; font-size:12px; padding:4px 10px; border-radius:3px; }}
        .gs-tag {{ background:#003594; color:#fff; font-size:12px; padding:4px 10px; border-radius:3px; }}
        .sub {{ color:#475569; font-size:13.5px; margin-bottom:22px; }}
        table {{ width:100%; border-collapse:collapse; font-size:14px; border:1px solid #cbd5e1; }}
        th {{ text-align:left; padding:12px 14px; background:#0f2942; color:#ffffff; font-weight:700; }}
        tr:nth-child(even) {{ background:#f8fafc; }}
        tr:hover {{ background:#eff6ff; }}
        td {{ padding:14px; border-bottom:1px solid #e2e8f0; }}
        .rep-link {{ color:#003594; font-weight:800; text-decoration:none; font-size:15px; }}
        .rep-link:hover {{ text-decoration:underline; }}
        .st-badge {{ padding:4px 10px; border-radius:4px; font-size:12px; font-weight:800; display:inline-block; }}
        .st-badge.acked {{ background:#dcfce7; color:#14532d; border:1px solid #16a34a; }}
        .st-badge.unacked {{ background:#ffe4e6; color:#881337; border:1px solid #e11d48; }}
        .act-btn {{ cursor:pointer; padding:6px 12px; border-radius:6px; border:1px solid #cbd5e1; background:#f8fafc; color:#0f172a; text-decoration:none; font-size:12.5px; font-weight:700; margin-right:6px; }}
        .open-btn {{ background:#003594; border-color:#003594; color:#ffffff; }}
    </style>
</head>
<body>
    <div class="container">
        <h1><span class="econ-tag">经济学人</span><span class="gs-tag">高盛 / 大摩 量化图表研报</span> 每日 A股 / 美股 深度投研归档总控台</h1>
        <div class="sub">
            所有数据与报告按 <code>年/月/日</code> 归档于 <code>D:\\code\\economist\\reports\\</code>（零 C 盘占用）。
            当前共归档 <strong>{len(reports)}</strong> 份报告，其中待审阅 (未读) <strong>{unacked_count}</strong> 份。每天 08:00 自动执行分析，09:00 自动在浏览器打开当日及所有未确认报告。
        </div>
        <table>
            <thead>
                <tr>
                    <th>归档日期</th>
                    <th>研报总览标题 & 归档路径</th>
                    <th>覆盖规模</th>
                    <th>生成时间</th>
                    <th>审阅状态</th>
                    <th>操作</th>
                </tr>
            </thead>
            <tbody>
                {"".join(rows_html)}
            </tbody>
        </table>
    </div>
    <script>
        const ACK_API_BASE = "http://{ACK_SERVER_HOST}:{ACK_SERVER_PORT}";
        if (window.location.protocol.startsWith("http")) {{
            document.querySelectorAll("[data-http-href]").forEach(el => el.setAttribute("href", el.getAttribute("data-http-href")));
        }}
        async function toggleArchiveAck(reportId, acked) {{
            try {{
                await fetch(ACK_API_BASE + "/api/ack", {{
                    method: "POST",
                    headers: {{ "Content-Type": "application/json" }},
                    body: JSON.stringify({{ report_id: reportId, acked: acked }})
                }});
                window.location.reload();
            }} catch (e) {{
                alert("请确认本地 ACK 服务 (127.0.0.1:{ACK_SERVER_PORT}) 已启动");
            }}
        }}
    </script>
</body>
</html>
"""
    idx_path = REPORTS_DIR / "index.html"
    idx_path.write_text(index_html, encoding="utf-8")
    return idx_path
