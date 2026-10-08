#!/usr/bin/env python3
"""Build the Control Room page (a single self-contained HTML file) from docs/swarm/control-room/data.json and docs/decisions/QUEUE.md.

Usage: python3 -I tools/control_room/build.py OUT.html
ARCH edits data.json at each check-in (lane states, strip, log, pictures), rebuilds, and republishes the page as an Artifact.
Decision-card answers do not live here: they live in the artifact's database, which the page writes and ARCH reads.
"""
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def parse_cards(text: str):
    """Parse docs/decisions/QUEUE.md: `### C-NNN  Title` followed by labelled lines."""
    cards = []
    parts = re.split(r"^### (C-\d+)\s+(.*)$", text, flags=re.M)
    # parts: [preamble, id, title, body, id, title, body, ...]
    for i in range(1, len(parts), 3):
        cid, title, body = parts[i], parts[i + 1].strip(), parts[i + 2]
        fields = {}
        for line in body.splitlines():
            m = re.match(r"^(Asked by|Options|Recommendation|Cost of being wrong|Default[^:]*|Answer):\s*(.*)$", line)
            if m:
                fields[m.group(1).split(" (")[0]] = m.group(2).strip()
        status_m = re.search(r"Status:\s*([^\n]*)", body)
        status = status_m.group(1).strip() if status_m else "OPEN"
        opts = []
        raw = fields.get("Options", "")
        pieces = re.split(r"\(([a-e])\)\s+", raw)
        for j in range(1, len(pieces), 2):
            opts.append({"key": pieces[j], "text": pieces[j + 1].strip().rstrip(";").rstrip(".").strip()})
        dm = re.match(r"\(?([a-e])\)?", fields.get("Default", "") or "")
        default_key = dm.group(1) if dm else (opts[0]["key"] if opts else "a")
        cards.append(
            {
                "id": cid,
                "title": title,
                "status": status,
                "options": opts,
                "defaultKey": default_key,
                "recommendation": fields.get("Recommendation", ""),
                "cost": fields.get("Cost of being wrong", ""),
            }
        )
    return cards


def main():
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "out" / "control-room.html"
    data = json.loads((ROOT / "docs/swarm/control-room/data.json").read_text(encoding="utf-8"))
    data["cards"] = parse_cards((ROOT / "docs/decisions/QUEUE.md").read_text(encoding="utf-8"))
    if not data.get("updated"):
        data["updated"] = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    template = (ROOT / "tools/control_room/template.html").read_text(encoding="utf-8")
    payload = json.dumps(data, ensure_ascii=False).replace("</", "<\\/")
    html = template.replace("__DATA__", payload)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(html, encoding="utf-8")
    print(f"wrote {out} ({len(html) // 1024} KiB, {len(data['cards'])} cards, {len(data['lanes'])} lanes)")


if __name__ == "__main__":
    main()
