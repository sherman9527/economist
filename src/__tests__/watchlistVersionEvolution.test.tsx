import { describe, expect, it, afterEach, beforeEach } from "vitest";
import fc from "fast-check";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { App } from "../App";
import { resetFallbackStateForTests } from "../services/tauriClient";
import {
  computeWilsonBoundsPct,
  normalizeBoundedWeightsPct,
} from "../domain/evolutionInvariants";

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

let activeRoot: Root | null = null;
let hostDiv: HTMLDivElement | null = null;

async function renderApp(): Promise<HTMLDivElement> {
  hostDiv = document.createElement("div");
  document.body.appendChild(hostDiv);
  activeRoot = createRoot(hostDiv);
  await act(async () => {
    activeRoot!.render(<App />);
  });
  await act(async () => {
    await Promise.resolve();
  });
  return hostDiv;
}

beforeEach(() => {
  resetFallbackStateForTests();
  document.documentElement.setAttribute("data-theme", "dark");
  document.documentElement.setAttribute("data-polarity", "cn-red-up");
});

afterEach(() => {
  if (activeRoot) {
    act(() => {
      activeRoot!.unmount();
    });
    activeRoot = null;
  }
  hostDiv?.remove();
  hostDiv = null;
});

describe("Watchlist Win-Rate Tracker, Multi-Version Shadow Arena & Anti-Overfit Walk-Forward UI", () => {
  it("adds a watched stock from Screener to Watchlist and renders T+1 net-of-25bps Win-Rate Scorecard", async () => {
    const dom = await renderApp();

    // Go to Live Screener
    const screenerNav = Array.from(
      dom.querySelectorAll<HTMLButtonElement>("aside button")
    ).find((b) => b.textContent?.includes("全A轮动与五维排雷"))!;
    await act(async () => {
      screenerNav.click();
    });

    // Click "加入自选观望" in Inspector Pane
    const watchBtn = dom.querySelector<HTMLButtonElement>("[data-testid='add-watchlist-btn']");
    expect(watchBtn).not.toBeNull();
    await act(async () => {
      watchBtn!.click();
    });

    expect(dom.textContent).toContain("已在自选胜率池");

    // Navigate to 100W Ledger page -> switch to Watchlist Win-Rate Pool
    const ledgerNav = Array.from(
      dom.querySelectorAll<HTMLButtonElement>("aside button")
    ).find((b) => b.textContent?.includes("100W模拟盘订单账本"))!;
    await act(async () => {
      ledgerNav.click();
    });

    const watchlistTabBtn = dom.querySelector<HTMLButtonElement>(
      "[data-testid='ledger-tab-watchlist']"
    );
    expect(watchlistTabBtn).not.toBeNull();
    await act(async () => {
      watchlistTabBtn!.click();
    });

    expect(dom.textContent).toContain("观望自选样本外胜率检验台");
    expect(dom.textContent).toContain("Wilson 95%");
    expect(dom.textContent).toContain("25bps");
    expect(dom.textContent).toContain("盈亏比");

    // Blur budget still <= 8
    expect(document.querySelectorAll('[data-blur-slot="active"]').length).toBeLessThanOrEqual(8);
  });

  it("compares strategy versions, diagnoses degradation root causes when target < base, switches versions, and runs walk-forward iteration", async () => {
    const dom = await renderApp();

    const factorNav = Array.from(
      dom.querySelectorAll<HTMLButtonElement>("aside button")
    ).find((b) => b.textContent?.includes("量化因子与参数插件"))!;
    await act(async () => {
      factorNav.click();
    });

    // Multi-version shadow tracking table must display legacy & current versions simultaneously
    expect(dom.textContent).toContain("策略与因子多版本影子竞技场");
    expect(dom.textContent).toContain("v1.0.0");
    expect(dom.textContent).toContain("v1.1.0");
    expect(dom.textContent).toContain("v1.2.0");

    // Select degraded experimental version v1.3.0-exp as target against v1.2.0 base to verify root-cause attribution ("找原因")
    const targetSelect = dom.querySelector<HTMLSelectElement>(
      "[data-testid='version-compare-target-select']"
    );
    expect(targetSelect).not.toBeNull();
    await act(async () => {
      targetSelect!.value = "v1.3.0-exp";
      targetSelect!.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () => {
      await Promise.resolve();
    });

    expect(dom.textContent).toContain("版本退化四维归因诊断");
    expect(dom.textContent).toContain("FACTOR_WEIGHT_DRAG");

    // Switch active version to v1.1.0 with one click
    const switchBtn = dom.querySelector<HTMLButtonElement>(
      "[data-testid='switch-version-v1.1.0']"
    );
    expect(switchBtn).not.toBeNull();
    await act(async () => {
      switchBtn!.click();
    });
    expect(dom.textContent).toContain("当前生效 v1.1.0");

    // Run anti-overfit purged walk-forward iteration
    const iterateBtn = dom.querySelector<HTMLButtonElement>(
      "[data-testid='run-anti-overfit-btn']"
    );
    expect(iterateBtn).not.toBeNull();
    await act(async () => {
      iterateBtn!.click();
    });
    expect(dom.textContent).toContain("5日隔离带滚动样本外验证");
    expect(document.querySelectorAll('[data-blur-slot="active"]').length).toBeLessThanOrEqual(8);
  });

  it("PBT: Wilson 95% CI brackets empirical win rate and bounded factor weights sum to 100%", () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 200 }),
        fc.integer({ min: 1, max: 200 }),
        (rawWins, total) => {
          const wins = Math.min(rawWins, total);
          const empPct = (wins / total) * 100;
          const [lo, hi] = computeWilsonBoundsPct(wins, total);
          expect(lo).toBeGreaterThanOrEqual(0);
          expect(hi).toBeLessThanOrEqual(100);
          expect(lo).toBeLessThanOrEqual(empPct + 0.05);
          expect(hi).toBeGreaterThanOrEqual(empPct - 0.05);
        }
      ),
      { numRuns: 100 }
    );

    fc.assert(
      fc.property(
        fc.array(fc.double({ min: 0, max: 200, noNaN: true }), {
          minLength: 6,
          maxLength: 6,
        }),
        (arr) => {
          const norm = normalizeBoundedWeightsPct(arr, 5, 35);
          const sum = norm.reduce((a, b) => a + b, 0);
          expect(Math.abs(sum - 100)).toBeLessThan(0.05);
          for (const w of norm) {
            expect(w).toBeGreaterThanOrEqual(4.99);
            expect(w).toBeLessThanOrEqual(35.01);
          }
        }
      ),
      { numRuns: 100 }
    );
  });
});
