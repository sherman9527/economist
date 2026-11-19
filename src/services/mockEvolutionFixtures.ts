import type {
  FactorAttributionDto,
  RootCauseFindingDto,
  VersionComparisonDto,
  VersionControlDashboardDto,
  VersionSnapshotDto,
  WatchlistItemDto,
  WatchlistOverviewDto,
} from "../types/evolution";
import { computeWilsonBoundsPct } from "../domain/evolutionInvariants";

export function summarizeMockWatchlist(items: WatchlistItemDto[]): WatchlistOverviewDto {
  const settled = items.filter((i) => i.outcome !== "TRACKING");
  const wins = settled.filter(
    (i) => i.outcome === "HIT_TARGET" || i.outcome === "SETTLED_5D_WIN"
  );
  const targetHitCount = settled.filter((i) => i.outcome === "HIT_TARGET").length;
  const stopHitCount = settled.filter((i) => i.outcome === "HIT_STOP").length;
  const tracking = items.filter((i) => i.outcome === "TRACKING");
  const livePos = tracking.filter((i) => i.netReturnPct > 0).length;
  const [wLo, wHi] = computeWilsonBoundsPct(wins.length, settled.length);

  let posSum = 0;
  let negSum = 0;
  let totalRet = 0;
  for (const s of settled) {
    totalRet += s.netReturnPct;
    if (s.netReturnPct > 0) posSum += s.netReturnPct;
    else if (s.netReturnPct < 0) negSum += Math.abs(s.netReturnPct);
  }
  const hiGroup = settled.filter((s) => s.convictionScore >= 68);
  const hiWins = hiGroup.filter(
    (s) => s.outcome === "HIT_TARGET" || s.outcome === "SETTLED_5D_WIN"
  ).length;
  const baseGroup = settled.filter((s) => s.convictionScore < 68);
  const baseWins = baseGroup.filter(
    (s) => s.outcome === "HIT_TARGET" || s.outcome === "SETTLED_5D_WIN"
  ).length;

  return {
    summary: {
      totalCount: items.length,
      activeTrackingCount: tracking.length,
      settledCount: settled.length,
      winCount: wins.length,
      lossCount: settled.length - wins.length,
      targetHitCount,
      stopHitCount,
      settledWinRatePct: settled.length > 0 ? Number(((wins.length / settled.length) * 100).toFixed(2)) : 0,
      livePositiveRatePct: tracking.length > 0 ? Number(((livePos / tracking.length) * 100).toFixed(2)) : 0,
      wilsonLowPct: Number(wLo.toFixed(2)),
      wilsonHighPct: Number(wHi.toFixed(2)),
      lowSampleWarning: settled.length < 20,
      profitFactor: negSum > 1e-6 ? Number((posSum / negSum).toFixed(2)) : posSum > 0 ? 9.99 : 0,
      expectancyPct: settled.length > 0 ? Number((totalRet / settled.length).toFixed(2)) : 0,
      highConvictionWinRatePct: hiGroup.length > 0 ? Number(((hiWins / hiGroup.length) * 100).toFixed(2)) : 0,
      baselineConvictionWinRatePct: baseGroup.length > 0 ? Number(((baseWins / baseGroup.length) * 100).toFixed(2)) : 0,
      roundTripCostBps: 25,
    },
    items,
  };
}

