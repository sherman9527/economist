import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { LiveScreenerPage } from "../pages/LiveScreenerPage";
import { StrategyPluginEditorSection } from "../components/factors/StrategyPluginEditorSection";
import { bootstrapTerminal } from "../services/tauriClient";

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

describe("Round 5 Final Release Gate UI Regressions", () => {
  it("re-enables Add-to-Watchlist button in Screener when a previously watched stock is already settled", async () => {
    const boot = await bootstrapTerminal();
    const firstCand = boot.candidates[0];
    const settledWatchlist = {
      ...boot.watchlist,
      items: [
        {
          symbol: firstCand.symbol,
          nameZh: firstCand.nameZh,
          board: firstCand.board,
          industrySw1: firstCand.industrySw1,
          addedTradeDate: "2026-11-10",
          addedAtIso: "2026-11-10T14:30:00+08:00",
          entryPriceYuan: 20.0,
          stopLossPriceYuan: 18.6,
          targetPriceYuan: 23.0,
          latestPriceYuan: 23.2,
          peakPriceYuan: 23.4,
          troughPriceYuan: 19.8,
          convictionScore: 82.0,
          sourceDecision: "WATCH_OBSERVE",
          versionIdAtEntry: "v1.2.0",
          rationaleZh: "前轮已止盈结案",
          holdingTradingDays: 3,
          netReturnPct: 14.75,
          mfePct: 17.0,
          maePct: -1.0,
          outcome: "HIT_TARGET" as const,
          settledTradeDate: "2026-11-13",
          settledPriceYuan: 23.0,
        },
      ],
    };

    const onAdd = vi.fn().mockResolvedValue(undefined);
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);

    await act(async () => {
      root.render(
        <LiveScreenerPage
          funnel={boot.funnel}
          candidates={boot.candidates}
          portfolio={boot.portfolio}
          watchlistOverview={settledWatchlist}
          klinePayload={boot.featuredKline}
          inspectedSymbol={firstCand.symbol}
          theme="dark"
          polarity="cn-red-up"
          isBlurActive={() => false}
          onPopoverOpenChange={() => {}}
          onChangeKline={async () => {}}
          onAddWatchlist={onAdd}
          onPlaceBuy={async () => {}}
        />
      );
    });

    const addBtn = host.querySelector('[data-testid="add-watchlist-btn"]') as HTMLButtonElement | null;
    expect(addBtn).not.toBeNull();
    expect(addBtn!.disabled).toBe(false);

    await act(async () => {
      addBtn!.click();
    });
    expect(onAdd).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.unmount();
    });
    host.remove();
  });

  it("normalizes 6 equal factor weights to exact 100% without 102% rounding drift", async () => {
    const boot = await bootstrapTerminal();
    const equalCfg = {
      ...boot.pluginConfig,
      factors: boot.pluginConfig.factors.map((f) => ({ ...f, weightPct: 10 })),
    };

    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);

    await act(async () => {
      root.render(
        <StrategyPluginEditorSection
          pluginConfig={equalCfg}
          blurActive={false}
          onSavePluginConfig={async () => {}}
        />
      );
    });

    expect(host.textContent).toContain("合计 60%");
    const normBtn = Array.from(host.querySelectorAll("button")).find((b) =>
      b.textContent?.includes("归一100%")
    );
    expect(normBtn).toBeDefined();

    await act(async () => {
      normBtn!.click();
    });

    expect(host.textContent).toContain("合计 100%");
    expect(host.textContent).not.toContain("合计 102%");

    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
});
