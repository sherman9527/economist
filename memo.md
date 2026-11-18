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
