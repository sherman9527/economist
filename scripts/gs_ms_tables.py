# -*- coding: utf-8 -*-
"""
Goldman Sachs (GS GIR) & Morgan Stanley (MS Research) Sell-Side Structuring Engine.
Transforms dense narrative paragraphs and ticker lists into:
  - Concise, high-readability 3-point Key Takeaways with bold tags
  - Single-stock institutional coverage rows (Ticker, Name, GS/MS Rating, EPS/PE Driver, Score, Concise Thesis)
  - Sector Allocation & EPS Revision Scorecard Table
  - Cross-Market Long/Short Pair-Trade Table
"""

import html
import re


BULL_KEYWORDS = [
    "强推", "超配", "受益", "利好", "放量", "改善", "领跑", "赢家", "承接", "避险溢价",
    "红利资金", "议价空间", "护城河", "显著优于", "反击", "份额提升", "顺畅交付",
    "消除", "延续", "跑赢", "刚性扩张", "垄断", "高景气", "确定性", "修复", "突围",
    "壁垒", "豪赚", "支撑", "提振", "扩张", "首选", "逆周期布局", "缓释", "解除",
    "反弹", "先锋", "粘性", "集中采购", "刚需", "点名核心标的", "避险属性",
]

BEAR_KEYWORDS = [
    "规避", "低配", "减持", "承压", "拖累", "暴跌", "重挫", "杀估值", "双杀", "暴雷",
    "坏账", "不良", "枯竭", "诉讼", "挫折", "受阻", "消退", "蒙尘", "萎缩", "挤压",
    "泥潭", "抛售", "侵蚀", "下调", "恶化", "违约", "减值", "内卷", "冲击", "警示",
    "放缓", "受制", "重估压力", "回吐", "获利了结", "杀跌", "流血", "蚕食",
]


EN_COMPANY_TO_ZH = {
    "Palo Alto Networks": "派拓网络",
    "Palo Alto": "派拓网络",
    "CrowdStrike": "众击安全",
    "Zscaler": "云安全龙头",
    "Fortinet": "飞塔信息",
    "DoorDash": "北美外卖龙头",
    "Instacart": "杂货配送龙头",
    "Uber Eats": "优步外卖",
    "Constellation Energy": "星座核电",
    "Constellation": "星座核电",
    "Vistra": "维斯特拉电力",
    "NextEra Energy": "新纪元能源",
    "First Solar": "第一太阳能",
    "Enphase Energy": "微型逆变器龙头",
    "Enphase": "微型逆变器龙头",
    "Freeport-McMoRan": "自由港铜业",
    "Southern Copper": "南方铜业",
    "Salesforce": "赛富时云软件",
    "ServiceNow": "数字化工作流",
    "Arista Networks": "阿里斯塔网络",
    "Vertiv": "维谛液冷技术",
    "Synchrony Financial": "同步金融信贷",
    "Synchrony": "同步金融信贷",
    "Capital One": "第一资本信贷",
    "Ally Financial": "盟友金融",
    "Equity Residential": "权益住宅房产",
    "Invitation Homes": "独栋出租房产",
    "Hims & Hers": "远程医疗龙头",
    "Eli Lilly": "礼来制药",
    "UnitedHealth": "联合健康",
    "Cigna": "信诺集团",
    "Nutrien": "纽崔恩钾肥",
    "Brookfield": "博枫资管",
    "Coinbase": "加密交易平台",
    "Warner Bros. Discovery": "华纳兄弟探索",
    "Paramount": "派拉蒙环球",
    "Netflix": "奈飞流媒体",
    "Meta": "脸书母公司",
    "Palantir": "帕兰提尔大数据",
    "Super Micro": "超微电脑",
    "Thermo Fisher": "赛默飞世尔",
    "Danaher": "丹纳赫生命科学",
    "BellRing Brands": "贝尔林营养品",
    "BellRing Brand": "贝尔林营养品",
    "Life Time Group": "高端连锁健身",
    "Charles River": "查尔斯河实验室",
    "The Trade Desk": "萃弈广告平台",
    "Trade Desk": "萃弈广告平台",
    "Cost Plus Drugs": "平价直销药房",
    "Cost Plus Drug": "平价直销药房",
    "Saudi Aramco": "沙特阿美",
    "William Nordhaus": "威廉·诺德豪斯",
    "Doximity": "医生云社区",
    "Teladoc": "泰拉多克远程医疗",
}

