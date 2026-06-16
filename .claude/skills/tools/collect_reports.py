#!/usr/bin/env python3
"""Collect and rank open machine self-diagnostic reports for /resolve.

Scans /.machine/reports/*.md (excluding README.md), parses each report's YAML-ish
frontmatter, and prints the backlog ranked highest-priority first so the /resolve
skill can pick the top one. Read-only: it never edits or deletes anything.

Ranking: severity (blocker > major > minor > annoyance) then oldest date, then
filename. Reports with status other than "open"/"" are excluded.

Output: JSON to stdout: {"count": N, "reports": [ {file, path, title, severity,
date, area, status, rank}, ... ]}. Exit 0 always (empty backlog -> count 0).
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

SEVERITY_ORDER = {"blocker": 0, "major": 1, "minor": 2, "annoyance": 3}
UNRANKED_SEVERITY = 99


def find_reports_dir() -> Path:
    """Locate /.machine/reports/ from env or by walking up from this script."""
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env:
        cand = Path(env) / ".machine" / "reports"
        if cand.is_dir():
            return cand
    here = Path(__file__).resolve()
    for parent in here.parents:
        cand = parent / ".machine" / "reports"
        if cand.is_dir():
            return cand
    # Fallback: assume repo root is three levels above .claude/skills/tools/
    return here.parents[3] / ".machine" / "reports"


def parse_frontmatter(text: str) -> dict[str, str]:
    """Parse the leading bullet/key frontmatter from a report body.

    Reports use a markdown bullet style: `- **severity:** major`. Also tolerates
    a `--- ... ---` YAML block. Returns lowercased keys.
    """
    fields: dict[str, str] = {}
    lines = text.splitlines()

    # Optional YAML block.
    if lines and lines[0].strip() == "---":
        for line in lines[1:]:
            if line.strip() == "---":
                break
            if ":" in line:
                k, _, v = line.partition(":")
                fields[k.strip().lower()] = v.strip()

    # Bullet style: - **key:** value
    for line in lines:
        s = line.strip()
        if not s.startswith("-"):
            continue
        s = s.lstrip("- ").replace("**", "")
        if ":" in s:
            k, _, v = s.partition(":")
            k = k.strip().lower()
            if k in {"date", "severity", "area", "status"} and k not in fields:
                fields[k] = v.strip()
    return fields


def title_of(text: str, fallback: str) -> str:
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("#"):
            return s.lstrip("# ").strip()
    return fallback


def main() -> int:
    reports_dir = find_reports_dir()
    items = []
    if reports_dir.is_dir():
        for path in sorted(reports_dir.glob("*.md")):
            if path.name.lower() == "readme.md":
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            fm = parse_frontmatter(text)
            status = fm.get("status", "open").lower()
            if status not in ("", "open"):
                continue
            severity = fm.get("severity", "").lower()
            items.append(
                {
                    "file": path.name,
                    "path": str(path),
                    "title": title_of(text, path.stem),
                    "severity": severity or "unspecified",
                    "date": fm.get("date", ""),
                    "area": fm.get("area", ""),
                    "status": status or "open",
                    "_sev": SEVERITY_ORDER.get(severity, UNRANKED_SEVERITY),
                }
            )

    items.sort(key=lambda r: (r["_sev"], r["date"] or "9999", r["file"]))
    for i, r in enumerate(items, 1):
        r["rank"] = i
        del r["_sev"]

    json.dump({"count": len(items), "reports": items}, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
