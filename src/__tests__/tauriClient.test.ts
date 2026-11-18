import { describe, expect, it, beforeEach } from "vitest";
import {
  ackEconomistReport,
  bootstrapTerminal,
  closeSimulatedPosition,
  getEconomistReportHtml,
  getStockKline,
  isTauriRuntime,
  placeSimulatedBuy,
  resetFallbackStateForTests,
  runMarketScan,
  runStorageHygiene,
  updatePluginConfig,
  wakeAgentSession,
} from "../services/tauriClient";
import { validatePortfolioInvariants } from "../domain/uiInvariants";

describe("Tauri v2 IPC Client & Stateful Fallback Adapter", () => {
  beforeEach(() => {
    resetFallbackStateForTests();
  });

  it("bootstraps valid 100W portfolio, <200Yi candidates, and 5 gemini-4-carbon agent sessions", async () => {
    expect(isTauriRuntime()).toBe(false);
    const boot = await bootstrapTerminal();

    expect(boot.funnel.totalActiveCount).toBeGreaterThan(5000);
    expect(boot.portfolio.initialCapitalYuan).toBe(1_000_000);
    expect(validatePortfolioInvariants(boot.portfolio).valid).toBe(true);

    const buyCandidates = boot.candidates.filter((c) => c.decision === "BUY");
    expect(buyCandidates.length).toBeGreaterThanOrEqual(3);
    for (const cand of buyCandidates) {
      expect(cand.totalMvYi).toBeLessThan(200);
      expect(cand.turnoverT0Pct).toBeGreaterThan(5);
      expect(cand.turnoverT1Pct).toBeGreaterThan(5);
      expect(cand.pitfallsPassedCount).toBe(5);
    }

    expect(boot.telemetry.sessions).toHaveLength(5);
    for (const sess of boot.telemetry.sessions) {
      expect(sess.model).toBe("gemini-4-carbon");
      expect(sess.reasoningEffort).toBe("high");
      expect(sess.specRelPath.startsWith("./")).toBe(true);
    }
  });

  it("supports 1d and 30m K-line payloads with buy markers and stop-loss line", async () => {
    const daily = await getStockKline("300624.SZ", "1d");
    const m30 = await getStockKline("300624.SZ", "30m");

    expect(daily.period).toBe("1d");
    expect(daily.bars.length).toBeGreaterThanOrEqual(30);
    expect(daily.buyMarkers.length).toBeGreaterThanOrEqual(1);
    expect(daily.activeStopLossYuan).toBeGreaterThan(0);

    expect(m30.period).toBe("30m");
    expect(m30.bars.length).toBeGreaterThanOrEqual(24);
  });

  it("enforces T+1 lock on newly bought position and allows closing T+1 unlocked position", async () => {
    const initial = await bootstrapTerminal();
    const openCount0 = initial.portfolio.openSlotsCount;

    // Place simulated buy on a fresh candidate not yet in positions
    const updatedAfterBuy = await placeSimulatedBuy({
      symbol: "688228.SH",
      lotQty: 300,
      rationaleZh: "30m放量突破MA20，五维排雷全清",
      stopLossPriceYuan: 34.8,
    });

    expect(updatedAfterBuy.openSlotsCount).toBe(openCount0 + 1);
    expect(validatePortfolioInvariants(updatedAfterBuy).valid).toBe(true);

    const newlyOpened = updatedAfterBuy.positions.find((p) => p.symbol === "688228.SH");
    expect(newlyOpened).toBeDefined();
    expect(newlyOpened?.sellableQty).toBe(0);
    expect(newlyOpened?.lockedT0Qty).toBe(300);

    // Attempting to close T+0 locked position must reject
    await expect(
      closeSimulatedPosition("688228.SH", "测试T+1日内禁卖保护")
    ).rejects.toThrow(/T\+1/);

    // Closing an existing T+1 unlocked position succeeds
    const sellableTarget = updatedAfterBuy.positions.find((p) => p.sellableQty > 0);
    expect(sellableTarget).toBeDefined();
    const afterClose = await closeSimulatedPosition(
      sellableTarget!.symbol,
      "止盈轮动调仓落袋"
    );
    expect(afterClose.openSlotsCount).toBe(updatedAfterBuy.openSlotsCount - 1);
    expect(afterClose.recentOrders[0].side).toBe("SELL");
    expect(afterClose.recentOrders[0].stampDutyYuan).toBeGreaterThan(0);
  });

  it("supports scan, plugin hot-reload, Economist ACK + embed HTML, storage hygiene & session wake", async () => {
    const scanRes = await runMarketScan(true);
    expect(scanRes.funnel.qualifiedBuyCount).toBeGreaterThan(0);
    expect(scanRes.alert).not.toBeNull();

    const cfg = await updatePluginConfig({
      minIntradayProjectedTurnoverPct: 6.2,
      insiderSellVetoPct: 1.0,
      holderSurgeVetoPct: 18.0,
      mainOutflowVetoRatio: -0.08,
      atrStopMult: 2.1,
      marketPolarity: "intl-green-up",
      factorWeights: {
        turnover_ignition_2d: 25,
        vol_expansion_ratio: 20,
        smallcap_liq_adj: 15,
        main_flow_persistence: 15,
        chip_concentration_pit: 15,
        low_atr_breakout: 10,
      },
    });
    expect(cfg.minIntradayProjectedTurnoverPct).toBe(6.2);
    expect(cfg.marketPolarity).toBe("intl-green-up");
    expect(cfg.maxTotalMvYi).toBe(200);
    expect(cfg.maxPositions).toBe(10);

    const boot = await bootstrapTerminal();
    const unacked = boot.economist.reports.find((r) => !r.acked);
    expect(unacked).toBeDefined();
    const afterAck = await ackEconomistReport(unacked!.reportId, true);
    expect(afterAck.unackedCount).toBe(boot.economist.unackedCount - 1);

    const html = await getEconomistReportHtml(unacked!.relPath);
    expect(html).toContain("embed-liquid-reader");

    const hygiene = await runStorageHygiene();
    expect(hygiene.storage.zeroAbsolutePathVerified).toBe(true);
    expect(hygiene.storage.buildFootprintBytes).toBeLessThan(hygiene.storage.buildCapBytes);

    const woken = await wakeAgentSession("quant_researcher", "复核今日30m换手放量因子OOS衰减");
    expect(woken.agentRole).toBe("quant_researcher");
    expect(woken.turnCount).toBeGreaterThan(1);
  });
});
