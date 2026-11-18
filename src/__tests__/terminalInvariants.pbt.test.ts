import { describe, expect, it } from "vitest";
import fc from "fast-check";
import type { BoardKind, PortfolioOverviewDto, PositionSlotDto, SimulatedOrderDto } from "../types/terminal";
import {
  clampBlurSlots,
  clampPopoverPosition,
  computeAShareFeeBreakdown,
  computeNext30mRotationCountdown,
  evaluateCstMarketSession,
  formatCurrencyYuan,
  formatSignedPct,
  getBoardTradingRule,
  normalizeStockDisplayLabel,
  resolveResponsiveLayoutMode,
  validateAShareOrderInput,
  validatePortfolioInvariants,
} from "../domain/uiInvariants";

describe("1. A-Share Board Price-Limit & Lot-Size Invariants (PBT)", () => {
  const boardArb = fc.constantFrom<BoardKind>("MAIN", "CHINEXT", "STAR", "BSE");

  it("enforces exact limit ratios and lot rules across MAIN, CHINEXT, STAR, and BSE", () => {
    expect(getBoardTradingRule("MAIN")).toEqual({ limitRatio: 0.1, minLotQty: 100, lotStep: 100 });
    expect(getBoardTradingRule("CHINEXT")).toEqual({ limitRatio: 0.2, minLotQty: 100, lotStep: 100 });
    expect(getBoardTradingRule("STAR")).toEqual({ limitRatio: 0.2, minLotQty: 200, lotStep: 1 });
    expect(getBoardTradingRule("BSE")).toEqual({ limitRatio: 0.3, minLotQty: 100, lotStep: 100 });
  });

  it("accepts in-band valid-lot orders and rejects out-of-band or invalid-lot orders", () => {
    fc.assert(
      fc.property(
        boardArb,
        fc.integer({ min: 200, max: 50000 }).map((cents) => cents / 100),
        fc.integer({ min: 1, max: 50 }),
        (board, prevCloseYuan, mult) => {
          const rule = getBoardTradingRule(board);
          const validQty = board === "STAR" ? 200 + mult : mult * 100;
          const okRes = validateAShareOrderInput({
            board,
            prevCloseYuan,
            orderPriceYuan: prevCloseYuan,
            lotQty: validQty,
          });
          expect(okRes.valid).toBe(true);
          expect(okRes.lowerLimitYuan).toBeLessThanOrEqual(prevCloseYuan);
          expect(okRes.upperLimitYuan).toBeGreaterThanOrEqual(prevCloseYuan);

          const overCeiling = validateAShareOrderInput({
            board,
            prevCloseYuan,
            orderPriceYuan: Number((okRes.upperLimitYuan + 0.02).toFixed(2)),
            lotQty: validQty,
          });
          expect(overCeiling.valid).toBe(false);

          const underMinLot = validateAShareOrderInput({
            board,
            prevCloseYuan,
            orderPriceYuan: prevCloseYuan,
            lotQty: rule.minLotQty - 1,
          });
          expect(underMinLot.valid).toBe(false);
        }
      ),
      { numRuns: 120 }
    );
  });
});

