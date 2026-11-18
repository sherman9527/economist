# Portable x64 A-Share Quant & Economist Terminal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一套零绝对路径、单文件 `< 1000` 行、构建产物 `< 5 GB`、同级目录自包含的便携式 `x64` **`Tauri 2 + Rust + React 19 / TypeScript + SQLite (bundled)`** A 股单策略（`< 200 亿市值 + 连续 2 日换手率 > 5% 放量 + 五维排雷`）`100W / <=10 只` 模拟盘量化与《经济学人》新闻一体化桌面终端。

**Architecture:** 后端采用严格四层 DDD（`domain` 纯金融模型与 AST 因子引擎、`application` 用例与 30m 交易轮动调度、`infrastructure` 静态编译 SQLite 10 表仓储 + 腾讯/东财 5,576 只全 A 股实时网关 + 插件热加载 + 28d Telemetry + `< 5 GB` 清理器 + `Gemini 4 Carbon` 会话监控、`interface` Tauri IPC）；前端采用三层 Apple Liquid Glass 响应式零文字穿模架构（`<= 8` 模糊层预算、TradingView `lightweight-charts` 买点/止损 K 线 + Apache ECharts 多维图表）。

**Tech Stack:** Rust stable (`x86_64-pc-windows-msvc` via `rsproxy.cn`), Tauri v2, `rusqlite` (`bundled` SQLite C source), `tokio`, `reqwest`, `encoding_rs`, `proptest`, React 19, TypeScript 5, Vite 6, `lightweight-charts` v5, `echarts`, `vitest`, `fast-check`.

**Spec:** `./docs/superpowers/specs/2026-11-18-ashare-quant-terminal-design.md` & `./openspec/changes/tauri-ashare-quant-terminal/`

## Global Constraints

1. **零绝对路径红线（Req #2）**：全仓源码、脚本、配置、SQLite 记录、JSON 与文档严禁出现盘符绝对路径（`[A-Za-z]:\` 或 `[A-Za-z]:/`），一律使用相对路径 `./`。
2. **单文件 `< 1000` 行硬红线（Req #14）**：所有 `.rs`、`.ts`、`.tsx`、`.css`、`.py`、`.ps1` 文件必须严格 `< 1000` 行（目标 `< 450` 行）。
3. **便携式 `x64` 同级存储与 `< 5 GB` 构建上限（Req #7, #8, #9）**：禁用安装包（`bundle.active = false`）；运行期强制绑定 `./webview_data`、`./.temp`、`./data`、`./config`、`./plugins`、`./reports`、`./logs`、`./telemetry`；构建缓存严控 `< 5.0 GB`。
4. **模型与会话持久化红线（Req #11, #13）**：主/子 Agent 统一锁定 `gemini-4-carbon` + `high` 推理强度，所有 `sessionId` 持久化于 `./data/economist_quant.db` 的 `agent_sessions` 表。
5. **中国镜像源优先**：Cargo 使用 `https://rsproxy.cn` 稀疏索引（TUNA 备源），NPM 使用华为云镜像 `https://mirrors.huaweicloud.com/repository/npm/`（腾讯云备源），SQLite 通过 `rusqlite` `bundled` 本地编译 C 源码，MSVC 通过 `vswhere.exe` 动态定位。

---

### Task 1: 中国镜像源工具链引导、构建体积守卫与历史脚本无损重构

**Files:**
- Create: `./.cargo/config.toml`
- Create: `./.npmrc`
- Create: `./scripts/env_msvc.ps1`
- Create: `./scripts/clean_build_cap.ps1`
- Create: `./scripts/bootstrap_sqlite_sessions.py`
- Modify: `./scripts/config.py`
- Modify: `./scripts/ack_server.py`
- Modify: `./scripts/run_daily.ps1`
- Modify: `./scripts/register_tasks.ps1`
- Create: `./scripts/renderer_pkg/__init__.py` (+ 拆分子模块 `styles.py`, `charts_svg.py`, `sections.py`, `shell.py`, `index_page.py`，每个 `< 650` 行)
- Modify: `./scripts/renderer.py` (精简为 `< 120` 行的兼容门面)
- Test: `./scripts/test_repo_hygiene.py`

**Interfaces:**
- Produces: 本地 `./.toolchain/` 下的 `rustc` / `cargo` (`x86_64-pc-windows-msvc`)、动态 MSVC 环境加载脚本 `./scripts/env_msvc.ps1`、`< 5 GB` 清理脚本 `./scripts/clean_build_cap.ps1`、初始化含真实主/子 `sessionId` 的 `./data/economist_quant.db`，以及满足“零绝对路径 + `< 1000` 行”的既有 Python 流水线。

