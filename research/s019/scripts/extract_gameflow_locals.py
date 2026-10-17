# -*- coding: utf-8 -*-
"""SX-019 step 3: extract highest-priority globalgameflow.drm, parse regions and
unit maps, locate/parse locals.bin from language archives, resolve name keys.

Usage:
  py -3 extract_gameflow_locals.py <game_dir> [out_dir]
"""
from __future__ import annotations

import hashlib
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from sottr_dtp import (  # noqa: E402
    TigerArchive, ArchiveRegistry, parse_drm, DtpFile, fnv1a64)

GAMEFLOW = "pcx64-w\\globalgameflow.drm"
LOCALS = "pcx64-w\\local\\locals.bin"
FAMILIES = ["bigfile", "bigfile.update1.002", "bigfile.update2.002", "bigfile.update3.002"]
TEXT_LANGS = ["english", "simplechinese"]


def parse_locals(data: bytes) -> dict:
    language_id, num_strings = struct.unpack_from("<ii", data, 0)
    strings: dict[str, str] = {}
    for i in range(num_strings):
        key_pos = struct.unpack_from("<i", data, 8 + 8 * i)[0]
        if key_pos == 0:
            continue
        end = data.index(b" ", key_pos)
        key = data[key_pos:end].decode("utf-8", "replace")
        value_end = data.index(b"\0", end + 1)
        value = data[end + 1:value_end].decode("utf-8", "replace")
        strings[key] = value
    return {"languageId": language_id, "numStrings": num_strings, "strings": strings}


