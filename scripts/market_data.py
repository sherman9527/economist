# -*- coding: utf-8 -*-
"""
Real-Time & Historical Market K-Line (OHLCV) Fetcher for US, A-Share, and HK Equities.
Fetches 40-day daily candlestick data (Open, Close, Low, High, Volume), computes MA5/MA20,
1D/5D/20D returns, and caches under ./data/market_cache (Zero C: drive usage).
"""

import json
import re
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from pathlib import Path
from typing import Callable, Optional

from config import DATA_DIR

MARKET_CACHE_DIR = DATA_DIR / "market_cache"
MARKET_CACHE_DIR.mkdir(parents=True, exist_ok=True)

USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"

# Core benchmark indices & pre-mapped popular symbols for instant resolution
BENCHMARK_SYMBOLS = [
    {"symbol": "SPY", "qt_code": "usSPY.AM", "name": "标普500指数 (SPY)", "market": "US"},
    {"symbol": "QQQ", "qt_code": "usQQQ.OQ", "name": "纳斯达克100 (QQQ)", "market": "US"},
    {"symbol": "TLT", "qt_code": "usTLT.OQ", "name": "20年期美债 (TLT)", "market": "US"},
    {"symbol": "000001.SH", "qt_code": "sh000001", "name": "上证指数 (000001.SH)", "market": "CN"},
    {"symbol": "399006.SZ", "qt_code": "sz399006", "name": "创业板指 (399006.SZ)", "market": "CN"},
    {"symbol": "NVDA", "qt_code": "usNVDA.OQ", "name": "英伟达 (NVDA)", "market": "US"},
    {"symbol": "MSFT", "qt_code": "usMSFT.OQ", "name": "微软 (MSFT)", "market": "US"},
    {"symbol": "GOOGL", "qt_code": "usGOOGL.OQ", "name": "谷歌 (GOOGL)", "market": "US"},
    {"symbol": "PANW", "qt_code": "usPANW.OQ", "name": "派拓网络 (PANW)", "market": "US"},
    {"symbol": "FCX", "qt_code": "usFCX.N", "name": "自由港铜业 (FCX)", "market": "US"},
    {"symbol": "FSLR", "qt_code": "usFSLR.OQ", "name": "第一太阳能 (FSLR)", "market": "US"},
    {"symbol": "WMT", "qt_code": "usWMT.OQ", "name": "沃尔玛 (WMT)", "market": "US"},
    {"symbol": "CVS", "qt_code": "usCVS.N", "name": "西维斯健康 (CVS)", "market": "US"},
    {"symbol": "XOM", "qt_code": "usXOM.N", "name": "埃克森美孚 (XOM)", "market": "US"},
    {"symbol": "300308.SZ", "qt_code": "sz300308", "name": "中际旭创 (300308.SZ)", "market": "CN"},
    {"symbol": "002463.SZ", "qt_code": "sz002463", "name": "沪电股份 (002463.SZ)", "market": "CN"},
    {"symbol": "601899.SH", "qt_code": "sh601899", "name": "紫金矿业 (601899.SH)", "market": "CN"},
    {"symbol": "603259.SH", "qt_code": "sh603259", "name": "药明康德 (603259.SH)", "market": "CN"},
    {"symbol": "02228.HK", "qt_code": "hk02228", "name": "晶泰控股 (02228.HK)", "market": "CN"},
    {"symbol": "601229.SH", "qt_code": "sh601229", "name": "上海银行 (601229.SH)", "market": "CN"},
    {"symbol": "601398.SH", "qt_code": "sh601398", "name": "工商银行 (601398.SH)", "market": "CN"},
    {"symbol": "300750.SZ", "qt_code": "sz300750", "name": "宁德时代 (300750.SZ)", "market": "CN"},
    {"symbol": "688041.SH", "qt_code": "sh688041", "name": "海光信息 (688041.SH)", "market": "CN"},
]

