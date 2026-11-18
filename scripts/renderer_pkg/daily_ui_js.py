# -*- coding: utf-8 -*-
"""Static interactive ACK, filter, and theme scripts for daily reports."""

DAILY_UI_JS = """
        function switchArticleChart(artIdx, sym, btnEl) {
            if (btnEl) {
                const bar = document.getElementById("art-tabs-" + artIdx);
                if (bar) bar.querySelectorAll(".art-chart-tab").forEach(b => b.classList.remove("active"));
                btnEl.classList.add("active");
            }
            const dom = document.getElementById("art-echart-" + artIdx);
            if (!dom || !window.echarts || !KLINE_MAP[sym]) return;
            let inst = chartInstances["art_" + artIdx] || echarts.init(dom);
            chartInstances["art_" + artIdx] = inst;
            inst.setOption(buildCandlestickOption(sym, true), true);
        }

        function switchArticleBarChart(artIdx, btnEl) {
            if (btnEl) {
                const bar = document.getElementById("art-tabs-" + artIdx);
                if (bar) bar.querySelectorAll(".art-chart-tab").forEach(b => b.classList.remove("active"));
                btnEl.classList.add("active");
            }
            const cfg = PER_ARTICLE_CHARTS[String(artIdx)];
            const dom = document.getElementById("art-echart-" + artIdx);
            if (!dom || !window.echarts || !cfg) return;
            const c = getThemeColors();
            let inst = chartInstances["art_" + artIdx] || echarts.init(dom);
            chartInstances["art_" + artIdx] = inst;
            const rows = cfg.stocks_bar || [];
            inst.setOption({
                tooltip: { trigger: "axis", axisPointer: { type: "shadow" } },
                legend: { data: ["1日涨跌(%)", "5日涨跌(%)", "20日涨跌(%)"], top: 2, textStyle: { color: c.text, fontSize: 11 } },
                grid: { left: 44, right: 14, top: 32, bottom: 38 },
                xAxis: { type: "category", data: rows.map(r => r.name.split("(")[0].trim()), axisLabel: { color: c.text, fontSize: 10, interval: 0, rotate: 12 } },
                yAxis: { type: "value", axisLabel: { formatter: "{value}%", color: c.muted, fontSize: 10 }, splitLine: { lineStyle: { color: c.gridLine } } },
                series: [
                    { name: "1日涨跌(%)", type: "bar", data: rows.map(r => r.d1), itemStyle: { color: "#3b82f6" } },
                    { name: "5日涨跌(%)", type: "bar", data: rows.map(r => r.d5), itemStyle: { color: "#10b981" } },
                    { name: "20日涨跌(%)", type: "bar", data: rows.map(r => r.d20), itemStyle: { color: "#6366f1" } }
                ]
            }, true);
        }

        function initAllArticleCharts() {
            Object.keys(PER_ARTICLE_CHARTS).forEach(k => {
                const cfg = PER_ARTICLE_CHARTS[k];
                if (cfg && cfg.syms && cfg.syms.length > 0) {
                    switchArticleChart(k, cfg.syms[0]);
                }
            });
        }

        function applyThemeDirect(themeName) {
            const htmlEl = document.documentElement;
            const next = themeName === "dark" ? "dark" : "light";
            htmlEl.setAttribute("data-theme", next);
            try { localStorage.setItem("economist_theme", next); } catch (e) {}
            const btn = document.getElementById("themeToggleBtn");
            if (btn) btn.textContent = next === "light" ? "🌙 切换暗色终端" : "☀️ 切换投行白底";
            const activeKBtn = document.querySelector(".kline-tab-btn.active");
            const activeSym = activeKBtn ? activeKBtn.getAttribute("data-sym") : (Object.keys(KLINE_MAP)[0] || "SPY");
            renderMainKline(activeSym);
            renderBacktestCharts();
            renderFourOverviewCharts();
            initAllArticleCharts();
        }

        function toggleTheme() {
            const current = document.documentElement.getAttribute("data-theme") || "light";
            const next = current === "light" ? "dark" : "light";
            applyThemeDirect(next);
            if (window.self !== window.top) {
                try { window.parent.postMessage({ type: "economist_child_theme_changed", theme: next }, "*"); } catch (e) {}
            }
        }

        function handleInPageReportNav(ev, reportId) {
            if (reportId === CURRENT_REPORT_ID) {
                if (ev) ev.preventDefault();
                window.scrollTo({ top: 0, behavior: "smooth" });
                return false;
            }
            if (window.self !== window.top) {
                if (ev) ev.preventDefault();
                try {
                    window.parent.postMessage({ type: "economist_switch_tab", pageId: reportId }, "*");
                } catch (e) {}
                return false;
            }
            return true;
        }

        (function initTheme() {
            const saved = localStorage.getItem("economist_theme") || "light";
            document.documentElement.setAttribute("data-theme", saved);
        })();

        function getLocalAck(reportId) {
            try {
                const raw = localStorage.getItem("economist_ack_" + reportId);
                return raw ? JSON.parse(raw) : null;
            } catch (e) { return null; }
        }

        function setLocalAck(reportId, acked, ackedAt) {
            try {
                localStorage.setItem("economist_ack_" + reportId, JSON.stringify({
                    report_id: reportId,
                    acked: !!acked,
                    acked_at: ackedAt || new Date().toISOString().replace("T", " ").slice(0, 19)
                }));
            } catch (e) {}
        }

        function renderAckUI(acked, ackedAt) {
            const pill = document.getElementById("ackStatusPill");
            const btn = document.getElementById("ackBtn");
            if (acked) {
                pill.className = "ack-status-pill acked";
                pill.textContent = "✅ 已审阅 (" + (ackedAt || "刚刚") + ")";
                btn.className = "ack-btn btn-acked";
                btn.innerHTML = "✅ 已确认审阅 · 点击撤销";
                btn.dataset.acked = "true";
            } else {
                pill.className = "ack-status-pill unacked";
                pill.textContent = "🔴 待审阅 (未读)";
                btn.className = "ack-btn btn-to-ack";
                btn.innerHTML = "✅ 确认已阅 (本期报告)";
                btn.dataset.acked = "false";
            }
        }

        function renderUnackedBanner(unackedReports) {
            const banner = document.getElementById("unackedBanner");
            const listEl = document.getElementById("unackedList");
            if (!banner || !listEl) return;
            if (document.documentElement.getAttribute("data-embed") === "1") {
                banner.classList.remove("visible");
                return;
            }

            const others = (unackedReports || []).filter(r => r && typeof r.report_id === "string" && /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/.test(r.report_id) && r.report_id !== CURRENT_REPORT_ID);
            if (others.length === 0) {
                banner.classList.remove("visible");
                listEl.innerHTML = "";
                return;
            }
            banner.classList.add("visible");
            const isHttp = window.location.protocol.startsWith("http");
            listEl.innerHTML = others.map(r => {
                const rid = escHtml(r.report_id);
                const href = isHttp
                    ? ("/reports/index.html?focus=" + encodeURIComponent(r.report_id))
                    : ("../../../index.html?focus=" + encodeURIComponent(r.report_id));
                return `<span class="unacked-chip">
                    <a href="${escHtml(href)}" onclick="return handleInPageReportNav(event, '${rid}')" style="color:inherit;text-decoration:none;">📄 ${rid} (页内切换)</a>
                    <button onclick="quickAckOther('${rid}')" style="cursor:pointer;background:#10b981;color:#fff;border:none;border-radius:4px;padding:2px 7px;font-size:11px;font-weight:800;">确认已阅</button>
                </span>`;
            }).join("");
        }

        async function syncWithServer() {
            const savedTheme = localStorage.getItem("economist_theme") || "light";
            const tBtn = document.getElementById("themeToggleBtn");
            if (tBtn) tBtn.textContent = savedTheme === "light" ? "🌙 切换暗色终端" : "☀️ 切换投行白底";

            const isHttp = window.location.protocol.startsWith("http");
            const portal = document.getElementById("archivePortalLink");
            if (isHttp) {
                document.querySelectorAll("[data-http-href]").forEach(el => {
                    el.setAttribute("href", el.getAttribute("data-http-href"));
                });
                if (portal) portal.setAttribute("href", "/reports/index.html?focus=" + encodeURIComponent(CURRENT_REPORT_ID));
            } else {
                if (portal) portal.setAttribute("href", "../../../index.html?focus=" + encodeURIComponent(CURRENT_REPORT_ID));
            }

            const local = getLocalAck(CURRENT_REPORT_ID);
            const initAcked = (local && typeof local.acked === "boolean") ? local.acked : INITIAL_ACK_SNAPSHOT.acked;
            const initAt = (local && local.acked_at) ? local.acked_at : INITIAL_ACK_SNAPSHOT.acked_at;
            renderAckUI(initAcked, initAt);

            try {
                const resp = await fetch(ACK_API_BASE + "/api/ack-state", { cache: "no-store" });
                if (!resp.ok) return;
                const data = await resp.json();
                const srvEntry = (data.reports || {})[CURRENT_REPORT_ID];

                if (local && local.acked && srvEntry && !srvEntry.acked) {
                    await postAckToServer(CURRENT_REPORT_ID, true);
                    return;
                }
                if (srvEntry) {
                    renderAckUI(srvEntry.acked, srvEntry.acked_at);
                    setLocalAck(CURRENT_REPORT_ID, srvEntry.acked, srvEntry.acked_at);
                }
                renderUnackedBanner(data.unacked_reports || []);
            } catch (e) {}
        }

        async function postAckToServer(reportId, acked) {
            const nowStr = new Date().toISOString().replace("T", " ").slice(0, 19);
            setLocalAck(reportId, acked, acked ? nowStr : null);
            if (reportId === CURRENT_REPORT_ID) {
                renderAckUI(acked, acked ? nowStr : null);
            }
            try {
                const resp = await fetch(ACK_API_BASE + "/api/ack", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ report_id: reportId, acked: acked })
                });
                if (resp.ok) {
                    const data = await resp.json();
                    if (reportId === CURRENT_REPORT_ID && data.entry) {
                        renderAckUI(data.entry.acked, data.entry.acked_at);
                        setLocalAck(reportId, data.entry.acked, data.entry.acked_at);
                    }
                    renderUnackedBanner(data.unacked_reports || []);
                    document.querySelectorAll("#sidebarArchiveList .arch-item").forEach(item => {
                        const rid = item.getAttribute("data-report-id");
                        const rInfo = (data.reports || {})[rid];
                        if (rInfo) {
                            item.classList.toggle("acked", !!rInfo.acked);
                            item.classList.toggle("unacked", !rInfo.acked);
                            const st = item.querySelector(".arch-status");
                            if (st) st.textContent = rInfo.acked ? "✅ 已审阅" : "🔴 待审阅";
                        }
                    });
                    if (window.self !== window.top) {
                        try {
                            window.parent.postMessage({
                                type: "economist_child_ack_changed",
                                report_id: reportId,
                                acked: !!(data.entry ? data.entry.acked : acked),
                                acked_at: data.entry ? data.entry.acked_at : nowStr,
                                reports: data.reports || {},
                                unacked_reports: data.unacked_reports || []
                            }, "*");
                        } catch (e) {}
                    }
                }
            } catch (e) {
                if (window.self !== window.top) {
                    try {
                        window.parent.postMessage({
                            type: "economist_child_ack_changed",
                            report_id: reportId,
                            acked: !!acked,
                            acked_at: acked ? nowStr : null
                        }, "*");
                    } catch (err2) {}
                }
            }
        }

        function resizeAllCharts() {
            Object.values(chartInstances).forEach(inst => {
                if (inst && inst.resize) {
                    try { inst.resize(); } catch (e) {}
                }
            });
        }

        window.addEventListener("message", (event) => {
            const msg = event.data;
            if (!msg || typeof msg !== "object") return;
            if (msg.type === "economist_tab_activated") {
                resizeAllCharts();
                requestAnimationFrame(() => setTimeout(resizeAllCharts, 40));
            } else if (msg.type === "economist_theme_sync" && msg.theme) {
                applyThemeDirect(msg.theme);
            } else if (msg.type === "economist_ack_sync") {
                const rMap = msg.reports || {};
                const myEntry = rMap[CURRENT_REPORT_ID];
                if (myEntry) {
                    renderAckUI(!!myEntry.acked, myEntry.acked_at);
                    setLocalAck(CURRENT_REPORT_ID, !!myEntry.acked, myEntry.acked_at);
                } else if (msg.report_id === CURRENT_REPORT_ID) {
                    renderAckUI(!!msg.acked, msg.acked_at);
                    setLocalAck(CURRENT_REPORT_ID, !!msg.acked, msg.acked_at);
                }
                document.querySelectorAll("#sidebarArchiveList .arch-item").forEach(item => {
                    const rid = item.getAttribute("data-report-id");
                    const rInfo = rMap[rid] || (rid === msg.report_id ? { acked: msg.acked } : null);
                    if (rInfo) {
                        item.classList.toggle("acked", !!rInfo.acked);
                        item.classList.toggle("unacked", !rInfo.acked);
                        const st = item.querySelector(".arch-status");
                        if (st) st.textContent = rInfo.acked ? "✅ 已审阅" : "🔴 待审阅";
                    }
                });
            }
        });

        function toggleAck() {
            const btn = document.getElementById("ackBtn");
            const currentlyAcked = btn.dataset.acked === "true";
            postAckToServer(CURRENT_REPORT_ID, !currentlyAcked);
        }

        function quickAckOther(reportId) {
            postAckToServer(reportId, true);
        }

        function jumpToArticle(anchorId) {
            const el = document.getElementById(anchorId);
            if (!el) return;
            el.style.display = "";
            el.scrollIntoView({ behavior: "smooth", block: "start" });
            el.classList.add("highlight-flash");
            setTimeout(() => el.classList.remove("highlight-flash"), 2400);
        }

        function filterMasterStocks(mode, btnEl) {
            const bar = btnEl ? btnEl.closest(".filter-toolbar") : null;
            if (bar) bar.querySelectorAll(".flt-btn").forEach(b => b.classList.remove("active"));
            if (btnEl) btnEl.classList.add("active");
            const rows = document.querySelectorAll(".stock-master-row");
            rows.forEach((row, idx) => {
                const mkt = row.dataset.market;
                const pol = row.dataset.polarity;
                let show = true;
                if (mode === "top24") show = (idx < 24);
                else if (mode === "bull") show = (pol === "bull");
                else if (mode === "bear") show = (pol === "bear");
                else if (mode === "US") show = (mkt === "US");
                else if (mode === "CN") show = (mkt === "CN");
                row.style.display = show ? "" : "none";
            });
        }

        function searchMasterStocks(query) {
            const q = (query || "").trim().toLowerCase();
            document.querySelectorAll(".stock-master-row").forEach((row, idx) => {
                const hay = row.dataset.search || "";
                row.style.display = (!q ? idx < 24 : hay.includes(q)) ? "" : "none";
            });
        }

        function filterNoteCards(mode, btnEl) {
            const bar = btnEl ? btnEl.closest(".filter-toolbar") : null;
            if (bar) bar.querySelectorAll(".flt-btn").forEach(b => b.classList.remove("active"));
            if (btnEl) btnEl.classList.add("active");
            document.querySelectorAll(".gs-note-card").forEach(card => {
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
            });
        }

        function searchNoteCards(query) {
            const q = (query || "").trim().toLowerCase();
            document.querySelectorAll(".gs-note-card").forEach(card => {
                const hay = card.dataset.search || "";
                const bodyText = card.innerText.toLowerCase();
                card.style.display = (!q || hay.includes(q) || bodyText.includes(q)) ? "" : "none";
            });
        }

        window.addEventListener("resize", () => {
            Object.values(chartInstances).forEach(inst => {
                if (inst && inst.resize) inst.resize();
            });
        });

        window.addEventListener("keydown", (e) => {
            if (e.altKey && (e.key === "a" || e.key === "A")) {
                e.preventDefault();
                toggleAck();
            }
        });

        window.addEventListener("DOMContentLoaded", () => {
            syncWithServer();
            const firstSym = Object.keys(KLINE_MAP)[0] || "SPY";
            renderMainKline(firstSym);
            renderBacktestCharts();
            renderFourOverviewCharts();
            initAllArticleCharts();
        });"""
