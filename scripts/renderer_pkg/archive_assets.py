# -*- coding: utf-8 -*-
"""Static CSS for single-window archive workspace."""

ARCHIVE_CSS = r"""    <style>
        :root, html[data-theme="light"] {
            --ws-bg: #f1f5f9;
            --ws-surface: #ffffff;
            --ws-subtle: #f8fafc;
            --ws-border: #cbd5e1;
            --ws-border-light: #e2e8f0;
            --ws-text: #0f172a;
            --ws-secondary: #334155;
            --ws-muted: #64748b;
            --ws-navy: #0a192f;
            --ws-tabbar: #0f2342;
            --ws-blue: #003594;
            --ws-accent: #2563eb;
            --ws-red: #e3120b;
            --ws-bull-bg: #dcfce7;
            --ws-bull-text: #14532d;
            --ws-bear-bg: #ffe4e6;
            --ws-bear-text: #881337;
            --ws-thead: #0f2942;
        }
        html[data-theme="dark"] {
            --ws-bg: #0b1120;
            --ws-surface: #111827;
            --ws-subtle: #1e293b;
            --ws-border: #334155;
            --ws-border-light: #1f2937;
            --ws-text: #f8fafc;
            --ws-secondary: #cbd5e1;
            --ws-muted: #94a3b8;
            --ws-navy: #050b14;
            --ws-tabbar: #0b1528;
            --ws-blue: #60a5fa;
            --ws-accent: #3b82f6;
            --ws-red: #ef4444;
            --ws-bull-bg: rgba(22, 163, 74, 0.22);
            --ws-bull-text: #86efac;
            --ws-bear-bg: rgba(225, 29, 72, 0.22);
            --ws-bear-text: #fda4af;
            --ws-thead: #1e293b;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        html, body {
            height: 100%; width: 100%; overflow: hidden;
            background: var(--ws-bg); color: var(--ws-text);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
            font-variant-numeric: tabular-nums;
        }
        .ws-shell {
            display: flex; flex-direction: column; height: 100vh; width: 100vw; overflow: hidden;
        }
        /* Row 1: Global Top Command Bar */
        .ws-topbar {
            height: 48px; flex-shrink: 0;
            background: var(--ws-navy); color: #ffffff;
            border-bottom: 1px solid rgba(148, 163, 184, 0.22);
            padding: 0 18px;
            display: flex; align-items: center; justify-content: space-between; gap: 12px;
        }
        .ws-brand { display: flex; align-items: center; gap: 10px; min-width: 0; }
        .econ-logo {
            background: #e3120b; color: #fff; font-weight: 800; font-size: 12px;
            padding: 4px 9px; border-radius: 3px; flex-shrink: 0;
        }
        .gs-logo {
            background: #1e3a8a; border: 1px solid #3b82f6; color: #dbeafe;
            font-size: 11.5px; font-weight: 700; padding: 3px 8px; border-radius: 3px; flex-shrink: 0;
        }
        .ws-title {
            font-size: 14.5px; font-weight: 800; color: #f8fafc;
            white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
        }
        .ws-controls { display: flex; align-items: center; gap: 7px; flex-shrink: 0; }
        .queue-pill {
            font-size: 12px; font-weight: 800; padding: 4px 10px; border-radius: 999px;
            border: 1px solid transparent; white-space: nowrap;
        }
        .queue-pill.has-unread { background: rgba(244, 63, 94, 0.22); color: #fecdd3; border-color: #f43f5e; }
        .queue-pill.all-clear { background: rgba(16, 185, 129, 0.22); color: #a7f3d0; border-color: #10b981; }

        .cmd-btn {
            cursor: pointer; background: #1e293b; color: #e2e8f0; border: 1px solid #475569;
            padding: 5px 10px; border-radius: 5px; font-size: 12px; font-weight: 700;
            display: inline-flex; align-items: center; gap: 5px; white-space: nowrap;
            transition: all 0.15s;
        }
        .cmd-btn:hover { background: #334155; border-color: #64748b; color: #fff; }
        .cmd-btn.ack-primary { background: #10b981; color: #052e16; border-color: #10b981; font-weight: 800; }
        .cmd-btn.ack-primary:hover { background: #34d399; }
        .cmd-btn.ack-done { background: #1e293b; color: #6ee7b7; border-color: #10b981; }
        .cmd-btn.ack-batch { background: #0f766e; color: #ccfbf1; border-color: #14b8a6; }
        .cmd-btn.ack-batch:hover { background: #0d9488; color: #fff; }

        /* Row 2: In-Page Pivot Tab Strip (Fluent/RADAR Page Tabs) */
        .ws-pivot-bar {
            height: 42px; flex-shrink: 0;
            background: var(--ws-tabbar);
            border-bottom: 2px solid #e3120b;
            padding: 0 14px;
            display: flex; align-items: flex-end; justify-content: space-between; gap: 10px;
        }
        .pivot-reports-scroll {
            display: flex; align-items: flex-end; gap: 4px;
            overflow-x: auto; overflow-y: hidden; flex: 1; min-width: 0;
            scrollbar-width: thin; scrollbar-color: #475569 transparent;
        }
        .pivot-reports-scroll::-webkit-scrollbar { height: 4px; }
        .pivot-reports-scroll::-webkit-scrollbar-thumb { background: #475569; border-radius: 2px; }

        .pivot-sys-group {
            display: flex; align-items: flex-end; gap: 5px; flex-shrink: 0;
            padding-left: 8px; border-left: 1px solid rgba(148, 163, 184, 0.25);
        }
        .pivot-tab {
            cursor: pointer; user-select: none;
            height: 36px; padding: 0 12px;
            background: rgba(30, 41, 59, 0.72);
            color: #cbd5e1;
            border: 1px solid rgba(100, 116, 139, 0.35);
            border-bottom: none;
            border-radius: 7px 7px 0 0;
            font-size: 12.5px; font-weight: 700;
            display: inline-flex; align-items: center; gap: 6px;
            white-space: nowrap; flex-shrink: 0;
            transition: background 0.14s, color 0.14s;
        }
        .pivot-tab:hover { background: rgba(51, 65, 85, 0.92); color: #ffffff; }
        .pivot-tab.active {
            background: var(--ws-bg);
            color: var(--ws-text);
            border-color: #e3120b;
            border-top: 3px solid #e3120b;
            font-weight: 800;
            height: 38px;
        }
        .pivot-tab.sys-tab.active {
            background: var(--ws-bg);
            color: var(--ws-blue);
            border-top: 3px solid #3b82f6;
        }
        .tab-dot {
            width: 8px; height: 8px; border-radius: 50%; display: inline-block; flex-shrink: 0;
        }
        .tab-dot.unacked { background: #fb7185; box-shadow: 0 0 6px rgba(251, 113, 133, 0.85); }
        .tab-dot.acked { background: #10b981; }
        .tab-tag-latest {
            font-size: 10px; font-weight: 800; padding: 1px 5px; border-radius: 3px;
            background: rgba(227, 18, 11, 0.2); color: #fca5a5;
        }
        .pivot-tab.active .tab-tag-latest { background: #fee2e2; color: #b91c1c; }
        .tab-close {
            margin-left: 2px; width: 16px; height: 16px; line-height: 15px;
            text-align: center; border-radius: 50%; font-size: 12px; opacity: 0.65;
        }
        .tab-close:hover { opacity: 1; background: rgba(239, 68, 68, 0.25); color: #fca5a5; }

        /* Full-Bleed In-Page Stage */
        .ws-stage {
            flex: 1; width: 100%; position: relative; overflow: hidden; background: var(--ws-bg);
        }
        .ws-page {
            display: none; width: 100%; height: 100%; overflow: hidden;
        }
        .ws-page.active {
            display: block;
        }
        .ws-page.scrollable-sys-page {
            overflow-y: auto; padding: 22px 26px 40px 26px;
        }
        .ws-iframe {
            width: 100%; height: 100%; border: none; display: block; background: var(--ws-bg);
        }

        /* System Pages (Audit & Archive Library) Institutional Styling */
        .sys-container { max-width: 1620px; margin: 0 auto; display: flex; flex-direction: column; gap: 20px; }
        .sys-card {
            background: var(--ws-surface); border: 1px solid var(--ws-border);
            border-radius: 10px; padding: 20px 22px;
            box-shadow: 0 1px 4px rgba(15, 23, 42, 0.05);
        }
        .sys-hdr {
            display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px;
            border-bottom: 2px solid var(--ws-thead); padding-bottom: 10px; margin-bottom: 16px;
        }
        .sys-title { font-size: 17px; font-weight: 800; color: var(--ws-text); display: flex; align-items: center; gap: 8px; }
        .sys-sub { font-size: 12.5px; color: var(--ws-muted); }

        .kpi-6grid {
            display: grid; grid-template-columns: repeat(auto-fit, minmax(225px, 1fr)); gap: 12px;
        }
        .kpi-box {
            background: var(--ws-subtle); border: 1px solid var(--ws-border);
            border-top: 3px solid var(--ws-blue); border-radius: 8px; padding: 13px 15px;
        }
        .kpi-box.kpi-green { border-top-color: #16a34a; }
        .kpi-box.kpi-gold { border-top-color: #d97706; }
        .kpi-box.kpi-red { border-top-color: #e11d48; }
        .kpi-lbl { font-size: 12px; font-weight: 700; color: var(--ws-muted); margin-bottom: 4px; }
        .kpi-num { font-size: 23px; font-weight: 800; font-family: monospace; color: var(--ws-text); }
        .kpi-note { font-size: 11.5px; color: var(--ws-secondary); margin-top: 4px; line-height: 1.4; }

        .ws-table-wrap { width: 100%; overflow-x: auto; border: 1px solid var(--ws-border); border-radius: 8px; }
        table.ws-table { width: 100%; border-collapse: collapse; font-size: 13px; }
        table.ws-table th {
            background: var(--ws-thead); color: #ffffff; text-align: left;
            padding: 10px 12px; font-size: 12px; font-weight: 700; white-space: nowrap;
        }
        table.ws-table td {
            padding: 10px 12px; border-bottom: 1px solid var(--ws-border-light); color: var(--ws-text);
            vertical-align: middle;
        }
        table.ws-table tr:nth-child(even) td { background: var(--ws-subtle); }
        table.ws-table tr:hover td { background: rgba(37, 99, 235, 0.07); }

        .pill-badge {
            display: inline-block; padding: 3px 8px; border-radius: 4px; font-size: 11.5px; font-weight: 800;
        }
        .pill-bull { background: var(--ws-bull-bg); color: var(--ws-bull-text); }
        .pill-bear { background: var(--ws-bear-bg); color: var(--ws-bear-text); }
        .pill-neutral { background: var(--ws-subtle); color: var(--ws-secondary); border: 1px solid var(--ws-border); }

        .tbl-btn {
            cursor: pointer; padding: 5px 10px; border-radius: 5px; font-size: 12px; font-weight: 700;
            border: 1px solid var(--ws-border); background: var(--ws-surface); color: var(--ws-text);
            margin-right: 5px;
        }
        .tbl-btn.primary { background: #003594; color: #ffffff; border-color: #003594; }
        .tbl-btn.primary:hover { background: #1d4ed8; }
        .tbl-btn:hover { border-color: var(--ws-accent); }

        .filter-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap; margin-bottom: 12px; }
        .flt-pills { display: flex; gap: 6px; flex-wrap: wrap; }
        .flt-pill {
            cursor: pointer; padding: 5px 12px; border-radius: 999px; font-size: 12px; font-weight: 700;
            border: 1px solid var(--ws-border); background: var(--ws-subtle); color: var(--ws-secondary);
        }
        .flt-pill.active { background: var(--ws-blue); color: #fff; border-color: var(--ws-blue); }
        .search-box {
            padding: 6px 12px; border-radius: 6px; border: 1px solid var(--ws-border);
            background: var(--ws-surface); color: var(--ws-text); font-size: 12.5px; min-width: 250px;
        }
        .dual-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
        @media (max-width: 1100px) { .dual-grid { grid-template-columns: 1fr; } }
    </style>"""
