import type {
  BoardKind,
  MarketSessionState,
  PortfolioOverviewDto,
} from "../types/terminal";
import type {
  BoardTradingRule,
  FeeBreakdown,
  OrderValidationResult,
  ResponsiveLayoutContract,
} from "../types/ui";

const CST_OFFSET_MS = 8 * 3_600_000;
const MAX_BLUR_BUDGET_HARD_CAP = 8;

const TRADING_ROTATION_SLOTS_MINUTES = [
  9 * 60 + 30,
  10 * 60,
  10 * 60 + 30,
  11 * 60,
  11 * 60 + 30,
  13 * 60,
  13 * 60 + 30,
  14 * 60,
  14 * 60 + 30,
  15 * 60,
] as const;

export function roundCurrency2(val: number): number {
  if (!Number.isFinite(val)) return 0;
  return Math.round((val + Number.EPSILON) * 100) / 100;
}

export function getBoardTradingRule(board: BoardKind): BoardTradingRule {
  switch (board) {
    case "MAIN":
      return { limitRatio: 0.1, minLotQty: 100, lotStep: 100 };
    case "CHINEXT":
      return { limitRatio: 0.2, minLotQty: 100, lotStep: 100 };
    case "STAR":
      return { limitRatio: 0.2, minLotQty: 200, lotStep: 1 };
    case "BSE":
      return { limitRatio: 0.3, minLotQty: 100, lotStep: 1 };
  }
}

export function validateAShareOrderInput(params: {
  board: BoardKind;
  prevCloseYuan: number;
  orderPriceYuan: number;
  lotQty: number;
}): OrderValidationResult {
  const { board, prevCloseYuan, orderPriceYuan, lotQty } = params;
  const rule = getBoardTradingRule(board);
  const reasons: string[] = [];

  if (!Number.isFinite(prevCloseYuan) || prevCloseYuan <= 0) {
    return {
      valid: false,
      lowerLimitYuan: 0,
      upperLimitYuan: 0,
      reasons: ["前收盘价无效"],
    };
  }

  const lowerLimitYuan = roundCurrency2(prevCloseYuan * (1 - rule.limitRatio));
  const upperLimitYuan = roundCurrency2(prevCloseYuan * (1 + rule.limitRatio));

  if (!Number.isFinite(orderPriceYuan) || orderPriceYuan <= 0) {
    reasons.push("委托价格必须大于 0");
  } else if (orderPriceYuan < lowerLimitYuan - 1e-6 || orderPriceYuan > upperLimitYuan + 1e-6) {
    reasons.push(
      `委托价 ¥${orderPriceYuan.toFixed(2)} 超出 ${board} 涨跌停区间 [¥${lowerLimitYuan.toFixed(2)}, ¥${upperLimitYuan.toFixed(2)}]`
    );
  }

  if (!Number.isInteger(lotQty) || lotQty < rule.minLotQty) {
    reasons.push(`${board} 最低申报股数为 ${rule.minLotQty} 股`);
  } else if ((lotQty - rule.minLotQty) % rule.lotStep !== 0) {
    reasons.push(`${board} 申报股数须按 ${rule.lotStep} 股递增`);
  }

  return {
    valid: reasons.length === 0,
    lowerLimitYuan,
    upperLimitYuan,
    reasons,
  };
}

export function computeAShareFeeBreakdown(
  side: "BUY" | "SELL",
  fillPriceYuan: number,
  lotQty: number
): FeeBreakdown {
  const notionalYuan = roundCurrency2(Math.max(0, fillPriceYuan) * Math.max(0, lotQty));
  if (notionalYuan <= 0) {
    return {
      notionalYuan: 0,
      commissionYuan: 0,
      stampDutyYuan: 0,
      slippageYuan: 0,
      totalFeeYuan: 0,
    };
  }
  const commissionYuan = roundCurrency2(Math.max(5, notionalYuan * 0.00025));
  const stampDutyYuan = side === "SELL" ? roundCurrency2(notionalYuan * 0.0005) : 0;
  const slippageYuan = roundCurrency2(notionalYuan * 0.0005);
  const totalFeeYuan = roundCurrency2(commissionYuan + stampDutyYuan + slippageYuan);
  return {
    notionalYuan,
    commissionYuan,
    stampDutyYuan,
    slippageYuan,
    totalFeeYuan,
  };
}

