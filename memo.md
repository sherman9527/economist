# 项目开发记忆中枢 (`memo.md`)

> **维护契约（Requirement #2 & #3）**：本文件为项目跨会话与每次上下文压缩（Compact）的持久化记忆源；每次发生 Compact 或阶段里程碑切换时必须同步更新。全仓（含本文档）严格禁止出现任何盘符绝对路径，统一使用相对仓库根目录或便携可执行目录（`./`）的相对路径。

---

## 1. 当前架构北极星与核心约束快照

- **目标形态**：基于 **Tauri 2 + Rust + React 19 / TypeScript + SQLite (`rusqlite` `bundled` 源码编译)** 的便携式 `x64`（`x86_64-pc-windows-msvc`，无安装包 `bundle.active = false`）A股量化交易与《经济学人》全球政经投研终端。
- **便携自包含布局**：运行期所有配置、数据库、插件、日志、Telemetry、研报归档与 WebView2 用户目录均严格落盘在可执行文件同级目录（`./config/`、`./data/`、`./plugins/`、`./logs/`、`./telemetry/`、`./reports/`、`./webview_data/`、`./.temp/`），零系统盘污染，零绝对路径。
- **工程红线**：
  1. **DDD + TDD + PBT**：按领域驱动设计划分 `domain` / `application` / `infrastructure` / `interface` 四层；Rust 端采用 `cargo test` + `proptest`，前端采用 `vitest` + `fast-check`。
  2. **单文件 `< 1000` 行硬红线**：全仓任何源码、脚本、样式与配置文件严禁超过 1000 行（由自动化架构红线测试强制拦截）。
  3. **构建产物 `< 5 GB` 上限**：Cargo 配置 `debug = 0` + 增量裁剪，配合构建前后与后台定时水位清理器（软水位 `4.0 GB`，硬上限 `5.0 GB`）。
  4. **全链路 Telemetry 与 28 天日志保留**：结构化 JSONL + SQLite 双写，自动滚动清理超过 `28d` 的历史日志与 Telemetry 事件。
  5. **模型锁定**：主控 Agent 与所有专家子 Agent 统一锁定 **`Gemini 4 Carbon` (`gemini-4-carbon`) + `high` 推理强度**，并将主/子 `sessionId` 持久化在 SQLite `agent_sessions` 表中以便随时通过 `--resume` / `--session-id` 唤醒原上下文。

---

## 2. 唯一核心量化策略与风控契约（100W 初始资金 / `<= 10` 只持仓）

- **本金与仓位**：初始模拟盘本金 `1,000,000.00 RMB`（SQLite 中以精确整数“分” `100_000_000` 存储，杜绝浮点误差），最多同时持有 `10` 只股票（`100` 股整手，科创板 `>= 200` 股）。
- **唯一迭代主策略（`smallcap_turnover_ignition`）**：
  - **股票池**：全市场 `5,000+` 只活跃 A 股（实测 `5,576` 只，剔除 `ST/*ST/退市整理`、停牌、上市未满 `60` 个交易日次新股、涨跌停锁死标的）。
  - **核心硬门槛**：**总市值 `< 200 亿 RMB`** 且 **连续 2 个交易日换手率 `> 5%` 并伴随放量**（盘中 30 分钟轮动采用 A 股 U 型分时成交分布折算预测换手率 `>= 5.5%` 且已实现换手率 `>= 2.5%`）。
  - **五维智能排雷（由量化专家 Agent 持续插件化迭代）**：
    1. 大股东/高管近 90 日净减持与大额解禁压制；
    2. 立案调查/行政处罚/问询函及《经济学人》宏观行业负面冲击；
    3. 最新一期股东户数环比激增（筹码发散）；
    4. 主力资金净流出超成交额阈值；
    5. 技术形态破位（收盘跌破 `MA20` 或长上影诱多天量滞涨）。
  - **插件化 6 因子截面打分（经申万行业 + 对数流通市值正交中性化与 MAD 去极值）**：理想反转 `F1`、凹向异常换手 `F2`、Amihud 非流动性压缩 `F3`、日内 APM 与尾盘 VWAP 偏离 `F4`、主力净流入信息比 `F5`、量价效率 `F6`。
  - **交易制度与止损闭环**：严格执行 A 股 `T+1` 锁仓解锁、主板 `±10%` / 创科 `±20%` / 北交 `±30%` 涨跌停校验、卖出 `0.05%` 印花税 + 佣金（最低 `5` 元）+ `15 bps` 小盘冲击滑点；每笔推荐与模拟订单均记录**推荐理由（`rationale`）**与**初始硬止损价（`stop_loss_price`）**，并在应用内 K 线图直绘买点标记与止损水平线。
  - **盘中 30 分钟轮动弹窗**：交易日 `09:30–11:30`、`13:00–15:00` 每 30 分钟自动扫描全市场，命中标的即时弹出非阻塞式玻璃态推荐提醒。

