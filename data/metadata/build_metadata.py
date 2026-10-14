#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""SX-008：从研究数据转换为版本化元数据（data/metadata/*.json）。

用法（仓库根目录）：

    py -3 data/metadata/build_metadata.py          # 生成三个数据文件与 manifest.json
    py -3 data/metadata/build_metadata.py --check  # 只校验现有文件与 manifest 一致

输入（仓库内研究参照副本，来自授权研究包 tool/data/）：
    research/python-v0.2/data/collectible_id_candidates.json
    research/python-v0.2/data/region_counter_candidates.json
    research/python-v0.2/data/reference_category_ids.json

输出：
    data/metadata/collectibles.json      条目与类别
    data/metadata/region_counters.json   区域与 43 项稀疏计数映射
    data/metadata/reference_sets.json    参考集合与攻略组
    data/metadata/manifest.json          输入与输出内容哈希清单

约束：
    - 只做显式转换：不推断内部名到游戏名称、不生成坐标、不建立 ID 到攻略编号的映射；
    - 名称、状态与位置分开；中文名未知为 null（人工简名带来源，不是正式游戏名称）；
    - 参考 ID 仅取两份作者标注 100% 参考槽位共同且相等的类别集合；
    - 43 项计数只采用研究转录的稀疏映射，缺项不是 0；
    - 生成后校验数量与交叉一致性；任一项不符即退出非零，不写出文件。

本脚本为数据构建工具（开发期使用，随仓库分发）；不进入产品运行路径。
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA_DIR = ROOT / "data" / "metadata"
RESEARCH_DIR = ROOT / "research" / "python-v0.2" / "data"

METADATA_VERSION = "0.1.0"
SCHEMA_VERSION = "1.0.0"
NAMESPACE = "sottr"
REVIEW_DATE = "2026-10-14"

# ---------------------------------------------------------------------------
# 交叉验证基线（来自已确认开发方案与 SX-001 基线记录；不符合即失败）
# ---------------------------------------------------------------------------
EXPECTED_ITEM_COUNT = 866
EXPECTED_NAME_COUNT = 871
EXPECTED_COUNTER_COUNT = 43
EXPECTED_REFERENCE_COUNTS = {
    "survival_caches": 65,
    "rare_resource_caches": 12,
    "monoliths": 12,
    "crypt_treasures": 10,
}
# 两份参考槽位的压缩文件 SHA-256（研究包 tool/evidence/public_sources.json 与
# evidence/progress/progress_evidence.json.input_sha256；不随产品分发存档字节）。
REFERENCE_SAVE_HASHES = [
    "e57947b199677baecb312134aa23b01c6260e42cd8c38ae113ce1d49912cd813",
    "6d9b468e51317025bdabb8ffeb91e789d4461547c2edb3786de5326d6c644ba2",
]

