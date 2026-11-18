/**
 * Canonical IPC & Domain DTO Contract between Tauri v2 Rust (`#[serde(rename_all = "camelCase")]`)
 * and React 19 TypeScript Apple Liquid Glass Frontend.
 * Strictly relative paths only; zero hardcoded absolute paths.
 */

export type ExchangeCode = "SH" | "SZ" | "BJ";
export type BoardKind = "MAIN" | "CHINEXT" | "STAR" | "BSE";
export type MarketPolarity = "cn-red-up" | "intl-green-up";
export type ThemeMode = "dark" | "light";
export type MarketSessionState = "PRE_OPEN" | "TRADING" | "LUNCH_BREAK" | "CLOSED";
export type EvidenceTag = "[backtest]" | "[OOS]" | "[assumption]";

export interface PitfallDimensionStatus {
  dimension: "INSIDER_REDUCTION" | "NEGATIVE_NEWS" | "HOLDER_SURGE" | "MAIN_OUTFLOW" | "TECH_BREAKDOWN";
  labelZh: string;
  passed: boolean;
  severity: "CLEAR" | "WARN" | "VETO";
  detailZh: string;
}

export interface ScreenerCandidateDto {
  symbol: string;
  nameZh: string;
  exchange: ExchangeCode;
  board: BoardKind;
  industrySw1: string;
  latestPriceYuan: number;
  prevCloseYuan?: number;
  pctChg: number;
  totalMvYi: number;
  circMvYi: number;
  turnoverT0Pct: number;
  turnoverT1Pct: number;
  projectedTurnoverPct: number;
  volExpansionRatio: number;
  mainNetInflowWan: number;
  ma20PriceYuan: number;
  atr14Yuan: number;
  convictionScore: number;
  suggestedStopLossYuan: number;
  suggestedTargetYuan: number;
  decision: "BUY" | "WATCH" | "REJECTED_BY_MINE";
  pitfallsPassedCount: number;
  pitfalls: PitfallDimensionStatus[];
  factorZScores: Record<string, number>;
  rationaleZh: string;
  klineBarDate: string;
}

export interface UniverseFunnelSummaryDto {
  totalActiveCount: number;
  tradableCount: number;
  under200YiCount: number;
  consecutiveTurnoverVolCount: number;
  pitfallClearedCount: number;
  qualifiedBuyCount: number;
  providerName: string;
  scanRttMs: number;
  scannedAtIso: string;
  marketSession: MarketSessionState;
}

export interface KlineBarDto {
  time: string; // 'YYYY-MM-DD' or 'YYYY-MM-DD HH:mm'
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  turnoverPct: number;
  ma5?: number | null;
  ma20?: number | null;
}

export interface KlineSignalMarkerDto {
  recId: string;
  symbol: string;
  barTime: string;
  entryPriceYuan: number;
  stopLossPriceYuan: number;
  targetPriceYuan?: number | null;
  convictionScore: number;
  rationaleZh: string;
  agentSessionId: string;
  pitfalls: PitfallDimensionStatus[];
  createdAtIso: string;
}

export interface StockKlinePayloadDto {
  symbol: string;
  nameZh: string;
  board: BoardKind;
  period: "1d" | "30m";
  bars: KlineBarDto[];
  buyMarkers: KlineSignalMarkerDto[];
  activeStopLossYuan?: number | null;
  recentTurnoversPct: { date: string; turnoverPct: number }[];
  factorBreakdown: { factorId: string; labelZh: string; zScore: number; weightPct: number }[];
}

export interface PositionSlotDto {
  positionId: string;
  slotNo: number; // 1..10
  symbol: string;
  nameZh: string;
  board: BoardKind;
  totalQty: number;
  sellableQty: number;
  lockedT0Qty: number;
  avgCostYuan: number;
  markPriceYuan: number;
  stopLossPriceYuan: number;
  stopDistancePct: number;
  unrealizedPnlYuan: number;
  unrealizedPnlPct: number;
  entryRationaleZh: string;
  openedAtIso: string;
}

export interface SimulatedOrderDto {
  orderId: string;
  recId?: string | null;
  symbol: string;
  nameZh: string;
  side: "BUY" | "SELL";
  triggerType: "QUANT_SIGNAL" | "STOP_LOSS" | "TAKE_PROFIT" | "REBALANCE_EXIT";
  status: "FILLED" | "REJECTED_LIMIT_LOCKED" | "REJECTED_RISK_CAP" | "PENDING";
  lotQty: number;
  entryPriceYuan: number;
  fillPriceYuan: number;
  stopLossPriceYuan: number;
  commissionYuan: number;
  stampDutyYuan: number;
  slippageYuan: number;
  rationaleZh: string;
  tradeDate: string;
  tPlus1UnlockDate: string;
  createdAtIso: string;
}

export interface EquityPointDto {
  date: string;
  portfolioNav: number;
  csi1000Nav: number;
  cni2000Nav: number;
  excessPct: number;
}