---

## 3. 已验证中国镜像源与本机编译链路记忆

- **Rustup / Cargo 中国镜像**：采用字节跳动 `rsproxy.cn`（`RUSTUP_DIST_SERVER=https://rsproxy.cn`、`RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup`、稀疏索引 `sparse+https://rsproxy.cn/index/`，实测 `633 ms` `200 OK`），清华 TUNA 作为备用；**禁用中科大 USTC 镜像（实测返回 `403 Forbidden`）**。
- **NPM 中国镜像**：本机网络环境下 `registry.npmmirror.com` 与官方源均存在 TLS 重置；经实测选用 **华为云 NPM 镜像（`https://mirrors.huaweicloud.com/repository/npm/`，`950 ms` `200 OK`）** 为主源，**腾讯云 NPM 镜像（`https://mirrors.cloud.tencent.com/npm/`）** 为备源。
- **MSVC x64 编译环境**：必须通过 `vswhere.exe -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64` 动态定位完整安装有 `VC\Auxiliary\Build\vcvars64.bat` 的 VS2022 工具链（避免误选仅含 Redistributable 的不完整 VS 目录），SQLite 通过 `rusqlite` `bundled` 特性直接从 C 源码编译静态链接进便携 `x64` `.exe`。

---

## 4. 持久化主/子 Agent 会话注册表快照（同步存储于 `./data/economist_quant.db`）

| 角色标识 (`agent_role`) | 持久化 `sessionId` | 模型与推理配置 | 规格相对路径 |
| :--- | :--- | :--- | :--- |
| `main_supervisor` | `0626dc52-8683-4d1c-befc-16509212d024` | `gemini-4-carbon` · `high` | `./memo.md` |
| `ui_designer` | `cbdcae76-1934-41ac-8160-43be116e5b9b` | `gemini-4-carbon` · `high` | `./.github/agents/designer.agent.md` |
| `quant_researcher` | `88ace910-c427-41db-a87f-46d8ae39735b` | `gemini-4-carbon` · `high` | `./.github/agents/quantitative-researcher.agent.md` |
| `risk_auditor` (`ddd_architect`) | `b075d939-e095-482e-8424-4aba1882b97c` | `gemini-4-carbon` · `high` | `./.github/agents/ddd-tauri-architect.agent.md` |

---

## 5. Compact 更新日志

- **Compact #1 (`2026-11-18` · Checkpoint: `Multi-expert OpenSpec and Superpowers architecture design`)**：
  - 完成《经济学人》单窗口页内 Pivot 工作台改造并提交历史基线；
  - 初始化 OpenSpec `1.4.1`，加载 Superpowers 与 OpenSpec 技能组；
  - 并行调度 3 名 `gemini-4-carbon` (`high`) 专家子 Agent 完成 Apple Liquid Glass 视觉防穿模规范、5,576 只全 A 股实时行情/排雷 API 实测与单策略多因子数学建模、Tauri 2 DDD 四层架构与中国镜像源实测；
  - 当前进入治理文档落盘、OpenSpec 四大标准产物（`proposal` / `specs` / `design` / `tasks`）严格校验、Superpowers 实施计划定稿及便携 `x64` 应用 TDD+PBT 全量构建阶段。

