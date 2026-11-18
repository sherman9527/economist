# -*- coding: utf-8 -*-
"""
Full Out-of-Sample (OOS) Retrospective Accuracy Audit for Economist Equity Research Reports.
Audits all generated daily reports (2026-11-07 through 2026-11-16) against live post-publication
OHLCV market data fetched via Tencent Finance (stored strictly under ./data).

Methodology (Zero Look-Ahead / Zero Pre-Report Momentum Contamination):
  - Report Cutoff Date D = run_key (note: 2026-11-07 & 2026-11-08 were backfilled on Monday 2026-11-09;
    both <2026-11-07 and <2026-11-09 anchor to Friday 2026-11-06 close for US; for CN we check both
    article-date cutoff <D and strict generation-time cutoff).
  - Pre-Report Baseline Close P0 = last trading close strictly BEFORE report cutoff date (date < cutoff_date).
  - Post-Report Forward Window = all trading sessions with date >= cutoff_date up to latest available bar
    (2026-11-13 close for US equities; 2026-11-16 session for A-share / HK equities).
  - Reports/markets with 0 post-report trading sessions (e.g., US stocks in 2026-11-14, 11-15, 11-16 reports
    prior to Monday 2026-11-16 US market open) are strictly classified as PENDING (excluded from OOS hit-rate
    denominators).
  - Evaluates:
      1) All Individual Stock Calls (expanding multi-ticker rows like 'APP / PLTR' so zero tickers are hidden)
      2) Unique First-Call Cohort (deduplicating repeated mentions across rolling 2-day windows by (symbol, polarity))
      3) Explicit Long/Short Pair Trades ('做多 X / 做空 Y' or 'Long X / Short Y')
"""

import glob
import json
import re
import sys
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from config import DATA_DIR
from gs_ms_tables import extract_all_structured_stocks, parse_ticker_entry
from market_data import BENCHMARK_SYMBOLS, fetch_kline_series, resolve_all_qt_codes

AUDIT_OUTPUT_FILE = DATA_DIR / "full_retrospective_audit_2026-11-16.json"


def _parse_pair_trade_legs(raw_str: str, market_type: str) -> tuple[list[tuple[str, str, str]], list[tuple[str, str, str]]]:
    """
    Extract (long_tickers, short_tickers) from explicit Pair Trade strings such as:
      - 'Pair Trade 配对交易：做多 WELL (医疗养老地产龙头) / 做空 CCL (嘉年华邮轮...)'
      - 'Pair Trade (配对交易) - [Long AVGO / Short NVDA] 做多...做空...'
      - 'Pair Trade 配对交易：做多 AVGO + AMD (算力替代组合) / 做空 NVDA (英伟达) - ...'
    """
    s = raw_str or ""
    long_part = ""
    short_part = ""

    # Pattern 1: [Long X / Short Y]
    m_en = re.search(r"\[\s*Long\s+([^/\]]+?)\s*/\s*Short\s+([^\]]+?)\s*\]", s, flags=re.I)
    if m_en:
        long_part, short_part = m_en.group(1), m_en.group(2)
    else:
        # Pattern 2: 做多 X / 做空 Y (- thesis)
        body = s.split(" - ", 1)[0] if " - " in s else s
        m_zh = re.search(r"(?:做多|Long)\s*([^/；;]+?)\s*(?:/|vs\.?|VS)\s*(?:做空|Short)\s*(.+)$", body, flags=re.I)
        if m_zh:
            long_part, short_part = m_zh.group(1), m_zh.group(2)

    if not long_part or not short_part:
        return [], []

    # Strip trailing narrative thesis clauses after comma in short_part (e.g. '，博弈...' / '，捕捉...')
    short_part = re.split(r"[，。；]|-\s", short_part)[0].strip()

    long_legs = resolve_all_qt_codes(long_part, market_type=market_type)
    short_legs = resolve_all_qt_codes(short_part, market_type=market_type)
    return long_legs, short_legs


