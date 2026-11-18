## ADDED Requirements

### Requirement: Portable x64 Co-Located Executable Without Installer
系统 MUST 编译产出独立的 `x86_64-pc-windows-msvc` 便携式可执行文件（禁用 MSI 与 NSIS 安装包打包），且应用运行期间产生或读写的所有文件（包括 SQLite 数据库、参数配置、策略与因子插件、运行日志、Telemetry 追踪、研报归档、临时文件及 WebView2 浏览器数据目录）SHALL 全部位于可执行文件所在的同级相对目录（`./`）下，严禁向系统盘用户目录写入运行期数据。

#### Scenario: Launch portable executable from any directory
- **WHEN** 用户从任意目录启动便携式 `x64` 可执行程序
- **THEN** 程序自动在自身同级目录下初始化或挂载 `./config`、`./data`、`./plugins`、`./reports`、`./logs`、`./telemetry`、`./webview_data` 与 `./.temp`，并将 `WEBVIEW2_USER_DATA_FOLDER` 绑定至同级 `./webview_data`

### Requirement: Bundled Source-Compiled SQLite Persistence
系统 SHALL 通过 `rusqlite` 的 `bundled` 特性直接从 C 源码编译并静态链接 SQLite 引擎，配置 `WAL` 日志模式与内存临时表（`PRAGMA temp_store = MEMORY`），无需目标机器预装任何外部数据库服务或动态链接库。

#### Scenario: Initialize local SQLite database on cold start
- **WHEN** 可执行程序首次启动且 `./data/economist_quant.db` 尚不存在
- **THEN** 内置静态编译的 SQLite 引擎自动创建数据库文件并幂等执行全部领域表（`agent_sessions`、`stock_universe_cache`、`factor_plugins`、`strategy_iterations`、`recommendations`、`simulated_orders`、`positions`、`portfolio_snapshots`、`economist_news_index`、`telemetry_events`）迁移

### Requirement: Zero Hardcoded Absolute Paths Across Repository and Persistence
项目源码、脚本、配置文件、文档及持久化落盘数据（SQLite 字段与 JSON 状态）中 MUST NOT 出现任何硬编码盘符绝对路径；所有持久化路径引用 SHALL 使用相对于项目或可执行目录的相对路径表示。

#### Scenario: Automated repository redline scan checks for absolute paths
- **WHEN** 运行自动化架构合规测试扫描全仓源码、脚本、配置与文档
- **THEN** 盘符绝对路径正则匹配计数严格为 `0`，且 SQLite 路径字段约束拒绝包含盘符冒号的字符串写入

### Requirement: DDD Layered Architecture with Sub-1000-Line File Ceiling and TDD/PBT Verification
系统核心 MUST 采用领域驱动设计（`domain`、`application`、`infrastructure`、`interface` 四层单向依赖），通过测试驱动开发（TDD）与基于属性的测试（Rust `proptest` 及前端 `fast-check`）验证核心金融与界面不变量，且仓库中任何单个源码、脚本或样式文件的总行数 MUST 严格小于 `1000` 行。

#### Scenario: File line-count gate and property-based test suite execution
- **WHEN** 执行 `cargo test` 与前端 `vitest` 测试套件
- **THEN** 所有领域单元测试与随机生成输入的属性测试全部通过，且全仓单文件最大行数审计断言每个受管文件严格 `< 1000` 行

### Requirement: Automatic Build Artifact Cleanup Under 5 GB Cap
系统与构建工具链 MUST 定期监控并清理编译中间产物与缓存，确保构建产物总体积在任何时刻 SHALL NOT 超过 `5 GB` 硬上限。

#### Scenario: Build cache approaches soft watermark
- **WHEN** 构建前/后清理脚本或应用后台巡检器检测到构建目录总占用超过 `4.0 GB` 软水位
- **THEN** 清理器按分级策略自动回收陈旧增量编译缓存、过期指纹依赖产物与临时文件，使总构建产物占用回落并严格保持在 `5.0 GB` 以下
