import React, { useEffect, useState } from "react";
import {
  ArrowUpRight,
  BookOpen,
  CheckCheck,
  CheckCircle2,
  Globe2,
  ShieldAlert,
  TrendingUp,
} from "lucide-react";
import type { EconomistModuleDto, EconomistReportItemDto } from "../types/terminal";
import type { EconomistSubPivot } from "../types/ui";
import { SafeText } from "../components/common/SafeText";
import { AsyncGlassButton } from "../components/common/AsyncGlassButton";

export interface EconomistNewsPageProps {
  economist: EconomistModuleDto;
  isBlurActive: (slotId: string) => boolean;
  onAckReport: (reportId: string, acked: boolean) => Promise<void>;
  onBatchAckUnread: (reportIds: string[]) => Promise<void>;
  onLoadReportHtml: (relPath: string) => Promise<string>;
  onJumpToKline: (symbol: string) => void;
}

export const EconomistNewsPage: React.FC<EconomistNewsPageProps> = ({
  economist,
  isBlurActive,
  onAckReport,
  onBatchAckUnread,
  onLoadReportHtml,
  onJumpToKline,
}) => {
  const [subPivot, setSubPivot] = useState<EconomistSubPivot>("chains");
  const [selectedReportId, setSelectedReportId] = useState<string>(
    economist.reports[0]?.reportId ?? ""
  );
  const [embedHtml, setEmbedHtml] = useState<string>("");
  const [loadingHtml, setLoadingHtml] = useState<boolean>(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const activeReport: EconomistReportItemDto | undefined =
    economist.reports.find((r) => r.reportId === selectedReportId) ??
    economist.reports[0];

  useEffect(() => {
    if (subPivot !== "reader" || !activeReport) return;
    let cancelled = false;
    setLoadingHtml(true);
    setLoadError(null);
    void onLoadReportHtml(activeReport.relPath)
      .then((raw) => {
        if (!cancelled) {
          setEmbedHtml(raw);
          setLoadingHtml(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setLoadError(err instanceof Error ? err.message : "读取离线研报 HTML 失败");
          setLoadingHtml(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [subPivot, activeReport, onLoadReportHtml]);

  const unreadIds = economist.reports.filter((r) => !r.acked).map((r) => r.reportId);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px", minWidth: 0 }}>
      {/* Header Strip & Sub-Pivot Switcher */}
      <section
        aria-label="经济学人跨市场投研导航"
        data-blur-slot={isBlurActive("econ-header") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-1"
        style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "12px" }}
      >
        <div className="min-w-zero">
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <Globe2 size={17} style={{ color: "var(--accent)", flexShrink: 0 }} />
            <strong style={{ fontSize: "15.5px" }}>
              《经济学人》全球政经深度研报与中美跨市场映射中枢
            </strong>
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: "6px", marginTop: "6px" }}>
            {economist.bearishRiskSectorsZh.map((sec) => (
              <span key={sec} className="glass-pill pill-risk-veto">
                <ShieldAlert size={11} />
                <span>空头规避：{sec}</span>
              </span>
            ))}
          </div>
        </div>

        <div className="glass-segmented" role="tablist" aria-label="经济学人子视图">
          {(
            [
              ["chains", "四阶跨市场传导"],
              ["ack-queue", `未读研报 ACK (${economist.unackedCount})`],
              ["hit-rate", "华尔街胜率回溯"],
              ["reader", "原版深度研报阅读器"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={subPivot === id}
              className="glass-seg-btn"
              onClick={() => setSubPivot(id)}
            >
              {label}
            </button>
          ))}
        </div>
      </section>

      {/* Sub-Pivot 1: 4-Stage Cross-Market Transmission & Sentiment Gauges */}
      {subPivot === "chains" && activeReport && (
        <section
          aria-label="四阶跨市场传导与情绪仪表"
          data-blur-slot={isBlurActive("econ-body") ? "active" : "optical"}
          className="glass-tier-2 stagger-enter stagger-2"
          style={{ display: "flex", flexDirection: "column", gap: "14px" }}
        >
          <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
            <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
              {economist.reports.map((rep) => (
                <button
                  key={rep.reportId}
                  type="button"
                  onClick={() => setSelectedReportId(rep.reportId)}
                  className={`glass-btn glass-btn-sm ${
                    rep.reportId === activeReport.reportId ? "" : "glass-btn-ghost"
                  }`}
                >
                  <span>{rep.date}</span>
                  {!rep.acked && (
                    <span className="glass-pill pill-risk-stoploss" style={{ minHeight: "18px", paddingInline: "5px", fontSize: "10px" }}>
                      未阅
                    </span>
                  )}
                </button>
              ))}
            </div>

            <div className="tabular-nums" style={{ display: "flex", flexWrap: "wrap", gap: "8px" }}>
              <span
                className={`glass-pill ${
                  activeReport.usScore >= 55
                    ? "pill-mkt-up"
                    : activeReport.usScore < 45
                      ? "pill-mkt-down"
                      : "pill-risk-stoploss"
                }`}
              >
                美股多空情绪 {activeReport.usScore} / 100
              </span>
              <span
                className={`glass-pill ${
                  activeReport.cnScore >= 55
                    ? "pill-risk-clear"
                    : activeReport.cnScore < 45
                      ? "pill-risk-veto"
                      : "pill-risk-stoploss"
                }`}
              >
                A股小盘景气映射 {activeReport.cnScore} / 100
              </span>
              <button
                type="button"
                onClick={() => setSubPivot("reader")}
                className="glass-btn glass-btn-ghost glass-btn-sm"
              >
                <BookOpen size={13} />
                <span>阅读原版研报</span>
              </button>
            </div>
          </div>

          <div className="glass-optical-card lh-prose" style={{ fontSize: "13.5px" }}>
            <strong>{activeReport.titleZh}</strong>
            <p style={{ margin: "6px 0 0", color: "var(--text-secondary)" }}>
              {activeReport.executiveSummaryZh}
            </p>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {activeReport.chains.map((chain) => (
              <article
                key={chain.chainTitle}
                className="glass-optical-card"
                style={{ display: "flex", flexDirection: "column", gap: "10px" }}
              >
                <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
                  <strong style={{ fontSize: "14.5px", color: "var(--accent)" }}>
                    {chain.chainTitle}
                  </strong>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
                    {chain.relatedCnSymbols.map((sym) => (
                      <button
                        key={sym}
                        type="button"
                        onClick={() => onJumpToKline(sym)}
                        className="glass-btn glass-btn-sm tabular-nums"
                        title="点击直跳该A股标的实时买点与止损K线主图"
                      >
                        <span>{sym}</span>
                        <ArrowUpRight size={13} />
                      </button>
                    ))}
                  </div>
                </div>

                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fit, minmax(215px, 1fr))",
                    gap: "10px",
                  }}
                >
                  <div className="lh-prose" style={{ fontSize: "12.5px" }}>
                    <div style={{ color: "var(--text-muted)", fontSize: "11.5px" }}>① 经济学人原文触发</div>
                    <div>{chain.economistTrigger}</div>
                  </div>
                  <div className="lh-prose" style={{ fontSize: "12.5px" }}>
                    <div style={{ color: "var(--text-muted)", fontSize: "11.5px" }}>② 美股板块映射</div>
                    <div>{chain.usMapping}</div>
                  </div>
                  <div className="lh-prose" style={{ fontSize: "12.5px" }}>
                    <div style={{ color: "var(--text-muted)", fontSize: "11.5px" }}>③ A股&lt;200亿换手轮动</div>
                    <div>{chain.cnMapping}</div>
                  </div>
                  <div className="lh-prose" style={{ fontSize: "12.5px" }}>
                    <div style={{ color: "var(--text-muted)", fontSize: "11.5px" }}>④ 中美定价温差洞察</div>
                    <div>{chain.divergenceInsight}</div>
                  </div>
                </div>
              </article>
            ))}
          </div>
        </section>
      )}

      {/* Sub-Pivot 2: Unread ACK Queue */}
      {subPivot === "ack-queue" && (
        <section
          aria-label="未读研报ACK持久化队列"
          data-blur-slot={isBlurActive("econ-body") ? "active" : "optical"}
          className="glass-tier-2 stagger-enter stagger-2"
          style={{ display: "flex", flexDirection: "column", gap: "12px" }}
        >
          <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
            <div>
              <strong style={{ fontSize: "15px" }}>
                研报已阅确认队列（双写 SQLite + `./data/ack_state.json`）
              </strong>
              <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                当前待阅 {economist.unackedCount} 篇 · 支持单篇确认或一键批量归档
              </div>
            </div>

            <AsyncGlassButton
              size="sm"
              disabled={unreadIds.length === 0}
              label={`批量已阅全部 (${unreadIds.length})`}
              loadingLabel="写入ACK状态..."
              successLabel="已全部确认"
              icon={<CheckCheck size={14} />}
              onClick={() => onBatchAckUnread(unreadIds)}
            />
          </div>

          {economist.reports.map((rep) => (
            <div
              key={rep.reportId}
              className="glass-optical-card"
              style={{
                display: "flex",
                flexWrap: "wrap",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "10px",
              }}
            >
              <div className="min-w-zero" style={{ flex: "1 1 320px" }}>
                <SafeText text={rep.titleZh} as="strong" />
                <div className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-muted)", marginTop: "3px" }}>
                  相对路径 `{rep.relPath}` · 核心精读 {rep.coreCount}/{rep.articleCount} 篇
                </div>
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <button
                  type="button"
                  onClick={() => {
                    setSelectedReportId(rep.reportId);
                    setSubPivot("reader");
                  }}
                  className="glass-btn glass-btn-ghost glass-btn-sm"
                >
                  打开全文
                </button>
                <AsyncGlassButton
                  size="sm"
                  variant={rep.acked ? "ghost" : "primary"}
                  label={rep.acked ? "已阅 ACK (点击撤销)" : "标记已阅 ACK"}
                  icon={<CheckCircle2 size={14} />}
                  onClick={() => onAckReport(rep.reportId, !rep.acked)}
                />
              </div>
            </div>
          ))}
        </section>
      )}

      {/* Sub-Pivot 3: Wall Street Multi-Horizon Hit-Rate Backtest */}
      {subPivot === "hit-rate" && (
        <section
          aria-label="华尔街跨市场多空胜率回溯"
          data-blur-slot={isBlurActive("econ-body") ? "active" : "optical"}
          className="glass-tier-2 stagger-enter stagger-2"
          style={{ display: "flex", flexDirection: "column", gap: "14px" }}
        >
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "12px" }}>
            <div className="glass-optical-card">
              <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>跨市场综合映射胜率 [OOS]</div>
              <strong className="lh-kpi tabular-nums text-mkt-up" style={{ fontSize: "26px" }}>
                {economist.backtestHitRatePct.toFixed(1)}%
              </strong>
            </div>
            <div className="glass-optical-card">
              <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>美股行业ETF 5D方向命中率</div>
              <strong className="lh-kpi tabular-nums" style={{ fontSize: "26px" }}>
                {economist.usHitRatePct.toFixed(1)}%
              </strong>
            </div>
            <div className="glass-optical-card">
              <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>A股小盘换手共振命中率</div>
              <strong className="lh-kpi tabular-nums" style={{ fontSize: "26px" }}>
                {economist.cnHitRatePct.toFixed(1)}%
              </strong>
            </div>
          </div>

          <div className="glass-optical-card lh-prose" style={{ fontSize: "13px" }}>
            <TrendingUp size={15} style={{ color: "var(--accent)", verticalAlign: "middle", marginRight: "6px" }} />
            回溯口径说明：仅当《经济学人》宏观行业多头评分 &ge; 68 且对应 A 股 &lt;200亿标的同步触发连续2日换手率 &gt;5% 与五维排雷全清时才计入右侧共振信号。
          </div>
        </section>
      )}

      {/* Sub-Pivot 4: Sandboxed Portable HTML Reader (?embed=1 via srcDoc) */}
      {subPivot === "reader" && activeReport && (
        <section
          aria-label="便携内嵌原版深度研报阅读器"
          data-blur-slot={isBlurActive("econ-body") ? "active" : "optical"}
          className="glass-tier-2 stagger-enter stagger-2"
          style={{ display: "flex", flexDirection: "column", gap: "10px" }}
        >
          <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
            <div className="tabular-nums" style={{ fontSize: "12.5px", color: "var(--text-secondary)" }}>
              零外部HTTP服务便携渲染：`{activeReport.relPath}?embed=1`
            </div>
            <div style={{ display: "flex", gap: "6px" }}>
              {economist.reports.map((r) => (
                <button
                  key={r.reportId}
                  type="button"
                  onClick={() => setSelectedReportId(r.reportId)}
                  className={`glass-btn glass-btn-sm ${
                    r.reportId === activeReport.reportId ? "" : "glass-btn-ghost"
                  }`}
                >
                  {r.date}
                </button>
              ))}
            </div>
          </div>

          {loadError && (
            <div role="alert" className="glass-pill pill-risk-veto">
              <span>研报加载告警：{loadError}</span>
            </div>
          )}
          {loadingHtml && !embedHtml && (
            <div className="glass-optical-card" style={{ fontSize: "12.5px", color: "var(--text-muted)" }}>
              正在加载便携离线研报与内联 ECharts 图表...
            </div>
          )}

          <iframe
            title={activeReport.titleZh}
            sandbox="allow-scripts"
            srcDoc={embedHtml}
            style={{
              width: "100%",
              minHeight: "460px",
              border: "1px solid var(--glass-rim-subtle)",
              borderRadius: "16px",
              background: "var(--bg-elevated-solid)",
            }}
          />
        </section>
      )}
    </div>
  );
};