KNOWN_QT_MAP = {
    "SPY": ("usSPY.AM", "标普500指数"),
    "QQQ": ("usQQQ.OQ", "纳斯达克100"),
    "TLT": ("usTLT.OQ", "20年期美债"),
    "NVDA": ("usNVDA.OQ", "英伟达"),
    "AVGO": ("usAVGO.OQ", "博通"),
    "MSFT": ("usMSFT.OQ", "微软"),
    "GOOGL": ("usGOOGL.OQ", "谷歌"),
    "META": ("usMETA.OQ", "脸书母公司"),
    "AMZN": ("usAMZN.OQ", "亚马逊"),
    "TSLA": ("usTSLA.OQ", "特斯拉"),
    "WMT": ("usWMT.OQ", "沃尔玛"),
    "TGT": ("usTGT.N", "塔吉特百货"),
    "COST": ("usCOST.OQ", "开市客"),
    "PANW": ("usPANW.OQ", "派拓网络"),
    "CRWD": ("usCRWD.OQ", "众击安全"),
    "ZS": ("usZS.OQ", "云安全龙头"),
    "FTNT": ("usFTNT.OQ", "飞塔信息"),
    "FCX": ("usFCX.N", "自由港铜业"),
    "SCCO": ("usSCCO.N", "南方铜业"),
    "NEM": ("usNEM.N", "纽蒙特矿业"),
    "XOM": ("usXOM.N", "埃克森美孚"),
    "CVX": ("usCVX.N", "雪佛龙"),
    "HAL": ("usHAL.N", "哈里伯顿"),
    "SLB": ("usSLB.N", "斯伦贝谢"),
    "CVS": ("usCVS.N", "西维斯健康"),
    "UNH": ("usUNH.N", "联合健康"),
    "CI": ("usCI.N", "信诺集团"),
    "HIMS": ("usHIMS.N", "远程医疗龙头"),
    "LLY": ("usLLY.N", "礼来制药"),
    "DASH": ("usDASH.OQ", "北美外卖龙头"),
    "CART": ("usCART.OQ", "杂货配送龙头"),
    "UBER": ("usUBER.N", "优步出行"),
    "FSLR": ("usFSLR.OQ", "第一太阳能"),
    "ENPH": ("usENPH.OQ", "微型逆变器龙头"),
    "NEE": ("usNEE.N", "新纪元能源"),
    "CEG": ("usCEG.OQ", "星座核电"),
    "VST": ("usVST.N", "维斯特拉电力"),
    "JPM": ("usJPM.N", "摩根大通"),
    "GS": ("usGS.N", "高盛集团"),
    "COF": ("usCOF.N", "第一资本信贷"),
    "SYF": ("usSYF.N", "同步金融信贷"),
    "ALLY": ("usALLY.N", "盟友金融"),
    "EQR": ("usEQR.N", "权益住宅房产"),
    "INVH": ("usINVH.N", "独栋出租房产"),
    "FXI": ("usFXI.AM", "中国大盘指数"),
    "HSBC": ("usHSBC.N", "汇丰控股"),
    "BAM": ("usBAM.N", "博枫资管"),
    "OAK": ("usBAM.N", "橡树资本"),
    "CRM": ("usCRM.N", "赛富时云软件"),
    "NOW": ("usNOW.N", "数字化工作流"),
    "ANET": ("usANET.N", "阿里斯塔网络"),
    "DELL": ("usDELL.N", "戴尔科技"),
    "TSM": ("usTSM.N", "台积电"),
    "ASML": ("usASML.OQ", "阿斯麦光刻机"),
    "VRT": ("usVRT.N", "维谛液冷技术"),
    "ETN": ("usETN.N", "伊顿电力设备"),
    "NFLX": ("usNFLX.OQ", "奈飞流媒体"),
    "DIS": ("usDIS.N", "迪士尼"),
    "WBD": ("usWBD.OQ", "华纳兄弟探索"),
    "PARA": ("usPARA.OQ", "派拉蒙环球"),
    "COIN": ("usCOIN.OQ", "加密交易平台"),
    "DJT": ("usDJT.OQ", "特朗普媒体集团"),
    "NTR": ("usNTR.N", "纽崔恩钾肥"),
    "ADM": ("usADM.N", "阿彻丹尼尔斯粮商"),
    "CF": ("usCF.N", "CF工业氮肥"),
    "BG": ("usBG.N", "邦吉粮商"),
    "DAL": ("usDAL.N", "达美航空"),
    "KHC": ("usKHC.N", "卡夫亨氏食品"),
    "PLTR": ("usPLTR.OQ", "帕兰提尔大数据"),
    "SMCI": ("usSMCI.OQ", "超微电脑"),
    "ADBE": ("usADBE.OQ", "奥多比软件"),
    "TMO": ("usTMO.N", "赛默飞世尔"),
    "DHR": ("usDHR.N", "丹纳赫生命科学"),
    "LTH": ("usLTH.N", "高端健身连锁"),
    "AI": ("usAI.N", "C3人工智能"),
    "AMD": ("usAMD.OQ", "超威半导体"),
    "MRVL": ("usMRVL.OQ", "迈威尔科技"),
    "ARM": ("usARM.OQ", "安谋控股"),
    "INTC": ("usINTC.OQ", "英特尔"),
    "MU": ("usMU.OQ", "美光科技"),
    "AMAT": ("usAMAT.OQ", "应用材料"),
    "LRCX": ("usLRCX.OQ", "泛林集团"),
    "SNPS": ("usSNPS.OQ", "新思科技"),
    "CDNS": ("usCDNS.OQ", "楷登电子"),
    "APP": ("usAPP.OQ", "移动广告平台"),
    "ARES": ("usARES.N", "阿瑞斯资管"),
    "OWL": ("usOWL.N", "蓝鸮私募信贷"),
    "BX": ("usBX.N", "黑石集团"),
    "KKR": ("usKKR.N", "KKR资管"),
    "CME": ("usCME.OQ", "芝加哥商品交易所"),
    "CBOE": ("usCBOE.AM", "芝加哥期权交易所"),
    "ICE": ("usICE.N", "洲际交易所"),
    "SCHW": ("usSCHW.N", "嘉信理财"),
    "HOOD": ("usHOOD.OQ", "罗宾汉证券"),
    "WELL": ("usWELL.N", "韦尔塔医疗房产"),
    "VTR": ("usVTR.N", "文塔斯医疗房产"),
    "THO": ("usTHO.N", "索尔房车"),
    "HOG": ("usHOG.N", "哈雷戴维森"),
    "CCL": ("usCCL.N", "嘉年华邮轮"),
    "RCL": ("usRCL.N", "皇家加勒比邮轮"),
    "PDD": ("usPDD.OQ", "拼多多"),
    "BABA": ("usBABA.N", "阿里巴巴"),
    "GLD": ("usGLD.AM", "黄金指数基金"),
    "XLF": ("usXLF.AM", "美国金融指数基金"),
    "IGV": ("usIGV.AM", "北美软件指数基金"),
    "SPHQ": ("usSPHQ.AM", "标普高质量指数基金"),
    "RSP": ("usRSP.AM", "标普等权指数基金"),
    "KWEB": ("usKWEB.AM", "中概互联网指数基金"),
    "KRE": ("usKRE.AM", "美国区域银行指数基金"),
    "LMT": ("usLMT.N", "洛克希德马丁"),
    "NOC": ("usNOC.N", "诺斯罗普格鲁曼"),
    "GD": ("usGD.N", "通用动力"),
    "RTX": ("usRTX.N", "雷神技术"),
    "KTOS": ("usKTOS.OQ", "克拉托斯防务"),
    "AVAV": ("usAVAV.OQ", "宇航环境无人机"),
    "SBUX": ("usSBUX.OQ", "星巴克"),
    "MCD": ("usMCD.N", "麦当劳"),
    "CMG": ("usCMG.N", "墨式烧烤"),
    "DRI": ("usDRI.N", "达登餐饮"),
    "YUM": ("usYUM.N", "百胜餐饮"),
    "NKE": ("usNKE.N", "耐克"),
    "LULU": ("usLULU.OQ", "露露乐蒙"),
    "TOL": ("usTOL.N", "托尔兄弟地产"),
    "LEN": ("usLEN.N", "莱纳建筑地产"),
    "UAL": ("usUAL.OQ", "美联航"),
    "PWR": ("usPWR.N", "广达电力工程"),
    "GEV": ("usGEV.N", "通用电气能源"),
    "DE": ("usDE.N", "迪尔农机"),
    "CAT": ("usCAT.N", "卡特彼勒"),
    "BIIB": ("usBIIB.OQ", "百健生物"),
    "REGN": ("usREGN.OQ", "再生元制药"),
    "ABBV": ("usABBV.N", "艾伯维制药"),
    "PFE": ("usPFE.N", "辉瑞制药"),
    "BMY": ("usBMY.N", "百时美施贵宝"),
    "NVO": ("usNVO.N", "诺和诺德"),
    "DNLI": ("usDNLI.OQ", "德纳利医药"),
    "TTD": ("usTTD.OQ", "萃弈广告平台"),
    "SNOW": ("usSNOW.N", "雪花云数据"),
    "DDOG": ("usDDOG.OQ", "数据狗云监控"),
    "NET": ("usNET.N", "云フレア网络"),
    "OKTA": ("usOKTA.OQ", "奥克塔身份安全"),
    "F": ("usF.N", "福特汽车"),
    "GM": ("usGM.N", "通用汽车"),
    "O": ("usO.N", "实在收益地产"),
    "U": ("usU.N", "优尼蒂引擎"),
    "W": ("usW.N", "韦费尔家居"),
    "S": ("usS.N", "哨兵一号安全"),
}

