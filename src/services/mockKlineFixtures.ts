import type {
  KlineBarDto,
  PitfallDimensionStatus,
  ScreenerCandidateDto,
  StockKlinePayloadDto,
} from "../types/terminal";
import { roundCurrency2 } from "../domain/uiInvariants";
import { inferBoardFromSymbol } from "../types/ui";

const DEFAULT_CLEAR_PITFALLS: PitfallDimensionStatus[] = [
  {
    dimension: "INSIDER_REDUCTION",
    labelZh: "大股东减持",
    passed: true,
    severity: "CLEAR",
    detailZh: "近90日无控股股东及董监高预减持公告",
  },
  {
    dimension: "NEGATIVE_NEWS",
    labelZh: "重大利空舆情",
    passed: true,
    severity: "CLEAR",
    detailZh: "无立案调查、问询函及商誉减值预警",
  },
  {
    dimension: "HOLDER_SURGE",
    labelZh: "股东数激增",
    passed: true,
    severity: "CLEAR",
    detailZh: "PIT最新披露股东户数环比下降 -6.8%，筹码趋于集中",
  },
  {
    dimension: "MAIN_OUTFLOW",
    labelZh: "主力资金出逃",
    passed: true,
    severity: "CLEAR",
    detailZh: "连续2日大单净流入占成交额 +9.4%",
  },
  {
    dimension: "TECH_BREAKDOWN",
    labelZh: "高位破位形态",
    passed: true,
    severity: "CLEAR",
    detailZh: "收盘稳站MA20上方，放量阳线突破箱体上沿",
  },
];

const TRADING_DATES_40D = [
  "2026-09-21", "2026-09-22", "2026-09-23", "2026-09-24", "2026-09-25",
  "2026-09-28", "2026-09-29", "2026-09-30", "2026-10-08", "2026-10-09",
  "2026-10-12", "2026-10-13", "2026-10-14", "2026-10-15", "2026-10-16",
  "2026-10-19", "2026-10-20", "2026-10-21", "2026-10-22", "2026-10-23",
  "2026-10-26", "2026-10-27", "2026-10-28", "2026-10-29", "2026-10-30",
  "2026-11-02", "2026-11-03", "2026-11-04", "2026-11-05", "2026-11-06",
  "2026-11-09", "2026-11-10", "2026-11-11", "2026-11-12", "2026-11-13",
  "2026-11-16", "2026-11-17", "2026-11-18",
];

const SLOTS_30M = [
  "09:30", "10:00", "10:30", "11:00", "13:30", "14:00", "14:30", "15:00",
];

function hashSeed(symbol: string): number {
  let h = 2166136261;
  for (let i = 0; i < symbol.length; i += 1) {
    h ^= symbol.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h);
}

