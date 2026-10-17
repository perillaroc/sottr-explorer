# -*- coding: utf-8 -*-
"""SX-022 派生表生成：区域绑定、区域名、单位名、本地化字符串。

只读消费：
  <work>/extracted/dtp_0360E8.bin            globalcollectibleinfo DTP（update3）
  <work>/extracted/locals_english.bin        locals.bin（英文，allvo_allplt）
  <work>/extracted/locals_simplechinese.bin  locals.bin（简中，allvo_allplt）
  <trreboot>/Templates/010 Editor/tr/tr11/lists/tr11zones.bt   （MIT）
  data/metadata/region_counters.json         区域计数器来源目标（区域绑定签名）

产出（随产品分发的事实性映射，进入仓库）：
  research/s022/data/region_bindings.json   区域哈希 → ASL slug + 签名证据
  research/s022/data/region_names.json      slug/特殊区域 → 中英显示名
  research/s022/data/unit_names.json        unit id → zone 名 + 中英显示名
  research/s022/data/item_names.json        条目 ID → 中英名（故事条目）
  research/s022/data/locale_summary.json    对齐统计（供复核）

用法：
  py -3 research/s022/scripts/build_derived_tables.py <work_dir> <trreboot_tools_dir> [out_dir]
"""
from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(REPO / "research" / "s019" / "scripts"))
from sottr_atlas import (  # noqa: E402
    NONSTORY_TYPE_CATEGORY, STORY_TYPE_SUBCATEGORY,
    align_locales, bind_regions, load_locals, load_source_targets_from_metadata,
    load_zone_names, parse_nonstory_items, parse_story_items,
)
from sottr_dtp import DtpFile  # noqa: E402

#: 无存档计数器、但仍有条目的特殊区域（区域绑定不适用，单独成组）
SPECIAL_REGIONS = {
    "26694CD8": {
        "slug": "_challenge_tombs",
        "name_en": "CHALLENGE TOMBS",
        "name_zh": "挑战古墓",
        "name_source": "curated",
        "note": "游戏数据里的挑战古墓聚合区域（regionProgressionIndex=999、hideOnMap=1）；"
                "不带存档区域计数器，按子关卡（各古墓）展示",
    },
    "29E50C12": {
        "slug": "_special",
        "name_en": "SPECIAL / CROSS-REGION",
        "name_zh": "特殊 / 跨区域",
        "name_source": "curated",
        "note": "游戏数据里的特殊区域（不在 26 区主表内）；"
                "含 Path of the Stars、Lara's Notebook 等跨区域条目",
    },
    "669D3CB3": {
        "slug": "croft_manor",
        "name_en": "CROFT MANOR",
        "name_zh": "克劳馥庄园",
        "name_source": "game_locale",
        "locale_key": "regionname_cm_croftmanor",
        "note": "Prologue 庄园区域，不参与 100%",
    },
}

#: 9 个主区域 → 游戏区域显示名键（键名来自 locals.bin，值为官方中英名）
SLUG_REGION_NAME_KEY = {
    "paititi": "regionname_pa_paititi02",
    "cozumel": "regionname_dd_day_of_dead",
    "cenote": "regionname_ce_cenotes",
    "kuwaq_yaku": "regionname_hv_hollow_village",
    "mission_of_san_juan": "regionname_sm_spanish_mission",
    "peruvian_jungle": "regionname_lj_lost_in_the_jungle",
    "porvenir_oil_fields": "regionname_an_apocalypse_2",
    "trial_of_the_eagle": "regionname_ct_citadel_of_wind",
    "head_of_the_serpent": "regionname_ct_devil_maw",
}

#: zone 名前缀 → 主区域 slug（用于把条目落到区域，也用于单位归属）
ZONE_PREFIX_SLUG = {
    "pa_hub_paititi": "paititi",
    "pa_paititi": "paititi",
    "hv_hollow_village": "kuwaq_yaku",
    "sm_spanish_mission": "mission_of_san_juan",
    "lj_lost_in_the_jungle": "peruvian_jungle",
    "dd_day_of_the_dead": "cozumel",
    "ce_0": "cenote",
    "an_apocalypse": "porvenir_oil_fields",
    "av_abandonned_village": "peruvian_jungle",
}