CN_NAME_QT_MAP = {
    "宁德时代": ("300750.SZ", "sz300750", "宁德时代"),
    "比亚迪": ("002594.SZ", "sz002594", "比亚迪"),
    "寒武纪": ("688256.SH", "sh688256", "寒武纪"),
    "海光信息": ("688041.SH", "sh688041", "海光信息"),
    "中芯国际": ("688981.SH", "sh688981", "中芯国际"),
    "工业富联": ("601138.SH", "sh601138", "工业富联"),
    "中际旭创": ("300308.SZ", "sz300308", "中际旭创"),
    "新易盛": ("300502.SZ", "sz300502", "新易盛"),
    "沪电股份": ("002463.SZ", "sz002463", "沪电股份"),
    "浪潮信息": ("000977.SZ", "sz000977", "浪潮信息"),
    "北方华创": ("002371.SZ", "sz002371", "北方华创"),
    "金山办公": ("688111.SH", "sh688111", "金山办公"),
    "科大讯飞": ("002230.SZ", "sz002230", "科大讯飞"),
    "紫金矿业": ("601899.SH", "sh601899", "紫金矿业"),
    "中国神华": ("601088.SH", "sh601088", "中国神华"),
    "中国海油": ("600938.SH", "sh600938", "中国海油"),
    "招商银行": ("600036.SH", "sh600036", "招商银行"),
    "工商银行": ("601398.SH", "sh601398", "工商银行"),
    "上海银行": ("601229.SH", "sh601229", "上海银行"),
    "药明康德": ("603259.SH", "sh603259", "药明康德"),
    "迈瑞医疗": ("300760.SZ", "sz300760", "迈瑞医疗"),
    "鱼跃医疗": ("002223.SZ", "sz002223", "鱼跃医疗"),
    "美好医疗": ("301363.SZ", "sz301363", "美好医疗"),
    "爱尔眼科": ("300015.SZ", "sz300015", "爱尔眼科"),
    "恒瑞医药": ("600276.SH", "sh600276", "恒瑞医药"),
    "百济神州": ("06160.HK", "hk06160", "百济神州"),
    "泡泡玛特": ("09992.HK", "hk09992", "泡泡玛特"),
    "名创优品": ("09896.HK", "hk09896", "名创优品"),
    "腾讯控股": ("00700.HK", "hk00700", "腾讯控股"),
    "美团": ("03690.HK", "hk03690", "美团"),
    "阳光电源": ("300274.SZ", "sz300274", "阳光电源"),
    "德业股份": ("605117.SH", "sh605117", "德业股份"),
    "中航沈飞": ("600760.SH", "sh600760", "中航沈飞"),
    "中国船舶": ("600150.SH", "sh600150", "中国船舶"),
    "中远海能": ("600026.SH", "sh600026", "中远海能"),
    "南方航空": ("600029.SH", "sh600029", "南方航空"),
    "中国国航": ("601111.SH", "sh601111", "中国国航"),
}