def parse_regions(dtp: DtpFile):
    root = dtp.ref_target(dtp.body_pos)
    if root is None:
        raise ValueError("gameflow root not internal")
    num_campsites, num_unitmaps, num_regions = dtp.u32(root), dtp.u32(root + 16), dtp.u32(root + 64)
    print(f"GameFlow: campsites={num_campsites} unitMaps={num_unitmaps} regions={num_regions}")
    entries = []
    target = dtp.ref_target(root + 72)
    for i in range(num_regions):
        b = target + i * 256  # RegionData inline size (ImagePath=72 with actionRef; stride verified by ref table)
        entries.append({
            "regionId": f"{dtp.u32(b):08X}",
            "debugName": dtp.string_at(dtp.ref_target(b + 8)) if dtp.ref_target(b + 8) else None,
            "regionProgressionIndex": dtp.i32(b + 16),
            "regionLocalNameKey": f"{dtp.u64(b + 24):016X}",
            "regionOverrideNameKey": f"{dtp.u64(b + 32):016X}",
            "isRetraversable": dtp.u8(b + 40),
            "countsTowardPercentComplete": dtp.u8(b + 41),
            "hideOnMap": dtp.u8(b + 48),
            "isAChallengeTomb": dtp.u8(b + 50),
            "isDLC": dtp.u8(b + 52),
            "coordinateX": dtp.f32(b + 60),
            "coordinateY": dtp.f32(b + 64),
            "uiUnitID": dtp.i32(b + 68),
        })
    unitmaps = []
    target = dtp.ref_target(root + 24)
    for i in range(num_unitmaps):
        b = target + i * 272  # MapData inline size (2x ImagePath=72; stride verified by ref table)
        secondary_target = dtp.ref_target(b + 240)
        secondary = []
        if secondary_target is not None:
            for k in range(dtp.u32(b + 236)):
                secondary.append(f"{dtp.u32(secondary_target + k * 4):08X}")
        unitmaps.append({
            "unitMapHashedString": f"{dtp.u32(b):08X}",
            "uiUnitID": dtp.i32(b + 4),
            "progressionOrder": dtp.i32(b + 8),
            "markerUnit": dtp.u16(b + 12),
            "primaryRegion": f"{dtp.u32(b + 232):08X}",
            "numSecondaryRegions": dtp.u32(b + 236),
            "secondaryRegions": secondary,
            "debugName": dtp.string_at(dtp.ref_target(b + 248)) if dtp.ref_target(b + 248) else None,
        })
    return entries, unitmaps


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        print("ERROR: output directory argument is required (use one outside the repo)")
        return 2
    game_dir = Path(sys.argv[1])
    out_dir = Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    registry = ArchiveRegistry(game_dir)
    report: dict = {}

    # --- gameflow (highest priority family) ---
    copies = []
    for fam in FAMILIES:
        archive = TigerArchive(game_dir / f"{fam}.000.tiger")
        copies.extend((fam, archive, e) for e in archive.find(GAMEFLOW))
    assert copies, "globalgameflow.drm not found"
    fam, archive, entry = copies[-1]
    drm = parse_drm(archive.read_file(entry), Path(GAMEFLOW))
    print(f"gameflow from {fam} ({len(drm.identifications)} resources)")
    best = None
    for i, (ident, loc) in enumerate(zip(drm.identifications, drm.locations)):
        if ident.type == 7:
            payload, how = registry.read_resource(ident, loc)
            print(f"  [{i}] id={loc.id:06X} sub={ident.sub_type} payload={len(payload)} ({how})")
            if best is None or len(payload) > len(best[2]):
                best = (i, ident, payload, how, loc)
    i, ident, payload, how, loc = best
    (out_dir / f"gameflow_dtp_{loc.id:06X}.bin").write_bytes(payload)
    dtp = DtpFile(payload)
    print(f"main DTP id={loc.id:06X} refdefs internal={dtp.num_internal} "
          f"wide={dtp.num_wide} packed={dtp.num_packed}")
    regions, unitmaps = parse_regions(dtp)
    report["gameflow"] = {"family": fam, "dtpId": f"{loc.id:06X}",
                          "sha256_16": hashlib.sha256(payload).hexdigest()[:16],
                          "regions": regions, "unitMaps": unitmaps}
    for r in regions:
        print(f"  region {r['regionId']} unit={r['uiUnitID']} "
              f"debug={r['debugName']!r} local={r['regionLocalNameKey']} "
              f"override={r['regionOverrideNameKey']} pct={r['countsTowardPercentComplete']}")

    # --- locals.bin (per language, highest priority) ---
    locals_report = {}
    for lang in TEXT_LANGS:
        families = [f"bigfile_{lang}_allvo_allplt",
                    f"bigfile.update1.002_{lang}_allvo_allplt",
                    f"bigfile.update2.002_{lang}_allvo_allplt",
                    f"bigfile.update3.002_{lang}_allvo_allplt"]
        hits = []
        for fam in families:
            base = game_dir / f"{fam}.000.tiger"
            if not base.exists():
                continue
            archive = TigerArchive(base)
            hits.extend((fam, archive, e) for e in archive.find(LOCALS))
        print(f"locals[{lang}]: {[(h[0], h[2].part, h[2].offset, h[2].size1) for h in hits]}")
        if not hits:
            continue
        fam, archive, entry = hits[-1]
        data = archive.read_file(entry)
        path = out_dir / f"locals_{lang}.bin"
        path.write_bytes(data)
        parsed = parse_locals(data)
        hashed = {}
        for key, value in parsed["strings"].items():
            hashed.setdefault(fnv1a64(key), value)
        for r in regions:
            for field in ("regionLocalNameKey", "regionOverrideNameKey"):
                text = hashed.get(int(r[field], 16))
                if text:
                    r[field + "Text"] = text
        locals_report[lang] = {
            "family": fam, "languageId": parsed["languageId"],
            "numStrings": parsed["numStrings"],
            "sha256_16": hashlib.sha256(data).hexdigest()[:16],
        }
        print(f"locals[{lang}] {fam}: languageId={parsed['languageId']} "
              f"strings={parsed['numStrings']} parsed={len(parsed['strings'])}")
    report["locals"] = locals_report
    print("region names after resolution:")
    for r in regions:
        print(f"  {r['regionId']} -> local={r.get('regionLocalNameKeyText')!r} "
              f"override={r.get('regionOverrideNameKeyText')!r}")

    (out_dir / "gameflow_report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"report -> {out_dir / 'gameflow_report.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
