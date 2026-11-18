import React from "react";
import type { MarketPolarity, ThemeMode } from "../../types/terminal";
import { SafeText } from "../common/SafeText";
import { useEChartsOption } from "./useEChartsInstance";

export interface AllocationDonutChartProps {
  sectorWeights: { nameZh: string; weightPct: number; valueYuan: number }[];
  openSlotsCount: number;
  maxSlots: number;
  theme: ThemeMode;
  polarity: MarketPolarity;
}

export const AllocationDonutChart: React.FC<AllocationDonutChartProps> = ({
  sectorWeights,
  openSlotsCount,
  maxSlots,
  theme,
  polarity,
}) => {
  const investedPct = sectorWeights
    .filter((s) => !s.nameZh.includes("现金"))
    .reduce((acc, s) => acc + s.weightPct, 0);

  const chartRef = useEChartsOption(
    (palette) => {
      const colors = [
        palette.chart1,
        palette.chart2,
        palette.chart3,
        palette.chart4,
        palette.chart6,
        palette.chart5,
      ];
      return {
        backgroundColor: "transparent",
        tooltip: {
          trigger: "item",
          backgroundColor: palette.tooltipBg,
          borderColor: palette.tooltipBorder,
          borderWidth: 1,
          textStyle: { color: palette.textPrimary, fontSize: 12 },
          formatter: "{b}: {c}%",
        },
        color: colors,
        series: [
          {
            name: "持仓与行业分布",
            type: "pie",
            radius: ["70%", "87%"],
            center: ["50%", "50%"],
            avoidLabelOverlap: true,
            itemStyle: {
              borderRadius: 6,
              borderColor: "transparent",
              borderWidth: 2,
            },
            label: { show: false },
            labelLine: { show: false },
            data: sectorWeights.map((item) => ({
              name: item.nameZh,
              value: Number(item.weightPct.toFixed(1)),
            })),
          },
        ],
      };
    },
    [sectorWeights],
    theme,
    polarity
  );

  const dotVars = [
    "var(--chart-1)",
    "var(--chart-2)",
    "var(--chart-3)",
    "var(--chart-4)",
    "var(--chart-6)",
    "var(--chart-5)",
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "10px", minWidth: 0 }}>
      <div>
        <div style={{ fontWeight: 700, fontSize: "14.5px" }}>申万一级行业与现金仓位</div>
        <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
          单票等权分仓 · 严格 &le;10 席位分散风控
        </div>
      </div>

      <div style={{ position: "relative", width: "100%", height: "178px", minWidth: 0 }}>
        <div ref={chartRef} style={{ width: "100%", height: "100%" }} />
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            pointerEvents: "none",
            textAlign: "center",
            padding: "12px",
          }}
        >
          <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>权益仓位</span>
          <strong className="lh-kpi tabular-nums" style={{ fontSize: "22px" }}>
            {investedPct.toFixed(1)}%
          </strong>
          <span className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-secondary)" }}>
            席位 {openSlotsCount}/{maxSlots}
          </span>
        </div>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "5px", minWidth: 0 }}>
        {sectorWeights.slice(0, 5).map((item, idx) => (
          <div
            key={item.nameZh}
            style={{
              display: "grid",
              gridTemplateColumns: "10px minmax(0, 1fr) auto",
              alignItems: "center",
              gap: "8px",
              fontSize: "12px",
              minWidth: 0,
            }}
          >
            <span
              style={{
                width: "8px",
                height: "8px",
                borderRadius: "999px",
                background: dotVars[idx % dotVars.length],
              }}
            />
            <SafeText text={item.nameZh} />
            <span className="tabular-nums" style={{ fontWeight: 600 }}>
              {item.weightPct.toFixed(1)}%
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};