- **Compact #2 (`2026-11-18` · Checkpoint: `OpenSpec design, China toolchain, and hygiene refactor`)**：
  - 完成 OpenSpec `tauri-ashare-quant-terminal` 严格校验（`4/4` 产物通过）、Superpowers 设计与计划文档、中国镜像便携 Rust `1.100.0` (`x86_64-pc-windows-msvc`) 工具链落盘及全仓遗留脚本零绝对路径 / `< 1000` 行重构（提交 `0710865`）；
  - 并行调度 2 名 `gemini-4-carbon` (`high`) 专项实现 Agent 完成 `./config/**` + `./plugins/**` + `./src-tauri/**`（4 层 DDD + Bundled SQLite + 5,500+ 全 A 股实时扫描 + 100W/10 席位 T+1 账本 + `proptest` PBT）与 `./src/**`（React 19 + TypeScript Apple Liquid Glass 六页工作台 + `lightweight-charts` 买点/止损线 K 线 + ECharts + `vitest`/`fast-check` PBT）；
  - 当前进入便携 `x64` 独立 `.exe`（`bundle.active = false`）联调打包、构建体积 `< 5 GB` 水位巡检、**5 轮深度专家 Code Review 与缺陷清零闭环**及最终 `git push origin main` 阶段。

- **Compact #3 (`2026-11-18` · Checkpoint: `Portable Tauri terminal built and Round 1 deep review`)**：
  - 完成全栈联调、解决 Pitfall #13（直接通过 Cargo 构建便携式 `.exe` 必须显式启用 `[features] default = ["custom-protocol"]` 以内联前端 `frontendDist` 静态资产，已沉淀至 `./rule.md`），打包验证 `./portable-release/economist-quant-terminal.exe`（`14.04 MB` 独立单目录便携版，实测全市场 `5,931` 只活跃 A 股扫描、6 只达标推荐、3 只 `100W` 模拟盘持仓及 11 份《经济学人》研报落盘 `./portable-release/data/economist_quant.db`，`.build` 自动水位清理至 `3.603 GB < 5.0 GB`，提交基线 `e215caf`）；
  - 完成第 1 轮深度专家 Code Review（DDD 四层隔离、并发账本锁与单飞扫描、UTF-8 多字节安全切片、`"1d" | "30m"` 与标准 `TelemetrySubsystem` 契约对齐、吊灯追踪止损保本上移、科创板零股规则及便携释放目录完整性），当前立即按 TDD 红绿循环落地第 1 轮全部修复，随后推进第 2–5 轮深度 Code Review 并推送至远程 `origin/main`。

- **Compact #4 (`2026-11-18` · Checkpoint: `Round 1 TDD red-green fixes applied`)**：
  - 完成第 1 轮深度 Code Review 全部 13 项修复的 TDD 红绿闭环（`42/42` Rust 单测/集成/红线/PBT 零警告全绿，`20/20` Vitest + `fast-check` PBT 全绿），并提交基线 `62c2a31`；
  - 启动第 2 轮机构级量化与 `100W/<=10` 账本深度审计。

- **Compact #5 (`2026-11-18` · Checkpoint: `Round 1 committed; Round 2 quant TDD underway`)**：
  - 第 1 轮深度 Code Review 已全绿提交（`62c2a31`）；第 2 轮机构级量化审计（`rubber-duck` · `gemini-4-carbon` · `high`）已完成全部领域数学缺陷定位与 `./src-tauri/tests/round2_quant_regressions.rs` TDD 红灯复现（`5/5` 精准失败），并已完成 `./src-tauri/src/domain/`（沪深北交易日历、涨跌停精确基点四舍五入、科创/北交零股步长、`09:30` U 型分时折算、五维排雷解禁与盘中上影线细化、FWL 行业收缩中性化、吊灯止损单调性、`T+1` 跨周末/节假日锁仓、含摩擦成本 `<=10%` 单票仓位计算）改造；
  - 当前立即完成行情源单位标准化（腾讯备用 K 线科创/北交股转手 `100x` 修正、流通股本换手率、东财 `HOLD_NOTICE_DATE` PIT 截断与 90 日净减持/30 日解禁）、应用层日线日期严格对齐与止损自动平仓执行，验证全量测试全绿后提交第 2 轮修复，继续推进第 3–5 轮深度 Code Review 并执行 `git push origin main`。

