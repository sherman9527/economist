import React from "react";
import { CandlestickChart, Filter, ShoppingBag } from "lucide-react";
import type {
  MarketPolarity,
  PortfolioOverviewDto,
  ScreenerCandidateDto,
  StockKlinePayloadDto,
  ThemeMode,
} from "../types/terminal";
import {
  computeAShareFeeBreakdown,
  formatCurrencyYuan,
  getBoardTradingRule,
  validateAShareOrderInput,
} from "../domain/uiInvariants";
import { SafeText } from "../components/common/SafeText";
import { PitfallAuditBadges } from "../components/common/PitfallAuditBadges";
import { AsyncGlassButton } from "../components/common/AsyncGlassButton";
import { QuantKlineTerminal } from "../components/charts/QuantKlineTerminal";

export interface ScreenerInspectorPaneProps {
  selectedSymbol: string;
  isNonCandidateInspect: boolean;
  activeCandidate: ScreenerCandidateDto | undefined;
  fallbackLeadCandidate: ScreenerCandidateDto | undefined;
  watchlist: ScreenerCandidateDto[];
  portfolio: PortfolioOverviewDto;
  klinePayload: StockKlinePayloadDto;
  customLots: number;
  orderNotice: string | null;
  theme: ThemeMode;
  polarity: MarketPolarity;
  inspectorBlurActive: boolean;
  popoverBlurActive: boolean;
  onPopoverOpenChange?: (open: boolean) => void;
  onChangeCustomLots: (lots: number) => void;
  onSelectCandidate: (cand: ScreenerCandidateDto) => void;
  onSwitchKlineSymbolPeriod: (symbol: string, period: "1d" | "30m") => void;
  onConfirmBuy: (cand: ScreenerCandidateDto, lots: number) => Promise<void>;
}

