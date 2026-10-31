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
        "medium": "text",
        "role": "tool",
    },
    "src:asl-sottr": {
        "url": "https://github.com/Atorizil/ASL-SOTTR",
        "fixed_revision": "b758ee890eac30d9e3b3477999a30f8f9e442f00",
        "accessed": "2026-10-11",
        "license": "上游未声明许可证；仅转录事实数据（偏移与计数），未复制源代码",
        "redistributable": "yes",
        "method": "区域/类别计数目标与磁盘偏移的事实转录及独立比对",
        "review": "定性为事实性数据（2026-10-31）：内存 stride 272 与存档 stride 104 交叉推导、公开样本佐证、图鉴与游戏计数 43/43 吻合",
        "medium": "text",
        "role": "tool",
    },
    "src:stella-saves": {
        "url": "https://tombraiders.net/stella/savegame/TR11saves.html",
        "fixed_revision": None,
        "accessed": "2026-10-13",
        "license": None,
        "redistributable": "pending",
        "method": "公开参考存档来源与压缩文件 SHA-256（两个作者标注 100% 槽位）",
        "review": "公开存档不随产品分发；仅记录来源与哈希",
        "medium": "text",
        "role": "save",
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
        "medium": "text",
        "role": "guide",
    },
    "src:stella-san-juan-monolith": {
        "url": "https://tombraiders.net/stella/walks/TR11walk/details/san-juan-8.html",
        "fixed_revision": None,
        "accessed": "2026-10-14",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "圣胡安石碑位置细节引用",
        "review": "仅链接引用",
        "medium": "text",
        "role": "guide",
    },
    "src:stella-san-juan-crypt": {
        "url": "https://tombraiders.net/stella/walks/TR11walk/details/san-juan-4.html",
        "fixed_revision": None,
        "accessed": "2026-10-14",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "圣胡安墓穴（Crypt of Huascar）位置线索引用",
        "review": "墓穴宝藏条目未逐一核对；仅组级线索",
        "medium": "text",
        "role": "guide",
    },
    "src:plan-doc": {
        "url": None,
        "fixed_revision": "sottr-explorer_开发方案.md v0.4（2026-10-11）",
        "accessed": "2026-10-13",
        "license": None,
        "redistributable": "pending",
        "method": "已确认方案中的区域/类别/组级线索转录（人工简名，不是游戏正式名称）",
        "review": "人工简名与组级线索；不冒充精确条目映射",
        "medium": None,
        "role": "research",
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
        "medium": None,
        "role": "research",
    },
    "src:3dm-full-walkthrough": {
        "url": "https://www.3dmgame.com/gl/3754628.html",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "中文图文全流程攻略目录与区域分页引用（57 页）",
        "review": "2026-10-20 已核实目录与关键区域页可访问；未逐条核对每个收集品",
        "medium": "text",
        "role": "guide",
    },
    "src:gamersky-full-walkthrough": {
        "url": "https://www.gamersky.com/handbook/201809/1098996.shtml",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "中文图文全收集流程攻略章节引用（11 页）",
        "review": "2026-10-20 已核实页面可访问；章节未逐区域命名，作为次级文本来源",
        "medium": "text",
        "role": "guide",
    },
    "src:gamersky-paititi-map": {
        "url": "https://www.gamersky.com/handbook/201809/1100600.shtml",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "帕依提提全收集地图与全物品位置引用",
        "review": "2026-10-20 已核实可访问；地图页不保证与图鉴条目逐条对应",
        "medium": "text",
        "role": "guide",
    },
    "src:gamersky-paititi-relics-documents": {
        "url": "https://www.gamersky.com/handbook/201809/1101523.shtml",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "站点保留权利；仅链接与事实引用",
        "redistributable": "pending",
        "method": "帕依提提遗物与文献位置引用",
        "review": "2026-10-20 已核实可访问；仅覆盖遗物与文献",
        "medium": "text",
        "role": "guide",
    },
    "src:bilibili-platinum-36p": {
        "url": "https://www.bilibili.com/video/BV1iM4y1g78i/",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "用户投稿；站点保留权利；仅链接",
        "redistributable": "pending",
        "method": "按分 P 区域流程引用；不下载或转发视频",
        "review": "2026-10-20 已核实 36P 分 P 列表与 ?p= 地址可访问",
        "medium": "video",
        "role": "guide",
    },
    "src:bilibili-collection-49p": {
        "url": "https://www.bilibili.com/video/BV11v4y1R7jC/",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "用户投稿；站点保留权利；仅链接",
        "redistributable": "pending",
        "method": "按分 P 区域流程引用；不下载或转发视频",
        "review": "2026-10-20 已核实 49P 分 P 列表可访问；作为次级视频来源",
        "medium": "video",
        "role": "guide",
    },
    "src:bilibili-paititi-collect": {
        "url": "https://www.bilibili.com/video/BV1bW4116739/",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "用户投稿；站点保留权利；仅链接",
        "redistributable": "pending",
        "method": "帕依提提收集专题引用；按整段视频链接",
        "review": "2026-10-20 已核实 28P 分 P 列表可访问；部分分 P 为单物品短片",
        "medium": "video",
        "role": "guide",
    },
    "src:bilibili-peruvian-jungle-collect": {
        "url": "https://www.bilibili.com/video/BV1uW411k7uq/",
        "fixed_revision": None,
        "accessed": "2026-10-20",
        "license": "用户投稿；站点保留权利；仅链接",
        "redistributable": "pending",
        "method": "秘鲁丛林收集专题引用；按整段视频链接",
        "review": "2026-10-20 已核实 23P 分 P 列表可访问",
        "medium": "video",
        "role": "guide",
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

# ---------------------------------------------------------------------------
# 阶段 C：区域 × 类别攻略组（成员来自全收集图鉴）
# ---------------------------------------------------------------------------
CATALOG_PATH = ROOT / "data" / "catalog" / "collectibles_catalog.json"

GUIDE_REGION_IDS = [
    "paititi",
    "kuwaq_yaku",
    "mission_of_san_juan",
    "peruvian_jungle",
    "cenote",
    "cozumel",
    "porvenir_oil_fields",
    "trial_of_the_eagle",
    "head_of_the_serpent",
]
# 图鉴中的游戏本地化区域名，仅用于攻略组显示名。
GUIDE_REGION_NAMES = {
    "paititi": "帕依提提",
    "kuwaq_yaku": "库阿克雅库",
    "mission_of_san_juan": "圣胡安教会",
    "peruvian_jungle": "秘鲁丛林",
    "cenote": "灰岩坑",
    "cozumel": "科祖梅尔",
    "porvenir_oil_fields": "波文尼尔石油田",
    "trial_of_the_eagle": "猎鹰的试炼",
    "head_of_the_serpent": "巨蛇之首",
}

# (图鉴 category, subcategory) -> (元数据类别, 组名类别)
GUIDE_CATEGORY_MAP = {
    ("artifacts", "relics"): ("relics", "遗物"),
    ("artifacts", "documents"): ("documents", "文献"),
    ("artifacts", "murals"): ("murals", "壁画"),
    ("survival_caches", None): ("survival_caches", "贮藏点"),
    ("monoliths", None): ("monoliths", "石碑"),
    ("crypt_treasures", None): ("crypt_treasures", "地下墓穴宝藏"),
    ("strongboxes", None): ("strongboxes", "保险箱"),
}
GUIDE_CATEGORY_NAMES = {value[0]: value[1] for value in GUIDE_CATEGORY_MAP.values()}

# 组声明的元数据类别；贮藏点合并普通与稀有（与计数口径一致）。
GUIDE_GROUP_CATEGORY_IDS = {
    "survival_caches": ["rare_resource_caches", "survival_caches"],
}

# 组与计数键的类别映射；只有这些类别可复用区域计数。
GUIDE_COUNTER_CATEGORY = {
    "survival_caches": "survival_caches",
    "monoliths": "monoliths",
    "crypt_treasures": "crypts",
}

TEXT_PRIMARY = "src:3dm-full-walkthrough"
TEXT_FALLBACK = "src:gamersky-full-walkthrough"
VIDEO_PRIMARY = "src:bilibili-platinum-36p"
VIDEO_FALLBACK = "src:bilibili-collection-49p"
EVIDENCE_REF = "src:research-evidence"

REGION_TEXT_SOURCES = {
    "paititi": ["src:gamersky-paititi-map", TEXT_PRIMARY, TEXT_FALLBACK],
}
# 圣胡安已有 Stella 基线来源，保留为其主图文来源。
REGION_CATEGORY_TEXT_PREFIX = {
    ("mission_of_san_juan", "survival_caches"): ["src:stella-san-juan"],
    ("mission_of_san_juan", "murals"): ["src:stella-san-juan"],
    ("mission_of_san_juan", "monoliths"): [
        "src:stella-san-juan",
        "src:stella-san-juan-monolith",
    ],
    ("mission_of_san_juan", "crypt_treasures"): ["src:stella-san-juan-crypt"],
}
REGION_CATEGORY_TEXT_SOURCES = {
    ("paititi", "relics"): ["src:gamersky-paititi-relics-documents"],
    ("paititi", "documents"): ["src:gamersky-paititi-relics-documents"],
}
REGION_VIDEO_SOURCES = {
    "paititi": [VIDEO_PRIMARY, "src:bilibili-paititi-collect", VIDEO_FALLBACK],
    "peruvian_jungle": [VIDEO_PRIMARY, "src:bilibili-peruvian-jungle-collect", VIDEO_FALLBACK],
    "porvenir_oil_fields": [VIDEO_FALLBACK, VIDEO_PRIMARY],
}


def guide_group_source_refs(region_id, category_id):
    refs = list(REGION_CATEGORY_TEXT_PREFIX.get((region_id, category_id), []))
    refs.extend(REGION_TEXT_SOURCES.get(region_id, [TEXT_PRIMARY, TEXT_FALLBACK]))
    refs.extend(REGION_CATEGORY_TEXT_SOURCES.get((region_id, category_id), []))
    refs.extend(REGION_VIDEO_SOURCES.get(region_id, [VIDEO_PRIMARY, VIDEO_FALLBACK]))
    refs.append(EVIDENCE_REF)
    deduped = []
    for ref in refs:
        if ref not in deduped:
            deduped.append(ref)
    return deduped


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


def build_guide_groups(region_counters, catalog):
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

    members = {}
    for item_id, item in catalog["items"].items():
        region_id = item.get("region_slug")
        if region_id not in GUIDE_REGION_IDS:
            continue
        mapped = GUIDE_CATEGORY_MAP.get((item.get("category"), item.get("subcategory")))
        if mapped is None:
            continue
        members.setdefault((region_id, mapped[0]), []).append(item_id)

    for (region_id, category_id), member_ids in sorted(members.items()):
        group_id = f"guide:{region_id}:{category_id}"
        member_ids = sorted(member_ids)
        target_count = None
        counter_keys = []
        counter_category = GUIDE_COUNTER_CATEGORY.get(category_id)
        if counter_category:
            counter_key = f"counter:{region_id}:{counter_category}"
            counter = region_counters.get(counter_key)
            if counter is not None:
                assert counter["source_total"] == len(member_ids), (
                    f"{group_id} 的计数目标 {counter['source_total']} "
                    f"与图鉴成员数 {len(member_ids)} 不一致"
                )
                target_count = len(member_ids)
                counter_keys = [counter_key]
        reference_set_id = (
            REFERENCE_SET_ID
            if category_id in REFERENCE_CATEGORY.values()
            else None
        )
        groups[group_id] = {
            "guide_group_id": group_id,
            "display_name": (
                f"{GUIDE_REGION_NAMES[region_id]} · {GUIDE_CATEGORY_NAMES[category_id]}"
            ),
            "name_kind": "curated_short",
            "region_id": region_id,
            "category_ids": sorted(
                GUIDE_GROUP_CATEGORY_IDS.get(category_id, [category_id])
            ),
            "member_ids": member_ids,
            "target_count": target_count,
            "mapping_kind": "guide_group_membership",
            "reference_set_id": reference_set_id,
            "counter_keys": sorted(counter_keys),
            "source_refs": guide_group_source_refs(region_id, category_id),
        }
    return groups


def build_documents(dictionary, counter_entries, reference_groups, catalog):
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
    guide_groups = build_guide_groups(region_counters, catalog)

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

    # 攻略组：成员必须在条目表、来源必须在来源表；guide: 组 ID 与区域/类别一致。
    for group_id, group in references["guide_groups"].items():
        assert group["member_ids"], f"{group_id} 没有成员"
        assert group["source_refs"], f"{group_id} 没有来源"
        for item_id in group["member_ids"]:
            assert item_id in collectibles["items"], f"{group_id} 成员 {item_id} 不在条目表"
        for ref in group["source_refs"]:
            assert ref in collectibles["sources"], f"{group_id} 来源 {ref} 不在来源表"
        if group_id.startswith("guide:"):
            _, region_id, category_id = group_id.split(":", 2)
            assert region_id == group["region_id"], group_id
            assert category_id in group["category_ids"], group_id
        if group["counter_keys"]:
            assert group["target_count"] == len(group["member_ids"]), group_id

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

    # 攻略组：参考类别内的组校验成员；有计数键的组校验目标数。
    for group_id, group in references["guide_groups"].items():
        if all(
            category_id in reference_set["reference_ids"]
            for category_id in group["category_ids"]
        ):
            union = set()
            for category_id in group["category_ids"]:
                union.update(reference_set["reference_ids"][category_id])
            for item_id in group["member_ids"]:
                assert item_id in union, f"{group_id} 成员 {item_id} 不在参考范围"
        if group["counter_keys"]:
            target = sum(
                counters["region_counters"][key]["source_total"]
                for key in group["counter_keys"]
            )
            assert group["target_count"] == target, (
                f"{group_id} 目标数 {group['target_count']} != {target}"
            )
        else:
            assert group["target_count"] is None, group_id

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
    catalog = load_json(CATALOG_PATH)

    documents = build_documents(dictionary, counter_entries, reference_groups, catalog)
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
        ]
        + [
            {
                "file": "data/catalog/collectibles_catalog.json",
                "sha256": sha256_file(CATALOG_PATH),
            }
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
