import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ColorType,
  CrosshairMode,
  LineStyle,
  createChart,
  type IChartApi,
  type SeriesMarker,
  type Time,
  type UTCTimestamp,
} from "lightweight-charts";
import { Sparkles, X, ShieldCheck } from "lucide-react";
import type {
  KlineSignalMarkerDto,
  MarketPolarity,
  ScreenerCandidateDto,
  StockKlinePayloadDto,
  ThemeMode,
} from "../../types/terminal";
import { clampPopoverPosition, normalizeStockDisplayLabel } from "../../domain/uiInvariants";
import { PitfallAuditBadges } from "../common/PitfallAuditBadges";
import { SafeText } from "../common/SafeText";
import { canRenderBrowserCharts, readChartTokenPalette } from "./useEChartsInstance";

export interface QuantKlineTerminalProps {
  payload: StockKlinePayloadDto;
  watchlist: ScreenerCandidateDto[];
  theme: ThemeMode;
  polarity: MarketPolarity;
  popoverBlurActive: boolean;
  onChangeSymbolPeriod: (symbol: string, period: "1d" | "30m") => void;
}

function toChartTime(barTime: string): Time {
  if (barTime.includes(" ")) {
    const iso = `${barTime.replace(" ", "T")}:00Z`;
    const sec = Math.floor(Date.parse(iso) / 1000);
    return sec as UTCTimestamp;
  }
  return barTime as Time;
}