export function validatePortfolioInvariants(portfolio: PortfolioOverviewDto): {
  valid: boolean;
  violations: string[];
} {
  const violations: string[] = [];

  if (portfolio.maxSlots > 10 || portfolio.maxSlots <= 0) {
    violations.push(`持仓席位上限异常 (${portfolio.maxSlots})`);
  }
  if (portfolio.positions.length > 10 || portfolio.positions.length > portfolio.maxSlots) {
    violations.push(`当前持仓数 (${portfolio.positions.length}) 超出 10 只上限`);
  }
  if (portfolio.openSlotsCount !== portfolio.positions.length) {
    violations.push(`openSlotsCount (${portfolio.openSlotsCount}) 与持仓列表长度不一致`);
  }
  if (portfolio.cashYuan < -0.01) {
    violations.push("可用现金不可为负");
  }

  for (const pos of portfolio.positions) {
    if (pos.sellableQty < 0 || pos.lockedT0Qty < 0) {
      violations.push(`${pos.symbol} T+1 可卖或锁仓股数为负`);
    }
    if (pos.totalQty !== pos.sellableQty + pos.lockedT0Qty) {
      violations.push(
        `${pos.symbol} 总持仓 (${pos.totalQty}) != 可卖 (${pos.sellableQty}) + T+1锁仓 (${pos.lockedT0Qty})`
      );
    }
    const stopCeilingYuan = Math.max(pos.avgCostYuan, pos.markPriceYuan);
    const isValidBreachedRatchetedStop =
      pos.stopLossPriceYuan > 0 &&
      pos.stopLossPriceYuan > pos.avgCostYuan &&
      pos.markPriceYuan <= pos.stopLossPriceYuan &&
      pos.stopDistancePct <= 0;
    if (
      !(
        (pos.stopLossPriceYuan > 0 && pos.stopLossPriceYuan < stopCeilingYuan) ||
        isValidBreachedRatchetedStop
      )
    ) {
      violations.push(
        `${pos.symbol} 止损价 (¥${pos.stopLossPriceYuan}) 必须严格低于 max(建仓均价 ¥${pos.avgCostYuan}, 现价 ¥${pos.markPriceYuan})`
      );
    }
  }

  for (const ord of portfolio.recentOrders) {
    if (ord.side === "BUY" && ord.status === "FILLED") {
      if (!(ord.stopLossPriceYuan > 0 && ord.stopLossPriceYuan < ord.entryPriceYuan)) {
        violations.push(
          `买单 ${ord.orderId} 止损价 (¥${ord.stopLossPriceYuan}) 未严格低于买入价 (¥${ord.entryPriceYuan})`
        );
      }
    }
  }

  return {
    valid: violations.length === 0,
    violations,
  };
}

export function evaluateCstMarketSession(utcMs: number): MarketSessionState {
  const cstDate = new Date(utcMs + CST_OFFSET_MS);
  const day = cstDate.getUTCDay();
  if (day === 0 || day === 6) {
    return "CLOSED";
  }
  const secOfDay =
    cstDate.getUTCHours() * 3600 + cstDate.getUTCMinutes() * 60 + cstDate.getUTCSeconds();

  if (secOfDay >= 9 * 3600 && secOfDay < 9 * 3600 + 30 * 60) {
    return "PRE_OPEN";
  }
  if (
    (secOfDay >= 9 * 3600 + 30 * 60 && secOfDay <= 11 * 3600 + 30 * 60) ||
    (secOfDay >= 13 * 3600 && secOfDay <= 15 * 3600)
  ) {
    return "TRADING";
  }
  if (secOfDay > 11 * 3600 + 30 * 60 && secOfDay < 13 * 3600) {
    return "LUNCH_BREAK";
  }
  return "CLOSED";
}

function pad2(num: number): string {
  return String(Math.floor(Math.max(0, num))).padStart(2, "0");
}

