# 观望自选胜率前瞻追踪与防过拟合滚动验证迭代引擎设计规范

**日期**：`2026-11-19`  
**状态**：Approved (`autopilot` 自主推进 · 严格遵循 `./quantitative_researcher.txt` 与 `./designer.txt`)  
**关联变更**：`./openspec/changes/tauri-ashare-quant-terminal`

---

## 1. 背景与核心目标

1. **观望加入自选与真实样本外胜率检验**：当用户在选股台或 30 分钟轮动提醒中选择**观望（暂不占用 `100W / <=10` 席位资金）**时，支持一键加入「观望自选观察池」，每次行情扫描自动盯市，按 **A 股 `T+1` 交收 + 双边 `25 bps` 真实摩擦成本** 统计扣费净胜率与 Wilson 95% 置信区间。
2. **防过拟合持续迭代（严禁曲线拟合）**：采用 `60% Train -> 5D Embargo -> 40% OOS` 时间序隔离滚动验证 + 五重防过拟合硬护栏（IS/OOS 符号一致性、PBO/Deflated IC_IR 多重检验惩罚、`|ρ|>0.65` 共线性降权、James-Stein `λ=0.25` 收缩与单步 `<=±3%` 限幅、AST 复杂度惩罚）。
3. **策略与因子全版本控制、老版本并行影子追踪与退化归因（Champion vs Legacy Shadow Arena）**：
   - **全版本快照与自由切换**：从基线 `v1.0.0`、`v1.1.0`、`v1.2.0` 到后续每次升级版本 `v1.2.x`，完整保存不可变的因子权重、自定义因子 AST、排雷与 ATR 止损参数及父子血缘（`parent_version_id`），支持随时**一键切换任意历史或新版本为当前生效版本**；
   - **老版本绝不废弃，持续并行影子盯市（Shadow Tracking）**：升级新版本后，所有历史老版本在后台与每次全市场扫描 / 自选股样本外窗口上**同步计算影子评分、样本外胜率、OOS RankIC 与模拟组合净超额收益**，直观展示「新版本相比老版本到底提升了多少（`Δ胜率`、`ΔRankIC`、`Δ净收益 bps`）」；
   - **新版不如老版时自动四维归因找原因（Root-Cause Attribution）**：一旦检测到新版本在样本外逊于任一老版本（`ΔOOS净收益 < 0` 或 `Δ胜率 < 0` 或 `ΔRankIC < 0`），自动执行精确数学归因分解：
     1. **因子权重迁移拖累分解** $\Delta \alpha_k = (w_k^{\text{new}} - w_k^{\text{old}}) \cdot r_{k,\text{OOS}}$（精准定位哪个被加权因子在当前行情失效、哪个被减权的老因子原本提供了保护）；
     2. **市场风格/波动率体制切换诊断**（区分趋势放量延续期 vs 冲高回落均值回归期导致的因子风格漂移）；
     3. **止损与排雷参数差异归因**（识别 `ATR` 倍数收紧引发的震荡洗盘止损，或风控阈值变动造成的漏网回撤）；
     4. **样本内过拟合衰减检验 + 专家复盘联动**（计算 IS→OOS IC 衰减率，支持一键唤醒 `quant_researcher` 深度复盘或一键切回更优老版本）。

---

## 2. 四层 DDD 模块划分（严守单文件 `< 500` / `<= 600` 行红线）

