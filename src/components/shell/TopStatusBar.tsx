import React, { useEffect, useState } from "react";
import { Clock, Moon, Radar, Sun, Wifi, ArrowUpDown } from "lucide-react";
import type {
  MarketPolarity,
  MarketSessionState,
  PortfolioOverviewDto,
  ThemeMode,
  UniverseFunnelSummaryDto,
} from "../../types/terminal";
import {
  computeNext30mRotationCountdown,
  formatCurrencyYuan,
  formatSignedPct,
} from "../../domain/uiInvariants";
import { AsyncGlassButton } from "../common/AsyncGlassButton";
import { SafeText } from "../common/SafeText";

export interface TopStatusBarProps {
  funnel: UniverseFunnelSummaryDto;
  portfolio: PortfolioOverviewDto;
  theme: ThemeMode;
  polarity: MarketPolarity;
  blurActive: boolean;
  onRunScan: () => Promise<void>;
  onToggleTheme: () => void;
  onTogglePolarity: () => void;
}

const SESSION_LABELS: Record<MarketSessionState, string> = {
  PRE_OPEN: "集合竞价 09:15-09:30",
  TRADING: "A股连续竞价交易中",
  LUNCH_BREAK: "午间休市 11:30-13:00",
  CLOSED: "A股闭市复盘态",
};

export const TopStatusBar: React.FC<TopStatusBarProps> = ({
  funnel,
  portfolio,
  theme,
  polarity,
  blurActive,
  onRunScan,
  onToggleTheme,
  onTogglePolarity,
}) => {
  const [nowMs, setNowMs] = useState<number>(() => Date.now());

  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  const rotation = computeNext30mRotationCountdown(nowMs);
  const effectiveSession = funnel.marketSession ?? rotation.session;
  const navPositive = portfolio.cumReturnPct >= 0;

  return (
    <header
      data-blur-slot={blurActive ? "active" : "optical"}
      className="glass-tier-1"
      style={{
        padding: "10px 16px",
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "10px 14px",
        minWidth: 0,
      }}
    >
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: "8px",
          minWidth: 0,
        }}
      >
        <span
          className={`glass-pill ${
            effectiveSession === "TRADING" ? "pill-risk-clear" : ""
          }`}
        >
          <span
            style={{
              width: "7px",
              height: "7px",
              borderRadius: "999px",
              background:
                effectiveSession === "TRADING"
                  ? "var(--accent)"
                  : "var(--risk-stoploss)",
              flexShrink: 0,
            }}
          />
          <span>{SESSION_LABELS[effectiveSession]}</span>
        </span>

        <span
          className="glass-pill tabular-nums"
          title={`下一轮动整半点窗口 (CST): ${rotation.nextSlotCstLabel}`}
        >
          <Clock size={13} style={{ color: "var(--accent)", flexShrink: 0 }} />
          <span>下次轮动 {rotation.formattedCountdown}</span>
        </span>

        <span className="glass-pill tabular-nums" style={{ maxWidth: "290px" }}>
          <Wifi size={13} style={{ color: "var(--chart-3)", flexShrink: 0 }} />
          <SafeText text={`${funnel.providerName} · ${funnel.scanRttMs}ms`} />
        </span>

        <span className="glass-pill tabular-nums">
          <span>100W净值</span>
          <strong>{formatCurrencyYuan(portfolio.totalEquityYuan)}</strong>
          <span className={navPositive ? "text-mkt-up" : "text-mkt-down"}>
            ({formatSignedPct(portfolio.cumReturnPct)})
          </span>
        </span>
      </div>

      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: "8px",
          minWidth: 0,
        }}
      >
        <AsyncGlassButton
          label="立即全A扫描"
          loadingLabel="扫描 5,576 只..."
          successLabel="全A扫描已刷新"
          icon={<Radar size={15} aria-hidden="true" />}
          onClick={onRunScan}
        />

        <button
          type="button"
          onClick={onTogglePolarity}
          className="glass-btn glass-btn-ghost glass-btn-sm"
          title="切换行情涨跌极性（止损琥珀金与排雷紫品红始终独立）"
        >
          <ArrowUpDown size={14} aria-hidden="true" />
          <span>{polarity === "cn-red-up" ? "红涨绿跌" : "绿涨红跌"}</span>
        </button>

        <button
          type="button"
          onClick={onToggleTheme}
          className="glass-btn glass-btn-ghost glass-btn-sm"
          aria-label={theme === "dark" ? "切换至浅色日光玻璃模式" : "切换至深色深曜石玻璃模式"}
        >
          {theme === "dark" ? <Sun size={14} /> : <Moon size={14} />}
          <span>{theme === "dark" ? "深色" : "浅色"}</span>
        </button>
      </div>
    </header>
  );
};
