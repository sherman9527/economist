# -*- coding: utf-8 -*-
"""
Checkpointed Deep Market Impact Analyzer powered by GitHub Copilot CLI
(Model: gemini-4-argon, Context: long_context [983k], Reasoning: high).

5-Stage Granular Checkpoint Architecture:
  Stage 1: rss_fetched          - Multi-channel RSS aggregation & filtering
  Stage 2: articles_enriched    - Per-article r.jina.ai lead paragraph & rubric enrichment
  Stage 3: batches_analyzed     - Deep A-share & US-stock analysis per batch (saved after EVERY batch)
  Stage 4: digest_analyzed      - Concise Chinese summary & market tag for remaining secondary articles
  Stage 5: macro_synthesized    - Cross-article macro synthesis, transmission chains, and sector matrix
"""

import json
import os
import re
import subprocess
import time
from datetime import datetime
from pathlib import Path
from typing import Callable, Optional

from config import (
    CHECKPOINTS_DIR,
    COPILOT_CONTEXT_TIER,
    COPILOT_LOGS_DIR,
    COPILOT_MODEL,
    COPILOT_REASONING_EFFORT,
    DATA_DIR,
    RUNS_DIR,
    TEMP_DIR,
)

GLOBAL_ANALYZED_CACHE_FILE = DATA_DIR / "analyzed_cache.json"


def load_global_analyzed_cache() -> dict:
    if GLOBAL_ANALYZED_CACHE_FILE.exists():
        try:
            return json.loads(GLOBAL_ANALYZED_CACHE_FILE.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {"core": {}, "digest": {}}


def save_global_analyzed_cache(cache_data: dict) -> None:
    tmp = GLOBAL_ANALYZED_CACHE_FILE.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(cache_data, ensure_ascii=False, indent=2), encoding="utf-8")
    tmp.replace(GLOBAL_ANALYZED_CACHE_FILE)