export const ScreenerInspectorPane: React.FC<ScreenerInspectorPaneProps> = ({
  selectedSymbol,
  isNonCandidateInspect,
  activeCandidate,
  fallbackLeadCandidate,
  watchlist,
  portfolio,
  klinePayload,
  customLots,
  orderNotice,
  theme,
  polarity,
  inspectorBlurActive,
  popoverBlurActive,
  onPopoverOpenChange,
  onChangeCustomLots,
  onSelectCandidate,
  onSwitchKlineSymbolPeriod,
  onConfirmBuy,
}) => {
  const alreadyHeld = activeCandidate
    ? portfolio.positions.some((p) => p.symbol === activeCandidate.symbol)
    : false;
  const slotsFull = !alreadyHeld && portfolio.openSlotsCount >= portfolio.maxSlots;
  const boardRule = activeCandidate ? getBoardTradingRule(activeCandidate.board) : null;
  const prevCloseRef = activeCandidate
    ? (activeCandidate.prevCloseYuan ??
      activeCandidate.latestPriceYuan / Math.max(0.01, 1 + activeCandidate.pctChg / 100))
    : 10;
  const orderCheck = activeCandidate
    ? validateAShareOrderInput({
        side: "BUY",
        board: activeCandidate.board,
        prevCloseYuan: prevCloseRef,
        orderPriceYuan: activeCandidate.latestPriceYuan,
        lotQty: customLots,
      })
    : null;
  const buyFeePreview = activeCandidate
    ? computeAShareFeeBreakdown("BUY", activeCandidate.latestPriceYuan, customLots)
    : null;
  const singleCapYuan = portfolio.totalEquityYuan * 0.1;
  const existingPos = activeCandidate
    ? portfolio.positions.find((p) => p.symbol === activeCandidate.symbol)
    : undefined;
  const existingPosMv = existingPos ? existingPos.markPriceYuan * existingPos.totalQty : 0;
  const exceedsSingleCap = buyFeePreview
    ? existingPosMv + buyFeePreview.notionalYuan + buyFeePreview.totalFeeYuan >
      singleCapYuan + 1e-6
    : false;

  return (
    <section
      aria-label="单股买点K线预览与模拟盘委托台"
      data-blur-slot={inspectorBlurActive ? "active" : "optical"}
      className="col-span-5 glass-tier-2 stagger-enter stagger-3"
      style={{ display: "flex", flexDirection: "column", gap: "12px" }}
    >
      {isNonCandidateInspect ? (
        <>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
            <div className="min-w-zero">
              <SafeText
                text={`${klinePayload.nameZh} (${selectedSymbol}) · 跨市场联动K线检视`}
                as="strong"
              />
              <div className="tabular-nums" style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                非当期&lt;200亿选股候选 · 仅检视K线与历史止损锚点（禁止误绑其他标的下单）
              </div>
            </div>
            <Filter size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
          </div>

          <QuantKlineTerminal
            payload={klinePayload}
            watchlist={watchlist}
            theme={theme}
            polarity={polarity}
            popoverBlurActive={popoverBlurActive}
            onPopoverOpenChange={onPopoverOpenChange}
            onChangeSymbolPeriod={onSwitchKlineSymbolPeriod}
          />

          <div
            className="glass-optical-card lh-prose"
            style={{ fontSize: "12.5px", display: "flex", flexDirection: "column", gap: "8px" }}
          >
            <div>
              当前检视标的 <strong>{klinePayload.nameZh} ({selectedSymbol})</strong> 为非当期&lt;200亿选股候选（如研报映射大市值锚点或已清仓历史标的），系统已自动锁定委托表单以防错下至其他代码。
            </div>
            {fallbackLeadCandidate && (
              <button
                type="button"
                className="glass-btn glass-btn-ghost glass-btn-sm"
                style={{ alignSelf: "flex-start" }}
                onClick={() => onSelectCandidate(fallbackLeadCandidate)}
              >
                切换至当期榜首候选 {fallbackLeadCandidate.nameZh}
              </button>
            )}
          </div>
        </>
      ) : activeCandidate && boardRule ? (
        <>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
            <div className="min-w-zero">
              <SafeText
                text={`${activeCandidate.nameZh} (${activeCandidate.symbol}) · 五维排雷审阅`}
                as="strong"
              />
              <div className="tabular-nums" style={{ fontSize: "12px", color: "var(--text-muted)" }}>
                建议止损 ¥{activeCandidate.suggestedStopLossYuan.toFixed(2)} · 目标 ¥
                {activeCandidate.suggestedTargetYuan.toFixed(2)} · 席位 {portfolio.openSlotsCount}/
                {portfolio.maxSlots}
              </div>
            </div>
            <Filter size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
          </div>

          <QuantKlineTerminal
            payload={klinePayload}
            watchlist={watchlist}
            theme={theme}
            polarity={polarity}
            popoverBlurActive={popoverBlurActive}
            onPopoverOpenChange={onPopoverOpenChange}
            onChangeSymbolPeriod={onSwitchKlineSymbolPeriod}
          />

          <div className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            <div className="lh-prose" style={{ fontSize: "12.5px" }}>
              {activeCandidate.rationaleZh}
            </div>
            <PitfallAuditBadges pitfalls={activeCandidate.pitfalls} compact={false} />
          </div>

          <div
            className="glass-optical-card"
            style={{ display: "flex", flexDirection: "column", gap: "8px" }}
          >
            <div
              style={{
                display: "flex",
                flexWrap: "wrap",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "10px",
              }}
            >
              <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "12.5px" }}>
                <span>委托股数</span>
                <input
                  type="number"
                  min={boardRule.minLotQty}
                  step={boardRule.lotStep}
                  value={customLots}
                  onChange={(e) => onChangeCustomLots(Number(e.target.value) || 0)}
                  className="glass-input tabular-nums"
                  style={{ width: "110px" }}
                />
              </label>

              <AsyncGlassButton
                label={
                  activeCandidate.decision === "REJECTED_BY_MINE"
                    ? "排雷一票否决禁买"
                    : slotsFull
                      ? "已满 10 席上限"
                      : exceedsSingleCap
                        ? "超单票10%上限"
                        : "记入100W模拟盘"
                }
                loadingLabel="写入SQLite..."
                successLabel="已记入模拟盘"
                disabled={
                  activeCandidate.decision === "REJECTED_BY_MINE" ||
                  slotsFull ||
                  exceedsSingleCap ||
                  Boolean(orderCheck && !orderCheck.valid)
                }
                icon={<ShoppingBag size={14} />}
                onClick={() => onConfirmBuy(activeCandidate, customLots)}
              />
            </div>

            {buyFeePreview && (
              <div className="tabular-nums" style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
                预估成交 {formatCurrencyYuan(buyFeePreview.notionalYuan)} · 佣金 ¥
                {buyFeePreview.commissionYuan.toFixed(2)} · 15bps滑点 ¥
                {buyFeePreview.slippageYuan.toFixed(2)} · 单票10%上限{" "}
                {formatCurrencyYuan(singleCapYuan)}
              </div>
            )}
          </div>

          {orderCheck && !orderCheck.valid && (
            <div className="glass-pill pill-risk-veto">
              <CandlestickChart size={13} />
              <span>{orderCheck.reasons.join("；")}</span>
            </div>
          )}
          {orderNotice && <div className="glass-pill pill-risk-clear">{orderNotice}</div>}
        </>
      ) : null}
    </section>
  );
};