# ---------------------------------------------------------------------------
# 来源表（三类文件写入同一份；合并时去重）
# ---------------------------------------------------------------------------
SOURCES = {
    "src:trrebootmodtools-dict": {
        "url": "https://github.com/arcusmaximus/TrRebootModTools",
        "fixed_revision": "a79c48b75f1ffaaf623dc73a625211bf98ff0504",
        "accessed": "2026-10-11",
        "license": "MIT",
        "redistributable": "yes",
        "method": "内部 ID 与候选名称字典转录（研究包 tool/data/collectible_id_candidates.json）",
        "review": "许可与固定提交已复核；候选名称不等于游戏显示名称",
    },
    "src:asl-sottr": {
        "url": "https://github.com/Atorizil/ASL-SOTTR",
        "fixed_revision": "b758ee890eac30d9e3b3477999a30f8f9e442f00",
        "accessed": "2026-10-11",
        "license": None,
        "redistributable": "pending",
        "method": "区域/类别计数目标与磁盘偏移的事实转录及独立比对",
        "review": "上游未声明许可证；仅独立转录事实，发布包含需 SX-018 再分发复核",
    },
    "src:stella-saves": {
        "url": "https://tombraiders.net/stella/savegame/TR11saves.html",
        "fixed_revision": None,
        "accessed": "2026-10-13",
        "license": None,
        "redistributable": "pending",
        "method": "公开参考存档来源与压缩文件 SHA-256（两个作者标注 100% 槽位）",
        "review": "公开存档不随产品分发；仅记录来源与哈希",
    },
    "src:stella-san-juan": {
        "url": (
            "https://tombraiders.net/stella/walks/TR11walk/collectibles/"
            "13-san-juan-murals-caches.html"
        ),
        "fixed_revision": "page updated 2021-10-31",
        "accessed": "2026-10-14",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "圣胡安贮藏点/石碑的区域、总数与位置描述引用",
        "review": "不打包攻略内容；组级线索不冒充精确条目映射",
    },
    "src:stella-san-juan-monolith": {
        "url": "https://tombraiders.net/stella/walks/TR11walk/details/san-juan-8.html",
        "fixed_revision": None,
        "accessed": "2026-10-14",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "圣胡安石碑位置细节引用",
        "review": "仅链接引用",
    },
    "src:stella-san-juan-crypt": {
        "url": "https://tombraiders.net/stella/walks/TR11walk/details/san-juan-4.html",
        "fixed_revision": None,
        "accessed": "2026-10-14",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "圣胡安墓穴（Crypt of Huascar）位置线索引用",
        "review": "墓穴宝藏条目未逐一核对；仅组级线索",
    },
    "src:plan-doc": {
        "url": None,
        "fixed_revision": "sottr-explorer_开发方案.md v0.4（2026-10-11）",
        "accessed": "2026-10-13",
        "license": None,
        "redistributable": "pending",
        "method": "已确认方案中的区域/类别/组级线索转录（人工简名，不是游戏正式名称）",
        "review": "人工简名与组级线索；不冒充精确条目映射",
    },
    "src:research-evidence": {
        "url": None,
        "fixed_revision": (
            "SOTTR_Save_Research v0.2（evidence/progress/FINDINGS.md、"
            "tool/evidence/public_sources.json）"
        ),
        "accessed": "2026-10-13",
        "license": "内部研究记录；不随产品分发",
        "redistributable": "no",
        "method": "独立计数交叉比对、参考集合共同性与公开样本哈希结论",
        "review": "仅转录事实结论；私人存档字节与绝对路径不进入任何数据文件",
    },
}

# ---------------------------------------------------------------------------
# 类别与区域（人工简名来自开发方案；未给出中文名的类别/区域为 null）
# ---------------------------------------------------------------------------
CATEGORIES = [
    # (category_id, display_name, source_refs)
    ("survival_caches", "贮藏点", ["src:plan-doc", "src:trrebootmodtools-dict"]),
    ("rare_resource_caches", "稀有资源贮藏点", ["src:plan-doc", "src:trrebootmodtools-dict"]),
    ("monoliths", "石碑", ["src:plan-doc", "src:trrebootmodtools-dict"]),
    ("crypt_treasures", "地下墓穴宝藏", ["src:plan-doc", "src:trrebootmodtools-dict"]),
    ("strongboxes", None, ["src:asl-sottr"]),
    ("relics", None, ["src:asl-sottr"]),
    ("documents", None, ["src:asl-sottr"]),
    ("murals", None, ["src:asl-sottr"]),
    ("crypts", None, ["src:asl-sottr"]),
]

REGION_ID_FROM_SOURCE = {
    "Cozumel": "cozumel",
    "Peruvian Jungle": "peruvian_jungle",
    "Kuaq Yaku": "kuwaq_yaku",  # 来源拼写；游戏常用拼写为 Kuwaq Yaku
    "Trial of the Eagle": "trial_of_the_eagle",
    "Paititi": "paititi",
    "Head of the Serpent": "head_of_the_serpent",
    "Cenote": "cenote",
    "Porvenir Oil Fields": "porvenir_oil_fields",
    "Mission of San Juan": "mission_of_san_juan",
}
REGION_DISPLAY_NAMES = {
    "cozumel": "科苏梅尔",
    "paititi": "帕伊提提",
    "mission_of_san_juan": "圣胡安",
}

