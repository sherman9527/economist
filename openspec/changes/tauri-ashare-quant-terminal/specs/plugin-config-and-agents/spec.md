## ADDED Requirements

### Requirement: Parameterized Plugin-Driven Strategy, Factor, and Pitfall Configuration
系统 SHALL 将数据源端点优先级、策略参数、排雷阈值、止损倍数及因子声明式表达式外置为相对路径 `./config/` 与 `./plugins/` 下的结构化配置与插件清单，并同步镜像于 SQLite，使得后续策略调优、新增因子与排雷迭代无需硬编码修改应用主程序即可热加载生效。

#### Scenario: Hot-reload updated factor weights and pitfall plugin parameters
- **WHEN** 量化专家 Agent 或用户在配置中心更新因子权重、排雷阈值或新增声明式因子插件并触发校验热加载
- **THEN** 系统通过 Schema 与安全 AST 校验后原子更新本地插件清单与 SQLite `factor_plugins` / `strategy_iterations` 记录，下一轮选股扫描立即按新配置执行而无需重新编译二进制程序

### Requirement: Persistent Main and Sub-Agent Session Registry Locked to Gemini 4 Carbon High Reasoning
系统 MUST 将主控 Agent 与所有专家子 Agent（UI 设计师、量化研究员、架构与风控审计员、《经济学人》分析师）的 `sessionId` 持久化存储于 SQLite 的 `agent_sessions` 表中，且所有主/子 Agent 调用 MUST 统一锁定模型 `gemini-4-carbon` 与推理强度 `high`，支持通过持久化 `sessionId` 恢复历史会话上下文。

#### Scenario: Wake persistent Quant Researcher Agent with retained context
- **WHEN** 主控监控器触发周期性因子迭代或异常自愈诊断，从 SQLite `agent_sessions` 表读取目标角色的持久化 `sessionId`
- **THEN** 调度器以 `--model gemini-4-carbon --reasoning-effort high` 及对应持久化 `sessionId` 唤醒子 Agent，延续既有研究记忆并在完成后更新 `turn_count` 与心跳时间戳

### Requirement: Continuous Main-Agent Health Supervision and Return-First Quant Autonomous Iteration
主控监控器 SHALL 持续巡检应用行情链路、SQLite 状态、UI 异常与子 Agent 执行健康度；当检测到接口报错或模型衰减时自动执行故障切换或自愈修复，系统健康空闲时调度量化专家 Agent 在样本外（OOS）扣费净收益门禁约束下自主迭代因子池与排雷权重。

#### Scenario: Promote candidate factor iteration only when out-of-sample net return gate passes
- **WHEN** 量化专家 Agent 提交候选因子插件或权重组合进入样本外回测评估
- **THEN** 系统仅在候选方案满足样本外 `|Rank_IC| >= 0.030`、`|IC_IR| >= 0.50`、低冗余相关性且扣除交易成本后的净收益风险比优于当前在线版本时将其激活，否则保留为候选观察态
