# -*- coding: utf-8 -*-
"""HTML body & bootstrap JSON payload builder for daily reports."""

import json
from config import ACK_SERVER_HOST, ACK_SERVER_PORT
from gs_ms_tables import localize_zh_text
from .utils import _esc, _fmt_pct_badge, _short_sentence

def build_daily_body_and_vars(ns: dict) -> str:
    report_date = ns['report_date']
    window_desc = ns['window_desc']
    generated_at = ns['generated_at']
    kline_map = ns['kline_map']
    core_articles = ns['core_articles']
    digest_articles = ns['digest_articles']
    unacked_banner_html = ns['unacked_banner_html']
    toc_links_html = ns['toc_links_html']
    archive_items_html = ns['archive_items_html']
    headline_zh = ns['headline_zh']
    us_score = ns['us_score']
    cn_score = ns['cn_score']
    spy_k = ns['spy_k']
    qqq_k = ns['qqq_k']
    sh_k = ns['sh_k']
    cyb_k = ns['cyb_k']
    exec_cards_html = ns['exec_cards_html']
    kline_tabs_html = ns['kline_tabs_html']
    backtest_summary = ns['backtest_summary']
    bt_records = ns['bt_records']
    bt_hit_count = ns['bt_hit_count']
    bt_miss_count = ns['bt_miss_count']
    backtest_rows_html = ns['backtest_rows_html']
    sector_rows_html = ns['sector_rows_html']
    master_stock_rows_html = ns['master_stock_rows_html']
    pair_trade_rows_html = ns['pair_trade_rows_html']
    article_cards_html = ns['article_cards_html']
    digest_rows_html = ns['digest_rows_html']
    initial_ack_snapshot = ns['initial_ack_snapshot']
    stock_meta_by_sym = ns['stock_meta_by_sym']
    sector_chart_categories = ns['sector_chart_categories']
    sector_us_scores = ns['sector_us_scores']
    sector_cn_scores = ns['sector_cn_scores']
    sector_real_5d = ns['sector_real_5d']
    diverging_items = ns['diverging_items']
    scatter_points = ns['scatter_points']
    per_article_chart_configs = ns['per_article_chart_configs']
    bt_chart_payload = ns['bt_chart_payload']
    return f"""
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
                            <div class="p-name">🏆 报告后样本外多空收益差</div>
                            <div class="p-val" style="color:#d97706;">{backtest_summary['long_short_spread_5d']:+.2f}%</div>
                            <div style="font-size:12px;color:var(--text-muted);">多头均涨跌 {backtest_summary['avg_long_ret_5d']:+.2f}% vs 空头 {backtest_summary['avg_short_ret_5d']:+.2f}%</div>
                        </div>
                    </div>
                    <div class="pulse-card" style="border-top-color:#e11d48;">
                        <div class="pulse-left">
                            <div class="p-name">🌐 分市场准确率 (美股 vs A/港)</div>
                            <div class="p-val">{backtest_summary.get('us_hit_rate', 0.0)}% / {backtest_summary.get('cn_hit_rate', 0.0)}%</div>
                            <div style="font-size:12px;color:var(--text-muted);">{_esc(backtest_summary.get('mkt_leader_note', ''))}</div>
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
                                <th style="width:95px;" class="td-center">报告后实盘</th>
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
"""
