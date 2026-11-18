import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentSessionDto,
  EconomistModuleDto,
  PlaceOrderInputDto,
  PortfolioOverviewDto,
  RotationAlertEventDto,
  ScreenerCandidateDto,
  StockKlinePayloadDto,
  StrategyPluginConfigDto,
  TelemetryDashboardDto,
  TerminalBootstrapDto,
  UniverseFunnelSummaryDto,
  UpdatePluginInputDto,
} from "../types/terminal";
import {
  computeAShareFeeBreakdown,
  evaluateCstMarketSession,
  getBoardTradingRule,
  roundCurrency2,
} from "../domain/uiInvariants";
import { buildMockStockKline } from "./mockKlineFixtures";
import { createInitialBootstrapState } from "./mockTerminalFixtures";

export interface MarketScanResponseDto {
  funnel: UniverseFunnelSummaryDto;
  candidates: ScreenerCandidateDto[];
  alert?: RotationAlertEventDto | null;
}

type AlertCallback = (payload: RotationAlertEventDto) => void;

let fallbackState: TerminalBootstrapDto = createInitialBootstrapState();
const fallbackAlertListeners = new Set<AlertCallback>();

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function resetFallbackStateForTests(): void {
  fallbackState = createInitialBootstrapState();
}

function cloneDto<T>(val: T): T {
  return JSON.parse(JSON.stringify(val)) as T;
}

function recomputePortfolioTotals(portfolio: PortfolioOverviewDto): void {
  const mv = portfolio.positions.reduce(
    (acc, p) => acc + p.markPriceYuan * p.totalQty,
    0
  );
  portfolio.marketValueYuan = roundCurrency2(mv);
  portfolio.totalEquityYuan = roundCurrency2(portfolio.cashYuan + portfolio.marketValueYuan);
  portfolio.cumReturnPct = roundCurrency2(
    ((portfolio.totalEquityYuan - portfolio.initialCapitalYuan) / portfolio.initialCapitalYuan) * 100
  );
  portfolio.openSlotsCount = portfolio.positions.length;
}

export async function bootstrapTerminal(): Promise<TerminalBootstrapDto> {
  if (isTauriRuntime()) {
    return invoke<TerminalBootstrapDto>("bootstrap_terminal");
  }
  return cloneDto(fallbackState);
}

export async function runMarketScan(forceRefresh: boolean): Promise<MarketScanResponseDto> {
  if (isTauriRuntime()) {
    return invoke<MarketScanResponseDto>("run_market_scan", { forceRefresh });
  }
  const nowIso = new Date().toISOString();
  const session = evaluateCstMarketSession(Date.now());
  const topPicks = fallbackState.candidates.filter((c) => c.decision === "BUY");

  fallbackState.funnel = {
    ...fallbackState.funnel,
    scanRttMs: forceRefresh ? 392 : 128,
    scannedAtIso: nowIso,
    marketSession: session,
    qualifiedBuyCount: topPicks.length,
  };

  const alert: RotationAlertEventDto = {
    slotTimestampCst: nowIso.slice(0, 16).replace("T", " "),
    qualifiedCount: topPicks.length,
    topPicks: topPicks.slice(0, 3),
  };

  fallbackState.telemetry.events.unshift({
    eventId: Date.now() % 1_000_000,
    tsIso: nowIso,
    severity: "INFO",
    subsystem: "MARKET",
    eventName: "manual_30m_rotation_scan",
    durationMs: fallbackState.funnel.scanRttMs,
    ok: true,
    message: `全A股 5,576 只快照扫描完成，命中 ${topPicks.length} 只 <200亿连续两日高换手排雷优选标的。`,
  });

  for (const listener of fallbackAlertListeners) {
    listener(cloneDto(alert));
  }

  return cloneDto({
    funnel: fallbackState.funnel,
    candidates: fallbackState.candidates,
    alert,
  });
}

