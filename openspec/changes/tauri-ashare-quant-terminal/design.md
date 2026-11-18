## Context

当前仓库起源于一套零系统盘占用的 Python《经济学人》每日全球政经与 A股/美股深度投研报告系统（含 12 频道抓取、断点续传分析、高盛/大摩估值表、多空回溯审计与单窗口页内 Pivot 审阅台）。根据 `./requirement.txt`、`./designer.txt` 与 `./quantitative_researcher.txt` 的演进要求，系统需全面升级为便携式 `x64` **`Tauri 2 + Rust + React 19 / TypeScript + SQLite`** 桌面级 A 股量化投研一体化终端，同时满足：
- **零绝对路径**、**单文件 `< 1000` 行**、**构建产物 `< 5 GB`**、**无安装包便携同级目录自包含**；
- **Apple Liquid Glass** 响应式零文字穿模界面与交易级 K 线（直绘买点与止损线）+ ECharts 图表；
- **全市场 `5,000+` 只 A 股外部实时行情** 与本地 SQLite 模拟盘账本读写分离；
- **唯一核心策略**（总市值 `< 200 亿 RMB` + 连续 2 日换手率 `> 5%` 放量 + 五维智能排雷），`100W` 初始资金、最多 `10` 只持仓、严格 `T+1` 与印花税/佣金/滑点计提，交易时段每 `30` 分钟轮动弹窗推荐；
- **插件化参数配置**、**SQLite 持久化主/子 Agent `sessionId`（统一锁定 `Gemini 4 Carbon` · `high`）**、**《经济学人》新闻板块融合** 与 **`28` 天全链路 Telemetry 监控**。

## Goals / Non-Goals

**Goals:**
- 打造开箱即用、可随目录整体移动的 `x86_64-pc-windows-msvc` 便携式 `.exe`，所有运行期数据（`./config`、`./data`、`./plugins`、`./reports`、`./logs`、`./telemetry`、`./webview_data`、`./.temp`）100% 收敛于可执行文件同级目录。
- 建立纯粹的四层 DDD（`domain` / `application` / `infrastructure` / `interface`）架构，核心金融计算采用整数“分”（`Fen`）与基点（`Bps`），辅以 TDD 单元测试与 `proptest` / `fast-check` 属性测试（PBT），并由自动化红线测试强制保证零绝对路径与单文件 `< 1000` 行。
- 实现 `1.0–1.5s` 级全市场 `5,500+` 只活跃 A 股快照扫描、30 分钟交易时段自动轮动与非阻塞推荐弹窗、K 线买点与止损水平线可视化及 `100W / <=10 只` 模拟盘完整闭环。
- 通过中国镜像源（`rsproxy.cn`、清华 TUNA、华为云/腾讯云 NPM）与源码静态编译（`rusqlite` `bundled`）彻底规避外部网络超时与主机安全策略拦截。

**Non-Goals:**
- **不制作 MSI / NSIS 系统安装程序**，不写注册表安装项，不向系统盘 `AppData` 散落缓存。
- **不发散多套并列选股策略**：严格聚焦 `< 200 亿市值 + 连续 2 日换手率 > 5% 放量 + 五维排雷` 这一套核心策略做深做透。
- **不接入真实券商实盘柜台资金划转接口**：全部订单与持仓在本地高保真 SQLite `T+1` 模拟账本中闭环执行与审计。

## Decisions

### Decision 1: 便携式同级目录引导与 SQLite C 源码静态编译 (`rusqlite` `bundled`)
- **选择**：在 Rust `main.rs` 创建 WebView 之前，通过 `std::env::current_exe()` 解析便携根目录，显式注入 `WEBVIEW2_USER_DATA_FOLDER=./webview_data` 与 `TEMP/TMP=./.temp`，并通过 `rusqlite = { features = ["bundled"] }` 使用本机 `vswhere` 动态发现的 MSVC x64 工具链从 C 源码编译 SQLite 静态打入 `.exe`；数据库开启 `WAL` 与 `PRAGMA temp_store = MEMORY`。
- **备选方案与取舍**：相比外部下载 `sqlite3.dll` 或使用异步 ORM 绑定，源码静态编译零外部二进制依赖、不会被企业安全策略拦截，且单文件体积小、事务延迟 `< 1ms`。

