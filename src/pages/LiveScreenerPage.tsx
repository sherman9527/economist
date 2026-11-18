import React, { useEffect, useMemo, useState } from "react";
import { Search } from "lucide-react";
import type {
  BoardKind,
  MarketPolarity,
  PortfolioOverviewDto,
  ScreenerCandidateDto,
  StockKlinePayloadDto,
  ThemeMode,
  UniverseFunnelSummaryDto,
} from "../types/terminal";
import {
  formatSignedPct,
  getBoardTradingRule,
  normalizeStockDisplayLabel,
} from "../domain/uiInvariants";
import { SafeText } from "../components/common/SafeText";
import { PitfallAuditBadges } from "../components/common/PitfallAuditBadges";
import { ScreenerInspectorPane } from "./ScreenerInspectorPane";

export interface LiveScreenerPageProps {
  funnel: UniverseFunnelSummaryDto;
  candidates: ScreenerCandidateDto[];
  portfolio: PortfolioOverviewDto;
  klinePayload: StockKlinePayloadDto;
  inspectedSymbol?: string;
  theme: ThemeMode;
  polarity: MarketPolarity;
  isBlurActive: (slotId: string) => boolean;
  onPopoverOpenChange?: (open: boolean) => void;
  onChangeKline: (symbol: string, period: "1d" | "30m") => void;
  onPlaceBuy: (cand: ScreenerCandidateDto, lotQty: number) => Promise<void>;
}

