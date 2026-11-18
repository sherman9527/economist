"""Repository Hygiene & Redline Gate Test (Requirement #2 & #14).

Asserts across the entire repository:
  1. ZERO hardcoded drive-letter absolute paths in source code, scripts,
     configs, documentation, and JSON state files.
  2. EVERY single source, script, style, and config file is strictly < 1000 lines.
"""

from __future__ import annotations

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parent.parent

IGNORED_DIRS = {
    ".git",
    ".temp",
    ".toolchain",
    ".build",
    "node_modules",
    "dist",
    "target",
    "gen",
    "portable-release",
    "webview_data",
    "logs",
    "telemetry",
    "__pycache__",
    "article_cache",
    "market_cache",
    "checkpoints",
}

LINE_LIMIT_EXTS = {
    ".rs",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".css",
    ".py",
    ".ps1",
    ".toml",
}

TEXT_SCAN_EXTS = LINE_LIMIT_EXTS | {".md", ".json", ".html"}

# Third-party vendored minified libraries exempt from line count check
VENDORED_FILENAMES = {"echarts.min.js"}

# Matches Windows drive-letter absolute paths while ignoring URL schemes like https://
ABS_PATH_RE = re.compile(r"(?<![A-Za-z0-9])[A-Za-z]:(?:\\\\|\\|//|/)[A-Za-z0-9_.\-]")


def iter_repo_files():
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = path.relative_to(ROOT).parts
        if any(p in IGNORED_DIRS for p in rel_parts):
            continue
        yield path


def run_hygiene_audit() -> tuple[list[str], list[str]]:
    line_violations: list[str] = []
    abs_path_violations: list[str] = []

    for path in iter_repo_files():
        rel = path.relative_to(ROOT).as_posix()
        suf = path.suffix.lower()
        if path.name in VENDORED_FILENAMES:
            continue

        if suf in TEXT_SCAN_EXTS:
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue

            lines = text.splitlines()
            if suf in LINE_LIMIT_EXTS and len(lines) >= 1000:
                line_violations.append(f"{rel}: {len(lines)} lines (>= 1000 redline)")

            # Skip requirement.txt itself only if user wrote paths there, scan everything else
            if rel == "requirement.txt":
                continue

            for idx, line in enumerate(lines, start=1):
                m = ABS_PATH_RE.search(line)
                if m:
                    snippet = line.strip()[:120]
                    abs_path_violations.append(f"{rel}:{idx} -> '{m.group(0)}' in `{snippet}`")

    return line_violations, abs_path_violations


if __name__ == "__main__":
    line_v, path_v = run_hygiene_audit()
    if line_v or path_v:
        print("=== REPOSITORY REDLINE VIOLATIONS DETECTED ===")
        for v in line_v:
            print(f"[LINE >= 1000] {v}")
        for v in path_v[:40]:
            print(f"[ABS PATH]     {v}")
        if len(path_v) > 40:
            print(f"... and {len(path_v) - 40} more absolute path matches.")
        sys.exit(1)

    print("[PASS] Repository hygiene gate passed: 0 absolute paths, all files < 1000 lines.")
