## 1. 治理体系、专家 Agent 规范与 SQLite 会话基座

- [x] 1.1 创建项目根级开发记忆 `./memo.md`、避坑指南 `./rule.md` 与进度交接单 `./handover.md`（全仓零绝对路径）
- [x] 1.2 创建锁定 `gemini-4-carbon`（`high` 推理强度）的持久化专家 Agent 定义：`./.github/agents/designer.agent.md`、`./.github/agents/quantitative-researcher.agent.md`、`./.github/agents/ddd-tauri-architect.agent.md`
- [x] 1.3 初始化 `./data/economist_quant.db` SQLite 数据库与 10 张核心表迁移，写入主控与专家子 Agent 的持久化 `sessionId` 注册表（`agent_sessions`）

## 2. 中国镜像源工具链引导与既有《经济学人》脚本合规化重构

- [x] 2.1 通过字节跳动 `rsproxy.cn` 镜像在项目相对目录 `./.toolchain/` 下引导安装 `stable-x86_64-pc-windows-msvc` Rust 工具链，配置 `./.cargo/config.toml`（`rsproxy` + 清华 TUNA 稀疏索引）与 `./.npmrc`（华为云 + 腾讯云 NPM 镜像）
- [x] 2.2 重构 `./scripts/config.py`、`./scripts/ack_server.py`、`./scripts/run_daily.ps1`、`./scripts/register_tasks.ps1` 与 `./data/ack_state.json`，彻底消除所有硬编码绝对路径并将默认模型升级为 `gemini-4-carbon`（`high`）
- [x] 2.3 将超 1000 行的 `./scripts/renderer.py` 拆分为单文件 `< 500` 行的模块化子包 `./scripts/renderer_pkg/`，确保原单窗口报告渲染与 ACK 功能无损通过回归验证

## 3. Rust DDD 领域核心层（`domain`）与 TDD + `proptest` 属性测试

- [x] 3.1 实现精确金融货币与比率值对象（整数 `Fen`、`Bps`、沪深北证券代码、板块涨跌停幅度 `±10%/20%/30%`、整手 `100/200` 股校验）及 `proptest` 属性测试
- [x] 3.2 实现唯一核心选股规约（总市值 `< 200 亿 RMB` + 连续 2 日换手率 `> 5%` 放量及盘中 U 型分时折算规约）与五维智能排雷否决引擎（减持、负面新闻/监管、股东户数环比、主力净流出、技术破位）及单元测试/PBT
- [x] 3.3 实现安全声明式因子表达式 AST 求值器、申万行业 + 对数流通市值截面正交中性化、`3*MAD` 去极值 Z-Score 合成器与 ATR 硬止损/吊灯移动止损计算器
- [x] 3.4 实现 `100W` 初始资金、最多 `10` 只持仓席位、严格 `T+1` 交收锁仓与印花税/佣金/滑点扣减的模拟盘组合聚合根（`PortfolioLedger`）及状态机属性测试

## 4. Rust 基础设施层（`infrastructure`）、插件中心、监控自愈与便携目录守卫

- [x] 4.1 实现便携同级目录解析器（`PortablePaths`）与 `WEBVIEW2_USER_DATA_FOLDER=./webview_data`、`TEMP=./.temp` 启动前置绑定，确保零系统盘写入且所有入库路径标准化为相对路径
- [x] 4.2 实现 `rusqlite`（`bundled` C 源码编译）仓储层：覆盖行情缓存、推荐买点、带止损与理由的模拟订单、`<=10` 席位持仓、因子插件、策略迭代、`agent_sessions`、《经济学人》新闻索引与 `telemetry_events`
- [x] 4.3 实现 `5,000+` 全 A 股外部实时行情双通道网关（腾讯 `qt.gtimg.cn` 秒级全市场批扫 + 东财 `push2his` 日线/30 分钟线历史换手率及数据中心五维排雷客户端，含主备自动故障切换）
- [x] 4.4 实现 `./config/` 与 `./plugins/{strategies,factors}/` 参数化配置与插件热加载器，落盘默认 `smallcap_turnover_ignition.json` 与 `core_factors_v1.json`
- [x] 4.5 实现《经济学人》相对路径研报与四阶传导数据同步索引器（桥接 `./data/ack_state.json`、`./data/runs/*.json` 与 `./reports/`）
- [x] 4.6 实现全链路 Telemetry 双写收集器、`28` 天日志/事件自动滚动清理器、`< 5 GB` 构建产物分级自动清理器（含 `./scripts/clean_build_cap.ps1`）及 `gemini-4-carbon` 子 Agent 会话唤醒自愈监控器