export function computeNext30mRotationCountdown(utcMs: number): {
  nextSlotCstLabel: string;
  remainingSeconds: number;
  formattedCountdown: string;
  session: MarketSessionState;
} {
  const cstNow = new Date(utcMs + CST_OFFSET_MS);
  const session = evaluateCstMarketSession(utcMs);

  for (let dayOffset = 0; dayOffset <= 5; dayOffset += 1) {
    const dayStartCstMs = Date.UTC(
      cstNow.getUTCFullYear(),
      cstNow.getUTCMonth(),
      cstNow.getUTCDate() + dayOffset,
      0,
      0,
      0,
      0
    );
    const candidateDay = new Date(dayStartCstMs);
    const dow = candidateDay.getUTCDay();
    if (dow === 0 || dow === 6) {
      continue;
    }

    for (const slotMinutes of TRADING_ROTATION_SLOTS_MINUTES) {
      const slotCstMs = dayStartCstMs + slotMinutes * 60_000;
      const slotUtcMs = slotCstMs - CST_OFFSET_MS;
      if (slotUtcMs > utcMs) {
        const remainingSeconds = Math.max(1, Math.ceil((slotUtcMs - utcMs) / 1000));
        const slotDate = new Date(slotCstMs);
        const yyyy = slotDate.getUTCFullYear();
        const mm = pad2(slotDate.getUTCMonth() + 1);
        const dd = pad2(slotDate.getUTCDate());
        const hh = pad2(Math.floor(slotMinutes / 60));
        const min = pad2(slotMinutes % 60);

        const hours = Math.floor(remainingSeconds / 3600);
        const mins = Math.floor((remainingSeconds % 3600) / 60);
        const secs = remainingSeconds % 60;
        const formattedCountdown =
          hours > 0 ? `${pad2(hours)}:${pad2(mins)}:${pad2(secs)}` : `${pad2(mins)}:${pad2(secs)}`;

        return {
          nextSlotCstLabel: `${yyyy}-${mm}-${dd} ${hh}:${min}`,
          remainingSeconds,
          formattedCountdown,
          session,
        };
      }
    }
  }

  return {
    nextSlotCstLabel: "09:30",
    remainingSeconds: 1800,
    formattedCountdown: "30:00",
    session,
  };
}

export function clampBlurSlots(
  requestedIds: readonly string[],
  maxBudget: number = MAX_BLUR_BUDGET_HARD_CAP
): Set<string> {
  const cap = Math.min(MAX_BLUR_BUDGET_HARD_CAP, Math.max(0, Math.floor(maxBudget)));
  const active = new Set<string>();
  for (const id of requestedIds) {
    if (active.size >= cap) break;
    if (id.trim().length > 0) {
      active.add(id);
    }
  }
  return active;
}

export function clampPopoverPosition(params: {
  anchorX: number;
  anchorY: number;
  popoverWidth: number;
  popoverHeight: number;
  paneWidth: number;
  paneHeight: number;
  margin?: number;
}): { x: number; y: number } {
  const margin = params.margin ?? 12;
  const maxX = Math.max(margin, params.paneWidth - params.popoverWidth - margin);
  const maxY = Math.max(margin, params.paneHeight - params.popoverHeight - margin);
  const rawX = Number.isFinite(params.anchorX) ? params.anchorX : margin;
  const rawY = Number.isFinite(params.anchorY) ? params.anchorY : margin;

  return {
    x: Math.min(maxX, Math.max(margin, Math.round(rawX))),
    y: Math.min(maxY, Math.max(margin, Math.round(rawY))),
  };
}

export function resolveResponsiveLayoutMode(viewportWidthPx: number): ResponsiveLayoutContract {
  const width = Number.isFinite(viewportWidthPx) ? viewportWidthPx : 1280;
  if (width < 640) {
    return { shellMode: "mobile-tabs", kpiCols: 1, mainSplit: "stack", screenerMode: "cards" };
  }
  if (width < 680) {
    return { shellMode: "compact-sidebar", kpiCols: 2, mainSplit: "stack", screenerMode: "cards" };
  }
  if (width < 1180) {
    return { shellMode: "compact-sidebar", kpiCols: 2, mainSplit: "stack", screenerMode: "table" };
  }
  return { shellMode: "expanded-sidebar", kpiCols: 4, mainSplit: "12-col", screenerMode: "table" };
}

export function formatCurrencyYuan(amount: number, fractionDigits: number = 2): string {
  const safe = Number.isFinite(amount) ? amount : 0;
  const sign = safe < 0 ? "-" : "";
  const formatted = Math.abs(safe).toLocaleString("en-US", {
    minimumFractionDigits: fractionDigits,
    maximumFractionDigits: fractionDigits,
  });
  return `${sign}¥${formatted}`;
}

export function formatSignedPct(pct: number, fractionDigits: number = 2): string {
  const safe = Number.isFinite(pct) ? pct : 0;
  const rounded = Number(safe.toFixed(fractionDigits));
  const prefix = rounded > 0 ? "+" : "";
  return `${prefix}${rounded.toFixed(fractionDigits)}%`;
}

export function normalizeStockDisplayLabel(symbol: string, nameZh: string): string {
  const cleanName = nameZh.trim().replace(/\s+/g, " ");
  const cleanSymbol = symbol.trim().toUpperCase();
  return `${cleanName} · ${cleanSymbol}`;
}
