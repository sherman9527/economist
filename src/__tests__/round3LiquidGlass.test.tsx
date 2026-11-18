import React, { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createRoot, type Root } from "react-dom/client";
import {
  computeAShareFeeBreakdown,
  validateAShareOrderInput,
} from "../domain/uiInvariants";
import { createInitialBootstrapState } from "../services/mockTerminalFixtures";
import { FactorPluginLabPage } from "../pages/FactorPluginLabPage";
import { LiveScreenerPage } from "../pages/LiveScreenerPage";
import { EconomistNewsPage } from "../pages/EconomistNewsPage";
import { TelemetrySessionsPage } from "../pages/TelemetrySessionsPage";
import { RotationAlertToast } from "../components/shell/RotationAlertToast";
import { normalizeKlineSeriesAndMarkers } from "../components/charts/klineTimeAlign";

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Array<{ root: Root; host: HTMLDivElement }> = [];

async function mountNode(el: React.ReactElement): Promise<{
  host: HTMLDivElement;
  rerender: (next: React.ReactElement) => Promise<void>;
}> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  mountedRoots.push({ root, host });
  await act(async () => {
    root.render(el);
  });
  await act(async () => {
    await Promise.resolve();
  });
  return {
    host,
    rerender: async (next: React.ReactElement) => {
      await act(async () => {
        root.render(next);
      });
    },
  };
}

afterEach(() => {
  while (mountedRoots.length > 0) {
    const item = mountedRoots.pop()!;
    act(() => {
      item.root.unmount();
    });
    item.host.remove();
  }
});