### Decision 2: 中国镜像源双活链路与 `< 5 GB` 构建体积主动防御
- **选择**：经实机探测验证，Rust 工具链安装至本地 `./.toolchain/` 并配置字节跳动 `rsproxy.cn` 稀疏索引为主源、清华 TUNA 为备源（排除返回 `403` 的 USTC 源）；前端 `.npmrc` 锁定华为云 NPM 镜像为主源、腾讯云为备源（排除出现 TLS 重置的默认源与 npmmirror）。同时在 `Cargo.toml` 设置 `[profile.dev] debug = 0; strip = "debuginfo"` 将构建目录体积从 `~6.5 GB` 压降至 `~1.7 GB`，并内置 `4.0 GB` 软水位 / `5.0 GB` 硬上限三级自动清理器。
- **备选方案与取舍**：仅靠事后 `cargo clean` 会导致反复全量重编且编译峰值瞬间突破 `5 GB`；编译期裁剪调试符号 + 增量指纹精准回收兼顾了二次增量编译速度与严格 `< 5 GB` 约束。

### Decision 3: 5,500+ 全 A 股“腾讯毫秒级全量批扫 + 东财历史换手/排雷深度增强”双通道行情架构
- **选择**：
  1. **全市场实时截面快照**：使用腾讯 `qt.gtimg.cn` 每批 `600` 只代码、10 批并发在 `~1.0s` 内拉齐沪深北全部 `5,576` 只活跃 A 股的现价、涨跌幅、成交额、当日换手率、总市值、流通市值与量比（`GBK` 解码）；
  2. **连续两日换手放量与 K 线核验**：对初筛 `< 200 亿市值` 且当日（或盘中 U 型分时折算）换手达标的候选池，并发调用东财 `push2his` 日线（`klt=101`）与 30 分钟线（`klt=30`）精准提取 `T-1` 换手率（`f61`）、20 日均量、`ATR_14` 与 `MA20`，并以腾讯 `ifzq.gtimg.cn` 分钟线作为自动降级备源；
  3. **五维排雷核验**：对接东财数据中心股东增减持、股东户数（严格以真实披露日 `HOLD_NOTICE_DATE` 过滤杜绝未来函数）、主力资金流（`f62`）、监管风险公告及本地《经济学人》新闻负面行业映射。
- **备选方案与取舍**：单一依赖东财全市场列表受限于服务端 `pz=100` 分页硬上限与高并发断流；腾讯全量秒扫 + 东财候选池深挖既保证了 100% 覆盖 `5,500+` 全 A 股，又将单轮 30 分钟扫描总耗时稳定控制在 `3–5s` 内。

### Decision 4: 声明式 JSON 因子/排雷插件 AST 求值与 SQLite `sessionId` 会话持久化自治
- **选择**：在 Rust `domain::quant` 中实现无 `eval` 注入风险的类型化因子表达式求值器与截面正交中性化（申万行业 + 对数流通市值 + `3*MAD` 去极值）管线，策略参数与因子定义全部存放在 `./config/*.toml` 与 `./plugins/{strategies,factors}/*.json`。主控与专家子 Agent 的 `sessionId` 持久化在 SQLite `agent_sessions` 表，统一通过 `copilot --model gemini-4-carbon --reasoning-effort high --resume <session_id> --log-dir ./logs/agents` 唤醒，新因子必须通过样本外（OOS）扣费净收益门禁方可激活。