# 计数类别（社区脚本口径）与参考类别（内部前缀）的显式对应。
COUNTER_CATEGORY = {
    "Strongboxes": "strongboxes",
    "Relics": "relics",
    "Documents": "documents",
    "Murals": "murals",
    "Survival Caches": "survival_caches",
    "Monoliths": "monoliths",
    "Crypts": "crypts",
}
# 计数合并口径：贮藏点计数同时对应普通与稀有贮藏点（研究证据：77 = 65 + 12）。
COUNTS_TOWARD = {
    "survival_caches": ["rare_resource_caches", "survival_caches"],
    "monoliths": ["monoliths"],
    "crypts": ["crypt_treasures"],
}
REFERENCE_CATEGORY = {
    "CollectibleTypeSurvivalCache_": "survival_caches",
    "RareResourceCacheCollectible_": "rare_resource_caches",
    "CollectibleTypeMonolith_": "monoliths",
    "CollectibleTypeCryptTreasure_": "crypt_treasures",
}

REFERENCE_SET_ID = "rs:stella-100pct-common"

# 圣胡安/帕伊提提的区域与组级线索（来自开发方案 12 项表与攻略组总数；
# 未建立任何 ID 到攻略编号或坐标的映射）。
SAN_JUAN_CACHES = [
    "0x0F28B6EC",
    "0x0F28B6ED",
    "0x0F28B6EE",
    "0x0F28B6EF",
    "0xAE1A2E7A",
    "0xD8CD8257",
    "0xFF9EAC6E",
    "0xFF9EAC79",
]
SAN_JUAN_MONOLITHS = ["0x439B19A6"]
SAN_JUAN_CRYPTS = ["0xBA3B0D06"]
PAITITI_CLUES = ["0x851B4915", "0xB419E6FE"]

GUIDE_GROUPS = [
    {
        "guide_group_id": "stella:san-juan-caches",
        "display_name": "圣胡安贮藏点组",
        "region_id": "mission_of_san_juan",
        "category_ids": ["rare_resource_caches", "survival_caches"],
        "member_ids": SAN_JUAN_CACHES,
        "target_count": 8,
        "counter_keys": ["counter:mission_of_san_juan:survival_caches"],
        "source_refs": [
            "src:stella-san-juan",
            "src:plan-doc",
            "src:asl-sottr",
            "src:research-evidence",
        ],
    },
    {
        "guide_group_id": "stella:san-juan-monolith",
        "display_name": "圣胡安石碑组",
        "region_id": "mission_of_san_juan",
        "category_ids": ["monoliths"],
        "member_ids": SAN_JUAN_MONOLITHS,
        "target_count": 1,
        "counter_keys": ["counter:mission_of_san_juan:monoliths"],
        "source_refs": [
            "src:stella-san-juan",
            "src:stella-san-juan-monolith",
            "src:plan-doc",
            "src:asl-sottr",
        ],
    },
    {
        "guide_group_id": "stella:san-juan-crypt",
        "display_name": "圣胡安墓穴宝藏组",
        "region_id": "mission_of_san_juan",
        "category_ids": ["crypt_treasures"],
        "member_ids": SAN_JUAN_CRYPTS,
        "target_count": 1,
        "counter_keys": ["counter:mission_of_san_juan:crypts"],
        "source_refs": [
            "src:stella-san-juan-crypt",
            "src:plan-doc",
            "src:asl-sottr",
        ],
    },
]


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def dump(path: Path, payload) -> None:
    text = json.dumps(payload, ensure_ascii=False, indent=2) + "\n"
    path.write_text(text, encoding="utf-8", newline="\n")


