# -*- coding: utf-8 -*-
"""
Robust multi-source Economist RSS fetcher + r.jina.ai article content enricher.
Includes exponential backoff retries, per-article disk caching, and relevance scoring.
"""

import hashlib
import json
import re
import time
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET
from datetime import datetime, timedelta, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Callable, Optional

from config import ARTICLE_CACHE_DIR, ECONOMIST_RSS_SECTIONS

USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
)

HIGH_IMPACT_KEYWORDS = [
    "market", "stock", "bond", "yield", "fed", "inflation", "tariff", "trade",
    "china", "chinese", "bank", "ai", "tech", "chip", "semiconductor", "openai",
    "copper", "oil", "energy", "climate", "el nino", "dollar", "currency",
    "economy", "economic", "election", "midterm", "democrat", "republican", "trump",
    "merger", "acquisition", "ipo", "venture", "cyber", "health", "cvs", "doordash",
    "amazon", "walmart", "youtube", "hollywood", "renter", "housing", "property",
    "debt", "tycoon", "lender", "aramco", "saudi", "drone", "defense", "starlink",
    "industrial", "laboratory", "manufacturing", "export", "supply chain", "fiscal",
]

NON_MARKET_NOISE_PATTERNS = [
    r"\brecruiting\b",
    r"\bwanted:\s*a\s+news\s+researcher\b",
    r"^cartoon:",
]


def http_get_with_retry(
    url: str,
    retries: int = 4,
    timeout: int = 20,
    backoff_base: float = 2.0,
    extra_headers: Optional[dict] = None,
) -> bytes:
    """HTTP GET with exponential backoff retry."""
    headers = {"User-Agent": USER_AGENT, "Accept": "*/*"}
    if extra_headers:
        headers.update(extra_headers)

    last_err = None
    for attempt in range(1, retries + 1):
        try:
            req = urllib.request.Request(url, headers=headers)
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return resp.read()
        except Exception as e:
            last_err = e
            if attempt < retries:
                sleep_sec = backoff_base ** attempt
                time.sleep(sleep_sec)
    raise RuntimeError(f"Failed to fetch {url} after {retries} attempts: {last_err}")


def compute_article_id(link: str, guid: str = "") -> str:
    raw = (link or guid).strip().lower()
    return hashlib.sha1(raw.encode("utf-8")).hexdigest()[:16]


def score_article_market_relevance(article: dict) -> int:
    """Compute a market relevance score (0-100) for prioritizing deep A-share/US-stock analysis."""
    title = (article.get("title") or "").lower()
    desc = (article.get("description") or "").lower()
    text = f"{title} {desc}"

    for pat in NON_MARKET_NOISE_PATTERNS:
        if re.search(pat, title, flags=re.I):
            return 5

    if title in ("politics", "business") and not desc.strip():
        # Weekly summary compilation bullet lists remain useful but secondary to standalone deep pieces
        return 45

    base_weight = max(article.get("section_weights", [6])) * 6  # 30..60
    kw_hits = 0
    for kw in HIGH_IMPACT_KEYWORDS:
        if kw in text:
            kw_hits += 1
    score = base_weight + min(kw_hits * 7, 38)

    # Boost explicit Finance, Business, China, Science/Tech, Leaders, Briefing
    sections = set(article.get("sections", []))
    if sections & {"finance-and-economics", "business", "china"}:
        score += 12
    elif sections & {"leaders", "briefing", "science-and-technology", "united-states"}:
        score += 8

    return min(score, 100)