def load_checkpoint(run_key: str) -> dict:
    cp_path = CHECKPOINTS_DIR / f"checkpoint_{run_key}.json"
    if cp_path.exists():
        try:
            return json.loads(cp_path.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {
        "run_key": run_key,
        "created_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "stage": "init",
        "world_in_brief": "",
        "articles": [],
        "core_article_ids": [],
        "digest_article_ids": [],
        "analyzed_by_id": {},
        "digest_by_id": {},
        "macro_synthesis": None,
    }


def save_checkpoint(cp: dict) -> Path:
    run_key = cp["run_key"]
    cp["updated_at"] = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    cp_path = CHECKPOINTS_DIR / f"checkpoint_{run_key}.json"
    tmp_path = cp_path.with_suffix(".json.tmp")
    tmp_path.write_text(json.dumps(cp, ensure_ascii=False, indent=2), encoding="utf-8")
    tmp_path.replace(cp_path)
    return cp_path


def extract_json_payload(raw_text: str):
    """Robustly extract JSON object or array from Copilot CLI response."""
    text = raw_text.strip()
    # Strip markdown code fences if present
    fence_matches = re.findall(r"```(?:json)?\s*(.*?)```", text, flags=re.S | re.I)
    candidates = fence_matches + [text]

    for cand in candidates:
        cand = cand.strip()
        if not cand:
            continue
        try:
            return json.loads(cand)
        except Exception:
            pass
        # Find outermost {...} or [...]
        first_brace = cand.find("{")
        first_bracket = cand.find("[")
        starts = [idx for idx in (first_brace, first_bracket) if idx != -1]
        if not starts:
            continue
        start_idx = min(starts)
        open_ch = cand[start_idx]
        close_ch = "}" if open_ch == "{" else "]"
        end_idx = cand.rfind(close_ch)
        if end_idx > start_idx:
            sub = cand[start_idx : end_idx + 1]
            try:
                return json.loads(sub)
            except Exception:
                # Attempt trailing comma cleanup
                cleaned = re.sub(r",\s*([}\]])", r"\1", sub)
                try:
                    return json.loads(cleaned)
                except Exception:
                    pass
    raise ValueError(f"Unable to parse JSON from Copilot output (len={len(raw_text)}): {raw_text[:400]}")


def call_copilot_gemini(
    prompt: str,
    retries: int = 3,
    timeout_sec: int = 240,
    log_fn: Callable[[str], None] = print,
) -> any:
    """
    Invoke Copilot CLI using gemini-4-argon + long_context (983k) + high reasoning effort.
    Includes automatic retry with exponential backoff and strict JSON validation.
    """
    env = os.environ.copy()
    env["TEMP"] = str(TEMP_DIR)
    env["TMP"] = str(TEMP_DIR)
    env["PYTHONIOENCODING"] = "utf-8"
    env.pop("PYTHONHOME", None)
    env.pop("PYTHONPATH", None)

    cmd = [
        "copilot",
        "-s",
        "--model",
        COPILOT_MODEL,
        "--context",
        COPILOT_CONTEXT_TIER,
        "--reasoning-effort",
        COPILOT_REASONING_EFFORT,
        "--no-custom-instructions",
        "--disable-builtin-mcps",
        "--disable-mcp-server",
        "diagnostics-mcp",
        "--disable-client-telemetry",
        "--available-tools",
        "",
        "--log-dir",
        str(COPILOT_LOGS_DIR),
    ]

    last_err = None
    for attempt in range(1, retries + 1):
        t0 = time.time()
        try:
            proc = subprocess.run(
                cmd,
                input=prompt,
                text=True,
                encoding="utf-8",
                capture_output=True,
                timeout=timeout_sec,
                env=env,
            )
            elapsed = time.time() - t0
            if proc.returncode != 0:
                raise RuntimeError(
                    f"Copilot exited with code {proc.returncode}: {(proc.stderr or proc.stdout)[:300]}"
                )
            parsed = extract_json_payload(proc.stdout)
            log_fn(f"[Copilot] ✓ {COPILOT_MODEL} ({COPILOT_CONTEXT_TIER}/{COPILOT_REASONING_EFFORT}) succeeded in {elapsed:.1f}s")
            return parsed
        except Exception as e:
            last_err = e
            log_fn(f"[Copilot] ⚠ Attempt {attempt}/{retries} failed ({e})")
            if attempt < retries:
                time.sleep(4 * attempt)
    raise RuntimeError(f"Copilot CLI failed after {retries} attempts: {last_err}")


def build_deep_batch_prompt(batch_articles: list[dict], world_in_brief: str = "") -> str:
    payload = []
    for a in batch_articles:
        payload.append(
            {
                "id": a["id"],
                "title": a["title"],
                "subtitle": a.get("description", ""),
                "fly_title": a.get("fly_title", ""),
                "sections": a.get("section_labels", []),
                "pub_date_cst": a.get("pub_date_cst", ""),
                "lead_excerpt": a.get("lead_excerpt", ""),
                "topics": a.get("topics", []),
                "link": a["link"],
            }
        )

    articles_json = json.dumps(payload, ensure_ascii=False, indent=2)
    brief_context = f"\n【同期《经济学人》The World in Brief 全球背景】：\n{world_in_brief[:1200]}\n" if world_in_brief else ""

    return f"""你是一位高盛全球投资研究部 (Goldman Sachs GIR) 与摩根士丹利研究部 (Morgan Stanley Research) 的联席跨市场（美股 + A股/港股）首席股票策略师。
请基于以下《经济学人》(The Economist) 最新文章，参考 **高盛/大摩卖方深度研报 (Sell-Side Equity Research Note)** 的分析框架（明确区分 Overweight 超配 / Equal-weight 标配 / Underweight 低配、拆解 EPS 盈利修正 vs P/E 估值倍数扩张/收缩、给出多空配对交易 Pair Trade），从 **A股/港股 与 美股 股票影响** 的视角进行机制级深度剖析。
{brief_context}
【待分析文章列表 (JSON)】：
{articles_json}

【分析深度与输出规范（高盛/大摩研报标准：结构清晰、要点分明、量化归因、严禁冗长砌词）】：
对列表中的每一篇文章，请输出以下字段：
1. `id`: 必须与输入文章的 `id` 完全一致。
2. `zh_title`: 精准、有投行研报锐度的中文标题翻译。
3. `theme_category`: 归类标签，仅限从以下选择其一：`["AI与硬科技", "美国政经与宏观", "中国金融与产业", "全球贸易与地缘", "大宗商品与能源", "消费与医疗商业"]`。
4. `core_thesis`: 原文核心事实与投行要点（用 `；` 或 `。` 分隔的 3 条精炼要点：第一句写原文硬核数据/事件，第二句写经济学底层供需/制度逻辑，第三句写跨市场核心定价结论，总字数 160~240字）。
5. `transmission_chain`: 跨市场四阶传导表（数组，4 个递进环节字符串，格式：`["一阶冲击：...", "二阶传导（利率/监管/供应链/CapEx）：...", "美股EPS与估值映射：...", "A股/港股映射与预期差：..."]`）。
6. `us_market`: 美股影响深度评估对象（高盛/大摩风格）：
   - `direction`: 仅限 `"强烈利好"`, `"温和利好"`, `"中性分化"`, `"温和利空"`, `"强烈利空"` 之一
   - `score`: 影响强度评分（整数 1~10，10为极高影响）
   - `sectors`: 受影响的美股核心板块数组（如 `["AI算力与云巨头 (UW)", "AI原生网络安全 (OW)"]`）
   - `tickers`: 具体美股代表性标的代码与投行评级数组（3~5个，必须标明多空方向或评级，格式如 `"PANW / CRWD (Palo Alto / CrowdStrike) - [OW 超配 | EPS上修] AI攻防升级驱动企业安全预算刚性扩张"` 或 `"NVDA (英伟达) - [UW 短期规避 | 估值收缩] 受监管与CapEx放缓担忧压制"`）
   - `deep_logic`: 美股深度多空逻辑剖析（分 3 个清晰层次：估值折现率冲击、EPS盈利弹性、机构仓位与多空博弈，150~220字）。
7. `cn_market`: A股与港股影响深度评估对象（高盛/大摩中国策略风格）：
   - `direction`: 仅限 `"强烈利好"`, `"温和利好"`, `"中性分化"`, `"温和利空"`, `"强烈利空"` 之一
   - `score`: 影响强度评分（整数 1~10，10为极高影响）
   - `sectors`: 受影响的A股/港股板块数组
   - `tickers`: 具体A股/港股核心代表性标的与投行评级数组（3~5个，格式如 `"中际旭创 (300308.SZ) / 新易盛 (300502.SZ) - [Conviction Buy 强推 | 订单与估值双升] 受益于全球AI硬件关税豁免与800G/1.6T需求"`）
   - `deep_logic`: A股/港股深度映射与内外盘预期差剖析（分 3 个清晰层次：外资与情绪传导、国内产业政策/份额替代、估值与基本面预期差，150~220字）。
8. `watch_indicators`: 后续 1~4 周需要重点跟踪的 2~3 个量化验证指标或催化剂（数组，每项 25~50 字）。

请严格只输出一个合法的 JSON 数组 `[...]`（不要输出任何额外说明文字），数组元素个数与输入文章数一致。"""


def build_digest_batch_prompt(digest_articles: list[dict]) -> str:
    payload = [
        {
            "id": a["id"],
            "title": a["title"],
            "subtitle": a.get("description", ""),
            "sections": a.get("section_labels", []),
            "pub_date_cst": a.get("pub_date_cst", ""),
        }
        for a in digest_articles
    ]
    return f"""请将以下《经济学人》同期其余文章翻译为精准中文标题，并给出简明核心提要及对A股/美股的潜在关联提示。
输入文章列表：
{json.dumps(payload, ensure_ascii=False, indent=2)}

请严格只输出一个合法的 JSON 数组 `[...]`，每个元素包含：
- `id`: 与输入一致
- `zh_title`: 中文标题
- `summary_zh`: 60~100字核心内容提要
- `market_note`: 30~60字对美股/A股的边际启示（若属纯社会文化话题则写明长周期或情绪面参考）
- `impact_level`: `"中"` 或 `"低"`"""


def build_macro_synthesis_prompt(
    analyzed_articles: list[dict],
    window_desc: str,
    world_in_brief: str = "",
) -> str:
    compact = []
    for a in analyzed_articles:
        compact.append(
            {
                "id": a["id"],
                "title": a.get("title"),
                "zh_title": a.get("zh_title"),
                "theme_category": a.get("theme_category"),
                "core_thesis": a.get("core_thesis"),
                "us_direction": a.get("us_market", {}).get("direction"),
                "us_score": a.get("us_market", {}).get("score"),
                "us_sectors": a.get("us_market", {}).get("sectors"),
                "cn_direction": a.get("cn_market", {}).get("direction"),
                "cn_score": a.get("cn_market", {}).get("score"),
                "cn_sectors": a.get("cn_market", {}).get("sectors"),
            }
        )

    return f"""你是顶级跨市场宏观策略首席。请基于《经济学人》({window_desc}) 已完成深度拆解的 {len(compact)} 篇重点文章，撰写一份纲举目张、逻辑严密的 **A股 / 美股 跨市场宏观总览与策略研判 JSON**。

【已完成拆解的重点文章汇总】：
{json.dumps(compact, ensure_ascii=False, indent=2)}

请严格只输出一个合法的 JSON 对象 `{{...}}`，结构如下：
{{
  "headline_zh": "本期总览主标题（20~35字，提炼最具冲击力的跨市场主线）",
  "executive_summary": "350~500字的深度宏观总览。要求串联本期《经济学人》揭示的全球核心政经与产业共振/背离（例如美国中期选举格局变化与AI资本开支重估、中国科技自主实验室升级与银行不良出清、全球科技贸易豁免与传统关税壁垒、商品周期与网络安全的攻防升级等），讲透对美股与A股定价锚的影响。",
  "us_market_overview": {{
    "overall_stance": "美股整体研判（如：短期高位震荡偏谨慎 / 结构性分化加剧）",
    "sentiment_score": 1到100的整数（50为中性，越高越偏多，越低越偏空），
    "core_drivers": ["核心驱动与压制因素1（80字左右）", "核心驱动与压制因素2（80字左右）", "核心驱动与压制因素3（80字左右）"],
    "long_short_allocation": {{
      "overweight": ["建议超配/关注方向1（含逻辑与代表Ticker）", "建议超配/关注方向2"],
      "underweight": ["建议规避/对冲方向1（含逻辑与代表Ticker）", "建议规避/对冲方向2"]
    }}
  }},
  "cn_market_overview": {{
    "overall_stance": "A股/港股整体研判（如：以我为主、硬科技自主可控与高股息化债双主线分化）",
    "sentiment_score": 1到100的整数（50为中性），
    "core_drivers": ["A股核心映射逻辑1（80字左右）", "A股核心映射逻辑2（80字左右）", "A股核心映射逻辑3（80字左右）"],
    "long_short_allocation": {{
      "overweight": ["A股/港股重点看好产业链1（含逻辑与代表标的）", "重点看好产业链2"],
      "underweight": ["需警惕风险或估值消化方向1", "需警惕方向2"]
    }}
  }},
  "cross_market_chains": [
    {{
      "chain_title": "核心跨市场传导主线名称（共生成 4 条主线）",
      "economist_trigger": "《经济学人》原文揭示的核心触发点",
      "macro_channel": "一阶/二阶宏观与产业传导媒介（利率/监管/供应链/CapEx）",
      "us_mapping": "美股映射表现与核心标的",
      "cn_mapping": "A股/港股映射表现与核心标的",
      "divergence_insight": "中美市场定价逻辑的核心差异与预期差"
    }}
  ],
  "risk_calendar_and_watchpoints": [
    "未来1~4周关键宏观/产业验证节点1",
    "关键验证节点2",
    "关键验证节点3",
    "关键验证节点4"
  ]
}}"""


def run_checkpointed_analysis(
    run_key: str,
    articles: list[dict],
    world_in_brief: str,
    window_desc: str,
    max_core_articles: int = 24,
    batch_size: int = 6,
    enrich_fn: Optional[Callable[[dict], dict]] = None,
    log_fn: Callable[[str], None] = print,
) -> dict:
    """
    Execute the 5-stage analysis pipeline with granular checkpointing.
    If interrupted at any batch, re-running resumes seamlessly from checkpoint_<run_key>.json.
    """
    cp = load_checkpoint(run_key)
    cp["window_desc"] = window_desc
    if world_in_brief:
        cp["world_in_brief"] = world_in_brief

    # Merge incoming articles with any already stored in checkpoint
    existing_by_id = {a["id"]: a for a in cp.get("articles", [])}
    merged_articles = []
    for art in articles:
        if art["id"] in existing_by_id:
            prev = existing_by_id[art["id"]]
            prev.update({k: v for k, v in art.items() if v})
            merged_articles.append(prev)
        else:
            merged_articles.append(dict(art))

    merged_articles.sort(key=lambda x: (x.get("relevance_score", 50), x.get("timestamp", 0)), reverse=True)
    cp["articles"] = merged_articles

    # Partition into Core Deep-Dive articles and Secondary Digest articles
    core_articles = [a for a in merged_articles if a.get("relevance_score", 0) >= 55][:max_core_articles]
    if len(core_articles) < min(8, len(merged_articles)):
        core_articles = merged_articles[: min(max_core_articles, len(merged_articles))]
    core_ids = [a["id"] for a in core_articles]
    core_id_set = set(core_ids)
    digest_articles = [a for a in merged_articles if a["id"] not in core_id_set]

    cp["core_article_ids"] = core_ids
    cp["digest_article_ids"] = [a["id"] for a in digest_articles]
    cp["stage"] = "rss_fetched"
    save_checkpoint(cp)

    # Stage 2: Enrich core articles via r.jina.ai (cached per article)
    if enrich_fn is not None:
        log_fn(f"[Pipeline] Stage 2: Enriching {len(core_articles)} core articles...")
        enriched_map = {}
        for idx, art in enumerate(core_articles, 1):
            enriched = enrich_fn(art)
            enriched_map[art["id"]] = enriched
            if idx % 5 == 0 or idx == len(core_articles):
                for i, item in enumerate(cp["articles"]):
                    if item["id"] in enriched_map:
                        cp["articles"][i] = enriched_map[item["id"]]
                save_checkpoint(cp)
        core_articles = [enriched_map.get(a["id"], a) for a in core_articles]
    cp["stage"] = "articles_enriched"
    save_checkpoint(cp)

    # Stage 3: Deep analysis in checkpointed batches
    global_cache = load_global_analyzed_cache()
    analyzed_by_id: dict = cp.get("analyzed_by_id", {})
    for a in core_articles:
        if a["id"] not in analyzed_by_id and a["id"] in global_cache.get("core", {}):
            analyzed_by_id[a["id"]] = global_cache["core"][a["id"]]
    cp["analyzed_by_id"] = analyzed_by_id

    pending_core = [a for a in core_articles if a["id"] not in analyzed_by_id]
    log_fn(
        f"[Pipeline] Stage 3: Core deep analysis — {len(analyzed_by_id)} cached in checkpoint/store, "
        f"{len(pending_core)} pending (batch_size={batch_size})"
    )

    for b_idx in range(0, len(pending_core), batch_size):
        batch = pending_core[b_idx : b_idx + batch_size]
        batch_num = (b_idx // batch_size) + 1
        total_batches = (len(pending_core) + batch_size - 1) // batch_size
        log_fn(
            f"[Pipeline] -> Analyzing Core Batch {batch_num}/{total_batches} "
            f"({len(batch)} articles) via {COPILOT_MODEL}..."
        )
        prompt = build_deep_batch_prompt(batch, world_in_brief=cp.get("world_in_brief", ""))
        result_list = call_copilot_gemini(prompt, retries=3, timeout_sec=300, log_fn=log_fn)
        if isinstance(result_list, dict) and "articles" in result_list:
            result_list = result_list["articles"]
        if not isinstance(result_list, list):
            raise RuntimeError(f"Expected JSON array from batch {batch_num}, got {type(result_list)}")

        result_map = {str(item.get("id", "")): item for item in result_list if isinstance(item, dict)}
        for idx_in_batch, src_art in enumerate(batch):
            aid = src_art["id"]
            item = result_map.get(aid)
            if not item and idx_in_batch < len(result_list) and isinstance(result_list[idx_in_batch], dict):
                item = result_list[idx_in_batch]
                item["id"] = aid
            if item:
                merged_res = dict(src_art)
                merged_res.update(item)
                analyzed_by_id[aid] = merged_res
                global_cache.setdefault("core", {})[aid] = merged_res

        cp["analyzed_by_id"] = analyzed_by_id
        save_checkpoint(cp)
        save_global_analyzed_cache(global_cache)
        log_fn(f"[Checkpoint] ✓ Saved Core Batch {batch_num}/{total_batches} ({len(analyzed_by_id)}/{len(core_articles)} done)")

    cp["stage"] = "batches_analyzed"
    save_checkpoint(cp)

    # Stage 4: Secondary digest articles translation & summary (if any)
    digest_by_id: dict = cp.get("digest_by_id", {})
    for a in digest_articles:
        if a["id"] not in digest_by_id and a["id"] in global_cache.get("digest", {}):
            digest_by_id[a["id"]] = global_cache["digest"][a["id"]]
    cp["digest_by_id"] = digest_by_id

    pending_digest = [a for a in digest_articles if a["id"] not in digest_by_id]
    if pending_digest:
        log_fn(f"[Pipeline] Stage 4: Summarizing {len(pending_digest)} secondary digest articles...")
        digest_batch_size = 15
        for d_idx in range(0, len(pending_digest), digest_batch_size):
            d_batch = pending_digest[d_idx : d_idx + digest_batch_size]
            d_prompt = build_digest_batch_prompt(d_batch)
            try:
                d_res = call_copilot_gemini(d_prompt, retries=2, timeout_sec=180, log_fn=log_fn)
                if isinstance(d_res, list):
                    d_map = {str(x.get("id", "")): x for x in d_res if isinstance(x, dict)}
                    for i_b, src_d in enumerate(d_batch):
                        did = src_d["id"]
                        matched = d_map.get(did) or (d_res[i_b] if i_b < len(d_res) else None)
                        if isinstance(matched, dict):
                            merged_d = dict(src_d)
                            merged_d.update(matched)
                            digest_by_id[did] = merged_d
                            global_cache.setdefault("digest", {})[did] = merged_d
            except Exception as e:
                log_fn(f"[Pipeline] ⚠ Secondary digest batch fallback: {e}")
                for src_d in d_batch:
                    digest_by_id[src_d["id"]] = {
                        **src_d,
                        "zh_title": src_d["title"],
                        "summary_zh": src_d.get("description") or "详见《经济学人》原文报道。",
                        "market_note": "长周期政经/社会文化背景参考",
                        "impact_level": "低",
                    }
            cp["digest_by_id"] = digest_by_id
            save_checkpoint(cp)
            save_global_analyzed_cache(global_cache)

    cp["stage"] = "digest_analyzed"
    save_checkpoint(cp)

    # Stage 5: Macro cross-article synthesis
    ordered_core = [analyzed_by_id[aid] for aid in core_ids if aid in analyzed_by_id]
    if not cp.get("macro_synthesis") and ordered_core:
        log_fn(f"[Pipeline] Stage 5: Running Macro & Cross-Market Synthesis via {COPILOT_MODEL}...")
        macro_prompt = build_macro_synthesis_prompt(
            ordered_core, window_desc=window_desc, world_in_brief=cp.get("world_in_brief", "")
        )
        macro_data = call_copilot_gemini(macro_prompt, retries=3, timeout_sec=240, log_fn=log_fn)
        cp["macro_synthesis"] = macro_data
        cp["stage"] = "macro_synthesized"
        save_checkpoint(cp)
        log_fn("[Checkpoint] ✓ Saved Stage 5 Macro Synthesis")

    # Save final structured run output to D:\code\economist\data\runs\analysis_<run_key>.json
    ordered_digest = [digest_by_id.get(a["id"], a) for a in digest_articles]
    final_output = {
        "run_key": run_key,
        "window_desc": window_desc,
        "generated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "model": f"{COPILOT_MODEL} ({COPILOT_CONTEXT_TIER}, reasoning={COPILOT_REASONING_EFFORT})",
        "total_articles": len(merged_articles),
        "core_count": len(ordered_core),
        "digest_count": len(ordered_digest),
        "world_in_brief": cp.get("world_in_brief", ""),
        "macro_synthesis": cp.get("macro_synthesis") or {},
        "core_articles": ordered_core,
        "digest_articles": ordered_digest,
    }
    run_file = RUNS_DIR / f"analysis_{run_key}.json"
    run_file.write_text(json.dumps(final_output, ensure_ascii=False, indent=2), encoding="utf-8")
    return final_output