export const LiveScreenerPage: React.FC<LiveScreenerPageProps> = ({
  funnel,
  candidates,
  portfolio,
  klinePayload,
  inspectedSymbol,
  theme,
  polarity,
  isBlurActive,
  onPopoverOpenChange,
  onChangeKline,
  onPlaceBuy,
}) => {
  const [query, setQuery] = useState("");
  const [boardFilter, setBoardFilter] = useState<"ALL" | BoardKind>("ALL");
  const [decisionFilter, setDecisionFilter] = useState<"ALL" | ScreenerCandidateDto["decision"]>("ALL");
  const [selectedSymbol, setSelectedSymbol] = useState<string>(
    inspectedSymbol ?? candidates[0]?.symbol ?? "300624.SZ"
  );
  const [customLots, setCustomLots] = useState<number>(300);
  const [orderNotice, setOrderNotice] = useState<string | null>(null);

  useEffect(() => {
    if (inspectedSymbol) {
      setSelectedSymbol(inspectedSymbol);
      setOrderNotice(null);
    }
  }, [inspectedSymbol]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return candidates.filter((c) => {
      if (boardFilter !== "ALL" && c.board !== boardFilter) return false;
      if (decisionFilter !== "ALL" && c.decision !== decisionFilter) return false;
      if (!q) return true;
      return (
        c.symbol.toLowerCase().includes(q) ||
        c.nameZh.toLowerCase().includes(q) ||
        c.industrySw1.toLowerCase().includes(q)
      );
    });
  }, [candidates, query, boardFilter, decisionFilter]);

  const matchedCandidate = candidates.find((c) => c.symbol === selectedSymbol);
  const isNonCandidateInspect = Boolean(
    selectedSymbol &&
      !matchedCandidate &&
      (klinePayload.symbol === selectedSymbol || inspectedSymbol === selectedSymbol)
  );
  const activeCandidate = isNonCandidateInspect
    ? undefined
    : (matchedCandidate ?? filtered[0] ?? candidates[0]);

  useEffect(() => {
    if (!activeCandidate) return;
    const rule = getBoardTradingRule(activeCandidate.board);
    setCustomLots((prev) =>
      prev < rule.minLotQty ? Math.max(rule.minLotQty, 300) : prev
    );
  }, [activeCandidate?.symbol, activeCandidate?.board]);

  const handleSelectCandidate = (cand: ScreenerCandidateDto) => {
    setSelectedSymbol(cand.symbol);
    const rule = getBoardTradingRule(cand.board);
    setCustomLots(Math.max(rule.minLotQty, 300));
    setOrderNotice(null);
    onChangeKline(cand.symbol, klinePayload.period);
  };

  const funnelStages = [
    { label: "1. 全A活跃池", count: funnel.totalActiveCount },
    { label: "2. 剔除停牌可交易", count: funnel.tradableCount },
    { label: "3. 总市值 < 200亿", count: funnel.under200YiCount },
    { label: "4. 连续2日换手>5%放量", count: funnel.consecutiveTurnoverVolCount },
    { label: "5. 五维排雷全通过", count: funnel.pitfallClearedCount },
    { label: "6. 量化优选买点", count: funnel.qualifiedBuyCount },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px", minWidth: 0 }}>
      {/* 6-Stage Funnel Bar */}
      <section
        aria-label="全A股六级筛选漏斗"
        data-blur-slot={isBlurActive("screener-funnel") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-1"
      >
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(155px, 1fr))",
            gap: "10px",
          }}
        >
          {funnelStages.map((st, idx) => (
            <div key={st.label} className="glass-optical-card" style={{ padding: "10px 12px" }}>
              <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
                <SafeText text={st.label} />
              </div>
              <div
                style={{
                  display: "flex",
                  alignItems: "baseline",
                  justifyContent: "space-between",
                  marginTop: "4px",
                }}
              >
                <strong
                  className="lh-kpi tabular-nums"
                  style={{
                    fontSize: "19px",
                    color: idx === 5 ? "var(--accent)" : "var(--text-primary)",
                  }}
                >
                  {st.count.toLocaleString()}
                </strong>
                <span className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                  {((st.count / Math.max(1, funnel.totalActiveCount)) * 100).toFixed(1)}%
                </span>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Main Split: Candidate Radar Table + Right Inspector Side-Pane */}
      <div className="bento-grid-12">
        <section
          aria-label="30分钟轮动候选与五维排雷雷达"
          data-blur-slot={isBlurActive("screener-table") ? "active" : "optical"}
          className="col-span-7 glass-tier-2 stagger-enter stagger-2"
          style={{ display: "flex", flexDirection: "column", gap: "12px" }}
        >
          {/* Search & Board/Decision Filters */}
          <div style={{ display: "flex", flexWrap: "wrap", gap: "8px", alignItems: "center" }}>
            <div style={{ position: "relative", flex: "1 1 190px", minWidth: 0 }}>
              <Search
                size={14}
                style={{
                  position: "absolute",
                  left: "11px",
                  top: "11px",
                  color: "var(--text-muted)",
                }}
              />
              <input
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="搜索代码 / 简称 / 申万行业..."
                className="glass-input"
                style={{ paddingLeft: "32px" }}
              />
            </div>

            <div className="glass-segmented" role="group" aria-label="板块筛选">
              {(["ALL", "MAIN", "CHINEXT", "STAR", "BSE"] as const).map((b) => (
                <button
                  key={b}
                  type="button"
                  className="glass-seg-btn"
                  aria-pressed={boardFilter === b}
                  onClick={() => setBoardFilter(b)}
                >
                  {b === "ALL" ? "全板块" : b}
                </button>
              ))}
            </div>

            <div className="glass-segmented" role="group" aria-label="决策状态筛选">
              {(["ALL", "BUY", "WATCH", "REJECTED_BY_MINE"] as const).map((d) => (
                <button
                  key={d}
                  type="button"
                  className="glass-seg-btn"
                  aria-pressed={decisionFilter === d}
                  onClick={() => setDecisionFilter(d)}
                >
                  {d === "ALL"
                    ? "全部"
                    : d === "BUY"
                      ? "优选买点"
                      : d === "WATCH"
                        ? "观察池"
                        : "排雷否决"}
                </button>
              ))}
            </div>
          </div>

          {/* Desktop/Tablet Table (>=681px) */}
          <div className="glass-table-wrap responsive-table-only">
            <table className="glass-table">
              <thead>
                <tr>
                  <th style={{ width: "22%" }}>代码 / 简称</th>
                  <th style={{ width: "16%" }}>现价 / 涨幅</th>
                  <th style={{ width: "13%" }}>总市值</th>
                  <th style={{ width: "20%" }}>两日换手 &amp; 量比</th>
                  <th style={{ width: "11%" }}>得分</th>
                  <th style={{ width: "18%" }}>五维排雷</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((cand) => {
                  const isSelected = activeCandidate?.symbol === cand.symbol;
                  const upCls = cand.pctChg >= 0 ? "text-mkt-up" : "text-mkt-down";
                  return (
                    <tr
                      key={cand.symbol}
                      onClick={() => handleSelectCandidate(cand)}
                      style={{
                        cursor: "pointer",
                        outline: isSelected ? "1px solid var(--accent)" : undefined,
                      }}
                    >
                      <td>
                        <SafeText text={cand.nameZh} as="strong" />
                        <div className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                          {cand.symbol} · {cand.industrySw1}
                        </div>
                      </td>
                      <td className="tabular-nums">
                        <div style={{ fontWeight: 600 }}>¥{cand.latestPriceYuan.toFixed(2)}</div>
                        <div className={upCls} style={{ fontSize: "11.5px" }}>
                          {formatSignedPct(cand.pctChg)}
                        </div>
                      </td>
                      <td className="tabular-nums">{cand.totalMvYi.toFixed(1)}亿</td>
                      <td className="tabular-nums">
                        <div>
                          {cand.turnoverT1Pct.toFixed(1)}%→{cand.turnoverT0Pct.toFixed(1)}%
                        </div>
                        <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                          倍量 {cand.volExpansionRatio.toFixed(2)}x
                        </div>
                      </td>
                      <td className="tabular-nums">
                        <span
                          className={`glass-pill ${
                            cand.decision === "BUY"
                              ? "pill-risk-clear"
                              : cand.decision === "REJECTED_BY_MINE"
                                ? "pill-risk-veto"
                                : "pill-risk-stoploss"
                          }`}
                        >
                          {cand.convictionScore.toFixed(1)}
                        </span>
                      </td>
                      <td>
                        <PitfallAuditBadges pitfalls={cand.pitfalls} compact />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* Narrow Window <=680px Stacked Cards */}
          <div className="responsive-cards-only">
            {filtered.map((cand) => (
              <div
                key={cand.symbol}
                onClick={() => handleSelectCandidate(cand)}
                className="glass-optical-card glass-optical-card-interactive"
                style={{ cursor: "pointer", display: "flex", flexDirection: "column", gap: "8px" }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}>
                  <SafeText
                    text={normalizeStockDisplayLabel(cand.symbol, cand.nameZh)}
                    as="strong"
                  />
                  <span
                    className={`tabular-nums ${cand.pctChg >= 0 ? "text-mkt-up" : "text-mkt-down"}`}
                  >
                    ¥{cand.latestPriceYuan.toFixed(2)} ({formatSignedPct(cand.pctChg)})
                  </span>
                </div>
                <div className="tabular-nums" style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                  市值 {cand.totalMvYi.toFixed(1)}亿 · 两日换手 {cand.turnoverT1Pct.toFixed(1)}%→
                  {cand.turnoverT0Pct.toFixed(1)}% · 得分 {cand.convictionScore.toFixed(1)}
                </div>
                <PitfallAuditBadges pitfalls={cand.pitfalls} compact />
              </div>
            ))}
          </div>
        </section>

        {/* Right Side Inspector Pane with Instant K-Line Buy/Stop Preview */}
        <ScreenerInspectorPane
          selectedSymbol={selectedSymbol}
          isNonCandidateInspect={isNonCandidateInspect}
          activeCandidate={activeCandidate}
          fallbackLeadCandidate={filtered[0] ?? candidates[0]}
          watchlist={candidates.slice(0, 4)}
          portfolio={portfolio}
          klinePayload={klinePayload}
          customLots={customLots}
          orderNotice={orderNotice}
          theme={theme}
          polarity={polarity}
          inspectorBlurActive={isBlurActive("screener-inspector")}
          popoverBlurActive={isBlurActive("kline-rationale-popover")}
          onPopoverOpenChange={onPopoverOpenChange}
          onChangeCustomLots={setCustomLots}
          onSelectCandidate={handleSelectCandidate}
          onSwitchKlineSymbolPeriod={(sym, per) => {
            setSelectedSymbol(sym);
            onChangeKline(sym, per);
          }}
          onConfirmBuy={async (cand, lots) => {
            await onPlaceBuy(cand, lots);
            setOrderNotice(
              `已建仓 ${cand.nameZh} ${lots} 股，止损锚定 ¥${cand.suggestedStopLossYuan.toFixed(2)}（T+1锁仓生效）`
            );
          }}
        />
      </div>
    </div>
  );
};
