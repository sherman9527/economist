import React, { useMemo, useState } from "react";
import { Bot, CheckCircle2, HardDrive, Sparkles, Trash2 } from "lucide-react";
import type {
  AgentSessionDto,
  TelemetryDashboardDto,
  TelemetryEventDto,
} from "../types/terminal";
import { SafeText } from "../components/common/SafeText";
import { AsyncGlassButton } from "../components/common/AsyncGlassButton";

export interface TelemetrySessionsPageProps {
  telemetry: TelemetryDashboardDto;
  isBlurActive: (slotId: string) => boolean;
  onRunHygiene: () => Promise<void>;
  onWakeSession: (
    role: AgentSessionDto["agentRole"],
    prompt: string
  ) => Promise<void>;
}

const ROLE_LABELS_ZH: Record<AgentSessionDto["agentRole"], string> = {
  main_supervisor: "主控调度 Supervisor",
  quant_researcher: "量化因子专家 Agent",
  ui_designer: "Liquid Glass 视觉专家",
  risk_auditor: "五维排雷风控审计 Agent",
  economist_analyst: "《经济学人》跨市场分析师",
};

export const TelemetrySessionsPage: React.FC<TelemetrySessionsPageProps> = ({
  telemetry,
  isBlurActive,
  onRunHygiene,
  onWakeSession,
}) => {
  const [subsystemFilter, setSubsystemFilter] = useState<
    "ALL" | TelemetryEventDto["subsystem"]
  >("ALL");
  const [wakePrompt, setWakePrompt] = useState<string>(
    "请增量复核今日30m换手放量标的五维排雷与止损盈亏比"
  );
  const [wakeNotice, setWakeNotice] = useState<string | null>(null);

  const { storage, sessions, events } = telemetry;
  const usedGb = storage.buildFootprintBytes / 1_073_741_824;
  const capGb = storage.buildCapBytes / 1_073_741_824;
  const sqliteMb = storage.sqliteDbBytes / 1_048_576;
  const quotaRatioPct = Math.min(
    100,
    (storage.buildFootprintBytes / Math.max(1, storage.buildCapBytes)) * 100
  );

  const filteredEvents = useMemo(
    () =>
      events.filter((e) =>
        subsystemFilter === "ALL" ? true : e.subsystem === subsystemFilter
      ),
    [events, subsystemFilter]
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px", minWidth: 0 }}>
      {/* Row 1: Portable Storage < 5 GB Gauge & Engineering Redline Compliance */}
      <section
        aria-label="便携构建空间水位与红线门禁状态"
        data-blur-slot={isBlurActive("telemetry-storage") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-1"
        style={{ display: "flex", flexDirection: "column", gap: "12px" }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "10px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <HardDrive size={17} style={{ color: "var(--accent)" }} />
            <strong style={{ fontSize: "15px" }}>
              便携式运行目录 `{storage.portableRootRel}` 空间配额与工程红线自检
            </strong>
          </div>

          <div style={{ display: "flex", flexWrap: "wrap", gap: "8px", alignItems: "center" }}>
            <span className="glass-pill pill-risk-clear tabular-nums">
              <CheckCircle2 size={12} />
              <span>
                {storage.zeroAbsolutePathVerified ? "0 硬编码绝对路径" : "路径告警"} · 单文件峰值{" "}
                {storage.maxFileLinesObserved} 行 (&lt;1000)
              </span>
            </span>

            <AsyncGlassButton
              size="sm"
              label="立即执行清理与28d日志巡检"
              loadingLabel="巡检清理中..."
              successLabel="水位清理已完成"
              icon={<Trash2 size={14} />}
              onClick={onRunHygiene}
            />
          </div>
        </div>

        <div className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <div className="tabular-nums" style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", gap: "8px", fontSize: "12.5px" }}>
            <span>
              构建缓存与产物占用：<strong>{usedGb.toFixed(2)} GB</strong> / 硬红线 {capGb.toFixed(2)} GB ({quotaRatioPct.toFixed(1)}%)
            </span>
            <span>
              内置 SQLite WAL 库体积：{sqliteMb.toFixed(1)} MB · 日志保留期 {storage.retentionDays} 天
            </span>
          </div>
          <div
            style={{
              width: "100%",
              minHeight: "9px",
              borderRadius: "999px",
              background: "rgba(148, 163, 184, 0.16)",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                width: `${Math.max(4, quotaRatioPct).toFixed(1)}%`,
                minHeight: "9px",
                borderRadius: "999px",
                background:
                  quotaRatioPct > 80 ? "var(--risk-veto)" : "var(--accent)",
              }}
            />
          </div>
        </div>
      </section>

      {/* Row 2: Persistent SQLite agent_sessions Matrix (gemini-4-carbon · high) */}
      <section
        aria-label="持久化专家Agent会话矩阵"
        data-blur-slot={isBlurActive("telemetry-sessions") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-2"
        style={{ display: "flex", flexDirection: "column", gap: "12px" }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "10px" }}>
          <div>
            <strong style={{ fontSize: "15px" }}>
              SQLite `agent_sessions` 持久化多专家会话中心
            </strong>
            <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
              全员锁定 `gemini-4-carbon · high` 推理强度，支持按原 UUID `--resume` 唤醒上下文持续迭代
            </div>
          </div>
          {wakeNotice && <span className="glass-pill pill-risk-clear">{wakeNotice}</span>}
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: "8px", alignItems: "center" }}>
          <input
            type="text"
            value={wakePrompt}
            onChange={(e) => setWakePrompt(e.target.value)}
            aria-label="唤醒专家Agent增量指令"
            placeholder="输入增量演进或复核指令，点击下方对应专家 [唤醒原Session迭代]..."
            className="glass-input"
            style={{ flex: "1 1 320px" }}
          />
        </div>

        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 275px), 1fr))",
            gap: "12px",
          }}
        >
          {sessions.map((sess) => (
            <div
              key={sess.agentRole}
              className="glass-optical-card"
              style={{ display: "flex", flexDirection: "column", gap: "8px", minWidth: 0 }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
                <span style={{ display: "flex", alignItems: "center", gap: "6px", fontWeight: 700, fontSize: "13.5px" }}>
                  <Bot size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
                  <SafeText text={ROLE_LABELS_ZH[sess.agentRole]} />
                </span>
                <span className="glass-pill pill-risk-clear tabular-nums" style={{ fontSize: "11px" }}>
                  {sess.model} · {sess.reasoningEffort}
                </span>
              </div>

              <div className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-secondary)" }}>
                <SafeText text={`UUID: ${sess.sessionId}`} showPopover />
                <SafeText text={`规范契约: ${sess.specRelPath}`} />
                <div>
                  累计轮次 {sess.turnCount} turns · 状态 {sess.status}
                </div>
              </div>

              <div style={{ display: "flex", justifyContent: "flex-end", marginTop: "2px" }}>
                <AsyncGlassButton
                  size="sm"
                  label="唤醒原Session迭代"
                  loadingLabel="Resume唤醒中..."
                  successLabel="已完成增量轮次"
                  icon={<Sparkles size={13} />}
                  onClick={async () => {
                    await onWakeSession(sess.agentRole, wakePrompt);
                    setWakeNotice(`已通过原 Session (${sess.sessionId.slice(0, 8)}...) 唤醒 ${ROLE_LABELS_ZH[sess.agentRole]}`);
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Row 3: Filterable 28-Day Structured Telemetry Stream */}
      <section
        aria-label="28天全链路结构化Telemetry日志流"
        data-blur-slot={isBlurActive("telemetry-events") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-3"
        style={{ display: "flex", flexDirection: "column", gap: "12px" }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
          <strong style={{ fontSize: "15px" }}>28 天结构化 Telemetry 事件审计流</strong>

          <div className="glass-segmented" role="group" aria-label="子系统过滤">
            {(
              ["ALL", "MARKET", "QUANT", "TRADING", "AGENT", "NEWS", "CLEANER", "UI"] as const
            ).map(
              (sub) => (
                <button
                  key={sub}
                  type="button"
                  className="glass-seg-btn"
                  aria-pressed={subsystemFilter === sub}
                  onClick={() => setSubsystemFilter(sub)}
                >
                  {sub}
                </button>
              )
            )}
          </div>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          {filteredEvents.map((ev) => (
            <div
              key={`${ev.eventId}-${ev.tsIso}`}
              className="glass-optical-card"
              style={{
                display: "flex",
                flexWrap: "wrap",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "8px",
                padding: "10px 14px",
              }}
            >
              <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "8px", minWidth: 0, flex: "1 1 340px" }}>
                <span
                  className={`glass-pill ${
                    ev.severity === "ERROR" || ev.severity === "CRITICAL"
                      ? "pill-risk-veto"
                      : ev.severity === "WARN"
                        ? "pill-risk-stoploss"
                        : "pill-risk-clear"
                  }`}
                >
                  {ev.subsystem} · {ev.severity}
                </span>
                <code style={{ fontSize: "12px", color: "var(--accent)" }}>{ev.eventName}</code>
                <SafeText text={ev.message} maxLines={2} showPopover />
              </div>

              <span className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
                {ev.durationMs != null ? `${ev.durationMs}ms · ` : ""}
                {ev.tsIso.replace("T", " ").slice(0, 19)}
              </span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
};
