# -*- coding: utf-8 -*-
"""SX-019 step 1: extract the highest-priority globalcollectibleinfo.drm and
report/compare its DTP payloads across archive families.

Usage:
  py -3 extract_collectibleinfo.py <game_dir> [out_dir]
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from sottr_dtp import (  # noqa: E402
    TigerArchive, ArchiveRegistry, parse_drm, DtpFile)

DRM_PATH = "pcx64-w\\globalcollectibleinfo.drm"
FAMILIES = ["bigfile", "bigfile.update1.002", "bigfile.update2.002", "bigfile.update3.002"]


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        print("ERROR: output directory argument is required (use one outside the repo)")
        return 2
    game_dir = Path(sys.argv[1])
    out_dir = Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    registry = ArchiveRegistry(game_dir)

    copies = []
    for fam in FAMILIES:
        archive = TigerArchive(game_dir / f"{fam}.000.tiger")
        for entry in archive.find(DRM_PATH):
            copies.append((fam, archive, entry))
    print(f"copies: {[(c[0], c[2].part, c[2].offset, c[2].size1) for c in copies]}")

    report: dict = {"copies": [], "dtpByFamily": {}}
    for fam, archive, entry in copies:
        drm_bytes = archive.read_file(entry)
        drm = parse_drm(drm_bytes, Path(DRM_PATH))
        row = {"family": fam, "sha256": hashlib.sha256(drm_bytes).hexdigest(),
               "resources": []}
        for i, (ident, loc) in enumerate(zip(drm.identifications, drm.locations)):
            payload, how = registry.read_resource(ident, loc)
            digest = hashlib.sha256(payload).hexdigest()[:16]
            row["resources"].append({
                "index": i, "type": ident.type, "id": f"{loc.id:06X}",
                "sub_type": ident.sub_type,
                "refdefs": ident.ref_definitions_size, "body": ident.body_size,
                "payload": len(payload), "stored": how,
                "target": f"archive{loc.archive_id}/part{loc.part}@{loc.offset:X}+{loc.size}",
                "sha256_16": digest,
            })
            if ident.type == 7:
                report["dtpByFamily"].setdefault(loc.id, {})[fam] = digest
            print(f"  {fam} [{i}] type={ident.type} id={loc.id:06X} payload={len(payload)} "
                  f"sha={digest} stored={how}")
            if fam == copies[-1][0] and ident.type == 7 and len(payload) > 40:
                (out_dir / f"dtp_{loc.id:06X}.bin").write_bytes(payload)
        report["copies"].append(row)

    report["differ"] = {
        f"{rid:06X}": len(set(v.values())) > 1
        for rid, v in report["dtpByFamily"].items()
    }
    print("payload differences across families:", report["differ"])
    (out_dir / "collectibleinfo_report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"report -> {out_dir / 'collectibleinfo_report.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
