import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ColorType,
  CrosshairMode,
  LineStyle,
  createChart,
  type IChartApi,
  type SeriesMarker,
  type Time,
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
import { normalizeKlineSeriesAndMarkers } from "./klineTimeAlign";
import { canRenderBrowserCharts, readChartTokenPalette } from "./useEChartsInstance";

export interface QuantKlineTerminalProps {
  payload: StockKlinePayloadDto;
  watchlist: ScreenerCandidateDto[];
  theme: ThemeMode;
  polarity: MarketPolarity;
  popoverBlurActive: boolean;
  loading?: boolean;
  onPopoverOpenChange?: (open: boolean) => void;
  onChangeSymbolPeriod: (symbol: string, period: "1d" | "30m") => void;
}

export const QuantKlineTerminal: React.FC<QuantKlineTerminalProps> = ({
  payload,
  watchlist,
  theme,
  polarity,
  popoverBlurActive,
  loading = false,
  onPopoverOpenChange,
  onChangeSymbolPeriod,
}) => {
  const paneWrapRef = useRef<HTMLDivElement | null>(null);
  const canvasHostRef = useRef<HTMLDivElement | null>(null);
  const [selectedMarker, setSelectedMarker] = useState<{
    marker: KlineSignalMarkerDto;
    x: number;
    y: number;
  } | null>(null);

  const aligned = useMemo(() => normalizeKlineSeriesAndMarkers(payload), [payload]);

  const markerByTimeKey = useMemo(() => {
    const map = new Map<string, KlineSignalMarkerDto>();
    for (const item of aligned.markers) {
      map.set(String(item.chartTime), item.marker);
      map.set(item.marker.barTime, item.marker);
    }
    return map;
  }, [aligned.markers]);

  useEffect(() => {
    setSelectedMarker(null);
  }, [payload.symbol, payload.period]);

  useEffect(() => {
    onPopoverOpenChange?.(selectedMarker !== null);
  }, [selectedMarker, onPopoverOpenChange]);

  useEffect(() => {
    if (!selectedMarker) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setSelectedMarker(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selectedMarker]);

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
    if (!host || !canRenderBrowserCharts() || aligned.bars.length === 0) return;

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
        aligned.bars.map((b) => ({
          time: b.chartTime,
          open: b.open,
          high: b.high,
          low: b.low,
          close: b.close,
        }))
      );

      // Isolate marker mounting so any unexpected series marker error never aborts stop-loss or MAs
      try {
        const markers: SeriesMarker<Time>[] = aligned.markers.map((item) => ({
          time: item.chartTime,
          position: "belowBar" as const,
          color: palette.mktUp,
          shape: "arrowUp" as const,
          text: `买点 ¥${item.marker.entryPriceYuan.toFixed(2)}`,
        }));
        candleSeries.setMarkers(markers);
      } catch {
        // Ignore isolated marker mismatch
      }

      // Decoupled Amber Dashed Stop-Loss Horizontal Price Line
      if (payload.activeStopLossYuan && payload.activeStopLossYuan > 0) {
        candleSeries.createPriceLine({
          price: payload.activeStopLossYuan,
          color: palette.riskStoploss,
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
        aligned.bars
          .filter((b) => typeof b.ma5 === "number")
          .map((b) => ({ time: b.chartTime, value: b.ma5 as number }))
      );

      const ma20Series = chart.addLineSeries({
        color: palette.chart5,
        lineWidth: 1,
        lineStyle: LineStyle.Dotted,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      ma20Series.setData(
        aligned.bars
          .filter((b) => typeof b.ma20 === "number")
          .map((b) => ({ time: b.chartTime, value: b.ma20 as number }))
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
        polarity === "cn-red-up"
          ? theme === "light" ? "rgba(220, 38, 38, 0.34)" : "rgba(255, 69, 58, 0.36)"
          : theme === "light" ? "rgba(22, 163, 74, 0.34)" : "rgba(48, 209, 88, 0.36)";
      const downVolAlpha =
        polarity === "cn-red-up"
          ? theme === "light" ? "rgba(22, 163, 74, 0.34)" : "rgba(48, 209, 88, 0.36)"
          : theme === "light" ? "rgba(220, 38, 38, 0.34)" : "rgba(255, 69, 58, 0.36)";
      volSeries.setData(
        aligned.bars.map((b) => ({
          time: b.chartTime,
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
          position: "relative",
          width: "100%",
          minHeight: "310px",
          borderRadius: "14px",
          overflow: "hidden",
          border: "1px solid var(--glass-rim-subtle)",
        }}
        aria-label="TradingView K线买点与止损水平线主图"
      >
        {(loading || aligned.bars.length === 0) && (
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: "12.5px",
              color: "var(--text-muted)",
              pointerEvents: "none",
            }}
          >
            {loading ? "正在加载实时 K 线与止损锚点..." : "当前周期暂无 K 线历史序列"}
          </div>
        )}
      </div>

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
              minWidth: 0,
            }}
          >
            <span style={{ display: "flex", alignItems: "center", gap: "6px", fontWeight: 700, fontSize: "13px", flex: "1 1 0%", minWidth: 0 }}>
              <ShieldCheck size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
              <SafeText text={`量化买点审计 · ${selectedMarker.marker.barTime}`} />
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
