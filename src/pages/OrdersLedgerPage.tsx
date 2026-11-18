import React, { useState } from "react";
import { CandlestickChart, Lock, ShieldAlert, Unlock } from "lucide-react";
import type { PortfolioOverviewDto, PositionSlotDto } from "../types/terminal";
import {
  formatCurrencyYuan,
  formatSignedPct,
  normalizeStockDisplayLabel,
} from "../domain/uiInvariants";
import { SafeText } from "../components/common/SafeText";
import { AsyncGlassButton } from "../components/common/AsyncGlassButton";

export interface OrdersLedgerPageProps {
  portfolio: PortfolioOverviewDto;
  isBlurActive: (slotId: string) => boolean;
  onJumpToKline: (symbol: string) => void;
  onClosePosition: (symbol: string, reasonZh: string) => Promise<void>;
}

const StopDistanceGauge: React.FC<{ pos: PositionSlotDto }> = ({ pos }) => {
  const pct = Math.max(0, Math.min(20, pos.stopDistancePct));
  const fillWidthPct = Math.min(100, Math.max(8, (pct / 16) * 100));
  const nearStop = pct <= 3.5;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "4px", minWidth: 0 }}>
      <div
        className="tabular-nums"
        style={{
          display: "flex",
          justifyContent: "space-between",
          fontSize: "11.5px",
          gap: "6px",
        }}
      >
        <span className="text-stoploss">止损 ¥{pos.stopLossPriceYuan.toFixed(2)}</span>
        <span>缓冲 {pos.stopDistancePct.toFixed(2)}%</span>
      </div>
      <div
        style={{
          width: "100%",
          minHeight: "7px",
          borderRadius: "999px",
          background: "rgba(148, 163, 184, 0.18)",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            width: `${fillWidthPct.toFixed(1)}%`,
            minHeight: "7px",
            borderRadius: "999px",
            background: nearStop ? "var(--risk-veto)" : "var(--risk-stoploss)",
          }}
        />
      </div>
    </div>
  );
};

