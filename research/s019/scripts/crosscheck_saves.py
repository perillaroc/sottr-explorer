# -*- coding: utf-8 -*-
"""SX-019: cross-check CLI observations (analysis.json) against the game-DTP
non-story collectible set; print coverage facts.

Usage:
  py -3 crosscheck_saves.py <analysis.json> <collectibleinfo.json>
"""
from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path


def main() -> int:
    analysis = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    items = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))["nonStory"]["items"]

    by_file = {}
    for entry in analysis["results"]:
        report = entry.get("report")
        if not report:
            continue
        records = report["observations"]["inventory"]["records"]
        by_file[entry["input_ref"]] = {int(r["internal_id"], 16) for r in records}

    dtp_by_id = {}
    dups = Counter()
    for it in items:
        rid = int(it["id"], 16)
        if rid == 0:
            continue
        if rid in dtp_by_id:
            dups[rid] += 1
        dtp_by_id.setdefault(rid, it)
    dtp_ids = set(dtp_by_id)
    union = set().union(*by_file.values()) if by_file else set()

    print(f"dtp items={len(items)} unique={len(dtp_ids)} duplicate_ids={ {k: v for k, v in dups.items()} }")
    for name, ids in sorted(by_file.items()):
        print(f"{name}: observed={len(ids)} intersection_with_dtp={len(ids & dtp_ids)}")
    print(f"union observed={len(union)} intersection_with_dtp={len(union & dtp_ids)}")
    print(f"dtp ids not observed in any save: {len(dtp_ids - union)}")

    per_type = Counter(dtp_by_id[rid]["type"] for rid in union & dtp_ids)
    totals = Counter(it["type"] for it in dtp_by_id.values())
    for t in sorted(totals):
        print(f"type {t}: observed {per_type.get(t, 0)}/{totals[t]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
