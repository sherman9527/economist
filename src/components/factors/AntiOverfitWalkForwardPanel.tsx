import React from "react";
import { ShieldAlert, Sparkles } from "lucide-react";
import type { EvolutionAuditLogDto } from "../../types/terminal";
import { formatSignedPct } from "../../domain/uiInvariants";
import { AsyncGlassButton } from "../common/AsyncGlassButton";

export interface AntiOverfitWalkForwardPanelProps {
  activeVersionId: string;
  logs: EvolutionAuditLogDto[];
  onRunIteration: () => Promise<void>;
}

export const AntiOverfitWalkForwardPanel: React.FC<AntiOverfitWalkForwardPanelProps> = ({
  activeVersionId,
  logs,
  onRunIteration,
}) => {
  const guards = [
    "1. 60% 训练 → 5个交易日隔离带 (Purged Embargo) → 40% 纯样本外验证，根除5日重叠收益泄漏",
    "2. 样本内外同号硬约束且 OOS RankIC >= +0.012，反向或噪音波动一律否决",
    "3. 多重检验惩罚 Deflated IC_IR > 0.25 且过拟合概率 PBO <= 35% (Bailey-López de Prado)",
    "4. 截面共线性抑制 (|ρ| > 0.65 自动降权) + AST 公式算子复杂度惩罚 (>12 节点拒绝膨胀)",
    "5. James-Stein 贝叶斯收缩 (λ = 0.25) + 单轮权重调幅 <= ±3.0% + 单因子上下界 [5%, 35%]",
  ];

  return (
    <section
      aria-label="防过拟合滚动验证迭代引擎"
      data-blur-slot="optical"
      className="glass-tier-2 stagger-enter stagger-3"
      style={{ display: "flex", flexDirection: "column", gap: "12px" }}
    >
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "10px",
        }}
      >
        <div>
          <strong style={{ fontSize: "15px" }}>
            严禁过拟合 · 滚动样本外策略与因子自进化台 (基线 {activeVersionId})
          </strong>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            每日 15:00 收盘自动执行 + 支持手动即时触发；未通过五重防过拟合门禁绝不篡改当前实盘版本
          </div>
        </div>

        <AsyncGlassButton
          dataTestId="run-anti-overfit-btn"
          icon={<Sparkles size={14} />}
          label="执行防过拟合滚动验证迭代"
          loadingLabel="60%IS/5D隔离/40%OOS检验中..."
          successLabel="滚动检验完成"
          onClick={onRunIteration}
        />
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
          gap: "8px",
        }}
      >
        {guards.map((g) => (
          <div key={g} className="glass-optical-card" style={{ padding: "8px 10px", fontSize: "11.5px", color: "var(--text-secondary)" }}>
            {g}
          </div>
        ))}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
        <strong style={{ fontSize: "13px" }}>历史滚动迭代审计日志（保留每一次晋升与过拟合拦截证据）</strong>
        {logs.map((log) => {
          const promoted = log.action === "PROMOTED";
          const insufficient = log.action === "INSUFFICIENT_SAMPLE";
          return (
            <div
              key={log.logId}
              className="glass-optical-card"
              style={{
                display: "flex",
                flexDirection: "column",
                gap: "5px",
                padding: "10px 12px",
                fontSize: "12px",
              }}
            >
              <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span className={`glass-pill ${promoted ? "pill-risk-clear" : "pill-risk-stoploss"} tabular-nums`}>
                    <ShieldAlert size={11} />
                    <span>
                      {promoted
                        ? `通过门禁晋升 → ${log.resultVersionId ?? log.baseVersionId}`
                        : insufficient
                        ? "样本截面不足 (严禁缩短5日隔离带)"
                        : "防过拟合门禁拦截 (保持稳健原版)"}
                    </span>
                  </span>
                  <span className="tabular-nums" style={{ color: "var(--text-muted)", fontSize: "11.5px" }}>
                    基线 {log.baseVersionId} · 触发源 {log.triggerSource}
                  </span>
                </div>

                <div className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-secondary)" }}>
                  PBO {(log.pboEstimate * 100).toFixed(1)}% · Deflated IR {log.deflatedIcIr.toFixed(2)} · OOS IC{" "}
                  {log.oosRankIc >= 0 ? "+" : ""}
                  {log.oosRankIc.toFixed(3)} · Δ胜率 {formatSignedPct(log.oosWinRateLiftPct)}
                </div>
              </div>
              <div style={{ color: "var(--text-primary)" }}>{log.reasonZh}</div>
            </div>
          );
        })}
      </div>
    </section>
  );
};
