# -*- coding: utf-8 -*-
"""Static CSS & early embed detector for daily reports."""

DAILY_HEAD_ASSETS = r"""    <style>
        :root, html[data-theme="light"] {
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
        }

        html[data-theme="dark"] {
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
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }
        html {
            scroll-behavior: smooth;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
            background: var(--bg-page); color: var(--text-main);
            font-size: 14px; line-height: 1.6;
            font-variant-numeric: tabular-nums;
        }
        .font-mono { font-family: "JetBrains Mono", Consolas, monospace; }

        /* Sticky Top Masthead */
        .top-masthead {
            position: sticky; top: 0; z-index: 1000;
            background: #0a192f; color: #ffffff;
            border-bottom: 3px solid var(--econ-red);
            padding: 10px 24px;
            display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px;
            box-shadow: 0 4px 14px rgba(0,0,0,0.2);
        }
        .mast-left { display: flex; align-items: center; gap: 12px; }
        .econ-badge {
            background: var(--econ-red); color: #fff; font-weight: 800; font-size: 12px;
            padding: 5px 10px; border-radius: 3px; text-transform: uppercase;
        }
        .gs-ms-tag {
            background: #1e3a8a; border: 1px solid #3b82f6; color: #dbeafe;
            font-size: 11.5px; font-weight: 700; padding: 4px 9px; border-radius: 3px;
        }
        .mast-main-title { font-size: 15.5px; font-weight: 800; color: #fff; }
        .mast-sub { font-size: 11.5px; color: #94a3b8; display: flex; gap: 12px; flex-wrap: wrap; }

        .mast-right { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
        .ack-status-pill { font-size: 12px; padding: 4px 11px; border-radius: 999px; font-weight: 700; }
        .ack-status-pill.unacked { background: rgba(244, 63, 94, 0.25); color: #fecdd3; border: 1px solid #f43f5e; }
        .ack-status-pill.acked { background: rgba(16, 185, 129, 0.25); color: #a7f3d0; border: 1px solid #10b981; }

        .ack-btn {
            cursor: pointer; border: none; border-radius: 6px; padding: 7px 15px;
            font-size: 13px; font-weight: 800; transition: all 0.18s;
        }
        .ack-btn.btn-to-ack { background: #10b981; color: #052e16; }
        .ack-btn.btn-acked { background: #1e293b; color: #6ee7b7; border: 1px solid #10b981; }
        .util-btn {
            cursor: pointer; background: #1e293b; color: #e2e8f0; border: 1px solid #475569;
            padding: 6px 11px; border-radius: 6px; font-size: 12px; font-weight: 600; text-decoration: none;
        }

        /* Un-ACKed Alert Banner */
        .unacked-banner {
            background: #fffbeb; color: #92400e; border-bottom: 2px solid #f59e0b;
            padding: 8px 24px; display: none; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px;
        }
        html[data-theme="dark"] .unacked-banner { background: #451a03; color: #fde68a; }
        .unacked-banner.visible { display: flex; }
        .unacked-list { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
        .unacked-chip {
            background: var(--bg-surface); border: 1px solid #d97706; color: var(--text-main);
            padding: 3px 10px; border-radius: 6px; font-size: 12px; font-weight: 700;
            display: inline-flex; align-items: center; gap: 6px;
        }

        /* Main Page Grid */
        .page-grid {
            display: grid; grid-template-columns: 255px 1fr; gap: 20px;
            max-width: 1760px; width: 100%; margin: 0 auto; padding: 20px;
        }
        @media (max-width: 1180px) {
            .page-grid { grid-template-columns: 1fr; }
            .sidebar-toc { position: static !important; height: auto !important; }
        }

        /* FIXED Sidebar TOC — Block Layout so items NEVER squish or clip! */
        .sidebar-toc {
            position: sticky; top: 72px; height: calc(100vh - 90px);
            overflow-y: auto; background: var(--bg-surface);
            border: 1px solid var(--border-color); border-radius: 10px; padding: 14px;
            box-shadow: var(--shadow-card);
        }
        .toc-hdr {
            font-size: 12px; font-weight: 800; color: var(--gs-blue);
            text-transform: uppercase; letter-spacing: 0.6px;
            border-bottom: 2px solid var(--gs-blue); padding-bottom: 6px; margin-bottom: 8px;
        }
        .toc-nav { display: block; margin-bottom: 14px; }
        .toc-link {
            display: block; color: var(--text-main); text-decoration: none;
            font-size: 13px; font-weight: 700; line-height: 1.45;
            padding: 7px 9px; margin-bottom: 3px; border-radius: 6px;
        }
        .toc-link:hover { background: var(--bg-row-hover); color: var(--gs-accent); }
        .toc-sublinks {
            display: block; padding-left: 8px; margin: 4px 0 8px 6px;
            border-left: 2px solid var(--border-color);
            max-height: 280px; overflow-y: auto;
        }
        .toc-sublink {
            display: block; flex-shrink: 0;
            height: 28px; line-height: 28px; padding: 0 8px; margin-bottom: 2px;
            color: var(--text-secondary); text-decoration: none; font-size: 12px;
            border-radius: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
        }
        .toc-sublink:hover { background: var(--bg-row-hover); color: var(--gs-accent); font-weight: 700; }
        .toc-num { color: var(--gs-accent); font-weight: 800; font-family: monospace; margin-right: 4px; }

        .archive-list { display: block; }
        .arch-item {
            display: block; padding: 7px 10px; margin-bottom: 6px; border-radius: 6px;
            background: var(--bg-subtle); border: 1px solid var(--border-color);
            text-decoration: none; color: var(--text-main); font-size: 12px;
        }
        .arch-item.current { border: 2px solid var(--gs-accent); background: var(--bg-row-hover); }
        .arch-item.unacked { border-left: 4px solid #e11d48; }
        .arch-item.acked { border-left: 4px solid #10b981; }
        .arch-row-top { display: flex; justify-content: space-between; align-items: center; font-weight: 700; }
        .arch-meta { font-size: 11px; color: var(--text-muted); margin-top: 2px; }

        /* Main Sections */
        .main-col { display: flex; flex-direction: column; gap: 22px; min-width: 0; }
        .report-section {
            background: var(--bg-surface); border: 1px solid var(--border-color);
            border-radius: 10px; padding: 20px 22px; box-shadow: var(--shadow-card);
            scroll-margin-top: 76px;
        }
        .sec-header {
            display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px;
            border-bottom: 2px solid var(--bg-table-head); padding-bottom: 10px; margin-bottom: 16px;
        }
        .sec-title { font-size: 17.5px; font-weight: 800; color: var(--text-main); display: flex; align-items: center; gap: 8px; }
        .sec-tag-pill {
            font-size: 11.5px; font-weight: 700; padding: 2px 9px; border-radius: 4px;
            background: var(--bg-subtle); color: var(--gs-blue); border: 1px solid var(--border-color);
        }

        /* Top Market Pulse Strip + 4 Concise Takeaway Cards */
        .market-pulse-strip {
            display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 12px; margin-bottom: 16px;
        }
        .pulse-card {
            background: var(--bg-subtle); border: 1px solid var(--border-color);
            border-top: 3px solid var(--gs-blue); border-radius: 8px; padding: 11px 14px;
            display: flex; justify-content: space-between; align-items: center; cursor: pointer;
        }
        .pulse-card:hover { border-color: var(--gs-accent); background: var(--bg-row-hover); }
        .pulse-left .p-name { font-size: 12px; font-weight: 700; color: var(--text-muted); }
        .pulse-left .p-val { font-size: 18px; font-weight: 800; font-family: monospace; color: var(--text-main); }
        .pulse-right { text-align: right; }

        .exec-4cards-grid {
            display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 12px; margin-bottom: 10px;
        }
        .exec-mini-card {
            background: var(--bg-subtle); border: 1px solid var(--border-color);
            border-left: 4px solid var(--econ-red); border-radius: 6px; padding: 10px 14px;
        }
        .emc-tag { font-size: 11.5px; font-weight: 800; color: var(--gs-blue); margin-bottom: 3px; text-transform: uppercase; }
        .emc-txt { font-size: 13px; color: var(--text-main); line-height: 1.5; }

        /* Main Interactive K-Line Terminal Layout */
        .kline-Selector-groups { display: flex; flex-direction: column; gap: 8px; margin-bottom: 14px; background: var(--bg-subtle); padding: 12px; border-radius: 8px; border: 1px solid var(--border-color); }
        .kgroup-row { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
        .kgroup-label { font-size: 12px; font-weight: 800; color: var(--gs-blue); min-width: 105px; }
        .kline-tab-btn {
            cursor: pointer; background: var(--bg-surface); border: 1px solid var(--border-color);
            border-radius: 5px; padding: 4px 9px; font-size: 12px; display: inline-flex; align-items: center; gap: 5px;
            color: var(--text-main); transition: all 0.15s;
        }
        .kline-tab-btn:hover, .kline-tab-btn.active { background: var(--gs-blue); color: #fff !important; border-color: var(--gs-blue); }
        .kline-tab-btn.active .kbtn-pct, .kline-tab-btn.active .kbtn-code { color: #fff !important; }
        .kbtn-name { font-weight: 700; }
        .kbtn-code { font-size: 11px; color: var(--text-muted); font-family: monospace; }
        .kbtn-pct { font-size: 11px; font-weight: 800; font-family: monospace; }
        .kbtn-up .kbtn-pct { color: #16a34a; }
        .kbtn-down .kbtn-pct { color: #e11d48; }

        .kline-terminal-split { display: grid; grid-template-columns: 1fr 330px; gap: 16px; align-items: stretch; }
        @media (max-width: 1100px) { .kline-terminal-split { grid-template-columns: 1fr; } }
        .main-echart-canvas { width: 100%; height: 420px; border: 1px solid var(--border-color); border-radius: 8px; background: var(--bg-surface); padding: 8px; }
        .kline-quant-sidecard {
            background: var(--bg-subtle); border: 1px solid var(--border-color); border-radius: 8px; padding: 16px;
            display: flex; flex-direction: column; justify-content: space-between; gap: 12px;
        }

        /* 2x2 Visual Chart Grid (LineChart, ColumnChart, BarChart, ScatterChart) */
        .charts-2x2-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 18px; }
        @media (max-width: 1050px) { .charts-2x2-grid { grid-template-columns: 1fr; } }
        .chart-panel-card {
            background: var(--bg-subtle); border: 1px solid var(--border-color); border-radius: 8px; padding: 14px;
        }
        .chart-panel-title { font-size: 14px; font-weight: 800; color: var(--text-main); margin-bottom: 4px; display: flex; justify-content: space-between; align-items: center; }
        .chart-panel-sub { font-size: 12px; color: var(--text-muted); margin-bottom: 8px; }
        .mid-echart-canvas { width: 100%; height: 320px; background: var(--bg-surface); border-radius: 6px; border: 1px solid var(--border-light); padding: 6px; }

        /* Tables */
        .table-scroll { overflow-x: auto; border: 1px solid var(--border-color); border-radius: 8px; }
        .gs-table { width: 100%; border-collapse: collapse; font-size: 13px; background: var(--bg-surface); }
        .gs-table thead th {
            background: var(--bg-table-head); color: var(--text-table-head);
            font-weight: 700; text-align: left; padding: 9px 12px; white-space: nowrap; font-size: 12.5px;
        }
        .gs-table tbody tr:nth-child(even) { background: var(--bg-row-even); }
        .gs-table tbody tr:hover { background: var(--bg-row-hover); }
        .gs-table td { padding: 9px 12px; border-bottom: 1px solid var(--border-light); vertical-align: middle; color: var(--text-main); }
        .td-center { text-align: center !important; }
        .td-right { text-align: right !important; }

        /* Badges & Pills */
        .gs-badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 11.5px; font-weight: 800; white-space: nowrap; }
        .gs-rating-cbuy { background: var(--cbuy-bg); color: var(--cbuy-text); border: 1px solid var(--cbuy-border); }
        .gs-rating-ow { background: var(--ow-bg); color: var(--ow-text); border: 1px solid var(--ow-border); }
        .gs-rating-ew { background: var(--ew-bg); color: var(--ew-text); border: 1px solid var(--ew-border); }
        .gs-rating-uw { background: var(--uw-bg); color: var(--uw-text); border: 1px solid var(--uw-border); }
        .gs-rating-uw-strong { background: var(--uws-bg); color: var(--uws-text); border: 1px solid var(--uws-border); }
        .bt-hit-strong { background: var(--cbuy-bg); color: var(--cbuy-text); border: 1px solid var(--cbuy-border); }
        .bt-hit { background: var(--ow-bg); color: var(--ow-text); border: 1px solid var(--ow-border); }
        .bt-diverge { background: #fef3c7; color: #92400e; border: 1px solid #f59e0b; }
        .bt-miss { background: var(--uw-bg); color: var(--uw-text); border: 1px solid var(--uw-border); }

        .pct-pill { display: inline-block; padding: 2px 7px; border-radius: 4px; font-size: 12px; font-weight: 800; font-family: monospace; white-space: nowrap; }
        .pct-up { background: rgba(22, 163, 74, 0.14); color: #15803d; }
        .pct-down { background: rgba(225, 29, 72, 0.14); color: #be123c; }
        .pct-flat { background: var(--bg-subtle); color: var(--text-muted); }
        html[data-theme="dark"] .pct-up { color: #4ade80; }
        html[data-theme="dark"] .pct-down { color: #fb7185; }

        .mkt-pill { display: inline-block; padding: 2px 6px; border-radius: 4px; font-size: 11px; font-weight: 800; }
        .mkt-us { background: rgba(29, 78, 216, 0.12); color: var(--gs-accent); }
        .mkt-cn { background: rgba(225, 29, 72, 0.12); color: #e11d48; }
        .cat-chip, .driver-chip, .idx-pill {
            display: inline-block; padding: 2px 7px; border-radius: 4px; font-size: 11.5px; font-weight: 700;
            background: var(--bg-subtle); color: var(--text-secondary); border: 1px solid var(--border-color); white-space: nowrap;
        }
        .mini-k-btn {
            cursor: pointer; background: rgba(29, 78, 216, 0.1); color: var(--gs-accent);
            border: 1px solid rgba(29, 78, 216, 0.3); border-radius: 4px; padding: 1px 6px; font-size: 11px; font-weight: 700; margin-left: 4px;
        }
        .mini-k-btn:hover { background: var(--gs-accent); color: #fff; }
        .tag-ow { background: #dcfce7; color: #14532d; font-size: 11px; font-weight: 800; padding: 1px 5px; border-radius: 3px; margin-right: 4px; }
        .tag-uw { background: #ffe4e6; color: #881337; font-size: 11px; font-weight: 800; padding: 1px 5px; border-radius: 3px; margin-right: 4px; }
        .compact-pick { font-size: 12.5px; margin-bottom: 3px; }
        .sub-links { margin-top: 3px; }
        .mini-jump, .jump-link { color: var(--gs-accent); font-weight: 700; text-decoration: none; font-size: 12px; margin-right: 4px; }

        /* Filter Toolbar */
        .filter-toolbar {
            display: flex; flex-wrap: wrap; gap: 8px; align-items: center; justify-content: space-between;
            background: var(--bg-subtle); border: 1px solid var(--border-color); padding: 10px 14px; border-radius: 8px; margin-bottom: 12px;
        }
        .filter-group { display: flex; flex-wrap: wrap; gap: 6px; }
        .flt-btn {
            cursor: pointer; background: var(--bg-surface); color: var(--text-secondary);
            border: 1px solid var(--border-color); padding: 5px 11px; border-radius: 5px; font-size: 12px; font-weight: 700;
        }
        .flt-btn.active, .flt-btn:hover { background: var(--gs-blue); color: #fff; border-color: var(--gs-blue); }
        .flt-search {
            background: var(--bg-surface); border: 1px solid var(--border-color); color: var(--text-main);
            padding: 5px 11px; border-radius: 5px; font-size: 12.5px; min-width: 220px;
        }

        /* Per-Article 2-Column Visual Research Card */
        .notes-stack { display: flex; flex-direction: column; gap: 20px; }
        .gs-note-card {
            background: var(--bg-surface); border: 1px solid var(--border-color);
            border-top: 4px solid var(--bg-table-head); border-radius: 10px; padding: 18px;
            scroll-margin-top: 82px;
        }
        .gs-note-card.highlight-flash { border-color: var(--gs-accent); box-shadow: 0 0 0 3px rgba(29, 78, 216, 0.25); }
        .note-header { border-bottom: 1px solid var(--border-light); padding-bottom: 10px; margin-bottom: 12px; }
        .note-meta-bar { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 6px; font-size: 12px; color: var(--text-muted); }
        .note-idx { font-family: monospace; font-weight: 800; color: var(--gs-blue); }
        .note-actions-right { margin-left: auto; display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
        .ext-btn { background: var(--bg-subtle); border: 1px solid var(--border-color); color: var(--gs-accent); padding: 2px 8px; border-radius: 4px; font-size: 11.5px; font-weight: 700; text-decoration: none; }
        .note-zh-title { font-size: 17.5px; font-weight: 800; color: var(--text-main); }

        /* 4-Stage Visual Flow Strip */
        .flow-strip-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; margin-bottom: 14px; }
        @media (max-width: 960px) { .flow-strip-grid { grid-template-columns: 1fr 1fr; } }
        .flow-step-card { background: var(--bg-subtle); border: 1px solid var(--border-light); border-left: 3px solid var(--gs-blue); border-radius: 6px; padding: 8px 10px; }
        .flow-step-hdr { font-size: 11px; font-weight: 800; color: var(--gs-blue); margin-bottom: 2px; }
        .flow-step-txt { font-size: 12.2px; color: var(--text-secondary); line-height: 1.45; }

        /* 2-Column Split inside Article Card */
        .note-visual-split { display: grid; grid-template-columns: 1.15fr 0.85fr; gap: 16px; align-items: start; }
        @media (max-width: 1100px) { .note-visual-split { grid-template-columns: 1fr; } }
        .takeaway-compact-box { background: var(--bg-subtle); border: 1px solid var(--border-light); border-radius: 6px; padding: 10px 14px; margin-bottom: 12px; }
        .box-hdr { font-size: 12px; font-weight: 800; color: var(--gs-blue); margin-bottom: 6px; }
        .crisp-takeaway-list { list-style: none; display: flex; flex-direction: column; gap: 5px; font-size: 13px; }
        .b-tag { background: rgba(0, 53, 148, 0.1); color: var(--gs-blue); font-weight: 800; font-size: 11.5px; padding: 1px 6px; border-radius: 3px; margin-right: 6px; }

        .note-right-chart-col { background: var(--bg-subtle); border: 1px solid var(--border-color); border-radius: 8px; padding: 10px; }
        .art-chart-toolbar { display: flex; flex-wrap: wrap; gap: 5px; margin-bottom: 8px; }
        .art-chart-tab {
            cursor: pointer; background: var(--bg-surface); border: 1px solid var(--border-color);
            color: var(--text-secondary); padding: 4px 8px; border-radius: 4px; font-size: 11.5px; font-weight: 700;
        }
        .art-chart-tab.active, .art-chart-tab:hover { background: var(--gs-blue); color: #fff; border-color: var(--gs-blue); }
        .art-echart-box { width: 100%; height: 270px; background: var(--bg-surface); border-radius: 6px; }

        .gs-deep-drawer { margin-top: 12px; background: var(--bg-subtle); border: 1px solid var(--border-light); border-radius: 6px; padding: 8px 14px; font-size: 12.5px; }
        .gs-deep-drawer summary { cursor: pointer; font-weight: 700; color: var(--gs-accent); }
        .drawer-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border-light); line-height: 1.65; color: var(--text-secondary); }
        @media (max-width: 900px) { .drawer-grid { grid-template-columns: 1fr; } }
        .drawer-block h4 { color: var(--text-main); font-size: 13px; margin-bottom: 4px; }

        .m-zh-title { font-weight: 800; color: var(--text-main); font-size: 13.5px; }
        .m-en-title { font-size: 11.5px; color: var(--text-muted); }
        .m-tickers { font-size: 11.5px; color: var(--text-secondary); margin-top: 2px; font-weight: 600; }
        .matrix-row { cursor: pointer; }

        .floating-nav { position: fixed; bottom: 20px; right: 20px; z-index: 990; display: flex; flex-direction: column; gap: 6px; }
        .float-pill { background: #0a192f; color: #fff; border: 1px solid #334155; padding: 7px 12px; border-radius: 999px; font-size: 12px; font-weight: 700; text-decoration: none; text-align: center; box-shadow: 0 4px 12px rgba(0,0,0,0.25); }
        .float-pill:hover { background: var(--gs-accent); }

        /* Embedded In-Page Tab Mode (?embed=1 inside reports/index.html single-window workspace) */
        html[data-embed="1"] .unacked-banner,
        html[data-embed="1"] #archivePortalLink,
        html[data-embed="1"] #themeToggleBtn {
            display: none !important;
        }
        html[data-embed="1"] .top-masthead {
            position: relative;
            background: var(--bg-surface);
            color: var(--text-main);
            border-bottom: 2px solid var(--border-color);
            box-shadow: none;
            padding: 10px 22px;
        }
        html[data-embed="1"] .mast-main-title {
            color: var(--text-main);
            font-size: 15px;
        }
        html[data-embed="1"] .mast-sub {
            color: var(--text-muted);
        }
        html[data-embed="1"] .ack-status-pill.unacked {
            background: var(--bear-bg); color: var(--bear-text); border-color: var(--bear-border);
        }
        html[data-embed="1"] .ack-status-pill.acked {
            background: var(--bull-bg); color: var(--bull-text); border-color: var(--bull-border);
        }
        html[data-embed="1"] .sidebar-toc {
            top: 14px;
            height: calc(100vh - 28px);
        }
        html[data-embed="1"] .report-section {
            scroll-margin-top: 16px;
        }
    </style>
    <script>
        (function detectEmbedEarly() {
            try {
                const isEmbedParam = new URLSearchParams(window.location.search).get("embed") === "1";
                const isFramed = window.self !== window.top;
                if (isEmbedParam || isFramed) {
                    document.documentElement.setAttribute("data-embed", "1");
                }
            } catch (e) {
                document.documentElement.setAttribute("data-embed", "1");
            }
        })();
    </script>"""
