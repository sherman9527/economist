# -*- coding: utf-8 -*-
"""
Real-Time & Historical Market K-Line (OHLCV) Fetcher for US, A-Share, and HK Equities.
Fetches 40-day daily candlestick data (Open, Close, Low, High, Volume), computes MA5/MA20,
1D/5D/20D returns, and caches under D:\\code\\economist\\data\\market_cache (Zero C: drive usage).
"""

import json
import re
import urllib.parse
import urllib.request
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
    {"symbol": "WMT", "qt_code": "usWMT.N", "name": "沃尔玛 (WMT)", "market": "US"},
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
    "WMT": ("usWMT.N", "沃尔玛"),
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
    "KHC": ("usKHC.OQ", "卡夫亨氏食品"),
    "PLTR": ("usPLTR.N", "帕兰提尔大数据"),
    "SMCI": ("usSMCI.OQ", "超微电脑"),
    "ADBE": ("usADBE.OQ", "奥多比软件"),
    "TMO": ("usTMO.N", "赛默飞世尔"),
    "DHR": ("usDHR.N", "丹纳赫生命科学"),
    "LTH": ("usLTH.N", "高端健身连锁"),
    "AI": ("usAI.N", "C3人工智能"),
}


def resolve_qt_code(raw_symbol_or_head: str, market_type: str = "US") -> Optional[tuple[str, str, str]]:
    """
    Resolve a raw ticker head string (e.g. '中际旭创 (300308.SZ)' or 'NVDA (英伟达)')
    into (canonical_symbol, qt_code, chinese_short_name).
    """
    text = (raw_symbol_or_head or "").strip()
    if not text:
        return None

    # Extract Chinese name if present
    zh_match = re.search(r"[\u4e00-\u9fff]{2,8}", text)
    zh_name = zh_match.group(0) if zh_match else ""

    # 1. Check A-share 6-digit code
    m_ashare = re.search(r"\b([0-9]{6})\.(SH|SZ|BJ)\b", text, flags=re.I)
    if m_ashare:
        digits, exch = m_ashare.group(1), m_ashare.group(2).upper()
        prefix = "sh" if exch == "SH" else ("sz" if exch == "SZ" else "bj")
        sym = f"{digits}.{exch}"
        return sym, f"{prefix}{digits}", zh_name or sym

    # 2. Check HK 4-5 digit code
    m_hk = re.search(r"\b([0-9]{4,5})\.HK\b", text, flags=re.I)
    if m_hk:
        digits = m_hk.group(1).zfill(5)
        sym = f"{digits}.HK"
        return sym, f"hk{digits}", zh_name or sym

    # 3. Check US uppercase ticker
    m_us = re.findall(r"\b([A-Z]{2,5})\b", text)
    for cand in m_us:
        if cand in ("SH", "SZ", "HK", "BJ", "ADR", "ETF", "US", "CN", "AI", "IT", "IRA", "FTC"):
            continue
        if cand in KNOWN_QT_MAP:
            qt_code, default_zh = KNOWN_QT_MAP[cand]
            return cand, qt_code, default_zh
        # Query Tencent smartbox for unknown US ticker
        resolved = query_smartbox(cand, prefer_market="us" if market_type == "US" else "")
        if resolved:
            return cand, resolved[0], resolved[1] or zh_name or cand

    # Check if AI (C3.ai) specifically
    if re.search(r"\bAI\b", text) and market_type == "US":
        return "AI", "usAI.N", zh_name or "C3.ai"

    return None


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


def fetch_kline_series(qt_code: str, symbol: str, name: str, market: str, days: int = 40) -> Optional[dict]:
    """
    Fetch daily K-line OHLCV from Tencent API with disk caching.
    Returns dict with dates, ohlc ([[open, close, low, high], ...]), volumes, closes, ma5, ma20,
    latest_close, chg_1d_pct, chg_5d_pct, chg_20d_pct.
    """
    safe_key = re.sub(r"[^a-zA-Z0-9_.-]", "_", qt_code)
    cache_file = MARKET_CACHE_DIR / f"kline_{safe_key}.json"

    if cache_file.exists():
        try:
            cached = json.loads(cache_file.read_text(encoding="utf-8"))
            if cached.get("empty"):
                return None
            if cached.get("dates") and len(cached["dates"]) >= 10:
                cached["name"] = name
                return cached
        except Exception:
            pass

    url = f"https://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param={qt_code},day,,,{days},qfq"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        raw = urllib.request.urlopen(req, timeout=6).read().decode("utf-8", errors="ignore")
        payload = json.loads(raw)
        k_obj = (payload.get("data") or {}).get(qt_code) or {}
        rows = k_obj.get("qfqday") or k_obj.get("day") or []
        if not rows or len(rows) < 5:
            cache_file.write_text(json.dumps({"empty": True}), encoding="utf-8")
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

        if len(closes) < 5:
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
            cache_file.write_text(json.dumps({"empty": True}), encoding="utf-8")
        except Exception:
            pass
        return None


def fetch_market_klines_for_report(stock_rows: list[dict], log_fn: Callable[[str], None] = print) -> dict[str, dict]:
    """
    Fetch real 40-day K-line series for benchmark indices + all resolvable stocks in the report.
    Returns dict keyed by symbol (e.g. 'NVDA', '300308.SZ', 'SPY', etc.).
    """
    kline_map: dict[str, dict] = {}

    # 1. Benchmarks first
    for bm in BENCHMARK_SYMBOLS:
        series = fetch_kline_series(bm["qt_code"], bm["symbol"], bm["name"], bm["market"])
        if series:
            kline_map[bm["symbol"]] = series

    # 2. Report stocks (up to 36 top stocks so charts are instant and comprehensive)
    seen_qt = {v["qt_code"] for v in kline_map.values()}
    for st in stock_rows:
        resolved = resolve_qt_code(st["head_display"], st["market_type"])
        if not resolved:
            continue
        sym, qt_code, zh_name = resolved
        st["resolved_symbol"] = sym
        st["zh_short_name"] = zh_name
        if sym in kline_map:
            st["kline"] = kline_map[sym]
            continue
        if qt_code in seen_qt or len(kline_map) >= 60:
            continue
        seen_qt.add(qt_code)
        series = fetch_kline_series(qt_code, sym, f"{zh_name} ({sym})", st["market_type"])
        if series:
            kline_map[sym] = series
            st["kline"] = series

    log_fn(f"[MarketData] ✓ Loaded real 40-day K-line (OHLCV) series for {len(kline_map)} indices & stocks")
    return kline_map