export function createMockWatchlist(): WatchlistOverviewDto {
  const items: WatchlistItemDto[] = [
    {
      symbol: "688228.SH",
      nameZh: "开普云",
      board: "STAR",
      industrySw1: "计算机",
      addedTradeDate: "2026-11-11",
      addedAtIso: "2026-11-11T10:30:00+08:00",
      entryPriceYuan: 41.2,
      stopLossPriceYuan: 38.4,
      targetPriceYuan: 47.38,
      latestPriceYuan: 47.65,
      peakPriceYuan: 48.1,
      troughPriceYuan: 40.6,
      convictionScore: 76.4,
      sourceDecision: "BUY",
      versionIdAtEntry: "v1.2.0",
      rationaleZh: "政务AI工作流放量突破，观望验证样本外胜率",
      holdingTradingDays: 4,
      netReturnPct: 15.41,
      mfePct: 16.75,
      maePct: -1.46,
      outcome: "HIT_TARGET",
      settledTradeDate: "2026-11-17",
      settledPriceYuan: 47.65,
    },
    {
      symbol: "836239.BJ",
      nameZh: "长虹能源",
      board: "BSE",
      industrySw1: "电力设备",
      addedTradeDate: "2026-11-10",
      addedAtIso: "2026-11-10T14:00:00+08:00",
      entryPriceYuan: 24.8,
      stopLossPriceYuan: 23.0,
      targetPriceYuan: 28.52,
      latestPriceYuan: 26.35,
      peakPriceYuan: 27.1,
      troughPriceYuan: 24.1,
      convictionScore: 71.8,
      sourceDecision: "WATCH",
      versionIdAtEntry: "v1.2.0",
      rationaleZh: "BBU高倍率电芯连续两日换手>6%但大盘分歧，纳入观望池跟踪",
      holdingTradingDays: 5,
      netReturnPct: 6.0,
      mfePct: 9.27,
      maePct: -2.82,
      outcome: "SETTLED_5D_WIN",
      settledTradeDate: "2026-11-17",
      settledPriceYuan: 26.35,
    },
    {
      symbol: "300766.SZ",
      nameZh: "每日互动",
      board: "CHINEXT",
      industrySw1: "计算机",
      addedTradeDate: "2026-11-12",
      addedAtIso: "2026-11-12T11:00:00+08:00",
      entryPriceYuan: 19.5,
      stopLossPriceYuan: 18.15,
      targetPriceYuan: 22.43,
      latestPriceYuan: 18.1,
      peakPriceYuan: 20.05,
      troughPriceYuan: 18.02,
      convictionScore: 63.2,
      sourceDecision: "WATCH",
      versionIdAtEntry: "v1.1.0",
      rationaleZh: "换手扩张但主力净流入偏弱，观望跟踪止损有效性",
      holdingTradingDays: 3,
      netReturnPct: -7.19,
      mfePct: 2.82,
      maePct: -7.59,
      outcome: "HIT_STOP",
      settledTradeDate: "2026-11-17",
      settledPriceYuan: 18.15,
    },
  ];
  return summarizeMockWatchlist(items);
}

const FACTOR_SPREADS_OOS_BPS: Record<string, number> = {
  F1_ideal_reversal: 186,
  F2_concave_abn_turnover: 154,
  F3_amihud_compression: 118,
  F4_intraday_apm_vwap: 92,
  F5_main_inflow_ir: 136,
  F6_pv_efficiency: -64,
};

