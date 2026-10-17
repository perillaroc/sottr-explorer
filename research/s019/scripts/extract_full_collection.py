# -*- coding: utf-8 -*-
"""Full-collection scope extract (2026-10-17 decision):
- story collectibles (Artifacts: Documents/Relics/Murals...) with ids/group/metricName
- per-region composition lists: challenge tombs / campsites / challenges / missions

Inputs are files produced by the other scripts in this directory (see ../README.md):
  <work>/extracted/dtp_0360E8.bin            (globalcollectibleinfo DTP, update3)
  <work>/extracted/gameflow_dtp_0360F4.bin   (globalgameflow DTP, update3)

Usage:
  py -3 extract_full_collection.py <work_dir> <out_dir>
Output: <out_dir>/full_collection_candidates.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

here = Path(__file__).parent
sys.path.insert(0, str(here))
from sottr_dtp import DtpFile  # noqa: E402


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        print("ERROR: work_dir and out_dir arguments are required (use out_dir outside the repo)")
        return 2
    work_dir = Path(sys.argv[1])
    out_dir = Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)

    dtp = DtpFile((work_dir / "extracted" / "dtp_0360E8.bin").read_bytes())
    root = dtp.ref_target(dtp.body_pos)

    def read_string(pos):
        t = dtp.ref_target(pos)
        return dtp.string_at(t) if t is not None else None

    story_target = dtp.ref_target(root)
    story_count = dtp.u32(root + 8)
    story = []
    for k in range(story_count):
        b = story_target + k * 168
        group = read_string(b + 16)
        tgt = dtp.ref_target(b + 152)
        cnt = dtp.u32(b + 160)
        for j in range(cnt):
            it = tgt + j * 496
            story.append({
                "id": f"{dtp.u32(it):08X}",
                "story": group,
                "metricName": read_string(it + 8),
                "nameText": read_string(it + 272),
                "regionId": f"{dtp.u32(it + 32):08X}",
            })
    print(f"story items: {len(story)} unique ids: {len({s['id'] for s in story})}")

    gameflow = DtpFile((work_dir / "extracted" / "gameflow_dtp_0360F4.bin").read_bytes())
    groot = gameflow.ref_target(gameflow.body_pos)
    num_regions = gameflow.u32(groot + 64)
    region_target = gameflow.ref_target(groot + 72)

    def u32_list(ref_pos, count_pos):
        tgt = gameflow.ref_target(ref_pos)
        cnt = gameflow.u32(count_pos)
        if tgt is None or cnt == 0 or cnt > 4096:
            return []
        return [f"{gameflow.u32(tgt + 4 * i):08X}" for i in range(cnt)]

    def objective_list(ref_pos, count_pos):
        tgt = gameflow.ref_target(ref_pos)
        cnt = gameflow.u32(count_pos)
        if tgt is None or cnt == 0 or cnt > 4096:
            return []
        return [(gameflow.u32(tgt + 8 * i), gameflow.u32(tgt + 8 * i + 4)) for i in range(cnt)]

    regions = []
    for i in range(num_regions):
        b = region_target + i * 256
        regions.append({
            "regionId": f"{gameflow.u32(b):08X}",
            "uiUnitID": gameflow.i32(b + 68),
            "countsTowardPercentComplete": gameflow.u8(b + 41),
            "isRetraversable": gameflow.u8(b + 40),
            "isAChallengeTomb": gameflow.u8(b + 50),
            "isDLC": gameflow.u8(b + 52),
            "challengeTombs": u32_list(b + 176, b + 184),
            "campsites": u32_list(b + 192, b + 200),
            "challenges": objective_list(b + 208, b + 216),
            "missions": objective_list(b + 224, b + 232),
        })

    print(f"regions={len(regions)} "
          f"challengeTombs={sum(len(r['challengeTombs']) for r in regions)} "
          f"campsites={sum(len(r['campsites']) for r in regions)} "
          f"challenges={sum(len(r['challenges']) for r in regions)} "
          f"missions={sum(len(r['missions']) for r in regions)}")

    out = out_dir / "full_collection_candidates.json"
    out.write_text(json.dumps({"story": story, "regions": regions}, indent=2, ensure_ascii=False),
                   encoding="utf-8")
    print(f"-> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
