# -*- coding: utf-8 -*-
"""SX-020 reconciliation: full-collection scope against game data and sources.

Covers:
- story groups: base vs DLC classification (isDLC/dlcHash), counts vs external 259/253
- region composition: challenges/missions entries with region + unit names
- region -> unit map consistency for non-story items (item.unit vs MapData)
- overall completion ratio test: save counters sum / source totals vs 0x28 value

Usage:
  py -3 reconcile_full_collection.py <work_dir> <out_dir>
"""
from __future__ import annotations

import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

here = Path(__file__).parent
sys.path.insert(0, str(here))
from sottr_dtp import DtpFile  # noqa: E402

ZONES_BT = here.parent / "s019_tools" / "TrRebootModTools" / "Templates" / "010 Editor" / "tr" / "tr11" / "lists" / "tr11zones.bt"


def load_zone_names() -> dict[int, str]:
    names = {}
    if not ZONES_BT.exists():
        return names
    pattern = re.compile(r'case (\d+): return "([^"]+)"')
    for match in pattern.finditer(ZONES_BT.read_text(encoding="utf-8", errors="replace")):
        names[int(match.group(1))] = match.group(2)
    return names


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    work_dir = Path(sys.argv[1])
    out_dir = Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    report: dict = {}

    # ---------- collectibleinfo: story groups + items ----------
    dtp = DtpFile((work_dir / "extracted" / "dtp_0360E8.bin").read_bytes())

    def s(ref_pos):
        t = dtp.ref_target(ref_pos)
        return dtp.string_at(t) if t is not None else None

    root = dtp.ref_target(dtp.body_pos)
    story_target = dtp.ref_target(root)
    story_count = dtp.u32(root + 8)
    groups = []
    story_items = []
    for k in range(story_count):
        b = story_target + k * 168
        group = {
            "name": s(b + 16),
            "isDLC": dtp.u8(b + 4),
            "dlcHash": f"{dtp.u32(b + 8):08X}",
            "count": dtp.u32(b + 160),
            "items": [],
        }
        tgt = dtp.ref_target(b + 152)
        for j in range(group["count"]):
            it = tgt + j * 496
            item = {
                "id": f"{dtp.u32(it):08X}",
                "metricName": s(it + 8),
                "nameText": s(it + 272),
                "regionId": f"{dtp.u32(it + 32):08X}",
                "unit": dtp.u32(it + 36),
                "incrementsRegionCount": dtp.u8(it + 473),
                "foundByDefault": dtp.u8(it + 474),
            }
            group["items"].append(item)
            story_items.append(item)
        groups.append(group)
    base_groups = [g for g in groups if not g["isDLC"]]
    dlc_groups = [g for g in groups if g["isDLC"]]
    base_items = sum(g["count"] for g in base_groups)
    dlc_items = sum(g["count"] for g in dlc_groups)
    print(f"story groups={len(groups)} base={len(base_groups)} dlc={len(dlc_groups)}")
    print(f"story items base={base_items} dlc={dlc_items} total={len(story_items)}")
    print("base groups:", [(g["name"], g["count"]) for g in base_groups])
    print("dlc groups:", [(g["name"], g["count"], g["dlcHash"]) for g in dlc_groups])
    report["story"] = {
        "groups": len(groups), "baseGroups": len(base_groups), "dlcGroups": len(dlc_groups),
        "baseItems": base_items, "dlcItems": dlc_items,
        "groupList": [{"name": g["name"], "isDLC": g["isDLC"], "count": g["count"]} for g in groups],
        "extraFlagCounts": {
            "incrementsRegionCount1": sum(1 for i in story_items if i["incrementsRegionCount"] == 1),
            "foundByDefault1": sum(1 for i in story_items if i["foundByDefault"] == 1),
        },
    }

    # ---------- gameflow: regions + unit maps ----------
    gameflow = DtpFile((work_dir / "extracted" / "gameflow_dtp_0360F4.bin").read_bytes())
    groot = gameflow.ref_target(gameflow.body_pos)
    num_regions = gameflow.u32(groot + 64)
    region_target = gameflow.ref_target(groot + 72)
    num_maps = gameflow.u32(groot + 16)
    map_target = gameflow.ref_target(groot + 24)

    def u32_list(node, ref_pos, count_pos, stride=4):
        tgt = node.ref_target(ref_pos)
        cnt = node.u32(count_pos)
        if tgt is None or cnt == 0 or cnt > 4096:
            return []
        vals = []
        for i in range(cnt):
            if stride == 4:
                vals.append(node.u32(tgt + 4 * i))
            else:
                vals.append((node.u32(tgt + 8 * i), node.u32(tgt + 8 * i + 4)))
        return vals

    zones = load_zone_names()
    maps = []
    for i in range(num_maps):
        b = map_target + i * 272
        maps.append({
            "hashedString": f"{gameflow.u32(b):08X}",
            "uiUnitID": gameflow.i32(b + 4),
            "markerUnit": gameflow.u16(b + 12),
            "primaryRegion": f"{gameflow.u32(b + 232):08X}",
            "numSecondary": gameflow.u32(b + 236),
        })
    region_units: dict[str, set[int]] = defaultdict(set)
    for m in maps:
        region_units[m["primaryRegion"]].add(m["uiUnitID"])
        region_units[m["primaryRegion"]].add(m["markerUnit"])

    regions = []
    for i in range(num_regions):
        b = region_target + i * 256
        rid = f"{gameflow.u32(b):08X}"
        units = sorted(u for u in region_units.get(rid, set()) if u)
        regions.append({
            "regionId": rid,
            "isDLC": gameflow.u8(b + 52),
            "countsTowardPercentComplete": gameflow.u8(b + 41),
            "challengeTombs": [f"{v:08X}" for v in u32_list(gameflow, b + 176, b + 184)],
            "campsites": [f"{v:08X}" for v in u32_list(gameflow, b + 192, b + 200)],
            "challenges": u32_list(gameflow, b + 208, b + 216, stride=8),
            "missions": u32_list(gameflow, b + 224, b + 232, stride=8),
            "units": units,
            "unitNames": [zones.get(u, f"unit{u}") for u in units],
        })
    tot = {k: sum(len(r[k]) for r in regions) for k in ("challengeTombs", "campsites", "challenges", "missions")}
    print(f"regions={len(regions)} totals={tot}")
    for r in regions:
        if r["challenges"] or r["missions"]:
            print(f"  {r['regionId']} units={r['unitNames'][:2]} "
                  f"challenges={r['challenges']} missions={r['missions']}")
    report["regions"] = regions
    report["regionTotals"] = tot

    # ---------- non-story items vs unit maps ----------
    mt = json.loads((work_dir / "extracted" / "mapping_candidates.json").read_text(encoding="utf-8"))
    unit_by_map = {}
    for m in maps:
        unit_by_map.setdefault(m["uiUnitID"], []).append(m)
        unit_by_map.setdefault(m["markerUnit"], []).append(m)
    ok = mismatch = nounit = 0
    for item in mt:
        unit = item["unit"]
        candidates = unit_by_map.get(unit, [])
        if not candidates:
            nounit += 1
            continue
        if any(m["primaryRegion"] == item["regionId"] for m in candidates):
            ok += 1
        else:
            mismatch += 1
    print(f"non-story item region check: ok={ok} mismatch={mismatch} no-unit-map={nounit}")
    report["regionCheck"] = {"ok": ok, "mismatch": mismatch, "noUnitMap": nounit}

    # ---------- overall completion ratio test ----------
    analysis = json.loads((work_dir / "cli_analyze" / "analysis.json").read_text(encoding="utf-8"))
    meta = json.loads((Path(__file__).parents[2] / "repo" / "sottr-explorer" / "data" / "metadata" / "region_counters.json").read_text(encoding="utf-8"))
    targets = {v["counter_key"]: v["source_total"] for v in meta["region_counters"].values()}
    for entry in analysis["results"]:
        if entry.get("input_ref") != "save1.dat" or not entry.get("report"):
            continue
        rep = entry["report"]
        counters = rep["observations"]["region_counters"]
        value_sum = sum(c["value"] for c in counters if c.get("value") is not None)
        target_sum = sum(c["source_target"] for c in counters if c.get("source_target") is not None)
        overall = rep["observations"]["overall_completion"]["value"]
        print(f"save1 counters: value_sum={value_sum} target_sum={target_sum} "
              f"ratio={value_sum / target_sum:.6f} overall={overall:.6f}")
        report["overallTest"] = {"valueSum": value_sum, "targetSum": target_sum,
                                 "ratio": value_sum / target_sum, "overall": overall,
                                 "counters": len(counters)}
        print("counter pairs:", [(c["region_id"], c["category_id"], c["value"], c.get("source_target")) for c in counters][:15])

    out = out_dir / "s020_reconciliation.json"
    out.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"-> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