def resolve_all_qt_codes(raw_symbol_or_head: str, market_type: str = "US") -> list[tuple[str, str, str]]:
    """
    Resolve ALL tradable stock tickers present in a single or multi-ticker head string
    (e.g. 'APP / PLTR (AppLovin / Palantir)' or '寒武纪-U (688256.SH) / 海光信息 (688041.SH)')
    into a deduplicated list of (canonical_symbol, qt_code, chinese_short_name).
    """
    text = (raw_symbol_or_head or "").strip()
    if not text:
        return []

    found: list[tuple[str, str, str]] = []
    seen_syms: set[str] = set()

    def _add(sym: str, qt: str, zh: str):
        if sym not in seen_syms:
            seen_syms.add(sym)
            found.append((sym, qt, zh))

    # Split multi-ticker chunks by '/' when outside or across tickers so local Chinese names pair properly
    segments = [seg.strip() for seg in re.split(r"\s*/\s*|\s*\+\s*|、", text) if seg.strip()]
    if not segments:
        segments = [text]

    # Pass 1: Explicit A-share and HK numeric codes across full text & segments
    for m_a in re.finditer(r"([\u4e00-\u9fff]{2,8})?[^\w\u4e00-\u9fff]*\b([0-9]{6})\.(SH|SZ|BJ)\b", text, flags=re.I):
        zh_pre, digits, exch = (m_a.group(1) or ""), m_a.group(2), m_a.group(3).upper()
        prefix = "sh" if exch == "SH" else ("sz" if exch == "SZ" else "bj")
        sym = f"{digits}.{exch}"
        zh = zh_pre or next((nm for nm, v in CN_NAME_QT_MAP.items() if v[0] == sym), "") or sym
        _add(sym, f"{prefix}{digits}", zh)

    for m_hk in re.finditer(r"([\u4e00-\u9fff]{2,8})?[^\w\u4e00-\u9fff]*\b([0-9]{4,5})\.HK\b", text, flags=re.I):
        zh_pre, digits = (m_hk.group(1) or ""), m_hk.group(2).zfill(5)
        sym = f"{digits}.HK"
        # Avoid double-counting dual-listed A+H in the exact same parenthetical like (601899.SH / 2899.HK)
        if zh_pre == "" and found and "/" in text and "(" in text:
            pass
        zh = zh_pre or next((nm for nm, v in CN_NAME_QT_MAP.items() if v[0] == sym), "") or sym
        _add(sym, f"hk{digits}", zh)

    # Pass 2: US tickers (including explicit PDD.US or standalone uppercase tickers)
    us_stopwords = {
        "SH", "SZ", "HK", "BJ", "ADR", "ETF", "US", "CN", "CSI", "IT", "IRA", "FTC",
        "PBM", "OEM", "VIP", "AUM", "EPS", "PE", "ROE", "ROIC", "ARR", "BPO", "DIY",
        "CEO", "CFO", "IPO", "MNC", "CNS", "BBB", "GLP", "FDA", "EPA", "LNG", "OPEC",
        "ASIC", "GPU", "CPU", "NPU", "TPU", "LLM", "CDMO", "CRO", "CXO", "REIT",
        "PUGC", "CTV", "MLR", "ESI", "ASP", "NIM", "NPL", "LPR", "FDI", "PMI", "CPI",
        "PPI", "GDP", "WTO", "IMF", "WHO", "OTC",
    }
    for m_us in re.finditer(r"\b([A-Z]{2,5})(?:\.US)?\b", text):
        cand = m_us.group(1)
        if cand in us_stopwords:
            continue
        if cand == "AI":
            # Only treat 'AI' as C3.ai when explicitly formatted as ticker, not 'AI算力'
            after = text[m_us.end():m_us.end() + 2]
            if re.match(r"[\u4e00-\u9fffA-Za-z]", after):
                continue
        if cand in KNOWN_QT_MAP:
            qt_code, default_zh = KNOWN_QT_MAP[cand]
            _add(cand, qt_code, default_zh)
        elif market_type == "US" or ".US" in m_us.group(0):
            resolved = query_smartbox(cand, prefer_market="us")
            if resolved:
                _add(cand, resolved[0], resolved[1] or cand)

    # Single-letter US tickers only when clearly isolated (e.g. 'F (福特)' or 'O / VICI')
    if market_type == "US" and not found:
        for m_1 in re.finditer(r"(?:^|[\s/(\[])([FGMOUWS])(?:\.US)?(?:$|[\s/)\]])", text):
            cand1 = m_1.group(1)
            if cand1 in KNOWN_QT_MAP:
                qt_code, default_zh = KNOWN_QT_MAP[cand1]
                _add(cand1, qt_code, default_zh)

    # Pass 3: Well-known Chinese stock names when written without numeric code (e.g. in Pair Trade legs)
    if not found or market_type == "CN":
        for cn_nm, (sym, qt, zh) in CN_NAME_QT_MAP.items():
            if cn_nm in text and sym not in seen_syms:
                # Skip HK duplicate if A-share of same company already added
                if any(existing_zh == zh for _, _, existing_zh in found):
                    continue
                _add(sym, qt, zh)

    return found