- **Compact #6 (`2026-11-18` · Checkpoint: `Round 2 quant fixes implemented; modularizing scan pipeline`)**：
  - 完成第 2 轮深度量化与 `100W/<=10` 账本全部修复及模块化拆分（新增 `./src-tauri/src/domain/trading_calendar.rs` 与 `./src-tauri/src/application/scan_alignment.rs`，确保全仓每个源码文件严格 `<= 600` 行），通过 `49/49` Rust 单测/架构红线/PBT/回归测试（零警告）与 `20/20` Vitest + `fast-check` PBT；
  - 提交第 2 轮基线（`c12856f`）并启动第 3 轮视觉与交互深度审计。

- **Compact #7 (`2026-11-18` · Checkpoint: `Round 2 committed; Round 3 Liquid Glass TDD fixes`)**：
  - 第 2 轮深度量化与 `100W/<=10` 账本审查已全绿提交（`c12856f`）；
  - 完成第 3 轮 Apple Liquid Glass 视觉光学与交互专家审查（`rubber-duck` · `gemini-4-carbon` · `high`），编写 `./src/__tests__/round3LiquidGlass.test.tsx` 5 项回归测试并验证 TDD 红灯（`5/5` 精准失败），已完成前两批修复（新增 `./src/components/charts/klineTimeAlign.ts` 解决 `30m` 数值时间戳与日级字符串买点标记混排崩溃、高特异性 `prefers-reduced-transparency` 回退、`15 bps` 小盘滑点与科创/北交零股平仓规则、六维标准因子 ID 对齐、多标的轮动弹窗与 `Escape` 键盘无障碍）；
  - 当前立即完成第 3 轮第 3 批页面级与研报 `srcDoc` ECharts 离线内联修复，跑通全量 Vitest + Rust + 构建 + 仓库卫生门禁并提交第 3 轮，随后推进第 4 轮（安全审计、因子 AST 安全与并发韧性）及第 5 轮（25 项需求终审、便携版 `.exe` 重新构建冒烟验证与文档同步），最后执行 `git push origin main`。

- **Compact #8 (`2026-11-18` · Checkpoint: `Round 3 committed; Round 4 security audit verified`)**：
  - 完成第 3 轮 Apple Liquid Glass 视觉光学、视口模糊预算（`<= 8`）、沙箱化离线内联 ECharts `srcDoc` 研报视图与单文件 `< 500` 行组件拆分修复，通过 `49/49` Rust 单测/架构红线、`25/25` Vitest、`npm run build` 及仓库卫生检查并提交基线（`b45f552`）；
  - 完成第 4 轮安全专项深度审计（`security-review` · `gemini-4-carbon` · `high`）并逐行复核确认全部 6 项发现（`<script>` 内联 JSON/链接存储型 XSS 与 `report_id` 严格校验、`127.0.0.1:4288` ACK 服务 `Origin`/`Host`/POST 负载与移除 `GET /api/ack-via-get` CSRF、`Path.is_relative_to` 目录穿越与 `re.sub` 反向引用防护、Tauri 主窗口 CSP 与内嵌研报离线 CSP、`AgentSupervisor` `--available-tools ""` / `stdin` 传参与安全二进制解析、账本 `checked_add`/`checked_mul` 防溢出及插件因子权重有限域校验）；
  - 按严格 TDD 红绿循环落地第 4 轮安全回归测试并提交基线（`c9f201e`）。

