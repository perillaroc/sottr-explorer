# -*- coding: utf-8 -*-
"""SX-022 验收：图鉴（区域 × 子类别）↔ 存档区域计数器逐行对账。

对账两列：
  - 总数：图鉴里「该区域该类别、非 DLC」的条目数  vs  存档计数器的来源目标数
  - 已收集：图鉴条目 ID 落在存档观察集合内的数量  vs  存档计数器的读数

43 行全部吻合才算通过（SX-022 实测：43/43）。这条同时验证区域绑定、故事子类别
推导（relic/document/mural）、DLC 排除、非故事类别映射与「观察集合 = 已收集」。

用法：
  py -3 research/s022/scripts/validate_region_counters.py <catalog.json> <analysis.json>
"""
from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

#: 图鉴类别 -> 存档计数器类别（仅比对两边都存在的类别）
CATALOG_TO_COUNTER = {
    "relics": "relics",
    "documents": "documents",
    "murals": "murals",
    "survival_caches": "survival_caches",
    "monoliths": "monoliths",
    "crypt_treasures": "crypts",
    "strongboxes": "strongboxes",
}


def observed_ids(analysis_path: Path, input_ref: str | None) -> set[int]:
    data = json.loads(analysis_path.read_text(encoding="utf-8"))
    results = data["results"] if "results" in data else [data]
    reports = [(entry.get("input_ref"), entry["report"]) for entry in results
               if entry.get("report")]
    if input_ref:
        reports = [(name, report) for name, report in reports if name == input_ref]
    if not reports:
        raise SystemExit(f"no report for input_ref={input_ref!r}")
    _name, report = reports[0]
    return {int(r["internal_id"], 16)
            for r in report["observations"]["inventory"]["records"]}, report


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    catalog = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    input_ref = sys.argv[3] if len(sys.argv) > 3 else None
    observed, report = observed_ids(Path(sys.argv[2]), input_ref)

    totals: dict[tuple[str, str], int] = Counter()
    collected: dict[tuple[str, str], int] = Counter()
    for key, item in catalog["items"].items():
        if item.get("is_dlc"):
            continue
        slug = item.get("region_slug")
        if not slug or slug.startswith("_"):
            continue
        if item["kind"] == "story":
            dimension = item.get("subcategory")
        else:
            dimension = item["category"]
        if dimension not in CATALOG_TO_COUNTER:
            continue
        totals[(slug, dimension)] += 1
        if int(key, 16) in observed:
            collected[(slug, dimension)] += 1

    counters = {(c["region_id"], c["category_id"]): c
                for c in report["observations"]["region_counters"]}

    print(f"{'region':<22}{'category':<18}{'atlas':>7}{'target':>8}{'got':>6}{'save':>6}  result")
    ok = mismatch = skipped = 0
    for (slug, dimension), total in sorted(totals.items()):
        counter_id = CATALOG_TO_COUNTER[dimension]
        row = counters.get((slug, counter_id))
        if row is None:
            skipped += 1
            continue
        got, target, save = collected[(slug, dimension)], row["source_target"], row["value"]
        good = total == target and got == save
        ok, mismatch = (ok + 1, mismatch) if good else (ok, mismatch + 1)
        print(f"{slug:<22}{dimension:<18}{total:>7}{target:>8}{got:>6}{save:>6}  "
              f"{'OK' if good else 'MISMATCH'}")
    print(f"\nmatched rows: {ok} OK / {mismatch} MISMATCH / {skipped} no counter")
    return 0 if mismatch == 0 and ok > 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
