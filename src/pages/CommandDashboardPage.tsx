import React, { useEffect, useState } from "react";
import { ArrowUpRight, Compass, Layers, ShieldCheck, TrendingUp } from "lucide-react";
import type {
  MarketPolarity,
  PortfolioOverviewDto,
  ScreenerCandidateDto,
  StockKlinePayloadDto,
  ThemeMode,
  UniverseFunnelSummaryDto,
} from "../types/terminal";
import { formatCurrencyYuan, formatSignedPct } from "../domain/uiInvariants";
import { SafeText } from "../components/common/SafeText";
import { EquityCurveChart } from "../components/charts/EquityCurveChart";
import { AllocationDonutChart } from "../components/charts/AllocationDonutChart";
import { QuantKlineTerminal } from "../components/charts/QuantKlineTerminal";
import { FactorColumnChart } from "../components/charts/FactorColumnChart";

export interface CommandDashboardPageProps {
  funnel: UniverseFunnelSummaryDto;
  portfolio: PortfolioOverviewDto;
  candidates: ScreenerCandidateDto[];
  klinePayload: StockKlinePayloadDto;
  theme: ThemeMode;
  polarity: MarketPolarity;
  isBlurActive: (slotId: string) => boolean;
  onPopoverOpenChange?: (open: boolean) => void;
  onChangeKline: (symbol: string, period: "1d" | "30m") => void;
  onOpenScreener: () => void;
}

function useAnimatedMetric(target: number): number {
  const [val, setVal] = useState<number>(target);

  useEffect(() => {
    if (
      typeof window === "undefined" ||
      (typeof window.matchMedia === "function" &&
        window.matchMedia("(prefers-reduced-motion: reduce)").matches)
    ) {
      setVal(target);
      return;
    }
    const start = target * 0.965;
    const duration = 420;
    const t0 = performance.now();
    let rafId = 0;

    const step = (now: number) => {
      const p = Math.max(0, Math.min(1, (now - t0) / duration));
      const eased = 1 - Math.pow(1 - p, 3);
      setVal(start + (target - start) * eased);
      if (p < 1) {
        rafId = requestAnimationFrame(step);
      }
    };
    rafId = requestAnimationFrame(step);
    return () => cancelAnimationFrame(rafId);
  }, [target]);

  return val;
}