- **Compact #9 (`2026-11-19` · Checkpoint: `Round 4 security TDD fixes verified green & Round 5 final release`)**：
  - 完成第 4 轮安全加固提交（`c9f201e`）后，完成第 5 轮 25 项需求终审门禁（`round5_release_gate_regressions.rs`）：修复带属性 `<head ...>` 下离线研报 CSP `<meta>` 先于内联 `<script>` 注入顺序、完善 Windows WebView2 Tauri v2 `tauri.localhost` / `worker-src 'self' blob:` CSP 策略、补齐 `./scripts/stage_portable_release.ps1` 全套《经济学人》Python 渲染包与离线 ECharts 资源打包；
  - 全量通过 `56/56` Rust 单测/PBT/架构红线/五轮回归测试、`25/25` Vitest + `fast-check` 属性测试、`3/3` Python 安全回归测试、OpenSpec `--strict` 校验与全仓零绝对路径 / `< 1000` 行卫生门禁，完成便携 `x64` `./portable-release/economist-quant-terminal.exe` 终版重打包、实机冒烟验证与 `git push origin main`。

- **Compact #10 (`2026-11-19` · Checkpoint: `Round 5 pushed; starting watchlist and anti-overfit iteration`)**：
  - 实机启动 `./portable-release/economist-quant-terminal.exe` 并验证 `11:00 CST` 30 分钟盘中自动轮动扫描（`5,931` 只全 A -> `4,532` 只 `<200亿` -> `21` 只双日换手 `>5%` 放量 -> `10` 只排雷绿灯，`8` 只 `BUY` 信号）与 `100W` 模拟盘 `T+1` 实时盯市；
  - 响应用户新增需求「观望可加入自选检验策略真实胜率 + 持续迭代策略与因子且严禁过拟合」，按 Superpowers 设计门禁 + 4 层 DDD 模块化隔离（保持全仓每个源码文件严格 `< 500` / `<= 600` 行）+ TDD 红绿循环 + `proptest`/`fast-check` 属性测试推进实施、重打包并热重启便携终端。

- **Compact #11 (`2026-11-19` · Checkpoint: `Watchlist, anti-overfit, and version control DDD core`)**：
  - 完成用户补充需求整合（「观望加入自选胜率池 + 严禁过拟合滚动迭代 + 全版本快照与老版本并行影子追踪 + 版本一键切换与升级前后提升对比 + 新版本不及老版本四维归因诊断 + 完成后执行五轮深度 Code & 策略 Review 并推送远程」），落盘设计规格 `./docs/superpowers/specs/2026-11-19-watchlist-winrate-and-anti-overfit-design.md`；
  - 按严格 TDD 红绿循环与 `proptest` 属性测试落地 Rust 4 层 DDD 核心（`./src-tauri/src/domain/{watchlist_tracker,anti_overfit,version_control}.rs`、`./src-tauri/src/infrastructure/{sqlite_schema,sqlite_watchlist_repo}.rs`、`./src-tauri/src/application/{pit_observation_builder,watchlist_service,evolution_service}.rs` 及 6 个新增 Tauri IPC 命令），当前立即跑通全量 Rust 测试、推进 React 19 Apple Liquid Glass 前端 TDD 联调、执行 **5 轮深度代码与量化策略审查**，最后重打包热重启 `./portable-release/economist-quant-terminal.exe` 并执行 `git push origin main`。

- **Compact #12 (`2026-11-19` · Checkpoint: `Liquid Glass watchlist and version control UI`)**：
  - 全量通过 `62/62` Rust 单测/PBT/架构红线（零警告），完成 React 19 Apple Liquid Glass 自选胜率计分卡（`./src/components/watchlist/WatchlistWinRatePanel.tsx`）、全版本影子竞技场与四维退化归因面板（`./src/components/factors/VersionControlArenaPanel.tsx`）及 5 日隔离带防过拟合滚动验证面板（`./src/components/factors/AntiOverfitWalkForwardPanel.tsx`）拆分与全链路联调；
  - 修复前端 `fast-check` 属性测试捕获的极端稀疏权重有界单纯形定向注水投影边界（`[0,0,0,0,1e-9,0]`）并跑通全量 Vitest + `npm run build`；当前立即启动 **5 轮深度代码与量化策略审查**（DDD 与版本切换一致性 -> 量化防过拟合数学严谨性 -> 历史版本公平影子回放与四维归因 -> 安全审计 -> 25 项门禁重打包热重启便携 `.exe` 并 `git push origin main`）。