describe("2. 100W Portfolio, T+1 Lock, Stop-Loss < Entry & Fee Breakdown Invariants (PBT)", () => {
  it("computes exact A-share commission (min ¥5), sell-only stamp duty, and slippage", () => {
    fc.assert(
      fc.property(
        fc.constantFrom<"BUY" | "SELL">("BUY", "SELL"),
        fc.integer({ min: 100, max: 20000 }).map((c) => c / 100),
        fc.integer({ min: 1, max: 100 }).map((lots) => lots * 100),
        (side, price, qty) => {
          const fees = computeAShareFeeBreakdown(side, price, qty);
          expect(fees.commissionYuan).toBeGreaterThanOrEqual(5);
          if (side === "BUY") {
            expect(fees.stampDutyYuan).toBe(0);
          } else {
            expect(fees.stampDutyYuan).toBeGreaterThan(0);
          }
          expect(fees.slippageYuan).toBeGreaterThan(0);
          expect(fees.totalFeeYuan).toBeCloseTo(
            fees.commissionYuan + fees.stampDutyYuan + fees.slippageYuan,
            2
          );
        }
      ),
      { numRuns: 100 }
    );
  });

  it("validates <=10 slots, T+1 totalQty === sellableQty + lockedT0Qty, and stopLoss < entry", () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 14 }),
        fc.boolean(),
        fc.boolean(),
        (slotCount, breakTPlus1, breakStopLoss) => {
          const positions: PositionSlotDto[] = Array.from({ length: slotCount }, (_, idx) => {
            const sellableQty = 200;
            const lockedT0Qty = 100;
            const avgCostYuan = 20;
            return {
              positionId: `pos-${idx + 1}`,
              slotNo: idx + 1,
              symbol: `30000${idx}.SZ`,
              nameZh: `测试标的${idx}`,
              board: "CHINEXT",
              totalQty: breakTPlus1 && idx === 0 ? 500 : sellableQty + lockedT0Qty,
              sellableQty,
              lockedT0Qty,
              avgCostYuan,
              markPriceYuan: 20.8,
              stopLossPriceYuan: breakStopLoss && idx === 0 ? 20.5 : 18.4,
              stopDistancePct: 11.5,
              unrealizedPnlYuan: 240,
              unrealizedPnlPct: 4.0,
              entryRationaleZh: "连续两日换手大于5%放量突破且五维排雷全清",
              openedAtIso: "2026-11-18T02:00:00Z",
            };
          });

          const orders: SimulatedOrderDto[] = [
            {
              orderId: "ord-1",
              recId: "rec-1",
              symbol: "300001.SZ",
              nameZh: "测试标的1",
              side: "BUY",
              triggerType: "QUANT_SIGNAL",
              status: "FILLED",
              lotQty: 300,
              entryPriceYuan: 20,
              fillPriceYuan: 20.01,
              stopLossPriceYuan: breakStopLoss && slotCount === 0 ? 20.0 : 18.4,
              commissionYuan: 5,
              stampDutyYuan: 0,
              slippageYuan: 3,
              rationaleZh: "量化买点触发",
              tradeDate: "2026-11-18",
              tPlus1UnlockDate: "2026-11-19",
              createdAtIso: "2026-11-18T02:00:00Z",
            },
          ];

          const overview: PortfolioOverviewDto = {
            initialCapitalYuan: 1_000_000,
            cashYuan: 800_000,
            marketValueYuan: 200_000,
            totalEquityYuan: 1_000_000,
            cumReturnPct: 0,
            dailyPnlYuan: 0,
            dailyReturnPct: 0,
            maxDrawdownPct: -2.1,
            openSlotsCount: positions.length,
            maxSlots: 10,
            totalFeesPaidYuan: 8,
            positions,
            recentOrders: orders,
            equityCurve: [],
            sectorWeights: [],
          };

          const check = validatePortfolioInvariants(overview);
          const shouldFail =
            slotCount > 10 || (slotCount > 0 && breakTPlus1) || breakStopLoss;
          expect(check.valid).toBe(!shouldFail);
        }
      ),
      { numRuns: 100 }
    );
  });
});