def resolve_qt_code(raw_symbol_or_head: str, market_type: str = "US") -> Optional[tuple[str, str, str]]:
    """
    Resolve a raw ticker head string (e.g. '中际旭创 (300308.SZ)' or 'NVDA (英伟达)')
    into the primary (canonical_symbol, qt_code, chinese_short_name).
    """
    all_res = resolve_all_qt_codes(raw_symbol_or_head, market_type=market_type)
    return all_res[0] if all_res else None


SMARTBOX_CACHE_FILE = MARKET_CACHE_DIR / "smartbox_cache.json"
try:
    _SMARTBOX_MEM_CACHE: dict = json.loads(SMARTBOX_CACHE_FILE.read_text(encoding="utf-8")) if SMARTBOX_CACHE_FILE.exists() else {}
except Exception:
    _SMARTBOX_MEM_CACHE = {}


def _save_smartbox_cache():
    try:
        SMARTBOX_CACHE_FILE.write_text(json.dumps(_SMARTBOX_MEM_CACHE, ensure_ascii=False), encoding="utf-8")
    except Exception:
        pass


def query_smartbox(query_str: str, prefer_market: str = "us") -> Optional[tuple[str, str]]:
    """Query Tencent smartbox API to resolve ticker -> (qt_code, name)."""
    cache_k = f"{query_str}_{prefer_market}"
    if cache_k in _SMARTBOX_MEM_CACHE:
        val = _SMARTBOX_MEM_CACHE[cache_k]
        return tuple(val) if val else None
    q = urllib.parse.quote(query_str)
    url = f"https://smartbox.gtimg.cn/s3/?v=2&q={q}&t=all"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        raw = urllib.request.urlopen(req, timeout=5).read().decode("utf-8", errors="ignore")
        m = re.search(r'v_hint="([^"]+)"', raw)
        if not m:
            _SMARTBOX_MEM_CACHE[cache_k] = None
            _save_smartbox_cache()
            return None
        entries = m.group(1).split("^")
        for ent in entries:
            fields = ent.split("~")
            if len(fields) < 3:
                continue
            mkt, code, name_enc = fields[0].lower(), fields[1], fields[2]
            if prefer_market and mkt != prefer_market:
                continue
            try:
                name_dec = name_enc.encode("utf-8").decode("unicode_escape")
            except Exception:
                name_dec = name_enc
            if mkt == "us":
                parts = code.split(".")
                sym_up = parts[0].upper()
                suf_up = parts[1].upper() if len(parts) > 1 else "OQ"
                res_val = (f"us{sym_up}.{suf_up}", name_dec)
                _SMARTBOX_MEM_CACHE[cache_k] = list(res_val)
                _save_smartbox_cache()
                return res_val
            elif mkt in ("sh", "sz", "hk"):
                res_val = (f"{mkt}{code}", name_dec)
                _SMARTBOX_MEM_CACHE[cache_k] = list(res_val)
                _save_smartbox_cache()
                return res_val
    except Exception:
        pass
    _SMARTBOX_MEM_CACHE[cache_k] = None
    _save_smartbox_cache()
    return None


