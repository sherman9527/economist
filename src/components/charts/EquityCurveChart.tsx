import React, { useMemo, useState } from "react";
import * as echarts from "echarts";
import type { EquityPointDto, MarketPolarity, ThemeMode } from "../../types/terminal";
import type { EquityRangeKey } from "../../types/ui";
import { formatSignedPct } from "../../domain/uiInvariants";
import { useEChartsOption } from "./useEChartsInstance";

export interface EquityCurveChartProps {
  points: EquityPointDto[];
  theme: ThemeMode;
  polarity: MarketPolarity;
}

const RANGE_OPTIONS: EquityRangeKey[] = ["1D", "1W", "1M", "1Y"];

export const EquityCurveChart: React.FC<EquityCurveChartProps> = ({
  points,
  theme,
  polarity,
}) => {
  const [range, setRange] = useState<EquityRangeKey>("1M");

  const visiblePoints = useMemo(() => {
    if (points.length === 0) return [];
    if (range === "1D") return points.slice(-4);
    if (range === "1W") return points.slice(-8);
    if (range === "1M") return points.slice(-22);
    return points;
  }, [points, range]);

  const latestExcess =
    visiblePoints.length > 0 ? visiblePoints[visiblePoints.length - 1].excessPct : 0;

  const chartRef = useEChartsOption(
    (palette) => ({
      backgroundColor: "transparent",
      animationDuration: 360,
      tooltip: {
        trigger: "axis",
        backgroundColor: palette.tooltipBg,
        borderColor: palette.tooltipBorder,
        borderWidth: 1,
        textStyle: { color: palette.textPrimary, fontSize: 12 },
      },
      legend: {
        top: 0,
        right: 0,
        itemWidth: 12,
        itemHeight: 6,
        textStyle: { color: palette.textSecondary, fontSize: 11.5 },
      },
      grid: {
        top: 34,
        left: 8,
        right: 12,
        bottom: 8,
        containLabel: true,
      },
      xAxis: {
        type: "category",
        boundaryGap: false,
        data: visiblePoints.map((p) => p.date.slice(5)),
        axisLine: { lineStyle: { color: palette.gridLine } },
        axisLabel: {
          color: palette.textMuted,
          fontSize: 11,
          hideOverlap: true,
        },
      },
      yAxis: {
        type: "value",
        scale: true,
        splitLine: { lineStyle: { color: palette.gridLine, type: "dashed" } },
        axisLabel: {
          color: palette.textMuted,
          fontSize: 11,
          formatter: (val: number) => val.toFixed(2),
        },
      },
      series: [
        {
          name: "100W模拟盘 (扣费后)",
          type: "line",
          smooth: 0.32,
          showSymbol: false,
          lineStyle: { width: 2.6, color: palette.chart1 },
          itemStyle: { color: palette.chart1 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: "rgba(56, 189, 248, 0.34)" },
              { offset: 1, color: "rgba(56, 189, 248, 0.0)" },
            ]),
          },
          data: visiblePoints.map((p) => p.portfolioNav),
        },
        {
          name: "中证1000",
          type: "line",
          smooth: 0.3,
          showSymbol: false,
          lineStyle: { width: 1.7, color: palette.chart2 },
          itemStyle: { color: palette.chart2 },
          data: visiblePoints.map((p) => p.csi1000Nav),
        },
        {
          name: "国证2000",
          type: "line",
          smooth: 0.3,
          showSymbol: false,
          lineStyle: { width: 1.7, type: "dashed", color: palette.chart3 },
          itemStyle: { color: palette.chart3 },
          data: visiblePoints.map((p) => p.cni2000Nav),
        },
      ],
    }),
    [visiblePoints],
    theme,
    polarity
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "10px", minWidth: 0 }}>
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
        <div className="min-w-zero">
          <div style={{ fontWeight: 700, fontSize: "14.5px" }}>
            累计净值曲线 vs 小盘宽基基准
          </div>
          <div className="tabular-nums" style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            区间相对中证1000超额{" "}
            <strong className={latestExcess >= 0 ? "text-mkt-up" : "text-mkt-down"}>
              {formatSignedPct(latestExcess)}
            </strong>{" "}
            <span className="glass-pill" style={{ minHeight: "20px", paddingInline: "7px", fontSize: "10.5px" }}>
              [backtest] 含佣金/印花税/滑点
            </span>
          </div>
        </div>

        <div className="glass-segmented" role="group" aria-label="净值时间跨度">
          {RANGE_OPTIONS.map((key) => (
            <button
              key={key}
              type="button"
              className="glass-seg-btn tabular-nums"
              aria-pressed={range === key}
              onClick={() => setRange(key)}
            >
              {key}
            </button>
          ))}
        </div>
      </div>

      <div
        ref={chartRef}
        style={{ width: "100%", height: "252px", minWidth: 0 }}
        aria-label="组合累计净值对比折线图"
      />
    </div>
  );
};