export async function getStockKline(
  symbol: string,
  period: "1d" | "30m"
): Promise<StockKlinePayloadDto> {
  if (isTauriRuntime()) {
    return invoke<StockKlinePayloadDto>("get_stock_kline", { symbol, period });
  }
  const cand = fallbackState.candidates.find((c) => c.symbol === symbol);
  return buildMockStockKline(symbol, period, cand);
}

export async function placeSimulatedBuy(
  input: PlaceOrderInputDto
): Promise<PortfolioOverviewDto> {
  if (isTauriRuntime()) {
    return invoke<PortfolioOverviewDto>("place_simulated_buy", { input });
  }
  const portfolio = fallbackState.portfolio;
  const existing = portfolio.positions.find((p) => p.symbol === input.symbol);
  if (!existing && portfolio.positions.length >= portfolio.maxSlots) {
    throw new Error(`模拟盘持仓席位已满 (${portfolio.maxSlots}/10)，请先平仓释放席位`);
  }

  const cand = fallbackState.candidates.find((c) => c.symbol === input.symbol);
  const board = cand?.board ?? "MAIN";
  const rule = getBoardTradingRule(board);
  const entryPrice = cand?.latestPriceYuan ?? 25.0;
  const fillPrice = roundCurrency2(entryPrice * 1.0015);

  const defaultLots = Math.max(
    rule.minLotQty,
    Math.floor(95_000 / fillPrice / 100) * 100
  );
  const lotQty = input.lotQty ?? defaultLots;
  if (lotQty < rule.minLotQty || (lotQty - rule.minLotQty) % rule.lotStep !== 0) {
    throw new Error(`${board} 买入股数须不低于 ${rule.minLotQty} 股且符合最小步长 ${rule.lotStep}`);
  }

  const rawStop = input.stopLossPriceYuan ?? cand?.suggestedStopLossYuan ?? roundCurrency2(entryPrice * 0.92);
  const stopLoss = rawStop < entryPrice ? rawStop : roundCurrency2(entryPrice * 0.93);
  const fees = computeAShareFeeBreakdown("BUY", entryPrice, lotQty);
  const fillNotional = roundCurrency2(fillPrice * lotQty);
  const existingValue = existing ? roundCurrency2(existing.markPriceYuan * existing.totalQty) : 0;
  const singleCapYuan = roundCurrency2(portfolio.initialCapitalYuan * 0.1);
  if (existingValue + fillNotional > singleCapYuan + 1e-6) {
    throw new Error(`单票仓位上限保护：持仓市值不得超过初始本金 10% (¥${singleCapYuan.toFixed(2)})`);
  }

  // fillPrice already embeds 15 bps impact slippage; cash pays fillNotional + commission
  const totalCost = roundCurrency2(fillNotional + fees.commissionYuan);

  if (totalCost > portfolio.cashYuan) {
    throw new Error(`可用现金不足：需 ¥${totalCost.toFixed(2)}，当前可用 ¥${portfolio.cashYuan.toFixed(2)}`);
  }

  portfolio.cashYuan = roundCurrency2(portfolio.cashYuan - totalCost);
  portfolio.totalFeesPaidYuan = roundCurrency2(portfolio.totalFeesPaidYuan + fees.commissionYuan);

  const nowIso = new Date().toISOString();
  const rationale =
    input.rationaleZh?.trim() ||
    cand?.rationaleZh ||
    "满足<200亿市值与连续两日换手>5%放量突破，五维排雷通过。";

  if (existing) {
    const newTotal = existing.totalQty + lotQty;
    existing.avgCostYuan = roundCurrency2(
      (existing.avgCostYuan * existing.totalQty + fillPrice * lotQty) / newTotal
    );
    existing.totalQty = newTotal;
    existing.lockedT0Qty += lotQty;
    existing.stopLossPriceYuan = Math.min(existing.stopLossPriceYuan, stopLoss);
  } else {
    portfolio.positions.push({
      positionId: `pos-${Date.now()}`,
      slotNo: portfolio.positions.length + 1,
      symbol: input.symbol,
      nameZh: cand?.nameZh ?? input.symbol,
      board,
      totalQty: lotQty,
      sellableQty: 0,
      lockedT0Qty: lotQty,
      avgCostYuan: fillPrice,
      markPriceYuan: entryPrice,
      stopLossPriceYuan: stopLoss,
      stopDistancePct: roundCurrency2(((entryPrice - stopLoss) / entryPrice) * 100),
      unrealizedPnlYuan: roundCurrency2((entryPrice - fillPrice) * lotQty),
      unrealizedPnlPct: roundCurrency2(((entryPrice - fillPrice) / fillPrice) * 100),
      entryRationaleZh: rationale,
      openedAtIso: nowIso,
    });
  }

  portfolio.recentOrders.unshift({
    orderId: `ord-${Date.now()}`,
    recId: `rec-${input.symbol.replace(".", "-").toLowerCase()}-latest`,
    symbol: input.symbol,
    nameZh: cand?.nameZh ?? input.symbol,
    side: "BUY",
    triggerType: "QUANT_SIGNAL",
    status: "FILLED",
    lotQty,
    entryPriceYuan: entryPrice,
    fillPriceYuan: fillPrice,
    stopLossPriceYuan: stopLoss,
    commissionYuan: fees.commissionYuan,
    stampDutyYuan: fees.stampDutyYuan,
    slippageYuan: fees.slippageYuan,
    rationaleZh: rationale,
    tradeDate: nowIso.slice(0, 10),
    tPlus1UnlockDate: "次一交易日 09:30",
    createdAtIso: nowIso,
  });

  recomputePortfolioTotals(portfolio);
  return cloneDto(portfolio);
}