EN_TERM_REPLACEMENTS = [
    (r"Almighty El Ni[ñn]o", "超级厄尔尼诺"),
    (r"\bEl Ni[ñn]o\b", "厄尔尼诺"),
    (r"\bAI for Science\b", "科学智能"),
    (r"\bAI Agent\b", "智能体"),
    (r"\bPrecision AI\b", "精准智能"),
    (r"\bRobotaxi\b", "无人驾驶出租车"),
    (r"\bUber Eats\b", "优步外卖"),
    (r"\bDrill, baby, drill\b", "传统油气扩产"),
    (r"\bButtonwood\b", "梧桐树金融专栏"),
    (r"\bCapex\b|\bCapEx\b", "资本开支"),
    (r"\bROIC\b", "投入资本回报率"),
    (r"\bARR\b", "经常性收入"),
    (r"\bPPA\b", "购电协议"),
    (r"\bPBM\b", "药品福利管理"),
    (r"\bDTC\b", "直面消费者模式"),
    (r"\bREITs\b|\bREIT\b", "不动产信托"),
    (r"\bFTC\b", "联邦贸易委员会"),
    (r"\bIRA\b", "通胀削减法案"),
    (r"\bERP\b", "风险溢价"),
    (r"\bBeta\b", "弹性"),
    (r"\bAlpha\b", "超额收益"),
    (r"\bSaaS\b", "云软件"),
    (r"\bPUGC\b", "专业用户创作"),
    (r"\bCTV\b", "联网电视"),
    (r"\bVLCC\b", "超大型油轮"),
    (r"\bCXO\b", "医药研发外包"),
    (r"\bAMC\b", "不良资产管理"),
    (r"\bETF\b", "指数基金"),
    (r"\bADR\b", "美股存托股"),
    (r"\bAIGC\b", "生成式人工智能"),
    (r"\bFSD\b", "全自动驾驶"),
    (r"\bEPS\b", "每股收益"),
    (r"\bP/E\b|\bPE\b", "市盈率估值"),
    (r"\bGPU\b", "算力芯片"),
    (r"\bAPI\b", "接口调用"),
    (r"\bPCB\b", "印制电路板"),
    (r"\bNOAA\b", "美国海洋大气局"),
    (r"\bSkydance\b", "天空之舞影业"),
    (r"\bTemu\b", "拼多多跨境"),
    (r"\bYouTube\b", "油管视频平台"),
    (r"\bOpenAI\b", "开放人工智能公司"),
    (r"\bMag-7\b", "美股科技七巨头"),
    (r"\bC端\b", "消费端"),
    (r"\bB端\b", "企业端"),
    (r"\bK型\b", "两极分化型"),
]


def localize_zh_text(text: str) -> str:
    """Replace English company names and financial jargon with Chinese while keeping stock ticker symbols."""
    if not text:
        return ""
    out = str(text)
    for en_name, zh_name in sorted(EN_COMPANY_TO_ZH.items(), key=lambda x: len(x[0]), reverse=True):
        out = re.sub(re.escape(en_name), zh_name, out, flags=re.I)
    for pat, repl in EN_TERM_REPLACEMENTS:
        out = re.sub(pat, repl, out, flags=re.I)
    # Remove redundant parenthetical English glosses after Chinese terms, e.g. 困境资产（Distressed Debt）
    out = re.sub(r"[（\(]\s*[A-Za-z][A-Za-z0-9\s\-,&'/]{3,45}\s*[）\)]", "", out)
    return out


def localize_stock_head(head: str) -> str:
    """Ensure company descriptions in parentheses are Chinese while preserving stock ticker symbols."""
    if not head:
        return ""
    out = str(head)
    for en_name, zh_name in sorted(EN_COMPANY_TO_ZH.items(), key=lambda x: len(x[0]), reverse=True):
        out = re.sub(r"\(\s*" + re.escape(en_name) + r"\s*\)", f"({zh_name})", out, flags=re.I)
        out = re.sub(re.escape(en_name), zh_name, out, flags=re.I)
    out = re.sub(r"\bETF\b", "指数基金", out)
    out = re.sub(r"\bADR\b", "存托股", out)
    return out


def _esc(val: any) -> str:
    if val is None:
        return ""
    return html.escape(str(val))


