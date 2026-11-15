# -*- coding: utf-8 -*-
"""Validate the per-item guide mapping dataset.

Reads only committed files, so CI does not need the private game extraction:

  py -3 data/catalog/check_guides.py
  py -3 data/catalog/check_guides.py --worklist   # print required items still lacking a verified link

Only ``scope=item`` + ``confidence=verified`` refs are merged into metadata as
``exact_guide_item`` locations. ``inferred`` / ``coverage_only`` refs are kept for
the curation record but never become product links.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CATALOG_PATH = ROOT / "data" / "catalog" / "collectibles_catalog.json"
METADATA_PATH = ROOT / "data" / "metadata" / "collectibles.json"
GUIDES_PATH = HERE / "guides.json"

# Deliberate coverage gate: update these numbers only when a new batch is committed.
EXPECTED = {
    "items": 352,
    "refs": 352,
    "verified_item": 352,
    "required_with_refs": 352,
}

SCOPES = {"item", "collection", "unit"}
CONFIDENCE = {"verified", "inferred", "coverage_only"}
DATE_RE = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$")


def load(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def main() -> int:
    catalog = load(CATALOG_PATH)
    metadata = load(METADATA_PATH)
    guides = load(GUIDES_PATH)
    errors: list[str] = []

    if guides.get("schema_version") != "1.0.0":
        errors.append(f"schema_version={guides.get('schema_version')!r}，期望 '1.0.0'")
    if not guides.get("guide_version"):
        errors.append("guide_version 为空")

    known_sources = set(metadata.get("sources", {}))
    known_sources.update(guides.get("sources", {}))
    catalog_items = catalog.get("items", {})
    required_ids = {
        item_id for item_id, item in catalog_items.items() if item.get("required_for_100")
    }

    refs_total = 0
    verified_item = 0
    required_with_refs: set[str] = set()
    seen: set[tuple[str, str, str]] = set()

    for item_id, refs in guides.get("items", {}).items():
        if item_id not in catalog_items:
            errors.append(f"{item_id}: 图鉴中没有该条目")
            continue
        if not isinstance(refs, list) or not refs:
            errors.append(f"{item_id}: refs 必须是非空数组")
            continue
        for index, ref in enumerate(refs):
            where = f"{item_id}[{index}]"
            if not isinstance(ref, dict):
                errors.append(f"{where}: 必须是对象")
                continue
            source_id = ref.get("source_id")
            if source_id not in known_sources:
                errors.append(f"{where}: 来源 {source_id!r} 不存在")
            locator = ref.get("locator")
            if not isinstance(locator, str) or not locator.strip() or "\n" in locator:
                errors.append(f"{where}: locator 必须是非空单行文本")
            scope = ref.get("scope")
            if scope not in SCOPES:
                errors.append(f"{where}: scope={scope!r} 非法")
            confidence = ref.get("confidence")
            if confidence not in CONFIDENCE:
                errors.append(f"{where}: confidence={confidence!r} 非法")
            verified_at = ref.get("verified_at")
            if not isinstance(verified_at, str) or not DATE_RE.match(verified_at):
                errors.append(f"{where}: verified_at 必须是 YYYY-MM-DD")
            region_id = ref.get("region_id")
            if region_id is not None:
                slugs = {
                    entry.get("slug")
                    for entry in catalog.get("regions", [])
                    if entry.get("slug")
                }
                if region_id not in slugs:
                    errors.append(f"{where}: region_id={region_id!r} 不在图鉴区域中")
            key = (item_id, str(source_id), str(locator))
            if key in seen:
                errors.append(f"{where}: 重复引用 {key}")
            seen.add(key)
            refs_total += 1
            if scope == "item" and confidence == "verified":
                verified_item += 1
                if item_id in required_ids:
                    required_with_refs.add(item_id)

    stats = {
        "items": len(guides.get("items", {})),
        "refs": refs_total,
        "verified_item": verified_item,
        "required_with_refs": len(required_with_refs),
    }
    for key, expected in EXPECTED.items():
        if stats[key] != expected:
            errors.append(f"{key}={stats[key]}，期望 {expected}")

    print(
        "guides: "
        + " ".join(f"{key}={value}" for key, value in stats.items())
        + f" required_total={len(required_ids)}"
        + f" uncovered_required={len(required_ids - required_with_refs)}"
    )

    if "--worklist" in sys.argv[1:]:
        text_only = "--text-only" in sys.argv[1:]
        pending = sorted(required_ids - required_with_refs)
        if text_only:
            pending = [
                item_id
                for item_id in pending
                if catalog_items[item_id].get("acquisition_note_scope")
                in ("collection", "unit")
            ]
        label = "text-only (collection/unit)" if text_only else "all"
        print(f"\nworklist: required items without a verified item link ({label}), {len(pending)} items")
        for item_id in pending:
            item = catalog_items[item_id]
            region = item.get("region_slug")
            unit = item.get("unit")
            category = item.get("category")
            subcategory = item.get("subcategory")
            note_scope = item.get("acquisition_note_scope")
            print(
                f"{item_id}\t{region}\t{category}"
                f"{'/' + subcategory if subcategory else ''}\tunit={unit}"
                f"\tnote={note_scope}"
            )

    if errors:
        for error in errors:
            print(f"FAIL {error}", file=sys.stderr)
        print(f"check_guides failed: {len(errors)} 项", file=sys.stderr)
        return 1
    print("check 通过：guides.json 结构、来源与覆盖计数一致")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