export async function closeSimulatedPosition(
  symbol: string,
  reasonZh: string
): Promise<PortfolioOverviewDto> {
  if (isTauriRuntime()) {
    return invoke<PortfolioOverviewDto>("close_simulated_position", { symbol, reasonZh });
  }
  const portfolio = fallbackState.portfolio;
  const idx = portfolio.positions.findIndex((p) => p.symbol === symbol);
  if (idx < 0) {
    throw new Error(`未找到持仓标的 ${symbol}`);
  }
  const pos = portfolio.positions[idx];
  if (pos.sellableQty <= 0) {
    throw new Error(`${pos.nameZh} (${symbol}) 受 A 股 T+1 交收制度保护，今日新建仓位次一交易日方可卖出`);
  }

  const sellQty = pos.sellableQty;
  const fillPrice = roundCurrency2(pos.markPriceYuan * 0.9985);
  const fees = computeAShareFeeBreakdown("SELL", pos.markPriceYuan, sellQty);
  const fillNotional = roundCurrency2(fillPrice * sellQty);
  const explicitFees = roundCurrency2(fees.commissionYuan + fees.stampDutyYuan);
  const netProceeds = roundCurrency2(fillNotional - explicitFees);

  portfolio.cashYuan = roundCurrency2(portfolio.cashYuan + netProceeds);
  portfolio.totalFeesPaidYuan = roundCurrency2(portfolio.totalFeesPaidYuan + explicitFees);

  if (pos.lockedT0Qty > 0) {
    pos.totalQty = pos.lockedT0Qty;
    pos.sellableQty = 0;
  } else {
    portfolio.positions.splice(idx, 1);
    portfolio.positions.forEach((p, i) => {
      p.slotNo = i + 1;
    });
  }

  const nowIso = new Date().toISOString();
  portfolio.recentOrders.unshift({
    orderId: `ord-${Date.now()}`,
    recId: null,
    symbol,
    nameZh: pos.nameZh,
    side: "SELL",
    triggerType: "REBALANCE_EXIT",
    status: "FILLED",
    lotQty: sellQty,
    entryPriceYuan: pos.avgCostYuan,
    fillPriceYuan: fillPrice,
    stopLossPriceYuan: pos.stopLossPriceYuan,
    commissionYuan: fees.commissionYuan,
    stampDutyYuan: fees.stampDutyYuan,
    slippageYuan: fees.slippageYuan,
    rationaleZh: reasonZh.trim() || "主动轮动调仓平仓",
    tradeDate: nowIso.slice(0, 10),
    tPlus1UnlockDate: nowIso.slice(0, 10),
    createdAtIso: nowIso,
  });

  recomputePortfolioTotals(portfolio);
  return cloneDto(portfolio);
}

