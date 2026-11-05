# -*- coding: utf-8 -*-
"""Acquisition guidance for the full-collection catalog (v0.2.0).

The catalog's ``acquisition_note`` fields are generated from this module:

* collection notes for required items without a world position (quest/story rewards);
* sub-level notes for items in units that have no basemap;
* optional per-item overrides.

The text is data-derived: counts and region lists are read from the catalog, and any
inference (e.g. "obtained through the collection's quest/story") is marked as curated
and paired with an explicit "not individually verified" sentence. No guide text is
copied.
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
GUIDANCE_PATH = HERE / "guidance.json"
BASEMAPS_PATH = REPO / "data" / "map" / "basemaps.json"
CATALOG_PATH = HERE / "collectibles_catalog.json"

SCOPES = ("item", "collection", "unit")
SOURCES = ("curated", "game_data")


class GuidanceError(ValueError):
    """Raised when the guidance file or the merged catalog violates an invariant."""


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_guidance(path: Path = GUIDANCE_PATH) -> dict:
    data = load_json(path)
    if not isinstance(data, dict):
        raise GuidanceError(f"{path}: 顶层必须是对象")
    return data


def basemap_units(path: Path = BASEMAPS_PATH) -> set[int]:
    data = load_json(path)
    units: set[int] = set()
    for entry in data.get("basemaps", {}).values():
        units.update(int(unit) for unit in entry.get("unit_ids", []))
    return units


def _region_order(regions: list[dict]) -> dict[str, int]:
    order: dict[str, int] = {}
    for index, region in enumerate(regions):
        slug = region.get("slug")
        if slug and slug not in order:
            order[slug] = index
    return order


def _region_label(region: dict | None, lang: str, slug: str) -> str:
    if not region:
        return "特殊/跨区域" if lang == "zh" else "special / cross-region"
    value = region.get("name_zh") if lang == "zh" else region.get("name_en")
    return value or slug


def _collection_items(items: dict, collection: str) -> list[dict]:
    return [item for item in items.values() if item.get("collection") == collection]


def _positioned_regions(items: list[dict], regions: list[dict]) -> list[str]:
    order = _region_order(regions)
    slugs = {
        item.get("region_slug")
        for item in items
        if item.get("position") and item.get("region_slug")
    }
    # Main regions first, then special/cross-region areas; both are shown so the
    # positioned count in the note always matches the listed regions.
    main = sorted(
        (slug for slug in slugs if slug and not slug.startswith("_")),
        key=lambda slug: order.get(slug, len(order)),
    )
    special = sorted(
        (slug for slug in slugs if slug and slug.startswith("_")),
        key=lambda slug: order.get(slug, len(order)),
    )
    return main + special


def _join_regions(slugs: list[str], regions: list[dict], lang: str) -> str:
    if not slugs:
        return "特殊/跨区域" if lang == "zh" else "special / cross-region"
    labels = []
    by_slug = {region.get("slug"): region for region in regions if region.get("slug")}
    for slug in slugs:
        labels.append(_region_label(by_slug.get(slug), lang, slug))
    return " / ".join(labels)


def _zh_tail(hint: str, limitation: str) -> str:
    hint = (hint or "").strip()
    if hint and not hint.endswith(("。", "！", "？", ".", "!", "?")):
        hint += "。"
    return f"{hint}{limitation}" if hint else limitation


def _en_tail(hint: str, limitation: str) -> str:
    hint = (hint or "").strip()
    return f"{hint} {limitation}" if hint else limitation


def _collection_note(entry: dict, others: list[dict], regions: list[dict]) -> tuple[str, str]:
    display_zh = entry.get("display_zh") or ""
    display_en = entry.get("display_en") or ""
    hint_zh = entry.get("hint_zh") or ""
    hint_en = entry.get("hint_en") or ""
    total = len(others)
    positioned = sum(1 for item in others if item.get("position"))
    region_slugs = _positioned_regions(others, regions)
    region_zh = _join_regions(region_slugs, regions, "zh")
    region_en = _join_regions(region_slugs, regions, "en")
    tail_zh = _zh_tail(hint_zh, "具体触发点未逐条核实。")
    tail_en = _en_tail(hint_en, "The exact trigger has not been individually verified.")

    if positioned:
        zh = (
            f"属于故事合集「{display_zh}」。本组共 {total} 件，其中 {positioned} 件在"
            f"{region_zh}有位置；本条没有世界坐标，随该合集对应的支线/剧情获得。{tail_zh}"
        )
        en = (
            f"Part of the story collection “{display_en}”. {positioned} of its {total} "
            f"entries have world positions in {region_en}; this entry has no world "
            f"coordinate and is obtained through the collection's quest/story. {tail_en}"
        )
    else:
        zh = (
            f"属于故事合集「{display_zh}」。本组 {total} 件都没有世界坐标，"
            f"随该合集对应的支线/剧情获得。{tail_zh}"
        )
        en = (
            f"Part of the story collection “{display_en}”. None of its {total} entries "
            f"has a world coordinate; they are obtained through the collection's "
            f"quest/story. {tail_en}"
        )
    return zh, en


def _unit_label(entry: dict, unit: int, lang: str) -> str:
    value = entry.get("display_zh") if lang == "zh" else entry.get("display_en")
    if value:
        return value
    zone = entry.get("zone")
    if zone:
        return zone
    return f"子关卡 {unit}" if lang == "zh" else f"Sub-level {unit}"


def _unit_note(entry: dict, unit: int, required_count: int, units: dict) -> tuple[str, str]:
    display_zh = _unit_label(units.get(str(unit), {}), unit, "zh")
    display_en = _unit_label(units.get(str(unit), {}), unit, "en")
    hint_zh = entry.get("hint_zh") or ""
    hint_en = entry.get("hint_en") or ""
    tail_zh = _zh_tail(hint_zh, "具体位置见下方攻略搜索。")
    tail_en = _en_tail(hint_en, "Use the guide search below for exact locations.")
    zh = (
        f"本子关卡「{display_zh}」暂无底图；本关有 {required_count} 件必收条目，"
        f"位置以关卡内相对坐标表示。{tail_zh}"
    )
    en = (
        f"The sub-level “{display_en}” has no basemap; it contains {required_count} "
        f"required items, shown as in-level relative coordinates. {tail_en}"
    )
    return zh, en


def _set_note(item: dict, zh: str, en: str, scope: str, source: str) -> None:
    item["acquisition_note"] = zh
    item["acquisition_note_en"] = en
    item["acquisition_note_scope"] = scope
    item["acquisition_note_source"] = source


def validate_guidance(
    guidance: dict,
    items: dict,
    units: dict,
    units_with_basemap: set[int],
) -> list[str]:
    """Validate keys and note shapes before merging; returns human-readable errors."""
    errors: list[str] = []
    source = guidance.get("source")
    if source not in SOURCES:
        errors.append(f"source 必须是 {SOURCES} 之一，实际 {source!r}")
    limitations = guidance.get("limitations")
    if not isinstance(limitations, list) or not all(isinstance(line, str) and line for line in limitations):
        errors.append("limitations 必须是非空字符串数组")

    collections = guidance.get("collections", {})
    if not isinstance(collections, dict):
        errors.append("collections 必须是对象")
        collections = {}
    for name, entry in collections.items():
        if not isinstance(entry, dict):
            errors.append(f"合集 {name}: 定义必须是对象")
            continue
        for key in ("display_zh", "display_en"):
            value = entry.get(key)
            if not isinstance(value, str) or not value.strip():
                errors.append(f"合集 {name}: {key} 必须是非空字符串")
        for key in ("hint_zh", "hint_en"):
            value = entry.get(key)
            if value is not None and not isinstance(value, str):
                errors.append(f"合集 {name}: {key} 必须是字符串或省略")
        others = _collection_items(items, name)
        if not others:
            errors.append(f"合集 {name}: 图鉴中没有条目")
        if not any(item.get("required_for_100") is True and not item.get("position") for item in others):
            errors.append(f"合集 {name}: 没有“必收 + 无坐标”条目，合集注记不会被使用")

    guidance_units = guidance.get("units", {})
    if not isinstance(guidance_units, dict):
        errors.append("units 必须是对象")
        guidance_units = {}
    for key, entry in guidance_units.items():
        if not str(key).isdigit():
            errors.append(f"unit {key!r}: 必须是数字字符串")
            continue
        unit = int(key)
        if str(unit) not in units:
            errors.append(f"unit {unit}: 图鉴中没有该子关卡")
        if unit in units_with_basemap:
            errors.append(f"unit {unit}: 已有底图，不应写子关卡注记")
        if not any(
            item.get("required_for_100") is True and item.get("unit") == unit
            for item in items.values()
        ):
            errors.append(f"unit {unit}: 没有必收条目，子关卡注记不会被使用")
        if not isinstance(entry, dict):
            errors.append(f"unit {unit}: 定义必须是对象")

    item_notes = guidance.get("items", {})
    if not isinstance(item_notes, dict):
        errors.append("items 必须是对象")
        item_notes = {}
    for item_id, entry in item_notes.items():
        if item_id not in items:
            errors.append(f"条目 {item_id}: 图鉴中没有该 ID")
        if not isinstance(entry, dict):
            errors.append(f"条目 {item_id}: 定义必须是对象")
            continue
        for key in ("text_zh", "text_en"):
            value = entry.get(key)
            if not isinstance(value, str) or not value.strip():
                errors.append(f"条目 {item_id}: {key} 必须是非空字符串")
    return errors


def apply_guidance(
    items: dict,
    guidance: dict,
    regions: list[dict],
    units: dict,
    units_with_basemap: set[int],
) -> dict:
    """Merge guidance into item fields; returns coverage statistics."""
    source = guidance.get("source") or "curated"
    collections = guidance.get("collections", {}) or {}
    guidance_units = guidance.get("units", {}) or {}
    item_notes = guidance.get("items", {}) or {}

    required_by_unit: dict[int, int] = {}
    for item in items.values():
        if item.get("required_for_100") is True and item.get("unit") not in (None, 0):
            unit = int(item["unit"])
            required_by_unit[unit] = required_by_unit.get(unit, 0) + 1

    collection_cache: dict[str, tuple[str, str]] = {}
    for name in collections:
        collection_cache[name] = _collection_note(collections[name], _collection_items(items, name), regions)

    unit_cache: dict[int, tuple[str, str]] = {}
    for key, entry in guidance_units.items():
        unit = int(key)
        unit_cache[unit] = _unit_note(entry, unit, required_by_unit.get(unit, 0), units)

    for item_id, item in items.items():
        item["acquisition_note"] = None
        item["acquisition_note_en"] = None
        item["acquisition_note_scope"] = None
        item["acquisition_note_source"] = None

        if item_id in item_notes:
            note = item_notes[item_id]
            _set_note(item, note["text_zh"], note["text_en"], "item", source)
            continue

        collection = item.get("collection")
        if not item.get("position") and collection in collection_cache:
            zh, en = collection_cache[collection]
            _set_note(item, zh, en, "collection", source)
            continue

        unit = item.get("unit")
        if unit not in (None, 0):
            unit = int(unit)
            if unit not in units_with_basemap and unit in unit_cache:
                zh, en = unit_cache[unit]
                _set_note(item, zh, en, "unit", source)

    return coverage(items, units_with_basemap)


def coverage(items: dict, units_with_basemap: set[int]) -> dict:
    stats = {
        "required": 0,
        "mapped": 0,
        "collection": 0,
        "unit": 0,
        "item": 0,
        "uncovered": 0,
    }
    for item in items.values():
        if item.get("required_for_100") is not True:
            continue
        stats["required"] += 1
        unit = item.get("unit")
        mapped = bool(item.get("position")) and unit not in (None, 0) and int(unit) in units_with_basemap
        if mapped:
            stats["mapped"] += 1
            continue
        if item.get("acquisition_note"):
            scope = item.get("acquisition_note_scope")
            if scope == "collection":
                stats["collection"] += 1
            elif scope == "unit":
                stats["unit"] += 1
            else:
                stats["item"] += 1
        else:
            stats["uncovered"] += 1
    return stats
