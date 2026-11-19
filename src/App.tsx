import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  AgentSessionDto,
  MarketPolarity,
  RotationAlertEventDto,
  ScreenerCandidateDto,
  TerminalBootstrapDto,
  ThemeMode,
  UpdatePluginInputDto,
} from "./types/terminal";
import type { WorkspacePageId } from "./types/ui";
import { clampBlurSlots } from "./domain/uiInvariants";
import {
  ackEconomistReport,
  bootstrapTerminal,
  closeSimulatedPosition,
  getEconomistReportHtml,
  getStockKline,
  placeSimulatedBuy,
  runMarketScan,
  runStorageHygiene,
  subscribeRotationAlert,
  updatePluginConfig,
  wakeAgentSession,
} from "./services/tauriClient";
import {
  addToWatchlist,
  getVersionControlDashboard,
  removeFromWatchlist,
  runAntiOverfitIteration,
  switchStrategyVersion,
} from "./services/evolutionClient";
import { createInitialBootstrapState } from "./services/mockTerminalFixtures";
import { MeshBackdrop } from "./components/shell/MeshBackdrop";
import { AppSidebar } from "./components/shell/AppSidebar";
import { TopStatusBar } from "./components/shell/TopStatusBar";
import { RotationAlertToast } from "./components/shell/RotationAlertToast";
import { CommandDashboardPage } from "./pages/CommandDashboardPage";
import { LiveScreenerPage } from "./pages/LiveScreenerPage";
import { OrdersLedgerPage } from "./pages/OrdersLedgerPage";
import { FactorPluginLabPage } from "./pages/FactorPluginLabPage";
import { EconomistNewsPage } from "./pages/EconomistNewsPage";
import { TelemetrySessionsPage } from "./pages/TelemetrySessionsPage";

const PAGE_BLUR_SLOTS: Record<WorkspacePageId, string[]> = {
  dashboard: [
    "dash-kpi-shell",
    "dash-equity-panel",
    "dash-donut-panel",
    "dash-kline-panel",
    "dash-factor-panel",
  ],
  screener: ["screener-funnel", "screener-table", "screener-inspector"],
  ledger: ["ledger-risk-strip", "ledger-positions", "ledger-orders"],
  "factor-lab": ["factor-leaderboard", "factor-chart", "factor-plugin-editor"],
  economist: ["econ-header", "econ-body"],
  telemetry: ["telemetry-storage", "telemetry-sessions", "telemetry-events"],
};