def _calc_ma(closes: list[float], window: int) -> list[Optional[float]]:
    out = []
    for i in range(len(closes)):
        if i + 1 < window:
            out.append(None)
        else:
            s = sum(closes[i + 1 - window : i + 1]) / window
            out.append(round(s, 2))
    return out


def fetch_kline_series(
    qt_code: str,
    symbol: str,
    name: str,
    market: str,
    days: int = 60,
    force_refresh: bool = False,
) -> Optional[dict]:
    """
    Fetch daily K-line OHLCV from Tencent API with same-day TTL disk caching.
    Returns dict with dates, ohlc ([[open, close, low, high], ...]), volumes, closes, ma5, ma20,
    latest_close, chg_1d_pct, chg_5d_pct, chg_20d_pct.
    """
    safe_key = re.sub(r"[^a-zA-Z0-9_.-]", "_", qt_code)
    cache_file = MARKET_CACHE_DIR / f"kline_{safe_key}.json"
    today_str = datetime.now().strftime("%Y-%m-%d")

    if not force_refresh and cache_file.exists():
        try:
            cached = json.loads(cache_file.read_text(encoding="utf-8"))
            if cached.get("fetched_day") == today_str:
                if cached.get("empty"):
                    return None
                c_dates = cached.get("dates") or []
                if len(c_dates) >= 10 and cached.get("requested_days", 40) >= days:
                    # Reject halted/delisted/exchange-migrated series with date discontinuities or penny OTC stubs
                    if c_dates[-1] < "2026-10-28" or c_dates[-5] < "2026-10-20":
                        return None
                    if market == "US" and (cached.get("latest_close") or 0) < 0.25:
                        return None
                    cached["name"] = name
                    return cached
        except Exception:
            pass

    url = f"https://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param={qt_code},day,,,{days},qfq"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        raw = urllib.request.urlopen(req, timeout=7).read().decode("utf-8", errors="ignore")
        payload = json.loads(raw)
        k_obj = (payload.get("data") or {}).get(qt_code) or {}
        rows = k_obj.get("qfqday") or k_obj.get("day") or []
        if not rows or len(rows) < 5:
            cache_file.write_text(
                json.dumps({"empty": True, "fetched_day": today_str}), encoding="utf-8"
            )
            return None

        dates = []
        ohlc = []  # ECharts candlestick order: [open, close, lowest, highest]
        closes = []
        volumes = []

        for r in rows:
            if len(r) < 5:
                continue
            d_str = str(r[0])
            o_val = round(float(r[1]), 2)
            c_val = round(float(r[2]), 2)
            h_val = round(float(r[3]), 2)
            l_val = round(float(r[4]), 2)
            v_val = round(float(r[5]), 0) if len(r) > 5 else 0
            dates.append(d_str)
            ohlc.append([o_val, c_val, l_val, h_val])
            closes.append(c_val)
            volumes.append(v_val)

        # Reject delisted / long-halted / pre-transfer tickers with date gaps or OTC penny stubs
        if (
            len(closes) < 10
            or dates[-1] < "2026-10-28"
            or dates[-5] < "2026-10-20"
            or (market == "US" and closes[-1] < 0.25)
        ):
            cache_file.write_text(
                json.dumps({"empty": True, "fetched_day": today_str, "reason": "stale_or_delisted"}),
                encoding="utf-8",
            )
            return None

        ma5 = _calc_ma(closes, 5)
        ma20 = _calc_ma(closes, 20)

        latest = closes[-1]
        prev_1d = closes[-2] if len(closes) >= 2 else latest
        prev_5d = closes[-6] if len(closes) >= 6 else closes[0]
        prev_20d = closes[-21] if len(closes) >= 21 else closes[0]

        chg_1d = round((latest - prev_1d) / prev_1d * 100, 2) if prev_1d else 0.0
        chg_5d = round((latest - prev_5d) / prev_5d * 100, 2) if prev_5d else 0.0
        chg_20d = round((latest - prev_20d) / prev_20d * 100, 2) if prev_20d else 0.0

        # Normalized 100-base series for multi-line comparison
        base_c = closes[0] if closes[0] else 1.0
        norm_series = [round((c / base_c - 1.0) * 100, 2) for c in closes]

        result = {
            "symbol": symbol,
            "qt_code": qt_code,
            "name": name,
            "market": market,
            "fetched_day": today_str,
            "requested_days": days,
            "latest_date": dates[-1],
            "latest_close": latest,
            "chg_1d_pct": chg_1d,
            "chg_5d_pct": chg_5d,
            "chg_20d_pct": chg_20d,
            "dates": dates,
            "ohlc": ohlc,
            "closes": closes,
            "volumes": volumes,
            "ma5": ma5,
            "ma20": ma20,
            "norm_pct": norm_series,
        }
        cache_file.write_text(json.dumps(result, ensure_ascii=False), encoding="utf-8")
        return result
    except Exception:
        try:
            cache_file.write_text(
                json.dumps({"empty": True, "fetched_day": today_str}), encoding="utf-8"
            )
        except Exception:
            pass
        return None


