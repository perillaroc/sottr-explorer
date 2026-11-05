# -*- coding: utf-8 -*-
"""Verify that every required catalog item has a map or acquisition guidance.

Reads only committed files, so it runs in CI without the private game extraction:

  py -3 data/catalog/check_guidance.py

The gate is intentionally strict: the committed catalog must match a fresh merge of
``guidance.json``, and the coverage numbers are fixed. If the dataset changes, update
this file deliberately instead of letting coverage drift.
"""
from __future__ import annotations

import copy
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from guidance import (  # noqa: E402
    BASEMAPS_PATH,
    CATALOG_PATH,
    SOURCES,
    SCOPES,
    GuidanceError,
    apply_guidance,
    basemap_units,
    load_guidance,
    load_json,
    validate_guidance,
)

EXPECTED = {
    "required": 352,
    "mapped": 243,
    "collection": 32,
    "unit": 77,
    "item": 0,
    "uncovered": 0,
}


def main() -> int:
    errors: list[str] = []
    catalog = load_json(CATALOG_PATH)
    guidance = load_guidance()
    units_with_basemap = basemap_units()

    if catalog.get("catalog_schema_version") != "2.1.0":
        errors.append(
            f"catalog_schema_version={catalog.get('catalog_schema_version')!r}，期望 '2.1.0'"
        )
    if not catalog.get("catalog_version"):
        errors.append("catalog_version 为空")

    errors.extend(
        validate_guidance(
            guidance,
            catalog["items"],
            catalog["units"],
            units_with_basemap,
        )
    )

    # Validate the committed note fields, then re-merge into a copy and compare. The
    # comparison catches a stale catalog when guidance.json changes.
    for item_id, item in catalog["items"].items():
        note = item.get("acquisition_note")
        if note is None:
            continue
        if not isinstance(note, str) or not note.strip():
            errors.append(f"{item_id}: acquisition_note 为空")
        note_en = item.get("acquisition_note_en")
        if not isinstance(note_en, str) or not note_en.strip():
            errors.append(f"{item_id}: acquisition_note_en 为空")
        scope = item.get("acquisition_note_scope")
        if scope not in SCOPES:
            errors.append(f"{item_id}: acquisition_note_scope={scope!r} 非法")
        source = item.get("acquisition_note_source")
        if source not in SOURCES:
            errors.append(f"{item_id}: acquisition_note_source={source!r} 非法")

    merged_items = copy.deepcopy(catalog["items"])
    stats = apply_guidance(
        merged_items,
        guidance,
        catalog["regions"],
        catalog["units"],
        units_with_basemap,
    )
    for item_id, item in catalog["items"].items():
        merged = merged_items[item_id]
        for key in (
            "acquisition_note",
            "acquisition_note_en",
            "acquisition_note_scope",
            "acquisition_note_source",
        ):
            if item.get(key) != merged.get(key):
                errors.append(f"{item_id}: {key} 与 guidance.json 重新合并的结果不一致")

    for name, entry in guidance.get("collections", {}).items():
        declared = next(
            (record for record in catalog.get("collections", []) if record.get("name") == name),
            None,
        )
        if declared is None:
            errors.append(f"合集 {name}: catalog.collections 中没有记录")
            continue
        for key in ("display_zh", "display_en"):
            if declared.get(key) != entry.get(key):
                errors.append(f"合集 {name}: {key} 与 catalog 不一致")

    for key, expected in EXPECTED.items():
        if stats[key] != expected:
            errors.append(f"{key}={stats[key]}，期望 {expected}")

    print(
        "coverage: "
        + " ".join(f"{key}={stats[key]}" for key in EXPECTED)
    )
    if errors:
        for error in errors:
            print(f"FAIL {error}", file=sys.stderr)
        print(f"check_guidance failed: {len(errors)} 项", file=sys.stderr)
        return 1
    print("check 通过：必收条目全部有地图或文字指引")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except GuidanceError as error:
        print(f"FAIL {error}", file=sys.stderr)
        raise SystemExit(1)