- [ ] **Step 1: 编写全仓零绝对路径与 `< 1000` 行合规测试 (`./scripts/test_repo_hygiene.py`) 并确认当前历史脚本触发失败**
  - 检查 `./scripts/` 与 `./data/ack_state.json` 是否存在盘符绝对路径及 `>= 1000` 行文件（预期检出 `scripts/renderer.py` 3,492 行与 `scripts/config.py` 等绝对路径）。
- [ ] **Step 2: 重构 `./scripts/*` 消除全部绝对路径，将 `COPILOT_MODEL` 升级为 `gemini-4-carbon`，并将 `./scripts/renderer.py` 拆分为 `./scripts/renderer_pkg/` 模块**
  - 使用 `Path(__file__).resolve().parent.parent` 与 `$PSScriptRoot` 替代硬编码路径；清理 `./data/ack_state.json` 中的 `abs_path` 字段；按样式、图表、章节卡片、总控页拆分 `renderer.py`。
- [ ] **Step 3: 配置 `./.cargo/config.toml`（`rsproxy.cn` 稀疏索引）与 `./.npmrc`（华为云 NPM 镜像），通过 `rsproxy.cn` 安装便携 Rust 工具链至 `./.toolchain/`，并编写 `./scripts/clean_build_cap.ps1` 与 `./scripts/bootstrap_sqlite_sessions.py`**
  - 验证 `rustc -V`、`cargo -V` 在动态发现的 VS2022 MSVC 环境下正常编译，并写入主/子 Agent `sessionId` 到 `./data/economist_quant.db`。
- [ ] **Step 4: 运行 `./scripts/test_repo_hygiene.py` 与原报告渲染回归校验**
  - 预期：`0` 绝对路径、全仓最大单文件 `< 1000` 行、`agent_sessions` 表含 4 条锁定 `gemini-4-carbon` (`high`) 的持久化会话记录。

---

### Task 2: Tauri 2 + Rust `domain` 领域核心层实现（TDD + `proptest` PBT）

**Files:**
- Create: `./src-tauri/Cargo.toml`
- Create: `./src-tauri/build.rs`
- Create: `./src-tauri/tauri.conf.json`
- Create: `./src-tauri/src/lib.rs`
- Create: `./src-tauri/src/main.rs`
- Create: `./src-tauri/src/domain/mod.rs`
- Create: `./src-tauri/src/domain/types.rs`
- Create: `./src-tauri/src/domain/ashare_rules.rs`
- Create: `./src-tauri/src/domain/strategy_gate.rs`
- Create: `./src-tauri/src/domain/pitfall_veto.rs`
- Create: `./src-tauri/src/domain/factor_engine.rs`
- Create: `./src-tauri/src/domain/portfolio_ledger.rs`

**Interfaces:**
- Produces:
  - `Fen(i64)`、`Bps(i32)`、`Board::{Main, ChiNext, Star, Bse}`、`compute_price_limits`、`validate_lot_size`、`compute_trade_fees`
  - `CoreStrategyGate::evaluate(...)`（硬锁 `< 200 亿` 总市值 + 连续 2 日换手率 `> 5%` 放量 + 盘中 U 型分时折算）
  - `PitfallVetoEngine::audit(...)`（减持、负面舆情/公告、股东户数环比、主力净流出、技术破位五维审计）
  - `FactorEngine::score_cross_section(...)`（声明式因子计算、`3*MAD` 去极值、行业+对数市值中性化、ATR 初始止损与吊灯移动止损）
  - `PortfolioLedger`（`100_000_000` 分即 `100W` 初始现金、`<= 10` 持仓席位、`T+1` 锁仓与次日解锁、买单强制携带 `rationale` 与 `stop_loss_price_fen < entry_price_fen`）

- [ ] **Step 1: 先编写 `domain` 层单元测试与 `proptest` 属性测试**
  - 覆盖：1) 任意买卖序列下 `cash + market_value == total_equity` 且 `open_positions <= 10`；2) 同日买入份额 `sellable_qty == 0`（`T+1` 铁律）；3) 市值 `>= 200 亿` 或任一日换手 `<= 5%` 必被策略门禁拒绝；4) 初始止损价严格落在 `[-7.5%, -4.0%]` 且 `< 入场价`。
- [ ] **Step 2: 实现纯 Rust `domain` 六个子模块（单文件 `< 400` 行，零外部 I/O）**
- [ ] **Step 3: 通过 `./scripts/env_msvc.ps1` 运行 `cargo test --lib domain`**
  - 预期：全部领域单元测试与百组随机样本 `proptest` 属性测试 `PASS`。

---

