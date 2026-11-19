import React, { useEffect, useState } from "react";
import { Cpu } from "lucide-react";
import type {
  FactorDiagnosticDto,
  MarketPolarity,
  StrategyPluginConfigDto,
  ThemeMode,
  UpdatePluginInputDto,
  VersionControlDashboardDto,
} from "../types/terminal";
import { SafeText } from "../components/common/SafeText";
import { useEChartsOption } from "../components/charts/useEChartsInstance";
import { StrategyPluginEditorSection } from "../components/factors/StrategyPluginEditorSection";
import { VersionControlArenaPanel } from "../components/factors/VersionControlArenaPanel";
import { AntiOverfitWalkForwardPanel } from "../components/factors/AntiOverfitWalkForwardPanel";
import { createMockVersionControl } from "../services/mockEvolutionFixtures";

export interface FactorPluginLabPageProps {
  pluginConfig: StrategyPluginConfigDto;
  versionControl?: VersionControlDashboardDto;
  theme: ThemeMode;
  polarity: MarketPolarity;
  isBlurActive: (slotId: string) => boolean;
  onSavePluginConfig: (input: UpdatePluginInputDto) => Promise<void>;
  onCompareVersions?: (baseId: string, targetId: string) => Promise<void>;
  onSwitchVersion?: (versionId: string) => Promise<void>;
  onRunAntiOverfitIteration?: () => Promise<void>;
  onWakeQuantDiagnosis?: (prompt: string) => Promise<void>;
}

const DEFAULT_VERSION_CONTROL = createMockVersionControl();
const NOOP_COMPARE = async () => undefined;
const NOOP_SWITCH = async () => undefined;
const NOOP_ITERATE = async () => undefined;

export const FactorPluginLabPage: React.FC<FactorPluginLabPageProps> = ({
  pluginConfig,
  versionControl = DEFAULT_VERSION_CONTROL,
  theme,
  polarity,
  isBlurActive,
  onSavePluginConfig,
  onCompareVersions = NOOP_COMPARE,
  onSwitchVersion = NOOP_SWITCH,
  onRunAntiOverfitIteration = NOOP_ITERATE,
  onWakeQuantDiagnosis,
}) => {
  const [selectedFactorId, setSelectedFactorId] = useState<string>(
    pluginConfig.factors[0]?.factorId ?? "F1_ideal_reversal"
  );
  const [diagChartMode, setDiagChartMode] = useState<"quantile" | "decay">("quantile");

  useEffect(() => {
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

      {/* Multi-Version Shadow Arena, Version Switcher & Degradation Root-Cause Diagnosis */}
      <VersionControlArenaPanel
        versionControl={versionControl}
        factors={pluginConfig.factors}
        onCompareVersions={onCompareVersions}
        onSwitchVersion={onSwitchVersion}
        onWakeQuantDiagnosis={onWakeQuantDiagnosis}
      />

      {/* Purged Walk-Forward Anti-Overfitting Iteration Engine */}
      <AntiOverfitWalkForwardPanel
        activeVersionId={versionControl.activeVersionId}
        logs={versionControl.evolutionLogs}
        onRunIteration={onRunAntiOverfitIteration}
      />

      {/* Parameterized Plugin Config Editor & Core Strategy Locks */}
      <StrategyPluginEditorSection
        pluginConfig={pluginConfig}
        blurActive={isBlurActive("factor-plugin-editor")}
        onSavePluginConfig={onSavePluginConfig}
      />
    </div>
  );
};