export const QuantKlineTerminal: React.FC<QuantKlineTerminalProps> = ({
  payload,
  watchlist,
  theme,
  polarity,
  popoverBlurActive,
  onChangeSymbolPeriod,
}) => {
  const paneWrapRef = useRef<HTMLDivElement | null>(null);
  const canvasHostRef = useRef<HTMLDivElement | null>(null);
  const [selectedMarker, setSelectedMarker] = useState<{
    marker: KlineSignalMarkerDto;
    x: number;
    y: number;
  } | null>(null);

  const markerByTimeKey = useMemo(() => {
    const map = new Map<string, KlineSignalMarkerDto>();
    for (const m of payload.buyMarkers) {
      map.set(String(toChartTime(m.barTime)), m);
      map.set(m.barTime, m);
    }
    return map;
  }, [payload.buyMarkers]);

  useEffect(() => {
    setSelectedMarker(null);
  }, [payload.symbol, payload.period]);

  const openMarkerPopover = (
    marker: KlineSignalMarkerDto,
    anchorX: number,
    anchorY: number
  ) => {
    const pane = paneWrapRef.current;
    const paneWidth = pane?.clientWidth || 680;
    const paneHeight = pane?.clientHeight || 360;
    const clamped = clampPopoverPosition({
      anchorX,
      anchorY,
      popoverWidth: 336,
      popoverHeight: 248,
      paneWidth,
      paneHeight,
      margin: 12,
    });
    setSelectedMarker({ marker, x: clamped.x, y: clamped.y });
  };

  useEffect(() => {
    const host = canvasHostRef.current;
    if (!host || !canRenderBrowserCharts()) return;

    const palette = readChartTokenPalette(theme, polarity);
    let chart: IChartApi | null = null;

    try {
      chart = createChart(host, {
        width: host.clientWidth > 0 ? host.clientWidth : 640,
        height: 310,
        layout: {
          background: { type: ColorType.Solid, color: "transparent" },
          textColor: palette.textMuted,
          fontFamily: "-apple-system, 'SF Pro Text', 'PingFang SC', system-ui, sans-serif",
          fontSize: 11,
        },
        grid: {
          vertLines: { color: palette.gridLine },
          horzLines: { color: palette.gridLine },
        },
        crosshair: { mode: CrosshairMode.Normal },
        rightPriceScale: {
          borderColor: palette.gridLine,
          scaleMargins: { top: 0.08, bottom: 0.26 },
        },
        timeScale: {
          borderColor: palette.gridLine,
          timeVisible: payload.period === "30m",
          secondsVisible: false,
        },
      });

      const candleSeries = chart.addCandlestickSeries({
        upColor: palette.mktUp,
        downColor: palette.mktDown,
        borderUpColor: palette.mktUp,
        borderDownColor: palette.mktDown,
        wickUpColor: palette.mktUp,
        wickDownColor: palette.mktDown,
      });

      candleSeries.setData(
        payload.bars.map((b) => ({
          time: toChartTime(b.time),
          open: b.open,
          high: b.high,
          low: b.low,
          close: b.close,
        }))
      );

      // Quant Agent Buy-Point Upward Markers
      const markers: SeriesMarker<Time>[] = payload.buyMarkers
        .map((m) => ({
          time: toChartTime(m.barTime),
          position: "belowBar" as const,
          color: palette.mktUp,
          shape: "arrowUp" as const,
          text: `买点 ¥${m.entryPriceYuan.toFixed(2)}`,
        }))
        .sort((a, b) => (String(a.time) < String(b.time) ? -1 : 1));
      candleSeries.setMarkers(markers);

      // Decoupled Amber Dashed Stop-Loss Horizontal Price Line (#FF9F0A)
      if (payload.activeStopLossYuan && payload.activeStopLossYuan > 0) {
        candleSeries.createPriceLine({
          price: payload.activeStopLossYuan,
          color: "#FF9F0A",
          lineWidth: 2,
          lineStyle: LineStyle.Dashed,
          axisLabelVisible: true,
          title: "止损线",
        });
      }

      // MA5 & MA20 Overlay Lines
      const ma5Series = chart.addLineSeries({
        color: palette.chart4,
        lineWidth: 1,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      ma5Series.setData(
        payload.bars
          .filter((b) => typeof b.ma5 === "number")
          .map((b) => ({ time: toChartTime(b.time), value: b.ma5 as number }))
      );

      const ma20Series = chart.addLineSeries({
        color: palette.chart5,
        lineWidth: 1,
        lineStyle: LineStyle.Dotted,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      ma20Series.setData(
        payload.bars
          .filter((b) => typeof b.ma20 === "number")
          .map((b) => ({ time: toChartTime(b.time), value: b.ma20 as number }))
      );

      // Linked Volume Histogram Sub-pane
      const volSeries = chart.addHistogramSeries({
        priceFormat: { type: "volume" },
        priceScaleId: "",
      });
      volSeries.priceScale().applyOptions({
        scaleMargins: { top: 0.79, bottom: 0 },
      });
      const upVolAlpha =
        polarity === "cn-red-up" ? "rgba(255, 69, 58, 0.36)" : "rgba(48, 209, 88, 0.36)";
      const downVolAlpha =
        polarity === "cn-red-up" ? "rgba(48, 209, 88, 0.36)" : "rgba(255, 69, 58, 0.36)";
      volSeries.setData(
        payload.bars.map((b) => ({
          time: toChartTime(b.time),
          value: b.volume,
          color: b.close >= b.open ? upVolAlpha : downVolAlpha,
        }))
      );

      chart.timeScale().fitContent();

      chart.subscribeClick((param) => {
        if (!param.time || !param.point) return;
        const hit = markerByTimeKey.get(String(param.time));
        if (hit) {
          openMarkerPopover(hit, param.point.x + 14, param.point.y - 20);
        }
      });
    } catch {
      // Headless jsdom fallback without WebGL/2D canvas context
    }

    const onResize = () => {
      if (chart && host.clientWidth > 0) {
        chart.applyOptions({ width: host.clientWidth });
      }
    };

    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      ro = new ResizeObserver(onResize);
      ro.observe(host);
    } else {
      window.addEventListener("resize", onResize);
    }

    return () => {
      ro?.disconnect();
      window.removeEventListener("resize", onResize);
      chart?.remove();
    };
  }, [payload, theme, polarity, markerByTimeKey]);

  const latestBar = payload.bars[payload.bars.length - 1];

  return (
    <div
      ref={paneWrapRef}
      style={{
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "10px",
        minWidth: 0,
      }}
    >
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "8px",
          minWidth: 0,
        }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "8px", minWidth: 0 }}>
          <strong style={{ fontSize: "15px" }}>
            {normalizeStockDisplayLabel(payload.symbol, payload.nameZh)}
          </strong>
          <span className="glass-pill tabular-nums">{payload.board}</span>
          {latestBar && (
            <span className="glass-pill tabular-nums">
              现价 ¥{latestBar.close.toFixed(2)} · 换手 {latestBar.turnoverPct.toFixed(2)}%
            </span>
          )}
          {payload.activeStopLossYuan && (
            <span className="glass-pill pill-risk-stoploss tabular-nums">
              琥珀止损线 ¥{payload.activeStopLossYuan.toFixed(2)}
            </span>
          )}
        </div>

        <div className="glass-segmented" role="group" aria-label="K线周期切换">
          {(["1d", "30m"] as const).map((p) => (
            <button
              key={p}
              type="button"
              className="glass-seg-btn tabular-nums"
              aria-pressed={payload.period === p}
              onClick={() => onChangeSymbolPeriod(payload.symbol, p)}
            >
              {p === "1d" ? "日K线 (1D)" : "30分钟轮动线"}
            </button>
          ))}
        </div>
      </div>

      {/* Quick Stock Switcher + Accessible Buy-Marker Anchors */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "6px",
          minWidth: 0,
        }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", gap: "5px", minWidth: 0 }}>
          {watchlist.slice(0, 6).map((cand) => {
            const active = cand.symbol === payload.symbol;
            return (
              <button
                key={cand.symbol}
                type="button"
                onClick={() => onChangeSymbolPeriod(cand.symbol, payload.period)}
                className={`glass-btn glass-btn-sm ${active ? "" : "glass-btn-ghost"}`}
              >
                <span>{cand.nameZh}</span>
              </button>
            );
          })}
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: "6px", minWidth: 0 }}>
          {payload.buyMarkers.map((m) => (
            <button
              key={m.recId}
              type="button"
              data-kline-buy-anchor={m.recId}
              onClick={() => openMarkerPopover(m, 180, 56)}
              className="glass-btn glass-btn-ghost glass-btn-sm tabular-nums"
              title="点击弹出 SQLite 存储的量化专家 Agent 推荐理由与五维排雷审计"
            >
              <Sparkles size={13} style={{ color: "var(--mkt-up)" }} />
              <span>买点 {m.barTime.slice(5)} @ ¥{m.entryPriceYuan.toFixed(2)}</span>
            </button>
          ))}
        </div>
      </div>

      <div
        ref={canvasHostRef}
        style={{
          width: "100%",
          minHeight: "310px",
          borderRadius: "14px",
          overflow: "hidden",
          border: "1px solid var(--glass-rim-subtle)",
        }}
        aria-label="TradingView K线买点与止损水平线主图"
      />

      {/* Edge-Clamped Tier-3 Glass Rationale Popover */}
      {selectedMarker && (
        <div
          role="dialog"
          aria-label="量化专家买点推荐理由明细"
          data-kline-rationale-popover=""
          data-blur-slot={popoverBlurActive ? "active" : "optical"}
          className="glass-tier-3 stagger-enter"
          style={{
            position: "absolute",
            left: `${selectedMarker.x}px`,
            top: `${selectedMarker.y}px`,
            width: "min(336px, calc(100% - 24px))",
            padding: "14px 15px",
            zIndex: 35,
            display: "flex",
            flexDirection: "column",
            gap: "8px",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: "8px",
            }}
          >
            <span style={{ display: "flex", alignItems: "center", gap: "6px", fontWeight: 700, fontSize: "13px" }}>
              <ShieldCheck size={15} style={{ color: "var(--accent)" }} />
              <span>量化买点审计 · {selectedMarker.marker.barTime}</span>
            </span>
            <button
              type="button"
              onClick={() => setSelectedMarker(null)}
              className="glass-btn glass-btn-ghost glass-btn-sm"
              style={{ paddingInline: "6px", minHeight: "26px" }}
              aria-label="关闭买点理由浮层"
            >
              <X size={13} />
            </button>
          </div>

          <div className="tabular-nums" style={{ display: "flex", flexWrap: "wrap", gap: "6px", fontSize: "12px" }}>
            <span className="glass-pill pill-mkt-up">
              建议买入 ¥{selectedMarker.marker.entryPriceYuan.toFixed(2)}
            </span>
            <span className="glass-pill pill-risk-stoploss">
              刚性止损 ¥{selectedMarker.marker.stopLossPriceYuan.toFixed(2)}
            </span>
            <span className="glass-pill">
              确信分 {selectedMarker.marker.convictionScore.toFixed(1)}
            </span>
          </div>

          <p className="lh-prose" style={{ margin: 0, fontSize: "12.5px", color: "var(--text-primary)" }}>
            {selectedMarker.marker.rationaleZh}
          </p>

          <PitfallAuditBadges pitfalls={selectedMarker.marker.pitfalls} compact />

          <div className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            <SafeText
              text={`SQLite Session: ${selectedMarker.marker.agentSessionId} (gemini-4-carbon · high)`}
            />
          </div>
        </div>
      )}
    </div>
  );
};
