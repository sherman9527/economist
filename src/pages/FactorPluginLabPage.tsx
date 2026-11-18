import React, { useEffect, useState } from "react";
import { Cpu, Lock, SlidersHorizontal } from "lucide-react";
import type {
  FactorDiagnosticDto,
  MarketPolarity,
  StrategyPluginConfigDto,
  ThemeMode,
  UpdatePluginInputDto,
} from "../types/terminal";
import { SafeText } from "../components/common/SafeText";
import { AsyncGlassButton } from "../components/common/AsyncGlassButton";
import { useEChartsOption } from "../components/charts/useEChartsInstance";

export interface FactorPluginLabPageProps {
  pluginConfig: StrategyPluginConfigDto;
  theme: ThemeMode;
  polarity: MarketPolarity;
  isBlurActive: (slotId: string) => boolean;
  onSavePluginConfig: (input: UpdatePluginInputDto) => Promise<void>;
}

export const FactorPluginLabPage: React.FC<FactorPluginLabPageProps> = ({
  pluginConfig,
  theme,
  polarity,
  isBlurActive,
  onSavePluginConfig,
}) => {
  const [selectedFactorId, setSelectedFactorId] = useState<string>(
    pluginConfig.factors[0]?.factorId ?? "F1_ideal_reversal"
  );
  const [diagChartMode, setDiagChartMode] = useState<"quantile" | "decay">("quantile");

  const [projTurnover, setProjTurnover] = useState(pluginConfig.minIntradayProjectedTurnoverPct);
  const [insiderVeto, setInsiderVeto] = useState(pluginConfig.insiderSellVetoPct);
  const [holderVeto, setHolderVeto] = useState(pluginConfig.holderSurgeVetoPct);
  const [outflowVeto, setOutflowVeto] = useState(pluginConfig.mainOutflowVetoRatio);
  const [atrMult, setAtrMult] = useState(pluginConfig.atrStopMult);
  const [cfgPolarity, setCfgPolarity] = useState<MarketPolarity>(pluginConfig.marketPolarity);
  const [weights, setWeights] = useState<Record<string, number>>(() =>
    Object.fromEntries(pluginConfig.factors.map((f) => [f.factorId, f.weightPct]))
  );
  const [statusMsg, setStatusMsg] = useState<string | null>(null);

  useEffect(() => {
    setProjTurnover(pluginConfig.minIntradayProjectedTurnoverPct);
    setInsiderVeto(pluginConfig.insiderSellVetoPct);
    setHolderVeto(pluginConfig.holderSurgeVetoPct);
    setOutflowVeto(pluginConfig.mainOutflowVetoRatio);
    setAtrMult(pluginConfig.atrStopMult);
    setCfgPolarity(pluginConfig.marketPolarity);
    setWeights(Object.fromEntries(pluginConfig.factors.map((f) => [f.factorId, f.weightPct])));
    setSelectedFactorId((prev) =>
      pluginConfig.factors.some((f) => f.factorId === prev)
        ? prev
        : (pluginConfig.factors[0]?.factorId ?? "F1_ideal_reversal")
    );
  }, [pluginConfig]);

  const activeFactor: FactorDiagnosticDto | undefined =
    pluginConfig.factors.find((f) => f.factorId === selectedFactorId) ?? pluginConfig.factors[0];

  const diagChartRef = useEChartsOption(
    (palette) => {
      const isQ = diagChartMode === "quantile";
      const xLabels = isQ
        ? Array.from({ length: 10 }, (_, i) => `Q${i + 1}`)
        : Array.from({ length: 10 }, (_, i) => `${i + 1}D`);
      const rawData = isQ
        ? activeFactor?.quantileReturnsPct ?? []
        : activeFactor?.icDecay ?? [];

      return {
        backgroundColor: "transparent",
        tooltip: {
          trigger: "axis",
          backgroundColor: palette.tooltipBg,
          borderColor: palette.tooltipBorder,
          borderWidth: 1,
          textStyle: { color: palette.textPrimary, fontSize: 12 },
        },
        grid: { top: 24, left: 8, right: 12, bottom: 8, containLabel: true },
        xAxis: {
          type: "category",
          data: xLabels,
          axisLine: { lineStyle: { color: palette.gridLine } },
          axisLabel: { color: palette.textMuted, fontSize: 11, hideOverlap: true },
        },
        yAxis: {
          type: "value",
          splitLine: { lineStyle: { color: palette.gridLine, type: "dashed" } },
          axisLabel: {
            color: palette.textMuted,
            fontSize: 11,
            formatter: (v: number) => (isQ ? `${v}%` : v.toFixed(2)),
          },
        },
        series: [
          isQ
            ? {
                name: "分层超额收益(%)",
                type: "bar",
                barMaxWidth: 24,
                data: rawData.map((v) => ({
                  value: v,
                  itemStyle: {
                    color: v >= 0 ? palette.mktUp : palette.mktDown,
                    borderRadius: v >= 0 ? [8, 8, 0, 0] : [0, 0, 8, 8],
                  },
                })),
              }
            : {
                name: "Rank IC 衰减",
                type: "line",
                smooth: 0.3,
                lineStyle: { width: 2.5, color: palette.chart1 },
                itemStyle: { color: palette.chart1 },
                data: rawData,
              },
        ],
      };
    },
    [activeFactor, diagChartMode],
    theme,
    polarity
  );

  const weightSum = Object.values(weights).reduce((a, b) => a + b, 0);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px", minWidth: 0 }}>
      {/* Top Row: 6-Factor Diagnostics Leaderboard + Q1-Q10 / IC Decay Chart */}
      <div className="bento-grid-12">
        <section
          aria-label="六维量化因子诊断榜单"
          data-blur-slot={isBlurActive("factor-leaderboard") ? "active" : "optical"}
          className="col-span-7 glass-tier-2 stagger-enter stagger-1"
          style={{ display: "flex", flexDirection: "column", gap: "12px" }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
            <div>
              <strong style={{ fontSize: "15px" }}>量化专家 Agent 六维核心因子库</strong>
              <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                清单路径 `./plugins/factors/core_factors_v1.json` · 点击切换右侧分层/衰减曲线
              </div>
            </div>
            <Cpu size={16} style={{ color: "var(--accent)", flexShrink: 0 }} />
          </div>

          <div className="glass-table-wrap responsive-table-only">
            <table className="glass-table">
              <thead>
                <tr>
                  <th style={{ width: "26%" }}>因子名称 / 证据标签</th>
                  <th style={{ width: "13%" }}>周期</th>
                  <th style={{ width: "14%" }}>Rank IC</th>
                  <th style={{ width: "13%" }}>IC_IR</th>
                  <th style={{ width: "14%" }}>OOS胜率</th>
                  <th style={{ width: "20%" }}>表达式预览</th>
                </tr>
              </thead>
              <tbody>
                {pluginConfig.factors.map((f) => {
                  const selected = f.factorId === activeFactor?.factorId;
                  return (
                    <tr
                      key={f.factorId}
                      onClick={() => setSelectedFactorId(f.factorId)}
                      style={{
                        cursor: "pointer",
                        outline: selected ? "1px solid var(--accent)" : undefined,
                      }}
                    >
                      <td>
                        <SafeText text={f.nameZh} as="strong" />
                        <span
                          className={`glass-pill ${
                            f.evidenceTag === "[OOS]" ? "pill-risk-clear" : ""
                          }`}
                          style={{ minHeight: "20px", fontSize: "10.5px", marginTop: "3px" }}
                        >
                          {f.evidenceTag}
                        </span>
                      </td>
                      <td className="tabular-nums">{f.barFreq}</td>
                      <td className={`tabular-nums ${f.rankIcMean >= 0 ? "text-mkt-up" : "text-mkt-down"}`}>
                        {f.rankIcMean >= 0 ? "+" : ""}
                        {f.rankIcMean.toFixed(3)}
                      </td>
                      <td className="tabular-nums">{f.icIr.toFixed(2)}</td>
                      <td className="tabular-nums">{f.oosWinRatePct.toFixed(1)}%</td>
                      <td>
                        <SafeText text={f.expr} showPopover />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <div className="responsive-cards-only">
            {pluginConfig.factors.map((f) => {
              const selected = f.factorId === activeFactor?.factorId;
              return (
                <div
                  key={f.factorId}
                  onClick={() => setSelectedFactorId(f.factorId)}
                  className="glass-optical-card glass-optical-card-interactive"
                  style={{
                    cursor: "pointer",
                    outline: selected ? "1px solid var(--accent)" : undefined,
                    display: "flex",
                    flexDirection: "column",
                    gap: "6px",
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}>
                    <SafeText text={f.nameZh} as="strong" />
                    <span className="glass-pill pill-risk-clear tabular-nums">{f.evidenceTag}</span>
                  </div>
                  <div className="tabular-nums" style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                    周期 {f.barFreq} · RankIC {f.rankIcMean >= 0 ? "+" : ""}
                    {f.rankIcMean.toFixed(3)} · IR {f.icIr.toFixed(2)} · OOS胜率{" "}
                    {f.oosWinRatePct.toFixed(1)}%
                  </div>
                  <SafeText text={f.expr} showPopover />
                </div>
              );
            })}
          </div>
        </section>

        <section
          aria-label="因子十分位收益与IC衰减图"
          data-blur-slot={isBlurActive("factor-chart") ? "active" : "optical"}
          className="col-span-5 glass-tier-2 stagger-enter stagger-2"
          style={{ display: "flex", flexDirection: "column", gap: "10px" }}
        >
          <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
            <div className="min-w-zero">
              <SafeText text={`${activeFactor?.nameZh ?? ""} · 截面检验`} as="strong" />
              <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
                Q1–Q10 单调性检验 &amp; 1–10日持有期 IC 衰减
              </div>
            </div>
            <div className="glass-segmented">
              <button
                type="button"
                className="glass-seg-btn"
                aria-pressed={diagChartMode === "quantile"}
                onClick={() => setDiagChartMode("quantile")}
              >
                Q1–Q10分层
              </button>
              <button
                type="button"
                className="glass-seg-btn"
                aria-pressed={diagChartMode === "decay"}
                onClick={() => setDiagChartMode("decay")}
              >
                1–10d衰减
              </button>
            </div>
          </div>

          <div ref={diagChartRef} style={{ width: "100%", height: "260px" }} />

          {activeFactor && (
            <div className="glass-optical-card" style={{ fontSize: "12px" }}>
              <div style={{ color: "var(--text-muted)" }}>截面算子公式</div>
              <code style={{ display: "block", marginTop: "4px", wordBreak: "break-all", color: "var(--accent)" }}>
                {activeFactor.expr}
              </code>
            </div>
          )}
        </section>
      </div>

      {/* Bottom Row: Parameterized Plugin Config Editor & Core Strategy Locks */}
      <section
        aria-label="参数化策略插件热加载中心"
        data-blur-slot={isBlurActive("factor-plugin-editor") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-3"
        style={{ display: "flex", flexDirection: "column", gap: "14px" }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
          <div>
            <strong style={{ fontSize: "15px" }}>
              参数化策略插件配置器 (`{pluginConfig.manifestRelPath}`)
            </strong>
            <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
              核心策略约束硬锁定，排雷否决阈值与截面因子权重支持免重编译热更新
            </div>
          </div>

          <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
            <span className="glass-pill pill-risk-stoploss tabular-nums">
              <Lock size={11} /> 市值 &lt;{pluginConfig.maxTotalMvYi}亿
            </span>
            <span className="glass-pill pill-risk-stoploss tabular-nums">
              <Lock size={11} /> 换手 &gt;{pluginConfig.minTurnoverPct}% × {pluginConfig.consecutiveDays}D
            </span>
            <span className="glass-pill pill-risk-stoploss tabular-nums">
              <Lock size={11} /> 本金 100W · &le;{pluginConfig.maxPositions}只
            </span>
          </div>
        </div>

        <div className="bento-grid-12">
          <div className="col-span-7" style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "10px" }}>
            <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
              <span>盘中预测日换手下限 (%)</span>
              <input
                type="number"
                step="0.2"
                min="5.0"
                max="20.0"
                value={projTurnover}
                onChange={(e) => setProjTurnover(Number(e.target.value))}
                className="glass-input tabular-nums"
              />
            </label>

            <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
              <span>大股东拟减持一票否决阈值 (%)</span>
              <input
                type="number"
                step="0.1"
                min="0.1"
                max="5.0"
                value={insiderVeto}
                onChange={(e) => setInsiderVeto(Number(e.target.value))}
                className="glass-input tabular-nums"
              />
            </label>

            <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
              <span>PIT股东户数单期激增否决 (%)</span>
              <input
                type="number"
                step="1"
                min="5"
                max="60"
                value={holderVeto}
                onChange={(e) => setHolderVeto(Number(e.target.value))}
                className="glass-input tabular-nums"
              />
            </label>

            <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
              <span>主力净流出占成交否决比</span>
              <input
                type="number"
                step="0.01"
                min="-0.3"
                max="-0.01"
                value={outflowVeto}
                onChange={(e) => setOutflowVeto(Number(e.target.value))}
                className="glass-input tabular-nums"
              />
            </label>

            <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
              <span>ATR(14) 动态止损乘数</span>
              <input
                type="number"
                step="0.1"
                min="1.0"
                max="4.0"
                value={atrMult}
                onChange={(e) => setAtrMult(Number(e.target.value))}
                className="glass-input tabular-nums"
              />
            </label>

            <div className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
              <span>默认行情色彩极性</span>
              <div className="glass-segmented">
                <button
                  type="button"
                  className="glass-seg-btn"
                  aria-pressed={cfgPolarity === "cn-red-up"}
                  onClick={() => setCfgPolarity("cn-red-up")}
                >
                  A股红涨
                </button>
                <button
                  type="button"
                  className="glass-seg-btn"
                  aria-pressed={cfgPolarity === "intl-green-up"}
                  onClick={() => setCfgPolarity("intl-green-up")}
                >
                  国际绿涨
                </button>
              </div>
            </div>
          </div>

          <div className="col-span-5 glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            <div className="tabular-nums" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "6px", fontSize: "12.5px" }}>
              <strong>六维因子权重配比 (%)</strong>
              <span style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                <span className={weightSum === 100 ? "text-risk-clear" : "text-stoploss"}>
                  合计 {weightSum}%
                </span>
                {weightSum > 0 && weightSum !== 100 && (
                  <button
                    type="button"
                    className="glass-btn glass-btn-ghost glass-btn-sm"
                    style={{ minHeight: "22px", paddingInline: "6px", fontSize: "11px" }}
                    onClick={() => {
                      const entries = pluginConfig.factors.map((f) => [
                        f.factorId,
                        Math.round(((weights[f.factorId] ?? f.weightPct) / weightSum) * 100),
                      ] as const);
                      setWeights(Object.fromEntries(entries));
                    }}
                  >
                    归一100%
                  </button>
                )}
              </span>
            </div>
            {pluginConfig.factors.map((f) => (
              <label
                key={f.factorId}
                style={{
                  display: "grid",
                  gridTemplateColumns: "minmax(0, 1fr) 78px",
                  alignItems: "center",
                  gap: "8px",
                  fontSize: "12px",
                }}
              >
                <SafeText text={f.nameZh} />
                <input
                  type="number"
                  min={0}
                  max={100}
                  step={5}
                  value={weights[f.factorId] ?? f.weightPct}
                  onChange={(e) =>
                    setWeights((prev) => ({
                      ...prev,
                      [f.factorId]: Number(e.target.value) || 0,
                    }))
                  }
                  className="glass-input tabular-nums"
                  style={{ minHeight: "30px", paddingBlock: "3px" }}
                />
              </label>
            ))}
          </div>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "10px" }}>
          <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            {statusMsg ?? "同步写入 SQLite strategy_configs 与 ./plugins/strategies/smallcap_turnover_ignition.json"}
          </span>
          <AsyncGlassButton
            label="校验并热加载至SQLite/插件"
            loadingLabel="校验并写入插件..."
            successLabel="插件热加载生效"
            disabled={weightSum <= 0}
            icon={<SlidersHorizontal size={15} />}
            onClick={async () => {
              await onSavePluginConfig({
                minIntradayProjectedTurnoverPct: projTurnover,
                insiderSellVetoPct: insiderVeto,
                holderSurgeVetoPct: holderVeto,
                mainOutflowVetoRatio: outflowVeto,
                atrStopMult: atrMult,
                marketPolarity: cfgPolarity,
                factorWeights: weights,
              });
              setStatusMsg(`已于 ${new Date().toLocaleTimeString()} 完成策略插件热加载（权重合计 ${weightSum}%）`);
            }}
          />
        </div>
      </section>
    </div>
  );
};