### Task 3: Rust `infrastructure` 基础设施层（Bundled SQLite、5576 全 A 股实时行情、插件、新闻与监控清理）

**Files:**
- Create: `./config/app.toml`
- Create: `./plugins/strategies/smallcap_turnover_ignition.json`
- Create: `./plugins/factors/core_factors_v1.json`
- Create: `./src-tauri/src/infrastructure/mod.rs`
- Create: `./src-tauri/src/infrastructure/portable_paths.rs`
- Create: `./src-tauri/src/infrastructure/sqlite_schema.rs`
- Create: `./src-tauri/src/infrastructure/sqlite_repo.rs`
- Create: `./src-tauri/src/infrastructure/market_tencent.rs`
- Create: `./src-tauri/src/infrastructure/market_eastmoney.rs`
- Create: `./src-tauri/src/infrastructure/plugin_store.rs`
- Create: `./src-tauri/src/infrastructure/economist_bridge.rs`
- Create: `./src-tauri/src/infrastructure/telemetry_cleaner.rs`
- Create: `./src-tauri/src/infrastructure/agent_supervisor.rs`

**Interfaces:**
- Consumes: `domain::*` 值对象、规约与组合聚合根。
- Produces:
  - `PortablePaths`（同级 `./config`、`./data`、`./plugins`、`./reports`、`./logs`、`./telemetry`、`./webview_data`、`./.temp`，相对路径标准化器）
  - `SqliteRepository`（10 张表 CRUD 与事务落盘）
  - `HybridMarketGateway`（腾讯 `qt.gtimg.cn` `5,500+` 全 A 股批量实时快照 + 东财 `push2his` 日线/30m K 线及 `datacenter-web` 减持/股东数 `HOLD_NOTICE_DATE`/公告五维排雷查询）
  - `PluginStore`（热加载/校验更新 `./plugins/**.json` 与 `./config/app.toml`）
  - `EconomistBridge`（同步 `./data/ack_state.json`、`./data/runs/analysis_*.json`、`./data/backtest_ledger.json` 至 SQLite 与前端 DTO，提取负面行业信号供排雷联动）
  - `OpsGuardian`（全链路 Telemetry 双写、`28d` 日志/事件清理、`< 5 GB` 构建目录分级清理、`gemini-4-carbon` 子 Agent `sessionId` 唤醒器）

- [ ] **Step 1: 编写 SQLite 迁移、相对路径约束、28d 过期清理、插件热加载与行情解析测试**
- [ ] **Step 2: 实现上述基础设施模块与默认配置/插件 JSON 清单（每个文件 `< 450` 行）**
- [ ] **Step 3: 运行 `cargo test infrastructure` 并执行实时外部 A 股行情连通性集成验证**
  - 预期：Bundled SQLite 零外部 DLL 通过全部事务测试，实时快照成功解析 `5,000+` 只活跃 A 股。

---

### Task 4: Rust `application` 用例层、30 分钟交易轮动调度、`interface` IPC 与架构红线门禁测试

**Files:**
- Create: `./src-tauri/src/application/mod.rs`
- Create: `./src-tauri/src/application/terminal_service.rs`
- Create: `./src-tauri/src/application/rotation_scheduler.rs`
- Create: `./src-tauri/src/interface/mod.rs`
- Create: `./src-tauri/src/interface/dto.rs`
- Create: `./src-tauri/src/interface/commands.rs`
- Test: `./src-tauri/tests/architecture_redlines.rs`

**Interfaces:**
- Produces Tauri Commands:
  - `bootstrap_terminal_state()`（冷启动实时拉取外部全市场快照 + 本地 SQLite 组合/订单/推荐/插件/新闻/会话一站式快照）
  - `run_full_market_scan(force_refresh: bool)`（执行 `5,500+` 全 A 股 `< 200 亿 + 双 5% 放量换手 + 五维排雷 + 6 因子中性化打分`，自动更新推荐与模拟盘信号）
  - `get_stock_kline_and_signals(symbol: String, period: String)`（返回实时日线/30m K 线 + 买点锚点列表 + 止损价格线 + 五维排雷详情）
  - `execute_simulated_order(req: OrderRequestDto)` / `close_simulated_position(symbol: String, reason: String)`
  - `get_plugins_and_config()` / `update_plugin_config(payload: PluginUpdateDto)`
  - `get_economist_module_data()` / `ack_economist_report(report_id: String, acked: bool)` / `read_economist_report_html(rel_path: String)`
  - `get_telemetry_and_sessions()` / `wake_expert_agent(role: String, task_prompt: String)` / `enforce_storage_hygiene()`

