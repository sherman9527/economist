## ADDED Requirements

### Requirement: Integrated Economist Macro Research News Module with ACK Continuity
系统 MUST 将既有《经济学人》全球政经与跨市场投研能力整合进桌面终端的「新闻板块」，全部通过相对路径（`./data/`、`./reports/`）索引历史与每日新增深度研报、四阶产业链传导链条、未读 `ACK` 审阅队列及跨期多空回溯胜率账本，并将宏观负面冲击行业信号联动输入量化排雷引擎。

#### Scenario: Browse Economist synthesis, acknowledge unread report, and cross-check A-share ticker
- **WHEN** 用户切换到应用内「新闻板块」查看宏观四阶传导卡片、点击未读研报 `ACK` 按钮或点击研报映射的 A 股代码徽章
- **THEN** 系统原子更新本地相对路径 `ACK` 状态与 SQLite `economist_news_index` 记录，并支持一键联动跳转至该 A 股标的的实时 K 线与排雷详情视图

### Requirement: End-to-End Pipeline Telemetry Monitoring and 28-Day Rolling Log Retention
系统 SHALL 对行情拉取、策略扫描、模拟盘撮合、插件热加载、Agent 会话唤醒、新闻索引与前端异常进行全链路结构化 Telemetry 埋点与分级日志记录，并自动执行 `28` 天滚动保留策略，清理早于 `28` 天的历史日志与追踪分片。

#### Scenario: Automatic pruning of logs and telemetry events older than 28 days
- **WHEN** 应用启动后或后台定时保留巡检器触发 `28` 天生命周期审计
- **THEN** 所有早于当前时间 `28` 天的日志文件与 SQLite `telemetry_events` 记录被分批安全清除并回收空闲页，近 `28` 天内的监控指标与日志完整保留可供检索
