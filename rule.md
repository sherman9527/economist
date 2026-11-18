# 项目避坑指南与工程红线手册 (`rule.md`)

> **维护契约（Requirement #2 & #4）**：每一次开发、构建、联调或测试中发现并解决的真实问题，都必须沉淀到本文件，形成可复用的防回归工程规则。全仓（含本文档）严禁出现任何硬编码绝对路径。

---

## 红线总纲（自动化测试强制门禁）

1. **零绝对路径红线（Req #2）**：全仓源码、脚本、配置、SQLite 记录、JSON 快照与文档中，严禁出现盘符绝对路径或系统根路径硬编码。运行期一律以便携可执行文件所在目录 `./` 进行相对寻址，入库路径统一标准化为 `./reports/...`、`./plugins/...`、`./config/...`。
2. **单文件 `< 1000` 行硬红线（Req #14）**：任何 `.rs`、`.ts`、`.tsx`、`.css`、`.py`、`.ps1` 文件行数不得达到或超过 `1000` 行（目标控制在 `150–450` 行以内，按领域职责拆分模块）。
3. **构建产物 `< 5 GB` 硬上限（Req #7）**：构建缓存与中间产物总体积不得超过 `5.0 GB`；`Cargo.toml` 开发配置默认关闭冗余调试符号（`debug = 0`），并启用构建前后与后台定时水位清理。
4. **便携式 `x64` 单目录自包含（Req #8 & #9）**：仅产出便携式 `x86_64-pc-windows-msvc` `.exe`（禁用 MSI / NSIS 安装包），运行期强制将 `WEBVIEW2_USER_DATA_FOLDER` 重定向至 `./webview_data`、临时目录重定向至 `./.temp`，确保零系统盘写入。
5. **Agent 模型与会话持久化红线（Req #11 & #13）**：主控与所有子 Agent 必须锁定 `gemini-4-carbon` + `high` 推理强度，所有主/子 `sessionId` 实时存入 SQLite `agent_sessions` 表。

---

## 已解决的实战避坑清单（Pitfall Playbook）

### Pitfall #01：NPM 默认源与淘宝镜像在本机网络出现 TLS 握手重置
- **现象**：直连官方源或常用 `registry.npmmirror.com` 拉取前端依赖时出现 `ERR_SSL` / 连接重置或策略拦截。
- **根因**：企业主机网络出口安全策略对部分境外及特定 CDN 证书链存在拦截或限流。
- **标准解法**：项目级 `.npmrc` 主源锁定经实测 `200 OK`（`~950 ms`）的 **华为云 NPM 镜像** `https://mirrors.huaweicloud.com/repository/npm/`，备源采用 **腾讯云 NPM 镜像** `https://mirrors.cloud.tencent.com/npm/`。

### Pitfall #02：中科大（USTC）Rust 镜像返回 `403 Forbidden`
- **现象**：请求 USTC `crates.io-index` 或 `rustup` 渠道时返回 HTTP `403`。
- **标准解法**：Rust 工具链与 Cargo 稀疏索引统一使用 **字节跳动 `rsproxy.cn`**（`RUSTUP_DIST_SERVER=https://rsproxy.cn`、`RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup`、`sparse+https://rsproxy.cn/index/`），备用源配置为清华 TUNA 稀疏索引 `sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/`；工具链本身安装于仓库内相对目录 `./.toolchain/`。

### Pitfall #03：多版本 Visual Studio 并存时误选仅含 Redistributable 的空壳目录导致 `link.exe` 缺失
- **现象**：机器上存在高版本 VS 目录但内部仅有运行时分发包，无 `cl.exe`、`link.exe` 与 `vcvars64.bat`，导致 Rust `x86_64-pc-windows-msvc` 链接失败。
- **标准解法**：构建脚本严禁写死编译器绝对路径，统一通过 `vswhere.exe -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath` 动态定位具备完整 x64 C/C++ 编译工具链与 Windows SDK (`rc.exe`/`mt.exe`) 的安装实例，并在同一子进程内加载其 `VC\Auxiliary\Build\vcvars64.bat`。

### Pitfall #04：SQLite 外部二进制/DLL 缺失或被系统策略拦截
- **现象**：依赖系统预装 `sqlite3.dll` 或预编译二进制下载时易受系统策略拦截或目标机缺库影响。
- **标准解法**：Rust 端 `rusqlite` 显式启用 `features = ["bundled"]`，直接将内置 SQLite C 源码（Amalgamation）通过本机 MSVC 编译并静态链接至便携 `.exe` 内，且初始化连接执行 `PRAGMA temp_store = MEMORY; PRAGMA journal_mode = WAL;`，兼顾零外部依赖与零系统盘临时文件。

### Pitfall #05：WebView2 默认在系统盘 `AppData` 创建数十 MB 缓存目录
- **现象**：Tauri 启动 Chromium WebView2 时若不显式指定数据目录，会自动向系统盘用户目录写入 `EBWebView` 缓存，违反 Requirement #9。
- **标准解法**：在 Rust `main.rs` 创建任何窗口前，基于 `std::env::current_exe()` 解析同级 `./webview_data` 与 `./.temp`，立即设置进程环境并在 `WebviewWindowBuilder::data_directory(...)` 中显式注入同级相对目录。

### Pitfall #06：宿主机残留 `PYTHONHOME` 环境变量导致 Python 3.14 启动崩溃
- **现象**：直接运行 `python` 报标准库编码模块加载失败（残留旧版 `PYTHONHOME` 干扰）。
- **标准解法**：所有调用 Python 的脚本与子进程包装器在入口处显式清空 `PYTHONHOME=""`、`PYTHONPATH=""` 并设置 `PYTHONIOENCODING="utf-8"`。