def split_to_bullets(text: str, labels: list[str] = None, max_items: int = 3) -> list[tuple[str, str]]:
    """
    Split a dense Chinese paragraph into 2-4 crisp, scannable bullet points with bold lead-in tags.
    Eliminates walls of text and localizes English terms to Chinese.
    """
    raw = localize_zh_text((text or "").strip())
    if not raw:
        return []

    # Split by Chinese period or semicolon
    parts = [p.strip() for p in re.split(r"[。；]+", raw) if len(p.strip()) > 6]
    if not parts:
        parts = [raw]

    # Group into at most max_items balanced chunks
    if len(parts) > max_items:
        grouped = []
        chunk_size = max(1, (len(parts) + max_items - 1) // max_items)
        for i in range(0, len(parts), chunk_size):
            chunk = "；".join(parts[i : i + chunk_size])
            if not chunk.endswith("。"):
                chunk += "。"
            grouped.append(chunk)
        parts = grouped[:max_items]
    else:
        parts = [p + ("。" if not p.endswith("。") else "") for p in parts]

    default_labels = labels or ["核心事实", "传导机制", "定价启示", "边际推演"]
    result = []
    for idx, p in enumerate(parts):
        lbl = default_labels[idx] if idx < len(default_labels) else f"要点 {idx+1}"
        result.append((lbl, p))
    return result


def infer_stock_rating_and_driver(
    thesis_text: str,
    market_direction: str,
    market_score: int,
) -> tuple[str, str, str, str]:
    """
    Infer Goldman Sachs / Morgan Stanley sell-side rating, CSS class, polarity ('bull'|'bear'|'neutral'),
    and primary fundamental driver ('EPS 盈利弹性' | 'P/E 估值重估' | 'CapEx/订单份额' | '资产负债表/信用')
    from the stock's specific rationale and market context.
    """
    txt = thesis_text or ""
    cleaned_for_bear = re.sub(r"(缓释|解除|消除|对冲|抵消)[^，。；]{0,12}(压力|风险|冲击|成本)", "", txt)
    bull_hits = sum(1 for kw in BULL_KEYWORDS if kw in txt)
    bear_hits = sum(1 for kw in BEAR_KEYWORDS if kw in cleaned_for_bear)

    # Explicit bracket rating override if present
    if "Conviction Buy" in txt or "强推" in txt:
        polarity = "bull"
    elif "UW" in txt or "规避" in txt or "减持" in txt:
        polarity = "bear"
    elif "OW" in txt or "超配" in txt:
        polarity = "bull"
    elif bull_hits > bear_hits:
        polarity = "bull"
    elif bear_hits > bull_hits:
        polarity = "bear"
    else:
        if "利好" in market_direction:
            polarity = "bull"
        elif "利空" in market_direction:
            polarity = "bear"
        else:
            polarity = "neutral"

    if polarity == "bull":
        if market_score >= 8 and bull_hits >= 1 and bear_hits == 0:
            rating = "强力买入"
            badge_cls = "gs-rating-cbuy"
        else:
            rating = "超配增持"
            badge_cls = "gs-rating-ow"
    elif polarity == "bear":
        if market_score >= 8 and bear_hits >= 1 and bull_hits == 0:
            rating = "强力规避"
            badge_cls = "gs-rating-uw-strong"
        else:
            rating = "低配减持"
            badge_cls = "gs-rating-uw"
    else:
        rating = "标配观望"
        badge_cls = "gs-rating-ew"

    # Determine primary fundamental driver in pure Chinese
    if any(k in txt for k in ["不良", "坏账", "拨备", "逾期", "违约", "减值", "负债", "资本充足"]):
        driver = ("资产质量 / 信用风险" if polarity == "bear" else "化债扩张 / 防御红利")
    elif any(k in txt for k in ["订单", "份额", "产能", "交付", "出海", "替代", "并购", "协同", "采购"]):
        driver = ("订单放量 / 份额扩张" if polarity != "bear" else "份额流失 / 竞争挤压")
    elif any(k in txt for k in ["估值", "折现", "溢价", "Beta", "利率", "ERP", "期权", "回吐"]):
        driver = ("估值倍数修复" if polarity == "bull" else "估值倍数收缩")
    elif any(k in txt for k in ["毛利", "净利", "利润", "盈利", "ARR", "降价", "价格战", "成本"]):
        driver = ("盈利预期上修" if polarity == "bull" else "盈利利润率承压")
    else:
        driver = ("盈利与估值双升" if polarity == "bull" else ("盈利与估值双杀" if polarity == "bear" else "基本面结构分化"))

    return rating, badge_cls, polarity, driver


def parse_ticker_entry(
    raw_ticker_str: str,
    market_type: str,
    market_direction: str,
    market_score: int,
    article_idx: int,
    article_zh_title: str,
    theme_category: str,
) -> dict:
    """
    Parse a raw ticker string into a structured GS/MS equity table row dict.
    Handles both:
      - "FXI (中国大盘股ETF - 金融权重占比高...)"
      - "MSFT (微软) / GOOGL (谷歌) - 巨额AI资本开支..."
      - "上海银行 (601229.SH) - 宝能系等历史大额..."
    """
    s = (raw_ticker_str or "").strip()
    head = s
    thesis = s

    # Case 1: Explicit " - " separator outside or inside parens
    if " - " in s:
        parts = s.split(" - ", 1)
        left, right = parts[0].strip(), parts[1].strip()
        # Check if left has an unclosed '(' because " - " was inside "(Name - Thesis)"
        if left.count("(") > left.count(")") and right.endswith(")"):
            right = right[:-1].strip()
            sub_parts = left.split("(", 1)
            symbol_part = sub_parts[0].strip()
            name_part = sub_parts[1].strip()
            head = f"{symbol_part} ({name_part})" if name_part else symbol_part
            thesis = right
        else:
            head = left
            thesis = right
    elif "：" in s:
        parts = s.split("：", 1)
        head, thesis = parts[0].strip(), parts[1].strip()

    # Clean up symbol & name display
    codes = re.findall(r"\b[0-9]{6}\.(?:SH|SZ|BJ)\b|\b[0-9]{4,5}\.HK\b|\b[A-Z]{2,5}\b", head)
    symbol_display = " / ".join(dict.fromkeys(codes[:3])) if codes else head[:18]

    rating, badge_cls, polarity, driver = infer_stock_rating_and_driver(
        thesis, market_direction, market_score
    )

    head_localized = localize_stock_head(head)
    thesis_localized = localize_zh_text(thesis)

    return {
        "market_type": market_type,  # "US" or "CN"
        "market_label": "🇺🇸 美股" if market_type == "US" else "🇨🇳 A股/港股",
        "head_display": head_localized,
        "symbol_display": symbol_display,
        "thesis": thesis_localized,
        "rating": rating,
        "badge_cls": badge_cls,
        "polarity": polarity,
        "driver": driver,
        "score": int(market_score or 5),
        "article_idx": article_idx,
        "article_anchor": f"#art-{article_idx}",
        "article_title": localize_zh_text(article_zh_title),
        "theme_category": theme_category,
    }


def extract_all_structured_stocks(core_articles: list[dict]) -> list[dict]:
    """Extract all US and CN/HK stock rows across all core articles."""
    all_rows = []
    for idx, art in enumerate(core_articles, 1):
        zh_title = art.get("zh_title") or art.get("title") or f"文章 #{idx}"
        cat = art.get("theme_category") or "宏观与产业"

        us_m = art.get("us_market") or {}
        us_dir = us_m.get("direction") or "中性分化"
        us_sc = int(us_m.get("score") or 5)
        for t_str in us_m.get("tickers") or []:
            row = parse_ticker_entry(t_str, "US", us_dir, us_sc, idx, zh_title, cat)
            all_rows.append(row)

        cn_m = art.get("cn_market") or {}
        cn_dir = cn_m.get("direction") or "中性分化"
        cn_sc = int(cn_m.get("score") or 5)
        for t_str in cn_m.get("tickers") or []:
            row = parse_ticker_entry(t_str, "CN", cn_dir, cn_sc, idx, zh_title, cat)
            all_rows.append(row)

    # Sort by score descending, then bull/bear priority
    all_rows.sort(key=lambda r: (r["score"], 1 if r["polarity"] == "bull" else 0), reverse=True)
    return all_rows


def build_sector_scorecard_rows(core_articles: list[dict], stock_rows: list[dict]) -> list[dict]:
    """
    Aggregate articles and stocks by theme_category into a Goldman Sachs / Morgan Stanley
    Sector Allocation & Conviction Scorecard Table.
    """
    by_cat: dict[str, dict] = {}
    for idx, art in enumerate(core_articles, 1):
        cat = art.get("theme_category") or "宏观与产业"
        entry = by_cat.setdefault(
            cat,
            {
                "category": cat,
                "article_indices": [],
                "us_scores": [],
                "cn_scores": [],
                "us_dirs": [],
                "cn_dirs": [],
                "us_sectors": [],
                "cn_sectors": [],
                "us_bull_stocks": [],
                "us_bear_stocks": [],
                "cn_bull_stocks": [],
                "cn_bear_stocks": [],
            },
        )
        entry["article_indices"].append(idx)
        us_m = art.get("us_market") or {}
        cn_m = art.get("cn_market") or {}
        entry["us_scores"].append(int(us_m.get("score") or 5))
        entry["cn_scores"].append(int(cn_m.get("score") or 5))
        entry["us_dirs"].append(us_m.get("direction") or "中性分化")
        entry["cn_dirs"].append(cn_m.get("direction") or "中性分化")
        for s in (us_m.get("sectors") or [])[:2]:
            if s not in entry["us_sectors"]:
                entry["us_sectors"].append(s)
        for s in (cn_m.get("sectors") or [])[:2]:
            if s not in entry["cn_sectors"]:
                entry["cn_sectors"].append(s)

    for st in stock_rows:
        cat = st["theme_category"]
        if cat not in by_cat:
            continue
        entry = by_cat[cat]
        short_name = st["head_display"]
        if st["market_type"] == "US":
            if st["polarity"] == "bull" and short_name not in entry["us_bull_stocks"]:
                entry["us_bull_stocks"].append(short_name)
            elif st["polarity"] == "bear" and short_name not in entry["us_bear_stocks"]:
                entry["us_bear_stocks"].append(short_name)
        else:
            if st["polarity"] == "bull" and short_name not in entry["cn_bull_stocks"]:
                entry["cn_bull_stocks"].append(short_name)
            elif st["polarity"] == "bear" and short_name not in entry["cn_bear_stocks"]:
                entry["cn_bear_stocks"].append(short_name)

    def _agg_stance(dirs: list[str], bull_list: list, bear_list: list) -> tuple[str, str]:
        bull_c = sum(1 for d in dirs if "利好" in d) + len(bull_list)
        bear_c = sum(1 for d in dirs if "利空" in d) + len(bear_list)
        if bull_c > bear_c + 1:
            return "超配看多", "gs-rating-ow"
        if bear_c > bull_c + 1:
            return "低配规避", "gs-rating-uw"
        return "标配分化", "gs-rating-ew"

    scorecard = []
    for cat, info in by_cat.items():
        us_avg = round(sum(info["us_scores"]) / max(1, len(info["us_scores"])), 1)
        cn_avg = round(sum(info["cn_scores"]) / max(1, len(info["cn_scores"])), 1)
        us_stance, us_cls = _agg_stance(info["us_dirs"], info["us_bull_stocks"], info["us_bear_stocks"])
        cn_stance, cn_cls = _agg_stance(info["cn_dirs"], info["cn_bull_stocks"], info["cn_bear_stocks"])
        scorecard.append(
            {
                "category": cat,
                "article_count": len(info["article_indices"]),
                "article_links": info["article_indices"],
                "us_stance": us_stance,
                "us_cls": us_cls,
                "us_avg_score": us_avg,
                "us_subsectors": " / ".join(info["us_sectors"][:3]),
                "us_bull": "；".join(info["us_bull_stocks"][:3]) or "精选防御龙头",
                "us_bear": "；".join(info["us_bear_stocks"][:3]) or "高估值无业绩标的",
                "cn_stance": cn_stance,
                "cn_cls": cn_cls,
                "cn_avg_score": cn_avg,
                "cn_subsectors": " / ".join(info["cn_sectors"][:3]),
                "cn_bull": "；".join(info["cn_bull_stocks"][:3]) or "结构性景气龙头",
                "cn_bear": "；".join(info["cn_bear_stocks"][:3]) or "尾部风险敞口标的",
            }
        )

    scorecard.sort(key=lambda x: (x["article_count"], x["us_avg_score"] + x["cn_avg_score"]), reverse=True)
    return scorecard