const MiniSparkline: React.FC<{ values: number[]; strokeVar: string }> = ({
  values,
  strokeVar,
}) => {
  if (values.length < 2) return null;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = Math.max(0.0001, max - min);
  const pts = values
    .map((v, idx) => {
      const x = (idx / (values.length - 1)) * 108;
      const y = 26 - ((v - min) / span) * 22;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  return (
    <svg width="110" height="28" viewBox="0 0 110 28" aria-hidden="true" style={{ flexShrink: 0 }}>
      <polyline
        fill="none"
        stroke={strokeVar}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        points={pts}
      />
    </svg>
  );
};

export const CommandDashboardPage: React.FC<CommandDashboardPageProps> = ({
  funnel,
  portfolio,
  candidates,
  klinePayload,
  theme,
  polarity,
  isBlurActive,
  onPopoverOpenChange,
  onChangeKline,
  onOpenScreener,
}) => {
  const animatedEquity = useAnimatedMetric(portfolio.totalEquityYuan);
  const animatedDailyPnl = useAnimatedMetric(portfolio.dailyPnlYuan);

  const lockedPositionsCount = portfolio.positions.filter((p) => p.lockedT0Qty > 0).length;
  const sellablePositionsCount = portfolio.positions.filter((p) => p.sellableQty > 0).length;
  const navSpark = portfolio.equityCurve.slice(-14).map((e) => e.portfolioNav);
  const excessSpark = portfolio.equityCurve.slice(-14).map((e) => e.excessPct);
  const buyWatchlist = candidates.filter((c) => c.decision !== "REJECTED_BY_MINE");

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px", minWidth: 0 }}>
      {/* Row 1: 4 KPI Optical Cards inside Tier-2 Bento Header Shell */}
      <section
        aria-label="核心资产与全市场信号指标"
        data-blur-slot={isBlurActive("dash-kpi-shell") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-1"
      >
        <div className="bento-grid-12">
          {/* KPI 1: 100W NAV */}
          <div className="col-span-3 glass-optical-card glass-optical-card-interactive">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
              <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>100W 模拟盘总资产</span>
              <span
                className={`glass-pill tabular-nums ${
                  portfolio.cumReturnPct >= 0 ? "pill-mkt-up" : "pill-mkt-down"
                }`}
              >
                {formatSignedPct(portfolio.cumReturnPct)}
              </span>
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "flex-end",
                justifyContent: "space-between",
                gap: "8px",
                marginTop: "8px",
              }}
            >
              <strong className="lh-kpi tabular-nums" style={{ fontSize: "22px" }}>
                {formatCurrencyYuan(animatedEquity)}
              </strong>
              <MiniSparkline values={navSpark} strokeVar="var(--chart-1)" />
            </div>
            <div className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-muted)", marginTop: "6px" }}>
              可用现金 {formatCurrencyYuan(portfolio.cashYuan)}
            </div>
          </div>

          {/* KPI 2: Daily PnL & Max Drawdown */}
          <div className="col-span-3 glass-optical-card glass-optical-card-interactive">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
              <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>当日盈亏 &amp; 最大回撤</span>
              <TrendingUp
                size={15}
                style={{
                  color: portfolio.dailyPnlYuan >= 0 ? "var(--mkt-up)" : "var(--mkt-down)",
                  flexShrink: 0,
                }}
              />
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "flex-end",
                justifyContent: "space-between",
                gap: "8px",
                marginTop: "8px",
              }}
            >
              <strong
                className={`lh-kpi tabular-nums ${
                  portfolio.dailyPnlYuan >= 0 ? "text-mkt-up" : "text-mkt-down"
                }`}
                style={{ fontSize: "22px" }}
              >
                {portfolio.dailyPnlYuan >= 0 ? "+" : ""}
                {formatCurrencyYuan(animatedDailyPnl)}
              </strong>
              <MiniSparkline
                values={excessSpark}
                strokeVar={
                  (excessSpark[excessSpark.length - 1] ?? 0) >= 0
                    ? "var(--mkt-up)"
                    : "var(--mkt-down)"
                }
              />
            </div>
            <div className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-muted)", marginTop: "6px" }}>
              日涨幅 {formatSignedPct(portfolio.dailyReturnPct)} · 最大回撤{" "}
              <span className="text-stoploss">-{Math.abs(portfolio.maxDrawdownPct).toFixed(2)}%</span>
            </div>
          </div>

          {/* KPI 3: Position Slots <= 10 & T+1 Lock */}
          <div className="col-span-3 glass-optical-card glass-optical-card-interactive">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
              <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>持仓席位与 T+1 交收</span>
              <Layers size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
            </div>
            <div style={{ display: "flex", alignItems: "baseline", gap: "8px", marginTop: "8px" }}>
              <strong className="lh-kpi tabular-nums" style={{ fontSize: "22px" }}>
                {portfolio.openSlotsCount} / {portfolio.maxSlots} 只
              </strong>
              <span style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                上限 10 席硬约束
              </span>
            </div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: "6px", marginTop: "8px" }}>
              <span className="glass-pill pill-risk-clear tabular-nums">
                T+1可卖 {sellablePositionsCount} 只
              </span>
              <span className="glass-pill pill-risk-stoploss tabular-nums">
                今日锁仓 {lockedPositionsCount} 只
              </span>
            </div>
          </div>

          {/* KPI 4: 5000+ All-A Rotation Signal Hit */}
          <div className="col-span-3 glass-optical-card glass-optical-card-interactive">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
              <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>5000+ 全A轮动命中信号</span>
              <ShieldCheck size={15} style={{ color: "var(--chart-3)", flexShrink: 0 }} />
            </div>
            <div style={{ display: "flex", alignItems: "baseline", gap: "8px", marginTop: "8px" }}>
              <strong className="lh-kpi tabular-nums" style={{ fontSize: "22px" }}>
                {funnel.qualifiedBuyCount} 只优选
              </strong>
              <span className="tabular-nums" style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                / {funnel.totalActiveCount.toLocaleString()} 活跃池
              </span>
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "6px",
                marginTop: "7px",
              }}
            >
              <SafeText
                text={`<200亿(${funnel.under200YiCount}) · 两日>5%(${funnel.consecutiveTurnoverVolCount})`}
                className="tabular-nums"
              />
              <button
                type="button"
                onClick={onOpenScreener}
                className="glass-btn glass-btn-ghost glass-btn-sm"
                style={{ flexShrink: 0 }}
              >
                <Compass size={13} />
                <span>排雷台</span>
                <ArrowUpRight size={12} />
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* Row 2: 2/3 Equity Curve + 1/3 Allocation Donut */}
      <div className="bento-grid-12">
        <section
          data-blur-slot={isBlurActive("dash-equity-panel") ? "active" : "optical"}
          className="col-span-8 glass-tier-2 stagger-enter stagger-2"
        >
          <EquityCurveChart
            points={portfolio.equityCurve}
            theme={theme}
            polarity={polarity}
          />
        </section>

        <section
          data-blur-slot={isBlurActive("dash-donut-panel") ? "active" : "optical"}
          className="col-span-4 glass-tier-2 stagger-enter stagger-2"
        >
          <AllocationDonutChart
            sectorWeights={portfolio.sectorWeights}
            openSlotsCount={portfolio.openSlotsCount}
            maxSlots={portfolio.maxSlots}
            theme={theme}
            polarity={polarity}
          />
        </section>
      </div>

      {/* Row 3: 2/3 TradingView K-Line Terminal + 1/3 Turnover & Factor Z-Score Column */}
      <div className="bento-grid-12">
        <section
          data-blur-slot={isBlurActive("dash-kline-panel") ? "active" : "optical"}
          className="col-span-8 glass-tier-2 stagger-enter stagger-3"
        >
          <QuantKlineTerminal
            payload={klinePayload}
            watchlist={buyWatchlist}
            theme={theme}
            polarity={polarity}
            popoverBlurActive={isBlurActive("kline-rationale-popover")}
            onPopoverOpenChange={onPopoverOpenChange}
            onChangeSymbolPeriod={onChangeKline}
          />
        </section>

        <section
          data-blur-slot={isBlurActive("dash-factor-panel") ? "active" : "optical"}
          className="col-span-4 glass-tier-2 stagger-enter stagger-3"
        >
          <FactorColumnChart
            klinePayload={klinePayload}
            theme={theme}
            polarity={polarity}
          />
        </section>
      </div>
    </div>
  );
};