#: unit 显示名人工补全（zone 名匹配不到 `regionname_*` 时使用；标 curated）
UNIT_DISPLAY_OVERRIDES = {
    "pa_hub_paititi": ("PAITITI HUB", "帕依提提主城"),
    "pa_hub_paititi_return_02": ("PAITITI RETURN", "帕依提提 · 归途"),
    "pa_hub_paititi_middle_island": ("MIDDLE ISLAND", "帕依提提 · 中岛"),
    "pa_hub_paititi_wilderness": ("PAITITI WILDERNESS", "帕依提提荒野"),
    "pa_hub_paititi_cr02": ("PAITITI CRYPT 02", "帕依提提墓穴 02"),
    "pa_hub_paititi_kings_tomb_dlc2": ("KING'S TOMB (DLC2)", "王之墓（DLC2）"),
    "sm_spanish_mission_village": ("SAN JUAN VILLAGE", "圣胡安村庄"),
    "sm_spanish_mission_cr04": ("SAN JUAN CRYPT 04", "圣胡安墓穴 04"),
    "sm_spanish_mission_crypt_003": ("SAN JUAN CRYPT 003", "圣胡安墓穴 003"),
    "hv_hollow_village_v2_01": ("KUWAQ YAKU", "库阿克雅库"),
    "hv_hollow_village_v2_02": ("KUWAQ YAKU", "库阿克雅库"),
    "hv_hollow_village_v2_03": ("KUWAQ YAKU", "库阿克雅库"),
    "lj_lost_in_the_jungle_v2_02": ("PERUVIAN JUNGLE", "秘鲁丛林"),
    "lj_lost_in_the_jungle_v2_03": ("PERUVIAN JUNGLE", "秘鲁丛林"),
    "lj_lost_in_the_jungle_v2_04": ("PERUVIAN JUNGLE", "秘鲁丛林"),
    "lj_lost_in_the_jungle_v2_connector_out": ("JUNGLE PASSAGE", "丛林通道"),
    "dd_day_of_the_dead_020": ("COZUMEL", "科祖梅尔"),
    "dd_day_of_the_dead_030": ("COZUMEL", "科祖梅尔"),
    "dd_day_of_the_dead_040": ("COZUMEL", "科祖梅尔"),
    "ce_010_fall": ("CENOTE FALLS", "灰岩坑 · 瀑布"),
    "ce_020_camp": ("CENOTE CAMP", "灰岩坑 · 营地"),
    "ce_030_den": ("CENOTE DEN", "灰岩坑 · 兽穴"),
    "ce_040_tomb": ("CENOTE TOMB", "灰岩坑 · 墓室"),
    "av_abandonned_village": ("ABANDONED VILLAGE", "荒废村庄"),
    "an_apocalypse_now_v2_03": ("RIVER BANK", "河岸"),
    "cr_el_dorado": ("EL DORADO", "黄金城"),
    "cr_lastbreath": ("LAST BREATH", "最后之息"),
    "cr_newfall_01": ("NEWFALL", "新生"),
    "cr_nepenthes": ("NEPENTHES", "猪笼草"),
    "crypt_009": ("CRYPT 09", "墓穴 09"),
    "dm_devils_maw": ("DEVIL'S MAW", "巨蛇之首"),
    "cm_croft_manor": ("CROFT MANOR", "克劳馥庄园"),
    "gym_outcasts": ("GYM: OUTCASTS", "训练场 · 流放者"),
    "gym_social_hub": ("GYM: SOCIAL HUB", "训练场 · 社交中心"),
    "gym_collectibles": ("GYM: COLLECTIBLES", "训练场 · 收集品"),
    "cr_dlc06_desecrated_site": ("DESECRATED SITE (DLC6)", "亵渎之地（DLC6）"),
    "dlc07_spanish_mission_village": ("SAN JUAN VILLAGE (DLC7)", "圣胡安村庄（DLC7）"),
    # 挑战古墓与联机关卡（有官方区域名的在 ZONE_REGION_NAME_KEY，其余在此补齐）
    "ct_gift_of_immortality": ("GIFT OF IMMORTALITY", "不朽之礼"),
    "ct_gift_of_immortality_connector": ("GIFT OF IMMORTALITY PASSAGE", "不朽之礼通道"),
    "ct_pillars_of_heaven": ("PILLARS OF HEAVEN", "天柱"),
    "ct_pillars_of_heaven_connector": ("PILLARS OF HEAVEN PASSAGE", "天柱通道"),
    "ct_court_of_death_connector": ("JUDGE'S GAZE PASSAGE", "审判凝视通道"),
    "ct_flooded_sanctuary_connector": ("THIRSTY GODS PASSAGE", "诸神之欲念通道"),
    "ct_gate_of_xibalba_connector": ("UNDERWORLD GATE PASSAGE", "冥界之门通道"),
    "ct_serpents_hollow_connector": ("ANCIENT AQUEDUCT PASSAGE", "古老水道通道"),
    "ct_warriors_trial_connector": ("PATH OF BATTLE PASSAGE", "战斗之路通道"),
    "ct_xolotl_connector": ("HOWLING CAVES PASSAGE", "呼啸洞穴通道"),
    "ct_spanish_galleon_connector": ("SAN CORDOBA PASSAGE", "圣科尔多瓦通道"),
    "ct_coop_firemountain": ("CO-OP TOMB: FIRE MOUNTAIN", "联机古墓 · 火之山"),
    "ct_coop_giant_statue": ("CO-OP TOMB: GIANT STATUE", "联机古墓 · 巨像"),
    "ct_coop_music_box": ("CO-OP TOMB: MUSIC BOX", "联机古墓 · 八音盒"),
    "ct_coop_the_caiman": ("CO-OP TOMB: THE CAIMAN", "联机古墓 · 凯门鳄"),
    "ct_coop_water_ride": ("CO-OP TOMB: WATER RIDE", "联机古墓 · 水道"),
    "ct_citadel_of_wind_connector": ("TRIAL OF THE EAGLE PASSAGE", "猎鹰的试炼通道"),
    "an_apocalypse_now_v2_01": ("RIVER BANK", "河岸"),
    "ct_tree_of_life_connector": ("TREE OF LIFE PASSAGE", "生命之树通道"),
}

