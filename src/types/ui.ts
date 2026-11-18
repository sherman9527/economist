import type { BoardKind, ExchangeCode } from "./terminal";

export type WorkspacePageId =
  | "dashboard"
  | "screener"
  | "ledger"
  | "factor-lab"
  | "economist"
  | "telemetry";

export type AsyncActionState = "idle" | "loading" | "success" | "error";

export type EquityRangeKey = "1D" | "1W" | "1M" | "1Y";

export type FactorChartMode = "turnover" | "zscore";

export type EconomistSubPivot = "chains" | "ack-queue" | "hit-rate" | "reader";

export interface KlineFocusRequest {
  symbol: string;
  period?: "1d" | "30m";
  highlightRecId?: string;
}

export interface BoardTradingRule {
  limitRatio: number;
  minLotQty: number;
  lotStep: number;
}

export interface OrderValidationResult {
  valid: boolean;
  lowerLimitYuan: number;
  upperLimitYuan: number;
  reasons: string[];
}

export interface FeeBreakdown {
  notionalYuan: number;
  commissionYuan: number;
  stampDutyYuan: number;
  slippageYuan: number;
  totalFeeYuan: number;
}

export interface ResponsiveLayoutContract {
  shellMode: "mobile-tabs" | "compact-sidebar" | "expanded-sidebar";
  kpiCols: 1 | 2 | 4;
  mainSplit: "stack" | "12-col";
  screenerMode: "cards" | "table";
}

export function inferBoardFromSymbol(symbol: string, exchange?: ExchangeCode): BoardKind {
  const code = symbol.split(".")[0] ?? symbol;
  if (exchange === "BJ" || code.startsWith("8") || code.startsWith("4") || code.startsWith("92")) {
    return "BSE";
  }
  if (code.startsWith("688") || code.startsWith("689")) {
    return "STAR";
  }
  if (code.startsWith("300") || code.startsWith("301")) {
    return "CHINEXT";
  }
  return "MAIN";
}