describe("Round 3 Deep UI/UX & Liquid Glass Regressions", () => {
  it("[C-01] aligns YYYY-MM-DD buy markers onto 30m numeric-timestamp bars and handles HH:mm:ss safely", () => {
    const base = createInitialBootstrapState();
    const aligned = normalizeKlineSeriesAndMarkers({
      ...base.featuredKline,
      period: "30m",
      bars: [
        {
          time: "2026-11-18 10:30:00",
          open: 60.2,
          high: 61.5,
          low: 59.8,
          close: 61.1,
          volume: 15200,
          turnoverPct: 2.1,
          ma5: 60.5,
          ma20: 59.4,
        },
        {
          time: "2026-11-18 10:00",
          open: 59.5,
          high: 60.4,
          low: 59.2,
          close: 60.2,
          volume: 18400,
          turnoverPct: 2.4,
          ma5: 60.1,
          ma20: 59.2,
        },
      ],
      buyMarkers: [
        {
          recId: "rec-30m-date-only",
          symbol: "300624.SZ",
          barTime: "2026-11-18",
          entryPriceYuan: 60.2,
          stopLossPriceYuan: 55.4,
          targetPriceYuan: 68.0,
          convictionScore: 88.5,
          rationaleZh: "30m换手放量共振买点",
          agentSessionId: "88ace910-c427-41db-a87f-46d8ae39735b",
          pitfalls: [],
          createdAtIso: "2026-11-18T02:30:00Z",
        },
      ],
    });

    expect(aligned.bars).toHaveLength(2);
    expect(typeof aligned.bars[0].chartTime).toBe("number");
    expect(Number.isFinite(aligned.bars[0].chartTime as number)).toBe(true);
    expect((aligned.bars[0].chartTime as number) < (aligned.bars[1].chartTime as number)).toBe(true);

    expect(aligned.markers).toHaveLength(1);
    expect(typeof aligned.markers[0].chartTime).toBe("number");
    expect(aligned.markers[0].chartTime).toBe(aligned.bars[0].chartTime);
  });

  it("[C-02] allows sandboxed script execution (without same-origin) in Economist reader and prevents mis-bound buy orders on non-pool linked symbols", async () => {
    const base = createInitialBootstrapState();
    const { host: econHost } = await mountNode(
      <EconomistNewsPage
        economist={base.economist}
        isBlurActive={() => true}
        onAckReport={vi.fn(async () => {})}
        onBatchAckUnread={vi.fn(async () => {})}
        onLoadReportHtml={vi.fn(async () => "<!doctype html><html data-embed='1'><body>Chart</body></html>")}
        onJumpToKline={vi.fn()}
      />
    );

    const readerTab = Array.from(econHost.querySelectorAll('[role="tab"]')).find((b) =>
      b.textContent?.includes("原版深度研报阅读器")
    ) as HTMLButtonElement;
    await act(async () => {
      readerTab.click();
    });

    const iframe = econHost.querySelector("iframe")!;
    expect(iframe.getAttribute("sandbox")).toBe("allow-scripts");
    expect(iframe.getAttribute("sandbox")).not.toContain("allow-same-origin");

    const nonPoolKline = {
      ...base.featuredKline,
      symbol: "300308.SZ",
      nameZh: "中际旭创",
    };
    const { host: screenerHost } = await mountNode(
      <LiveScreenerPage
        funnel={base.funnel}
        candidates={base.candidates}
        portfolio={base.portfolio}
        klinePayload={nonPoolKline}
        inspectedSymbol="300308.SZ"
        theme="dark"
        polarity="cn-red-up"
        isBlurActive={() => true}
        onChangeKline={vi.fn()}
        onPlaceBuy={vi.fn(async () => {})}
      />
    );

    expect(screenerHost.textContent).toContain("300308.SZ");
    expect(screenerHost.textContent).toContain("非当期<200亿选股候选");
  });

  it("[C-03] uses canonical F1..F6 factor IDs in bootstrap fixtures and resyncs FactorPluginLabPage sliders when props update", async () => {
    const base = createInitialBootstrapState();
    expect(base.pluginConfig.factors.map((f) => f.factorId)).toEqual([
      "F1_ideal_reversal",
      "F2_concave_abn_turnover",
      "F3_amihud_compression",
      "F4_intraday_apm_vwap",
      "F5_main_inflow_ir",
      "F6_pv_efficiency",
    ]);

    const { host, rerender } = await mountNode(
      <FactorPluginLabPage
        pluginConfig={base.pluginConfig}
        theme="dark"
        polarity="cn-red-up"
        isBlurActive={() => true}
        onSavePluginConfig={vi.fn(async () => {})}
      />
    );

    const updatedCfg = {
      ...base.pluginConfig,
      factors: base.pluginConfig.factors.map((f, idx) => ({
        ...f,
        weightPct: idx === 0 ? 40 : 12,
      })),
    };
    await rerender(
      <FactorPluginLabPage
        pluginConfig={updatedCfg}
        theme="dark"
        polarity="cn-red-up"
        isBlurActive={() => true}
        onSavePluginConfig={vi.fn(async () => {})}
      />
    );

    const f1Label = Array.from(host.querySelectorAll("label")).find((l) =>
      l.textContent?.includes("高换手理想反转因子")
    );
    const f1Input = f1Label?.querySelector("input") as HTMLInputElement | undefined;
    expect(Number(f1Input?.value)).toBe(40);
  });

  it("[H-02] enforces 15 bps slippage, allows odd-lot full SELL liquidation, and binds STAR/BSE 1-share lot steps in LiveScreenerPage", async () => {
    const buyFees = computeAShareFeeBreakdown("BUY", 20, 500);
    expect(buyFees.slippageYuan).toBe(15);

    const oddSell = validateAShareOrderInput({
      side: "SELL",
      board: "STAR",
      prevCloseYuan: 30,
      orderPriceYuan: 30,
      lotQty: 137,
      availableQty: 137,
    });
    expect(oddSell.valid).toBe(true);

    const base = createInitialBootstrapState();
    const starCand = base.candidates.find((c) => c.board === "STAR")!;
    const { host } = await mountNode(
      <LiveScreenerPage
        funnel={base.funnel}
        candidates={[starCand]}
        portfolio={base.portfolio}
        klinePayload={{ ...base.featuredKline, symbol: starCand.symbol, nameZh: starCand.nameZh }}
        theme="dark"
        polarity="cn-red-up"
        isBlurActive={() => true}
        onChangeKline={vi.fn()}
        onPlaceBuy={vi.fn(async () => {})}
      />
    );

    const qtyLabel = Array.from(host.querySelectorAll("label")).find((l) =>
      l.textContent?.includes("委托股数")
    );
    const qtyInput = qtyLabel?.querySelector("input") as HTMLInputElement;
    expect(Number(qtyInput.min)).toBe(200);
    expect(Number(qtyInput.step)).toBe(1);
  });

  it("[H-05 & M-04] renders multi-pick selector & Escape dismissal in RotationAlertToast and includes UI filter in TelemetrySessionsPage", async () => {
    const base = createInitialBootstrapState();
    const onDismiss = vi.fn();
    const { host: toastHost } = await mountNode(
      <RotationAlertToast
        alert={{
          slotTimestampCst: "2026-11-18 10:30",
          qualifiedCount: 2,
          topPicks: base.candidates.slice(0, 2),
        }}
        blurActive={true}
        onDismiss={onDismiss}
        onInspectKline={vi.fn()}
        onSimulatedBuy={vi.fn(async () => {})}
      />
    );

    const secondPickBtn = Array.from(toastHost.querySelectorAll("button")).find((b) =>
      b.textContent?.includes(base.candidates[1].nameZh)
    ) as HTMLButtonElement | undefined;
    expect(secondPickBtn).toBeDefined();
    await act(async () => {
      secondPickBtn!.click();
    });
    expect(toastHost.textContent).toContain(base.candidates[1].symbol);

    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(onDismiss).toHaveBeenCalledTimes(1);

    const { host: teleHost } = await mountNode(
      <TelemetrySessionsPage
        telemetry={base.telemetry}
        isBlurActive={() => true}
        onRunHygiene={vi.fn(async () => {})}
        onWakeSession={vi.fn(async () => {})}
      />
    );
    const uiBtn = Array.from(teleHost.querySelectorAll("button")).find(
      (b) => b.textContent?.trim() === "UI"
    );
    expect(uiBtn).toBeDefined();
  });
});