## 5. Rust 应用编排与 Tauri v2 IPC 接口层（`application` & `interface`）

- [x] 5.1 实现启动即拉取全市场外部实时行情 + 读取本地 SQLite 账本、手动/自动全市场选股扫描、模拟盘下单与止损巡检、配置热更新及新闻 ACK 用例服务
- [x] 5.2 实现 A 股法定交易时段（周一至周五 `09:30–11:30`、`13:00–15:00`）每 `30` 分钟自动轮动调度器，命中新推荐标的时向前端主动发射 `quant://rotation-alert` 事件
- [x] 5.3 暴露类型安全的 Tauri v2 Commands 并编写全仓架构红线自动化测试（断言零绝对路径、全仓单文件 `< 1000` 行、构建目录 `< 5 GB`）

## 6. React 19 + TypeScript Apple Liquid Glass 六页前端与防穿模验证

- [x] 6.1 搭建前端工程骨架与三层 Apple Liquid Glass 光学样式系统（`tokens.css`、`typography.css`、`glass-surfaces.css`、`20s+` 慢速网格流体背景、深浅双主题、红涨绿跌与风控色解耦、`<= 8` 模糊层守卫与 `<SafeText>` 防穿模组件）
- [x] 6.2 实现金融可视化核心组件：TradingView `lightweight-charts` 量化买点与止损水平线 K 线终端（含推荐理由玻璃浮层）、Apache ECharts 累计超额净值曲线、持仓/行业圆环图、换手放量与因子得分柱状图
- [x] 6.3 实现第 1–3 页：组合与策略总览仪表盘、`5,000+` 全 A 股 30 分钟轮动雷达与五维排雷终端、`100W` 模拟盘订单簿与 `<=10` 席位 `T+1` 持仓风控台（含全局 30 分钟轮动推荐弹窗 `RotationAlertToast`）
- [x] 6.4 实现第 4–6 页：量化专家 Agent 因子实验室与参数化插件配置中心、《经济学人》全球政经新闻板块（四阶传导、未读 ACK 队列、回溯胜率审计、原版研报阅读器）、全链路 Telemetry 与 `28` 天日志及 Agent `sessionId` 会话中心
- [x] 6.5 编写前端 `vitest` + `fast-check` 属性测试（验证极值中英文排版零溢出契约、`tabular-nums` 对齐、`<=10` 席位与 `T+1` 规则、`<=8` 模糊槽位上限）

## 7. 便携式 `x64` 产物编译、端到端实盘行情验收与交付

- [x] 7.1 执行 `cargo test` 全量单元/属性/红线测试与前端 `vitest` 测试并确保 100% 通过
- [x] 7.2 编译构建无安装包的 `x86_64-pc-windows-msvc` 便携可执行文件并组装同级运行目录，实机验证 `5,500+` 只全 A 股实时行情拉取、选股排雷推荐、K 线买点/止损渲染与 SQLite 模拟订单落盘
- [x] 7.3 执行构建产物 `< 5 GB` 水位审计，同步更新 `./memo.md`、`./rule.md`、`./handover.md` 与 OpenSpec 任务状态并完成 Git 提交
