# -*- coding: utf-8 -*-
"""Daily visual report data preparation and HTML document assembly."""

from datetime import datetime
import json
from pathlib import Path
from ack_server import load_ack_state, register_report
from backtester import evaluate_backtest_and_attribution
from config import REPORTS_DIR
from gs_ms_tables import (
    build_sector_scorecard_rows,
    extract_all_structured_stocks,
    localize_zh_text,
    parse_ticker_entry,
    split_to_bullets,
)
from market_data import fetch_market_klines_for_report, resolve_qt_code
from .archive_index import render_archive_index
from .daily_body import build_daily_body_and_vars
from .daily_charts_js import DAILY_CHARTS_JS
from .daily_head_assets import DAILY_HEAD_ASSETS
from .daily_ui_js import DAILY_UI_JS
from .utils import (
    _build_svg_sparkline,
    _clean_sec_labels_zh,
    _dir_filter_tag,
    _dir_to_gs_badge,
    _esc,
    _fmt_pct_badge,
    _short_sentence,
)

def render_daily_report(analysis_data: dict, report_date: str) -> Path:
    """
    Render a visual, ECharts-driven Goldman Sachs / Morgan Stanley HTML report
    archived under ./reports/YYYY/MM/DD/.
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

        # Parse stocks for this article and attach kline (excluding explicit Pair Trade lines)
        art_stocks = []
        for t_str in us_m.get("tickers") or []:
            row = parse_ticker_entry(t_str, "US", us_dir, us_sc, idx, zh_title, cat)
            if row.get("is_pair_trade"):
                continue
            res = resolve_qt_code(row["head_display"], "US")
            if res and res[0] in kline_map:
                row["resolved_symbol"] = res[0]
                row["zh_short_name"] = res[2]
                row["kline"] = kline_map[res[0]]
            art_stocks.append(row)
        for t_str in cn_m.get("tickers") or []:
            row = parse_ticker_entry(t_str, "CN", cn_dir, cn_sc, idx, zh_title, cat)
            if row.get("is_pair_trade"):
                continue
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
    bt_hit_count = sum(1 for r in bt_records if r["is_hit"] and not r.get("is_pending"))
    bt_miss_count = sum(1 for r in bt_records if (not r["is_hit"]) and not r.get("is_pending"))
    backtest_rows_html = []
    for b_idx, rec in enumerate(bt_records, 1):
        sym = rec["symbol"]
        k_btn = f'<button class="mini-k-btn" onclick="jumpAndShowKline(\'{_esc(sym)}\')">📈 K线</button>'
        hide_style = 'style="display:none;"' if b_idx > 18 else ""
        hit_flag = "pending" if rec.get("is_pending") else ("hit" if rec["is_hit"] else "miss")
        win_tag = rec.get("eval_window_tag", "样本外")
        backtest_rows_html.append(
            f"""
            <tr class="bt-audit-row" {hide_style} data-hit="{hit_flag}" data-idx="{b_idx}">
                <td class="td-center"><span class="mkt-pill mkt-{rec['market_type'].lower()}">{_esc(rec["market_label"])}</span></td>
                <td>
                    <strong>{_esc(rec["head_display"][:22])}</strong> {k_btn}
                    <div style="font-size:11px;color:var(--text-muted);">{_esc(rec["theme_category"])} · <a href="{rec['article_anchor']}" onclick="jumpToArticle('art-{rec['article_idx']}');return false;">研报#{rec['article_idx']:02d}</a></div>
                </td>
                <td class="td-center"><span class="gs-badge {rec['badge_cls']}">{_esc(rec["rating"])}</span></td>
                <td class="td-center">{_fmt_pct_badge(rec["ret_5d"])}<div style="font-size:10.5px;color:var(--text-muted);margin-top:2px;">{_esc(win_tag)}</div></td>
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

    # Sidebar Archive Links (in-page navigation via workspace focus or postMessage)
    archive_items_html = []
    for rep in all_reports:
        rid = rep.get("report_id", "")
        r_acked = rep.get("acked", False)
        is_current = rid == report_date
        status_icon = "✅ 已审阅" if r_acked else "🔴 待审阅"
        cls = "arch-item current" if is_current else ("arch-item acked" if r_acked else "arch-item unacked")
        href = f"/reports/index.html?focus={rid}"
        file_href = f"../../../index.html?focus={rid}"
        archive_items_html.append(
            f"""
            <a class="{cls}" href="{file_href}" data-http-href="{href}" data-report-id="{_esc(rid)}" onclick="return handleInPageReportNav(event, '{_esc(rid)}')">
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

    body_and_vars = build_daily_body_and_vars(locals())
    html_doc = (
        "<!DOCTYPE html>\n<html lang=\"zh-CN\" data-theme=\"light\">\n<head>\n"
        "    <meta charset=\"UTF-8\">\n"
        "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n"
        f"    <title>{_esc(headline_zh)} | GS/MS 量化图表研报 ({_esc(report_date)})</title>\n"
        "    <script src=\"/reports/assets/echarts.min.js\" onerror=\"this.onerror=null;this.src='../../../assets/echarts.min.js';\"></script>\n"
        + DAILY_HEAD_ASSETS + "\n"
        + body_and_vars + "\n"
        + DAILY_CHARTS_JS + "\n"
        + DAILY_UI_JS + "\n    </script>\n</body>\n</html>\n"
    )
    out_path.write_text(html_doc, encoding="utf-8")
    render_archive_index()
    return out_path
