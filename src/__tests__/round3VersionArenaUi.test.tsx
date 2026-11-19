import React, { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createRoot, type Root } from "react-dom/client";
import { VersionControlArenaPanel } from "../components/factors/VersionControlArenaPanel";
import { AntiOverfitWalkForwardPanel } from "../components/factors/AntiOverfitWalkForwardPanel";
import type { EvolutionAuditLogDto, VersionControlDashboardDto } from "../types/terminal";

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

const buildMockDashboard = (
  baseVersionId: string,
  targetVersionId: string
): VersionControlDashboardDto => ({
  activeVersionId: targetVersionId,
  versions: [
    {
      versionId: "v1.0.0",
      parentVersionId: null,
      sourceTag: "GENESIS",
      createdAt: "2026-11-01T08:00:00Z",
      summaryZh: "创世基线稳健版",
      isActive: false,
      factorWeightsPct: { F1_ideal_reversal: 20 },
      minIntradayProjectedTurnoverPct: 5.0,
      atrStopMult: 2.0,
      oosSampleDays: 10,
      oosRankIcMean: 0.042,
      oosWinRatePct: 60.0,
      oosCumNetRetBps: 185.0,
      oosMaxDrawdownBps: 45.0,
    },
    {
      versionId: "v1.2.0",
      parentVersionId: "v1.0.0",
      sourceTag: "BASELINE",
      createdAt: "2026-11-10T08:00:00Z",
      summaryZh: "标准均衡六因子版",
      isActive: targetVersionId === "v1.2.0",
      factorWeightsPct: { F1_ideal_reversal: 22 },
      minIntradayProjectedTurnoverPct: 5.5,
      atrStopMult: 1.8,
      oosSampleDays: 10,
      oosRankIcMean: 0.051,
      oosWinRatePct: 64.0,
      oosCumNetRetBps: 220.0,
      oosMaxDrawdownBps: 38.0,
    },
    {
      versionId: "v1.2.1",
      parentVersionId: "v1.2.0",
      sourceTag: "WALK_FORWARD_PROMOTED",
      createdAt: "2026-11-19T08:00:00Z",
      summaryZh: "滚动晋升新版",
      isActive: targetVersionId === "v1.2.1",
      factorWeightsPct: { F1_ideal_reversal: 24 },
      minIntradayProjectedTurnoverPct: 5.5,
      atrStopMult: 1.8,
      oosSampleDays: 10,
      oosRankIcMean: 0.058,
      oosWinRatePct: 66.5,
      oosCumNetRetBps: 255.0,
      oosMaxDrawdownBps: 35.0,
    },
  ],
  comparison: {
    baseVersionId,
    targetVersionId,
    winRateLiftPct: 2.5,
    rankIcLift: 0.007,
    netRetLiftBps: 35.0,
    drawdownImprovementBps: 3.0,
    isUnderperforming: false,
    headlineVerdictZh: "升级验证有效",
    factorAttributions: [],
    rootCauses: [],
  },
  evolutionLogs: [],
});

describe("Review Round 3 — VersionControlArenaPanel & AntiOverfitWalkForwardPanel regressions", () => {
  it("renders responsive-cards-only fallback so <=680px narrow viewports keep version cards and switch buttons", async () => {
    const onCompare = vi.fn().mockResolvedValue(undefined);
    const onSwitch = vi.fn().mockResolvedValue(undefined);
    const { host } = await mountNode(
      <VersionControlArenaPanel
        versionControl={buildMockDashboard("v1.0.0", "v1.2.0")}
        factors={[]}
        onCompareVersions={onCompare}
        onSwitchVersion={onSwitch}
      />
    );

    const mobileCards = host.querySelector(".responsive-cards-only");
    expect(mobileCards).not.toBeNull();
    const cardBtn = host.querySelector('[data-testid="switch-version-card-v1.0.0"]');
    expect(cardBtn).not.toBeNull();
  });

  it("passes updated targetVersionId (not stale mount state) when base version selector changes after promotion", async () => {
    const onCompare = vi.fn().mockResolvedValue(undefined);
    const onSwitch = vi.fn().mockResolvedValue(undefined);

    const { host, rerender } = await mountNode(
      <VersionControlArenaPanel
        versionControl={buildMockDashboard("v1.0.0", "v1.2.0")}
        factors={[]}
        onCompareVersions={onCompare}
        onSwitchVersion={onSwitch}
      />
    );

    // Simulate promotion updating props to compare v1.2.0 -> v1.2.1
    await rerender(
      <VersionControlArenaPanel
        versionControl={buildMockDashboard("v1.2.0", "v1.2.1")}
        factors={[]}
        onCompareVersions={onCompare}
        onSwitchVersion={onSwitch}
      />
    );

    const baseSelect = host.querySelector(
      '[data-testid="version-compare-base-select"]'
    ) as HTMLSelectElement;
    expect(baseSelect).not.toBeNull();
    await act(async () => {
      baseSelect.value = "v1.0.0";
      baseSelect.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(onCompare).toHaveBeenCalledWith("v1.0.0", "v1.2.1");
  });

  it("distinguishes INSUFFICIENT_SAMPLE from overfit rejection in AntiOverfitWalkForwardPanel", async () => {
    const logItem = {
      logId: 1,
      evaluatedAt: "2026-11-19T08:00:00Z",
      triggerSource: "MANUAL",
      baseVersionId: "v1.2.0",
      resultVersionId: null,
      action: "INSUFFICIENT_SAMPLE" as EvolutionAuditLogDto["action"],
      pboEstimate: 1.0,
      deflatedIcIr: 0.0,
      oosRankIc: 0.0,
      oosWinRateLiftPct: 0.0,
      reasonZh: "有效交易截面仅 8 日（低于 5 日非重叠隔离带所需最低 14 日门槛）",
    };
    const { host } = await mountNode(
      <AntiOverfitWalkForwardPanel
        activeVersionId="v1.2.0"
        logs={[logItem]}
        onRunIteration={vi.fn().mockResolvedValue(undefined)}
      />
    );

    expect(host.textContent).toContain("样本截面不足");
  });
});
