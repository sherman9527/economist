import React, { useEffect, useState } from "react";
import { BellRing, CandlestickChart, Eye, ShoppingBag, X } from "lucide-react";
import type { RotationAlertEventDto, ScreenerCandidateDto } from "../../types/terminal";
import { formatSignedPct, normalizeStockDisplayLabel } from "../../domain/uiInvariants";
import { SafeText } from "../common/SafeText";
import { AsyncGlassButton } from "../common/AsyncGlassButton";

export interface RotationAlertToastProps {
  alert: RotationAlertEventDto | null;
  blurActive: boolean;
  onDismiss: () => void;
  onInspectKline: (candidate: ScreenerCandidateDto) => void;
  onAddWatchlist?: (candidate: ScreenerCandidateDto) => Promise<void>;
  onSimulatedBuy: (candidate: ScreenerCandidateDto) => Promise<void>;
}

export const RotationAlertToast: React.FC<RotationAlertToastProps> = ({
  alert,
  blurActive,
  onDismiss,
  onInspectKline,
  onAddWatchlist,
  onSimulatedBuy,
}) => {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [inlineError, setInlineError] = useState<string | null>(null);

  useEffect(() => {
    setSelectedIdx(0);
    setInlineError(null);
  }, [alert?.slotTimestampCst]);

  useEffect(() => {
    if (!alert || alert.topPicks.length === 0) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onDismiss();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [alert, onDismiss]);

  if (!alert || alert.topPicks.length === 0) {
    return null;
  }

  const safeIdx = Math.min(selectedIdx, alert.topPicks.length - 1);
  const lead = alert.topPicks[safeIdx];
  const upTone = lead.pctChg >= 0 ? "text-mkt-up" : "text-mkt-down";

  return (
    <aside
      role="region"
      aria-label="30分钟全A轮动买点提醒"
      data-blur-slot={blurActive ? "active" : "optical"}
      className="glass-tier-3 stagger-enter"
      style={{
        position: "fixed",
        top: "78px",
        right: "18px",
        width: "min(392px, calc(100vw - 24px))",
        padding: "15px 16px",
        zIndex: 70,
        display: "flex",
        flexDirection: "column",
        gap: "10px",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "8px",
          minWidth: 0,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "8px", flex: "1 1 0%", minWidth: 0 }}>
          <BellRing size={16} style={{ color: "var(--accent)", flexShrink: 0 }} />
          <SafeText
            text={`30m 轮动雷达命中 (${alert.qualifiedCount}) · ${alert.slotTimestampCst}`}
            as="strong"
          />
        </div>
        <button
          type="button"
          onClick={onDismiss}
          className="glass-btn glass-btn-ghost glass-btn-sm"
          aria-label="关闭30分钟轮动提醒"
          style={{ paddingInline: "7px", minHeight: "28px", flexShrink: 0 }}
        >
          <X size={14} />
        </button>
      </div>

      {alert.topPicks.length > 1 && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }} role="tablist" aria-label="轮动候选切换">
          {alert.topPicks.map((pick, idx) => (
            <button
              key={pick.symbol}
              type="button"
              role="tab"
              aria-selected={idx === safeIdx}
              onClick={() => {
                setSelectedIdx(idx);
                setInlineError(null);
              }}
              className={`glass-btn glass-btn-sm ${idx === safeIdx ? "" : "glass-btn-ghost"}`}
            >
              <span>{pick.nameZh}</span>
            </button>
          ))}
        </div>
      )}

      <div className="glass-optical-card" style={{ padding: "11px 12px" }}>
        <div
          style={{
            display: "flex",
            alignItems: "baseline",
            justifyContent: "space-between",
            gap: "8px",
            minWidth: 0,
          }}
        >
          <SafeText
            text={normalizeStockDisplayLabel(lead.symbol, lead.nameZh)}
            as="strong"
          />
          <span className={`tabular-nums ${upTone}`} style={{ fontWeight: 700, flexShrink: 0 }}>
            ¥{lead.latestPriceYuan.toFixed(2)} ({formatSignedPct(lead.pctChg)})
          </span>
        </div>

        <div
          className="tabular-nums"
          style={{
            display: "flex",
            flexWrap: "wrap",
            gap: "8px",
            marginTop: "6px",
            fontSize: "12px",
          }}
        >
          <span className="glass-pill pill-risk-stoploss">
            止损线 ¥{lead.suggestedStopLossYuan.toFixed(2)}
          </span>
          <span className="glass-pill">
            市值 {lead.totalMvYi.toFixed(1)}亿
          </span>
          <span className="glass-pill">
            两日换手 {lead.turnoverT1Pct.toFixed(1)}%→{lead.turnoverT0Pct.toFixed(1)}%
          </span>
        </div>

        <div style={{ marginTop: "8px", fontSize: "12.5px", color: "var(--text-secondary)" }}>
          <SafeText text={lead.rationaleZh} maxLines={2} showPopover />
        </div>
      </div>

      {inlineError && (
        <div role="alert" className="glass-pill pill-risk-veto" style={{ fontSize: "11.5px" }}>
          <span>{inlineError}</span>
        </div>
      )}

      <div style={{ display: "flex", flexWrap: "wrap", gap: "8px", justifyContent: "flex-end" }}>
        <button
          type="button"
          onClick={() => onInspectKline(lead)}
          className="glass-btn glass-btn-ghost glass-btn-sm"
        >
          <CandlestickChart size={14} aria-hidden="true" />
          <span>查看K线买点</span>
        </button>
        {onAddWatchlist && (
          <AsyncGlassButton
            size="sm"
            variant="ghost"
            label="加入自选观望"
            loadingLabel="记入自选..."
            successLabel="已在自选"
            icon={<Eye size={14} aria-hidden="true" />}
            onClick={() => onAddWatchlist(lead)}
          />
        )}
        <AsyncGlassButton
          size="sm"
          label="模拟买入"
          loadingLabel="委托入库..."
          successLabel="已记入模拟盘"
          icon={<ShoppingBag size={14} aria-hidden="true" />}
          onClick={async () => {
            setInlineError(null);
            try {
              await onSimulatedBuy(lead);
            } catch (err: unknown) {
              setInlineError(err instanceof Error ? err.message : "模拟买入被风控拦截");
              throw err;
            }
          }}
        />
      </div>
    </aside>
  );
};
