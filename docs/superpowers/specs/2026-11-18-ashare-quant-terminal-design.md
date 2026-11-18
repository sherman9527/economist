# 便携式 `x64` A 股单策略量化与《经济学人》投研一体化终端架构设计规范（Superpowers Spec）

- **日期**：`2026-11-18`
- **关联 OpenSpec 变更**：`./openspec/changes/tauri-ashare-quant-terminal/`
- **协同设计专家 Agent（统一锁定 `gemini-4-carbon` · `high`）**：
  - `ui_designer` (`cbdcae76-1934-41ac-8160-43be116e5b9b`) — Apple Liquid Glass 视觉、防穿模与金融图表架构
  - `quant_researcher` (`88ace910-c427-41db-a87f-46d8ae39735b`) — 5,576 只全 A 股行情实测、唯一核心策略、五维排雷与 100W 模拟盘数学规范
  - `risk_auditor` / `ddd_architect` (`b075d939-e095-482e-8424-4aba1882b97c`) — Tauri 2 + Rust + SQLite (`bundled`) DDD 四层架构、中国镜像源与红线守卫

---

## 1. 25 项核心需求全量映射矩阵（`./requirement.txt`）

| 编号 | 核心需求摘要 | 架构落地方案 | 验收门禁 |
| :--- | :--- | :--- | :--- |
| #1 | Git 版本管理 | 全程基于本地 Git 主干分阶段原子提交 | `git status` 干净、提交历史清晰 |
| #2 | 全仓严禁绝对路径 | `PortablePaths` 基于 `current_exe()` 运行期瞬态解析，落盘/源码/脚本/文档 100% 相对路径 `./` | Rust 自动化红线测试扫描全仓 `0` 匹配 |
| #3 | 根目录 `./memo.md` 开发记忆（每次 Compact 更新） | 记录架构快照、镜像源、策略版本、会话表与 Compact 日志 | 文件就位且随里程碑实时更新 |
| #4 | 根目录 `./rule.md` 避坑指南 | 沉淀 12 项实测避坑规则（镜像 TLS/403、MSVC 动态发现、东财 `pz=100`、`HOLD_NOTICE_DATE` 等） | 每次踩坑修复必补录 |
| #5 | 根目录 `./handover.md` 进度交接 | 清晰维护已完成、进行中与待办清单 | 交付前全量勾选核对 |
| #6 | `Tauri 2 + Rust + React/TS + SQLite` | Tauri v2 + Rust 2021 + React 19 / TypeScript 5 + `rusqlite` (`bundled` C 源码编译) | 端到端类型安全编译通过 |
| #7 | 定期清理构建产物 `< 5 GB` | `Cargo.toml` `debug = 0` + `./scripts/clean_build_cap.ps1` + Rust 后台三级水位清理器（软 `4 GB` / 硬 `5 GB`） | 构建前后实测占用 `< 5.0 GB` |
| #8 | 便携式 `x64`（不要安装包） | `x86_64-pc-windows-msvc` 目标，`tauri.conf.json` 设置 `"bundle": { "active": false }` | 独立 `.exe` 直接启动 |
| #9 | 运行输出全部在 `.exe` 同级目录 | `./config`、`./data`、`./plugins`、`./reports`、`./logs`、`./telemetry`、`./webview_data`、`./.temp` | 零系统盘写入 |
| #10 | 多尺寸兼容、零文字穿模、风格字体统一 + `designer` Agent | `./.github/agents/designer.agent.md` + 三层 Liquid Glass + 禁用固定文本高 + `min-width: 0` + `tabular-nums` | 多断点布局契约 PBT 测试通过 |
| #11 | 主/子 Agent `sessionId` 存 SQLite 供原上下文唤醒 | SQLite `agent_sessions` 表持久化主/子 `sessionId`，通过 `--resume <session_id>` 恢复 | SQLite 实表查询校验 |
| #12 | 创建 `quantitative_researcher` 专家 Agent | `./.github/agents/quantitative-researcher.agent.md`，严格执行 A 股 `T+1`/涨跌停/扣费/中性化协议 | 规范落盘并接入迭代闭环 |
| #13 | 主/子 Agent 锁定 `Gemini 4 Carbon` + `high` | Agent Frontmatter、SQLite `CHECK` 约束与 Rust 唤醒器全部硬锁定 `gemini-4-carbon` + `high` | 数据库约束与单元测试双重锁定 |
| #14 | `DDD + TDD + PBT`，单文件 `< 1000` 行硬红线 | 四层 DDD + `proptest` + `vitest`/`fast-check`；拆分旧 `renderer.py`，所有新文件控制在 `60–450` 行 | 自动化行数红线测试断言 `< 1000` |
| #15 | 插件化与参数化配置驱动 | `./config/*.toml` + `./plugins/strategies/*.json` + `./plugins/factors/*.json` 热加载 | 修改 JSON 即生效，零改码 |
| #16 | 记录量化推荐、K 线标买点与止损、模拟订单存 SQLite | `recommendations` + `simulated_orders` 存 `rationale` 与 `stop_loss_price_fen`，`QuantKlineTerminal` 直绘 | K 线买点徽章+琥珀止损线联动 |
| #17 | 外部稳定实时行情 vs 本地 SQLite 实时读取 | 启动从腾讯 `qt.gtimg.cn` + 东财实时拉取；订单/持仓/因子/会话实时查本地 SQLite | 冷启动抓包与数据库实测 |
| #18 | 全链路 Telemetry 监控 + 日志保留 `28d` | JSONL + SQLite `telemetry_events` 双写，每 6 小时自动清理 `> 28` 天数据 | 28 天过期分片自动回收测试 |
| #19 | 因子与插件由量化专家持续迭代添加 | 安全因子表达式 AST + OOS 扣费 `Rank_IC` / `Calmar` 晋升门禁 | 插件新增与评估流水线完备 |
| #20 | 初始资金 `100W`、最多买 `10` 只股票 | `initial_cash_fen = 100_000_000`，`positions.slot_no BETWEEN 1 AND 10` 唯一索引 + 领域不变量 | `proptest` 随机交易序列不破界 |
| #21 | 唯一策略：`< 200 亿市值 + 连续 2 日换手 > 5% 放量` + 五维排雷 | 硬锁 `< 200 亿` 与双日 `> 5%` 换手放量；排雷覆盖减持/舆情/股东数/主力流/破位 | 策略规约与排雷否决测试 |
| #22 | 股票池 `5,000+` 全 A 股 | 沪深北全量活跃 A 股（实测 `5,576` 只）全覆盖扫描 | 实时扫描返回 `> 5,000` 只有效标的 |
| #23 | 既有《经济学人》整合进「新闻板块」 | 第 5 页原生集成四阶传导、未读 ACK 队列、回溯审计与相对路径原版研报阅读器 | 页内浏览与一键 ACK 双向同步 |
| #24 | 主控全程监控自愈，空闲驱动量化 Agent 收益优先迭代 | Rust `SupervisorMonitor` 巡检 API/DB/IC 衰减，异常自愈，正常时调度 OOS 因子优化 | 监控面板实时展示健康度与会话 |
| #25 | 交易时间每 `30` 分钟轮动，命中推荐即时弹窗提醒 | 周一至周五 `09:30–11:30, 13:00–15:00` 30m 调度发射事件，右上角弹出 Tier 3 玻璃推荐卡 | 轮动调度器与弹窗组件联动验证 |