export interface PortfolioOverviewDto {
  initialCapitalYuan: number; // 1,000,000
  cashYuan: number;
  marketValueYuan: number;
  totalEquityYuan: number;
  cumReturnPct: number;
  dailyPnlYuan: number;
  dailyReturnPct: number;
  maxDrawdownPct: number;
  openSlotsCount: number; // 0..10
  maxSlots: number; // 10
  totalFeesPaidYuan: number;
  positions: PositionSlotDto[];
  recentOrders: SimulatedOrderDto[];
  equityCurve: EquityPointDto[];
  sectorWeights: { nameZh: string; weightPct: number; valueYuan: number }[];
}

export interface FactorDiagnosticDto {
  factorId: string;
  nameZh: string;
  expr: string;
  barFreq: "1d" | "30m";
  weightPct: number;
  rankIcMean: number;
  icIr: number;
  oosWinRatePct: number;
  evidenceTag: EvidenceTag;
  status: "ACTIVE" | "CANDIDATE" | "DEPRECATED";
  quantileReturnsPct: number[]; // Q1..Q10
  icDecay: number[]; // horizon 1..10d
}

export interface StrategyPluginConfigDto {
  strategyId: string;
  version: string;
  maxTotalMvYi: number; // locked 200.0
  minTurnoverPct: number; // locked 5.0
  consecutiveDays: number; // locked 2
  minIntradayProjectedTurnoverPct: number;
  maxPositions: number; // locked 10
  initialCapitalYuan: number; // locked 1000000
  insiderSellVetoPct: number;
  holderSurgeVetoPct: number;
  mainOutflowVetoRatio: number;
  atrStopMult: number;
  marketPolarity: MarketPolarity;
  factors: FactorDiagnosticDto[];
  manifestRelPath: string;
  rawStrategyJson: string;
}

export interface EconomistReportItemDto {
  reportId: string;
  date: string;
  titleZh: string;
  relPath: string;
  articleCount: number;
  coreCount: number;
  acked: boolean;
  ackedAt?: string | null;
  usScore: number;
  cnScore: number;
  executiveSummaryZh: string;
  chains: {
    chainTitle: string;
    economistTrigger: string;
    usMapping: string;
    cnMapping: string;
    divergenceInsight: string;
    relatedCnSymbols: string[];
  }[];
}

export interface EconomistModuleDto {
  unackedCount: number;
  reports: EconomistReportItemDto[];
  backtestHitRatePct: number;
  usHitRatePct: number;
  cnHitRatePct: number;
  bearishRiskSectorsZh: string[];
}

export interface AgentSessionDto {
  agentRole: "main_supervisor" | "quant_researcher" | "ui_designer" | "economist_analyst" | "risk_auditor";
  sessionId: string;
  parentSessionId?: string | null;
  model: "gemini-4-carbon";
  reasoningEffort: "high";
  specRelPath: string;
  turnCount: number;
  status: "IDLE" | "RUNNING" | "DEGRADED" | "RECOVERING";
  lastWokenAt?: string | null;
}

export interface StorageHygieneDto {
  portableRootRel: string;
  buildFootprintBytes: number;
  buildCapBytes: number; // 5_368_709_120 (5 GB)
  sqliteDbBytes: number;
  retentionDays: number; // 28
  zeroAbsolutePathVerified: boolean;
  maxFileLinesObserved: number;
}

export interface TelemetryEventDto {
  eventId: number;
  tsIso: string;
  severity: "DEBUG" | "INFO" | "WARN" | "ERROR" | "CRITICAL";
  subsystem: "MARKET" | "QUANT" | "TRADING" | "AGENT" | "NEWS" | "UI" | "CLEANER";
  eventName: string;
  durationMs?: number | null;
  ok: boolean;
  message: string;
}

export interface TelemetryDashboardDto {
  sessions: AgentSessionDto[];
  storage: StorageHygieneDto;
  events: TelemetryEventDto[];
}

export interface RotationAlertEventDto {
  slotTimestampCst: string;
  qualifiedCount: number;
  topPicks: ScreenerCandidateDto[];
}

export interface TerminalBootstrapDto {
  funnel: UniverseFunnelSummaryDto;
  candidates: ScreenerCandidateDto[];
  portfolio: PortfolioOverviewDto;
  featuredKline: StockKlinePayloadDto;
  pluginConfig: StrategyPluginConfigDto;
  economist: EconomistModuleDto;
  telemetry: TelemetryDashboardDto;
}

export interface PlaceOrderInputDto {
  symbol: string;
  lotQty?: number | null;
  rationaleZh?: string | null;
  stopLossPriceYuan?: number | null;
}

export interface UpdatePluginInputDto {
  minIntradayProjectedTurnoverPct: number;
  insiderSellVetoPct: number;
  holderSurgeVetoPct: number;
  mainOutflowVetoRatio: number;
  atrStopMult: number;
  marketPolarity: MarketPolarity;
  factorWeights: Record<string, number>;
}
