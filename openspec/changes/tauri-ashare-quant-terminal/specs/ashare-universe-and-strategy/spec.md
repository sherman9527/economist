## ADDED Requirements

### Requirement: Live 5000+ Full A-Share External Market Data Ingestion vs Local SQLite Ledger
系统启动与盘中轮动时，股票行情数据 MUST 从高可用外部 A 股实时行情源（覆盖沪、深、北全市场 `5,000+` 只活跃 A 股，具备主备源自动故障切换）实时拉取；而模拟盘订单、持仓、因子插件、策略版本与 Agent 会话记录 MUST 实时从本地 SQLite 数据库查询与持久化。

#### Scenario: Cold open fetches live full-market quotes and local SQLite portfolio state
- **WHEN** 用户打开桌面应用或点击“立即全 A 扫描”
- **THEN** 后端从外部实时行情接口拉取全市场 `5,000+` 只活跃 A 股最新快照并刷新行情缓存，同时从本地 `./data/economist_quant.db` 实时读取 `100W` 模拟盘持仓、历史订单、推荐标注与因子配置返回前端渲染

### Requirement: Single Core Small-Cap Consecutive High-Turnover Volume-Expansion Strategy with Pitfall Veto
系统 MUST 聚焦并唯一迭代单核心选股策略：**总市值 `< 200 亿 RMB` 且连续 `2` 个交易日换手率 `> 5%` 并伴随放量**（自动剔除 `ST/*ST/退市整理`、停牌、上市未满 `60` 交易日次新股及涨停封死标的），并前置执行由量化专家 Agent 持续维护的五维排雷否决引擎（大股东/高管减持、监管/负面新闻、股东户数环比激增、主力资金净流出、技术形态破位）。

#### Scenario: Filter 5000+ A-shares through core strategy gate and 5-dimension pitfall veto
- **WHEN** 策略引擎对全市场快照与历史日线/分时数据执行选股评估
- **THEN** 任何市值 `>= 200 亿 RMB`、未满足连续 2 日换手 `> 5%` 放量或触发任一硬性排雷否决项的股票均被拦截并记录具体排雷审计原因，仅全部通过的标的进入截面中性化多因子打分推荐队列

### Requirement: 1,000,000 RMB Simulated Portfolio with 10-Stock Cap, T+1 Settlement, and Stop-Loss Persistence
模拟盘账本 MUST 以 `1,000,000.00 RMB`（`100W`，精确整数“分”计价）作为初始本金，严格限制同时持仓股票数量不超过 `10` 只，强制执行 A 股 `T+1` 交收锁仓、整手 `100` 股（科创板 `>= 200` 股）、板块涨跌停（`±10%/20%/30%`）及真实卖出印花税/佣金/冲击滑点扣减，并将每笔订单的**推荐理由（`rationale`）**与**止损价格（`stop_loss_price`）**持久化至 SQLite。

#### Scenario: Place simulated buy order and enforce T+1 lock and 10-slot ceiling
- **WHEN** 模拟盘根据量化推荐信号执行买入下单，或尝试在同日卖出当日新买份额，或在已满 `10` 只持仓时发起第 `11` 只新股买入
- **THEN** 合规买单扣除佣金与滑点后落盘至 `simulated_orders`（含非空 `rationale` 与低于入场价的 `stop_loss_price`），当日买入股数记为 `T+1` 锁定不可卖状态，且超出 `10` 只席位上限的新开仓请求被风控引擎明确拒绝