def item_note(item_id: str) -> str | None:
    if item_id in SAN_JUAN_CACHES or item_id in SAN_JUAN_MONOLITHS:
        return "区域与攻略组来自攻略页与整组计数证据；未建立 ID 到攻略编号或坐标的映射"
    if item_id in SAN_JUAN_CRYPTS:
        return "区域与攻略组来自攻略页与墓穴计数证据；未建立 ID 到攻略编号或坐标的映射"
    if item_id in PAITITI_CLUES:
        return "仅有区域线索；具体定位未解决"
    return None


def item_review(item_id: str):
    if item_id in SAN_JUAN_CACHES or item_id in SAN_JUAN_MONOLITHS or item_id in SAN_JUAN_CRYPTS:
        return {
            "state": "draft",
            "note": "组级定位已记录；ID 到攻略编号与坐标未建立",
            "date": REVIEW_DATE,
        }
    if item_id in PAITITI_CLUES:
        return {"state": "draft", "note": "区域线索已记录；具体定位未解决", "date": REVIEW_DATE}
    return None


def build_items(dictionary, reference_ids_by_category):
    """构建 items 表：全部候选名称保留；类别与区域仅在证据支持时填入。"""
    category_of = {}
    for category_id, ids in reference_ids_by_category.items():
        for item_id in ids:
            category_of[item_id] = category_id

    items = {}
    for item_id in sorted(dictionary):
        aliases = list(dictionary[item_id])
        category_id = category_of.get(item_id)
        region_id = None
        mappings = []
        source_refs = ["src:trrebootmodtools-dict"]

        if item_id in SAN_JUAN_CACHES:
            region_id = "mission_of_san_juan"
            mappings = [
                {
                    "mapping_kind": "guide_group_membership",
                    "region_id": region_id,
                    "guide_group_id": "stella:san-juan-caches",
                    "guide_item": None,
                    "target_count": 8,
                    "coordinates": None,
                    "map_transform": None,
                    "applicable_mode": "campaign",
                    "applicable_platform": None,
                    "source_refs": ["src:stella-san-juan", "src:plan-doc"],
                }
            ]
        elif item_id in SAN_JUAN_MONOLITHS:
            region_id = "mission_of_san_juan"
            mappings = [
                {
                    "mapping_kind": "guide_group_membership",
                    "region_id": region_id,
                    "guide_group_id": "stella:san-juan-monolith",
                    "guide_item": None,
                    "target_count": 1,
                    "coordinates": None,
                    "map_transform": None,
                    "applicable_mode": "campaign",
                    "applicable_platform": None,
                    "source_refs": ["src:stella-san-juan", "src:stella-san-juan-monolith"],
                }
            ]
        elif item_id in SAN_JUAN_CRYPTS:
            region_id = "mission_of_san_juan"
            mappings = [
                {
                    "mapping_kind": "guide_group_membership",
                    "region_id": region_id,
                    "guide_group_id": "stella:san-juan-crypt",
                    "guide_item": None,
                    "target_count": 1,
                    "coordinates": None,
                    "map_transform": None,
                    "applicable_mode": "campaign",
                    "applicable_platform": None,
                    "source_refs": ["src:stella-san-juan-crypt"],
                }
            ]
        elif item_id in PAITITI_CLUES:
            region_id = "paititi"
            mappings = [
                {
                    "mapping_kind": "region_clue",
                    "region_id": region_id,
                    "guide_group_id": None,
                    "guide_item": None,
                    "target_count": None,
                    "coordinates": None,
                    "map_transform": None,
                    "applicable_mode": "campaign",
                    "applicable_platform": None,
                    "source_refs": ["src:research-evidence", "src:plan-doc"],
                }
            ]

        if category_id is not None:
            source_refs = ["src:trrebootmodtools-dict", "src:stella-saves"]
        note = item_note(item_id)
        notes = []
        if note:
            notes.append(note)
        elif category_id is not None:
            notes.append("类别来自参考集分组转录；区域未解决")

        items[item_id] = {
            "internal_id": item_id,
            "aliases": aliases,
            "display_name": None,
            "display_language": None,
            "name_source": None,
            "name_kind": "unknown",
            "category_id": category_id,
            "region_id": region_id,
            "applicability": {
                "game": "sottr",
                "platform": None,
                "mode": None,
                "build_range": None,
                "dlc": None,
                "notes": notes,
            },
            "location_mappings": mappings,
            "source_refs": source_refs,
            "review": item_review(item_id),
        }
    return items


