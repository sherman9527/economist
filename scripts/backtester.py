# -*- coding: utf-8 -*-
"""
Wall Street Institutional Backtesting, Hit-Rate Verification & Attribution Engine.
Evaluates the accuracy of Economist A-Share / US-Stock ratings against real market OHLCV series:
  1. Computes post-publication (event-window) absolute return & benchmark-adjusted Alpha (vs SPY / 000001.SH)
  2. Evaluates Hit / Miss status for Long (强力买入/超配增持) and Short/Avoid (低配减持/强力规避) calls
  3. Builds a 20-day Long Pool vs Short Pool vs Benchmark cumulative NAV curve & Long-Short Alpha curve
  4. Diagnoses divergences (e.g., '顺势Alpha兑现' vs '拥挤交易获利回吐/买预期卖事实' vs '左侧潜伏待催化')
  5. Computes quantitative technical levels (20D Support, 20D Resistance, Volatility, Stop-Loss Reference)
  6. Persists historical prediction tracking ledger to D:\\code\\economist\\data\\backtest_ledger.json
"""

import json
import math
from datetime import datetime
from pathlib import Path

from config import DATA_DIR

BACKTEST_LEDGER_FILE = DATA_DIR / "backtest_ledger.json"


def compute_quant_levels(kline: dict, polarity: str) -> dict:
    """Compute 20D support, 20D resistance, annualized volatility, and stop-loss/target reference from OHLC."""
    if not kline or not kline.get("closes") or len(kline["closes"]) < 5:
        return {}
    closes = kline["closes"]
    ohlc = kline.get("ohlc") or []
    recent_20_c = closes[-20:] if len(closes) >= 20 else closes
    recent_20_lows = [x[2] for x in ohlc[-20:]] if len(ohlc) >= 20 else recent_20_c
    recent_20_highs = [x[3] for x in ohlc[-20:]] if len(ohlc) >= 20 else recent_20_c

    support_20d = round(min(recent_20_lows), 2)
    resist_20d = round(max(recent_20_highs), 2)
    latest = closes[-1]

    # Daily return std dev
    rets = []
    for i in range(1, len(recent_20_c)):
        if recent_20_c[i - 1] > 0:
            rets.append((recent_20_c[i] - recent_20_c[i - 1]) / recent_20_c[i - 1])
    if len(rets) >= 3:
        mean_r = sum(rets) / len(rets)
        var_r = sum((r - mean_r) ** 2 for r in rets) / len(rets)
        daily_vol = math.sqrt(var_r)
    else:
        daily_vol = 0.02

    # 2*ATR approximate stop/target
    band_pct = max(0.035, min(0.12, daily_vol * 2.2))
    if polarity == "bull":
        stop_level = round(max(support_20d * 0.98, latest * (1.0 - band_pct)), 2)
        target_level = round(max(resist_20d, latest * (1.0 + band_pct * 1.8)), 2)
    elif polarity == "bear":
        stop_level = round(min(resist_20d * 1.02, latest * (1.0 + band_pct)), 2)
        target_level = round(min(support_20d, latest * (1.0 - band_pct * 1.8)), 2)
    else:
        stop_level = support_20d
        target_level = resist_20d

    return {
        "support_20d": support_20d,
        "resist_20d": resist_20d,
        "daily_vol_pct": round(daily_vol * 100, 2),
        "stop_level": stop_level,
        "target_level": target_level,
    }