### Pitfall #07：东方财富 `clist/get` 全市场行情接口单页硬上限 `pz=100` 及高并发断流
- **现象**：请求 `pz=5000` 时服务端静默截断仅返回 `100` 条；若瞬时并发 `>20` 页连接会被远端重置返回空字节。
- **标准解法**：
  1. **5,576 只全 A 股秒级快照**：主通道采用腾讯财经批量行情接口 `https://qt.gtimg.cn/q=...`（每批 `600` 只代码，10 批并发约 `1.0s` 拉齐全部 `5,576` 只活跃 A 股现价、涨跌幅、成交额、换手率、总市值、流通市值、量比，注意响应编码为 `GBK`，需使用 `encoding_rs::GBK` 解码）；
  2. **东财分页增强**：需要行业分类（`f100`）、上市日期（`f26`）与主力净流入（`f62`）时，严格按 `pz=100` 分页且并发度限制 `<= 8`，或按换手率降序 `fid=f8&po=1` 精准拉取前 10 页（覆盖全市场全部换手率 `> 5%` 标的）。

### Pitfall #08：腾讯 30 分钟 K 线接口 `web.ifzq.gtimg.cn` 302 跳转至失效域名
- **现象**：日 K 线在 `web.ifzq.gtimg.cn` 正常，但请求 `mkline` 分钟线时被 302 重定向到无法解析的内部子域名。
- **标准解法**：30 分钟与日线历史 K 线主源使用东财 `push2his.eastmoney.com/api/qt/stock/kline/get`（直接内含历史精确换手率 `f61`）；腾讯分钟线备源必须使用去掉 `web.` 前缀的 **`https://ifzq.gtimg.cn/appstock/app/kline/mkline`**。

### Pitfall #09：股东户数排雷接口 `RPT_HOLDERNUM_DET` 的公告日字段空值陷阱
- **现象**：按常规 `NOTICE_DATE` 过滤或排序股东户数变化时出现大量 `null`，导致筹码发散排雷漏检或引发未来函数（Look-ahead Bias）。
- **标准解法**：必须使用非空的真实披露日字段 **`HOLD_NOTICE_DATE`** 作为点位时间（PIT）过滤与排序键。

### Pitfall #10：A股“红涨绿跌”极性与止损/排雷危险色的语义冲突
- **现象**：若将排雷否决徽章、止损触发警告与上涨行情共用同一种红色，会导致交易员一眼误将“触及止损/重大利空”看成“上涨利好”。
- **标准解法**：严格解耦 `--mkt-up` / `--mkt-down`（默认 A 股红涨 `#FF453A` / 绿跌 `#30D158`，支持一键切换国际绿涨红跌）与风控语义色（排雷否决固定紫品红 `--risk-veto: #FF2D55`，止损线固定琥珀金 `--risk-stoploss: #FF9F0A`）。

### Pitfall #11：Apple Liquid Glass 多重高斯模糊导致掉帧与中文“文字穿模”
- **现象**：同屏对数十行表格或卡片叠加 `backdrop-filter: blur(...)` 会造成 GPU 合成器掉帧；对中文徽章/标题设置 `line-height: 1` 或固定像素 `height` 结合 `overflow: hidden` 会截断中文括号与英文下伸部，弹性容器漏写 `min-width: 0` 会撑爆相邻数字列。
- **标准解法**：
  1. 单视口真实 `backdrop-filter` 容器严格限制 **`<= 8` 个**，内部子卡片一律使用无模糊的半透明高光光学层 `.glass-optical-2`；
  2. 全局禁用文本元素固定 `height`（改用 `min-height` + `padding-block`），中文 UI 行高下限 `>= 1.28`（正文 `>= 1.65`），所有 Flex/Grid 文本祖先强制 `min-width: 0` + `minmax(0, 1fr)`；
  3. 所有金额、涨跌幅、换手率、倒计时强制启用 `font-variant-numeric: tabular-nums`。

### Pitfall #12：Copilot CLI 子进程唤醒时 MCP 初始化拖慢响应与日志写 C 盘
- **现象**：后台唤醒专家 Agent 若加载默认外部 MCP 插件或使用默认日志路径，会导致单次响应阻塞 `>60s` 并在用户目录写日志。
- **标准解法**：Rust Supervisor 唤醒 Agent 时统一携带 `--model gemini-4-carbon --reasoning-effort high --resume <session_id> --log-dir ./logs/agents --no-custom-instructions --disable-builtin-mcps --disable-client-telemetry`。

### Pitfall #13：绕过 `@tauri-apps/cli` 直接用 `cargo build --release` 构建便携版 `.exe` 时未开启 `custom-protocol`
- **现象**：为满足便携免安装（`bundle.active = false`）并规避外部打包器二进制下载，直接运行 `cargo build --release` 生成独立 `x64` `.exe` 时，若 `tauri` 依赖未开启 `custom-protocol` 特性，`tauri::generate_context!()` 会默认处于开发模式（尝试连接 `devUrl: http://localhost:1420`），导致独立便携 `.exe` 白屏且无法触发前端 IPC 初始化。
- **标准解法**：先执行 `npm run build` 生成 `./dist/`，并在 `./src-tauri/Cargo.toml` 中显式声明 `[features] default = ["custom-protocol"]` 与 `custom-protocol = ["tauri/custom-protocol"]`，使 `tauri-codegen` 将全部前端静态资源静态内嵌至独立便携 `x64` `.exe` 中并通过 `http://tauri.localhost` 协议离线加载。

