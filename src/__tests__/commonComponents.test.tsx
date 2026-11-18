import { describe, expect, it, vi, afterEach } from "vitest";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { SafeText } from "../components/common/SafeText";
import { AsyncGlassButton } from "../components/common/AsyncGlassButton";
import { PitfallAuditBadges } from "../components/common/PitfallAuditBadges";
import type { PitfallDimensionStatus } from "../types/terminal";

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

let mountedRoots: { root: Root; container: HTMLDivElement }[] = [];

function mountNode(ui: React.ReactElement): HTMLDivElement {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  act(() => {
    root.render(ui);
  });
  mountedRoots.push({ root, container });
  return container;
}

afterEach(() => {
  for (const item of mountedRoots) {
    act(() => {
      item.root.unmount();
    });
    item.container.remove();
  }
  mountedRoots = [];
});

describe("Zero-Clipping SafeText, AsyncGlassButton & Decoupled Pitfall Badges", () => {
  it("renders SafeText with min-width:0, full-text title fallback, and hover popover for long CJK/Latin strings", () => {
    const longLabel =
      "《经济学人》跨市场深度传导：*ST景峰-DR (000908.SZ) 极端中英混排防穿模验证字符串";
    const el = mountNode(<SafeText text={longLabel} maxLines={1} showPopover />);
    const target = el.querySelector("[data-safe-text]") as HTMLElement | null;
    expect(target).not.toBeNull();
    expect(target?.getAttribute("title")).toBe(longLabel);
    expect(target?.textContent).toBe(longLabel);

    act(() => {
      target?.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, clientX: 120, clientY: 80 }));
    });
    const pop = document.body.querySelector("[data-safe-text-popover]");
    expect(pop).not.toBeNull();
    expect(pop?.textContent).toContain("*ST景峰-DR");
  });

  it("transitions AsyncGlassButton smoothly through idle -> loading -> success", async () => {
    let resolveTask: () => void = () => {};
    const handler = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveTask = resolve;
        })
    );

    const el = mountNode(
      <AsyncGlassButton label="立即全A扫描" loadingLabel="扫描中..." successLabel="扫描完成" onClick={handler} />
    );
    const btn = el.querySelector("button")!;
    expect(btn.textContent).toContain("立即全A扫描");

    await act(async () => {
      btn.click();
    });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(btn.getAttribute("data-async-state")).toBe("loading");

    await act(async () => {
      resolveTask();
    });
    expect(btn.getAttribute("data-async-state")).toBe("success");
  });

  it("renders 5 pitfall badges with decoupled VETO (#FF2D55), WARN (#FF9F0A), and CLEAR semantics", () => {
    const pitfalls: PitfallDimensionStatus[] = [
      { dimension: "INSIDER_REDUCTION", labelZh: "减持", passed: false, severity: "VETO", detailZh: "预减持2%" },
      { dimension: "NEGATIVE_NEWS", labelZh: "舆情", passed: true, severity: "CLEAR", detailZh: "正常" },
      { dimension: "HOLDER_SURGE", labelZh: "股东数", passed: false, severity: "WARN", detailZh: "+11%" },
      { dimension: "MAIN_OUTFLOW", labelZh: "主力流", passed: true, severity: "CLEAR", detailZh: "净流入" },
      { dimension: "TECH_BREAKDOWN", labelZh: "破位", passed: true, severity: "CLEAR", detailZh: "站上MA20" },
    ];
    const el = mountNode(<PitfallAuditBadges pitfalls={pitfalls} />);
    const badges = el.querySelectorAll("[data-pitfall-severity]");
    expect(badges).toHaveLength(5);
    expect(badges[0].getAttribute("data-pitfall-severity")).toBe("VETO");
    expect(badges[2].getAttribute("data-pitfall-severity")).toBe("WARN");
  });
});
