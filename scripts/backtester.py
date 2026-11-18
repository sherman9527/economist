# -*- coding: utf-8 -*-
"""
Wall Street Institutional Backtesting, Hit-Rate Verification & Attribution Engine.
Evaluates the accuracy of Economist A-Share / US-Stock ratings against real market OHLCV series:
  1. Computes post-publication (event-window) absolute return & benchmark-adjusted Alpha (vs SPY / 000001.SH)
  2. Evaluates Hit / Miss status for Long (强力买入/超配增持) and Short/Avoid (低配减持/强力规避) calls
  3. Builds a 20-day Long Pool vs Short Pool vs Benchmark cumulative NAV curve & Long-Short Alpha curve
  4. Diagnoses divergences (e.g., '顺势Alpha兑现' vs '拥挤交易获利回吐/买预期卖事实' vs '左侧潜伏待催化')
  5. Computes quantitative technical levels (20D Support, 20D Resistance, Volatility, Stop-Loss Reference)
  6. Persists historical prediction tracking ledger to ./data/backtest_ledger.json
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

    def _compute_oos_window(kl_obj: dict, cutoff_day: str) -> dict:
        dates = kl_obj.get("dates") or []
        closes = kl_obj.get("closes") or []
        if len(dates) < 5 or len(dates) != len(closes):
            return {"pending": True, "fwd_bars": 0, "ret_pct": 0.0}
        anchor_idx = None
        for idx_d, d_str in enumerate(dates):
            if d_str < cutoff_day:
                anchor_idx = idx_d
            else:
                break
        if anchor_idx is None:
            return {"pending": True, "fwd_bars": 0, "ret_pct": 0.0}
        fwd_bars = (len(dates) - 1) - anchor_idx
        if fwd_bars <= 0 or dates[-1] < cutoff_day or closes[anchor_idx] <= 0:
            return {
                "pending": True,
                "fwd_bars": 0,
                "anchor_date": dates[anchor_idx],
                "latest_date": dates[-1],
                "ret_pct": kl_obj.get("chg_5d_pct", 0.0),
            }
        ret_oos = round((closes[-1] / closes[anchor_idx] - 1.0) * 100.0, 2)
        return {
            "pending": False,
            "fwd_bars": fwd_bars,
            "anchor_date": dates[anchor_idx],
            "anchor_close": closes[anchor_idx],
            "latest_date": dates[-1],
            "latest_close": closes[-1],
            "ret_pct": ret_oos,
        }

    us_bm_oos = _compute_oos_window(spy_k, report_date)
    cn_bm_oos = _compute_oos_window(sh_k, report_date)

    art_pub_day_map = {idx: (a.get("pub_day") or report_date) for idx, a in enumerate(core_articles, 1)}

    evaluated_records = []
    pending_records = []
    seen_sym_polarity = set()

    long_hits = 0
    long_total = 0
    short_hits = 0
    short_total = 0
    us_hits = 0
    us_total = 0
    cn_hits = 0
    cn_total = 0
    pending_us_total = 0
    pending_cn_total = 0

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

        pub_day = art_pub_day_map.get(st["article_idx"], report_date)
        closes = kl.get("closes") or []
        latest_close = closes[-1] if closes else 1.0

        oos_info = _compute_oos_window(kl, report_date)
        is_pending = oos_info["pending"]
        fwd_bars = oos_info["fwd_bars"]

        ret_1d = kl.get("chg_1d_pct", 0.0)
        bm_1d = us_bm_1d if st["market_type"] == "US" else cn_bm_1d
        alpha_1d = round(ret_1d - bm_1d, 2)

        if not is_pending:
            eval_ret = oos_info["ret_pct"]
            bm_oos_obj = us_bm_oos if st["market_type"] == "US" else cn_bm_oos
            bm_eval = bm_oos_obj["ret_pct"] if not bm_oos_obj["pending"] else 0.0
            eval_alpha = round(eval_ret - bm_eval, 2)
            eval_window_tag = f"报告后+{fwd_bars}日"
        else:
            eval_ret = kl.get("chg_5d_pct", 0.0)
            bm_eval = us_bm_5d if st["market_type"] == "US" else cn_bm_5d
            eval_alpha = round(eval_ret - bm_eval, 2)
            eval_window_tag = "待开盘(前5日)"

        ret_5d = eval_ret
        alpha_5d = eval_alpha
        event_ret_pct = eval_ret

        if is_pending:
            is_hit = False
            verdict_tag = "⏳ 待开盘样本外验证"
            verdict_cls = "gs-rating-ew"
            diagnosis = (
                f"报告发布于 {report_date}，对应市场尚未产生报告后收盘K线（基准锚定价 {latest_close}）；"
                f"发布前5日趋势参考 {eval_ret:+.2f}%，关键支撑 {q_levels.get('support_20d', '--')} / 压力 {q_levels.get('resist_20d', '--')}。"
            )
        else:
            # Strict Out-of-Sample Wall Street Hit / Validation Criterion:
            # Bull call hits if OOS return > 0% OR (OOS Alpha > 0% and drawdown > -2%)
            # Bear/Avoid call hits if OOS return < 0% OR OOS Alpha < -0.5%
            if polarity == "bull":
                is_hit = (ret_5d > 0.0) or (alpha_5d > 0.0 and ret_5d > -2.0)
            else:
                is_hit = (ret_5d < 0.0) or (alpha_5d < -0.5)

            if is_hit:
                if polarity == "bull":
                    if ret_5d >= 3.0:
                        verdict_tag = "✅ 强力兑现 (主升浪共振)"
                        verdict_cls = "bt-hit-strong"
                        diagnosis = f"多头逻辑获样本外资金强力确认，{eval_window_tag}上涨 {ret_5d:+.2f}%（超额收益 {alpha_5d:+.2f}%），突破短期均线压制。"
                    else:
                        verdict_tag = "✅ 稳健跑赢 (正收益/正Alpha)"
                        verdict_cls = "bt-hit"
                        diagnosis = f"在报告发布后（{eval_window_tag}）取得正收益或抗跌超额表现（实盘 {ret_5d:+.2f}%，相对基准超额 {alpha_5d:+.2f}%）。"
                else:
                    if ret_5d <= -5.0:
                        verdict_tag = "✅ 精准排雷 (空头下跌兑现)"
                        verdict_cls = "bt-hit-strong"
                        diagnosis = f"空头预警在样本外精准兑现，{eval_window_tag}下挫 {ret_5d:+.2f}%（相对基准 {alpha_5d:+.2f}%），成功规避重大回撤。"
                    else:
                        verdict_tag = "✅ 规避有效 (下行/跑输基准)"
                        verdict_cls = "bt-hit"
                        diagnosis = f"受基本面逆风压制样本外表现疲弱（{eval_window_tag} {ret_5d:+.2f}%，相对基准 {alpha_5d:+.2f}%），低配规避策略有效。"
            else:
                if polarity == "bull":
                    if ret_1d >= 1.5:
                        verdict_tag = "🔄 回调后初现企稳反弹"
                        verdict_cls = "bt-diverge"
                        diagnosis = (
                            f"【多因子归因】{eval_window_tag}受板块Beta去杠杆拖累回撤 {ret_5d:+.2f}%，最新1日已反弹 {ret_1d:+.2f}%；"
                            f"关键支撑位 {q_levels.get('support_20d', '--')}，止损参考 {q_levels.get('stop_level', '--')}。"
                        )
                    else:
                        verdict_tag = "❌ 买预期卖事实 / 拥挤回调"
                        verdict_cls = "bt-miss"
                        diagnosis = (
                            f"【经济学人叙事滞后/逆向归因】虽中长期逻辑顺畅，但短期多头仓位拥挤或大盘Beta回落（{eval_window_tag} {ret_5d:+.2f}%）；"
                            f"严禁追高，等待回踩20日支撑位 {q_levels.get('support_20d', '--')} 企稳。"
                        )
                else:
                    verdict_tag = "❌ 空头逼空 / 逆势上涨背离"
                    verdict_cls = "bt-miss"
                    diagnosis = (
                        f"【预期差复盘】{eval_window_tag}受资金博弈或政策对冲推动逆势上涨 {ret_5d:+.2f}%（高于基准 {alpha_5d:+.2f}%），"
                        f"上方20日压力位 {q_levels.get('resist_20d', '--')}，需观察后续财报是否证伪。"
                    )

        st["backtest"] = {
            "pub_day": pub_day,
            "is_pending": is_pending,
            "fwd_bars": fwd_bars,
            "eval_window_tag": eval_window_tag,
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

        rec_item = {
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
            "is_pending": is_pending,
            "eval_window_tag": eval_window_tag,
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

        if is_pending:
            if st["market_type"] == "US":
                pending_us_total += 1
            else:
                pending_cn_total += 1
            pending_records.append(rec_item)
            continue

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

        evaluated_records.append(rec_item)

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

    # Sort evaluated records: show standout hits first, then divergences, then pending at end
    evaluated_records.sort(
        key=lambda r: (1 if r["is_hit"] else 0, abs(r["ret_5d"])),
        reverse=True,
    )
    all_display_records = evaluated_records + pending_records

    if us_total == 0 and pending_us_total > 0:
        mkt_leader_note = f"A/港已验 {cn_total} 只 · 美股 {pending_us_total} 只待开盘"
    elif cn_total == 0 and pending_cn_total > 0:
        mkt_leader_note = f"美股已验 {us_total} 只 · A/港 {pending_cn_total} 只待开盘"
    elif us_hit_rate >= cn_hit_rate + 3.0:
        mkt_leader_note = f"美股样本外兑现领先 ({us_hits}/{us_total} vs A/港 {cn_hits}/{cn_total})"
    elif cn_hit_rate >= us_hit_rate + 3.0:
        mkt_leader_note = f"A股/港股样本外兑现领先 ({cn_hits}/{cn_total} vs 美股 {us_hits}/{us_total})"
    else:
        mkt_leader_note = f"跨市场样本外胜率均衡 (美股 {us_hits}/{us_total} · A/港 {cn_hits}/{cn_total})"

    summary = {
        "report_date": report_date,
        "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "total_evaluated": total_eval,
        "total_hits": total_hits,
        "pending_total": pending_us_total + pending_cn_total,
        "overall_hit_rate": overall_hit_rate,
        "long_total": long_total,
        "long_hits": long_hits,
        "long_hit_rate": long_hit_rate,
        "short_total": short_total,
        "short_hits": short_hits,
        "short_hit_rate": short_hit_rate,
        "us_total": us_total,
        "cn_total": cn_total,
        "us_hit_rate": us_hit_rate,
        "cn_hit_rate": cn_hit_rate,
        "mkt_leader_note": mkt_leader_note,
        "avg_long_ret_5d": avg_long_ret,
        "avg_short_ret_5d": avg_short_ret,
        "long_short_spread_5d": long_short_spread_5d,
        "nav_dates": ref_dates,
        "long_curve": long_curve,
        "short_curve": short_curve,
        "bm_curve": bm_curve,
        "ls_alpha_curve": ls_alpha_curve,
        "records": all_display_records,
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
