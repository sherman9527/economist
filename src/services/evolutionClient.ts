import { invoke } from "@tauri-apps/api/core";
import type {
  AddWatchlistInputDto,
  VersionControlDashboardDto,
  WatchlistOverviewDto,
} from "../types/terminal";
import { normalizeBoundedWeightsPct } from "../domain/evolutionInvariants";
import {
  buildMockComparison,
  summarizeMockWatchlist,
} from "./mockEvolutionFixtures";
import {
  cloneDto,
  getMutableFallbackState,
  isTauriRuntime,
} from "./tauriClient";

export async function getWatchlistOverview(): Promise<WatchlistOverviewDto> {
  if (isTauriRuntime()) {
    return invoke<WatchlistOverviewDto>("get_watchlist_overview");
  }
  const st = getMutableFallbackState();
  return cloneDto(st.watchlist);
}

export async function addToWatchlist(
  input: AddWatchlistInputDto
): Promise<WatchlistOverviewDto> {
  if (isTauriRuntime()) {
    return invoke<WatchlistOverviewDto>("add_to_watchlist", { input });
  }
  const st = getMutableFallbackState();
  const nowIso = new Date().toISOString();
  const today = nowIso.slice(0, 10);
  const existingIdx = st.watchlist.items.findIndex(
    (x) => x.symbol === input.symbol
  );
  const entry = {
    symbol: input.symbol,
    nameZh: input.nameZh,
    board: input.board,
    industrySw1: input.industrySw1,
    addedTradeDate: today,
    addedAtIso: nowIso,
    entryPriceYuan: input.entryPriceYuan,
    stopLossPriceYuan: input.stopLossPriceYuan,
    targetPriceYuan: input.targetPriceYuan,
    latestPriceYuan: input.entryPriceYuan,
    peakPriceYuan: input.entryPriceYuan,
    troughPriceYuan: input.entryPriceYuan,
    convictionScore: input.convictionScore,
    sourceDecision: input.sourceDecision,
    versionIdAtEntry: st.versionControl.activeVersionId,
    rationaleZh: input.rationaleZh,
    holdingTradingDays: 0,
    netReturnPct: -0.25,
    mfePct: 0,
    maePct: 0,
    outcome: "TRACKING" as const,
    settledTradeDate: null,
    settledPriceYuan: null,
  };
  const nextItems =
    existingIdx >= 0
      ? st.watchlist.items.map((it, idx) => (idx === existingIdx ? entry : it))
      : [entry, ...st.watchlist.items];
  st.watchlist = summarizeMockWatchlist(nextItems);
  return cloneDto(st.watchlist);
}

export async function removeFromWatchlist(
  symbol: string
): Promise<WatchlistOverviewDto> {
  if (isTauriRuntime()) {
    return invoke<WatchlistOverviewDto>("remove_from_watchlist", { symbol });
  }
  const st = getMutableFallbackState();
  const nextItems = st.watchlist.items.filter((x) => x.symbol !== symbol);
  st.watchlist = summarizeMockWatchlist(nextItems);
  return cloneDto(st.watchlist);
}

export async function getVersionControlDashboard(
  baseVersionId?: string | null,
  targetVersionId?: string | null
): Promise<VersionControlDashboardDto> {
  if (isTauriRuntime()) {
    return invoke<VersionControlDashboardDto>("get_version_control_dashboard", {
      baseVersionId: baseVersionId ?? null,
      targetVersionId: targetVersionId ?? null,
    });
  }
  const st = getMutableFallbackState();
  const vc = st.versionControl;
  const active =
    vc.versions.find((v) => v.isActive) ?? vc.versions[vc.versions.length - 1];
  const fallbackBase =
    active.parentVersionId ??
    vc.versions.find((v) => v.versionId !== active.versionId)?.versionId ??
    active.versionId;
  const effBase = baseVersionId ?? fallbackBase;
  const effTarget = targetVersionId ?? active.versionId;
  vc.comparison = buildMockComparison(vc.versions, effBase, effTarget);
  return cloneDto(vc);
}

export async function switchStrategyVersion(
  versionId: string
): Promise<VersionControlDashboardDto> {
  if (isTauriRuntime()) {
    return invoke<VersionControlDashboardDto>("switch_strategy_version", {
      versionId,
    });
  }
  const st = getMutableFallbackState();
  const target = st.versionControl.versions.find(
    (v) => v.versionId === versionId
  );
  if (!target) {
    throw new Error(`未找到策略版本快照: ${versionId}`);
  }
  for (const v of st.versionControl.versions) {
    v.isActive = v.versionId === versionId;
  }
  st.versionControl.activeVersionId = versionId;
  st.pluginConfig.version = versionId.replace(/^v/, "");
  st.pluginConfig.minIntradayProjectedTurnoverPct =
    target.minIntradayProjectedTurnoverPct;
  st.pluginConfig.atrStopMult = target.atrStopMult;
  st.pluginConfig.factors = st.pluginConfig.factors.map((f) => ({
    ...f,
    weightPct: target.factorWeightsPct[f.factorId] ?? f.weightPct,
  }));
  const baseId =
    target.parentVersionId ??
    st.versionControl.versions.find((v) => v.versionId !== versionId)
      ?.versionId ??
    versionId;
  st.versionControl.comparison = buildMockComparison(
    st.versionControl.versions,
    baseId,
    versionId
  );
  return cloneDto(st.versionControl);
}

export async function runAntiOverfitIteration(): Promise<VersionControlDashboardDto> {
  if (isTauriRuntime()) {
    return invoke<VersionControlDashboardDto>("run_anti_overfit_iteration");
  }
  const st = getMutableFallbackState();
  const nowIso = new Date().toISOString();
  const active =
    st.versionControl.versions.find((v) => v.isActive) ??
    st.versionControl.versions[0];
  const rawTilted = st.pluginConfig.factors.map((f) => f.weightPct + (f.rankIcMean > 0.045 ? 1.8 : -1.2));
  const bounded = normalizeBoundedWeightsPct(rawTilted, 5, 35);
  void bounded;

  st.versionControl.evolutionLogs.unshift({
    logId: Date.now() % 1_000_000,
    evaluatedAt: nowIso,
    triggerSource: "MANUAL_UI",
    baseVersionId: active.versionId,
    resultVersionId: null,
    action: "REJECTED_OVERFIT_GUARD",
    pboEstimate: 0.29,
    deflatedIcIr: 0.41,
    oosRankIc: active.oosRankIcMean,
    oosWinRateLiftPct: 0.35,
    reasonZh: `5日隔离带滚动样本外验证完成：候选权重 OOS 增量仅 +0.35%（低于单步显著性门槛 +0.80%），保持当前版本 ${active.versionId} 稳健运行，严拒样本内噪音微调。`,
  });
  return cloneDto(st.versionControl);
}