export function buildMockComparison(
  versions: VersionSnapshotDto[],
  baseId: string,
  targetId: string
): VersionComparisonDto {
  const base = versions.find((v) => v.versionId === baseId) ?? versions[0];
  const target = versions.find((v) => v.versionId === targetId) ?? versions[versions.length - 1];
  const winRateLiftPct = Number((target.oosWinRatePct - base.oosWinRatePct).toFixed(2));
  const rankIcLift = Number((target.oosRankIcMean - base.oosRankIcMean).toFixed(4));
  const netRetLiftBps = Number((target.oosCumNetRetBps - base.oosCumNetRetBps).toFixed(2));
  const drawdownImprovementBps = Number((base.oosMaxDrawdownBps - target.oosMaxDrawdownBps).toFixed(2));
  const isUnderperforming =
    netRetLiftBps < -5 || (winRateLiftPct < -1.5 && rankIcLift < 0);

  const factorAttributions: FactorAttributionDto[] = Object.keys(FACTOR_SPREADS_OOS_BPS).map(
    (fid) => {
      const bw = base.factorWeightsPct[fid] ?? 16.67;
      const tw = target.factorWeightsPct[fid] ?? 16.67;
      const dw = Number((tw - bw).toFixed(2));
      const sp = FACTOR_SPREADS_OOS_BPS[fid];
      return {
        factorId: fid,
        baseWeightPct: bw,
        targetWeightPct: tw,
        weightDeltaPct: dw,
        factorOosSpreadBps: sp,
        alphaContributionBps: Number(((dw / 100) * sp).toFixed(2)),
      };
    }
  );

  const rootCauses: RootCauseFindingDto[] = [];
  if (isUnderperforming) {
    rootCauses.push(
      {
        category: "FACTOR_WEIGHT_DRAG",
        severity: "CRITICAL",
        impactBps: -48.6,
        titleZh: "因子权重调拨负贡献：高换手追涨因子加仓拖累样本外净值",
        detailZh: `${target.versionId} 相比 ${base.versionId} 上调 F6_pv_efficiency (+16.0%) 并削减 F1_ideal_reversal (-12.0%)，近期高换手追涨分组在样本外遭兑现反杀。`,
        remediationZh: `建议一键切回 ${base.versionId} 或启用 James-Stein 收缩防过拟合权重。`,
      },
      {
        category: "MARKET_REGIME_SHIFT",
        severity: "WARN",
        impactBps: -24.0,
        titleZh: "市场微观结构切换：小盘冲高回落长上影频率抬升",
        detailZh: "近 20 日样本外长上影比率升至 41%，纯突破动量版本胜率受损，反转与非流动性压缩因子占优。",
        remediationZh: "保持高换手理想反转因子 F1 权重 >= 20%，避免顺周期追高过拟合。",
      },
      {
        category: "STOP_LOSS_PARAM",
        severity: "INFO",
        impactBps: -11.5,
        titleZh: "风控与盘中换手门限差异放大震荡摩擦",
        detailZh: `ATR 止损乘数由 ${base.atrStopMult.toFixed(1)} 缩紧至 ${target.atrStopMult.toFixed(1)}，小盘高波动洗盘期触发提前止损频次上升。`,
        remediationZh: "维持 ATR(14) 止损乘数在 2.0 附近以兼顾盈亏比与洗盘容忍度。",
      }
    );
  }

  return {
    baseVersionId: base.versionId,
    targetVersionId: target.versionId,
    winRateLiftPct,
    rankIcLift,
    netRetLiftBps,
    drawdownImprovementBps,
    isUnderperforming,
    headlineVerdictZh: isUnderperforming
      ? `退化预警：${target.versionId} 在平行样本外跟踪中落后老版本 ${base.versionId}（净收益 ${netRetLiftBps.toFixed(1)} bps，胜率 ${winRateLiftPct.toFixed(1)}%），已定位 ${rootCauses.length} 项具体根因！`
      : `升级验证有效：${target.versionId} 相比 ${base.versionId} 样本外胜率提升 +${winRateLiftPct.toFixed(1)}%，累计净超额 +${netRetLiftBps.toFixed(1)} bps。`,
    factorAttributions,
    rootCauses,
  };
}

