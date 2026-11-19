import React, { useEffect, useState } from "react";
import { Lock, SlidersHorizontal } from "lucide-react";
import type {
  MarketPolarity,
  StrategyPluginConfigDto,
  UpdatePluginInputDto,
} from "../../types/terminal";
import { SafeText } from "../common/SafeText";
import { AsyncGlassButton } from "../common/AsyncGlassButton";

export interface StrategyPluginEditorSectionProps {
  pluginConfig: StrategyPluginConfigDto;
  blurActive: boolean;
  onSavePluginConfig: (input: UpdatePluginInputDto) => Promise<void>;
}

export const StrategyPluginEditorSection: React.FC<StrategyPluginEditorSectionProps> = ({
  pluginConfig,
  blurActive,
  onSavePluginConfig,
}) => {
  const [projTurnover, setProjTurnover] = useState(pluginConfig.minIntradayProjectedTurnoverPct);
  const [insiderVeto, setInsiderVeto] = useState(pluginConfig.insiderSellVetoPct);
  const [holderVeto, setHolderVeto] = useState(pluginConfig.holderSurgeVetoPct);
  const [outflowVeto, setOutflowVeto] = useState(pluginConfig.mainOutflowVetoRatio);
  const [atrMult, setAtrMult] = useState(pluginConfig.atrStopMult);
  const [cfgPolarity, setCfgPolarity] = useState<MarketPolarity>(pluginConfig.marketPolarity);
  const [weights, setWeights] = useState<Record<string, number>>(() =>
    Object.fromEntries(pluginConfig.factors.map((f) => [f.factorId, f.weightPct]))
  );
  const [statusMsg, setStatusMsg] = useState<string | null>(null);

  useEffect(() => {
    setProjTurnover(pluginConfig.minIntradayProjectedTurnoverPct);
    setInsiderVeto(pluginConfig.insiderSellVetoPct);
    setHolderVeto(pluginConfig.holderSurgeVetoPct);
    setOutflowVeto(pluginConfig.mainOutflowVetoRatio);
    setAtrMult(pluginConfig.atrStopMult);
    setCfgPolarity(pluginConfig.marketPolarity);
    setWeights(Object.fromEntries(pluginConfig.factors.map((f) => [f.factorId, f.weightPct])));
  }, [pluginConfig]);

  const weightSum = Object.values(weights).reduce((a, b) => a + b, 0);

  return (
    <section
      aria-label="参数化策略插件热加载中心"
      data-blur-slot={blurActive ? "active" : "optical"}
      className="glass-tier-2 stagger-enter stagger-3"
      style={{ display: "flex", flexDirection: "column", gap: "14px" }}
    >
      <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
        <div>
          <strong style={{ fontSize: "15px" }}>
            参数化策略插件配置器 (`{pluginConfig.manifestRelPath}` · v{pluginConfig.version})
          </strong>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            每次保存自动生成不可变版本快照并纳入平行样本外影子追踪，老版本永不丢失
          </div>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
          <span className="glass-pill pill-risk-stoploss tabular-nums">
            <Lock size={11} /> 市值 &lt;{pluginConfig.maxTotalMvYi}亿
          </span>
          <span className="glass-pill pill-risk-stoploss tabular-nums">
            <Lock size={11} /> 换手 &gt;{pluginConfig.minTurnoverPct}% × {pluginConfig.consecutiveDays}D
          </span>
          <span className="glass-pill pill-risk-stoploss tabular-nums">
            <Lock size={11} /> 本金 100W · &le;{pluginConfig.maxPositions}只
          </span>
        </div>
      </div>

      <div className="bento-grid-12">
        <div
          className="col-span-7"
          style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "10px" }}
        >
          <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
            <span>盘中预测日换手下限 (%)</span>
            <input
              type="number"
              step="0.2"
              min="5.0"
              max="20.0"
              value={projTurnover}
              onChange={(e) => setProjTurnover(Number(e.target.value))}
              className="glass-input tabular-nums"
            />
          </label>

          <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
            <span>大股东拟减持一票否决阈值 (%)</span>
            <input
              type="number"
              step="0.1"
              min="0.1"
              max="5.0"
              value={insiderVeto}
              onChange={(e) => setInsiderVeto(Number(e.target.value))}
              className="glass-input tabular-nums"
            />
          </label>

          <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
            <span>PIT股东户数单期激增否决 (%)</span>
            <input
              type="number"
              step="1"
              min="5"
              max="60"
              value={holderVeto}
              onChange={(e) => setHolderVeto(Number(e.target.value))}
              className="glass-input tabular-nums"
            />
          </label>

          <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
            <span>主力净流出占成交否决比</span>
            <input
              type="number"
              step="0.01"
              min="-0.3"
              max="-0.01"
              value={outflowVeto}
              onChange={(e) => setOutflowVeto(Number(e.target.value))}
              className="glass-input tabular-nums"
            />
          </label>

          <label className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
            <span>ATR(14) 动态止损乘数</span>
            <input
              type="number"
              step="0.1"
              min="1.0"
              max="4.0"
              value={atrMult}
              onChange={(e) => setAtrMult(Number(e.target.value))}
              className="glass-input tabular-nums"
            />
          </label>

          <div className="glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "12.5px" }}>
            <span>默认行情色彩极性</span>
            <div className="glass-segmented">
              <button
                type="button"
                className="glass-seg-btn"
                aria-pressed={cfgPolarity === "cn-red-up"}
                onClick={() => setCfgPolarity("cn-red-up")}
              >
                A股红涨
              </button>
              <button
                type="button"
                className="glass-seg-btn"
                aria-pressed={cfgPolarity === "intl-green-up"}
                onClick={() => setCfgPolarity("intl-green-up")}
              >
                国际绿涨
              </button>
            </div>
          </div>
        </div>

        <div className="col-span-5 glass-optical-card" style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <div className="tabular-nums" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "6px", fontSize: "12.5px" }}>
            <strong>六维因子权重配比 (%)</strong>
            <span style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span className={Math.round(weightSum) === 100 ? "text-risk-clear" : "text-stoploss"}>
                合计 {weightSum.toFixed(0)}%
              </span>
              {weightSum > 0 && Math.round(weightSum) !== 100 && (
                <button
                  type="button"
                  className="glass-btn glass-btn-ghost glass-btn-sm"
                  style={{ minHeight: "22px", paddingInline: "6px", fontSize: "11px" }}
                  onClick={() => {
                    const entries = pluginConfig.factors.map(
                      (f) =>
                        [
                          f.factorId,
                          Math.round(((weights[f.factorId] ?? f.weightPct) / weightSum) * 100),
                        ] as [string, number]
                    );
                    const sumRounded = entries.reduce((acc, [, v]) => acc + v, 0);
                    if (entries.length > 0 && sumRounded !== 100) {
                      let maxIdx = 0;
                      for (let i = 1; i < entries.length; i += 1) {
                        if (entries[i][1] > entries[maxIdx][1]) maxIdx = i;
                      }
                      entries[maxIdx][1] += 100 - sumRounded;
                    }
                    setWeights(Object.fromEntries(entries));
                  }}
                >
                  归一100%
                </button>
              )}
            </span>
          </div>
          {pluginConfig.factors.map((f) => (
            <label
              key={f.factorId}
              style={{
                display: "grid",
                gridTemplateColumns: "minmax(0, 1fr) 78px",
                alignItems: "center",
                gap: "8px",
                fontSize: "12px",
              }}
            >
              <SafeText text={f.nameZh} />
              <input
                type="number"
                min={0}
                max={100}
                step={2}
                value={weights[f.factorId] ?? f.weightPct}
                onChange={(e) =>
                  setWeights((prev) => ({
                    ...prev,
                    [f.factorId]: Number(e.target.value) || 0,
                  }))
                }
                className="glass-input tabular-nums"
                style={{ minHeight: "30px", paddingBlock: "3px" }}
              />
            </label>
          ))}
        </div>
      </div>

      <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", alignItems: "center", gap: "10px" }}>
        <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
          {statusMsg ?? "同步写入 SQLite 版本快照树与 ./plugins/strategies/smallcap_turnover_ignition.json"}
        </span>
        <AsyncGlassButton
          label="生成新版本快照并热加载生效"
          loadingLabel="生成版本快照..."
          successLabel="新版本已入库生效"
          disabled={weightSum <= 0}
          icon={<SlidersHorizontal size={15} />}
          onClick={async () => {
            await onSavePluginConfig({
              minIntradayProjectedTurnoverPct: projTurnover,
              insiderSellVetoPct: insiderVeto,
              holderSurgeVetoPct: holderVeto,
              mainOutflowVetoRatio: outflowVeto,
              atrStopMult: atrMult,
              marketPolarity: cfgPolarity,
              factorWeights: weights,
            });
            setStatusMsg(`已于 ${new Date().toLocaleTimeString()} 生成新策略版本快照并热加载（权重已归一至 100%）`);
          }}
        />
      </div>
    </section>
  );
};
