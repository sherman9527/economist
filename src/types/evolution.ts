/**
 * Watchlist Observation Win-Rate Pool, Multi-Version Shadow Arena,
 * Root-Cause Attribution & Purged Walk-Forward Anti-Overfit DTO Contract.
 * Strictly mirrors Rust `#[serde(rename_all = "camelCase")]` in `evolution_dto.rs`.
 */

export type WatchlistOutcomeStatus =
  | "TRACKING"
  | "HIT_TARGET"
  | "HIT_STOP"
  | "SETTLED_5D_WIN"
  | "SETTLED_5D_LOSS";

export interface AddWatchlistInputDto {
  symbol: string;
  nameZh: string;
  board: string;
  industrySw1: string;
  entryPriceYuan: number;
  stopLossPriceYuan: number;
  targetPriceYuan: number;
  convictionScore: number;
  sourceDecision: string;
  rationaleZh: string;
  factorZScores: Record<string, number>;
}

export interface WatchlistSummaryDto {
  totalCount: number;
  activeTrackingCount: number;
  settledCount: number;
  winCount: number;
  lossCount: number;
  targetHitCount: number;
  stopHitCount: number;
  settledWinRatePct: number;
  livePositiveRatePct: number;
  wilsonLowPct: number;
  wilsonHighPct: number;
  lowSampleWarning: boolean;
  profitFactor: number;
  expectancyPct: number;
  highConvictionWinRatePct: number;
  baselineConvictionWinRatePct: number;
  roundTripCostBps: number;
}

export interface WatchlistItemDto {
  symbol: string;
  nameZh: string;
  board: string;
  industrySw1: string;
  addedTradeDate: string;
  addedAtIso: string;
  entryPriceYuan: number;
  stopLossPriceYuan: number;
  targetPriceYuan: number;
  latestPriceYuan: number;
  peakPriceYuan: number;
  troughPriceYuan: number;
  convictionScore: number;
  sourceDecision: string;
  versionIdAtEntry: string;
  rationaleZh: string;
  holdingTradingDays: number;
  netReturnPct: number;
  mfePct: number;
  maePct: number;
  outcome: WatchlistOutcomeStatus;
  settledTradeDate?: string | null;
  settledPriceYuan?: number | null;
}

export interface WatchlistOverviewDto {
  summary: WatchlistSummaryDto;
  items: WatchlistItemDto[];
}

export interface VersionSnapshotDto {
  versionId: string;
  parentVersionId?: string | null;
  sourceTag: string;
  createdAt: string;
  summaryZh: string;
  isActive: boolean;
  factorWeightsPct: Record<string, number>;
  minIntradayProjectedTurnoverPct: number;
  atrStopMult: number;
  oosSampleDays: number;
  oosRankIcMean: number;
  oosWinRatePct: number;
  oosCumNetRetBps: number;
  oosMaxDrawdownBps: number;
}

export interface FactorAttributionDto {
  factorId: string;
  baseWeightPct: number;
  targetWeightPct: number;
  weightDeltaPct: number;
  factorOosSpreadBps: number;
  alphaContributionBps: number;
}

export type RootCauseCategory =
  | "FACTOR_WEIGHT_DRAG"
  | "MARKET_REGIME_SHIFT"
  | "STOP_LOSS_PARAM"
  | "OVERFIT_DECAY";

export interface RootCauseFindingDto {
  category: RootCauseCategory;
  severity: "INFO" | "WARN" | "CRITICAL";
  impactBps: number;
  titleZh: string;
  detailZh: string;
  remediationZh: string;
}

export interface VersionComparisonDto {
  baseVersionId: string;
  targetVersionId: string;
  winRateLiftPct: number;
  rankIcLift: number;
  netRetLiftBps: number;
  drawdownImprovementBps: number;
  isUnderperforming: boolean;
  headlineVerdictZh: string;
  factorAttributions: FactorAttributionDto[];
  rootCauses: RootCauseFindingDto[];
}

export interface EvolutionAuditLogDto {
  logId: number;
  evaluatedAt: string;
  triggerSource: string;
  baseVersionId: string;
  resultVersionId?: string | null;
  action: "PROMOTED" | "REJECTED_OVERFIT_GUARD" | "ROLLED_BACK";
  pboEstimate: number;
  deflatedIcIr: number;
  oosRankIc: number;
  oosWinRateLiftPct: number;
  reasonZh: string;
}

export interface VersionControlDashboardDto {
  activeVersionId: string;
  versions: VersionSnapshotDto[];
  comparison?: VersionComparisonDto | null;
  evolutionLogs: EvolutionAuditLogDto[];
}