describe("3. CST Trading Session & Next 30-Minute Rotation Countdown Invariants (PBT)", () => {
  it("identifies PRE_OPEN, TRADING, LUNCH_BREAK, and CLOSED accurately in UTC+8 CST", () => {
    // 2026-11-18 is Wednesday. 10:15 CST == 02:15 UTC
    expect(evaluateCstMarketSession(Date.UTC(2026, 10, 18, 1, 15, 0))).toBe("PRE_OPEN");
    expect(evaluateCstMarketSession(Date.UTC(2026, 10, 18, 2, 15, 0))).toBe("TRADING");
    expect(evaluateCstMarketSession(Date.UTC(2026, 10, 18, 4, 10, 0))).toBe("LUNCH_BREAK");
    expect(evaluateCstMarketSession(Date.UTC(2026, 10, 18, 6, 20, 0))).toBe("TRADING");
    expect(evaluateCstMarketSession(Date.UTC(2026, 10, 18, 8, 5, 0))).toBe("CLOSED");
    // Saturday 2026-11-21 10:15 CST == 02:15 UTC
    expect(evaluateCstMarketSession(Date.UTC(2026, 10, 21, 2, 15, 0))).toBe("CLOSED");
  });

  it("always schedules the next 30m rotation on a valid Mon-Fri CST trading half-hour", () => {
    const validHalfHours = new Set([
      "09:30",
      "10:00",
      "10:30",
      "11:00",
      "11:30",
      "13:00",
      "13:30",
      "14:00",
      "14:30",
      "15:00",
    ]);

    fc.assert(
      fc.property(
        fc.integer({
          min: Date.UTC(2026, 10, 16, 0, 0, 0),
          max: Date.UTC(2026, 10, 30, 23, 59, 59),
        }),
        (utcMs) => {
          const res = computeNext30mRotationCountdown(utcMs);
          expect(res.remainingSeconds).toBeGreaterThan(0);
          expect(res.remainingSeconds).toBeLessThanOrEqual(3 * 86400);
          const hhmm = res.nextSlotCstLabel.slice(-5);
          expect(validHalfHours.has(hhmm)).toBe(true);
          expect(res.formattedCountdown).toMatch(/^\d{2}:\d{2}(:\d{2})?$/);
        }
      ),
      { numRuns: 150 }
    );
  });
});

describe("4. Liquid Glass Blur Budget <= 8, Edge-Clamped Popovers & Extremal CJK Formatting (PBT)", () => {
  it("never activates more than 8 backdrop-filter slots regardless of requested slot count", () => {
    fc.assert(
      fc.property(
        fc.array(fc.string({ minLength: 1, maxLength: 18 }), { minLength: 0, maxLength: 40 }),
        fc.integer({ min: 1, max: 20 }),
        (ids, requestedMax) => {
          const active = clampBlurSlots(ids, requestedMax);
          expect(active.size).toBeLessThanOrEqual(8);
          expect(active.size).toBeLessThanOrEqual(requestedMax);
        }
      ),
      { numRuns: 120 }
    );
  });

  it("clamps popover coordinates inside [12, paneSize - popoverSize - 12] across 390px..2560px", () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 390, max: 2560 }),
        fc.integer({ min: 420, max: 1440 }),
        fc.integer({ min: 180, max: 340 }),
        fc.integer({ min: 140, max: 320 }),
        fc.integer({ min: -300, max: 3000 }),
        fc.integer({ min: -300, max: 2000 }),
        (paneWidth, paneHeight, popoverWidth, popoverHeight, anchorX, anchorY) => {
          const pos = clampPopoverPosition({
            anchorX,
            anchorY,
            popoverWidth,
            popoverHeight,
            paneWidth,
            paneHeight,
            margin: 12,
          });
          expect(pos.x).toBeGreaterThanOrEqual(12);
          expect(pos.x).toBeLessThanOrEqual(Math.max(12, paneWidth - popoverWidth - 12));
          expect(pos.y).toBeGreaterThanOrEqual(12);
          expect(pos.y).toBeLessThanOrEqual(Math.max(12, paneHeight - popoverHeight - 12));
        }
      ),
      { numRuns: 150 }
    );
  });

  it("formats extremal CJK/Latin strings, ¥1,000,000.00, and -29.98% cleanly across breakpoints", () => {
    expect(formatCurrencyYuan(1_000_000)).toBe("¥1,000,000.00");
    expect(formatSignedPct(-29.98)).toBe("-29.98%");
    expect(formatSignedPct(19.994)).toBe("+19.99%");
    expect(normalizeStockDisplayLabel("000908.SZ", " *ST景峰-DR ")).toBe("*ST景峰-DR · 000908.SZ");

    expect(resolveResponsiveLayoutMode(390).screenerMode).toBe("cards");
    expect(resolveResponsiveLayoutMode(1280).mainSplit).toBe("12-col");
    expect(resolveResponsiveLayoutMode(2560).kpiCols).toBe(4);
  });
});
