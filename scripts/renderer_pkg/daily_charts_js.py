# -*- coding: utf-8 -*-
"""Static ECharts option & overview chart builders for daily reports."""

DAILY_CHARTS_JS = """
        function getThemeColors() {
            const isDark = (document.documentElement.getAttribute("data-theme") === "dark");
            return {
                isDark,
                text: isDark ? "#e2e8f0" : "#0f172a",
                muted: isDark ? "#94a3b8" : "#64748b",
                gridLine: isDark ? "#1e293b" : "#e2e8f0",
                upColor: "#16a34a",
                downColor: "#e11d48",
                usColor: "#2563eb",
                cnColor: "#e11d48",
                accentGold: "#d97706"
            };
        }

        function buildCandlestickOption(sym, compactMode) {
            const kd = KLINE_MAP[sym];
            const c = getThemeColors();
            if (!kd) return {};
            const dates = kd.dates || [];
            const ohlc = kd.ohlc || [];
            const vols = kd.volumes || [];
            const ma5 = kd.ma5 || [];
            const ma20 = kd.ma20 || [];

            return {
                animation: false,
                tooltip: {
                    trigger: "axis",
                    axisPointer: { type: "cross" }
                },
                legend: {
                    data: ["日K线", "5日均线", "20日均线"],
                    top: 2,
                    textStyle: { color: c.text, fontSize: compactMode ? 11 : 12 }
                },
                grid: compactMode
                    ? [
                        { left: 48, right: 14, top: 30, height: "58%" },
                        { left: 48, right: 14, top: "74%", height: "16%" }
                    ]
                    : [
                        { left: 56, right: 20, top: 36, height: "60%" },
                        { left: 56, right: 20, top: "74%", height: "16%" }
                    ],
                xAxis: [
                    {
                        type: "category",
                        data: dates,
                        boundaryGap: true,
                        axisLine: { lineStyle: { color: c.muted } },
                        axisLabel: { color: c.muted, fontSize: 11 }
                    },
                    {
                        type: "category",
                        gridIndex: 1,
                        data: dates,
                        axisLabel: { show: false },
                        axisTick: { show: false }
                    }
                ],
                yAxis: [
                    {
                        scale: true,
                        splitLine: { lineStyle: { color: c.gridLine } },
                        axisLabel: { color: c.muted, fontSize: 11 }
                    },
                    {
                        scale: true,
                        gridIndex: 1,
                        splitNumber: 2,
                        axisLabel: { show: false },
                        splitLine: { show: false }
                    }
                ],
                series: [
                    {
                        name: "日K线",
                        type: "candlestick",
                        data: ohlc,
                        itemStyle: {
                            color: c.upColor,
                            color0: c.downColor,
                            borderColor: c.upColor,
                            borderColor0: c.downColor
                        },
                        markLine: {
                            symbol: ["none", "none"],
                            label: { formatter: "研报发布日 (" + CURRENT_REPORT_ID + ")", fontSize: 10, color: c.accentGold },
                            lineStyle: { color: c.accentGold, type: "dashed", width: 1.5 },
                            data: [{ xAxis: dates.find(d => d >= CURRENT_REPORT_ID) || dates[dates.length - 1] }]
                        }
                    },
                    {
                        name: "5日均线",
                        type: "line",
                        data: ma5,
                        smooth: true,
                        showSymbol: false,
                        lineStyle: { width: 1.6, color: "#f59e0b" }
                    },
                    {
                        name: "20日均线",
                        type: "line",
                        data: ma20,
                        smooth: true,
                        showSymbol: false,
                        lineStyle: { width: 1.6, color: "#6366f1" }
                    },
                    {
                        name: "成交量",
                        type: "bar",
                        xAxisIndex: 1,
                        yAxisIndex: 1,
                        data: vols.map((v, idx) => ({
                            value: v,
                            itemStyle: {
                                color: (ohlc[idx] && ohlc[idx][1] >= ohlc[idx][0]) ? "rgba(22,163,74,0.55)" : "rgba(225,29,72,0.55)"
                            }
                        }))
                    }
                ]
            };
        }

        function renderMainKline(sym, btnEl) {
            if (!KLINE_MAP[sym]) return;
            document.querySelectorAll(".kline-tab-btn").forEach(b => {
                b.classList.toggle("active", b.getAttribute("data-sym") === sym);
            });
            if (btnEl) btnEl.classList.add("active");

            const dom = document.getElementById("mainKlineChart");
            if (dom && window.echarts) {
                let inst = chartInstances["mainKline"] || echarts.init(dom);
                chartInstances["mainKline"] = inst;
                inst.setOption(buildCandlestickOption(sym, false), true);
            }

            const kd = KLINE_MAP[sym];
            const meta = STOCK_META[sym] || {};
            const side = document.getElementById("mainKlineSidecard");
            if (side && kd) {
                const fmtPct = (v) => {
                    const s = v > 0 ? "+" : "";
                    const cl = v > 0 ? "pct-up" : (v < 0 ? "pct-down" : "pct-flat");
                    return `<span class="pct-pill ${cl}">${s}${v.toFixed(2)}%</span>`;
                };
                const ratingStr = meta.rating || (kd.chg_5d_pct >= 0 ? "超配增持" : "标配观望");
                const jumpHtml = meta.article_idx
                    ? `<a href="#art-${meta.article_idx}" onclick="jumpToArticle('art-${meta.article_idx}');return false;" class="jump-link">跳转关联研报卡 #${meta.article_idx} (${meta.article_title}) ↓</a>`
                    : `<span style="color:var(--text-muted);font-size:12px;">基准核心指数标的</span>`;
                side.innerHTML = `
                    <div>
                        <div style="font-size:12px;color:var(--text-muted);font-weight:700;">当前选中实盘标的 (${kd.latest_date})</div>
                        <div style="font-size:20px;font-weight:800;color:var(--text-main);margin:4px 0;">${kd.name}</div>
                        <div style="font-size:28px;font-weight:800;font-family:monospace;color:var(--gs-blue);margin-bottom:10px;">
                            ${kd.latest_close.toFixed(2)}
                        </div>
                        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;background:var(--bg-surface);padding:10px;border-radius:6px;border:1px solid var(--border-light);margin-bottom:12px;">
                            <div><div style="font-size:11px;color:var(--text-muted);">1日涨跌</div>${fmtPct(kd.chg_1d_pct)}</div>
                            <div><div style="font-size:11px;color:var(--text-muted);">5日涨跌</div>${fmtPct(kd.chg_5d_pct)}</div>
                            <div><div style="font-size:11px;color:var(--text-muted);">20日涨跌</div>${fmtPct(kd.chg_20d_pct)}</div>
                        </div>
                        <div style="margin-bottom:8px;">
                            <span style="font-size:12px;font-weight:700;color:var(--text-muted);">投行评级：</span>
                            <strong style="color:var(--gs-blue);">${ratingStr}</strong>
                            ${meta.verdict_tag ? `<span style="margin-left:6px;font-size:11.5px;font-weight:800;color:#15803d;">(${meta.verdict_tag})</span>` : ""}
                        </div>
                        ${meta.support_20d ? `
                        <div style="display:grid;grid-template-columns:1fr 1fr;gap:6px;font-size:11.5px;font-family:monospace;background:var(--bg-surface);padding:8px;border-radius:6px;border:1px solid var(--border-light);margin-bottom:8px;">
                            <div>🟢 20日支撑: <strong>${meta.support_20d}</strong> | 止损: <strong>${meta.stop_level}</strong></div>
                            <div>🔴 20日压力: <strong>${meta.resist_20d}</strong> | 目标: <strong>${meta.target_level}</strong></div>
                        </div>` : ""}
                        <div style="margin-bottom:8px;">
                            <span style="font-size:12px;font-weight:700;color:var(--text-muted);">冲击维度：</span>
                            <span class="driver-chip">${meta.driver || "宏观流动性与估值折现"}</span>
                        </div>
                        <div style="font-size:13px;color:var(--text-secondary);background:var(--bg-surface);padding:10px;border-radius:6px;border-left:3px solid var(--econ-red);">
                            <strong>《经济学人》催化逻辑：</strong><br>
                            ${meta.thesis_short || "反映近期美国中期选举政治重构、人工智能资本开支投入回报率审视与全球关税豁免预期差。"}
                        </div>
                    </div>
                    <div style="padding-top:8px;border-top:1px solid var(--border-light);">${jumpHtml}</div>
                `;
            }
        }

        function jumpAndShowKline(sym) {
            if (!KLINE_MAP[sym]) return;
            document.getElementById("sec-kline-terminal").scrollIntoView({ behavior: "smooth", block: "start" });
            renderMainKline(sym);
        }

        function renderBacktestCharts() {
            if (!window.echarts || !BACKTEST_DATA) return;
            const c = getThemeColors();

            // 1. 20D Long-Short Portfolio NAV Curve
            const navDom = document.getElementById("chartBacktestNav");
            if (navDom) {
                let inst = chartInstances["btNav"] || echarts.init(navDom);
                chartInstances["btNav"] = inst;
                inst.setOption({
                    tooltip: { trigger: "axis", valueFormatter: (v) => (v !== undefined ? v + "%" : "") },
                    legend: {
                        data: ["多空对冲超额净值", "超配多头组合", "规避空头组合", "跨市场基准"],
                        top: 2,
                        textStyle: { color: c.text, fontSize: 11 }
                    },
                    grid: { left: 46, right: 18, top: 38, bottom: 26 },
                    xAxis: { type: "category", data: BACKTEST_DATA.nav_dates, axisLabel: { color: c.muted, fontSize: 11 } },
                    yAxis: { type: "value", axisLabel: { formatter: "{value}%", color: c.muted, fontSize: 11 }, splitLine: { lineStyle: { color: c.gridLine } } },
                    series: [
                        {
                            name: "多空对冲超额净值",
                            type: "line",
                            smooth: true,
                            data: BACKTEST_DATA.ls_alpha_curve,
                            lineStyle: { width: 3, color: "#d97706" },
                            itemStyle: { color: "#d97706" },
                            areaStyle: { color: "rgba(217, 119, 6, 0.12)" }
                        },
                        {
                            name: "超配多头组合",
                            type: "line",
                            smooth: true,
                            showSymbol: false,
                            data: BACKTEST_DATA.long_curve,
                            lineStyle: { width: 2, color: "#16a34a" },
                            itemStyle: { color: "#16a34a" }
                        },
                        {
                            name: "规避空头组合",
                            type: "line",
                            smooth: true,
                            showSymbol: false,
                            data: BACKTEST_DATA.short_curve,
                            lineStyle: { width: 2, type: "dashed", color: "#e11d48" },
                            itemStyle: { color: "#e11d48" }
                        },
                        {
                            name: "跨市场基准",
                            type: "line",
                            smooth: true,
                            showSymbol: false,
                            data: BACKTEST_DATA.bm_curve,
                            lineStyle: { width: 1.5, color: "#64748b" },
                            itemStyle: { color: "#64748b" }
                        }
                    ]
                }, true);
            }

            // 2. Hit Rate Breakdown Bar Chart
            const hitDom = document.getElementById("chartBacktestHit");
            if (hitDom) {
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
                inst.setOption({
                    tooltip: { trigger: "axis", valueFormatter: (v) => v + "%" },
                    grid: { left: 46, right: 20, top: 28, bottom: 32 },
                    xAxis: { type: "category", data: cats, axisLabel: { color: c.text, fontSize: 11, interval: 0 } },
                    yAxis: { type: "value", min: 0, max: 100, axisLabel: { formatter: "{value}%", color: c.muted, fontSize: 11 }, splitLine: { lineStyle: { color: c.gridLine } } },
                    series: [{
                        name: "实盘命中率",
                        type: "bar",
                        barWidth: "42%",
                        data: vals.map((v, idx) => ({
                            value: v,
                            itemStyle: {
                                color: idx === 4 ? "#059669" : (v >= 65 ? "#2563eb" : "#d97706"),
                                borderRadius: [4, 4, 0, 0]
                            },
                            label: { show: true, position: "top", formatter: "{c}%", color: c.text, fontWeight: "bold", fontSize: 11 }
                        })),
                        markLine: {
                            silent: true,
                            lineStyle: { color: "#e11d48", type: "dashed", width: 1.5 },
                            label: { formatter: "50% 随机基准线", fontSize: 10, color: "#e11d48" },
                            data: [{ yAxis: 50 }]
                        }
                    }]
                }, true);
            }
        }

        function filterBacktestRows(mode, btnEl) {
            const bar = btnEl ? btnEl.closest(".filter-toolbar") : null;
            if (bar) bar.querySelectorAll(".flt-btn").forEach(b => b.classList.remove("active"));
            if (btnEl) btnEl.classList.add("active");
            document.querySelectorAll(".bt-audit-row").forEach(row => {
                const h = row.getAttribute("data-hit");
                const idx = parseInt(row.getAttribute("data-idx") || "0", 10);
                let show = true;
                if (mode === "top18") show = (idx <= 18);
                else if (mode === "hit") show = (h === "hit");
                else if (mode === "miss") show = (h === "miss");
                row.style.display = show ? "" : "none";
            });
        }

        function renderFourOverviewCharts() {
            if (!window.echarts) return;
            const c = getThemeColors();

            // 1. Multi-Series Normalized Return Line Chart
            const normDom = document.getElementById("chartNormLine");
            if (normDom) {
                let inst = chartInstances["normLine"] || echarts.init(normDom);
                chartInstances["normLine"] = inst;
                const compareSyms = ["QQQ", "SPY", "TLT", "000001.SH", "FSLR", "WMT", "603259.SH", "601899.SH"].filter(s => KLINE_MAP[s]);
                const refDates = compareSyms.length ? KLINE_MAP[compareSyms[0]].dates : [];
                const series = compareSyms.map(s => ({
                    name: KLINE_MAP[s].name.split("(")[0].trim(),
                    type: "line",
                    smooth: true,
                    showSymbol: false,
                    data: KLINE_MAP[s].norm_pct.slice(-refDates.length),
                    lineStyle: { width: 2.2 }
                }));
                inst.setOption({
                    tooltip: { trigger: "axis", valueFormatter: (v) => (v !== undefined ? v + "%" : "") },
                    legend: { top: 2, textStyle: { color: c.text, fontSize: 11 } },
                    grid: { left: 46, right: 16, top: 44, bottom: 26 },
                    xAxis: { type: "category", data: refDates, axisLabel: { color: c.muted, fontSize: 11 } },
                    yAxis: { type: "value", axisLabel: { formatter: "{value}%", color: c.muted, fontSize: 11 }, splitLine: { lineStyle: { color: c.gridLine } } },
                    series
                }, true);
            }

            // 2. Grouped Sector Conviction & Real Return Column Chart
            const secDom = document.getElementById("chartSectorCol");
            if (secDom) {
                let inst = chartInstances["sectorCol"] || echarts.init(secDom);
                chartInstances["sectorCol"] = inst;
                inst.setOption({
                    tooltip: { trigger: "axis", axisPointer: { type: "shadow" } },
                    legend: { data: ["🇺🇸 美股冲击分(1-10)", "🇨🇳 A股冲击分(1-10)", "代表股5日均涨跌(%)"], top: 2, textStyle: { color: c.text, fontSize: 11 } },
                    grid: { left: 46, right: 46, top: 38, bottom: 42 },
                    xAxis: {
                        type: "category",
                        data: SECTOR_CHART_DATA.categories,
                        axisLabel: { color: c.text, fontSize: 11, interval: 0, rotate: 14 }
                    },
                    yAxis: [
                        { type: "value", name: "评分", min: 0, max: 10, axisLabel: { color: c.muted, fontSize: 11 }, splitLine: { lineStyle: { color: c.gridLine } } },
                        { type: "value", name: "5日涨幅", axisLabel: { formatter: "{value}%", color: c.muted, fontSize: 11 }, splitLine: { show: false } }
                    ],
                    series: [
                        { name: "🇺🇸 美股冲击分(1-10)", type: "bar", data: SECTOR_CHART_DATA.us_scores, itemStyle: { color: "#2563eb", borderRadius: [3,3,0,0] } },
                        { name: "🇨🇳 A股冲击分(1-10)", type: "bar", data: SECTOR_CHART_DATA.cn_scores, itemStyle: { color: "#059669", borderRadius: [3,3,0,0] } },
                        { name: "代表股5日均涨跌(%)", type: "line", yAxisIndex: 1, data: SECTOR_CHART_DATA.real_5d, symbolSize: 7, lineStyle: { width: 2.2, color: "#d97706" }, itemStyle: { color: "#d97706" } }
                    ]
                }, true);
            }

            // 3. Top Stock Diverging Bar Chart
            const barDom = document.getElementById("chartTopBar");
            if (barDom) {
                let inst = chartInstances["topBar"] || echarts.init(barDom);
                chartInstances["topBar"] = inst;
                const labels = DIVERGING_BAR_DATA.map(d => d.label);
                const scores = DIVERGING_BAR_DATA.map(d => ({
                    value: d.score,
                    itemStyle: { color: d.score >= 0 ? "#16a34a" : "#e11d48", borderRadius: 3 }
                }));
                const chg5d = DIVERGING_BAR_DATA.map(d => d.chg_5d);
                inst.setOption({
                    tooltip: { trigger: "axis", axisPointer: { type: "shadow" } },
                    legend: { data: ["多空评级强度 (-10~+10)", "实盘5日涨跌幅 (%)"], top: 2, textStyle: { color: c.text, fontSize: 11 } },
                    grid: { left: 135, right: 24, top: 34, bottom: 24 },
                    xAxis: { type: "value", splitLine: { lineStyle: { color: c.gridLine } }, axisLabel: { color: c.muted, fontSize: 11 } },
                    yAxis: { type: "category", data: labels, axisLabel: { color: c.text, fontSize: 11 } },
                    series: [
                        { name: "多空评级强度 (-10~+10)", type: "bar", data: scores },
                        { name: "实盘5日涨跌幅 (%)", type: "scatter", data: chg5d, symbolSize: 9, itemStyle: { color: "#2563eb" } }
                    ]
                }, true);
            }

            // 4. 4-Quadrant Article Impact Scatter Chart
            const scatDom = document.getElementById("chartQuadrantScatter");
            if (scatDom) {
                let inst = chartInstances["quadScatter"] || echarts.init(scatDom);
                chartInstances["quadScatter"] = inst;
                inst.setOption({
                    tooltip: {
                        formatter: (params) => {
                            const d = params.data;
                            return `<strong>#${d.idx} ${d.title}</strong><br/>赛道: ${d.category}<br/>🇺🇸 美股影响: ${d.value[0]}<br/>🇨🇳 A股影响: ${d.value[1]}<br/><em>点击气泡直达研报卡</em>`;
                        }
                    },
                    grid: { left: 46, right: 24, top: 28, bottom: 34 },
                    xAxis: {
                        type: "value", name: "🇺🇸 美股多空 (-10↔+10)", nameLocation: "middle", nameGap: 22,
                        min: -10, max: 10, splitLine: { lineStyle: { color: c.gridLine } }, axisLabel: { color: c.muted, fontSize: 11 }
                    },
                    yAxis: {
                        type: "value", name: "🇨🇳 A股多空 (-10↔+10)", min: -10, max: 10,
                        splitLine: { lineStyle: { color: c.gridLine } }, axisLabel: { color: c.muted, fontSize: 11 }
                    },
                    series: [{
                        type: "scatter",
                        data: SCATTER_POINTS.map(p => ({
                            value: [p.x, p.y],
                            idx: p.idx,
                            anchor: p.anchor,
                            title: p.title,
                            category: p.category,
                            symbolSize: p.size,
                            itemStyle: {
                                color: (p.y > 0 && p.x < 0) ? "#059669" : (p.y > 0 && p.x > 0 ? "#2563eb" : "#e11d48"),
                                opacity: 0.85
                            },
                            label: { show: true, formatter: "#" + p.idx, position: "inside", color: "#fff", fontSize: 10, fontWeight: "bold" }
                        })),
                        markLine: {
                            silent: true,
                            lineStyle: { color: c.muted, type: "dashed" },
                            data: [{ xAxis: 0 }, { yAxis: 0 }]
                        }
                    }]
                }, true);
                inst.off("click");
                inst.on("click", (params) => {
                    if (params.data && params.data.anchor) {
                        jumpToArticle(params.data.anchor);
                    }
                });
            }
        }"""
