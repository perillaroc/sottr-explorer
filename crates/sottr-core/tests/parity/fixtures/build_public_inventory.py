#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""SX-010 公开夹具清单生成器。

从研究包公开样本证据汇总为仓库内清单 `public_samples.json`（仅公开数据：
下载 URL、ZIP 哈希、成员名与逐样本 SHA-256；不含私人路径或存档字节）。

用法（仓库根目录）：

    py -3 crates/sottr-core/tests/parity/fixtures/build_public_inventory.py \
        --sources plan/SOTTR_Save_Research/evidence/public_samples/sources.json \
        --analysis plan/SOTTR_Save_Research/evidence/public_samples/public_analysis.json \
        --out crates/sottr-core/tests/parity/fixtures/public_samples.json

    # 只校验现有清单与输入一致（不写文件）
    ... --check

输入为研究包证据（不随仓库分发；SX-010 执行时来自 SOTTR_Save_Research 交接包）：
    evidence/public_samples/sources.json        三个公开 ZIP 的 URL/哈希/成员
    evidence/public_samples/public_analysis.json 逐样本压缩与解压哈希、完成度候选、前缀计数

约定：
    - 20 个战役样本 = 三个归档中 save<number>.dat（14 + 1 + 5）；
    - challengetomb.dat 单列为额外条目（Python v0.2 名称模式不匹配，不在对照中）；
    - NG+ 归档的 profile.dat 未在公开分析证据中单列哈希，按名称匹配做容器层对照；
    - 排序固定：归档按 sources.json 顺序，样本按名称自然序。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

CHECK_KEYS = (
    "schema_note", "generated_from", "archives", "samples",
)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def natural_key(name: str):
    return [int(part) if part.isdigit() else part for part in re.split(r"(\d+)", name)]


def build(sources_path: Path, analysis_path: Path) -> dict:
    sources = json.loads(sources_path.read_text(encoding="utf-8"))
    analysis = json.loads(analysis_path.read_text(encoding="utf-8"))

    archives = []
    archive_order = []
    for entry in sources:
        name = Path(entry["url"]).name.removesuffix(".zip")
        archives.append({
            "name": name,
            "url": entry["url"],
            "sha256": entry["sha256"],
            "bytes": entry["bytes"],
            "members": entry["members"],
        })
        archive_order.append(name)

    samples = []
    for item in analysis:
        path = Path(item["path"])
        archive = path.parts[0]
        name = path.name
        if name == "challengetomb.dat":
            kind = "challenge_tomb"
            note = ("挑战古墓文件：Python v0.2 名称模式只识别 profile/save<number>，"
                    "因此该文件不在差分对照中（不是 20 个战役样本之一）。")
        else:
            kind = "campaign"
            note = None
        samples.append({
            "archive": archive,
            "relative_path": item["path"],
            "name": name,
            "kind": kind,
            "compressed_sha256": item["sha256"],
            "raw_sha256": item["raw_sha256"],
            "raw_length": item["raw_length"],
            "completion_candidate": item["completion_candidate"],
            "prefix_known": item["prefix_known"],
            "prefix_nonzero": item["prefix_nonzero"],
            "note": note,
        })

    # 公开分析证据未单列 NG+ 归档 profile.dat 的哈希；按名称匹配、容器层对照。
    sample_names = {sample["name"] for sample in samples}
    if "profile.dat" not in sample_names:
        for archive in archives:
            if "profile.dat" in archive["members"]:
                samples.append({
                    "archive": archive["name"],
                    "relative_path": f"{archive['name']}/profile.dat",
                    "name": "profile.dat",
                    "kind": "profile",
                    "compressed_sha256": None,
                    "raw_sha256": None,
                    "raw_length": None,
                    "completion_candidate": None,
                    "prefix_known": None,
                    "prefix_nonzero": None,
                    "note": ("公开 NG+ 归档的资料文件：公开分析证据未单列其哈希；"
                             "仅按名称匹配做容器/哈希原始观察对照，不计入 20 个战役样本。"),
                })

    order = {name: index for index, name in enumerate(archive_order)}
    samples.sort(key=lambda sample: (order.get(sample["archive"], 99),
                                     natural_key(sample["name"])))

    campaign_count = sum(sample["kind"] == "campaign" for sample in samples)
    assert campaign_count == 20, f"expected 20 campaign samples, got {campaign_count}"

    return {
        "schema_note": ("SX-010 公开夹具清单：仅公开数据（来源 URL、ZIP 哈希、逐样本"
                        "压缩/解压 SHA-256 与前缀计数）；存档字节不随仓库分发。"),
        "generated_from": {
            "sources": {"file": "evidence/public_samples/sources.json",
                        "sha256": sha256_bytes(sources_path.read_bytes())},
            "analysis": {"file": "evidence/public_samples/public_analysis.json",
                         "sha256": sha256_bytes(analysis_path.read_bytes())},
            "generator": "crates/sottr-core/tests/parity/fixtures/build_public_inventory.py",
        },
        "archives": archives,
        "samples": samples,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--analysis", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--check", action="store_true",
                        help="只校验现有输出与输入一致，不写文件")
    args = parser.parse_args()

    document = build(args.sources, args.analysis)
    rendered = json.dumps(document, ensure_ascii=False, indent=2) + "\n"

    if args.check:
        existing = json.loads(args.out.read_text(encoding="utf-8"))
        for key in CHECK_KEYS:
            if existing.get(key) != document[key]:
                print(f"CHECK FAILED: {key} 不一致", file=sys.stderr)
                return 1
        print(f"CHECK PASSED: {args.out} 与输入证据一致（"
              f"{sum(s['kind'] == 'campaign' for s in document['samples'])} 个战役样本）")
        return 0

    args.out.write_text(rendered, encoding="utf-8", newline="\n")
    campaign = sum(sample["kind"] == "campaign" for sample in document["samples"])
    print(f"WROTE {args.out}: {len(document['samples'])} 条（战役 {campaign}、"
          f"资料 {sum(s['kind'] == 'profile' for s in document['samples'])}、"
          f"挑战古墓 {sum(s['kind'] == 'challenge_tomb' for s in document['samples'])}）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