export async function updatePluginConfig(
  input: UpdatePluginInputDto
): Promise<StrategyPluginConfigDto> {
  if (isTauriRuntime()) {
    return invoke<StrategyPluginConfigDto>("update_plugin_config", { input });
  }
  const cfg = fallbackState.pluginConfig;
  cfg.minIntradayProjectedTurnoverPct = input.minIntradayProjectedTurnoverPct;
  cfg.insiderSellVetoPct = input.insiderSellVetoPct;
  cfg.holderSurgeVetoPct = input.holderSurgeVetoPct;
  cfg.mainOutflowVetoRatio = input.mainOutflowVetoRatio;
  cfg.atrStopMult = input.atrStopMult;
  cfg.marketPolarity = input.marketPolarity;
  cfg.factors = cfg.factors.map((f) => ({
    ...f,
    weightPct: input.factorWeights[f.factorId] ?? f.weightPct,
  }));
  cfg.rawStrategyJson = JSON.stringify(
    {
      strategyId: cfg.strategyId,
      version: cfg.version,
      coreLocks: {
        maxTotalMvYi: cfg.maxTotalMvYi,
        minTurnoverPct: cfg.minTurnoverPct,
        consecutiveDays: cfg.consecutiveDays,
        initialCapitalYuan: cfg.initialCapitalYuan,
        maxPositions: cfg.maxPositions,
      },
      tunableParams: input,
    },
    null,
    2
  );
  return cloneDto(cfg);
}

export async function ackEconomistReport(
  reportId: string,
  acked: boolean
): Promise<EconomistModuleDto> {
  if (isTauriRuntime()) {
    return invoke<EconomistModuleDto>("ack_economist_report", { reportId, acked });
  }
  const nowIso = new Date().toISOString();
  fallbackState.economist.reports = fallbackState.economist.reports.map((r) =>
    r.reportId === reportId ? { ...r, acked, ackedAt: acked ? nowIso : null } : r
  );
  fallbackState.economist.unackedCount = fallbackState.economist.reports.filter(
    (r) => !r.acked
  ).length;
  return cloneDto(fallbackState.economist);
}

