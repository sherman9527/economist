import React, { useState } from "react";
import type { MarketPolarity, StockKlinePayloadDto, ThemeMode } from "../../types/terminal";
import type { FactorChartMode } from "../../types/ui";
import { useEChartsOption } from "./useEChartsInstance";

export interface FactorColumnChartProps {
  klinePayload: StockKlinePayloadDto;
  theme: ThemeMode;
  polarity: MarketPolarity;
}

function shortFactorAxisTag(factorId: string, labelZh: string): string {
  const code = factorId.match(/^(F\d)/)?.[1];
  const clean = labelZh.replace(/因子$/, "");
  if (code) {
    return `${code} ${clean.length > 5 ? clean.slice(-4) : clean}`;
  }
  return clean;
}

export const FactorColumnChart: React.FC<FactorColumnChartProps> = ({
  klinePayload,
  theme,
  polarity,
}) => {
  const [mode, setMode] = useState<FactorChartMode>("turnover");

  const chartRef = useEChartsOption(
    (palette) => {
      const isTurnover = mode === "turnover";
      const categories = isTurnover
        ? klinePayload.recentTurnoversPct.map((t) => t.date)
        : klinePayload.factorBreakdown.map((f) => shortFactorAxisTag(f.factorId, f.labelZh));

      const values = isTurnover
        ? klinePayload.recentTurnoversPct.map((t) => ({
            value: t.turnoverPct,
            itemStyle: {
              color: t.turnoverPct >= 5.0 ? palette.chart4 : palette.chart6,
              borderRadius: [8, 8, 0, 0],
            },
          }))
        : klinePayload.factorBreakdown.map((f) => ({
            value: Number(f.zScore.toFixed(2)),
            itemStyle: {
              color: f.zScore >= 0 ? palette.chart1 : palette.mktDown,
              borderRadius: f.zScore >= 0 ? [8, 8, 0, 0] : [0, 0, 8, 8],
            },
          }));

      return {
        backgroundColor: "transparent",
        animationDuration: 320,
        tooltip: {
          trigger: "axis",
          backgroundColor: palette.tooltipBg,
          borderColor: palette.tooltipBorder,
          borderWidth: 1,
          textStyle: { color: palette.textPrimary, fontSize: 12 },
        },
        grid: {
          top: 24,
          left: 8,
          right: 12,
          bottom: 8,
          containLabel: true,
        },
        xAxis: {
          type: "category",
          data: categories,
          axisLine: { lineStyle: { color: palette.gridLine } },
          axisLabel: {
            color: palette.textMuted,
            fontSize: 11,
            hideOverlap: true,
            interval: 0,
          },
        },
        yAxis: {
          type: "value",
          splitLine: { lineStyle: { color: palette.gridLine, type: "dashed" } },
          axisLabel: {
            color: palette.textMuted,
            fontSize: 11,
            formatter: (v: number) => (isTurnover ? `${v}%` : v.toFixed(1)),
          },
        },
        series: [
          {
            name: isTurnover ? "日换手率(%)" : "截面Z-Score",
            type: "bar",
            barMaxWidth: 28,
            data: values,
            markLine: isTurnover
              ? {
                  symbol: "none",
                  lineStyle: { color: palette.riskStoploss, type: "dashed", width: 1.5 },
                  label: {
                    formatter: "5%点火线",
                    color: palette.riskStoploss,
                    fontSize: 10.5,
                  },
                  data: [{ yAxis: 5.0 }],
                }
              : undefined,
          },
        ],
      };
    },
    [klinePayload, mode],
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
            {klinePayload.nameZh} · 换手与因子共振
          </div>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            连续2日 &gt;5% 换手放量验证 &amp; 六维截面得分
          </div>
        </div>

        <div className="glass-segmented" role="group" aria-label="柱状图模式">
          <button
            type="button"
            className="glass-seg-btn"
            aria-pressed={mode === "turnover"}
            onClick={() => setMode("turnover")}
          >
            两日换手
          </button>
          <button
            type="button"
            className="glass-seg-btn"
            aria-pressed={mode === "zscore"}
            onClick={() => setMode("zscore")}
          >
            因子Z分
          </button>
        </div>
      </div>

      <div
        ref={chartRef}
        style={{ width: "100%", height: "288px", minWidth: 0 }}
        aria-label="连续换手放量与因子截面得分圆角柱状图"
      />
    </div>
  );
};