def fetch_all_rss_articles(log_fn: Callable[[str], None] = print) -> tuple[list[dict], datetime]:
    """
    Fetch and deduplicate articles across all 12 Economist RSS sections.
    Returns (sorted_articles_list, max_pub_datetime_utc).
    """
    by_link: dict[str, dict] = {}
    max_dt: Optional[datetime] = None

    for sec_slug, sec_label, sec_weight in ECONOMIST_RSS_SECTIONS:
        rss_url = f"https://www.economist.com/{sec_slug}/rss.xml"
        try:
            raw_xml = http_get_with_retry(rss_url, retries=4, timeout=20)
            root = ET.fromstring(raw_xml)
            items = root.findall(".//item")
            log_fn(f"[RSS] ✓ {sec_slug:24s} -> {len(items)} items")
        except Exception as e:
            log_fn(f"[RSS] ⚠ {sec_slug:24s} failed: {e}")
            continue

        for it in items:
            link = (it.findtext("link") or "").strip()
            title = (it.findtext("title") or "").strip()
            desc = (it.findtext("description") or "").strip()
            guid = (it.findtext("guid") or "").strip()
            pub_str = (it.findtext("pubDate") or "").strip()
            if not link or not title or not pub_str:
                continue

            # Clean HTML tags in description if any
            desc_clean = re.sub(r"<[^>]+>", " ", desc)
            desc_clean = re.sub(r"\s+", " ", desc_clean).strip()
            title_clean = re.sub(r"\s+", " ", title).strip()

            try:
                dt = parsedate_to_datetime(pub_str)
                if dt.tzinfo is None:
                    dt = dt.replace(tzinfo=timezone.utc)
                else:
                    dt = dt.astimezone(timezone.utc)
            except Exception:
                continue

            if max_dt is None or dt > max_dt:
                max_dt = dt

            if link not in by_link:
                art_id = compute_article_id(link, guid)
                by_link[link] = {
                    "id": art_id,
                    "guid": guid,
                    "link": link,
                    "title": title_clean,
                    "description": desc_clean,
                    "pub_date_utc": dt.strftime("%Y-%m-%d %H:%M:%S UTC"),
                    "pub_date_cst": dt.astimezone(timezone(timedelta(hours=8))).strftime("%Y-%m-%d %H:%M"),
                    "pub_day": dt.strftime("%Y-%m-%d"),
                    "timestamp": dt.timestamp(),
                    "sections": [sec_slug],
                    "section_labels": [sec_label],
                    "section_weights": [sec_weight],
                }
            else:
                entry = by_link[link]
                if sec_slug not in entry["sections"]:
                    entry["sections"].append(sec_slug)
                    entry["section_labels"].append(sec_label)
                    entry["section_weights"].append(sec_weight)

    if max_dt is None:
        max_dt = datetime.now(timezone.utc)

    articles = list(by_link.values())
    for art in articles:
        art["relevance_score"] = score_article_market_relevance(art)

    articles.sort(key=lambda x: (x["timestamp"], x["relevance_score"]), reverse=True)
    return articles, max_dt


def filter_articles_by_window(
    articles: list[dict],
    max_dt: datetime,
    days: int = 3,
    target_date: Optional[str] = None,
) -> list[dict]:
    """
    Filter articles within [cutoff, end_dt].
    If a single day has very few articles (e.g. weekend), expands window slightly so daily report stays rich.
    """
    if target_date:
        end_dt = datetime.strptime(target_date, "%Y-%m-%d").replace(
            hour=23, minute=59, second=59, tzinfo=timezone.utc
        )
        if end_dt > max_dt and target_date >= max_dt.strftime("%Y-%m-%d"):
            end_dt = max_dt
    else:
        end_dt = max_dt

    cutoff_dt = end_dt - timedelta(days=days)
    filtered = [
        a
        for a in articles
        if cutoff_dt.timestamp() <= a["timestamp"] <= end_dt.timestamp() + 86400
    ]

    # If daily 1d run falls on Sunday/Monday after Thursday's big weekly drop, include up to 4d so report has substance
    if len(filtered) < 8 and days < 4:
        fallback_cutoff = end_dt - timedelta(days=4)
        filtered = [
            a
            for a in articles
            if fallback_cutoff.timestamp() <= a["timestamp"] <= end_dt.timestamp() + 86400
        ]

    filtered.sort(key=lambda x: (x["relevance_score"], x["timestamp"]), reverse=True)
    return filtered


def parse_jina_markdown(md_text: str, fallback_title: str = "") -> dict:
    """Extract clean article metadata and lead paragraphs from r.jina.ai Markdown output."""
    lines = [ln.strip() for ln in md_text.splitlines()]
    fly_title = ""
    dateline = ""
    lead_paragraphs = []
    topics = []
    image_url = ""

    # Find headline position (# Title)
    h1_idx = -1
    for idx, ln in enumerate(lines):
        if ln.startswith("# "):
            h1_idx = idx
            break

    if h1_idx > 0:
        for back in range(max(0, h1_idx - 6), h1_idx):
            candidate = lines[back]
            if "|" in candidate and "economist.com" in candidate:
                clean_fly = re.sub(r"\[([^\]]+)\]\([^\)]+\)", r"\1", candidate).strip()
                fly_title = clean_fly
                break

    # Scan after h1 until paywall marker ("Already have an account?" or "Continue with a free trial")
    scan_start = h1_idx + 1 if h1_idx != -1 else 0
    in_explore_more = False
    for idx in range(scan_start, len(lines)):
        ln = lines[idx]
        if not ln:
            continue
        if "Already have an account?" in ln or "Continue with a free trial" in ln:
            break
        if ln.startswith("### Explore more"):
            in_explore_more = True
            continue
        if in_explore_more:
            if ln.startswith("* * *") or ln.startswith("## "):
                break
            m_topic = re.match(r"^\[([^\]]+)\]\(https://www\.economist\.com/topics/[^\)]+\)$", ln)
            if m_topic:
                topics.append(m_topic.group(1))
            continue

        if not image_url and ln.startswith("![Image"):
            m_img = re.search(r"\((https://www\.economist\.com/cdn-cgi/image/[^\)]+)\)", ln)
            if m_img:
                image_url = m_img.group(1)
            continue

        if re.search(r"\b202\d\b.*\|.*\bmin read\b", ln, flags=re.I) or re.search(r"\bmin read\b", ln, flags=re.I):
            if len(ln) < 100:
                dateline = ln
                continue

        if ln.startswith(("#", "!", "[", "Share", "Photograph:", "* * *", "Menu", "Subscribe", "Log in")):
            continue

        clean_para = re.sub(r"\[([^\]]+)\]\([^\)]+\)", r"\1", ln).strip()
        if len(clean_para) >= 45:
            lead_paragraphs.append(clean_para)

    return {
        "fly_title": fly_title,
        "dateline": dateline,
        "lead_excerpt": "\n\n".join(lead_paragraphs[:4]),
        "topics": topics[:6],
        "image_url": image_url,
    }