1. **Domain 层（纯数学与状态机 · 零 I/O）**
   - `./src-tauri/src/domain/watchlist_tracker.rs`：
     - 管理观望自选条目生命周期（`TRACKING` / `HIT_TARGET` / `HIT_STOP` / `SETTLED_5D_WIN` / `SETTLED_5D_LOSS`）；
     - 严格执行 **A 股 `T+1` 交收约束**：纳入当日（`elapsed_trading_days == 0`）仅更新峰值/谷值，绝不允许触发日内 `T+0` 止盈/止损结算，杜绝虚假回测胜率；
     - 严格扣除 **A 股小市值双边 `25 bps` 摩擦成本**（卖出印花税 `5 bps` + 双边佣金 `5 bps` + 小盘冲击滑点 `15 bps`），唯有扣费后净收益率 `net_return_bps > 0` 方计为胜场；
     - 计算 **Wilson 95% 置信区间** `[wilson_low_pct, wilson_high_pct]`、样本容量 $N$、盈亏比（Payoff Ratio）、单笔期望净收益（Expectancy）以及高确信分档（`score >= 68` vs `< 68`）分层胜率验证。
   - `./src-tauri/src/domain/anti_overfit.rs`：
     - **核心策略硬锚点锁定**：`< 200 亿` 总市值 + 连续 2 日换手率 `> 5%` 且放量突破永不篡改；
     - **时间序严格隔离（Purged + Embargoed Walk-Forward Split）**：按交易日严格切分为 `60%` 训练集（Train IS）→ `5` 个交易日隔离带（Embargo，消除 `5D` 前瞻收益标签重叠泄露）→ `40%` 样本外验证集（Test OOS）；
     - **五重防过拟合硬护栏**：
       1. **IS/OOS 符号一致性与衰减门禁**：若因子在训练集与样本外 `RankIC` 符号反转，或样本外复合 `OOS RankIC <= 0.012` / `OOS IC_IR < 0.25`，直接一票否决（`REJECTED_OVERFIT_GUARD`）；
       2. **多重检验惩罚（Deflated IC_IR & PBO 过拟合概率估计）**：对候选评估自由度施加 Bailey-López de Prado 多重检验惩罚，当 `pbo_risk_pct > 35%` 时拒绝晋升；
       3. **共线性冗余惩罚（`|ρ| > 0.65`）**：对高相关同质因子自动削减边际权重，禁止动量/换手重复计分；
       4. **James-Stein 贝叶斯收缩与单步漂移上限（`λ = 0.25`，单步变化 `<= ±3.0%`）**：新权重 $w_i' = \text{clip}((1-\lambda)w_i + \lambda w_i^{\text{OOS}}, w_i - 3\%, w_i + 3\%)$，单因子上下界锁定 `[5.0%, 35.0%]` 并精确归一至 `100.0%`；
       5. **AST 复杂度奥卡姆剃刀与一键回滚**：算子数超限自动降权，所有接受与被拒迭代全量写入审计表并支持一键回滚上一稳定版本。

2. **Infrastructure 层**
   - `./src-tauri/src/infrastructure/sqlite_schema.rs`：幂等迁移新增 `watchlist_items` 与 `factor_evolution_logs` 两张表（兼容既有 `./portable-release/data/economist_quant.db` 零丢数升级）；
   - `./src-tauri/src/infrastructure/sqlite_watchlist_repo.rs`：独立承载观望自选与防过拟合迭代日志持久化，保持 `sqlite_repo.rs` 精简。

3. **Application & Interface 层**
   - `./src-tauri/src/application/evolution_service.rs`：编排每次行情扫描后的自选池实时盯市/胜率结算、盘后/手动防过拟合滚动验证迭代、插件版本递增落盘与一键回滚，并唤醒 `quant_researcher` 会话记录审计；
   - `./src-tauri/src/interface/{dto.rs,commands.rs}`：新增 `add_to_watchlist`、`remove_from_watchlist`、`get_watchlist_tracker`、`run_anti_overfit_iteration`、`rollback_plugin_iteration` 五个强类型 IPC 命令，并将 `watchlist` 与 `evolution` 快照随 `bootstrap_terminal` / `run_market_scan` 同步推送前端。

4. **React 19 Apple Liquid Glass UI 层（严守视口活跃模糊层 `<= 8` 与单文件 `< 500` 行）**
   - `LiveScreenerPage`、`ScreenerInspectorPane`、`RotationAlertToast`：新增「👁 观望加入自选 / 已在自选池」一键操作；
   - `./src/components/watchlist/WatchlistWinRatePanel.tsx`：嵌入模拟交割单页签切换与选股台摘要，直观呈现 `[OOS]` 扣费净胜率、Wilson 95% 置信区间、MFE/MAE、分层胜率及逐笔盯市台账；
   - `./src/components/factors/AntiOverfitIterationPanel.tsx`：嵌入因子插件实验室，展示 `Train vs OOS RankIC`、5 日隔离带、PBO 过拟合风险率、权重收缩前后对比、历史晋升/拦截审计流水，以及「执行防过拟合滚动验证迭代」与「回滚上一稳定版本」按钮。