export function createMockVersionControl(): VersionControlDashboardDto {
  const versions: VersionSnapshotDto[] = [
    {
      versionId: "v1.0.0",
      parentVersionId: null,
      sourceTag: "GENESIS_EQUAL_WEIGHT",
      createdAt: "2026-10-15T09:00:00+08:00",
      summaryZh: "初代等权六因子基线（无截面因子倾斜，标准 2.2x ATR 止损）",
      isActive: false,
      factorWeightsPct: {
        F1_ideal_reversal: 16.67,
        F2_concave_abn_turnover: 16.67,
        F3_amihud_compression: 16.67,
        F4_intraday_apm_vwap: 16.67,
        F5_main_inflow_ir: 16.66,
        F6_pv_efficiency: 16.66,
      },
      minIntradayProjectedTurnoverPct: 5.5,
      atrStopMult: 2.2,
      oosSampleDays: 24,
      oosRankIcMean: 0.041,
      oosWinRatePct: 54.2,
      oosCumNetRetBps: 142.5,
      oosMaxDrawdownBps: 168.0,
    },
    {
      versionId: "v1.1.0",
      parentVersionId: "v1.0.0",
      sourceTag: "LIQUIDITY_DEFENSIVE",
      createdAt: "2026-10-29T15:05:00+08:00",
      summaryZh: "强化 Amihud 非流动性压缩与主力净流入信息比的稳健防守版",
      isActive: false,
      factorWeightsPct: {
        F1_ideal_reversal: 18,
        F2_concave_abn_turnover: 16,
        F3_amihud_compression: 24,
        F4_intraday_apm_vwap: 12,
        F5_main_inflow_ir: 20,
        F6_pv_efficiency: 10,
      },
      minIntradayProjectedTurnoverPct: 5.8,
      atrStopMult: 2.0,
      oosSampleDays: 24,
      oosRankIcMean: 0.048,
      oosWinRatePct: 57.6,
      oosCumNetRetBps: 196.4,
      oosMaxDrawdownBps: 132.0,
    },
    {
      versionId: "v1.2.0",
      parentVersionId: "v1.1.0",
      sourceTag: "PRODUCTION_BASELINE",
      createdAt: "2026-11-12T15:05:00+08:00",
      summaryZh: "生产主力版：高换手理想反转 + 凹向异常换手点火 + 五维排雷耦合",
      isActive: true,
      factorWeightsPct: {
        F1_ideal_reversal: 22,
        F2_concave_abn_turnover: 20,
        F3_amihud_compression: 16,
        F4_intraday_apm_vwap: 16,
        F5_main_inflow_ir: 14,
        F6_pv_efficiency: 12,
      },
      minIntradayProjectedTurnoverPct: 5.8,
      atrStopMult: 2.0,
      oosSampleDays: 24,
      oosRankIcMean: 0.056,
      oosWinRatePct: 61.4,
      oosCumNetRetBps: 248.8,
      oosMaxDrawdownBps: 118.5,
    },
    {
      versionId: "v1.3.0-exp",
      parentVersionId: "v1.2.0",
      sourceTag: "MANUAL_MOMENTUM_TILT",
      createdAt: "2026-11-17T15:10:00+08:00",
      summaryZh: "激进追涨实验版：重仓量价突破 F6 (28%) 并收紧 ATR 止损至 1.5x（保留影子对照）",
      isActive: false,
      factorWeightsPct: {
        F1_ideal_reversal: 10,
        F2_concave_abn_turnover: 18,
        F3_amihud_compression: 12,
        F4_intraday_apm_vwap: 18,
        F5_main_inflow_ir: 14,
        F6_pv_efficiency: 28,
      },
      minIntradayProjectedTurnoverPct: 6.6,
      atrStopMult: 1.5,
      oosSampleDays: 24,
      oosRankIcMean: 0.033,
      oosWinRatePct: 51.8,
      oosCumNetRetBps: 112.4,
      oosMaxDrawdownBps: 194.0,
    },
  ];

  return {
    activeVersionId: "v1.2.0",
    versions,
    comparison: buildMockComparison(versions, "v1.1.0", "v1.2.0"),
    evolutionLogs: [
      {
        logId: 1,
        evaluatedAt: "2026-11-18T15:00:12+08:00",
        triggerSource: "POST_CLOSE_1500",
        baseVersionId: "v1.2.0",
        resultVersionId: null,
        action: "REJECTED_OVERFIT_GUARD",
        pboEstimate: 0.42,
        deflatedIcIr: 0.19,
        oosRankIc: 0.011,
        oosWinRateLiftPct: -1.8,
        reasonZh:
          "5日隔离带滚动样本外验证拦截：候选权重过拟合风险 PBO=42.0% > 35% 红线且 OOS 胜率未显著超越 v1.2.0，拒绝晋升以保护实盘信号。",
      },
      {
        logId: 2,
        evaluatedAt: "2026-11-12T15:05:00+08:00",
        triggerSource: "POST_CLOSE_1500",
        baseVersionId: "v1.1.0",
        resultVersionId: "v1.2.0",
        action: "PROMOTED",
        pboEstimate: 0.16,
        deflatedIcIr: 0.74,
        oosRankIc: 0.056,
        oosWinRateLiftPct: 3.8,
        reasonZh:
          "通过 60% 训练 -> 5日隔离带 -> 40% 样本外检验（PBO=16.0%，Deflated IC_IR=0.74），经 λ=0.25 收缩后晋升为 v1.2.0。",
      },
    ],
  };
}
