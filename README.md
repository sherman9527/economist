# economist

《经济学人》(The Economist) 每日 A股 / 美股 机构级量化图表与深度投研总览系统。

## 核心特性

- **每日双阶段自动调度（Windows 计划任务）**：
  - `08:00` (`EconomistDaily_0800_Analyze`)：自动聚合《经济学人》12 大核心频道最新文章，通过 `Copilot CLI (gemini-4-argon · long_context 983k · high reasoning)` 完成一阶至四阶跨市场传导分析与个股评级。
  - `09:00` (`EconomistDaily_0900_Open`)：启动本地审阅服务 (`127.0.0.1:4288`)，自动在浏览器打开**当日最新报告 + 所有历史未确认已阅 (Un-ACKed) 报告**。
- **高盛 / 大摩卖方量化图表排版**：
  - 交互式实盘日 K 线终端（40 日 OHLCV + 5日/20日均线 + 成交量 + 事件锚点）
  - 华尔街实盘胜率回溯验证、20 日多空对冲超额净值曲线、20日支撑/阻力/止损风控位与预期差背离归因
  - 四维量化看板（累计收益折线图、一级赛道景气柱状图、多空评级双向条形图、四象限散点图）
  - 单篇研报卡内嵌专属 K 线图、关联股涨跌幅柱状图与 20 日 SVG 迷你走势图
- **5 阶段 Checkpoint 断点续传与零 C 盘占用**：
  - 所有脚本、数据、断点、归档报告 (`reports/YYYY/MM/DD/`)、日志及临时缓存均保存在本仓库目录下。

## 目录结构

```text
economist/
├── scripts/
│   ├── config.py            # 全局路径、RSS 频道权重与模型配置
│   ├── fetcher.py           # 多频道 RSS 聚合、去重与正文导语抓取
│   ├── analyzer.py          # 5 阶段 Checkpoint 深度投研分析引擎
│   ├── gs_ms_tables.py      # 投行评级结构化与全中文金融术语本地化
│   ├── market_data.py       # A股/港股/美股 40 日实盘日 K 线抓取与缓存
│   ├── backtester.py        # 华尔街实盘胜率回溯、多空净值与风控位计算
│   ├── renderer.py          # ECharts 可视化中文研报 HTML 渲染器
│   ├── ack_server.py        # 本地 127.0.0.1:4288 报告与审阅状态持久化服务
│   ├── main.py              # CLI 主入口 (analyze / open / both / backfill-3d)
│   ├── run_daily.ps1        # 每日运行包装脚本（含环境隔离与自动重试）
│   └── register_tasks.ps1   # 注册 08:00 与 09:00 Windows 计划任务
├── data/
│   ├── ack_state.json       # 报告归档与已阅 (ACK) 状态注册表
│   ├── backtest_ledger.json # 历史多空胜率与超额收益回溯账本
│   └── runs/                # 按日归档的结构化分析结果 JSON
└── reports/
    ├── index.html           # 历史归档与审阅状态总控台
    ├── assets/              # 本地化 ECharts 图表引擎
    └── YYYY/MM/DD/          # 按年/月/日归档的交互式 HTML 报告
```

## 快速使用

```powershell
# 注册每日 08:00 分析与 09:00 自动打开计划任务
powershell -ExecutionPolicy Bypass -File .\scripts\register_tasks.ps1

# 手动执行当日分析并在浏览器打开（连同未读历史报告）
powershell -ExecutionPolicy Bypass -File .\scripts\run_daily.ps1 -Action Both -Days 2
```