### Decision 5: Apple Liquid Glass 八槽模糊预算与防文字穿模排版契约
- **选择**：顶层背景使用纯 GPU `transform` 驱动的 `23s/29s/37s` 慢速三色流体网格；全局真实 `backdrop-filter` 严格限制在侧边栏、顶栏、主 Bento 壳层与浮层共 `<= 8` 个槽位，嵌套子卡改用高光折射边框层 `.glass-optical-2`。排版层全面禁用文本容器固定 `height`、强制 `min-width: 0` + `minmax(0, 1fr)`、中英安全行高 `>= 1.28 / 1.42 / 1.68`、`tabular-nums` 等宽数字，并解耦 A 股红涨绿跌极性与止损琥珀色/排雷品红色。K 线采用 Canvas 硬件加速的 TradingView `lightweight-charts` 绘制蜡烛图、买点箭头与止损线，组合净值/行业分布/因子得分采用按需 Tree-shaking 的 Apache ECharts。

## Risks / Trade-offs

- **[Risk] 外部免费行情源单端点瞬时限流或超时** → **Mitigation**：内置主备双源（腾讯 `qt.gtimg.cn` / `ifzq.gtimg.cn` ↔ 东财 `push2` / `82.push2`）自动熔断切换、指数退避重试与令牌桶并发限流（`<= 8` 并发），并在顶部状态栏与 Telemetry 实时展示数据源延迟。
- **[Risk] 盘中早盘前 30 分钟换手率尚未累积满全日 5% 导致漏选或误选** → **Mitigation**：采用基于 A 股历史分时成交分布的 U 型日内累计成交曲线折算预测全日换手率（要求预测换手 `>= 5.5%` 且已实现换手硬下限 `>= 2.5%`），并在配置插件中参数化暴露。
- **[Risk] 小盘高换手标的次日高波动触发跳空低开或跌停锁死** → **Mitigation**：严格执行 `<= 10` 席位分散、单笔止损风险敞口 `<= 0.8% NAV`、综合前低与 `2*ATR_14` 的 `[-7.5%, -4.0%]` 初始硬止损、`+6%` 浮盈启动吊灯移动止损，以及跌停无法成交时的 `T+1` / 延迟平仓真实账本状态机。
- **[Risk] 迭代过程中单文件膨胀突破 1000 行或混入绝对路径** → **Mitigation**：按领域职责拆分为 `< 350` 行的细粒度模块（包括先将历史 3,492 行的 `scripts/renderer.py` 拆分为子包），并在 `cargo test` 中内置全仓文件行数与零绝对路径强制扫描测试。

## Migration Plan

1. **治理与规范先行**：落盘 `./memo.md`、`./rule.md`、`./handover.md`、`./.github/agents/*.agent.md`、OpenSpec 四大标准文档及 Superpowers 设计/实施计划，初始化 `./data/economist_quant.db` 持久化当前主/子 Agent `sessionId`。
2. **历史脚本无损合规化**：重构 `./scripts/*.py` 与 `./scripts/*.ps1` 为纯相对路径寻址，将 `./scripts/renderer.py` 拆分为单文件 `< 500` 行的 `./scripts/renderer_pkg/`，保持原《经济学人》定时抓取与 HTML 报告 100% 兼容。
3. **中国镜像源工具链引导与后端/前端增量构建**：拉取本地 Rust 工具链与前端依赖，按 `domain → infrastructure → application → interface → React Liquid Glass UI` 顺序以 TDD + PBT 推进实现。
4. **便携产物组装与双端联调**：产出便携目录 `economist-quant.exe` 及同级 `./config`、`./plugins`、`./data`、`./reports`，完成全量测试、`< 5 GB` 清理校验与 Git 提交。

## Open Questions

- 无阻塞性未决问题：25 项核心需求、视觉规范、量化协议与中国镜像源实测参数均已对齐并参数化外置至 `./config/` 与 `./plugins/`，后续阈值微调可由量化专家 Agent 直接通过配置文件热更新完成。
