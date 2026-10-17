# -*- coding: utf-8 -*-
"""Build the full-collection catalog / atlas (SX-021 stage A; SX-022 schema 2.0.0).

Inputs (outside the repo, produced by research/s019 + research/s022 scripts):
  <work>/extracted/dtp_0360E8.bin            globalcollectibleinfo DTP (update3)
  <work>/extracted/gameflow_dtp_0360F4.bin   globalgameflow DTP (update3)
Repo inputs:
  research/python-v0.2/data/collectible_id_candidates.json   (MIT name dictionary)
  research/s022/data/{region_bindings,region_names,unit_names,item_names}.json
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
S022_DATA = REPO / "research" / "s022" / "data"
OUT_PATH = Path(__file__).resolve().parent / "collectibles_catalog.json"
sys.path.insert(0, str(REPO / "research" / "s019" / "scripts"))
sys.path.insert(0, str(REPO / "research" / "s022" / "scripts"))
from sottr_atlas import (  # noqa: E402
    NONSTORY_TYPE_CATEGORY, POSITION_SPACE, STORY_TYPE_SUBCATEGORY,
    parse_nonstory_items, parse_story_items,
)
from sottr_dtp import DtpFile  # noqa: E402

# 非故事条目 type -> 类别（依据 SX-019/SX-020 提取与外部 100% 口径）
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

CATEGORY_NAMES = {
    "artifacts": "Artifacts（故事收藏：遗物 / 文献 / 壁画）",
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

#: 子类别显示名（故事条目按游戏数据 type 细分）
SUBCATEGORY_NAMES = {
    "relics": "Relic（遗物）",
    "documents": "Document（文献）",
    "murals": "Mural（壁画）",
}

#: 故事子类别 -> 获取方式规则键（类别 `artifacts` 的默认规则只是回退）
SUBCATEGORY_RULE_KEY = {
    "relics": "relic",
    "documents": "document",
    "murals": "mural",
}

#: 获取方式规则（人工整理，`source: curated`；SX-022 逐类别撰写，未逐条与游戏内核实）
ACQUISITION_RULES = {
    "relic": {
        "text_zh": "在关卡内拾取发光的小物件。学会「求生本能」相关技能后可用求生本能高亮附近收集品；"
                   "购买该区域的档案图（Archivist Map）可在区域地图上标出未收集的遗物。",
        "text_en": "Pick it up in the level. Survival Instinct (once unlocked) highlights nearby "
                   "collectibles; an Archivist Map for the region marks uncollected relics on the map.",
    },
    "document": {
        "text_zh": "在关卡内拾取文献。与遗物相同：可用求生本能高亮，购买区域档案图后会在区域地图上标出。",
        "text_en": "Pick it up in the level; Survival Instinct highlights it and the region's "
                   "Archivist Map marks it on the map.",
    },
    "mural": {
        "text_zh": "在关卡内找到墙面壁画并上前查看。部分壁画需要先照明（火把 / 光源）或解开附近机关；"
                   "可用求生本能高亮。",
        "text_en": "Find the wall mural and examine it. Some need a light source or a nearby puzzle "
                   "solved first; Survival Instinct highlights them.",
    },
    "survival_cache": {
        "text_zh": "多为散落在关卡里的箱子或背包，打开即得资源。可用求生本能高亮；"
                   "购买区域档案图后会在区域地图上标出。",
        "text_en": "A crate or backpack lying in the level; open it for resources. Highlighted by "
                   "Survival Instinct and by the region's Archivist Map.",
    },
    "monolith": {
        "text_zh": "走到石碑前用「解读」能力（升级解锁的玛雅 / 印加语言技能）解读。解读后会在该区域地图上"
                   "标出附近的贮藏点、古墓与挑战。",
        "text_en": "Walk up to the monolith and read it with the language skill (unlocked by "
                   "upgrades); reading it marks nearby caches, tombs and challenges on the region map.",
    },
    "crypt_treasure": {
        "text_zh": "进入墓穴（Crypt）并打开石棺取得。墓穴入口通常在区域地图上有标记。",
        "text_en": "Enter the crypt and open the sarcophagus. Crypt entrances are usually marked on "
                   "the region map.",
    },
    "crypt_entrance": {
        "text_zh": "找到并进入墓穴入口即记录（不必取走内部宝藏）。不参与 100%。",
        "text_en": "Finding and entering the crypt entrance records it. Not required for 100%.",
    },
    "strongbox": {
        "text_zh": "需要用撬锁工具（Lockpick，随剧情解锁）撬开。不参与 100%。",
        "text_en": "Requires the Lockpick (unlocked through the story). Not required for 100%.",
    },
    "explorer_backpack": {
        "text_zh": "拾取倒下的探险者遗物背包，可获得技能点。不参与 100%。",
        "text_en": "Pick up a fallen explorer's backpack for skill points. Not required for 100%.",
    },
    "archivist_map": {
        "text_zh": "拾取后会在该区域地图上标出所有收集品位置。不参与 100%。",
        "text_en": "Picking it up marks all collectibles in the region on the map. Not required for 100%.",
    },
    "challenge_tomb": {
        "text_zh": "完成挑战古墓内的解谜与机关，在尽头检查石碑即记录。"
                   "注意：游戏数据里 12 个古墓名共用 9 个 ID，判定按 ID 计数、不拆分。",
        "text_en": "Finish the tomb's puzzles and examine the stele at the end. Note: 12 tomb names "
                   "share 9 IDs in game data, so detection is per ID.",
    },
    "gym": {
        "text_zh": "克劳馥庄园与联机训练场景内的练习用收集品。不参与 100%。",
        "text_en": "Practice collectibles in Croft Manor / co-op training areas. Not required for 100%.",
    },
    "test": {
        "text_zh": "游戏数据中的占位条目（Document_Test），不会出现在正常流程。",
        "text_en": "Placeholder entry in game data (Document_Test); never appears in normal play.",
    },
    "croft_manor_extra": {
        "text_zh": "克劳馥庄园（Prologue）内的信件 / 照片等。不参与 100%。",
        "text_en": "Letters and photos inside Croft Manor (Prologue). Not required for 100%.",
    },
}

#: 条目 -> 获取方式规则键
def rule_key_for(category: str, subcategory: str | None) -> str:
    if category == "artifacts" and subcategory:
        return {"relics": "relic", "documents": "document", "murals": "mural"}.get(subcategory, "relic")
    return {
        "survival_caches": "survival_cache",
        "monoliths": "monolith",
        "crypt_treasures": "crypt_treasure",
        "crypt_entrances": "crypt_entrance",
        "strongboxes": "strongbox",
        "explorer_backpacks": "explorer_backpack",
        "archivist_maps": "archivist_map",
        "challenge_tombs": "challenge_tomb",
        "gym_training": "gym",
        "test_items": "test",
        "croft_manor_extra": "croft_manor_extra",
    }.get(category, "relic")


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    work = Path(sys.argv[1])
    check = "--check" in sys.argv

    dictionary = load_json(DICT_PATH)["mapping"]
    names = {k.upper(): (v[0] if isinstance(v, list) and v else None) for k, v in dictionary.items()}
    bindings = load_json(S022_DATA / "region_bindings.json")["bindings"]
    region_names = load_json(S022_DATA / "region_names.json")
    unit_names = load_json(S022_DATA / "unit_names.json")
    item_names = load_json(S022_DATA / "item_names.json")

    dtp = DtpFile((work / "extracted" / "dtp_0360E8.bin").read_bytes())
    story_rows = parse_story_items(dtp)
    nonstory_rows = parse_nonstory_items(dtp)

    # 区域哈希 -> slug（已绑定区域 + 特殊区域）
    hash_to_slug = {h: b["region_slug"] for h, b in bindings.items()}
    for slug, entry in region_names.items():
        if entry.get("region_hash"):
            hash_to_slug.setdefault(entry["region_hash"], slug)

    def position_of(row: dict) -> dict | None:
        # 坐标是「关卡内归一化」坐标，没有所属子关卡时无法解释，不保留。
        if not row.get("unit"):
            return None
        x, y = row["coordinate_x"], row["coordinate_y"]
        if x == 0.0 and y == 0.0:
            return None
        if not (0.0 <= x <= 1.0 and 0.0 <= y <= 1.0):
            return None  # 训练场等越界坐标不作为关卡内相对位置
        return {"x": round(x, 6), "y": round(y, 6), "space": POSITION_SPACE}

    items: dict[str, dict] = {}
    collections: list[dict] = []
    seen_collections: set[str] = set()

    for row in story_rows:
        iid = f"0x{row['id']}"
        group = row["collection"]
        required = (not row["is_dlc"]) and group != "Prologue"
        category = "croft_manor_extra" if group == "Prologue" else "artifacts"
        subcategory = STORY_TYPE_SUBCATEGORY.get(row["type"])
        local = item_names.get(iid, {})
        items[iid] = {
            "kind": "story",
            "name": names.get(iid.upper()) or row["metric_name"],
            "name_en": local.get("name_en"),
            "name_zh": local.get("name_zh"),
            "name_source": local.get("name_source"),
            "collection": group,
            "is_dlc": row["is_dlc"],
            "required_for_100": required,
            "region_hash": row["region_hash"],
            "region_slug": hash_to_slug.get(row["region_hash"]),
            "category": category,
            "subcategory": subcategory,
            "marker": row["marker"],
            "unit": row["unit"],
            "position": position_of(row),
            "acquisition_note": None,
        }
        if group not in seen_collections:
            seen_collections.add(group)
            collections.append({"name": group, "is_dlc": row["is_dlc"],
                                "count": 0, "required_for_100": required})
    for entry in collections:
        entry["count"] = sum(1 for v in items.values()
                             if v["kind"] == "story" and v["collection"] == entry["name"])

    # 故事与故事之间可能重名且共用 ID（实测无重复，保留防御）
    shared_ids: dict[str, list[str]] = {}
    for row in nonstory_rows:
        iid = f"0x{row['id']}"
        metric = row["metric_name"]
        if iid in items:
            if items[iid]["name"] != metric:
                shared_ids.setdefault(iid, [items[iid]["name"]])
                if metric not in shared_ids[iid]:
                    shared_ids[iid].append(metric)
            continue
        category = NONSTORY_TYPE_CATEGORY.get(row["type"], f"unknown_type_{row['type']}")
        if "gym" in (metric or "").lower():
            category = "gym_training"
        base_required = CATEGORY_REQUIRED.get(category, False)
        required = None if base_required is None else (base_required and not row["is_dlc"])
        items[iid] = {
            "kind": "nonstory",
            "name": names.get(iid.upper()) or metric,
            "name_en": None,
            "name_zh": None,
            "name_source": None,
            "type": row["type"],
            "marker": row["marker"],
            "is_dlc": row["is_dlc"],
            "required_for_100": required,
            "region_hash": row["region_hash"],
            "region_slug": hash_to_slug.get(row["region_hash"]),
            "category": category,
            "subcategory": None,
            "unit": row["unit"],
            "position": position_of(row),
            "acquisition_note": None,
        }
    for iid, shared in shared_ids.items():
        items[iid]["shared_id"] = True
        items[iid]["shared_names"] = shared

    # ---------------- gameflow regions ----------------
    gameflow = DtpFile((work / "extracted" / "gameflow_dtp_0360F4.bin").read_bytes())
    groot = gameflow.ref_target(gameflow.body_pos)
    num_regions = gameflow.u32(groot + 64)
    region_target = gameflow.ref_target(groot + 72)

    def list_at(ref_pos, count_pos, stride=4):
        target = gameflow.ref_target(ref_pos)
        count = gameflow.u32(count_pos)
        if target is None or count == 0 or count > 4096:
            return []
        return [gameflow.u32(target + stride * i) for i in range(count)]

    item_counts = Counter(v["region_hash"] for v in items.values())
    regions = []
    covered_hashes: set[str] = set()
    for i in range(num_regions):
        b = region_target + i * 256
        region_hash = f"{gameflow.u32(b):08X}"
        covered_hashes.add(region_hash)
        binding = bindings.get(region_hash)
        slug = hash_to_slug.get(region_hash)
        named = region_names.get(slug, {}) if slug else {}
        regions.append({
            "region_hash": region_hash,
            "slug": slug,
            "name_en": named.get("name_en"),
            "name_zh": named.get("name_zh"),
            "name_source": named.get("name_source"),
            "note": named.get("note"),
            "is_dlc": bool(gameflow.u8(b + 52)),
            "counts_toward_percent": bool(gameflow.u8(b + 41)),
            "item_count": item_counts.get(region_hash, 0),
            "binding": None if not binding else {
                "signature_distance": binding["signature_distance"],
                "runner_up_distance": binding["runner_up_distance"],
                "signature": binding["signature"],
                "source_targets": binding["targets"],
            },
            "challenge_tombs": list_at(b + 176, b + 184),
            "campsites": list_at(b + 192, b + 200),
            "challenges": list_at(b + 208, b + 216, stride=8),
            "missions": list_at(b + 224, b + 232, stride=8),
        })

    # 特殊区域：条目引用了、但不在游戏区域主表里的哈希（如 29E50C12）也要有区域记录，
    # 否则条目无处归属。只记录条目计数与归属口径说明，不声称是 26 区主表的一部分。
    for slug, named in region_names.items():
        region_hash = named.get("region_hash")
        if not region_hash or region_hash in covered_hashes or not item_counts.get(region_hash):
            continue
        regions.append({
            "region_hash": region_hash,
            "slug": slug,
            "name_en": named.get("name_en"),
            "name_zh": named.get("name_zh"),
            "name_source": named.get("name_source"),
            "note": named.get("note"),
            "is_dlc": False,
            "counts_toward_percent": False,
            "item_count": item_counts[region_hash],
            "binding": None,
            "challenge_tombs": [],
            "campsites": [],
            "challenges": [],
            "missions": [],
        })

    # ---------------- totals / categories ----------------
    by_cat = Counter(v["category"] for v in items.values())
    required_by_cat = Counter(v["category"] for v in items.values() if v["required_for_100"])
    categories = []
    for cat, count in sorted(by_cat.items()):
        required = CATEGORY_REQUIRED.get(cat, False)
        rule_key = rule_key_for(cat, None)
        rule = ACQUISITION_RULES[rule_key]
        entry = {
            "id": cat,
            "name": CATEGORY_NAMES.get(cat, cat),
            "count": count,
            "required_for_100": required,
            "required_count": required_by_cat.get(cat, 0),
            "acquisition_rule_key": rule_key,
        }
        if cat == "artifacts":
            entry["subcategories"] = [
                {"id": sub_id, "name": SUBCATEGORY_NAMES[sub_id],
                 "count": sum(1 for v in items.values()
                              if v["category"] == "artifacts" and v.get("subcategory") == sub_id),
                 "acquisition_rule_key": SUBCATEGORY_RULE_KEY[sub_id]}
                for sub_id in ("relics", "documents", "murals")
            ]
        else:
            present = [sub_id for sub_id in ("relics", "documents", "murals")
                       if any(v["category"] == cat and v.get("subcategory") == sub_id
                              for v in items.values())]
            if present:
                entry["subcategories"] = [
                    {"id": sub_id, "name": SUBCATEGORY_NAMES[sub_id],
                     "count": sum(1 for v in items.values()
                                  if v["category"] == cat and v.get("subcategory") == sub_id),
                     "acquisition_rule_key": SUBCATEGORY_RULE_KEY[sub_id]}
                    for sub_id in present
                ]
        if required is None:
            entry["note"] = "未闭合（12 名/9 对象/9 唯一 ID，5 个共用占位 ID）；按 T22 不进入缺失结论"
        categories.append(entry)

    acquisition_rules = {
        key: {
            "text_zh": rule["text_zh"],
            "text_en": rule["text_en"],
            "source": "curated",
            "limitations": ["人工整理的类别通用说明，未逐条与游戏内核实"],
        }
        for key, rule in ACQUISITION_RULES.items()
    }

    catalog = {
        "catalog_schema_version": "2.0.0",
        "catalog_version": "2.0.0",
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
            "locale_source": "locals.bin（英 / 简中，allvo_allplt）按表下标对齐；见 research/s022/README.md",
            "zone_source": "TrRebootModTools tr11zones.bt（MIT）",
            "region_binding": "存档区域计数器来源目标 vs 目录区域×类别计数（L1 最近邻，距离 0）；"
                              "证据见 research/s022/data/region_bindings.json",
            "acquisition_text": "类别通用规则为人工整理（curated）；逐项注记见 items[].acquisition_note",
            "date": "2026-10-17",
            "license_decision": "用户已确认商业游戏数据可随交付版发布（2026-10-17）",
        },
        "totals": {
            "items": len(items),
            "story": sum(1 for v in items.values() if v["kind"] == "story"),
            "nonstory": sum(1 for v in items.values() if v["kind"] == "nonstory"),
            "required_for_100": sum(1 for v in items.values() if v["required_for_100"]),
            "dlc": sum(1 for v in items.values() if v["is_dlc"]),
            "with_position": sum(1 for v in items.values() if v["position"]),
            "with_region": sum(1 for v in items.values() if v["region_slug"]),
        },
        "acquisition_rules": acquisition_rules,
        "categories": categories,
        "collections": collections,
        "units": unit_names,
        "regions": regions,
        "items": items,
    }

    # 自检
    story_required = sum(1 for v in items.values()
                         if v["kind"] == "story" and v["required_for_100"])
    checks = {
        "story_total_302": catalog["totals"]["story"] == 302,
        "nonstory_total_168": catalog["totals"]["nonstory"] == 168,
        "story_required_253": story_required == 253,
        "all_items_have_region": catalog["totals"]["with_region"] == len(items),
        "story_names_localized": sum(1 for v in items.values()
                                     if v["kind"] == "story" and v["name_zh"]) == 302,
        "all_units_named": all(v["zone"] for v in unit_names.values()),
    }
    print("checks:", checks)
    if not all(checks.values()):
        print("SELF-CHECK FAILED; refusing to write")
        return 1

    if check:
        current = load_json(OUT_PATH)
        same = json.dumps(current, sort_keys=True) == json.dumps(catalog, sort_keys=True)
        print("--check:", "identical" if same else "DIFFERS")
        return 0 if same else 1

    OUT_PATH.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"written {OUT_PATH} ({OUT_PATH.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
