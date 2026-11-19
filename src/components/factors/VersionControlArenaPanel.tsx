import React, { useState } from "react";
import { AlertTriangle, CheckCircle2, GitBranch, RotateCcw, Sparkles } from "lucide-react";
import type {
  FactorDiagnosticDto,
  VersionControlDashboardDto,
} from "../../types/terminal";
import { formatSignedPct } from "../../domain/uiInvariants";
import { SafeText } from "../common/SafeText";
import { AsyncGlassButton } from "../common/AsyncGlassButton";

export interface VersionControlArenaPanelProps {
  versionControl: VersionControlDashboardDto;
  factors: FactorDiagnosticDto[];
  onCompareVersions: (baseId: string, targetId: string) => Promise<void>;
  onSwitchVersion: (versionId: string) => Promise<void>;
  onWakeQuantDiagnosis?: (prompt: string) => Promise<void>;
}

export const VersionControlArenaPanel: React.FC<VersionControlArenaPanelProps> = ({
  versionControl,
  factors,
  onCompareVersions,
  onSwitchVersion,
  onWakeQuantDiagnosis,
}) => {
  const { activeVersionId, versions, comparison } = versionControl;
  const [baseId, setBaseId] = useState<string>(
    comparison?.baseVersionId ?? versions[0]?.versionId ?? "v1.0.0"
  );
  const [targetId, setTargetId] = useState<string>(
    comparison?.targetVersionId ?? activeVersionId
  );

  const factorNameMap = Object.fromEntries(factors.map((f) => [f.factorId, f.nameZh]));

  const handleSelectBase = (nextBase: string) => {
    setBaseId(nextBase);
    void onCompareVersions(nextBase, targetId);
  };

  const handleSelectTarget = (nextTarget: string) => {
    setTargetId(nextTarget);
    void onCompareVersions(baseId, nextTarget);
  };

  return (
    <section
      aria-label="策略与因子多版本影子竞技场"
      data-blur-slot="optical"
      className="glass-tier-2 stagger-enter stagger-3"
      style={{ display: "flex", flexDirection: "column", gap: "14px" }}
    >
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "8px",
        }}
      >
        <div>
          <strong style={{ fontSize: "15px" }}>
            策略与因子多版本影子竞技场（老版本持续平行打分对比）
          </strong>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            每次升级均保留全部历史版本在同一 40% 样本外窗口平行追踪，支持一键切换与落后四维归因找原因
          </div>
        </div>
        <span className="glass-pill pill-risk-clear tabular-nums">
          <GitBranch size={12} /> 当前生效 {activeVersionId}
        </span>
      </div>

      <div className="glass-table-wrap responsive-table-only">
        <table className="glass-table">
          <thead>
            <tr>
              <th style={{ width: "18%" }}>版本号 / 父谱系</th>
              <th style={{ width: "26%" }}>版本说明</th>
              <th style={{ width: "12%" }}>样本外 RankIC</th>
              <th style={{ width: "12%" }}>样本外净胜率</th>
              <th style={{ width: "16%" }}>累计净超额 / 回撤</th>
              <th style={{ width: "16%" }}>版本控制</th>
            </tr>
          </thead>
          <tbody>
            {versions.map((v) => {
              const retTone = v.oosCumNetRetBps >= 0 ? "text-mkt-up" : "text-mkt-down";
              return (
                <tr
                  key={v.versionId}
                  style={{
                    outline: v.isActive ? "1px solid var(--accent)" : undefined,
                  }}
                >
                  <td className="tabular-nums">
                    <strong>{v.versionId}</strong>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                      {v.parentVersionId ? `源自 ${v.parentVersionId}` : "创世基线"} · {v.atrStopMult.toFixed(1)}x ATR
                    </div>
                  </td>
                  <td>
                    <SafeText text={v.summaryZh} maxLines={2} showPopover />
                  </td>
                  <td className={`tabular-nums ${v.oosRankIcMean >= 0 ? "text-mkt-up" : "text-mkt-down"}`}>
                    {v.oosRankIcMean >= 0 ? "+" : ""}
                    {v.oosRankIcMean.toFixed(3)}
                  </td>
                  <td className="tabular-nums" style={{ fontWeight: 600 }}>
                    {v.oosWinRatePct.toFixed(1)}%
                  </td>
                  <td className="tabular-nums">
                    <span className={retTone} style={{ fontWeight: 700 }}>
                      {v.oosCumNetRetBps >= 0 ? "+" : ""}
                      {v.oosCumNetRetBps.toFixed(1)} bps
                    </span>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                      回撤 {v.oosMaxDrawdownBps.toFixed(1)} bps
                    </div>
                  </td>
                  <td>
                    {v.isActive ? (
                      <span className="glass-pill pill-risk-clear tabular-nums">
                        <CheckCircle2 size={12} /> 正在实盘扫描生效
                      </span>
                    ) : (
                      <AsyncGlassButton
                        size="sm"
                        variant="ghost"
                        dataTestId={`switch-version-${v.versionId}`}
                        label="切换设为当前版本"
                        loadingLabel="切换中..."
                        successLabel="已切换生效"
                        onClick={() => onSwitchVersion(v.versionId)}
                      />
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
        <div
          style={{
            display: "flex",
            flexWrap: "wrap",
            justifyContent: "space-between",
            alignItems: "center",
            gap: "10px",
          }}
        >
          <strong style={{ fontSize: "13.5px" }}>升级前后对比与退化归因（老版本基准 vs 目标新版本）</strong>
          <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "8px", fontSize: "12.5px" }}>
            <label style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span>基准老版本</span>
              <select
                data-testid="version-compare-base-select"
                value={comparison?.baseVersionId ?? baseId}
                onChange={(e) => handleSelectBase(e.target.value)}
                className="glass-input tabular-nums"
                style={{ minHeight: "30px", width: "130px" }}
              >
                {versions.map((v) => (
                  <option key={v.versionId} value={v.versionId}>
                    {v.versionId}
                  </option>
                ))}
              </select>
            </label>

            <span>→</span>

            <label style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span>对比目标版本</span>
              <select
                data-testid="version-compare-target-select"
                value={comparison?.targetVersionId ?? targetId}
                onChange={(e) => handleSelectTarget(e.target.value)}
                className="glass-input tabular-nums"
                style={{ minHeight: "30px", width: "130px" }}
              >
                {versions.map((v) => (
                  <option key={v.versionId} value={v.versionId}>
                    {v.versionId}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>

        {comparison && (
          <>
            <div
              className={`glass-pill ${
                comparison.isUnderperforming ? "pill-risk-veto" : "pill-risk-clear"
              }`}
              style={{ alignSelf: "flex-start", fontSize: "12px", padding: "6px 12px" }}
            >
              {comparison.isUnderperforming ? <AlertTriangle size={14} /> : <CheckCircle2 size={14} />}
              <span>{comparison.headlineVerdictZh}</span>
            </div>

            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))",
                gap: "8px",
              }}
            >
              <div className="glass-optical-card" style={{ padding: "8px 10px" }}>
                <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>ΔOOS 胜率提升</div>
                <strong
                  className={`tabular-nums ${comparison.winRateLiftPct >= 0 ? "text-mkt-up" : "text-mkt-down"}`}
                  style={{ fontSize: "17px" }}
                >
                  {formatSignedPct(comparison.winRateLiftPct)}
                </strong>
              </div>
              <div className="glass-optical-card" style={{ padding: "8px 10px" }}>
                <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>ΔOOS RankIC 增量</div>
                <strong
                  className={`tabular-nums ${comparison.rankIcLift >= 0 ? "text-mkt-up" : "text-mkt-down"}`}
                  style={{ fontSize: "17px" }}
                >
                  {comparison.rankIcLift >= 0 ? "+" : ""}
                  {comparison.rankIcLift.toFixed(4)}
                </strong>
              </div>
              <div className="glass-optical-card" style={{ padding: "8px 10px" }}>
                <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>Δ累计净超额收益</div>
                <strong
                  className={`tabular-nums ${comparison.netRetLiftBps >= 0 ? "text-mkt-up" : "text-mkt-down"}`}
                  style={{ fontSize: "17px" }}
                >
                  {comparison.netRetLiftBps >= 0 ? "+" : ""}
                  {comparison.netRetLiftBps.toFixed(1)} bps
                </strong>
              </div>
              <div className="glass-optical-card" style={{ padding: "8px 10px" }}>
                <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>回撤改善幅度</div>
                <strong
                  className={`tabular-nums ${comparison.drawdownImprovementBps >= 0 ? "text-risk-clear" : "text-stoploss"}`}
                  style={{ fontSize: "17px" }}
                >
                  {comparison.drawdownImprovementBps >= 0 ? "+" : ""}
                  {comparison.drawdownImprovementBps.toFixed(1)} bps
                </strong>
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "8px" }}>
              {comparison.factorAttributions.map((fa) => (
                <div key={fa.factorId} className="glass-optical-card tabular-nums" style={{ padding: "8px 10px", fontSize: "11.5px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "6px" }}>
                    <strong>{factorNameMap[fa.factorId] ?? fa.factorId}</strong>
                    <span className={fa.alphaContributionBps >= 0 ? "text-mkt-up" : "text-mkt-down"}>
                      贡献 {fa.alphaContributionBps >= 0 ? "+" : ""}
                      {fa.alphaContributionBps.toFixed(1)} bps
                    </span>
                  </div>
                  <div style={{ color: "var(--text-muted)", marginTop: "3px" }}>
                    权重 {fa.baseWeightPct.toFixed(1)}% → {fa.targetWeightPct.toFixed(1)}% ({fa.weightDeltaPct >= 0 ? "+" : ""}
                    {fa.weightDeltaPct.toFixed(1)}%) · 截面多空 {fa.factorOosSpreadBps.toFixed(0)} bps
                  </div>
                </div>
              ))}
            </div>

            {comparison.isUnderperforming && comparison.rootCauses.length > 0 && (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "8px",
                  padding: "12px",
                  borderRadius: "14px",
                  border: "1px solid rgba(248, 113, 113, 0.38)",
                  background: "rgba(127, 29, 29, 0.14)",
                }}
              >
                <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
                  <strong style={{ fontSize: "13.5px" }}>
                    版本退化四维归因诊断（为什么 {comparison.targetVersionId} 不如老版本 {comparison.baseVersionId}？）
                  </strong>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
                    <AsyncGlassButton
                      size="sm"
                      icon={<RotateCcw size={13} />}
                      label={`一键回退至优胜老版本 ${comparison.baseVersionId}`}
                      loadingLabel="回退切换中..."
                      successLabel="已切回老版本"
                      onClick={() => onSwitchVersion(comparison.baseVersionId)}
                    />
                    {onWakeQuantDiagnosis && (
                      <AsyncGlassButton
                        size="sm"
                        variant="ghost"
                        icon={<Sparkles size={13} />}
                        label="唤醒量化专家Agent深度复盘"
                        loadingLabel="唤醒 gemini-4-carbon..."
                        successLabel="已提交复盘"
                        onClick={() =>
                          onWakeQuantDiagnosis(
                            `请深度复盘策略版本 ${comparison.targetVersionId} 相比 ${comparison.baseVersionId} 退化 ${comparison.netRetLiftBps.toFixed(1)}bps 的四维原因并给出防过拟合修正方案`
                          )
                        }
                      />
                    )}
                  </div>
                </div>

                {comparison.rootCauses.map((rc, idx) => (
                  <div key={`${rc.category}-${idx}`} className="glass-optical-card" style={{ padding: "10px 12px", fontSize: "12px" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}>
                      <strong>
                        [{rc.category}] {rc.titleZh}
                      </strong>
                      <span className="tabular-nums text-mkt-down" style={{ fontWeight: 700 }}>
                        影响 {rc.impactBps.toFixed(1)} bps
                      </span>
                    </div>
                    <div style={{ color: "var(--text-secondary)", marginTop: "4px" }}>{rc.detailZh}</div>
                    <div style={{ color: "var(--accent)", marginTop: "4px" }}>修正建议：{rc.remediationZh}</div>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </section>
  );
};
