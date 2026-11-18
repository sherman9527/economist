import { describe, expect, it, afterEach, beforeEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { App } from "../App";
import { resetFallbackStateForTests } from "../services/tauriClient";

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

describe("Apple Liquid Glass 6-Workspace Shell, <=8 Blur Budget & Engineering Redlines", () => {
  it("keeps active backdrop-filter slots <= 8 across all 6 workspace pages and popovers", async () => {
    const dom = await renderApp();

    const checkBlurCap = () => {
      const activeBlurNodes = document.querySelectorAll('[data-blur-slot="active"]');
      expect(activeBlurNodes.length).toBeGreaterThanOrEqual(2);
      expect(activeBlurNodes.length).toBeLessThanOrEqual(8);
    };

    checkBlurCap();

    // Open K-line rationale popover on Dashboard
    const buyAnchor = dom.querySelector("[data-kline-buy-anchor]") as HTMLButtonElement | null;
    expect(buyAnchor).not.toBeNull();
    await act(async () => {
      buyAnchor!.click();
    });
    const rationaleDialog = dom.querySelector("[data-kline-rationale-popover]");
    expect(rationaleDialog).not.toBeNull();
    expect(rationaleDialog?.textContent).toContain("gemini-4-carbon");
    checkBlurCap();

    // Trigger 30m rotation scan toast & verify blur budget still <= 8
    const buttons = Array.from(dom.querySelectorAll("button"));
    const scanBtn = buttons.find((b) => b.textContent?.includes("立即全A扫描"));
    expect(scanBtn).toBeDefined();
    await act(async () => {
      scanBtn!.click();
    });
    expect(document.body.textContent).toContain("30m 轮动雷达命中");
    checkBlurCap();

    // Navigate through all 6 pages via sidebar buttons and verify each page mounts within <= 8 blur slots
    const navLabels = [
      "全A轮动与五维排雷",
      "100W模拟盘订单账本",
      "量化因子与参数插件",
      "《经济学人》政经研报",
      "Telemetry与Agent会话",
      "组合与策略总览",
    ];

    for (const label of navLabels) {
      const navBtn = Array.from(
        dom.querySelectorAll<HTMLButtonElement>("aside button")
      ).find((b) => b.textContent?.includes(label));
      expect(navBtn).toBeDefined();
      await act(async () => {
        navBtn!.click();
      });
      checkBlurCap();
    }
  });

  it("toggles dark/light theme, red/green polarity, and jumps from Economist ticker to live K-line", async () => {
    const dom = await renderApp();

    const polarityBtn = Array.from(
      dom.querySelectorAll<HTMLButtonElement>("header button")
    ).find((b) => b.textContent?.includes("红涨绿跌"));
    expect(polarityBtn).toBeDefined();
    await act(async () => {
      polarityBtn!.click();
    });
    expect(document.documentElement.getAttribute("data-polarity")).toBe("intl-green-up");

    const themeBtn = Array.from(
      dom.querySelectorAll<HTMLButtonElement>("header button")
    ).find((b) => b.textContent?.includes("深色"));
    expect(themeBtn).toBeDefined();
    await act(async () => {
      themeBtn!.click();
    });
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");

    // Navigate to Economist page and click related A-share ticker 836239.BJ to jump to live K-line
    const econNav = Array.from(
      dom.querySelectorAll<HTMLButtonElement>("aside button")
    ).find((b) => b.textContent?.includes("《经济学人》政经研报"))!;
    await act(async () => {
      econNav.click();
    });

    const tickerJumpBtn = Array.from(
      dom.querySelectorAll<HTMLButtonElement>("button")
    ).find((b) => b.textContent?.trim() === "836239.BJ");
    expect(tickerJumpBtn).toBeDefined();
    await act(async () => {
      tickerJumpBtn!.click();
    });
    expect(dom.textContent).toContain("长虹能源 · 836239.BJ");
  });

  it("enforces < 500 lines per file and 0 hardcoded absolute paths across all frontend files", () => {
    const rawSources = import.meta.glob<string>(
      [
        "../**/*.{ts,tsx,css}",
        "../../package.json",
        "../../tsconfig.json",
        "../../vite.config.ts",
        "../../index.html",
      ],
      { query: "?raw", import: "default", eager: true }
    );

    const entries = Object.entries(rawSources);
    expect(entries.length).toBeGreaterThanOrEqual(30);

    const unixTmp = ["", "tmp", ""].join("/");
    const unixVarTmp = ["", "var", "tmp"].join("/");
    const driveOrTmpPattern = new RegExp(
      ["\\b[A-Za-z]:\\\\", "\\b[A-Za-z]:/(?!/)", unixTmp, unixVarTmp].join("|")
    );

    for (const [relFile, content] of entries) {
      const lines = content.split(/\r?\n/).length;
      expect(lines, `${relFile} exceeded 500 lines (${lines})`).toBeLessThan(500);
      expect(
        driveOrTmpPattern.test(content),
        `${relFile} contains a forbidden absolute or tmp path`
      ).toBe(false);
    }
  });
});