def build_categories():
    return {
        category_id: {
            "category_id": category_id,
            "display_name": display_name,
            "name_kind": "curated_short" if display_name else "unknown",
            "source_refs": source_refs,
        }
        for category_id, display_name, source_refs in CATEGORIES
    }


def build_regions(region_ids):
    regions = {}
    for region_id in sorted(region_ids):
        display_name = REGION_DISPLAY_NAMES.get(region_id)
        source_refs = ["src:asl-sottr"]
        if display_name:
            source_refs = ["src:plan-doc", "src:asl-sottr"]
        regions[region_id] = {
            "region_id": region_id,
            "display_name": display_name,
            "name_kind": "curated_short" if display_name else "unknown",
            "source_refs": source_refs,
        }
    return regions


def build_region_counters(counter_entries):
    counters = {}
    for entry in sorted(
        counter_entries, key=lambda e: (e["serialized_offset"], e["category"])
    ):
        region_id = REGION_ID_FROM_SOURCE[entry["region_as_spelled_in_source"]]
        category_id = COUNTER_CATEGORY[entry["category"]]
        counts_toward = sorted(set(COUNTS_TOWARD.get(category_id, [])))
        counter_key = f"counter:{region_id}:{category_id}"
        counters[counter_key] = {
            "counter_key": counter_key,
            "region_id": region_id,
            "category_id": category_id,
            "serialized_offset": entry["serialized_offset"],
            "serialized_row_index": entry["serialized_row_index"],
            "serialized_relative_offset": entry["serialized_relative_offset"],
            "source_total": entry["source_total"],
            "counts_toward": counts_toward,
            "source_refs": ["src:asl-sottr"],
        }
    return counters


def build_reference_set(reference_groups):
    category_ids = [category for category in REFERENCE_CATEGORY.values()]
    reference_ids = {}
    for group_name, category_id in REFERENCE_CATEGORY.items():
        group = reference_groups[group_name]
        reference_ids[category_id] = sorted(set(group["reference_ids"]))
    return {
        "reference_set_id": REFERENCE_SET_ID,
        "version": "1.0.0",
        "category_ids": sorted(category_ids),
        "reference_ids": dict(sorted(reference_ids.items())),
        "applicability": {
            "game": "sottr",
            "platform": "windows_steam",
            "mode": "campaign",
            "build_range": None,
            "dlc": None,
            "notes": [
                "参考集合为两个作者标注 100% 参考槽位的共同且相等集合；不证明全 DLC/全版本/全模式适用",
                "差集为空不等于全游戏收集完成",
            ],
        },
        "source_save_hashes": list(REFERENCE_SAVE_HASHES),
        "dedup_rule": "集合去重：重复记录保留但只按唯一 ID 计一次；仅使用两份参考槽位共同且相等的类别集合",
        "source_refs": ["src:stella-saves", "src:research-evidence"],
    }


def build_guide_groups():
    groups = {}
    for definition in GUIDE_GROUPS:
        group_id = definition["guide_group_id"]
        groups[group_id] = {
            "guide_group_id": group_id,
            "display_name": definition["display_name"],
            "name_kind": "curated_short",
            "region_id": definition["region_id"],
            "category_ids": sorted(definition["category_ids"]),
            "member_ids": sorted(definition["member_ids"]),
            "target_count": definition["target_count"],
            "mapping_kind": "guide_group_membership",
            "reference_set_id": REFERENCE_SET_ID,
            "counter_keys": sorted(definition["counter_keys"]),
            "source_refs": definition["source_refs"],
        }
    return groups


