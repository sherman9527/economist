# -*- coding: utf-8 -*-
"""Static JS controller for single-window multi-tab workspace."""

ARCHIVE_JS = """        let activePageId = "";
        let archiveFilterMode = "all";
        const mountedIframes = {};

        function escHtml(s) {
            return String(s == null ? "" : s)
                .replace(/&/g, "&amp;")
                .replace(/</g, "&lt;")
                .replace(/>/g, "&gt;")
                .replace(/"/g, "&quot;");
        }

        function getSortedReports() {
            return Object.values(reportsMap || {}).sort((a, b) => {
                const da = a.date || a.report_id || "";
                const db = b.date || b.report_id || "";
                return db.localeCompare(da);
            });
        }

        function mergeLocalStorageAcks() {
            getSortedReports().forEach(r => {
                try {
                    const raw = localStorage.getItem("economist_ack_" + r.report_id);
                    if (raw) {
                        const parsed = JSON.parse(raw);
                        if (parsed && typeof parsed.acked === "boolean" && parsed.acked && !r.acked) {
                            r.acked = true;
                            r.acked_at = parsed.acked_at || r.acked_at;
                        }
                    }
                } catch (e) {}
            });
        }

        function saveLocalAck(rid, acked, ackedAt) {
            try {
                localStorage.setItem("economist_ack_" + rid, JSON.stringify({
                    report_id: rid,
                    acked: !!acked,
                    acked_at: ackedAt || new Date().toISOString().replace("T", " ").slice(0, 19)
                }));
            } catch (e) {}
        }

        function computeInitialTabs() {
            const sorted = getSortedReports();
            if (sorted.length === 0) return [];
            const latestId = sorted[0].report_id;
            const params = new URLSearchParams(window.location.search);
            const focusParam = params.get("focus") || "";

            const unreadIds = sorted.filter(r => !r.acked).map(r => r.report_id);
            const set = new Set();
            set.add(latestId);
            unreadIds.forEach(id => set.add(id));
            if (focusParam && reportsMap[focusParam]) {
                set.add(focusParam);
            }
            if (set.size === 1 && sorted.length > 1 && unreadIds.length === 0) {
                sorted.slice(0, Math.min(3, sorted.length)).forEach(r => set.add(r.report_id));
            }
            return sorted.map(r => r.report_id).filter(id => set.has(id));
        }

        function buildIframeSrc(rep) {
            const rel = rep.rel_path || "";
            const isHttp = window.location.protocol.startsWith("http");
            const base = isHttp ? ("/reports/" + rel) : rel;
            return base + (base.includes("?") ? "&embed=1" : "?embed=1");
        }

        function ensureReportPageMounted(reportId) {
            const rep = reportsMap[reportId];
            if (!rep) return null;
            let pageEl = document.getElementById("ws-page-" + reportId);
            if (!pageEl) {
                pageEl = document.createElement("section");
                pageEl.className = "ws-page";
                pageEl.id = "ws-page-" + reportId;
                document.getElementById("wsStage").appendChild(pageEl);
            }
            return pageEl;
        }

        function activatePage(pageId) {
            if (!pageId) return;
            const isSys = (pageId === "__audit__" || pageId === "__archive__");
            if (!isSys && !reportsMap[pageId]) return;

            if (!isSys && !openTabIds.includes(pageId)) {
                const sortedOrder = getSortedReports().map(r => r.report_id);
                openTabIds.push(pageId);
                openTabIds.sort((a, b) => sortedOrder.indexOf(a) - sortedOrder.indexOf(b));
            }

            activePageId = pageId;

            document.querySelectorAll(".ws-page").forEach(el => el.classList.remove("active"));

            if (isSys) {
                const sysEl = document.getElementById("ws-page-" + pageId);
                if (sysEl) sysEl.classList.add("active");
            } else {
                const pageEl = ensureReportPageMounted(pageId);
                if (pageEl) {
                    pageEl.classList.add("active");
                    if (!mountedIframes[pageId]) {
                        const iframe = document.createElement("iframe");
                        iframe.className = "ws-iframe";
                        iframe.src = buildIframeSrc(reportsMap[pageId]);
                        iframe.setAttribute("title", "经济学人研报 " + pageId);
                        iframe.addEventListener("load", () => {
                            const curTheme = document.documentElement.getAttribute("data-theme") || "light";
                            try {
                                iframe.contentWindow.postMessage({ type: "economist_theme_sync", theme: curTheme }, "*");
                                iframe.contentWindow.postMessage({ type: "economist_ack_sync", reports: reportsMap }, "*");
                                iframe.contentWindow.postMessage({ type: "economist_tab_activated" }, "*");
                            } catch (e) {}
                        });
                        pageEl.appendChild(iframe);
                        mountedIframes[pageId] = iframe;
                    } else {
                        const iframe = mountedIframes[pageId];
                        const curTheme = document.documentElement.getAttribute("data-theme") || "light";
                        try {
                            iframe.contentWindow.postMessage({ type: "economist_theme_sync", theme: curTheme }, "*");
                            iframe.contentWindow.postMessage({ type: "economist_tab_activated" }, "*");
                        } catch (e) {}
                    }
                }
            }

            if (window.location.protocol.startsWith("http")) {
                try {
                    const url = new URL(window.location.href);
                    url.searchParams.set("focus", pageId);
                    window.history.replaceState(null, "", url.toString());
                } catch (e) {}
            }

            renderTopWorkspaceUI();
            const activeTabEl = document.querySelector(`.pivot-tab[data-page-id="${CSS.escape(pageId)}"]`);
            if (activeTabEl && activeTabEl.scrollIntoView) {
                activeTabEl.scrollIntoView({ behavior: "smooth", block: "nearest", inline: "nearest" });
            }
        }

        function closeReportTab(ev, reportId) {
            if (ev) ev.stopPropagation();
            if (openTabIds.length <= 1) return;
            const idx = openTabIds.indexOf(reportId);
            if (idx === -1) return;
            openTabIds.splice(idx, 1);
            if (activePageId === reportId) {
                const nextId = openTabIds[Math.min(idx, openTabIds.length - 1)] || "__archive__";
                activatePage(nextId);
            } else {
                renderTopWorkspaceUI();
            }
        }

        function stepReportTab(delta) {
            const navSequence = [...openTabIds, "__audit__", "__archive__"];
            if (navSequence.length === 0) return;
            let idx = navSequence.indexOf(activePageId);
            if (idx === -1) idx = 0;
            const nextIdx = (idx + delta + navSequence.length) % navSequence.length;
            activatePage(navSequence[nextIdx]);
        }

        function jumpNextUnreadTab() {
            const sorted = getSortedReports();
            const unread = sorted.filter(r => !r.acked).map(r => r.report_id);
            if (unread.length === 0) return;
            const curIdx = unread.indexOf(activePageId);
            const nextId = unread[(curIdx + 1) % unread.length];
            activatePage(nextId);
        }

        function renderTopWorkspaceUI() {
            const sorted = getSortedReports();
            const latestId = sorted.length > 0 ? sorted[0].report_id : "";
            const unreadCount = sorted.filter(r => !r.acked).length;

            const totEl = document.getElementById("totalArchiveCount");
            if (totEl) totEl.textContent = String(sorted.length);

            const qPill = document.getElementById("unackedQueuePill");
            if (qPill) {
                if (unreadCount > 0) {
                    qPill.className = "queue-pill has-unread";
                    qPill.textContent = `🔴 待审阅: ${unreadCount} 份`;
                } else {
                    qPill.className = "queue-pill all-clear";
                    qPill.textContent = "✅ 全部报告已审阅完毕";
                }
            }

            const nextUnreadBtn = document.getElementById("nextUnreadBtn");
            const batchBtn = document.getElementById("batchAckAllBtn");
            if (nextUnreadBtn) nextUnreadBtn.style.display = unreadCount > 0 ? "" : "none";
            if (batchBtn) batchBtn.style.display = unreadCount > 0 ? "" : "none";

            const titleEl = document.getElementById("wsActiveTitle");
            const activeAckBtn = document.getElementById("activePageAckBtn");

            if (activePageId === "__audit__") {
                if (titleEl) titleEl.textContent = "跨期样本外实盘回溯总审计 (覆盖全部 9 期归档报告)";
                if (activeAckBtn) activeAckBtn.style.display = "none";
            } else if (activePageId === "__archive__") {
                if (titleEl) titleEl.textContent = `全量历史归档日历总库 (共 ${sorted.length} 期 · 页内秒切无弹窗)`;
                if (activeAckBtn) activeAckBtn.style.display = "none";
            } else {
                const curRep = reportsMap[activePageId];
                if (curRep && titleEl) {
                    titleEl.textContent = `${curRep.report_id}｜${curRep.title || "《经济学人》A股/美股深度投研总览"}`;
                }
                if (curRep && activeAckBtn) {
                    activeAckBtn.style.display = "";
                    if (curRep.acked) {
                        activeAckBtn.className = "cmd-btn ack-done";
                        activeAckBtn.textContent = "✅ 本页已阅 · 点击撤销";
                    } else {
                        activeAckBtn.className = "cmd-btn ack-primary";
                        activeAckBtn.textContent = "✅ 确认本页已阅 (Alt+A)";
                    }
                }
            }

            // Render Report Pivot Tabs
            const strip = document.getElementById("reportTabsStrip");
            if (strip) {
                strip.innerHTML = openTabIds.map(rid => {
                    const r = reportsMap[rid];
                    if (!r) return "";
                    const isAct = (rid === activePageId);
                    const shortDate = rid.length >= 10 ? rid.slice(5) : rid;
                    const dotCls = r.acked ? "acked" : "unacked";
                    const latestBadge = (rid === latestId) ? '<span class="tab-tag-latest">最新</span>' : "";
                    const closeHtml = (openTabIds.length > 1)
                        ? `<span class="tab-close" title="从标签栏收起 (可在归档库随时重开)" onclick="closeReportTab(event, '${escHtml(rid)}')">×</span>`
                        : "";
                    const tip = `${rid}｜${r.title || ""} (${r.acked ? "已审阅" : "未读待审阅"})`;
                    return `<div class="pivot-tab ${isAct ? "active" : ""}" data-page-id="${escHtml(rid)}" title="${escHtml(tip)}" onclick="activatePage('${escHtml(rid)}')">
                        <span class="tab-dot ${dotCls}"></span>
                        <span>📄 ${escHtml(shortDate)}</span>
                        ${latestBadge}
                        ${closeHtml}
                    </div>`;
                }).join("");
            }

            // Update System Tab active states
            document.querySelectorAll("#systemTabsStrip .pivot-tab").forEach(el => {
                el.classList.toggle("active", el.getAttribute("data-page-id") === activePageId);
            });

            renderArchiveTable();
        }

        function broadcastAckSync() {
            Object.values(mountedIframes).forEach(iframe => {
                if (iframe && iframe.contentWindow) {
                    try {
                        iframe.contentWindow.postMessage({ type: "economist_ack_sync", reports: reportsMap }, "*");
                    } catch (e) {}
                }
            });
        }

        async function setReportAckState(reportId, acked) {
            const rep = reportsMap[reportId];
            if (!rep) return;
            const nowStr = new Date().toISOString().replace("T", " ").slice(0, 19);
            rep.acked = !!acked;
            rep.acked_at = acked ? nowStr : null;
            saveLocalAck(reportId, acked, rep.acked_at);
            renderTopWorkspaceUI();
            broadcastAckSync();

            try {
                const resp = await fetch(ACK_API_BASE + "/api/ack", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ report_id: reportId, acked: !!acked })
                });
                if (resp.ok) {
                    const data = await resp.json();
                    if (data.reports) reportsMap = data.reports;
                    renderTopWorkspaceUI();
                    broadcastAckSync();
                }
            } catch (e) {
                // Offline/file:// mode: ACK remains persisted in localStorage
            }
        }

        function toggleActiveReportAck() {
            if (!activePageId || activePageId.startsWith("__")) return;
            const rep = reportsMap[activePageId];
            if (!rep) return;
            setReportAckState(activePageId, !rep.acked);
        }

        async function batchAckAllUnread() {
            const unreadIds = getSortedReports().filter(r => !r.acked).map(r => r.report_id);
            if (unreadIds.length === 0) return;
            const nowStr = new Date().toISOString().replace("T", " ").slice(0, 19);
            unreadIds.forEach(rid => {
                if (reportsMap[rid]) {
                    reportsMap[rid].acked = true;
                    reportsMap[rid].acked_at = nowStr;
                    saveLocalAck(rid, true, nowStr);
                }
            });
            renderTopWorkspaceUI();
            broadcastAckSync();

            try {
                const resp = await fetch(ACK_API_BASE + "/api/ack-batch", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ report_ids: unreadIds, acked: true })
                });
                if (resp.ok) {
                    const data = await resp.json();
                    if (data.reports) reportsMap = data.reports;
                    renderTopWorkspaceUI();
                    broadcastAckSync();
                }
            } catch (e) {
                // Offline/file:// mode: batch ACK remains persisted in localStorage
            }
        }

        function applyWorkspaceTheme(themeName) {
            const next = themeName === "dark" ? "dark" : "light";
            document.documentElement.setAttribute("data-theme", next);
            try { localStorage.setItem("economist_theme", next); } catch (e) {}
            const btn = document.getElementById("wsThemeBtn");
            if (btn) btn.textContent = next === "light" ? "🌙 暗色终端" : "☀️ 投行白底";
            Object.values(mountedIframes).forEach(iframe => {
                if (iframe && iframe.contentWindow) {
                    try {
                        iframe.contentWindow.postMessage({ type: "economist_theme_sync", theme: next }, "*");
                    } catch (e) {}
                }
            });
        }

        function toggleWorkspaceTheme() {
            const cur = document.documentElement.getAttribute("data-theme") || "light";
            applyWorkspaceTheme(cur === "light" ? "dark" : "light");
        }

        function setArchiveFilter(mode, btnEl) {
            archiveFilterMode = mode;
            document.querySelectorAll(".flt-pill").forEach(b => b.classList.remove("active"));
            if (btnEl) btnEl.classList.add("active");
            renderArchiveTable();
        }

        function renderArchiveTable() {
            const tbody = document.getElementById("archiveTableBody");
            if (!tbody) return;
            const q = ((document.getElementById("archiveSearchInput") || {}).value || "").trim().toLowerCase();
            const rows = getSortedReports().filter(r => {
                if (archiveFilterMode === "unacked" && r.acked) return false;
                if (archiveFilterMode === "acked" && !r.acked) return false;
                if (q) {
                    const hay = `${r.report_id} ${r.title || ""} ${r.window_desc || ""}`.toLowerCase();
                    if (!hay.includes(q)) return false;
                }
                return true;
            });

            tbody.innerHTML = rows.map(r => {
                const rid = r.report_id || "";
                const acked = !!r.acked;
                const badge = acked
                    ? `<span class="pill-badge pill-bull">✅ 已审阅 (${escHtml(r.acked_at || "已确认")})</span>`
                    : `<span class="pill-badge pill-bear">🔴 待审阅 (未读)</span>`;
                const ackBtnLabel = acked ? "撤销已阅" : "✅ 确认已阅";
                return `<tr>
                    <td style="font-weight:800;font-family:monospace;font-size:14px;color:var(--ws-blue);">${escHtml(rid)}</td>
                    <td>
                        <a href="javascript:void(0)" onclick="activatePage('${escHtml(rid)}')" style="color:var(--ws-blue);font-weight:800;text-decoration:none;font-size:14px;">
                            ${escHtml(r.title || rid)}
                        </a>
                        <div style="font-size:11.5px;color:var(--ws-muted);margin-top:3px;">
                            本地路径: <code>reports/${escHtml(r.rel_path || "")}</code> ｜ 采样窗口: ${escHtml(r.window_desc || "-")}
                        </div>
                    </td>
                    <td><strong>${r.core_count || 0}</strong> 篇深度卡 / 共 ${r.article_count || 0} 篇</td>
                    <td style="font-family:monospace;font-size:12px;">${escHtml(r.generated_at || "-")}</td>
                    <td>${badge}</td>
                    <td>
                        <button class="tbl-btn primary" onclick="activatePage('${escHtml(rid)}')">📑 页内打开此页</button>
                        <button class="tbl-btn" onclick="setReportAckState('${escHtml(rid)}', ${!acked})">${ackBtnLabel}</button>
                    </td>
                </tr>`;
            }).join("");
        }

        function fmtPct(v) {
            if (v == null || isNaN(v)) return "--";
            const n = Number(v);
            const sign = n > 0 ? "+" : "";
            const cls = n > 0 ? "pill-bull" : (n < 0 ? "pill-bear" : "pill-neutral");
            return `<span class="pill-badge ${cls}" style="font-family:monospace;">${sign}${n.toFixed(2)}%</span>`;
        }

        function renderAuditSystemPage() {
            const box = document.getElementById("auditPageContainer");
            if (!box) return;
            if (!auditData || !auditData.overall_per_report_dedup) {
                box.innerHTML = `<div class="sys-card"><div class="sys-title">🎯 暂无跨期回溯审计快照</div><div class="sys-sub" style="margin-top:8px;">运行 <code>python scripts/audit_all_reports.py</code> 后即可在此页内查看全量样本外胜率。</div></div>`;
                return;
            }
            const ov = auditData.overall_per_report_dedup || {};
            const us = auditData.overall_us || {};
            const cn = auditData.overall_cn || {};
            const bear = auditData.overall_bear || {};
            const uniqUs = ((auditData.unique_first_call_cohort || {}).us) || {};
            const pairs = auditData.pair_trades_audit || {};
            const byRep = auditData.by_report || {};

            const repRowsHtml = Object.keys(byRep).sort().reverse().map(rid => {
                const m = byRep[rid] || {};
                return `<tr>
                    <td style="font-family:monospace;font-weight:800;color:var(--ws-blue);">${escHtml(rid)}</td>
                    <td><strong>${m.count || 0}</strong> 笔 (多${m.bull_count || 0} / 空${m.bear_count || 0})</td>
                    <td><strong>${(m.abs_hit_rate || 0).toFixed(1)}%</strong></td>
                    <td><strong>${(m.alpha_hit_rate || 0).toFixed(1)}%</strong></td>
                    <td><span class="pill-badge ${m.institutional_hit_rate >= 55 ? 'pill-bull' : 'pill-neutral'}">${(m.institutional_hit_rate || 0).toFixed(1)}%</span></td>
                    <td>${fmtPct(m.avg_bull_ret_pct)}</td>
                    <td>${fmtPct(m.avg_bear_ret_pct)}</td>
                    <td>${fmtPct(m.long_short_spread_pct)}</td>
                    <td><button class="tbl-btn primary" onclick="activatePage('${escHtml(rid)}')">📑 页内打开该期</button></td>
                </tr>`;
            }).join("");

            const pairRowsHtml = (pairs.top_pairs || []).slice(0, 12).map(p => {
                const winBadge = p.pair_win
                    ? '<span class="pill-badge pill-bull">✅ 配对胜出</span>'
                    : '<span class="pill-badge pill-bear">❌ 配对回撤</span>';
                return `<tr>
                    <td style="font-family:monospace;font-weight:700;">${escHtml(p.report_date)}</td>
                    <td>${escHtml(p.market_type === 'US' ? '🇺🇸 美股' : '🇨🇳 A股')} · ${escHtml(p.category || '')}</td>
                    <td>${escHtml(p.long_desc || '')}</td>
                    <td>${escHtml(p.short_desc || '')}</td>
                    <td>${fmtPct(p.spread_pct)}</td>
                    <td>${winBadge}</td>
                    <td><button class="tbl-btn" onclick="activatePage('${escHtml(p.report_date)}')">查看原报告</button></td>
                </tr>`;
            }).join("");

            const bestRowsHtml = [
                ...(auditData.best_longs || []).slice(0, 5),
                ...(auditData.best_shorts || []).slice(0, 4)
            ].map(c => `<tr>
                <td style="font-family:monospace;">${escHtml(c.report_date)}</td>
                <td><strong>${escHtml(c.name || c.symbol)}</strong> <code>(${escHtml(c.symbol)})</code></td>
                <td><span class="pill-badge ${c.polarity === 'bull' ? 'pill-bull' : 'pill-bear'}">${escHtml(c.rating)}</span></td>
                <td>${fmtPct(c.ret_cum_pct)}</td>
                <td>${fmtPct(c.alpha_cum_pct)}</td>
                <td><button class="tbl-btn" onclick="activatePage('${escHtml(c.report_date)}')">页内定位</button></td>
            </tr>`).join("");

            const missRowsHtml = [
                ...(auditData.worst_long_misses || []).slice(0, 5),
                ...(auditData.worst_short_misses || []).slice(0, 4)
            ].map(c => `<tr>
                <td style="font-family:monospace;">${escHtml(c.report_date)}</td>
                <td><strong>${escHtml(c.name || c.symbol)}</strong> <code>(${escHtml(c.symbol)})</code></td>
                <td><span class="pill-badge ${c.polarity === 'bull' ? 'pill-bull' : 'pill-bear'}">${escHtml(c.rating)}</span></td>
                <td>${fmtPct(c.ret_cum_pct)}</td>
                <td>${fmtPct(c.alpha_cum_pct)}</td>
                <td><button class="tbl-btn" onclick="activatePage('${escHtml(c.report_date)}')">页内复盘</button></td>
            </tr>`).join("");

            box.innerHTML = `
                <div class="sys-card">
                    <div class="sys-hdr">
                        <div>
                            <div class="sys-title">🎯 跨期全量样本外 (Out-of-Sample) 股票预测准确度回溯总表</div>
                            <div class="sys-sub">审计时间：${escHtml(auditData.audited_at || '-')} ｜ 严格以各期研报锚定收盘价 T0 对比最新实盘价，剥离未开盘样本 (${auditData.pending_unopened_calls_count || 0} 笔待开盘) 与中性标配 (${auditData.neutral_ew_calls_count || 0} 笔)</div>
                        </div>
                    </div>
                    <div class="kpi-6grid">
                        <div class="kpi-box">
                            <div class="kpi-lbl">已开盘样本外有效验证</div>
                            <div class="kpi-num">${ov.count || 0} 笔</div>
                            <div class="kpi-note">机构综合胜率 <strong>${(ov.institutional_hit_rate || 0).toFixed(1)}%</strong> ｜ 超额Alpha胜率 <strong>${(ov.alpha_hit_rate || 0).toFixed(1)}%</strong></div>
                        </div>
                        <div class="kpi-box kpi-green">
                            <div class="kpi-lbl">🇺🇸 美股映射实盘胜率</div>
                            <div class="kpi-num">${(us.institutional_hit_rate || 0).toFixed(1)}%</div>
                            <div class="kpi-note">首推独立股综合胜率 <strong>${(uniqUs.institutional_hit_rate || 0).toFixed(1)}%</strong> (${us.count || 0}笔 · T+1胜率 ${(us.t1_abs_hit_rate || 0).toFixed(1)}%)</div>
                        </div>
                        <div class="kpi-box kpi-green">
                            <div class="kpi-lbl">🛡️ 减持/规避 (空头排雷) Alpha胜率</div>
                            <div class="kpi-num">${(bear.alpha_hit_rate || 0).toFixed(1)}%</div>
                            <div class="kpi-note">共 ${bear.count || 0} 笔看空预警 ｜ 被规避标的均值跑输基准 <strong>${(bear.avg_bear_alpha_pct || 0).toFixed(2)}%</strong></div>
                        </div>
                        <div class="kpi-box kpi-gold">
                            <div class="kpi-lbl">⚖️ 高盛/大摩式配对交易胜率</div>
                            <div class="kpi-num">${(pairs.win_rate_pct || 0).toFixed(1)}%</div>
                            <div class="kpi-note">${pairs.evaluated_count || 0} 组实盘配对 (${pairs.win_count || 0}胜) ｜ 多空平均净价差 <strong>+${(pairs.avg_spread_pct || 0).toFixed(2)}%</strong></div>
                        </div>
                        <div class="kpi-box kpi-red">
                            <div class="kpi-lbl">🇨🇳 A股映射相对基准胜率</div>
                            <div class="kpi-num">${(cn.alpha_hit_rate || 0).toFixed(1)}%</div>
                            <div class="kpi-note">同期创业板跌 -4.17% 逆风下绝对胜率 ${(cn.abs_hit_rate || 0).toFixed(1)}% (多空剪刀差 +${(cn.long_short_spread_pct || 0).toFixed(2)}%)</div>
                        </div>
                    </div>
                </div>

                <div class="sys-card">
                    <div class="sys-hdr">
                        <div class="sys-title">📅 各期研报样本外实盘命中率演进明细 (点击可直接页内切至对应报告)</div>
                    </div>
                    <div class="ws-table-wrap">
                        <table class="ws-table">
                            <thead>
                                <tr>
                                    <th>报告期次</th>
                                    <th>已开盘样本数</th>
                                    <th>绝对方向胜率</th>
                                    <th>超额Alpha胜率</th>
                                    <th>机构综合胜率</th>
                                    <th>多头平均涨跌</th>
                                    <th>空头标的涨跌</th>
                                    <th>多空收益剪刀差</th>
                                    <th>页内跳转</th>
                                </tr>
                            </thead>
                            <tbody>${repRowsHtml || '<tr><td colspan="9">今日新生成报告待下一交易日收盘后纳入样本外统计</td></tr>'}</tbody>
                        </table>
                    </div>
                </div>

                <div class="sys-card">
                    <div class="sys-hdr">
                        <div class="sys-title">⚖️ 多空配对交易 (Pair Trade) 实盘净价差验证榜</div>
                    </div>
                    <div class="ws-table-wrap">
                        <table class="ws-table">
                            <thead>
                                <tr>
                                    <th>期次</th>
                                    <th>市场与赛道</th>
                                    <th>多头腿 (Long) 实盘表现</th>
                                    <th>空头腿 (Short) 实盘表现</th>
                                    <th>多空净价差 (Spread)</th>
                                    <th>验证结果</th>
                                    <th>定位</th>
                                </tr>
                            </thead>
                            <tbody>${pairRowsHtml}</tbody>
                        </table>
                    </div>
                </div>

                <div class="dual-grid">
                    <div class="sys-card">
                        <div class="sys-hdr"><div class="sys-title">🏆 实盘高光红榜 (多头暴涨 & 精准排雷)</div></div>
                        <div class="ws-table-wrap">
                            <table class="ws-table">
                                <thead><tr><th>期次</th><th>标的</th><th>评级</th><th>区间涨跌</th><th>超额Alpha</th><th>查看</th></tr></thead>
                                <tbody>${bestRowsHtml}</tbody>
                            </table>
                        </div>
                    </div>
                    <div class="sys-card">
                        <div class="sys-hdr"><div class="sys-title">⚠️ 实盘偏差黑榜 (叙事抢跑与逆风误判复盘)</div></div>
                        <div class="ws-table-wrap">
                            <table class="ws-table">
                                <thead><tr><th>期次</th><th>标的</th><th>评级</th><th>区间涨跌</th><th>超额Alpha</th><th>复盘</th></tr></thead>
                                <tbody>${missRowsHtml}</tbody>
                            </table>
                        </div>
                    </div>
                </div>
            `;
        }

        window.addEventListener("message", (event) => {
            const msg = event.data;
            if (!msg || typeof msg !== "object") return;
            if (msg.type === "economist_switch_tab" && msg.pageId) {
                activatePage(msg.pageId);
            } else if (msg.type === "economist_child_theme_changed" && msg.theme) {
                applyWorkspaceTheme(msg.theme);
            } else if (msg.type === "economist_child_ack_changed") {
                if (msg.reports && Object.keys(msg.reports).length > 0) {
                    reportsMap = msg.reports;
                } else if (msg.report_id && reportsMap[msg.report_id]) {
                    reportsMap[msg.report_id].acked = !!msg.acked;
                    reportsMap[msg.report_id].acked_at = msg.acked_at || null;
                }
                if (msg.report_id) {
                    saveLocalAck(msg.report_id, !!msg.acked, msg.acked_at);
                }
                renderTopWorkspaceUI();
                broadcastAckSync();
            }
        });

        window.addEventListener("keydown", (e) => {
            if (e.altKey && (e.key === "a" || e.key === "A")) {
                e.preventDefault();
                toggleActiveReportAck();
            } else if (e.altKey && e.key === "ArrowLeft") {
                e.preventDefault();
                stepReportTab(-1);
            } else if (e.altKey && e.key === "ArrowRight") {
                e.preventDefault();
                stepReportTab(1);
            }
        });

        async function bootstrapWorkspace() {
            const savedTheme = localStorage.getItem("economist_theme") || "light";
            applyWorkspaceTheme(savedTheme);
            mergeLocalStorageAcks();

            try {
                const resp = await fetch(ACK_API_BASE + "/api/ack-state", { cache: "no-store" });
                if (resp.ok) {
                    const data = await resp.json();
                    if (data.reports) reportsMap = data.reports;
                    mergeLocalStorageAcks();
                }
            } catch (e) {}

            try {
                const aResp = await fetch(ACK_API_BASE + "/api/audit-summary", { cache: "no-store" });
                if (aResp.ok) {
                    const aJson = await aResp.json();
                    if (aJson && aJson.audit) auditData = aJson.audit;
                }
            } catch (e) {}

            openTabIds = computeInitialTabs();
            renderAuditSystemPage();

            const params = new URLSearchParams(window.location.search);
            const focusParam = params.get("focus") || "";
            const sorted = getSortedReports();
            const defaultPage = (focusParam && (focusParam === "__audit__" || focusParam === "__archive__" || reportsMap[focusParam]))
                ? focusParam
                : (openTabIds[0] || (sorted[0] ? sorted[0].report_id : "__archive__"));

            activatePage(defaultPage);
        }

        window.addEventListener("DOMContentLoaded", bootstrapWorkspace);"""