def enrich_single_article(article: dict, log_fn: Callable[[str], None] = print) -> dict:
    """
    Enrich an article with r.jina.ai extracted lead content and metadata.
    Uses persistent per-article cache under D:\\code\\economist\\data\\article_cache.
    """
    art_id = article["id"]
    cache_file = ARTICLE_CACHE_DIR / f"{art_id}.json"

    if cache_file.exists():
        try:
            cached = json.loads(cache_file.read_text(encoding="utf-8"))
            if cached.get("lead_excerpt") or cached.get("enriched_attempted"):
                merged = dict(article)
                merged.update(cached)
                # Keep newest section labels/scores
                merged["sections"] = article["sections"]
                merged["section_labels"] = article["section_labels"]
                merged["relevance_score"] = article["relevance_score"]
                return merged
        except Exception:
            pass

    jina_url = f"https://r.jina.ai/{article['link']}"
    enriched_data = {
        "fly_title": "",
        "dateline": "",
        "lead_excerpt": "",
        "topics": [],
        "image_url": "",
        "enriched_attempted": True,
        "enriched_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
    }

    try:
        raw_bytes = http_get_with_retry(jina_url, retries=3, timeout=18, backoff_base=1.8)
        md_text = raw_bytes.decode("utf-8", errors="ignore")
        parsed = parse_jina_markdown(md_text, fallback_title=article.get("title", ""))
        enriched_data.update(parsed)
        log_fn(
            f"[Enrich] ✓ {article['title'][:48]}... "
            f"(excerpt={len(enriched_data['lead_excerpt'])} chars)"
        )
    except Exception as e:
        log_fn(f"[Enrich] ⚠ Fallback to RSS metadata for '{article['title'][:42]}': {e}")

    merged = dict(article)
    merged.update(enriched_data)
    try:
        cache_file.write_text(json.dumps(merged, ensure_ascii=False, indent=2), encoding="utf-8")
    except Exception:
        pass
    return merged


def fetch_world_in_brief(log_fn: Callable[[str], None] = print) -> str:
    """Fetch The Economist's daily 'The World in Brief' headline summary via r.jina.ai."""
    cache_file = ARTICLE_CACHE_DIR / f"world_in_brief_{datetime.now().strftime('%Y-%m-%d')}.txt"
    if cache_file.exists():
        try:
            txt = cache_file.read_text(encoding="utf-8")
            if len(txt) > 100:
                return txt
        except Exception:
            pass

    try:
        raw = http_get_with_retry(
            "https://r.jina.ai/https://www.economist.com/the-world-in-brief",
            retries=3,
            timeout=18,
        ).decode("utf-8", errors="ignore")
        idx = raw.find("The world in brief")
        if idx != -1:
            end_idx = raw.find("Already have an account?", idx)
            snippet = raw[idx : end_idx if end_idx != -1 else idx + 3500]
            clean = re.sub(r"!\[Image[^\]]*\]\([^\)]+\)", "", snippet)
            clean = re.sub(r"\[([^\]]+)\]\([^\)]+\)", r"\1", clean)
            clean = re.sub(r"\n{3,}", "\n\n", clean).strip()
            if len(clean) > 80:
                cache_file.write_text(clean, encoding="utf-8")
                log_fn(f"[WorldInBrief] ✓ Captured ({len(clean)} chars)")
                return clean
    except Exception as e:
        log_fn(f"[WorldInBrief] ⚠ Skipped: {e}")
    return ""
