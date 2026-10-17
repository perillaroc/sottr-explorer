# -*- coding: utf-8 -*-
"""Build the full-collection catalog (SX-021, stage A).

Inputs (outside the repo, produced by research/s019 + research/s020 scripts):
  <work>/extracted/dtp_0360E8.bin            globalcollectibleinfo DTP (update3)
  <work>/extracted/gameflow_dtp_0360F4.bin   globalgameflow DTP (update3)
Repo inputs:
  research/python-v0.2/data/collectible_id_candidates.json   (MIT name dictionary)
  schemas/catalog.schema.json                (output contract)

Output:
  data/catalog/collectibles_catalog.json

Usage:
  py -3 data/catalog/build_catalog.py <work_dir> [--check]
"""
from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DICT_PATH = REPO / "research" / "python-v0.2" / "data" / "collectible_id_candidates.json"
OUT_PATH = Path(__file__).resolve().parent / "collectibles_catalog.json"
sys.path.insert(0, str(REPO / "research" / "s019" / "scripts"))
from sottr_dtp import DtpFile  # noqa: E402

# 非故事条目 type -> 类别（依据 SX-019/SX-020 提取与外部 100% 口径）
NONSTORY_CATEGORY = {
    2: "survival_caches",      # 65 SurvivalCache + 12 RareResource = 77 必收；另 4 gym
    4: "monoliths",
    5: "explorer_backpacks",
    6: "crypt_treasures",
    7: "archivist_maps",
    8: "strongboxes",
    9: "challenge_tombs",
    11: "crypt_entrances",
    1: "test_items",
}
CATEGORY_REQUIRED = {
    "artifacts": True,
    "monoliths": True,
    "survival_caches": True,
    "crypt_treasures": True,
    "challenge_tombs": None,   # 未闭合：12 名/9 对象/9 唯一 ID（5 个共用占位 ID），不进入缺失结论
    "explorer_backpacks": False,
    "archivist_maps": False,
    "strongboxes": False,
    "crypt_entrances": False,
    "test_items": False,
    "gym_training": False,
    "croft_manor_extra": False,
}


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    work = Path(sys.argv[1])
    check = "--check" in sys.argv

    dictionary = json.loads(DICT_PATH.read_text(encoding="utf-8"))["mapping"]
    names = {k.upper(): (v[0] if isinstance(v, list) and v else None) for k, v in dictionary.items()}

    # ---------------- collectibleinfo ----------------
    dtp = DtpFile((work / "extracted" / "dtp_0360E8.bin").read_bytes())

    def s(ref_pos):
        t = dtp.ref_target(ref_pos)
        return dtp.string_at(t) if t is not None else None

    root = dtp.ref_target(dtp.body_pos)
    story_target = dtp.ref_target(root)
    story_count = dtp.u32(root + 8)
    ns_ref = root + 32
    ns_count = dtp.u32(ns_ref + 8)

    items: dict[str, dict] = {}
    collections = []

    for k in range(story_count):
        b = story_target + k * 168
        group = s(b + 16)
        is_dlc = bool(dtp.u8(b + 4))
        tgt = dtp.ref_target(b + 152)
        count = dtp.u32(b + 160)
        required = (not is_dlc) and group != "Prologue"
        for j in range(count):
            it = tgt + j * 496
            iid = f"0x{dtp.u32(it):08X}"
            items[iid] = {
                "kind": "story",
                "name": names.get(iid.upper()) or s(it + 8),
                "collection": group,
                "is_dlc": is_dlc,
                "required_for_100": required,
                "region_hash": f"{dtp.u32(it + 32):08X}",
                "category": "croft_manor_extra" if group == "Prologue" else "artifacts",
            }
        collections.append({"name": group, "is_dlc": is_dlc, "count": count,
                            "required_for_100": required})

    ns_target = dtp.ref_target(ns_ref)
    shared_ids: dict[str, list[str]] = {}
    for g in range(ns_count):
        gb = ns_target + g * 32
        g_dlc = bool(dtp.u8(gb + 4))
        ref = gb + 16
        tgt = dtp.ref_target(ref)
        count = dtp.u32(gb + 24)
        for j in range(count):
            it = tgt + j * 240
            iid = f"0x{dtp.u32(it):08X}"
            metric = s(it + 8)
            if iid in items:
                if items[iid]["kind"] == "nonstory" and items[iid]["name"] != metric:
                    shared_ids.setdefault(iid, [items[iid]["name"]])
                    shared_ids[iid].append(metric)
                continue
            type_id = dtp.i32(it + 16)
            category = NONSTORY_CATEGORY.get(type_id, f"unknown_type_{type_id}")
            if "gym" in (metric or "").lower():
                category = "gym_training"
            base_required = CATEGORY_REQUIRED.get(category, False)
            if base_required is None:
                required = None
            else:
                required = base_required and not g_dlc
            items[iid] = {
                "kind": "nonstory",
                "name": names.get(iid.upper()) or metric,
                "type": type_id,
                "marker": s(it + 232),
                "is_dlc": g_dlc,
                "required_for_100": required,
                "region_hash": f"{dtp.u32(it + 32):08X}",
                "unit": dtp.u32(it + 36),
                "category": category,
            }
    for iid, names_list in shared_ids.items():
        items[iid]["shared_id"] = True
        items[iid]["shared_names"] = names_list

    # ---------------- gameflow regions ----------------
    gameflow = DtpFile((work / "extracted" / "gameflow_dtp_0360F4.bin").read_bytes())
    groot = gameflow.ref_target(gameflow.body_pos)
    num_regions = gameflow.u32(groot + 64)
    region_target = gameflow.ref_target(groot + 72)

    def list_at(ref_pos, count_pos, stride=4):
        tgt = gameflow.ref_target(ref_pos)
        cnt = gameflow.u32(count_pos)
        if tgt is None or cnt == 0 or cnt > 4096:
            return []
        return [gameflow.u32(tgt + stride * i) for i in range(cnt)]

    regions = []
    for i in range(num_regions):
        b = region_target + i * 256
        regions.append({
            "region_hash": f"{gameflow.u32(b):08X}",
            "is_dlc": bool(gameflow.u8(b + 52)),
            "counts_toward_percent": bool(gameflow.u8(b + 41)),
            "challenge_tombs": list_at(b + 176, b + 184),
            "campsites": list_at(b + 192, b + 200),
            "challenges": list_at(b + 208, b + 216, stride=8),
            "missions": list_at(b + 224, b + 232, stride=8),
        })

    # ---------------- totals / categories ----------------
    by_cat = Counter(v["category"] for v in items.values())
    required_by_cat = Counter(v["category"] for v in items.values() if v["required_for_100"])
    categories = []
    cat_names = {
        "artifacts": "Artifacts（故事收藏）",
        "croft_manor_extra": "克劳馥庄园（Prologue，不要求）",
        "survival_caches": "Survival Caches（生存贮藏点）",
        "monoliths": "Monoliths（石碑）",
        "crypt_treasures": "Crypt Treasures（墓穴宝藏）",
        "challenge_tombs": "Challenge Tombs（挑战古墓）",
        "explorer_backpacks": "Explorer Backpacks（探险者背包，可选）",
        "archivist_maps": "Archivist Maps（档案图，可选）",
        "strongboxes": "Strongboxes（保险箱，可选）",
        "crypt_entrances": "Crypt Entrances（墓穴入口，可选）",
        "gym_training": "训练场（gym，不要求）",
        "test_items": "测试项（不要求）",
    }
    for cat, count in sorted(by_cat.items()):
        required = CATEGORY_REQUIRED.get(cat, False)
        entry = {
            "id": cat,
            "name": cat_names.get(cat, cat),
            "count": count,
            "required_for_100": required,
            "required_count": required_by_cat.get(cat, 0),
        }
        if required is None:
            entry["note"] = "未闭合（12 名/9 对象/9 唯一 ID，5 个共用占位 ID）；按 T22 不进入缺失结论"
        categories.append(entry)

    catalog = {
        "catalog_schema_version": "1.0.0",
        "catalog_version": "1.0.0",
        "generated_from": {
            "game": {"product": "Shadow of the Tomb Raider", "steam_appid": 750920,
                     "data": "bigfile.update3.002（最高优先级）"},
            "extraction": {
                "collectibleinfo_dtp_sha256_16": hashlib.sha256(
                    (work / "extracted" / "dtp_0360E8.bin").read_bytes()).hexdigest()[:16],
                "gameflow_dtp_sha256_16": hashlib.sha256(
                    (work / "extracted" / "gameflow_dtp_0360F4.bin").read_bytes()).hexdigest()[:16],
            },
            "name_source": "TrRebootModTools tr11collectibles.bt（MIT，见 THIRD_PARTY_LICENSES.md）",
            "date": "2026-10-17",
            "license_decision": "用户已确认商业游戏数据可随交付版发布（2026-10-17）",
        },
        "totals": {
            "items": len(items),
            "story": sum(1 for v in items.values() if v["kind"] == "story"),
            "nonstory": sum(1 for v in items.values() if v["kind"] == "nonstory"),
            "required_for_100": sum(1 for v in items.values() if v["required_for_100"]),
            "dlc": sum(1 for v in items.values() if v["is_dlc"]),
        },
        "categories": categories,
        "collections": collections,
        "regions": regions,
        "items": items,
    }

    # self-validation
    story_required = sum(1 for v in items.values()
                         if v["kind"] == "story" and v["required_for_100"])
    required_total = sum(1 for v in items.values() if v["required_for_100"])
    required_ns = Counter(v["category"] for v in items.values()
                          if v["kind"] == "nonstory" and v["required_for_100"])
    checks = {
        "story_total_302": catalog["totals"]["story"] == 302,
        "nonstory_total_168": catalog["totals"]["nonstory"] == 168,
        "story_required_253": story_required == 253,
    }
    print("checks:", checks)
    print(f"required total={required_total} story={story_required} nonstory={dict(required_ns)}")
    if not all(checks.values()):
        print("SELF-CHECK FAILED; refusing to write")
        return 1

    if check:
        current = json.loads(OUT_PATH.read_text(encoding="utf-8"))
        same = json.dumps(current, sort_keys=True) == json.dumps(catalog, sort_keys=True)
        print("--check:", "identical" if same else "DIFFERS")
        return 0 if same else 1

    OUT_PATH.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"written {OUT_PATH} ({OUT_PATH.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
