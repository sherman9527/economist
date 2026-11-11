# -*- coding: utf-8 -*-
"""
Global configuration for The Economist Daily Market Impact Analysis System.
Strictly self-contained under D:\\code\\economist (Zero C: drive usage).
"""

import os
import sys
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

BASE_DIR = Path(r"D:\code\economist")
SCRIPTS_DIR = BASE_DIR / "scripts"
DATA_DIR = BASE_DIR / "data"
ARTICLE_CACHE_DIR = DATA_DIR / "article_cache"
RUNS_DIR = DATA_DIR / "runs"
CHECKPOINTS_DIR = BASE_DIR / "checkpoints"
REPORTS_DIR = BASE_DIR / "reports"
LOGS_DIR = BASE_DIR / "logs"
COPILOT_LOGS_DIR = LOGS_DIR / "copilot"
TEMP_DIR = BASE_DIR / ".temp"

ACK_STATE_FILE = DATA_DIR / "ack_state.json"

# Enforce zero C: drive temp usage for all child processes
for d in [
    SCRIPTS_DIR,
    DATA_DIR,
    ARTICLE_CACHE_DIR,
    RUNS_DIR,
    CHECKPOINTS_DIR,
    REPORTS_DIR,
    LOGS_DIR,
    COPILOT_LOGS_DIR,
    TEMP_DIR,
]:
    d.mkdir(parents=True, exist_ok=True)

os.environ["TEMP"] = str(TEMP_DIR)
os.environ["TMP"] = str(TEMP_DIR)
os.environ["TMPDIR"] = str(TEMP_DIR)
os.environ["PYTHONIOENCODING"] = "utf-8"
# Clear conflicting Python 3.11 env vars if present
os.environ.pop("PYTHONHOME", None)
os.environ.pop("PYTHONPATH", None)

ACK_SERVER_HOST = "127.0.0.1"
ACK_SERVER_PORT = int(os.environ.get("ECONOMIST_PORT", "4288"))

ECONOMIST_RSS_SECTIONS = [
    ("finance-and-economics", "财经与金融 (Finance & Economics)", 10),
    ("business", "商业与公司 (Business)", 9),
    ("leaders", "社论与纲领 (Leaders)", 9),
    ("briefing", "深度专题 (Briefing)", 9),
    ("china", "中国专栏 (China)", 10),
    ("united-states", "美国政经 (United States)", 8),
    ("science-and-technology", "科技与AI (Science & Technology)", 9),
    ("international", "国际关系 (International)", 7),
    ("asia", "亚太政经 (Asia)", 7),
    ("europe", "欧洲政经 (Europe)", 6),
    ("britain", "英国政经 (Britain)", 5),
    ("the-world-this-week", "本周全球速览 (The World This Week)", 7),
]

COPILOT_MODEL = "gemini-4-argon"
COPILOT_CONTEXT_TIER = "long_context"  # 983k context window
COPILOT_REASONING_EFFORT = "high"