def build_documents(dictionary, counter_entries, reference_groups):
    reference_ids_by_category = {
        category_id: sorted(set(reference_groups[group_name]["reference_ids"]))
        for group_name, category_id in REFERENCE_CATEGORY.items()
    }
    items = build_items(dictionary, reference_ids_by_category)
    categories = build_categories()
    regions = build_regions(
        REGION_ID_FROM_SOURCE[e["region_as_spelled_in_source"]] for e in counter_entries
    )
    region_counters = build_region_counters(counter_entries)
    reference_set = build_reference_set(reference_groups)
    guide_groups = build_guide_groups()

    def empty_sections():
        return {
            "items": {},
            "categories": {},
            "regions": {},
            "guide_groups": {},
            "reference_sets": {},
            "region_counters": {},
            "sources": dict(SOURCES),
        }

    collectibles = {
        "schema_version": SCHEMA_VERSION,
        "metadata_version": METADATA_VERSION,
        "namespace": NAMESPACE,
        **empty_sections(),
    }
    collectibles["items"] = items
    collectibles["categories"] = categories

    counters = {
        "schema_version": SCHEMA_VERSION,
        "metadata_version": METADATA_VERSION,
        "namespace": NAMESPACE,
        **empty_sections(),
    }
    counters["regions"] = regions
    counters["region_counters"] = region_counters

    references = {
        "schema_version": SCHEMA_VERSION,
        "metadata_version": METADATA_VERSION,
        "namespace": NAMESPACE,
        **empty_sections(),
    }
    references["reference_sets"] = {REFERENCE_SET_ID: reference_set}
    references["guide_groups"] = guide_groups

    return collectibles, counters, references


def validate(dictionary, counter_entries, reference_groups, documents):
    collectibles, counters, references = documents

    assert len(dictionary) == EXPECTED_ITEM_COUNT, (
        f"字典 ID 数 {len(dictionary)} != {EXPECTED_ITEM_COUNT}"
    )
    name_count = sum(len(names) for names in dictionary.values())
    assert name_count == EXPECTED_NAME_COUNT, f"名称数 {name_count} != {EXPECTED_NAME_COUNT}"
    assert len(counter_entries) == EXPECTED_COUNTER_COUNT, (
        f"计数条目 {len(counter_entries)} != {EXPECTED_COUNTER_COUNT}"
    )

    # 参考四组数量与共同集合
    for group_name, category_id in REFERENCE_CATEGORY.items():
        group = reference_groups[group_name]
        assert group["identical_reference_sets"] is True, f"{group_name} 参考集不相同"
        assert len(group["reference_ids"]) == EXPECTED_REFERENCE_COUNTS[category_id], (
            f"{group_name} 数量 {len(group['reference_ids'])} "
            f"!= {EXPECTED_REFERENCE_COUNTS[category_id]}"
        )
        assert group["reference_count"] == len(group["reference_ids"])
    reference_set = references["reference_sets"][REFERENCE_SET_ID]
    for category_id, expected in EXPECTED_REFERENCE_COUNTS.items():
        assert len(reference_set["reference_ids"][category_id]) == expected
    reference_ids = {
        item_id for ids in reference_set["reference_ids"].values() for item_id in ids
    }
    assert len(reference_ids) == 99, f"参考 ID 总数 {len(reference_ids)} != 99"
    missing = [item_id for item_id in reference_ids if item_id not in collectibles["items"]]
    assert not missing, f"参考 ID 不在条目表：{missing}"

    # 43 项计数：偏移恒等式、区域/类别与作用域合计
    for counter_key, entry in counters["region_counters"].items():
        expected_offset = 0x10BC + entry["serialized_row_index"] * 104 + entry[
            "serialized_relative_offset"
        ]
        assert entry["serialized_offset"] == expected_offset, counter_key
        assert entry["region_id"] in counters["regions"], counter_key
        assert entry["category_id"] in collectibles["categories"], counter_key
        for category_id in entry["counts_toward"]:
            assert category_id in reference_set["category_ids"], counter_key
    scope_source_totals = {}
    for entry in counters["region_counters"].values():
        key = tuple(entry["counts_toward"])
        scope_source_totals[key] = scope_source_totals.get(key, 0) + entry["source_total"]
    assert scope_source_totals[("rare_resource_caches", "survival_caches")] == 77
    assert scope_source_totals[("monoliths",)] == 12
    assert scope_source_totals[("crypt_treasures",)] == 10

    # 攻略组：成员落于参考范围；计数目标与组目标一致
    for group_id, group in references["guide_groups"].items():
        union = set()
        for category_id in group["category_ids"]:
            union.update(reference_set["reference_ids"][category_id])
        for item_id in group["member_ids"]:
            assert item_id in union, f"{group_id} 成员 {item_id} 不在参考范围"
        target = sum(
            counters["region_counters"][key]["source_total"] for key in group["counter_keys"]
        )
        assert group["target_count"] == target, f"{group_id} 目标数 {group['target_count']} != {target}"

    # 不产生精确点位或条目映射的假数据
    for item_id, item in collectibles["items"].items():
        for mapping in item["location_mappings"]:
            assert mapping["mapping_kind"] in ("guide_group_membership", "region_clue"), item_id
            assert mapping["coordinates"] is None, item_id
            assert mapping["guide_item"] is None, item_id

    print("校验通过：")
    print(f"  items={len(collectibles['items'])} aliases={name_count}")
    print(f"  categories={len(collectibles['categories'])} regions={len(counters['regions'])}")
    print(f"  region_counters={len(counters['region_counters'])}")
    print(f"  reference_ids={ {k: len(v) for k, v in reference_set['reference_ids'].items()} }")
    print(f"  guide_groups={len(references['guide_groups'])}")
    print(f"  scope_source_totals={scope_source_totals}")