export const App: React.FC = () => {
  const [terminal, setTerminal] = useState<TerminalBootstrapDto>(() =>
    createInitialBootstrapState()
  );
  const [activePage, setActivePage] = useState<WorkspacePageId>("dashboard");
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [theme, setTheme] = useState<ThemeMode>("dark");
  const [polarity, setPolarity] = useState<MarketPolarity>("cn-red-up");
  const [rotationAlert, setRotationAlert] = useState<RotationAlertEventDto | null>(null);
  const [klinePopoverOpen, setKlinePopoverOpen] = useState<boolean>(false);
  const [inspectedSymbol, setInspectedSymbol] = useState<string | undefined>(undefined);
  const [errorBanner, setErrorBanner] = useState<string | null>(null);
  const dismissedAlertSlotRef = useRef<string | null>(null);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    document.documentElement.setAttribute("data-polarity", polarity);
  }, [polarity]);

  useEffect(() => {
    setKlinePopoverOpen(false);
  }, [activePage]);

  useEffect(() => {
    let mounted = true;
    void bootstrapTerminal()
      .then((dto) => {
        if (!mounted) return;
        setTerminal(dto);
        setPolarity(dto.pluginConfig.marketPolarity);
      })
      .catch((err: unknown) => {
        if (mounted) {
          setErrorBanner(err instanceof Error ? err.message : "终端初始化加载异常");
        }
      });

    let unlistenFn: (() => void) | null = null;
    void subscribeRotationAlert((payload) => {
      if (
        mounted &&
        payload.topPicks.length > 0 &&
        payload.slotTimestampCst !== dismissedAlertSlotRef.current
      ) {
        setRotationAlert(payload);
      }
    }).then((fn) => {
      unlistenFn = fn;
    });

    return () => {
      mounted = false;
      unlistenFn?.();
    };
  }, []);

  const activeBlurSlots = useMemo(() => {
    const requested: string[] = ["shell-sidebar", "shell-topbar"];
    if (rotationAlert) {
      requested.push("toast-rotation");
    }
    if (klinePopoverOpen) {
      requested.push("kline-rationale-popover");
    }
    requested.push(...PAGE_BLUR_SLOTS[activePage]);
    return clampBlurSlots(requested, 8);
  }, [activePage, rotationAlert, klinePopoverOpen]);

  const isBlurActive = useCallback(
    (slotId: string) => activeBlurSlots.has(slotId),
    [activeBlurSlots]
  );

  const handleRunScan = useCallback(async () => {
    setErrorBanner(null);
    dismissedAlertSlotRef.current = null;
    const res = await runMarketScan(true);
    setTerminal((prev) => ({
      ...prev,
      funnel: res.funnel,
      candidates: res.candidates,
      watchlist: res.watchlist ?? prev.watchlist,
    }));
    if (res.alert && res.alert.topPicks.length > 0) {
      setRotationAlert(res.alert);
    }
  }, []);

  const handleChangeKline = useCallback(async (symbol: string, period: "1d" | "30m") => {
    try {
      const kline = await getStockKline(symbol, period);
      setTerminal((prev) => ({ ...prev, featuredKline: kline }));
    } catch (err: unknown) {
      setErrorBanner(err instanceof Error ? err.message : "K线切换加载失败");
    }
  }, []);

  const handleJumpToKline = useCallback(
    (symbol: string) => {
      setInspectedSymbol(symbol);
      void handleChangeKline(symbol, "1d");
      setActivePage("screener");
    },
    [handleChangeKline]
  );

  const handlePlaceBuy = useCallback(
    async (cand: ScreenerCandidateDto, lotQty?: number) => {
      setErrorBanner(null);
      try {
        const updatedPortfolio = await placeSimulatedBuy({
          symbol: cand.symbol,
          lotQty: lotQty ?? null,
          rationaleZh: cand.rationaleZh,
          stopLossPriceYuan: cand.suggestedStopLossYuan,
        });
        setTerminal((prev) => ({ ...prev, portfolio: updatedPortfolio }));
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "模拟买入委托被风控拦截";
        setErrorBanner(msg);
        throw err;
      }
    },
    []
  );

  const handleClosePosition = useCallback(async (symbol: string, reasonZh: string) => {
    setErrorBanner(null);
    try {
      const updatedPortfolio = await closeSimulatedPosition(symbol, reasonZh);
      setTerminal((prev) => ({ ...prev, portfolio: updatedPortfolio }));
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : "模拟平仓受阻";
      setErrorBanner(msg);
      throw err;
    }
  }, []);

  const handleSavePluginConfig = useCallback(async (input: UpdatePluginInputDto) => {
    const updatedCfg = await updatePluginConfig(input);
    const updatedVc = await getVersionControlDashboard();
    setPolarity(updatedCfg.marketPolarity);
    setTerminal((prev) => ({
      ...prev,
      pluginConfig: updatedCfg,
      versionControl: updatedVc,
    }));
  }, []);

  const handleAddWatchlist = useCallback(async (cand: ScreenerCandidateDto) => {
    setErrorBanner(null);
    const updatedWatchlist = await addToWatchlist({
      symbol: cand.symbol,
      nameZh: cand.nameZh,
      board: cand.board,
      industrySw1: cand.industrySw1,
      entryPriceYuan: cand.latestPriceYuan,
      stopLossPriceYuan: cand.suggestedStopLossYuan,
      targetPriceYuan: cand.suggestedTargetYuan,
      convictionScore: cand.convictionScore,
      sourceDecision: cand.decision,
      rationaleZh: cand.rationaleZh,
      factorZScores: cand.factorZScores,
    });
    setTerminal((prev) => ({ ...prev, watchlist: updatedWatchlist }));
  }, []);

  const handleRemoveWatchlist = useCallback(async (symbol: string) => {
    const updatedWatchlist = await removeFromWatchlist(symbol);
    setTerminal((prev) => ({ ...prev, watchlist: updatedWatchlist }));
  }, []);

  const handleCompareVersions = useCallback(async (baseId: string, targetId: string) => {
    const vc = await getVersionControlDashboard(baseId, targetId);
    setTerminal((prev) => ({ ...prev, versionControl: vc }));
  }, []);

  const handleSwitchVersion = useCallback(async (versionId: string) => {
    const vc = await switchStrategyVersion(versionId);
    const matched = vc.versions.find((v) => v.versionId === versionId);
    setTerminal((prev) => ({
      ...prev,
      versionControl: vc,
      pluginConfig: matched
        ? {
            ...prev.pluginConfig,
            version: versionId.replace(/^v/, ""),
            minIntradayProjectedTurnoverPct: matched.minIntradayProjectedTurnoverPct,
            atrStopMult: matched.atrStopMult,
            factors: prev.pluginConfig.factors.map((f) => ({
              ...f,
              weightPct: matched.factorWeightsPct[f.factorId] ?? f.weightPct,
            })),
          }
        : prev.pluginConfig,
    }));
  }, []);

  const handleRunAntiOverfit = useCallback(async () => {
    const vc = await runAntiOverfitIteration();
    setTerminal((prev) => ({ ...prev, versionControl: vc }));
  }, []);

  const handleAckReport = useCallback(async (reportId: string, acked: boolean) => {
    const updatedEcon = await ackEconomistReport(reportId, acked);
    setTerminal((prev) => ({ ...prev, economist: updatedEcon }));
  }, []);

  const handleBatchAckUnread = useCallback(async (reportIds: string[]) => {
    let latestEcon = null;
    for (const id of reportIds) {
      latestEcon = await ackEconomistReport(id, true);
    }
    if (latestEcon) {
      const finalEcon = latestEcon;
      setTerminal((prev) => ({ ...prev, economist: finalEcon }));
    }
  }, []);

  const handleRunHygiene = useCallback(async () => {
    const updatedTelemetry = await runStorageHygiene();
    setTerminal((prev) => ({ ...prev, telemetry: updatedTelemetry }));
  }, []);

  const handleWakeSession = useCallback(
    async (role: AgentSessionDto["agentRole"], prompt: string) => {
      const woken = await wakeAgentSession(role, prompt);
      setTerminal((prev) => ({
        ...prev,
        telemetry: {
          ...prev.telemetry,
          sessions: prev.telemetry.sessions.map((s) =>
            s.agentRole === role ? woken : s
          ),
        },
      }));
    },
    []
  );

  return (
    <>
      <MeshBackdrop />

      <div className="terminal-shell">
        <AppSidebar
          activePage={activePage}
          collapsed={sidebarCollapsed}
          unackedEconomistCount={terminal.economist.unackedCount}
          qualifiedPicksCount={terminal.funnel.qualifiedBuyCount}
          portableRootRel={terminal.telemetry.storage.portableRootRel}
          blurActive={isBlurActive("shell-sidebar")}
          onSelectPage={setActivePage}
          onToggleCollapse={() => setSidebarCollapsed((c) => !c)}
        />

        <div className="terminal-main-col">
          <TopStatusBar
            funnel={terminal.funnel}
            portfolio={terminal.portfolio}
            theme={theme}
            polarity={polarity}
            blurActive={isBlurActive("shell-topbar")}
            onRunScan={handleRunScan}
            onToggleTheme={() => setTheme((t) => (t === "dark" ? "light" : "dark"))}
            onTogglePolarity={() =>
              setPolarity((p) => (p === "cn-red-up" ? "intl-green-up" : "cn-red-up"))
            }
          />

          {errorBanner && (
            <div role="alert" className="glass-pill pill-risk-veto" style={{ alignSelf: "flex-start" }}>
              <span>风控提示：{errorBanner}</span>
            </div>
          )}

          <main className="min-w-zero" aria-live="polite">
            {activePage === "dashboard" && (
              <CommandDashboardPage
                funnel={terminal.funnel}
                portfolio={terminal.portfolio}
                candidates={terminal.candidates}
                klinePayload={terminal.featuredKline}
                theme={theme}
                polarity={polarity}
                isBlurActive={isBlurActive}
                onPopoverOpenChange={setKlinePopoverOpen}
                onChangeKline={(sym, per) => void handleChangeKline(sym, per)}
                onOpenScreener={() => setActivePage("screener")}
              />
            )}

            {activePage === "screener" && (
              <LiveScreenerPage
                funnel={terminal.funnel}
                candidates={terminal.candidates}
                portfolio={terminal.portfolio}
                watchlistOverview={terminal.watchlist}
                klinePayload={terminal.featuredKline}
                inspectedSymbol={inspectedSymbol}
                theme={theme}
                polarity={polarity}
                isBlurActive={isBlurActive}
                onPopoverOpenChange={setKlinePopoverOpen}
                onChangeKline={(sym, per) => {
                  setInspectedSymbol(sym);
                  void handleChangeKline(sym, per);
                }}
                onAddWatchlist={handleAddWatchlist}
                onPlaceBuy={(cand, lots) => handlePlaceBuy(cand, lots)}
              />
            )}

            {activePage === "ledger" && (
              <OrdersLedgerPage
                portfolio={terminal.portfolio}
                watchlist={terminal.watchlist}
                isBlurActive={isBlurActive}
                onJumpToKline={handleJumpToKline}
                onClosePosition={handleClosePosition}
                onRemoveWatchlist={handleRemoveWatchlist}
              />
            )}

            {activePage === "factor-lab" && (
              <FactorPluginLabPage
                pluginConfig={terminal.pluginConfig}
                versionControl={terminal.versionControl}
                theme={theme}
                polarity={polarity}
                isBlurActive={isBlurActive}
                onSavePluginConfig={handleSavePluginConfig}
                onCompareVersions={handleCompareVersions}
                onSwitchVersion={handleSwitchVersion}
                onRunAntiOverfitIteration={handleRunAntiOverfit}
                onWakeQuantDiagnosis={(prompt) =>
                  handleWakeSession("quant_researcher", prompt)
                }
              />
            )}

            {activePage === "economist" && (
              <EconomistNewsPage
                economist={terminal.economist}
                isBlurActive={isBlurActive}
                onAckReport={handleAckReport}
                onBatchAckUnread={handleBatchAckUnread}
                onLoadReportHtml={getEconomistReportHtml}
                onJumpToKline={handleJumpToKline}
              />
            )}

            {activePage === "telemetry" && (
              <TelemetrySessionsPage
                telemetry={terminal.telemetry}
                isBlurActive={isBlurActive}
                onRunHygiene={handleRunHygiene}
                onWakeSession={handleWakeSession}
              />
            )}
          </main>
        </div>
      </div>

      <RotationAlertToast
        alert={rotationAlert}
        blurActive={isBlurActive("toast-rotation")}
        onDismiss={() => {
          if (rotationAlert) {
            dismissedAlertSlotRef.current = rotationAlert.slotTimestampCst;
          }
          setRotationAlert(null);
        }}
        onInspectKline={(cand) => {
          setRotationAlert(null);
          handleJumpToKline(cand.symbol);
        }}
        onAddWatchlist={(cand) => handleAddWatchlist(cand)}
        onSimulatedBuy={async (cand) => {
          await handlePlaceBuy(cand);
          setRotationAlert(null);
        }}
      />
    </>
  );
};