def compute_oos_return(kline: dict, cutoff_date: str) -> dict | None:
    """
    Compute strictly out-of-sample forward return from last close < cutoff_date to latest close >= cutoff_date.
    Returns None if kline is missing or has 0 post-cutoff trading sessions.
    """
    if not kline:
        return None
    dates = kline.get("dates") or []
    closes = kline.get("closes") or []
    ohlc = kline.get("ohlc") or []
    if len(dates) < 5 or len(dates) != len(closes):
        return None

    # Find last index strictly before cutoff_date
    anchor_idx = None
    for i, d in enumerate(dates):
        if d < cutoff_date:
            anchor_idx = i
        else:
            break

    if anchor_idx is None:
        return None

    fwd_bars = (len(dates) - 1) - anchor_idx
    if fwd_bars <= 0 or dates[-1] < cutoff_date:
        return {
            "pending": True,
            "anchor_date": dates[anchor_idx],
            "anchor_close": closes[anchor_idx],
            "latest_date": dates[-1],
            "latest_close": closes[-1],
            "fwd_bars": 0,
        }

    p0 = closes[anchor_idx]
    if p0 <= 0:
        return None

    p_latest = closes[-1]
    p_t1 = closes[anchor_idx + 1]
    p_t3 = closes[anchor_idx + 3] if fwd_bars >= 3 else None
    p_t5 = closes[anchor_idx + 5] if fwd_bars >= 5 else None

    post_highs = [ohlc[j][3] for j in range(anchor_idx + 1, len(dates))] if len(ohlc) == len(dates) else closes[anchor_idx + 1:]
    post_lows = [ohlc[j][2] for j in range(anchor_idx + 1, len(dates))] if len(ohlc) == len(dates) else closes[anchor_idx + 1:]

    return {
        "pending": False,
        "anchor_date": dates[anchor_idx],
        "anchor_close": round(p0, 3),
        "first_post_date": dates[anchor_idx + 1],
        "latest_date": dates[-1],
        "latest_close": round(p_latest, 3),
        "fwd_bars": fwd_bars,
        "ret_cum_pct": round((p_latest / p0 - 1.0) * 100.0, 2),
        "ret_t1_pct": round((p_t1 / p0 - 1.0) * 100.0, 2),
        "ret_t3_pct": round((p_t3 / p0 - 1.0) * 100.0, 2) if p_t3 is not None else None,
        "ret_t5_pct": round((p_t5 / p0 - 1.0) * 100.0, 2) if p_t5 is not None else None,
        "max_gain_pct": round((max(post_highs) / p0 - 1.0) * 100.0, 2) if post_highs else 0.0,
        "max_dd_pct": round((min(post_lows) / p0 - 1.0) * 100.0, 2) if post_lows else 0.0,
    }