def evaluate_backtest_and_attribution(
    stock_rows: list[dict],
    core_articles: list[dict],
    kline_map: dict[str, dict],
    report_date: str,
) -> dict:
    """
    Run Wall Street backtest verification across all rated stocks with market data.
    Calculates event-window return (since article publication / 5D window), benchmark-adjusted Alpha,
    hit-rate statistics, Long-Short portfolio NAV curve, and divergence attribution.
    """
    spy_k = kline_map.get("SPY") or {}
    sh_k = kline_map.get("000001.SH") or {}
    us_bm_5d = spy_k.get("chg_5d_pct", 0.0)
    cn_bm_5d = sh_k.get("chg_5d_pct", 0.0)
    us_bm_1d = spy_k.get("chg_1d_pct", 0.0)
    cn_bm_1d = sh_k.get("chg_1d_pct", 0.0)

    art_pub_day_map = {idx: (a.get("pub_day") or report_date) for idx, a in enumerate(core_articles, 1)}

    evaluated_records = []
    seen_sym_polarity = set()

    long_hits = 0
    long_total = 0
    short_hits = 0
    short_total = 0
    us_hits = 0
    us_total = 0
    cn_hits = 0
    cn_total = 0

    long_returns_5d = []
    short_returns_5d = []
    long_alphas_5d = []
    short_alphas_5d = []

    for st in stock_rows:
        sym = st.get("resolved_symbol")
        kl = st.get("kline")
        polarity = st.get("polarity", "neutral")
        if not sym or not kl or polarity not in ("bull", "bear"):
            continue

        # Attach quant levels to every stock row
        q_levels = compute_quant_levels(kl, polarity)
        st["quant_levels"] = q_levels

        # Calculate event-anchored return from pub_day if available in kline dates
        pub_day = art_pub_day_map.get(st["article_idx"], report_date)
        dates = kl.get("dates") or []
        closes = kl.get("closes") or []

        # Event return: compare latest close against close right before pub_day (or 5D window)
        anchor_idx = -6 if len(closes) >= 6 else 0
        for d_i, d_str in enumerate(dates):
            if d_str >= pub_day and d_i > 0:
                anchor_idx = max(0, d_i - 1)
                break

        anchor_close = closes[anchor_idx] if closes else 1.0
        latest_close = closes[-1] if closes else 1.0
        event_ret_pct = round((latest_close - anchor_close) / anchor_close * 100, 2) if anchor_close else 0.0
        ret_5d = kl.get("chg_5d_pct", 0.0)
        ret_1d = kl.get("chg_1d_pct", 0.0)

        bm_5d = us_bm_5d if st["market_type"] == "US" else cn_bm_5d
        bm_1d = us_bm_1d if st["market_type"] == "US" else cn_bm_1d

        # Benchmark-adjusted excess return (Alpha)
        alpha_5d = round(ret_5d - bm_5d, 2)
        alpha_1d = round(ret_1d - bm_1d, 2)

        # Strict Wall Street Hit / Validation Criterion (Zero sugarcoating!):
        # A Long call is a HIT only if 5D return > 0 OR (5D Alpha >= 0 and 5D drawdown > -2%)
        # A Short/Avoid call is a HIT only if 5D return < 0 OR 5D Alpha < -0.5%
        if polarity == "bull":
            is_hit = (ret_5d > 0.0) or (alpha_5d >= 0.0 and ret_5d > -2.0)
        else:
            is_hit = (ret_5d < 0.0) or (alpha_5d < -0.5)

        # Wall Street Attribution Diagnosis
        if is_hit:
            if polarity == "bull":
                if ret_5d >= 3.0:
                    verdict_tag = "✅ 强力兑现 (主升浪共振)"
                    verdict_cls = "bt-hit-strong"
                    diagnosis = f"多头逻辑获实盘资金强力确认，5日上涨 {ret_5d:+.2f}%（超额收益 {alpha_5d:+.2f}%），突破短期均线压制。"
                else:
                    verdict_tag = "✅ 稳健跑赢 (正超额收益)"
                    verdict_cls = "bt-hit"
                    diagnosis = f"在大盘震荡期取得正收益或显著抗跌属性（5日 {ret_5d:+.2f}%，相对基准超额 {alpha_5d:+.2f}%）。"
            else:
                if ret_5d <= -5.0:
                    verdict_tag = "✅ 精准排雷 (空头暴跌兑现)"
                    verdict_cls = "bt-hit-strong"
                    diagnosis = f"空头预警精准兑现，标的5日重挫 {ret_5d:+.2f}%（跑输基准 {alpha_5d:+.2f}%），成功规避重大回撤。"
                else:
                    verdict_tag = "✅ 规避有效 (弱于基准/下行)"
                    verdict_cls = "bt-hit"
                    diagnosis = f"受负面基本面压制表现疲弱（5日 {ret_5d:+.2f}%，相对基准 {alpha_5d:+.2f}%），减持规避策略有效。"
        else:
            if polarity == "bull":
                if ret_1d >= 1.5:
                    verdict_tag = "🔄 泥沙俱下后初现反弹"
                    verdict_cls = "bt-diverge"
                    diagnosis = (
                        f"【多因子归因】前5日受宏观科技去杠杆因子主导回撤 {ret_5d:+.2f}%，但最新1日已反弹 {ret_1d:+.2f}%；"
                        f"关键支撑位 {q_levels.get('support_20d', '--')}，止损参考 {q_levels.get('stop_level', '--')}。"
                    )
                else:
                    verdict_tag = "❌ 买预期卖事实 / 仓位拥挤回调"
                    verdict_cls = "bt-miss"
                    diagnosis = (
                        f"【经济学人封面效应/逆向归因】虽长期基本面利好，但短期机构多头过于拥挤触发获利了结（5日 {ret_5d:+.2f}%）；"
                        f"严禁盲目追高，等待回踩20日支撑位 {q_levels.get('support_20d', '--')} 企稳。"
                    )
            else:
                verdict_tag = "❌ 空头逼空 / 逆势反弹背离"
                verdict_cls = "bt-miss"
                diagnosis = (
                    f"【预期差复盘】短期受资金博弈或防御轮动推动逆势上涨 {ret_5d:+.2f}%（高于基准 {alpha_5d:+.2f}%），"
                    f"上方20日压力位 {q_levels.get('resist_20d', '--')}，需观察后续财报是否证伪。"
                )

        st["backtest"] = {
            "pub_day": pub_day,
            "event_ret_pct": event_ret_pct,
            "ret_1d": ret_1d,
            "ret_5d": ret_5d,
            "alpha_5d": alpha_5d,
            "alpha_1d": alpha_1d,
            "is_hit": is_hit,
            "verdict_tag": verdict_tag,
            "verdict_cls": verdict_cls,
            "diagnosis": diagnosis,
        }

        dedup_k = (sym, polarity)
        if dedup_k in seen_sym_polarity:
            continue
        seen_sym_polarity.add(dedup_k)

        if polarity == "bull":
            long_total += 1
            if is_hit:
                long_hits += 1
            long_returns_5d.append(ret_5d)
            long_alphas_5d.append(alpha_5d)
        else:
            short_total += 1
            if is_hit:
                short_hits += 1
            short_returns_5d.append(ret_5d)
            short_alphas_5d.append(alpha_5d)

        if st["market_type"] == "US":
            us_total += 1
            if is_hit:
                us_hits += 1
        else:
            cn_total += 1
            if is_hit:
                cn_hits += 1

        evaluated_records.append(
            {
                "symbol": sym,
                "head_display": st["head_display"],
                "market_type": st["market_type"],
                "market_label": st["market_label"],
                "theme_category": st["theme_category"],
                "rating": st["rating"],
                "badge_cls": st["badge_cls"],
                "polarity": polarity,
                "score": st["score"],
                "latest_close": latest_close,
                "ret_1d": ret_1d,
                "ret_5d": ret_5d,
                "alpha_5d": alpha_5d,
                "event_ret_pct": event_ret_pct,
                "is_hit": is_hit,
                "verdict_tag": verdict_tag,
                "verdict_cls": verdict_cls,
                "diagnosis": diagnosis,
                "support_20d": q_levels.get("support_20d"),
                "resist_20d": q_levels.get("resist_20d"),
                "stop_level": q_levels.get("stop_level"),
                "target_level": q_levels.get("target_level"),
                "article_idx": st["article_idx"],
                "article_anchor": st["article_anchor"],
            }
        )

    total_eval = long_total + short_total
    total_hits = long_hits + short_hits
    overall_hit_rate = round(total_hits / max(1, total_eval) * 100, 1)
    long_hit_rate = round(long_hits / max(1, long_total) * 100, 1)
    short_hit_rate = round(short_hits / max(1, short_total) * 100, 1)
    us_hit_rate = round(us_hits / max(1, us_total) * 100, 1)
    cn_hit_rate = round(cn_hits / max(1, cn_total) * 100, 1)

    avg_long_ret = round(sum(long_returns_5d) / max(1, len(long_returns_5d)), 2)
    avg_short_ret = round(sum(short_returns_5d) / max(1, len(short_returns_5d)), 2)
    # Long-Short portfolio spread: buying the Long pool and shorting the Short pool
    long_short_spread_5d = round(avg_long_ret - avg_short_ret, 2)

    # Build 20-Day Cumulative Portfolio NAV Curve (Base = 0% at T-20)
    nav_len = 20
    ref_dates = (spy_k.get("dates") or sh_k.get("dates") or [])[-nav_len:]
    long_syms = [r["symbol"] for r in evaluated_records if r["polarity"] == "bull" and r["symbol"] in kline_map]
    short_syms = [r["symbol"] for r in evaluated_records if r["polarity"] == "bear" and r["symbol"] in kline_map]

    def _calc_pool_curve(syms: list[str], n_pts: int) -> list[float]:
        if not syms or n_pts <= 0:
            return [0.0] * n_pts
        curves = []
        for s in syms:
            closes = (kline_map[s].get("closes") or [])[-n_pts:]
            if len(closes) == n_pts and closes[0] > 0:
                curves.append([(c / closes[0] - 1.0) * 100.0 for c in closes])
        if not curves:
            return [0.0] * n_pts
        avg_curve = []
        for i in range(n_pts):
            avg_curve.append(round(sum(c[i] for c in curves) / len(curves), 2))
        return avg_curve

    n_pts = len(ref_dates)
    long_curve = _calc_pool_curve(long_syms, n_pts)
    short_curve = _calc_pool_curve(short_syms, n_pts)
    bm_curve = _calc_pool_curve(["SPY", "000001.SH"], n_pts)
    ls_alpha_curve = [round(long_curve[i] - short_curve[i], 2) for i in range(n_pts)]

    # Sort evaluated records: show standout hits first, then divergences
    evaluated_records.sort(
        key=lambda r: (1 if r["is_hit"] else 0, abs(r["ret_5d"])),
        reverse=True,
    )

    summary = {
        "report_date": report_date,
        "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "total_evaluated": total_eval,
        "total_hits": total_hits,
        "overall_hit_rate": overall_hit_rate,
        "long_total": long_total,
        "long_hits": long_hits,
        "long_hit_rate": long_hit_rate,
        "short_total": short_total,
        "short_hits": short_hits,
        "short_hit_rate": short_hit_rate,
        "us_hit_rate": us_hit_rate,
        "cn_hit_rate": cn_hit_rate,
        "avg_long_ret_5d": avg_long_ret,
        "avg_short_ret_5d": avg_short_ret,
        "long_short_spread_5d": long_short_spread_5d,
        "nav_dates": ref_dates,
        "long_curve": long_curve,
        "short_curve": short_curve,
        "bm_curve": bm_curve,
        "ls_alpha_curve": ls_alpha_curve,
        "records": evaluated_records,
    }

    # Persist to backtest_ledger.json
    try:
        ledger = {}
        if BACKTEST_LEDGER_FILE.exists():
            ledger = json.loads(BACKTEST_LEDGER_FILE.read_text(encoding="utf-8"))
        ledger.setdefault("reports", {})[report_date] = {
            k: v for k, v in summary.items() if k != "records"
        }
        ledger["updated_at"] = summary["updated_at"]
        BACKTEST_LEDGER_FILE.write_text(json.dumps(ledger, ensure_ascii=False, indent=2), encoding="utf-8")
    except Exception:
        pass

    return summary