export async function getEconomistReportHtml(relPath: string): Promise<string> {
  if (isTauriRuntime()) {
    return invoke<string>("get_economist_report_html", { relPath });
  }
  const report =
    fallbackState.economist.reports.find((r) => r.relPath === relPath) ??
    fallbackState.economist.reports[0];

  const chainSections = report.chains
    .map(
      (c) => `
      <section class="chain-card">
        <h3>${c.chainTitle}</h3>
        <p><strong>《经济学人》原文催化剂：</strong>${c.economistTrigger}</p>
        <p><strong>美股映射：</strong>${c.usMapping}</p>
        <p><strong>A股小盘轮动映射：</strong>${c.cnMapping}</p>
        <p><strong>中美定价温差：</strong>${c.divergenceInsight}</p>
        <p class="tickers">关联标的：${c.relatedCnSymbols.join(" · ")}</p>
      </section>`
    )
    .join("");

  return `<!doctype html>
<html lang="zh-CN" class="embed-liquid-reader">
<head>
<meta charset="UTF-8" />
<style>
  body { margin: 0; padding: 24px 28px; font-family: -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif; background: #0a1020; color: #eef4ff; line-height: 1.72; }
  .meta { font-size: 12px; color: #93a7c8; margin-bottom: 8px; font-variant-numeric: tabular-nums; }
  h1 { font-size: 20px; line-height: 1.42; margin: 0 0 14px; color: #f8fbff; }
  .lead { padding: 14px 16px; border-radius: 14px; background: rgba(56, 189, 248, 0.1); border: 1px solid rgba(148, 197, 255, 0.22); margin-bottom: 18px; }
  .chain-card { padding: 16px 18px; border-radius: 16px; background: rgba(255, 255, 255, 0.045); border: 1px solid rgba(255, 255, 255, 0.1); margin-bottom: 14px; }
  .chain-card h3 { margin: 0 0 10px; font-size: 16px; color: #7dd3fc; }
  .chain-card p { margin: 6px 0; font-size: 14px; color: #d6e2f5; }
  .tickers { color: #fbbf24 !important; font-variant-numeric: tabular-nums; font-weight: 600; }
</style>
</head>
<body>
  <div class="meta">便携相对路径：${report.relPath}?embed=1 · 日期 ${report.date} · 核心深读 ${report.coreCount}/${report.articleCount} 篇</div>
  <h1>${report.titleZh}</h1>
  <div class="lead">${report.executiveSummaryZh}</div>
  ${chainSections}
</body>
</html>`;
}

export async function runStorageHygiene(): Promise<TelemetryDashboardDto> {
  if (isTauriRuntime()) {
    return invoke<TelemetryDashboardDto>("run_storage_hygiene");
  }
  const nowIso = new Date().toISOString();
  fallbackState.telemetry.storage.buildFootprintBytes = Math.max(
    410_000_000,
    fallbackState.telemetry.storage.buildFootprintBytes - 24_117_248
  );
  fallbackState.telemetry.events.unshift({
    eventId: Date.now() % 1_000_000,
    tsIso: nowIso,
    severity: "INFO",
    subsystem: "CLEANER",
    eventName: "manual_hygiene_executed",
    durationMs: 41,
    ok: true,
    message: "执行 ./ 便携目录临时缓存清理与 28 天滚动日志压缩，零绝对路径校验通过。",
  });
  return cloneDto(fallbackState.telemetry);
}

export async function wakeAgentSession(
  agentRole: AgentSessionDto["agentRole"],
  prompt: string
): Promise<AgentSessionDto> {
  if (isTauriRuntime()) {
    return invoke<AgentSessionDto>("wake_agent_session", { agentRole, prompt });
  }
  const target = fallbackState.telemetry.sessions.find((s) => s.agentRole === agentRole);
  if (!target) {
    throw new Error(`未找到持久化 Agent 会话: ${agentRole}`);
  }
  const nowIso = new Date().toISOString();
  target.turnCount += 1;
  target.status = "IDLE";
  target.lastWokenAt = nowIso;

  fallbackState.telemetry.events.unshift({
    eventId: Date.now() % 1_000_000,
    tsIso: nowIso,
    severity: "INFO",
    subsystem: "AGENT",
    eventName: `resume_${agentRole}_ok`,
    durationMs: 840,
    ok: true,
    message: `通过 --resume ${target.sessionId} (gemini-4-carbon · high) 完成增量任务: ${prompt.slice(0, 64)}`,
  });

  return cloneDto(target);
}

export async function subscribeRotationAlert(
  onAlert: AlertCallback
): Promise<UnlistenFn> {
  if (isTauriRuntime()) {
    return listen<RotationAlertEventDto>("quant://rotation-alert", (evt) => {
      onAlert(evt.payload);
    });
  }
  fallbackAlertListeners.add(onAlert);
  return () => {
    fallbackAlertListeners.delete(onAlert);
  };
}