def fetch_market_klines_for_report(stock_rows: list[dict], log_fn: Callable[[str], None] = print) -> dict[str, dict]:
    """
    Fetch real 60-day K-line series concurrently for benchmark indices + all resolvable stocks in the report.
    Returns dict keyed by symbol (e.g. 'NVDA', '300308.SZ', 'SPY', etc.).
    """
    kline_map: dict[str, dict] = {}
    tasks_to_fetch: dict[str, tuple[str, str, str, str]] = {}  # sym -> (qt_code, sym, label, market)
    seen_qt: set[str] = set()

    # 1. Queue benchmarks first
    for bm in BENCHMARK_SYMBOLS:
        sym = bm["symbol"]
        qt = bm["qt_code"]
        seen_qt.add(qt)
        tasks_to_fetch[sym] = (qt, sym, bm["name"], bm["market"])

    # 2. Queue all report stocks (primary + secondary tickers up to 220 unique symbols)
    row_resolutions: list[tuple[dict, str, str]] = []
    for st in stock_rows:
        resolved = resolve_qt_code(st["head_display"], st["market_type"])
        if not resolved:
            continue
        sym, qt_code, zh_name = resolved
        row_resolutions.append((st, sym, zh_name))
        if sym not in tasks_to_fetch and qt_code not in seen_qt and len(tasks_to_fetch) < 220:
            seen_qt.add(qt_code)
            tasks_to_fetch[sym] = (qt_code, sym, f"{zh_name} ({sym})", st["market_type"])

    with ThreadPoolExecutor(max_workers=12) as pool:
        fut_map = {
            pool.submit(fetch_kline_series, qt, sym, label, mkt): sym
            for sym, (qt, sym, label, mkt) in tasks_to_fetch.items()
        }
        for fut in as_completed(fut_map):
            sym = fut_map[fut]
            try:
                series = fut.result()
                if series:
                    kline_map[sym] = series
            except Exception:
                pass

    for st, sym, zh_name in row_resolutions:
        st["resolved_symbol"] = sym
        st["zh_short_name"] = zh_name
        if sym in kline_map:
            st["kline"] = kline_map[sym]

    log_fn(f"[MarketData] ✓ Loaded real 60-day K-line (OHLCV) series for {len(kline_map)} indices & stocks")
    return kline_map