---

## 2. 系统分层架构（DDD 四层边界）

```text
+-----------------------------------------------------------------------------------+
| Presentation Layer (React 19 + TypeScript + Apple Liquid Glass, <= 8 Blur Slots)  |
|  [P1 组合与K线总览] [P2 5500+全A轮动排雷] [P3 100W订单持仓] [P4 因子与插件中心]    |
|  [P5 经济学人新闻板块] [P6 全链路Telemetry与Agent会话] + [30m 轮动玻璃弹窗]        |
+-----------------------------------------+-----------------------------------------+
                                          | Tauri v2 Typed IPC Commands & Events
+-----------------------------------------v-----------------------------------------+
| Interface Layer (src-tauri/src/interface/)                                        |
|  commands/{market_cmd, portfolio_cmd, plugin_cmd, news_cmd, telemetry_cmd}.rs     |
+-----------------------------------------+-----------------------------------------+
                                          |
+-----------------------------------------v-----------------------------------------+
| Application Layer (src-tauri/src/application/)                                    |
|  ScanUniverseService · RotationScheduler (30m CST) · OrderExecutionService        |
|  PluginHotReloadService · EconomistSyncService · SupervisorSelfHealService        |
+-----------------------------------------+-----------------------------------------+
                                          | Trait Ports (Repository / Gateway)
+-----------------------------------------v-----------------------------------------+
| Domain Layer (src-tauri/src/domain/ — Pure Rust, Zero I/O, TDD + proptest PBT)    |
|  types (Fen, Bps, Symbol, Board) · strategy (Cap<200亿 & 2D Turnover>5% VolUp)    |
|  pitfall (5D Veto Spec) · factor_ast (Safe Expr + Industry/Cap Neutralize + MAD)  |
|  portfolio (100W Cash, <=10 Slots, T+1 Lock, StampDuty/Comm/Slippage, ATR Stop)   |
+-----------------------------------------+-----------------------------------------+
                                          ^ Implements Ports
+-----------------------------------------+-----------------------------------------+
| Infrastructure Layer (src-tauri/src/infrastructure/)                              |
|  persistence (rusqlite bundled WAL 10 tables) · market (Tencent Batch + EastMoney)|
|  plugins (TOML/JSON loader) · agents (Gemini 4 Carbon high session runner)        |
|  news (Relative Economist indexer) · ops (28d Reaper + <5GB Build Cleaner)        |
+-----------------------------------------------------------------------------------+
```

