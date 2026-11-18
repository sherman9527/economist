import { useEffect, useRef } from "react";
import * as echarts from "echarts";
import type { MarketPolarity, ThemeMode } from "../../types/terminal";

export interface ChartTokenPalette {
  chart1: string;
  chart2: string;
  chart3: string;
  chart4: string;
  chart5: string;
  chart6: string;
  mktUp: string;
  mktDown: string;
  riskVeto: string;
  riskStoploss: string;
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  gridLine: string;
  tooltipBg: string;
  tooltipBorder: string;
}

export function readChartTokenPalette(theme: ThemeMode, polarity: MarketPolarity): ChartTokenPalette {
  if (typeof window === "undefined") {
    return {
      chart1: "#38bdf8",
      chart2: "#818cf8",
      chart3: "#2dd4bf",
      chart4: "#fbbf24",
      chart5: "#fb7185",
      chart6: "#94a3b8",
      mktUp: polarity === "cn-red-up" ? "#ff453a" : "#30d158",
      mktDown: polarity === "cn-red-up" ? "#30d158" : "#ff453a",
      riskVeto: "#ff2d55",
      riskStoploss: "#ff9f0a",
      textPrimary: theme === "dark" ? "#f3f7ff" : "#0f172a",
      textSecondary: theme === "dark" ? "#a9bbd8" : "#334155",
      textMuted: theme === "dark" ? "#7689ab" : "#5b6e88",
      gridLine: "rgba(148, 163, 184, 0.14)",
      tooltipBg: theme === "dark" ? "rgba(15, 24, 46, 0.92)" : "rgba(248, 251, 255, 0.95)",
      tooltipBorder: theme === "dark" ? "rgba(255, 255, 255, 0.2)" : "rgba(15, 23, 42, 0.14)",
    };
  }

  const cs = getComputedStyle(document.documentElement);
  const pick = (name: string, fallback: string) => cs.getPropertyValue(name).trim() || fallback;

  return {
    chart1: pick("--chart-1", "#38bdf8"),
    chart2: pick("--chart-2", "#818cf8"),
    chart3: pick("--chart-3", "#2dd4bf"),
    chart4: pick("--chart-4", "#fbbf24"),
    chart5: pick("--chart-5", "#fb7185"),
    chart6: pick("--chart-6", "#94a3b8"),
    mktUp: pick("--mkt-up", polarity === "cn-red-up" ? "#ff453a" : "#30d158"),
    mktDown: pick("--mkt-down", polarity === "cn-red-up" ? "#30d158" : "#ff453a"),
    riskVeto: pick("--risk-veto", "#ff2d55"),
    riskStoploss: pick("--risk-stoploss", "#ff9f0a"),
    textPrimary: pick("--text-primary", theme === "dark" ? "#f3f7ff" : "#0f172a"),
    textSecondary: pick("--text-secondary", theme === "dark" ? "#a9bbd8" : "#334155"),
    textMuted: pick("--text-muted", theme === "dark" ? "#7689ab" : "#5b6e88"),
    gridLine: pick("--chart-grid", "rgba(148, 163, 184, 0.14)"),
    tooltipBg: theme === "dark" ? "rgba(15, 24, 46, 0.92)" : "rgba(248, 251, 255, 0.95)",
    tooltipBorder: theme === "dark" ? "rgba(255, 255, 255, 0.22)" : "rgba(15, 23, 42, 0.14)",
  };
}

export function canRenderBrowserCharts(): boolean {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return false;
  }
  if (typeof navigator !== "undefined" && /jsdom/i.test(navigator.userAgent)) {
    return false;
  }
  return typeof window.matchMedia === "function";
}

export function useEChartsOption(
  buildOption: (palette: ChartTokenPalette) => echarts.EChartsOption,
  deps: readonly unknown[],
  theme: ThemeMode,
  polarity: MarketPolarity
) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el || !canRenderBrowserCharts()) return;

    let instance: echarts.ECharts | null = null;
    try {
      instance = echarts.init(el, undefined, {
        renderer: "svg",
        width: el.clientWidth > 0 ? el.clientWidth : 540,
        height: el.clientHeight > 0 ? el.clientHeight : 250,
      });
      chartRef.current = instance;
    } catch {
      return;
    }

    const handleResize = () => {
      if (!instance || !el) return;
      if (el.clientWidth > 0 && el.clientHeight > 0) {
        instance.resize({ width: el.clientWidth, height: el.clientHeight });
      }
    };

    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      ro = new ResizeObserver(handleResize);
      ro.observe(el);
    } else {
      window.addEventListener("resize", handleResize);
    }

    return () => {
      ro?.disconnect();
      window.removeEventListener("resize", handleResize);
      instance?.dispose();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    const inst = chartRef.current;
    if (!inst) return;
    const palette = readChartTokenPalette(theme, polarity);
    inst.setOption(buildOption(palette), { notMerge: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [theme, polarity, ...deps]);

  return containerRef;
}