def run_full_audit():
    run_files = sorted(glob.glob(str(DATA_DIR / "runs" / "analysis_*.json")))
    print(f"[Audit] Found {len(run_files)} daily reports to audit.")

    reports_data = []
    all_needed_tickers: dict[str, tuple[str, str, str]] = {}  # sym -> (qt_code, zh_name, market)

    for bm in BENCHMARK_SYMBOLS:
        all_needed_tickers[bm["symbol"]] = (bm["qt_code"], bm["name"], bm["market"])

    for fp in run_files:
        d = json.loads(Path(fp).read_text(encoding="utf-8"))
        rk = d.get("run_key")
        gen_at = d.get("generated_at", "")
        core = d.get("core_articles") or []
        stock_rows = extract_all_structured_stocks(core, include_pairs=False)

        # Also extract explicit pair trades
        pair_rows = []
        for idx, art in enumerate(core, 1):
            zh_title = art.get("zh_title") or art.get("title") or f"#{idx}"
            cat = art.get("theme_category") or "宏观"
            for m_key, m_type in [("us_market", "US"), ("cn_market", "CN")]:
                m_obj = art.get(m_key) or {}
                for t_str in m_obj.get("tickers") or []:
                    if re.search(r"Pair\s*Trade|配对交易", t_str, flags=re.I):
                        l_legs, s_legs = _parse_pair_trade_legs(t_str, m_type)
                        for sym, qt, zh in l_legs + s_legs:
                            mkt_actual = "US" if qt.startswith("us") else "CN"
                            all_needed_tickers[sym] = (qt, zh, mkt_actual)
                        pair_rows.append({
                            "report_date": rk,
                            "market_type": m_type,
                            "article_idx": idx,
                            "article_title": zh_title,
                            "category": cat,
                            "raw_str": t_str,
                            "long_legs": l_legs,
                            "short_legs": s_legs,
                        })

        for r in stock_rows:
            resolved_list = resolve_all_qt_codes(r["head_display"], market_type=r["market_type"])
            r["resolved_list"] = resolved_list
            for sym, qt, zh in resolved_list:
                mkt_actual = "US" if qt.startswith("us") else "CN"
                all_needed_tickers[sym] = (qt, zh, mkt_actual)

        reports_data.append({
            "report_date": rk,
            "generated_at": gen_at,
            "macro": d.get("macro_synthesis") or {},
            "core_count": len(core),
            "stock_rows": stock_rows,
            "pair_rows": pair_rows,
        })

    print(f"[Audit] Refreshing live 60-day OHLCV K-lines for {len(all_needed_tickers)} unique symbols...")
    kline_db: dict[str, dict] = {}
    with ThreadPoolExecutor(max_workers=16) as pool:
        fut_map = {
            pool.submit(fetch_kline_series, qt, sym, zh, mkt, 60): sym
            for sym, (qt, zh, mkt) in all_needed_tickers.items()
        }
        for fut in as_completed(fut_map):
            sym = fut_map[fut]
            try:
                res = fut.result()
                if res:
                    kline_db[sym] = res
            except Exception:
                pass

    print(f"[Audit] Live OHLCV loaded: {len(kline_db)} / {len(all_needed_tickers)} active symbols.")
    spy_k = kline_db.get("SPY")
    qqq_k = kline_db.get("QQQ")
    sh_k = kline_db.get("000001.SH")
    cyb_k = kline_db.get("399006.SZ")
    print(f"  SPY latest: {spy_k['latest_date']} close={spy_k['latest_close']}" if spy_k else "  SPY missing!")
    print(f"  000001.SH latest: {sh_k['latest_date']} close={sh_k['latest_close']}" if sh_k else "  SH missing!")

    all_oos_calls = []
    pending_calls = []
    neutral_calls = []
    evaluated_pairs = []

    for rep in reports_data:
        rk = rep["report_date"]
        gen_at = rep["generated_at"]
        # Note: 2026-11-07, 11-08, 11-09 were generated on 2026-11-09 midday.
        # Using article report_date rk tests the Economist publication-day signal;
        # we also record strict post-generation return (cutoff = gen_day) for 11-07/08/09 CN stocks!
        gen_day = gen_at[:10] if gen_at else rk

        for r in rep["stock_rows"]:
            pol = r["polarity"]
            resolved_list = r["resolved_list"]
            # Deduplicate dual-listed A+H in same row (e.g. 601899.SH / 02899.HK) by keeping primary listing per company name
            dedup_row_syms = []
            seen_zh_in_row = set()
            for sym, qt, zh in resolved_list:
                base_zh = re.sub(r"[-A-Z0-9()（）\s]", "", zh)[:4]
                if base_zh and base_zh in seen_zh_in_row and len(resolved_list) > 1:
                    continue
                if base_zh:
                    seen_zh_in_row.add(base_zh)
                dedup_row_syms.append((sym, qt, zh))

            for sym, qt, zh in dedup_row_syms:
                kl = kline_db.get(sym)
                if not kl:
                    continue
                actual_mkt = "US" if qt.startswith("us") else "CN"
                bm_kl = spy_k if actual_mkt == "US" else sh_k

                oos = compute_oos_return(kl, rk)
                if not oos:
                    continue

                if oos.get("pending"):
                    if pol in ("bull", "bear"):
                        pending_calls.append({
                            "report_date": rk,
                            "symbol": sym,
                            "name": zh,
                            "market": actual_mkt,
                            "polarity": pol,
                            "rating": r["rating"],
                            "anchor_date": oos["anchor_date"],
                            "anchor_close": oos["anchor_close"],
                            "category": r["theme_category"],
                            "thesis": r["thesis"][:90],
                        })
                    continue

                bm_oos = compute_oos_return(bm_kl, rk) if bm_kl else None
                bm_ret = bm_oos["ret_cum_pct"] if (bm_oos and not bm_oos.get("pending")) else 0.0
                bm_t1 = bm_oos["ret_t1_pct"] if (bm_oos and not bm_oos.get("pending")) else 0.0

                ret_cum = oos["ret_cum_pct"]
                alpha_cum = round(ret_cum - bm_ret, 2)
                ret_t1 = oos["ret_t1_pct"]
                alpha_t1 = round(ret_t1 - bm_t1, 2)

                # Strict Post-Generation Check (for 11-07/08 backfilled on 11-09)
                strict_gen_oos = compute_oos_return(kl, gen_day) if gen_day > rk else oos
                strict_ret = strict_gen_oos["ret_cum_pct"] if (strict_gen_oos and not strict_gen_oos.get("pending")) else ret_cum

                rec = {
                    "report_date": rk,
                    "generated_day": gen_day,
                    "symbol": sym,
                    "qt_code": qt,
                    "name": zh,
                    "head_display": r["head_display"],
                    "market": actual_mkt,
                    "category": r["theme_category"],
                    "rating": r["rating"],
                    "polarity": pol,
                    "score": r["score"],
                    "driver": r["driver"],
                    "anchor_date": oos["anchor_date"],
                    "anchor_close": oos["anchor_close"],
                    "latest_date": oos["latest_date"],
                    "latest_close": oos["latest_close"],
                    "fwd_bars": oos["fwd_bars"],
                    "ret_cum_pct": ret_cum,
                    "bm_ret_pct": bm_ret,
                    "alpha_cum_pct": alpha_cum,
                    "ret_t1_pct": ret_t1,
                    "alpha_t1_pct": alpha_t1,
                    "ret_t3_pct": oos["ret_t3_pct"],
                    "ret_t5_pct": oos["ret_t5_pct"],
                    "max_gain_pct": oos["max_gain_pct"],
                    "max_dd_pct": oos["max_dd_pct"],
                    "strict_gen_ret_pct": strict_ret,
                    "article_title": r["article_title"],
                    "thesis": r["thesis"][:110],
                }

                if pol == "neutral":
                    neutral_calls.append(rec)
                elif pol in ("bull", "bear"):
                    # 1. Pure Absolute Direction Hit (Bull: > 0%, Bear: < 0%)
                    rec["abs_hit"] = (ret_cum > 0.0) if pol == "bull" else (ret_cum < 0.0)
                    # 2. Pure Excess Alpha Hit vs SPY / 000001.SH (Bull: alpha > 0%, Bear: alpha < 0%)
                    rec["alpha_hit"] = (alpha_cum > 0.0) if pol == "bull" else (alpha_cum < 0.0)
                    # 3. Signed Directional Return (positive = money made in direction of call)
                    rec["signed_ret_pct"] = ret_cum if pol == "bull" else round(-ret_cum, 2)
                    rec["signed_alpha_pct"] = alpha_cum if pol == "bull" else round(-alpha_cum, 2)
                    rec["t1_abs_hit"] = (ret_t1 > 0.0) if pol == "bull" else (ret_t1 < 0.0)
                    rec["t1_alpha_hit"] = (alpha_t1 > 0.0) if pol == "bull" else (alpha_t1 < 0.0)
                    all_oos_calls.append(rec)

        # Evaluate explicit Pair Trades for this report
        for pr in rep["pair_rows"]:
            l_rets = []
            s_rets = []
            l_names = []
            s_names = []
            pending_pair = False
            for sym, qt, zh in pr["long_legs"]:
                kl = kline_db.get(sym)
                oos = compute_oos_return(kl, rk) if kl else None
                if oos and oos.get("pending"):
                    pending_pair = True
                elif oos and not oos.get("pending"):
                    l_rets.append(oos["ret_cum_pct"])
                    l_names.append(f"{zh}({sym} {oos['ret_cum_pct']:+.2f}%)")
            for sym, qt, zh in pr["short_legs"]:
                kl = kline_db.get(sym)
                oos = compute_oos_return(kl, rk) if kl else None
                if oos and oos.get("pending"):
                    pending_pair = True
                elif oos and not oos.get("pending"):
                    s_rets.append(oos["ret_cum_pct"])
                    s_names.append(f"{zh}({sym} {oos['ret_cum_pct']:+.2f}%)")
            if l_rets and s_rets and not pending_pair:
                avg_l = round(sum(l_rets) / len(l_rets), 2)
                avg_s = round(sum(s_rets) / len(s_rets), 2)
                spread = round(avg_l - avg_s, 2)
                evaluated_pairs.append({
                    "report_date": rk,
                    "market_type": pr["market_type"],
                    "category": pr["category"],
                    "long_desc": " + ".join(l_names),
                    "short_desc": " + ".join(s_names),
                    "long_ret_pct": avg_l,
                    "short_ret_pct": avg_s,
                    "spread_pct": spread,
                    "pair_win": spread > 0.0,
                    "raw_str": pr["raw_str"][:120],
                })

    def _summarize_bucket(items: list[dict]) -> dict:
        n = len(items)
        if n == 0:
            return {"count": 0}
        abs_hits = sum(1 for x in items if x["abs_hit"])
        alpha_hits = sum(1 for x in items if x["alpha_hit"])
        either_hits = sum(1 for x in items if (x["abs_hit"] or x["alpha_hit"]))
        both_hits = sum(1 for x in items if (x["abs_hit"] and x["alpha_hit"]))
        t1_abs = sum(1 for x in items if x["t1_abs_hit"])
        t1_alpha = sum(1 for x in items if x["t1_alpha_hit"])

        bulls = [x for x in items if x["polarity"] == "bull"]
        bears = [x for x in items if x["polarity"] == "bear"]
        avg_bull_ret = round(sum(x["ret_cum_pct"] for x in bulls) / len(bulls), 2) if bulls else 0.0
        avg_bear_ret = round(sum(x["ret_cum_pct"] for x in bears) / len(bears), 2) if bears else 0.0
        avg_bull_alpha = round(sum(x["alpha_cum_pct"] for x in bulls) / len(bulls), 2) if bulls else 0.0
        avg_bear_alpha = round(sum(x["alpha_cum_pct"] for x in bears) / len(bears), 2) if bears else 0.0

        return {
            "count": n,
            "bull_count": len(bulls),
            "bear_count": len(bears),
            "abs_hit_rate": round(abs_hits / n * 100.0, 1),
            "alpha_hit_rate": round(alpha_hits / n * 100.0, 1),
            "institutional_hit_rate": round(either_hits / n * 100.0, 1),
            "double_hit_rate": round(both_hits / n * 100.0, 1),
            "t1_abs_hit_rate": round(t1_abs / n * 100.0, 1),
            "t1_alpha_hit_rate": round(t1_alpha / n * 100.0, 1),
            "avg_signed_ret_pct": round(sum(x["signed_ret_pct"] for x in items) / n, 2),
            "avg_signed_alpha_pct": round(sum(x["signed_alpha_pct"] for x in items) / n, 2),
            "avg_bull_ret_pct": avg_bull_ret,
            "avg_bear_ret_pct": avg_bear_ret,
            "long_short_spread_pct": round(avg_bull_ret - avg_bear_ret, 2) if (bulls and bears) else None,
            "avg_bull_alpha_pct": avg_bull_alpha,
            "avg_bear_alpha_pct": avg_bear_alpha,
        }

    # Deduplicate within same report first: (report_date, symbol, polarity)
    per_report_dedup = []
    seen_rep_sym = set()
    for c in all_oos_calls:
        k = (c["report_date"], c["symbol"], c["polarity"])
        if k not in seen_rep_sym:
            seen_rep_sym.add(k)
            per_report_dedup.append(c)

    # Unique First-Call Cohort across ALL reports: earliest report_date for each (symbol, polarity)
    first_call_cohort = []
    seen_first = set()
    for c in sorted(per_report_dedup, key=lambda x: x["report_date"]):
        k = (c["symbol"], c["polarity"])
        if k not in seen_first:
            seen_first.add(k)
            first_call_cohort.append(c)

    by_report = {}
    for rk in sorted({r["report_date"] for r in reports_data}):
        sub = [c for c in per_report_dedup if c["report_date"] == rk]
        pend_sub = [p for p in pending_calls if p["report_date"] == rk]
        by_report[rk] = {
            **_summarize_bucket(sub),
            "us_oos": _summarize_bucket([c for c in sub if c["market"] == "US"]),
            "cn_oos": _summarize_bucket([c for c in sub if c["market"] == "CN"]),
            "pending_count": len(pend_sub),
        }

    by_category = {}
    for cat in sorted({c["category"] for c in per_report_dedup}):
        by_category[cat] = _summarize_bucket([c for c in per_report_dedup if c["category"] == cat])

    by_rating = {}
    for rt in ["强力买入", "超配增持", "低配减持", "强力规避"]:
        by_rating[rt] = _summarize_bucket([c for c in per_report_dedup if c["rating"] == rt])

    # Deduplicate pair trades by (report_date, long_desc_syms, short_desc_syms)
    pair_wins = sum(1 for p in evaluated_pairs if p["pair_win"])
    avg_pair_spread = round(sum(p["spread_pct"] for p in evaluated_pairs) / max(1, len(evaluated_pairs)), 2)

    # Check conflicted / reversed calls (same symbol rated bull in one context and bear in another)
    sym_polarities = defaultdict(set)
    for c in per_report_dedup:
        sym_polarities[c["symbol"]].add((c["report_date"], c["polarity"], c["rating"]))
    conflicted_syms = {
        s: sorted(list(v)) for s, v in sym_polarities.items()
        if len({p for _, p, _ in v}) > 1
    }

    # Top Best Hits & Worst Misses (from First-Call Cohort for non-repetitive clarity)
    best_longs = sorted(
        [c for c in first_call_cohort if c["polarity"] == "bull"],
        key=lambda x: x["ret_cum_pct"],
        reverse=True,
    )[:12]
    best_shorts = sorted(
        [c for c in first_call_cohort if c["polarity"] == "bear"],
        key=lambda x: x["ret_cum_pct"],
    )[:12]
    worst_long_misses = sorted(
        [c for c in first_call_cohort if c["polarity"] == "bull"],
        key=lambda x: x["ret_cum_pct"],
    )[:12]
    worst_short_misses = sorted(
        [c for c in first_call_cohort if c["polarity"] == "bear"],
        key=lambda x: x["ret_cum_pct"],
        reverse=True,
    )[:12]

    audit_result = {
        "audited_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "benchmarks_snapshot": {
            sym: {
                "latest_date": kline_db[sym]["latest_date"],
                "latest_close": kline_db[sym]["latest_close"],
                "chg_1d_pct": kline_db[sym]["chg_1d_pct"],
                "chg_5d_pct": kline_db[sym]["chg_5d_pct"],
            }
            for sym in ["SPY", "QQQ", "TLT", "000001.SH", "399006.SZ"]
            if sym in kline_db
        },
        "overall_per_report_dedup": _summarize_bucket(per_report_dedup),
        "overall_us": _summarize_bucket([c for c in per_report_dedup if c["market"] == "US"]),
        "overall_cn": _summarize_bucket([c for c in per_report_dedup if c["market"] == "CN"]),
        "overall_bull": _summarize_bucket([c for c in per_report_dedup if c["polarity"] == "bull"]),
        "overall_bear": _summarize_bucket([c for c in per_report_dedup if c["polarity"] == "bear"]),
        "unique_first_call_cohort": {
            "all": _summarize_bucket(first_call_cohort),
            "us": _summarize_bucket([c for c in first_call_cohort if c["market"] == "US"]),
            "cn": _summarize_bucket([c for c in first_call_cohort if c["market"] == "CN"]),
            "bull": _summarize_bucket([c for c in first_call_cohort if c["polarity"] == "bull"]),
            "bear": _summarize_bucket([c for c in first_call_cohort if c["polarity"] == "bear"]),
        },
        "pending_unopened_calls_count": len(pending_calls),
        "neutral_ew_calls_count": len(neutral_calls),
        "pair_trades_audit": {
            "evaluated_count": len(evaluated_pairs),
            "win_count": pair_wins,
            "win_rate_pct": round(pair_wins / max(1, len(evaluated_pairs)) * 100.0, 1),
            "avg_spread_pct": avg_pair_spread,
            "top_pairs": sorted(evaluated_pairs, key=lambda x: x["spread_pct"], reverse=True)[:10],
            "worst_pairs": sorted(evaluated_pairs, key=lambda x: x["spread_pct"])[:8],
        },
        "by_report": by_report,
        "by_rating": by_rating,
        "by_category": by_category,
        "conflicted_symbols_count": len(conflicted_syms),
        "conflicted_symbols_sample": dict(list(conflicted_syms.items())[:15]),
        "best_longs": best_longs,
        "best_shorts": best_shorts,
        "worst_long_misses": worst_long_misses,
        "worst_short_misses": worst_short_misses,
    }

    AUDIT_OUTPUT_FILE.write_text(json.dumps(audit_result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"[Audit] Saved full retrospective audit JSON -> {AUDIT_OUTPUT_FILE}")
    return audit_result


if __name__ == "__main__":
    res = run_full_audit()
    print("\n=== SUMMARY (Per-Report Deduplicated OOS Calls) ===")
    print(json.dumps(res["overall_per_report_dedup"], ensure_ascii=False, indent=2))
    print("\n=== SUMMARY (Unique First-Call Cohort) ===")
    print(json.dumps(res["unique_first_call_cohort"]["all"], ensure_ascii=False, indent=2))
    print("\n=== PAIR TRADES AUDIT ===")
    pt = res["pair_trades_audit"]
    print(f"Count={pt['evaluated_count']}, WinRate={pt['win_rate_pct']}%, AvgSpread={pt['avg_spread_pct']:+.2f}%")
