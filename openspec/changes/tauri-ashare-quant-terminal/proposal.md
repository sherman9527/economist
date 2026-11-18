## Why

当前仓库仅包含基于 Python 脚本与静态 HTML 的《经济学人》每日宏观研报生成工具，缺乏面向全市场 A 股实时行情扫描、严格遵循中国市场交易微观结构（`T+1`、涨跌停、印花税/滑点、五维排雷）的量化选股执行闭环与便携式桌面交互体验。我们需要将其升级为一套零系统盘占用、零绝对路径、插件化配置驱动的 **便携式 `x64` `Tauri 2 + Rust + React/TypeScript + SQLite` A 股单策略量化投研与《经济学人》新闻一体化终端**，由持久化会话的 `Gemini 4 Carbon (high)` 多专家 Agent 协同自治迭代，追求风险可控前提下的长期净超额收益最大化。

## What Changes

- **便携式 `x64` 桌面端基座重构**：引入 **Tauri 2 + Rust + React 19 / TypeScript + SQLite (`rusqlite` `bundled` C 源码编译)** 四层 DDD 架构，仅编译产出无安装包的便携式 `x64` 可执行程序；运行期配置、SQLite 数据库、策略/因子插件、28 天日志、Telemetry、《经济学人》归档与 WebView2 缓存全部收敛于可执行文件同级相对目录（`./`）。
- **工程质量与体积红线治理**：**BREAKING（内部脚本路径契约升级）** 全面清除历史 Python/PowerShell 脚本与状态 JSON 中的硬编码绝对路径并拆分超 1000 行渲染脚本；全仓强制单文件 `< 1000` 行、零绝对路径，采用 **DDD + TDD + PBT（属性测试 `proptest` & `fast-check`）** 验证核心金融不变量，并内置 `< 5 GB` 构建缓存自动清理器。
- **Apple Liquid Glass 响应式零穿模 UI 系统**：创建专属 `designer` 专家 Agent 规范，打造深色默认 + 浅色双主题三层液态玻璃界面（20s+ 缓慢流体网格背景、单屏 `<= 8` 模糊层预算、等宽数字对齐、弹性防溢出排版杜绝任何缩放比例下的中文“文字穿模”），集成 TradingView `lightweight-charts` K 线（直绘量化买点与止损价格线）与 Apache ECharts 多维图表。
- **5,000+ 全 A 股实时行情、单策略引擎与 100W 模拟盘闭环**：创建专属 `quantitative_researcher` 专家 Agent 规范，对接高可用全市场 A 股实时行情源（启动即从外部实时拉取 `5,500+` 只 A 股快照，订单/持仓/因子/会话实时读写本地 SQLite）；聚焦唯一核心策略 **「总市值 `< 200 亿 RMB` + 连续 2 日换手率 `> 5%` 放量 + 五维智能排雷（减持/负面新闻/股东户数激增/主力流出/技术破位）」**，管理 `100W` 初始资金、最多 `10` 只持仓席位的 `T+1` 模拟盘，持久化每笔推荐理由与止损价，并在交易时段每 30 分钟轮动弹窗提醒。
- **插件化配置驱动、多 Agent 会话持久化自治与《经济学人》新闻板块融合**：所有策略阈值、排雷规则与因子表达式外置为热插拔 JSON/TOML 插件配置；主控与全部子 Agent 锁定 `gemini-4-carbon` (`high` reasoning) 并将 `sessionId` 持久化于 SQLite `agent_sessions` 表实现原上下文唤醒自愈与因子自主迭代；原《经济学人》投研报告与回溯审计无缝整合进应用内「新闻板块」，辅以全链路 `28d` Telemetry 监控。

## Capabilities

### New Capabilities
- `portable-tauri-ddd-core`: 便携式 `x64` Tauri 2 + Rust + SQLite (`bundled`) DDD 四层核心架构、同级目录自包含存储、零绝对路径门禁、单文件 `< 1000` 行约束、TDD+PBT 测试体系与 `< 5 GB` 构建产物自动清理能力。
- `apple-liquid-glass-ui`: 遵循 Apple Liquid Glass 光学规范的响应式六页桌面工作台、防文字穿模排版系统、红涨绿跌极性解耦、K 线买点与止损线可视化及交易时段 30 分钟轮动玻璃态弹窗交互能力。
- `ashare-universe-and-strategy`: `5,000+` 全 A 股实时外部行情采集、唯一核心策略（`< 200 亿市值 + 连续 2 日换手率 > 5% 放量`）、五维排雷否决引擎、截面中性化多因子打分与 `100W / <=10 只 / T+1` 模拟盘订单止损账本能力。
- `plugin-config-and-agents`: 参数化热插拔因子/排雷/策略配置中心、SQLite 主/子 Agent `sessionId` 持久化注册表（锁定 `Gemini 4 Carbon` `high`）以及主控自愈监控与量化 Agent 样本外门禁自主迭代能力。
- `economist-news-and-telemetry`: 既有《经济学人》跨市场深度投研流水线向应用内「新闻板块」的无损相对路径集成（含未读 ACK 联动与负面行业排雷信号输入）及全链路 `28d` 日志与 Telemetry 滚动监控能力。

### Modified Capabilities
<!-- 仓库首次初始化 OpenSpec，openspec/specs/ 下尚无既有基线 capability，全部作为 New Capabilities 建模 -->

## Impact

- **新增核心目录与模块**：`./src-tauri/`（Rust DDD 四层架构与 SQLite 引擎）、`./src/`（React 19 + TypeScript Apple Liquid Glass 六页前端）、`./config/` 与 `./plugins/`（参数化 TOML 与因子/策略/排雷 JSON 插件）、`./.github/agents/`（专家 Agent 定义）、`./memo.md`、`./rule.md`、`./handover.md`。
- **重构既有模块**：`./scripts/config.py`、`./scripts/ack_server.py`、`./scripts/run_daily.ps1`、`./scripts/register_tasks.ps1` 移除全部硬编码绝对路径并统一模型配置为 `gemini-4-carbon`；`./scripts/renderer.py`（原 3,492 行）拆分为单文件 `< 500` 行的 `./scripts/renderer_pkg/` 子模块。
- **外部依赖与工具链策略**：Rust 工具链与 Crates 优先通过中国镜像 `rsproxy.cn` / 清华 TUNA 拉取，SQLite 通过 `rusqlite` `bundled` 特性本地编译 C 源码，前端 NPM 依赖通过华为云 / 腾讯云中国镜像拉取，全程隔离于项目本地目录避免系统策略拦截或系统盘污染。