- [ ] **Step 1: 编写 `./src-tauri/tests/architecture_redlines.rs` 强制验证全仓零绝对路径、单文件 `< 1000` 行与 30m 交易时段窗口判定**
- [ ] **Step 2: 实现 `TerminalService`、`RotationScheduler`（交易日 `09:30–11:30, 13:00–15:00` 每 30 分钟轮动并发射 `quant://rotation-alert`）与 Tauri Commands**
- [ ] **Step 3: 运行 `cargo test` 验证后端全套单元、属性与架构红线测试通过**

---

### Task 5: React 19 + TypeScript Apple Liquid Glass 六页前端与 `vitest`/`fast-check` PBT

**Files:**
- Create: `./package.json`, `./tsconfig.json`, `./vite.config.ts`, `./index.html`
- Create: `./src/styles/{tokens.css, typography.css, glass-surfaces.css}`
- Create: `./src/types/terminal.ts`
- Create: `./src/domain/uiInvariants.ts`
- Create: `./src/services/tauriClient.ts`
- Create: `./src/components/primitives/{MeshBackdrop.tsx, GlassPanel.tsx, GlassButton.tsx, GlassSegmentedControl.tsx, SafeText.tsx, MetricDeltaChip.tsx}`
- Create: `./src/components/shell/{AppSidebar.tsx, TopStatusBar.tsx, RotationAlertToast.tsx}`
- Create: `./src/components/charts/{echartsSetup.ts, QuantKlineTerminal.tsx, EquityCurveChart.tsx, AllocationDonutChart.tsx, FactorColumnChart.tsx}`
- Create: `./src/pages/{CommandDashboardPage.tsx, LiveScreenerPage.tsx, OrdersLedgerPage.tsx, FactorPluginLabPage.tsx, EconomistNewsPage.tsx, TelemetrySessionsPage.tsx}`
- Create: `./src/{App.tsx, main.tsx}`
- Test: `./src/__tests__/terminalInvariants.pbt.test.ts`

**Interfaces:**
- Consumes: Task 4 定义的全部 Tauri v2 Commands 与 `quant://rotation-alert` 事件（并内置浏览器预览态降级适配器便于独立开发调试）。
- Produces: 响应式兼容 `390px–2560px`、零文字穿模、单屏 `<= 8` 模糊层、支持深浅双主题与红涨绿跌切换的六页 Liquid Glass 量化投研工作台。

- [ ] **Step 1: 通过华为云 NPM 镜像安装前端依赖，先行编写 `./src/__tests__/terminalInvariants.pbt.test.ts`（`fast-check` 属性测试校验极值文本零穿模规则、A 股手数与涨跌停价格计算、`<= 10` 席位上限与 `<= 8` 模糊槽位预算）**
- [ ] **Step 2: 实现三层 Liquid Glass 样式令牌、防穿模基础原语与 K 线（含买点箭头+琥珀止损线+理由浮层）/ ECharts 图表组件**
- [ ] **Step 3: 实现六大功能页面与交易时段 30 分钟轮动推荐弹窗 `RotationAlertToast`**
- [ ] **Step 4: 运行 `npm test`（`vitest run`）与 `npm run build`（`tsc --noEmit && vite build`），确认零类型错误与全部属性测试通过**

---

### Task 6: 便携式 `x64` `.exe` 编译打包、5,576 只全 A 股实盘链路验收与 Git 提交

**Files:**
- Create: `./scripts/stage_portable_release.ps1`
- Modify: `./README.md`, `./memo.md`, `./rule.md`, `./handover.md`, `./openspec/changes/tauri-ashare-quant-terminal/tasks.md`

- [ ] **Step 1: 编译产出 `x86_64-pc-windows-msvc` Release 便携 `.exe`，通过 `./scripts/stage_portable_release.ps1` 将可执行文件与同级 `./config`、`./plugins`、`./data`、`./reports`、`./logs`、`./telemetry` 组装就位**
- [ ] **Step 2: 运行实机冒烟与全链路验收测试（真实拉取 `5,500+` 只活跃 A 股快照、执行 `< 200 亿 + 双 5% 换手放量 + 五维排雷` 扫描、生成带买点与止损价的推荐及 `100W` 模拟盘订单写入 SQLite、校验 WebView2 同级缓存与进程健康启动）**
- [ ] **Step 3: 执行 `./scripts/clean_build_cap.ps1` 确保构建缓存 `< 5.0 GB`，重跑 `cargo test` 全仓红线扫描（确认 `0` 绝对路径、所有文件 `< 1000` 行）**
- [ ] **Step 4: 同步更新 `./memo.md`、`./handover.md`、OpenSpec `tasks.md` 并创建原子 Git 提交**
