import React, { useMemo, useState } from "react";
import { Bot, CheckCircle2, Cpu, HardDrive, Network, Sparkles, Trash2 } from "lucide-react";
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
  main_supervisor: "主控调度总管",
  quant_researcher: "量化因子研究专家",
  ui_designer: "液态玻璃视觉专家",
  risk_auditor: "五维排雷风控审计",
  economist_analyst: "《经济学人》跨市场分析师",
};

const ROLE_DUTY_ZH: Record<AgentSessionDto["agentRole"], string> = {
  main_supervisor: "统筹跨市场研报、量化选股、风控门禁与版本演进",
  quant_researcher: "截面正交因子评估、滚动防过拟合与权重调优",
  ui_designer: "光学玻璃层级预算、K线交互与零裁切排版巡检",
  risk_auditor: "减持/立案/筹码发散/主力流出五维一票否决复核",
  economist_analyst: "全球宏观政经脉络提炼与 A 股/美股产业链映射",
};

const SESSION_STATUS_ZH: Record<AgentSessionDto["status"], string> = {
  IDLE: "待命",
  RUNNING: "运行中",
  DEGRADED: "降级告警",
  RECOVERING: "恢复重连",
};

function formatCompactUuid(uuid: string): string {
  return uuid.length > 18 ? `${uuid.slice(0, 8)}…${uuid.slice(-6)}` : uuid;
}

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

      {/* Row 2: Persistent SQLite agent_sessions Matrix (1 Supervisor + 4 Specialists, single unified model badge) */}
      <section
        aria-label="持久化专家Agent会话矩阵"
        data-blur-slot={isBlurActive("telemetry-sessions") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-2"
        style={{ display: "flex", flexDirection: "column", gap: "12px" }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "10px" }}>
          <div style={{ minWidth: 0 }}>
            <strong style={{ fontSize: "15px" }}>
              多专家持久化会话协同矩阵
            </strong>
            <div style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "2px" }}>
              全员共享统一推理底座，支持按原会话 UUID 断点续接上下文持续迭代
            </div>
          </div>

          <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "8px" }}>
            {sessions[0] && (
              <span
                data-unified-model-badge="true"
                className="glass-pill pill-risk-clear tabular-nums"
                title="所有专家会话固定使用同一高推理强度模型底座，无需逐卡重复标注"
              >
                <Cpu size={13} style={{ flexShrink: 0 }} />
                <span>
                  统一底座 · {sessions[0].model} ({sessions[0].reasoningEffort})
                </span>
              </span>
            )}
            {wakeNotice && <span className="glass-pill pill-risk-clear">{wakeNotice}</span>}
          </div>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: "8px", alignItems: "center" }}>
          <input
            type="text"
            value={wakePrompt}
            onChange={(e) => setWakePrompt(e.target.value)}
            aria-label="唤醒专家Agent增量指令"
            placeholder="输入增量演进或复核指令，点击下方对应专家 [唤醒原会话迭代]..."
            className="glass-input"
            style={{ flex: "1 1 320px" }}
          />
        </div>

        {(() => {
          const supervisor = sessions.find((s) => s.agentRole === "main_supervisor");
          const specialists = sessions.filter((s) => s.agentRole !== "main_supervisor");

          return (
            <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
              {supervisor && (
                <div
                  data-supervisor-banner="true"
                  data-agent-card={supervisor.agentRole}
                  className="glass-optical-card"
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: "10px 16px",
                    padding: "12px 16px",
                    borderLeft: "3px solid var(--accent)",
                  }}
                >
                  <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "10px", minWidth: 0, flex: "1 1 280px" }}>
                    <span style={{ display: "inline-flex", alignItems: "center", gap: "7px", fontWeight: 700, fontSize: "14px" }}>
                      <Network size={16} style={{ color: "var(--accent)", flexShrink: 0 }} />
                      <span>{ROLE_LABELS_ZH[supervisor.agentRole]}</span>
                    </span>
                    <span
                      className={`glass-pill tabular-nums ${
                        supervisor.status === "DEGRADED"
                          ? "pill-risk-veto"
                          : supervisor.status === "RECOVERING"
                            ? "pill-risk-stoploss"
                            : "pill-risk-clear"
                      }`}
                      style={{ fontSize: "11px", minHeight: "23px", paddingInline: "8px" }}
                    >
                      {SESSION_STATUS_ZH[supervisor.status] ?? supervisor.status} · {supervisor.turnCount} 轮
                    </span>
                    <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                      {ROLE_DUTY_ZH[supervisor.agentRole]}
                    </span>
                  </div>

                  <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "10px" }}>
                    <span
                      className="tabular-nums"
                      title={`完整会话 UUID: ${supervisor.sessionId}`}
                      style={{ fontSize: "11.5px", color: "var(--text-secondary)" }}
                    >
                      会话 {formatCompactUuid(supervisor.sessionId)} · 契约 {supervisor.specRelPath}
                    </span>
                    <AsyncGlassButton
                      size="sm"
                      label="唤醒主控迭代"
                      loadingLabel="正在唤醒会话..."
                      successLabel="已完成增量轮次"
                      icon={<Sparkles size={13} />}
                      onClick={async () => {
                        await onWakeSession(supervisor.agentRole, wakePrompt);
                        setWakeNotice(
                          `已通过原会话 (${supervisor.sessionId.slice(0, 8)}…) 唤醒 ${ROLE_LABELS_ZH[supervisor.agentRole]}`
                        );
                      }}
                    />
                  </div>
                </div>
              )}

              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 235px), 1fr))",
                  gap: "12px",
                }}
              >
                {specialists.map((sess) => (
                  <div
                    key={sess.agentRole}
                    data-specialist-card="true"
                    data-agent-card={sess.agentRole}
                    className="glass-optical-card"
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      justifyContent: "space-between",
                      gap: "10px",
                      minWidth: 0,
                      padding: "13px 14px",
                    }}
                  >
                    <div style={{ display: "flex", flexDirection: "column", gap: "6px", minWidth: 0 }}>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px", minWidth: 0 }}>
                        <span style={{ display: "flex", alignItems: "center", gap: "6px", fontWeight: 700, fontSize: "13.5px", minWidth: 0 }}>
                          <Bot size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
                          <SafeText text={ROLE_LABELS_ZH[sess.agentRole]} />
                        </span>
                        <span
                          className={`glass-pill tabular-nums ${
                            sess.status === "DEGRADED"
                              ? "pill-risk-veto"
                              : sess.status === "RECOVERING"
                                ? "pill-risk-stoploss"
                                : "pill-risk-clear"
                          }`}
                          style={{ fontSize: "11px", minHeight: "22px", paddingInline: "8px" }}
                        >
                          {SESSION_STATUS_ZH[sess.status] ?? sess.status} · {sess.turnCount} 轮
                        </span>
                      </div>

                      <div style={{ fontSize: "11.5px", color: "var(--text-muted)", lineHeight: 1.45 }}>
                        {ROLE_DUTY_ZH[sess.agentRole]}
                      </div>
                    </div>

                    <div
                      className="tabular-nums"
                      style={{
                        display: "flex",
                        flexDirection: "column",
                        gap: "3px",
                        padding: "7px 9px",
                        borderRadius: "10px",
                        background: "rgba(148, 163, 184, 0.07)",
                        fontSize: "11.5px",
                        color: "var(--text-secondary)",
                        minWidth: 0,
                      }}
                    >
                      <div
                        title={`完整会话 UUID: ${sess.sessionId}`}
                        style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}
                      >
                        <span style={{ color: "var(--text-muted)", flexShrink: 0 }}>会话 ID</span>
                        <span>{formatCompactUuid(sess.sessionId)}</span>
                      </div>
                      <div style={{ display: "flex", justifyContent: "space-between", gap: "8px", minWidth: 0 }}>
                        <span style={{ color: "var(--text-muted)", flexShrink: 0 }}>规范契约</span>
                        <SafeText text={sess.specRelPath} showPopover />
                      </div>
                    </div>

                    <div style={{ display: "flex", justifyContent: "flex-end" }}>
                      <AsyncGlassButton
                        size="sm"
                        label="唤醒原会话迭代"
                        loadingLabel="正在唤醒会话..."
                        successLabel="已完成增量轮次"
                        icon={<Sparkles size={13} />}
                        onClick={async () => {
                          await onWakeSession(sess.agentRole, wakePrompt);
                          setWakeNotice(
                            `已通过原会话 (${sess.sessionId.slice(0, 8)}…) 唤醒 ${ROLE_LABELS_ZH[sess.agentRole]}`
                          );
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          );
        })()}
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
