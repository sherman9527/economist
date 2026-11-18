# -*- coding: utf-8 -*-
"""Single-window multi-page Pivot workspace renderer (./reports/index.html)."""

import json
from pathlib import Path
from ack_server import load_ack_state
from config import ACK_SERVER_HOST, ACK_SERVER_PORT, DATA_DIR, REPORTS_DIR
from .archive_assets import ARCHIVE_CSS
from .archive_js import ARCHIVE_JS

def render_archive_index() -> Path:
    """
    Render ./reports/index.html as a Single-Window Multi-Page Workspace
    (Fluent/RADAR-style top Pivot Pages: Today + All Un-ACKed Reports + Cross-Report Audit + Archive Library).
    Eliminates multi-window browser popups while keeping every un-ACKed report immediately accessible in-page.
    """
    state = load_ack_state()
    reports_map = state.get("reports", {})
    reports = sorted(
        reports_map.values(),
        key=lambda x: x.get("date", x.get("report_id", "")),
        reverse=True,
    )
    unacked_count = sum(1 for r in reports if not r.get("acked", False))

    audit_data = None
    audit_files = sorted(DATA_DIR.glob("full_retrospective_audit_*.json"), reverse=True)
    if audit_files:
        try:
            audit_data = json.loads(audit_files[0].read_text(encoding="utf-8"))
        except Exception:
            audit_data = None

    initial_reports_json = json.dumps(reports_map, ensure_ascii=False)
    initial_audit_json = json.dumps(audit_data, ensure_ascii=False)

    body_html = f"""</head>
<body>
    <div class="ws-shell">
        <!-- Row 1: Single-Window Command Header -->
        <header class="ws-topbar">
            <div class="ws-brand">
                <span class="econ-logo">经济学人</span>
                <span class="gs-logo">高盛 / 大摩 单窗口多页终端</span>
                <span class="ws-title" id="wsActiveTitle">正在加载页内投研工作台...</span>
            </div>
            <div class="ws-controls">
                <span id="unackedQueuePill" class="queue-pill has-unread">🔴 待审阅: {unacked_count} 份</span>
                <button class="cmd-btn" onclick="stepReportTab(-1)" title="上一页签 (Alt + ←)">◀ 上一页</button>
                <button class="cmd-btn" onclick="stepReportTab(1)" title="下一页签 (Alt + →)">下一页 ▶</button>
                <button class="cmd-btn" id="nextUnreadBtn" onclick="jumpNextUnreadTab()" title="直接跳至下一份未读报告页签">⏭ 下一份未读</button>
                <button class="cmd-btn ack-primary" id="activePageAckBtn" onclick="toggleActiveReportAck()" title="快捷键 Alt + A">✅ 确认本页已阅</button>
                <button class="cmd-btn ack-batch" id="batchAckAllBtn" onclick="batchAckAllUnread()">✅ 全部一键已阅</button>
                <button class="cmd-btn" id="wsThemeBtn" onclick="toggleWorkspaceTheme()">🌙 暗色终端</button>
            </div>
        </header>

        <!-- Row 2: In-Page Pivot Tabs (Report Pages + System Audit & Archive Pages) -->
        <nav class="ws-pivot-bar" aria-label="页内报告页签栏">
            <div id="reportTabsStrip" class="pivot-reports-scroll"></div>
            <div id="systemTabsStrip" class="pivot-sys-group">
                <div class="pivot-tab sys-tab" data-page-id="__audit__" onclick="activatePage('__audit__')">
                    🎯 跨期回溯总审计
                </div>
                <div class="pivot-tab sys-tab" data-page-id="__archive__" onclick="activatePage('__archive__')">
                    🗂️ 历史归档日历库 (<span id="totalArchiveCount">{len(reports)}</span>)
                </div>
            </div>
        </nav>

        <!-- Full-Height In-Page Content Stage -->
        <main class="ws-stage" id="wsStage">
            <!-- Dynamic Report Iframe Pages are mounted here lazily and kept alive -->

            <!-- System Page 1: Cross-Report Out-of-Sample Retrospective Accuracy Audit -->
            <section class="ws-page scrollable-sys-page" id="ws-page-__audit__">
                <div class="sys-container" id="auditPageContainer"></div>
            </section>

            <!-- System Page 2: All Archived Reports Directory (Opens In-Page Tabs, Zero New Windows) -->
            <section class="ws-page scrollable-sys-page" id="ws-page-__archive__">
                <div class="sys-container">
                    <div class="sys-card">
                        <div class="sys-hdr">
                            <div>
                                <div class="sys-title">🗂️ 《经济学人》A股 / 美股 深度图表研报 · 全量按年月日归档总库</div>
                                <div class="sys-sub">
                                    存储目录：<code>./reports/YYYY/MM/DD/</code>（严格零 C 盘占用）· 点击任意报告均在<strong>当前窗口页内页签 (In-Page Tab)</strong> 秒切打开，绝不弹出多余浏览器窗口。
                                </div>
                            </div>
                        </div>
                        <div class="filter-row">
                            <div class="flt-pills">
                                <button class="flt-pill active" onclick="setArchiveFilter('all', this)">全部归档报告</button>
                                <button class="flt-pill" onclick="setArchiveFilter('unacked', this)">🔴 仅看待审阅 (未读)</button>
                                <button class="flt-pill" onclick="setArchiveFilter('acked', this)">✅ 已确认审阅</button>
                            </div>
                            <input type="text" class="search-box" id="archiveSearchInput" placeholder="🔍 按日期 (2026-11-16) 或标题关键词筛选..." oninput="renderArchiveTable()">
                        </div>
                        <div class="ws-table-wrap">
                            <table class="ws-table">
                                <thead>
                                    <tr>
                                        <th style="width:125px;">归档日期</th>
                                        <th>研报核心标题 & 本地归档路径</th>
                                        <th style="width:185px;">图表与研报卡规模</th>
                                        <th style="width:165px;">生成时间</th>
                                        <th style="width:190px;">审阅确认状态 (ACK)</th>
                                        <th style="width:230px;">页内操作 (不弹新窗口)</th>
                                    </tr>
                                </thead>
                                <tbody id="archiveTableBody"></tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </section>
        </main>
    </div>

    <script>
        const ACK_API_BASE = "http://{ACK_SERVER_HOST}:{ACK_SERVER_PORT}";
        let reportsMap = {initial_reports_json};
        let auditData = {initial_audit_json};
        let openTabIds = [];"""
    index_html = (
        "<!DOCTYPE html>\n<html lang=\"zh-CN\" data-theme=\"light\">\n<head>\n"
        "    <meta charset=\"UTF-8\">\n"
        "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n"
        f"    <title>《经济学人》A股/美股 投研单窗口多页工作台 (共{len(reports)}期 · 待阅{unacked_count}期)</title>\n"
        + ARCHIVE_CSS + "\n"
        + body_html + "\n"
        + ARCHIVE_JS + "\n    </script>\n</body>\n</html>\n"
    )
    idx_path = REPORTS_DIR / "index.html"
    idx_path.write_text(index_html, encoding="utf-8")
    return idx_path