export function buildMockStockKline(
  symbol: string,
  period: "1d" | "30m",
  candidate?: ScreenerCandidateDto
): StockKlinePayloadDto {
  const seed = hashSeed(symbol);
  const nameZh = candidate?.nameZh ?? "小盘轮动标的";
  const board = candidate?.board ?? inferBoardFromSymbol(symbol);
  const targetClose = candidate?.latestPriceYuan ?? roundCurrency2(18 + (seed % 2600) / 100);
  const t1Turn = candidate?.turnoverT1Pct ?? 6.4;
  const t0Turn = candidate?.turnoverT0Pct ?? 8.9;
  const stopLoss =
    candidate?.suggestedStopLossYuan ?? roundCurrency2(targetClose * 0.92);
  const targetPrice =
    candidate?.suggestedTargetYuan ?? roundCurrency2(targetClose * 1.16);

  const rawTimestamps: string[] =
    period === "1d"
      ? TRADING_DATES_40D
      : ["2026-11-13", "2026-11-16", "2026-11-17", "2026-11-18"].flatMap((d) =>
          SLOTS_30M.map((hm) => `${d} ${hm}`)
        );

  const totalBars = rawTimestamps.length;
  const startBase = targetClose * 0.84;
  const closes: number[] = [];

  for (let i = 0; i < totalBars; i += 1) {
    const progress = i / Math.max(1, totalBars - 1);
    const wave =
      Math.sin((i + (seed % 7)) * 0.52) * 0.024 +
      Math.cos((i + (seed % 5)) * 0.31) * 0.014;
    const breakoutLift = i >= totalBars - 3 ? (i - (totalBars - 4)) * 0.026 : 0;
    const c =
      i === totalBars - 1
        ? targetClose
        : roundCurrency2(startBase * (1 + progress * 0.12 + wave + breakoutLift));
    closes.push(Math.max(2.5, c));
  }

  const bars: KlineBarDto[] = rawTimestamps.map((ts, idx) => {
    const prev = idx > 0 ? closes[idx - 1] : roundCurrency2(closes[idx] * 0.985);
    const close = closes[idx];
    const open = roundCurrency2(prev * (1 + (((idx + seed) % 5) - 2) * 0.003));
    const spread = Math.max(0.08, Math.abs(close - open) * 0.65 + close * 0.011);
    const high = roundCurrency2(Math.max(open, close) + spread * 0.72);
    const low = roundCurrency2(Math.max(1.0, Math.min(open, close) - spread * 0.64));

    const isIgnitionWindow = idx >= totalBars - 2;
    const baseTurn = 2.8 + ((seed + idx * 13) % 21) / 10;
    const turnoverPct =
      idx === totalBars - 2
        ? t1Turn
        : idx === totalBars - 1
          ? t0Turn
          : Number(baseTurn.toFixed(2));
    const volume = Math.round(
      (isIgnitionWindow ? 195_000 : 88_000) * (0.8 + (turnoverPct / 6.0) * 0.75)
    );

    const ma5Slice = closes.slice(Math.max(0, idx - 4), idx + 1);
    const ma20Slice = closes.slice(Math.max(0, idx - 19), idx + 1);
    const ma5 =
      idx >= 4
        ? roundCurrency2(ma5Slice.reduce((a, b) => a + b, 0) / ma5Slice.length)
        : null;
    const ma20 =
      idx >= 19
        ? roundCurrency2(ma20Slice.reduce((a, b) => a + b, 0) / ma20Slice.length)
        : null;

    return { time: ts, open, high, low, close, volume, turnoverPct, ma5, ma20 };
  });

  const signalBar = bars[bars.length - 1];
  const earlierBar = bars[Math.max(0, bars.length - 9)];

  return {
    symbol,
    nameZh,
    board,
    period,
    bars,
    activeStopLossYuan: stopLoss,
    buyMarkers: [
      {
        recId: `rec-${symbol.replace(".", "-").toLowerCase()}-prev`,
        symbol,
        barTime: earlierBar.time,
        entryPriceYuan: earlierBar.close,
        stopLossPriceYuan: roundCurrency2(earlierBar.close * 0.925),
        targetPriceYuan: roundCurrency2(earlierBar.close * 1.14),
        convictionScore: 83.5,
        rationaleZh:
          "前序箱体缩量回踩MA20确认支撑，连续两日换手温和站上5.2%完成首次试盘建仓锚点。",
        agentSessionId: "b7c4e912-54f1-4a82-9d01-7f1e8820a311",
        pitfalls: candidate?.pitfalls ?? DEFAULT_CLEAR_PITFALLS,
        createdAtIso: "2026-11-06T06:30:00Z",
      },
      {
        recId: `rec-${symbol.replace(".", "-").toLowerCase()}-latest`,
        symbol,
        barTime: signalBar.time,
        entryPriceYuan: signalBar.close,
        stopLossPriceYuan: stopLoss,
        targetPriceYuan: targetPrice,
        convictionScore: candidate?.convictionScore ?? 91.4,
        rationaleZh:
          candidate?.rationaleZh ??
          `总市值低于200亿，连续2日换手率(${t1Turn.toFixed(2)}% -> ${t0Turn.toFixed(2)}%)且放量突破MA20，五维排雷5/5全清。`,
        agentSessionId: "b7c4e912-54f1-4a82-9d01-7f1e8820a311",
        pitfalls: candidate?.pitfalls ?? DEFAULT_CLEAR_PITFALLS,
        createdAtIso: "2026-11-18T06:30:00Z",
      },
    ],
    recentTurnoversPct: [
      { date: "11-12", turnoverPct: 3.42 },
      { date: "11-13", turnoverPct: 4.18 },
      { date: "11-16", turnoverPct: 4.86 },
      { date: "11-17", turnoverPct: t1Turn },
      { date: "11-18", turnoverPct: t0Turn },
    ],
    factorBreakdown: [
      {
        factorId: "turnover_ignition_2d",
        labelZh: "两日换手点火",
        zScore: candidate?.factorZScores?.turnover_ignition_2d ?? 1.94,
        weightPct: 25,
      },
      {
        factorId: "vol_expansion_ratio",
        labelZh: "量比温和扩张",
        zScore: candidate?.factorZScores?.vol_expansion_ratio ?? 1.68,
        weightPct: 20,
      },
      {
        factorId: "smallcap_liq_adj",
        labelZh: "小盘流动性溢价",
        zScore: candidate?.factorZScores?.smallcap_liq_adj ?? 1.41,
        weightPct: 15,
      },
      {
        factorId: "main_flow_persistence",
        labelZh: "主力净流入持续",
        zScore: candidate?.factorZScores?.main_flow_persistence ?? 1.52,
        weightPct: 15,
      },
      {
        factorId: "chip_concentration_pit",
        labelZh: "PIT筹码集中度",
        zScore: candidate?.factorZScores?.chip_concentration_pit ?? 1.26,
        weightPct: 15,
      },
      {
        factorId: "low_atr_breakout",
        labelZh: "低波动箱体突破",
        zScore: candidate?.factorZScores?.low_atr_breakout ?? 1.15,
        weightPct: 10,
      },
    ],
  };
}
