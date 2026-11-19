import React from "react";
import { CandlestickChart, Eye, ShieldCheck, Trash2 } from "lucide-react";
import type { WatchlistItemDto, WatchlistOverviewDto } from "../../types/terminal";
import { formatSignedPct, normalizeStockDisplayLabel } from "../../domain/uiInvariants";
import { SafeText } from "../common/SafeText";
import { AsyncGlassButton } from "../common/AsyncGlassButton";

export interface WatchlistWinRatePanelProps {
  watchlist: WatchlistOverviewDto;
  blurActive: boolean;
  onJumpToKline: (symbol: string) => void;
  onRemoveWatchlist: (symbol: string) => Promise<void>;
}

function outcomeBadgeMeta(outcome: WatchlistItemDto["outcome"]): {
  label: string;
  cls: string;
} {
  switch (outcome) {
    case "HIT_TARGET":
      return { label: "触达目标止盈", cls: "pill-risk-clear" };
    case "SETTLED_5D_WIN":
      return { label: "5日兑现净盈", cls: "pill-risk-clear" };
    case "HIT_STOP":
      return { label: "触及止损离场", cls: "pill-risk-veto" };
    case "SETTLED_5D_LOSS":
      return { label: "5日届满净亏", cls: "pill-risk-veto" };
    default:
      return { label: "T+1跟踪观察中", cls: "pill-risk-stoploss" };
  }
}