#: 挑战古墓/联机关卡 zone → 官方区域名键（用于取中英显示名）
ZONE_REGION_NAME_KEY = {
    "ct_citadel_of_wind": "regionname_ct_citadel_of_wind",
    "ct_warriors_trial": "regionname_ct_warriors_trial",
    "ct_court_of_death": "regionname_ct_court_of_death",
    "ct_serpents_hollow": "regionname_ct_serpents_hollow",
    "ct_tree_of_life": "regionname_ct_tree_of_life",
    "ct_flooded_sanctuary": "regionname_ct_flooded_sanctuary",
    "ct_gate_of_xibalba": "regionname_ct_gate_xibalba",
    "ct_xolotl": "regionname_ct_xolotl",
    "ct_sky_temple_v2": "regionname_ct_sky_temple",
    "ct_spanish_galleon": "regionname_ct_spanish_galleon",
}


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    work = Path(sys.argv[1])
    trreboot = Path(sys.argv[2])
    out_dir = Path(sys.argv[3]) if len(sys.argv) > 3 else HERE.parent / "data"
    out_dir.mkdir(parents=True, exist_ok=True)

    dtp = DtpFile((work / "extracted" / "dtp_0360E8.bin").read_bytes())
    story = parse_story_items(dtp)
    nonstory = parse_nonstory_items(dtp)
    print(f"story items={len(story)} nonstory items={len(nonstory)}")

    # ---- 本地化：英/简中按表下标对齐 ----
    en_entries = load_locals(work / "extracted" / "locals_english.bin")
    zh_entries = load_locals(work / "extracted" / "locals_simplechinese.bin")
    by_key: dict[str, tuple[str, str]] = {}
    mismatched = 0
    for (en_key, en_val), (zh_key, zh_val) in zip(en_entries, zh_entries):
        if not en_key or not zh_key:
            continue
        if en_key != zh_key:
            mismatched += 1
            continue
        by_key.setdefault(en_key, (en_val, zh_val))
    text_map = align_locales(en_entries, zh_entries)
    text_map.pop("__keys_mismatch__", None)
    print(f"locals: en={len(en_entries)} zh={len(zh_entries)} "
          f"keys={len(by_key)} key_mismatch={mismatched} "
          f"text_pairs={len(text_map)}")

    # ---- 单位命名 ----
    zone_names = load_zone_names(
        trreboot / "Templates" / "010 Editor" / "tr" / "tr11" / "lists" / "tr11zones.bt")
    used_units = sorted({row["unit"] for row in story + nonstory if row["unit"]})
    unit_names: dict[str, dict] = {}
    missing_zone = []
    for unit in used_units:
        zone = zone_names.get(unit)
        if not zone:
            missing_zone.append(unit)
        name_en = name_zh = None
        source = None
        key = ZONE_REGION_NAME_KEY.get(zone or "")
        if key and key in by_key:
            name_en, name_zh = by_key[key]
            source = "game_locale"
        elif zone in UNIT_DISPLAY_OVERRIDES:
            name_en, name_zh = UNIT_DISPLAY_OVERRIDES[zone]
            source = "curated"
        else:
            name_en = zone
            source = "zone_name"
        unit_names[str(unit)] = {
            "zone": zone,
            "display_en": name_en,
            "display_zh": name_zh,
            "name_source": source,
            "region_slug": next((slug for prefix, slug in ZONE_PREFIX_SLUG.items()
                                 if zone and zone.startswith(prefix)), None),
        }
    print(f"units={len(unit_names)} zone_missing={missing_zone} "
          f"named={sum(1 for v in unit_names.values() if v['display_zh'])}")

    # ---- 区域绑定（签名匹配）----
    catalog_shaped = []
    for row in story:
        catalog_shaped.append({
            "id": row["id"],
            "region_hash": row["region_hash"],
            "is_dlc": row["is_dlc"],
            "signature_category": STORY_TYPE_SUBCATEGORY.get(row["type"]),
        })
    for row in nonstory:
        category = NONSTORY_TYPE_CATEGORY.get(row["type"])
        if "gym" in (row["metric_name"] or "").lower():
            category = "gym_training"
        catalog_shaped.append({
            "id": row["id"],
            "region_hash": row["region_hash"],
            "is_dlc": row["is_dlc"],
            "signature_category": category,
        })

    targets = load_source_targets_from_metadata(REPO / "data" / "metadata")
    bindings = bind_regions(catalog_shaped, targets, exclude=set(SPECIAL_REGIONS))
    for region_hash, binding in sorted(bindings.items()):
        print(f"  {region_hash} -> {binding['region_slug']:<22} "
              f"dist={binding['signature_distance']} runner_up={binding['runner_up_distance']}")

    # ---- 区域名 ----
    region_names: dict[str, dict] = {}
    for region_hash, binding in bindings.items():
        slug = binding["region_slug"]
        key = SLUG_REGION_NAME_KEY.get(slug)
        en, zh = by_key.get(key, (None, None))
        region_names[slug] = {
            "region_hash": region_hash,
            "name_en": en or slug,
            "name_zh": zh,
            "name_source": "game_locale" if zh else "slug_fallback",
            "locale_key": key,
        }
    region_names.update({entry["slug"]: dict(entry, region_hash=hash_)
                         for hash_, entry in SPECIAL_REGIONS.items()})
    for slug, entry in sorted(region_names.items()):
        print(f"  region {slug:<22} {entry.get('name_en')} / {entry.get('name_zh')}")

    # ---- 条目名（故事条目：游戏英文明文 + locals 官方中文）----
    item_names: dict[str, dict] = {}
    hit_prefix = hit_text = miss = 0
    for row in story:
        iid = f"0x{row['id']}"
        metric = (row["metric_name"] or "").lower()
        name_en = row["name_en"]
        name_zh = None
        source = None
        prefix = f"{metric}_title_" if metric else None
        if prefix:
            for key, (en, zh) in by_key.items():
                if key.startswith(prefix):
                    name_zh = zh
                    source = "game_locale"
                    hit_prefix += 1
                    break
        if name_zh is None and name_en:
            name_zh = text_map.get(name_en)
            if name_zh:
                source = "game_locale"
                hit_text += 1
        if name_zh is None:
            miss += 1
        item_names[iid] = {"name_en": name_en, "name_zh": name_zh, "name_source": source}
    print(f"story names: by_key_prefix={hit_prefix} by_text={hit_text} unresolved={miss}")

    # ---- 写出 ----
    (out_dir / "region_bindings.json").write_text(json.dumps({
        "source": "SX-022 区域绑定：目录区域×类别计数 vs 存档区域计数器来源目标（L1 最近邻）",
        "dimensions": list(bindings[next(iter(bindings))]["signature"].keys()) if bindings else [],
        "bindings": bindings,
    }, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    (out_dir / "region_names.json").write_text(
        json.dumps(region_names, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    (out_dir / "unit_names.json").write_text(
        json.dumps(unit_names, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    (out_dir / "item_names.json").write_text(
        json.dumps(item_names, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    summary = {
        "locals": {"english_entries": len(en_entries), "chinese_entries": len(zh_entries),
                   "aligned_keys": len(by_key), "key_mismatch": mismatched,
                   "text_pairs": len(text_map)},
        "units": {"total": len(unit_names),
                  "zone_resolved": len(unit_names) - len(missing_zone),
                  "zone_missing": missing_zone},
        "story_names": {"by_key_prefix": hit_prefix, "by_text": hit_text, "unresolved": miss},
        "regions": {"bound": len(bindings), "special": sorted(SPECIAL_REGIONS)},
    }
    (out_dir / "locale_summary.json").write_text(
        json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"written -> {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
