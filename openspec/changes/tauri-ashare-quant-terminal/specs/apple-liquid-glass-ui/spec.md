## ADDED Requirements

### Requirement: Apple Liquid Glass Three-Tier Visual System and Dual Theme
前端工作台 SHALL 实现遵循 Apple Liquid Glass 规范的三层半透明折射表面体系（固定 `20s+` 缓慢漂移网格渐变背景、`1px` 顶部高光折射边、默认深色曜石海军蓝主题与可切换浅色日光主题），且同一视口内激活真实 `backdrop-filter` 的容器数量 MUST NOT 超过 `8` 个以保障 `60fps` 帧率，同时支持 `prefers-reduced-motion` 与 `prefers-reduced-transparency` 无障碍降级。

#### Scenario: Switch theme and market polarity while preserving 60fps blur budget
- **WHEN** 用户在顶部状态栏切换深色/浅色主题或切换“红涨绿跌 / 绿涨红跌”行情极性
- **THEN** 全局玻璃表面、ECharts 图表与 K 线涨跌配色即时同步更新，风控止损琥珀色与排雷否决品红色保持语义独立，且视口激活模糊层计数保持 `<= 8`

### Requirement: Responsive Multi-Size Layout with Zero Text Clipping
界面 MUST 统一全局字体与字号阶梯，适配从紧凑窄窗到宽屏高分显示器及系统 `125%`/`150%` DPI 缩放，禁止文本承载元素使用固定像素高度切割字形，强制所有金融数字启用等宽对齐（`tabular-nums`），确保在任何窗口尺寸下均不发生中英文“文字穿模”、重叠或破框溢出。

#### Scenario: Render extremal stock names and metrics across narrow and desktop viewports
- **WHEN** 窗口宽度在紧凑断点与宽屏桌面断点间调整，且界面渲染含 `*ST` 前缀的长中文简称、长研报标题与大幅涨跌金额
- **THEN** 所有弹性网格列保持边界不溢出、中文与下伸字母不被上下裁切、数值列小数点垂直对齐，被省略长文本悬停时通过单实例玻璃浮层完整展示

### Requirement: Interactive K-Line Chart with Buy-Point Markers and Stop-Loss Price Lines
应用 MUST 内置交互式蜡烛图（K 线）与成交量副图终端，并在对应标的 K 线上直观绘制量化专家 Agent 产出的历史与最新买点锚点标记及止损价格水平线，支持点击买点查看持久化于 SQLite 的详细推荐理由与五维排雷核验结果。

#### Scenario: Inspect recommended stock on K-line terminal
- **WHEN** 用户在总览页、全 A 股轮动雷达或持仓账本中选中任意获推荐或持仓股票
- **THEN** K 线终端渲染该股实时与历史蜡烛图、均线与成交量柱，并在推荐触发 K 线柱标注买点价格箭头、绘制琥珀色止损水平虚线，点击买点即浮出完整推荐理由卡片

### Requirement: Trading-Hours 30-Minute Rotation Recommendation Alert Popup
在 A 股法定交易时段（周一至周五 `09:30–11:30`、`13:00–15:00`）内，系统 SHALL 每 `30` 分钟自动执行一轮全市场策略轮动扫描；一旦出现通过核心策略与五维排雷的新增推荐标的，界面 MUST 即时弹出非阻塞式玻璃态提醒卡片。

#### Scenario: Qualifying stock discovered during 30-minute intraday rotation
- **WHEN** 交易时段 30 分钟轮动任务完成全市场扫描并产出至少 `1` 只满足 `< 200 亿市值 + 连续 2 日换手 > 5% 放量 + 五维排雷通过` 的推荐股票
- **THEN** 桌面端右上角弹出 Tier 3 液态玻璃推荐提醒，展示股票代码/简称、现价、建议止损价与核心推荐理由摘要，并提供一键定位 K 线买点与记入模拟盘入口