export const WatchlistWinRatePanel: React.FC<WatchlistWinRatePanelProps> = ({
  watchlist,
  blurActive,
  onJumpToKline,
  onRemoveWatchlist,
}) => {
  const { summary, items } = watchlist;

  const kpiCards = [
    {
      label: `样本外已结算净胜率 (${summary.winCount}/${summary.settledCount})`,
      value: `${summary.settledWinRatePct.toFixed(1)}%`,
      sub: `Wilson 95%置信 [${summary.wilsonLowPct.toFixed(1)}%, ${summary.wilsonHighPct.toFixed(1)}%]`,
    },
    {
      label: `扣除 ${summary.roundTripCostBps}bps 双边摩擦盈亏比`,
      value: `${summary.profitFactor.toFixed(2)}x`,
      sub: `单笔数学期望 ${formatSignedPct(summary.expectancyPct)}`,
    },
    {
      label: "高确信度 (>=68分) 分层胜率",
      value: `${summary.highConvictionWinRatePct.toFixed(1)}%`,
      sub: `常规组对照 ${summary.baselineConvictionWinRatePct.toFixed(1)}%`,
    },
    {
      label: "在途跟踪浮盈占比",
      value: `${summary.livePositiveRatePct.toFixed(1)}%`,
      sub: `跟踪中 ${summary.activeTrackingCount} 只 · 止盈 ${summary.targetHitCount} / 止损 ${summary.stopHitCount}`,
    },
  ];

  return (
    <section
      aria-label="观望自选样本外胜率检验台"
      data-blur-slot={blurActive ? "active" : "optical"}
      className="glass-tier-2 stagger-enter stagger-2"
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
            观望自选样本外胜率检验台 ({summary.totalCount} 只)
          </strong>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            严格遵循 A 股 T+1 次日解锁与 {summary.roundTripCostBps}bps 双边佣金印花滑点扣减，杜绝虚高胜率幻觉
          </div>
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
          <span className="glass-pill pill-risk-clear tabular-nums">
            <ShieldCheck size={12} /> 5日标准评估窗口
          </span>
          {summary.lowSampleWarning && (
            <span className="glass-pill pill-risk-stoploss tabular-nums">
              已结算样本 &lt;20 笔 · 参考置信下界 {summary.wilsonLowPct.toFixed(1)}%
            </span>
          )}
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(195px, 1fr))",
          gap: "10px",
        }}
      >
        {kpiCards.map((c) => (
          <div key={c.label} className="glass-optical-card" style={{ padding: "11px 13px" }}>
            <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>{c.label}</div>
            <strong
              className="lh-kpi tabular-nums"
              style={{ fontSize: "20px", display: "block", marginTop: "4px" }}
            >
              {c.value}
            </strong>
            <div className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-secondary)", marginTop: "3px" }}>
              {c.sub}
            </div>
          </div>
        ))}
      </div>

      {items.length === 0 ? (
        <div
          className="glass-optical-card"
          style={{ padding: "18px", textAlign: "center", color: "var(--text-muted)", fontSize: "13px" }}
        >
          <Eye size={18} style={{ marginBottom: "6px", opacity: 0.7 }} />
          <div>当前自选观望池为空，可在「全A轮动与五维排雷」页面将暂不建仓标的加入自选持续验证策略胜率。</div>
        </div>
      ) : (
        <>
          <div className="glass-table-wrap responsive-table-only">
            <table className="glass-table">
              <thead>
                <tr>
                  <th style={{ width: "20%" }}>标的 / 纳入版本</th>
                  <th style={{ width: "16%" }}>纳入价 / 现价</th>
                  <th style={{ width: "16%" }}>止损 / 目标锚点</th>
                  <th style={{ width: "16%" }}>净收益 &amp; MFE/MAE</th>
                  <th style={{ width: "16%" }}>T+1检验状态</th>
                  <th style={{ width: "16%" }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {items.map((item) => {
                  const meta = outcomeBadgeMeta(item.outcome);
                  const posTone = item.netReturnPct >= 0 ? "text-mkt-up" : "text-mkt-down";
                  return (
                    <tr key={item.symbol}>
                      <td>
                        <SafeText text={item.nameZh} as="strong" />
                        <div className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                          {item.symbol} · {item.versionIdAtEntry} · 得分 {item.convictionScore.toFixed(1)}
                        </div>
                      </td>
                      <td className="tabular-nums">
                        <div>基准 ¥{item.entryPriceYuan.toFixed(2)}</div>
                        <div style={{ fontSize: "11.5px", color: "var(--text-secondary)" }}>
                          最新 ¥{(item.settledPriceYuan ?? item.latestPriceYuan).toFixed(2)} ({item.holdingTradingDays}D)
                        </div>
                      </td>
                      <td className="tabular-nums" style={{ fontSize: "12px" }}>
                        <div className="text-stoploss">止损 ¥{item.stopLossPriceYuan.toFixed(2)}</div>
                        <div className="text-risk-clear">目标 ¥{item.targetPriceYuan.toFixed(2)}</div>
                      </td>
                      <td className="tabular-nums">
                        <div className={posTone} style={{ fontWeight: 700 }}>
                          {formatSignedPct(item.netReturnPct)}
                        </div>
                        <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                          冲高 {formatSignedPct(item.mfePct)} / 回撤 {formatSignedPct(item.maePct)}
                        </div>
                      </td>
                      <td>
                        <span className={`glass-pill ${meta.cls}`} style={{ fontSize: "11px" }}>
                          {meta.label}
                        </span>
                        <div className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "3px" }}>
                          纳入 {item.addedTradeDate}
                        </div>
                      </td>
                      <td>
                        <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
                          <button
                            type="button"
                            onClick={() => onJumpToKline(item.symbol)}
                            className="glass-btn glass-btn-ghost glass-btn-sm"
                          >
                            <CandlestickChart size={13} />
                            <span>K线复盘</span>
                          </button>
                          <AsyncGlassButton
                            size="sm"
                            variant="ghost"
                            icon={<Trash2 size={12} />}
                            label="移出"
                            loadingLabel="移除中..."
                            successLabel="已移出"
                            onClick={() => onRemoveWatchlist(item.symbol)}
                          />
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <div className="responsive-cards-only">
            {items.map((item) => {
              const meta = outcomeBadgeMeta(item.outcome);
              return (
                <div
                  key={item.symbol}
                  className="glass-optical-card"
                  style={{ display: "flex", flexDirection: "column", gap: "6px" }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}>
                    <SafeText text={normalizeStockDisplayLabel(item.symbol, item.nameZh)} as="strong" />
                    <span className={`glass-pill ${meta.cls}`}>{meta.label}</span>
                  </div>
                  <div className="tabular-nums" style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                    版本 {item.versionIdAtEntry} · 基准 ¥{item.entryPriceYuan.toFixed(2)} · 净收益{" "}
                    {formatSignedPct(item.netReturnPct)}
                  </div>
                  <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                    <button
                      type="button"
                      onClick={() => onJumpToKline(item.symbol)}
                      className="glass-btn glass-btn-ghost glass-btn-sm"
                    >
                      K线复盘
                    </button>
                    <AsyncGlassButton
                      size="sm"
                      variant="ghost"
                      label="移出自选"
                      onClick={() => onRemoveWatchlist(item.symbol)}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </>
      )}
    </section>
  );
};
