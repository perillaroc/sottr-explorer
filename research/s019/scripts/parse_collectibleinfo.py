# -*- coding: utf-8 -*-
"""SX-019 step 2: parse the GlobalCollectibleInfo DTP object graph.

Layout per public binary template (tr11globalcollectibleinfo + trobjectcommon):
- GlobalCollectibleInfo: CollectibleStoryCollection inline (28 B), FSkip(4),
  NonStoryCollectibleCollection inline (32 B), ...
- NonStoryCollectibleCollection: Ref/u32 + skip + Ref/u32 + skip
- NonStoryCollectibleGroup (32 B): groupType i32, isDLC u8, dlcHash u32, Ref,
  count u32 (+4 skip)
- NonStoryCollectibleItem (240 B): id, metricName(Ref), type, regionId, unit,
  placementID (PlacementPath 32 B), elevation, coordinates, area marker block,
  map reveal block, revealed object, selectedDescription, doNotShowOnMap,
  mapMarkerIcon(Ref)

Usage:
  py -3 parse_collectibleinfo.py <payload.bin> [out.json]
"""
from __future__ import annotations

import json
import struct
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from sottr_dtp import DtpFile  # noqa: E402

ITEM_SIZE = 240
GROUP_SIZE = 32


def placement(dtp: DtpFile, pos: int) -> dict:
    return {
        "listType": f"{dtp.u32(pos):08X}",
        "zoneId": dtp.i32(pos + 4),
        "id0": f"{dtp.u64(pos + 8):016X}",
        "id1": f"{dtp.u64(pos + 16):016X}",
        "moreIdsRefKind": dtp.ref_kind(pos + 24),
    }


def read_string_ref(dtp: DtpFile, pos: int) -> str | None:
    target = dtp.ref_target(pos)
    if target is None:
        return None
    return dtp.string_at(target)


def parse_groups(dtp: DtpFile, ref_pos: int, count: int) -> list[dict]:
    target = dtp.ref_target(ref_pos)
    groups = []
    if target is None:
        return groups
    for g in range(count):
        base = target + g * GROUP_SIZE
        groups.append({
            "groupType": dtp.i32(base),
            "isDLC": dtp.u8(base + 4),
            "dlcHash": f"{dtp.u32(base + 8):08X}",
            "collectiblesRefKind": dtp.ref_kind(base + 16),
            "count": dtp.u32(base + 24),
            "ref": base,
        })
    return groups


def parse_items(dtp: DtpFile, ref_pos: int, count: int, group_index: int) -> list[dict]:
    target = dtp.ref_target(ref_pos)
    items = []
    if target is None:
        return items
    for i in range(count):
        b = target + i * ITEM_SIZE
        item = {
            "groupIndex": group_index,
            "structOffset": b,
            "id": f"{dtp.u32(b):08X}",
            "metricName": read_string_ref(dtp, b + 8),
            "type": dtp.i32(b + 16),
            "regionId": f"{dtp.u32(b + 32):08X}",
            "unit": dtp.u32(b + 36),
            "placement": placement(dtp, b + 40),
            "elevation": dtp.i32(b + 72),
            "coordinateX": dtp.f32(b + 76),
            "coordinateY": dtp.f32(b + 80),
            "instanceZ": dtp.f32(b + 84),
            "hasAreaMarker": dtp.u8(b + 88),
            "areaRegionId": f"{dtp.u32(b + 92):08X}",
            "areaUnit": dtp.u32(b + 96),
            "areaPlacement": placement(dtp, b + 100),
            "areaElevation": dtp.i32(b + 132),
            "areaCoordinateX": dtp.f32(b + 136),
            "areaCoordinateY": dtp.f32(b + 140),
            "areaInstanceZ": dtp.f32(b + 144),
            "areaMarkerScale": dtp.f32(b + 148),
            "mapRevealGroupID": f"{dtp.u32(b + 152):08X}",
            "mapRevealGroupIDsRefKind": dtp.ref_kind(b + 160),
            "mapRevealGroupIDsCount": dtp.u32(b + 168),
            "mapRevealAsKnown": dtp.u8(b + 172),
            "revealedObjectUnit": dtp.u32(b + 176),
            "revealedObjectPlacement": placement(dtp, b + 180),
            "selectedDescription": f"{dtp.u64(b + 216):016X}",
            "doNotShowOnMap": dtp.u8(b + 224),
            "mapMarkerIcon": read_string_ref(dtp, b + 232),
        }
        items.append(item)
    return items


def main() -> int:
    payload = Path(sys.argv[1]).read_bytes()
    out_path = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(sys.argv[1]).with_suffix(".json")
    dtp = DtpFile(payload)
    root = dtp.ref_target(dtp.body_pos)
    print(f"body_pos={dtp.body_pos} root_target={root} "
          f"root_kind={dtp.ref_kind(dtp.body_pos)}")
    if root is None:
        print("root ref is not internal; aborting")
        return 2

    story_single_ref = root
    story_single_count = dtp.u32(root + 8)
    story_multi_ref = root + 16
    story_multi_count = dtp.u32(root + 24)
    print(f"story single={story_single_count} multi={story_multi_count} "
          f"(ref kinds {dtp.ref_kind(story_single_ref)}/{dtp.ref_kind(story_multi_ref)})")

    ns = root + 32
    ns_single_ref, ns_single_count = ns, dtp.u32(ns + 8)
    ns_multi_ref, ns_multi_count = ns + 16, dtp.u32(ns + 24)
    print(f"nonstory single={ns_single_count} multi={ns_multi_count} "
          f"(ref kinds {dtp.ref_kind(ns_single_ref)}/{dtp.ref_kind(ns_multi_ref)})")

    groups = parse_groups(dtp, ns_single_ref, ns_single_count)
    items: list[dict] = []
    for g, group in enumerate(groups):
        group_items = parse_items(dtp, group["ref"] + 16, group["count"], g)
        items.extend(group_items)
    print(f"groups={len(groups)} items={len(items)}")

    type_counter = Counter(item["type"] for item in items)
    region_counter = Counter(item["regionId"] for item in items)
    unit_counter = Counter(item["unit"] for item in items)
    print("type distribution:", dict(sorted(type_counter.items())))
    print("region distribution:", dict(sorted(region_counter.items())))
    print("unit distribution:", dict(sorted(unit_counter.items())))

    report = {
        "story": {"singleCount": story_single_count, "multiCount": story_multi_count},
        "nonStory": {
            "singleCount": ns_single_count,
            "multiCount": ns_multi_count,
            "groups": groups,
            "items": items,
        },
    }
    out_path.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"report -> {out_path}")

    for item in items[:8]:
        print(json.dumps(item, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