export const OrdersLedgerPage: React.FC<OrdersLedgerPageProps> = ({
  portfolio,
  isBlurActive,
  onJumpToKline,
  onClosePosition,
}) => {
  const [feedback, setFeedback] = useState<string | null>(null);

  const riskMetrics = [
    { label: "初始本金锁定", value: formatCurrencyYuan(portfolio.initialCapitalYuan) },
    { label: "可用现金安全垫", value: formatCurrencyYuan(portfolio.cashYuan) },
    { label: "持仓总市值", value: formatCurrencyYuan(portfolio.marketValueYuan) },
    { label: "持仓席位占用", value: `${portfolio.openSlotsCount} / ${portfolio.maxSlots} 只` },
    { label: "累计印花税/佣金/滑点", value: formatCurrencyYuan(portfolio.totalFeesPaidYuan) },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px", minWidth: 0 }}>
      {/* Portfolio Risk Strip */}
      <section
        aria-label="100W模拟盘资金与摩擦成本横条"
        data-blur-slot={isBlurActive("ledger-risk-strip") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-1"
      >
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(175px, 1fr))",
            gap: "10px",
          }}
        >
          {riskMetrics.map((m) => (
            <div key={m.label} className="glass-optical-card" style={{ padding: "11px 14px" }}>
              <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>{m.label}</div>
              <strong className="lh-kpi tabular-nums" style={{ fontSize: "19px", display: "block", marginTop: "4px" }}>
                {m.value}
              </strong>
            </div>
          ))}
        </div>
      </section>

      {/* Active Positions Table (<= 10 Slots + T+1 Lock + Stop-Loss Gauge) */}
      <section
        aria-label="当前持仓席位与止损监控"
        data-blur-slot={isBlurActive("ledger-positions") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-2"
        style={{ display: "flex", flexDirection: "column", gap: "12px" }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
          <div>
            <strong style={{ fontSize: "15px" }}>
              活跃持仓席位 ({portfolio.openSlotsCount}/{portfolio.maxSlots})
            </strong>
            <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
              严格执行 A 股 T+1 交收锁仓与单票琥珀金动态止损线保护
            </div>
          </div>
          {feedback && <span className="glass-pill pill-risk-clear">{feedback}</span>}
        </div>

        <div className="glass-table-wrap responsive-table-only">
          <table className="glass-table">
            <thead>
              <tr>
                <th style={{ width: "19%" }}>席位 / 标的</th>
                <th style={{ width: "17%" }}>持仓 &amp; T+1交收</th>
                <th style={{ width: "16%" }}>均价 / 现价</th>
                <th style={{ width: "15%" }}>浮动盈亏</th>
                <th style={{ width: "17%" }}>止损距离标尺</th>
                <th style={{ width: "16%" }}>风控操作</th>
              </tr>
            </thead>
            <tbody>
              {portfolio.positions.map((pos) => {
                const lockedAll = pos.sellableQty === 0;
                const pnlPositive = pos.unrealizedPnlYuan >= 0;
                return (
                  <tr key={pos.positionId}>
                    <td>
                      <SafeText
                        text={`#${pos.slotNo} ${pos.nameZh}`}
                        as="strong"
                      />
                      <div className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                        {pos.symbol} · {pos.board}
                      </div>
                    </td>
                    <td className="tabular-nums">
                      <div>{pos.totalQty.toLocaleString()} 股</div>
                      <span
                        className={`glass-pill ${
                          lockedAll ? "pill-risk-stoploss" : "pill-risk-clear"
                        }`}
                        style={{ minHeight: "22px", fontSize: "11px", marginTop: "3px" }}
                      >
                        {lockedAll ? <Lock size={11} /> : <Unlock size={11} />}
                        <span>
                          {lockedAll
                            ? `T+1锁仓 ${pos.lockedT0Qty}`
                            : `可卖 ${pos.sellableQty}`}
                        </span>
                      </span>
                    </td>
                    <td className="tabular-nums">
                      <div>成本 ¥{pos.avgCostYuan.toFixed(2)}</div>
                      <div style={{ color: "var(--text-secondary)", fontSize: "12px" }}>
                        现价 ¥{pos.markPriceYuan.toFixed(2)}
                      </div>
                    </td>
                    <td className={`tabular-nums ${pnlPositive ? "text-mkt-up" : "text-mkt-down"}`}>
                      <div style={{ fontWeight: 700 }}>
                        {pnlPositive ? "+" : ""}
                        {formatCurrencyYuan(pos.unrealizedPnlYuan)}
                      </div>
                      <div style={{ fontSize: "11.5px" }}>
                        {formatSignedPct(pos.unrealizedPnlPct)}
                      </div>
                    </td>
                    <td>
                      <StopDistanceGauge pos={pos} />
                    </td>
                    <td>
                      <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
                        <button
                          type="button"
                          onClick={() => onJumpToKline(pos.symbol)}
                          className="glass-btn glass-btn-ghost glass-btn-sm"
                        >
                          <CandlestickChart size={13} />
                          <span>定位买点K线</span>
                        </button>
                        <AsyncGlassButton
                          size="sm"
                          variant={lockedAll ? "ghost" : "primary"}
                          disabled={lockedAll}
                          title={
                            lockedAll
                              ? "A股T+1交收保护：今日买入仓位须待次一交易日解锁后方可模拟平仓"
                              : "按实时对手价扣除印花税与滑点平仓"
                          }
                          label={lockedAll ? "T+1锁仓中" : "模拟平仓"}
                          loadingLabel="平仓结算..."
                          successLabel="已平仓"
                          onClick={async () => {
                            await onClosePosition(pos.symbol, "手动风控调仓卖出");
                            setFeedback(`已完成 ${pos.nameZh} (${pos.symbol}) 模拟平仓结算`);
                          }}
                        />
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>

        {/* <=680px Stacked Position Cards */}
        <div className="responsive-cards-only">
          {portfolio.positions.map((pos) => {
            const lockedAll = pos.sellableQty === 0;
            return (
              <div key={pos.positionId} className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}>
                  <SafeText text={normalizeStockDisplayLabel(pos.symbol, pos.nameZh)} as="strong" />
                  <span className={lockedAll ? "glass-pill pill-risk-stoploss" : "glass-pill pill-risk-clear"}>
                    {lockedAll ? "T+1锁仓" : `可卖 ${pos.sellableQty}股`}
                  </span>
                </div>
                <StopDistanceGauge pos={pos} />
                <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                  <button
                    type="button"
                    onClick={() => onJumpToKline(pos.symbol)}
                    className="glass-btn glass-btn-ghost glass-btn-sm"
                  >
                    定位买点K线
                  </button>
                  <AsyncGlassButton
                    size="sm"
                    disabled={lockedAll}
                    title={lockedAll ? "T+1日内禁止回转卖出" : undefined}
                    label={lockedAll ? "T+1锁仓" : "模拟平仓"}
                    onClick={() => onClosePosition(pos.symbol, "窄窗风控调仓卖出")}
                  />
                </div>
              </div>
            );
          })}
        </div>
      </section>

      {/* SQLite Historical Simulated Orders Ledger */}
      <section
        aria-label="SQLite历史模拟订单与推荐理由台账"
        data-blur-slot={isBlurActive("ledger-orders") ? "active" : "optical"}
        className="glass-tier-2 stagger-enter stagger-3"
        style={{ display: "flex", flexDirection: "column", gap: "12px" }}
      >
        <div>
          <strong style={{ fontSize: "15px" }}>
            SQLite 历史模拟委托与推荐理由审计账本
          </strong>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            每笔交易完整持久化成交价、刚性止损线、三项摩擦费用与量化专家入场依据
          </div>
        </div>

        <div className="glass-table-wrap">
          <table className="glass-table">
            <thead>
              <tr>
                <th style={{ width: "16%" }}>日期 / 标的</th>
                <th style={{ width: "13%" }}>方向 / 触发</th>
                <th style={{ width: "15%" }}>成交价 / 股数</th>
                <th style={{ width: "12%" }}>刚性止损价</th>
                <th style={{ width: "18%" }}>佣金 · 印花税 · 滑点</th>
                <th style={{ width: "26%" }}>SQLite 推荐理由存证</th>
              </tr>
            </thead>
            <tbody>
              {portfolio.recentOrders.map((ord) => (
                <tr key={ord.orderId}>
                  <td>
                    <SafeText text={`${ord.nameZh} (${ord.symbol})`} as="strong" />
                    <div className="tabular-nums" style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                      {ord.tradeDate} · 解锁 {ord.tPlus1UnlockDate}
                    </div>
                  </td>
                  <td>
                    <span
                      className={`glass-pill ${
                        ord.side === "BUY" ? "pill-mkt-up" : "pill-mkt-down"
                      }`}
                    >
                      {ord.side === "BUY" ? "买入建仓" : "卖出平仓"}
                    </span>
                    <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "3px" }}>
                      {ord.triggerType}
                    </div>
                  </td>
                  <td className="tabular-nums">
                    <div>¥{ord.fillPriceYuan.toFixed(2)}</div>
                    <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
                      {ord.lotQty.toLocaleString()} 股
                    </div>
                  </td>
                  <td className="tabular-nums">
                    <span className="glass-pill pill-risk-stoploss">
                      <ShieldAlert size={11} />
                      <span>¥{ord.stopLossPriceYuan.toFixed(2)}</span>
                    </span>
                  </td>
                  <td className="tabular-nums" style={{ fontSize: "12px" }}>
                    <div>佣 ¥{ord.commissionYuan.toFixed(2)} · 税 ¥{ord.stampDutyYuan.toFixed(2)}</div>
                    <div style={{ color: "var(--text-muted)", fontSize: "11px" }}>
                      滑点 ¥{ord.slippageYuan.toFixed(2)}
                    </div>
                  </td>
                  <td>
                    <SafeText text={ord.rationaleZh} maxLines={2} showPopover />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
};