---

## 3. 核心金融数学与风控模型摘要

1. **核心入池硬门槛**：
   - 剔除 `ST/*ST/退`、停牌、上市交易日 `< 60` 天、当日涨停锁死（距涨停价 `< 0.2%`）；
   - 总市值 $MV_{\text{total}} < 200\times 10^8\text{ RMB}$；
   - 连续放量高换手：$TR_{T-1} > 5.0\%$ 且 $V_{T-1} \ge 1.15 \cdot \text{MA}_{20}(V)_{T-1}$；收盘模式要求 $TR_T > 5.0\%$ 且 $V_T \ge 1.02 \cdot V_{T-1}$，盘中第 $\tau \in [30, 240]$ 分钟按 U 型分时累计成交权重 $w(\tau)$ 折算预测换手 $\widehat{TR}_T = TR_{T,\tau}/w(\tau) \ge 5.5\%$ 且已实现 $TR_{T,\tau} \ge 2.5\%$。
2. **五维排雷一票否决（`PitfallVetoEngine`）**：
   - 大股东/高管近 90 日净减持占流通盘 `>= 0.5%` → 否决；
   - 近 60 日公告含 `立案|留置|行政处罚|退市风险|非标意见|违规担保` 或命中《经济学人》强利空行业警示 → 否决；
   - 最新披露日（`HOLD_NOTICE_DATE`）股东户数环比增幅 `> +25%` → 否决（`+10%~+25%` 扣分）；
   - 当日主力净流出占成交额 `< -8%` → 否决；
   - 现价跌破 `MA20` 或长上影线比例 `(High - max(Open,Close)) / (High - Low) > 0.55` 且滞涨 → 否决。
3. **6 因子截面中性化合成与仓位止损**：
   - 因子集：`F1` 理想反转、`F2` 凹向异常换手、`F3` Amihud 非流动性压缩、`F4` 日内尾盘 VWAP 动量、`F5` 主力净流入信息比、`F6` 量价效率；经 `3*MAD` 去极值与行业/对数市值中性化后按插件权重线性合成 `conviction_score (0..100)`。
   - 初始硬止损：$P_{\text{stop},0} = \text{clip}(\min(L_{T-1}, P_{\text{exec}} - 2.0\cdot \text{ATR}_{14}), 0.925 P_{\text{exec}}, 0.960 P_{\text{exec}})$；浮盈 $\ge +6\%$ 后启用吊灯移动止损。
   - 仓位上限：单票名义本金 $\le 10\% \cdot \text{NAV}$、单笔止损风险敞口 $\le 0.8\% \cdot \text{NAV}$、单日参与率 $\le 1.5\% \cdot \text{ADV}_{20d}$，总持仓席位硬上限 `10` 只，严格 `T+1` 次日解锁可卖。