def main() -> int:
    check_only = "--check" in sys.argv[1:]
    dictionary = load_json(RESEARCH_DIR / "collectible_id_candidates.json")["mapping"]
    counter_entries = load_json(RESEARCH_DIR / "region_counter_candidates.json")["entries"]
    reference_groups = load_json(RESEARCH_DIR / "reference_category_ids.json")["groups"]

    documents = build_documents(dictionary, counter_entries, reference_groups)
    validate(dictionary, counter_entries, reference_groups, documents)

    file_names = ["collectibles.json", "region_counters.json", "reference_sets.json"]
    if not check_only:
        DATA_DIR.mkdir(parents=True, exist_ok=True)
        for file_name, document in zip(file_names, documents):
            dump(DATA_DIR / file_name, document)

    manifest = {
        "metadata_version": METADATA_VERSION,
        "converter": "data/metadata/build_metadata.py",
        "source_inputs": [
            {
                "file": f"research/python-v0.2/data/{name}",
                "sha256": sha256_file(RESEARCH_DIR / name),
            }
            for name in (
                "collectible_id_candidates.json",
                "region_counter_candidates.json",
                "reference_category_ids.json",
            )
        ],
        "files": [
            {
                "file_id": file_name,
                "sha256": sha256_file(DATA_DIR / file_name),
                "bytes": (DATA_DIR / file_name).stat().st_size,
            }
            for file_name in file_names
        ],
    }

    if check_only:
        existing = load_json(DATA_DIR / "manifest.json")
        for name, record in zip(file_names, existing["files"]):
            current = sha256_file(DATA_DIR / name)
            if current != record["sha256"]:
                print(f"FAIL {name} 内容哈希与 manifest 不一致", file=sys.stderr)
                return 1
        print("check 通过：数据文件与 manifest 哈希一致")
        return 0

    dump(DATA_DIR / "manifest.json", manifest)
    for record in manifest["files"]:
        print(f"写入 {record['file_id']} bytes={record['bytes']} sha256={record['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
