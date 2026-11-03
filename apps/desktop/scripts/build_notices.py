# -*- coding: utf-8 -*-
"""许可通知与依赖许可全文生成。

输出两份随应用内嵌分发的离线文件：
- THIRD_PARTY_NOTICES.txt：来源、声明与依赖清单（关于页常驻展示）。
- THIRD_PARTY_LICENSES.txt：产品许可、仓库内第三方全文，以及每个依赖自带的
  LICENSE/COPYING/NOTICE 全文（按内容去重；无许可文件的包按项目所选许可补全）。

依赖工具：`cargo metadata`（Rust 依赖元数据与 registry 路径）与
`pnpm licenses list --prod --json`（生产 JavaScript 依赖及其路径）；
失败时对应小节标注为“待复核”，不虚构内容。

用法（仓库根目录）：
  py -3 apps/desktop/scripts/build_notices.py
"""
from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
OUT = REPO / "THIRD_PARTY_NOTICES.txt"
LICENSES_OUT = REPO / "THIRD_PARTY_LICENSES.txt"
MIT_TEXT = REPO / "licenses" / "TrRebootModTools-MIT.txt"
LUCIDE_TEXT = REPO / "licenses" / "lucide-ISC.txt"
PRODUCT_LICENSE = REPO / "LICENSE"

LICENSE_NAME = re.compile(r"(?i)^(licen[cs]e|copying|notice)")
MAX_LICENSE_BYTES = 300_000


def app_version() -> str:
    """应用版本的唯一来源：Cargo.toml 的 [workspace.package].version。"""
    cargo = (REPO / "Cargo.toml").read_text(encoding="utf-8")
    section = re.search(r"\[workspace\.package\](.*?)(?=\n\[|$)", cargo, re.S)
    match = re.search(r'^\s*version\s*=\s*"([^"]+)"', section.group(1) if section else "", re.M)
    return match.group(1) if match else "?"

# 依赖声明里常见的 SPDX 标识；顺序也是无许可文件时的选择优先级。
LICENSE_PREFERENCE = ["MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "ISC", "Zlib", "MPL-2.0"]

# 许可文本辨认指纹（顺序不可随意调整：BSD-3 必须先于 BSD-2，MPL 先于 MIT）。
FAMILIES: list[tuple[str, tuple[str, ...]]] = [
    ("Apache-2.0", ("Apache License", "Version 2.0")),
    ("MPL-2.0", ("Mozilla Public License Version 2.0",)),
    ("MIT-0", ("MIT No Attribution",)),
    ("MIT", ("Permission is hereby granted, free of charge",)),
    ("BSD-3-Clause", ("Redistribution and use in source and binary forms", "Neither the name")),
    ("BSD-2-Clause", ("Redistribution and use in source and binary forms",)),
    ("Zlib", ("This software is provided 'as-is'",)),
    ("ISC", ("Permission to use, copy, modify, and/or distribute this software",)),
    ("0BSD", ("THE SOFTWARE IS PROVIDED",)),
    ("Unlicense", ("This is free and unencumbered software",)),
    ("CC0-1.0", ("CC0 1.0 Universal",)),
    ("Unicode-3.0", ("UNICODE, INC. LICENSE AGREEMENT",)),
    ("LGPL-2.1", ("GNU LESSER GENERAL PUBLIC LICENSE",)),
    ("BSL-1.0", ("Boost Software License",)),
]

# 判断「这是完整许可正文」而非仅指向许可的短说明/SPDX 清单。
FULL_MARKERS = {
    "Apache-2.0": "TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION",
    "MPL-2.0": '1.1. "Contributor"',
    "MIT": "THE SOFTWARE IS PROVIDED",
    "MIT-0": "THE SOFTWARE IS PROVIDED",
    "BSD-3-Clause": "DISCLAIMED. IN NO EVENT SHALL",
    "BSD-2-Clause": "DISCLAIMED. IN NO EVENT SHALL",
    "ISC": "THE SOFTWARE IS PROVIDED",
    "0BSD": "THE SOFTWARE IS PROVIDED",
    "Zlib": "Permission is granted to anyone to use this software",
    "Unicode-3.0": "COPYRIGHT AND PERMISSION NOTICE",
    "Unlicense": "This is free and unencumbered software",
    "CC0-1.0": "CC0 1.0 Universal",
    "BSL-1.0": "Boost Software License",
    "LGPL-2.1": "GNU LESSER GENERAL PUBLIC LICENSE",
}

BODY_MARKERS = {
    "MIT": "Permission is hereby granted, free of charge",
    "BSD-3-Clause": "Redistribution and use in source and binary forms",
    "BSD-2-Clause": "Redistribution and use in source and binary forms",
    "Zlib": "This software is provided 'as-is'",
    "ISC": "Permission to use, copy, modify, and/or distribute this software",
    "0BSD": "Permission to use, copy, modify, and/or distribute this software",
}


@dataclass
class Dependency:
    ecosystem: str
    name: str
    version: str
    license: str
    authors: list[str] = field(default_factory=list)
    repository: str = ""
    files: list[tuple[str, str]] = field(default_factory=list)


def run(args: list[str], cwd: Path) -> tuple[int, str, str]:
    # Windows 上 pnpm 是 .cmd，subprocess 不经 PATHEXT 解析，需显式定位。
    executable = shutil.which(args[0])
    if executable is None:
        return 127, "", f"找不到可执行文件：{args[0]}"
    try:
        proc = subprocess.run(
            [executable, *args[1:]],
            cwd=cwd,
            capture_output=True,
            shell=False,
        )
        return (
            proc.returncode,
            proc.stdout.decode("utf-8", errors="replace"),
            proc.stderr.decode("utf-8", errors="replace"),
        )
    except OSError as error:
        return 127, "", str(error)


def read_text(path: Path) -> str:
    raw = path.read_bytes()
    for encoding in ("utf-8-sig", "utf-8", "latin-1"):
        try:
            return raw.decode(encoding).replace("\r\n", "\n").replace("\r", "\n").strip()
        except UnicodeDecodeError:
            continue
    return raw.decode("utf-8", errors="replace").replace("\r\n", "\n").strip()


def license_files(directory: Path) -> list[Path]:
    if not directory.is_dir():
        return []
    result: list[Path] = []
    try:
        for entry in sorted(directory.iterdir()):
            if not entry.is_file() or not LICENSE_NAME.match(entry.name):
                continue
            try:
                if entry.stat().st_size > MAX_LICENSE_BYTES:
                    continue
            except OSError:
                continue
            result.append(entry)
    except OSError:
        return []
    return result


def collect_files(directories: list[Path]) -> list[tuple[str, str]]:
    seen: set[tuple[str, str]] = set()
    files: list[tuple[str, str]] = []
    for directory in directories:
        for path in license_files(directory):
            text = read_text(path)
            if not text:
                continue
            key = (path.name, text)
            if key in seen:
                continue
            seen.add(key)
            files.append(key)
    return files


def collect_rust() -> tuple[list[Dependency], list[tuple[str, str, str]], str]:
    code, stdout, stderr = run(
        ["cargo", "metadata", "--format-version", "1", "--manifest-path", "Cargo.toml"], REPO
    )
    if code != 0:
        return [], [], f"cargo metadata 失败，待复核：{stderr.strip()[:200]}"
    metadata = json.loads(stdout)
    dependencies: list[Dependency] = []
    rows: list[tuple[str, str, str]] = []
    for package in metadata.get("packages", []):
        if not package.get("source"):
            continue
        manifest = Path(package["manifest_path"])
        directory = manifest.parent
        dependencies.append(
            Dependency(
                ecosystem="Rust",
                name=package["name"],
                version=package["version"],
                license=package.get("license") or "UNKNOWN",
                authors=list(package.get("authors") or []),
                repository=package.get("repository") or "",
                files=collect_files([directory]),
            )
        )
        rows.append((package["name"], package["version"], package.get("license") or "UNKNOWN"))
    dependencies.sort(key=lambda item: (item.name, item.version))
    rows.sort()
    return dependencies, rows, ""


def collect_js() -> tuple[list[Dependency], list[tuple[str, str, str]], str]:
    code, stdout, stderr = run(
        ["pnpm", "licenses", "list", "--prod", "--json"], REPO / "apps" / "desktop"
    )
    if code != 0 or not stdout.strip():
        return [], [], f"pnpm licenses 不可用，待复核：{stderr.strip()[:200]}"
    data = json.loads(stdout)
    dependencies: list[Dependency] = []
    rows: list[tuple[str, str, str]] = []
    for license_id, packages in data.items():
        for package in packages:
            versions = ", ".join(package.get("versions", [])) or "?"
            paths = [Path(value) for value in package.get("paths", []) if value]
            dependencies.append(
                Dependency(
                    ecosystem="JavaScript",
                    name=package.get("name", "?"),
                    version=versions,
                    license=license_id,
                    authors=[package["author"]] if package.get("author") else [],
                    repository=package.get("homepage") or "",
                    files=collect_files(paths),
                )
            )
            rows.append((package.get("name", "?"), versions, license_id))
    dependencies.sort(key=lambda item: (item.name, item.version))
    rows.sort()
    return dependencies, rows, ""


def normalized(text: str) -> str:
    return re.sub(r"\s+", " ", text)


def detect_family(text: str) -> str:
    text = normalized(text)
    # Unicode V3 与旧版 Unicode 许可都含 MIT 式授权句，必须优先识别。
    if "UNICODE LICENSE V3" in text or "UNICODE, INC. LICENSE AGREEMENT" in text:
        return "Unicode-3.0"
    # ISC 与 0BSD 的前半段几乎相同：ISC 要求保留版权声明，0BSD 不要求。
    if (
        re.search(
            r"Permission to use, copy, modify, and/or distribute this software for any purpose "
            r"with or without fee is hereby granted\.",
            text,
        )
        and "provided that the above copyright notice" not in text
    ):
        return "0BSD"
    for family, markers in FAMILIES:
        if all(marker in text for marker in markers):
            return family
    return "Other"


def is_full_license(text: str) -> bool:
    marker = FULL_MARKERS.get(detect_family(text))
    return bool(marker and marker in normalized(text))


def package_has_full_text(dependency: Dependency) -> bool:
    return any(is_full_license(text) for _, text in dependency.files)


def family_order(family: str) -> tuple[int, str]:
    for index, (known, _) in enumerate(FAMILIES):
        if known == family:
            return (index, family)
    return (len(FAMILIES), family)


def canonical_body(unique_texts: dict[str, dict], family: str) -> str | None:
    candidates: list[tuple[int, int, str]] = []
    for item in unique_texts.values():
        text = item["text"]
        if detect_family(text) != family or not is_full_license(text):
            continue
        names = {name.upper() for name in item["filenames"]}
        penalty = 0
        if family == "MIT" and not any("MIT" in name for name in names):
            penalty += 1
        if any("NOTICE" in name or "THIRD" in name for name in names):
            penalty += 3
        candidates.append((penalty, len(text), text))
    if not candidates:
        return None
    candidates.sort()
    text = candidates[0][2]
    marker = BODY_MARKERS.get(family)
    if marker:
        pattern = re.compile(re.escape(marker).replace(r"\ ", r"\s+"))
        match = pattern.search(text)
        if match and match.start() > 0:
            text = text[match.start():]
    elif family == "MPL-2.0":
        index = text.find("Mozilla Public License Version 2.0")
        if index > 0:
            text = text[index:]
    return text.strip()


def choose_license(expression: str) -> str:
    cleaned = expression.replace("(", " ").replace(")", " ").replace("/", " OR ")
    parts = [part.strip() for part in re.split(r"\s+(?:OR|AND)\s+", cleaned) if part.strip()]
    for preferred in LICENSE_PREFERENCE:
        for part in parts:
            if part == preferred or part.startswith(preferred + " WITH "):
                return preferred
    return parts[0] if parts else expression


def holder_of(dependency: Dependency) -> str:
    if dependency.authors:
        return "; ".join(dependency.authors)
    # SPDX 指针文件通常没有正文，但带 PackageCopyrightText。
    for _, text in dependency.files:
        for line in text.splitlines():
            if line.startswith("PackageCopyrightText:"):
                value = line.split(":", 1)[1].strip()
                if value:
                    return value
    if dependency.repository:
        return f"见上游仓库 {dependency.repository}"
    return "包作者（上游未提供）"


def build_licenses(
    dependencies: list[Dependency],
    rust_error: str,
    js_error: str,
) -> tuple[str, dict[str, int], list[str]]:
    unique: dict[str, dict] = {}
    for dependency in dependencies:
        usage = f"{dependency.ecosystem}: {dependency.name} {dependency.version} — {dependency.license}"
        for filename, text in dependency.files:
            digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
            item = unique.setdefault(
                digest,
                {"text": text, "filenames": set(), "usages": set()},
            )
            item["filenames"].add(filename)
            item["usages"].add(usage)

    missing = [dependency for dependency in dependencies if not dependency.files]
    pointer = [
        dependency
        for dependency in dependencies
        if dependency.files and not package_has_full_text(dependency)
    ]
    unresolved: list[str] = []
    lines: list[str] = []
    version = app_version()

    lines.append("SOTTR Explorer — 第三方依赖许可全文（离线副本）")
    lines.append(f"版本：{version}    生成：apps/desktop/scripts/build_notices.py")
    lines.append("")
    lines.append("本文件随应用内嵌分发，并在「关于」页内展开查看。")
    lines.append("包内自带的许可文本按内容去重；同一文本的多个依赖列在同一节。")
    lines.append("包内没有完整许可正文的依赖，按项目所选许可转载完整正文并列出作者/仓库。")
    lines.append("")
    lines.append("=" * 78)
    lines.append("0. 产品许可与仓库内第三方全文")
    lines.append("=" * 78)
    lines.append("")
    lines.append("产品许可（MIT + 资产排除；仓库 LICENSE）：")
    lines.append("-" * 78)
    if PRODUCT_LICENSE.exists():
        lines.append(read_text(PRODUCT_LICENSE))
    else:
        lines.append("（LICENSE 缺失，待复核）")
        unresolved.append("仓库 LICENSE")
    lines.append("")
    for label, path in (
        ("TrRebootModTools（内部 ID 名称字典与列表模板；MIT）", MIT_TEXT),
        ("Lucide（应用图标几何与界面图标库；ISC）", LUCIDE_TEXT),
    ):
        lines.append(f"{label}：{path.relative_to(REPO).as_posix()}")
        lines.append("-" * 78)
        if path.exists():
            lines.append(read_text(path))
        else:
            lines.append("（全文缺失，待复核）")
            unresolved.append(str(path.relative_to(REPO)))
        lines.append("")

    lines.append("=" * 78)
    lines.append(f"1. 依赖包自带的许可文本（去重后 {len(unique)} 份）")
    lines.append("=" * 78)
    lines.append("")
    ordered = sorted(
        unique.values(),
        key=lambda item: (
            family_order(detect_family(item["text"])),
            sorted(item["filenames"])[0].upper(),
            len(item["text"]),
            item["text"][:120],
        ),
    )
    for index, item in enumerate(ordered, start=1):
        family = detect_family(item["text"])
        filenames = ", ".join(sorted(item["filenames"]))
        lines.append(f"[{index}/{len(ordered)}] {family} · 文件：{filenames}")
        lines.append("使用方：")
        for usage in sorted(item["usages"]):
            lines.append(f"  - {usage}")
        lines.append("-" * 78)
        lines.append(item["text"])
        lines.append("")

    lines.append("=" * 78)
    lines.append(f"2. 包内未附完整许可正文的依赖（{len(missing) + len(pointer)} 项）")
    lines.append("=" * 78)
    lines.append("")
    lines.append("以下包在本地 registry / node_modules 中没有完整的 LICENSE/COPYING/NOTICE 正文：")
    lines.append(f"- 没有任何许可文件：{len(missing)} 个；")
    lines.append(f"- 只有许可标识/SPDX 指针，没有正文：{len(pointer)} 个。")
    lines.append("项目按优先 MIT、其次 Apache-2.0、BSD、ISC、Zlib、MPL-2.0 的顺序选择许可；")
    lines.append("版权行取自包元数据作者；上游如有不同要求，以包元数据与仓库为准。")
    lines.append("")

    def describe(dependency: Dependency) -> str:
        return (
            f"- {dependency.ecosystem}: {dependency.name} {dependency.version} — "
            f"声明 {dependency.license}；Copyright: {holder_of(dependency)}；"
            f"仓库: {dependency.repository or '未提供'}"
        )

    if missing:
        lines.append(f"--- 2.1 没有许可文件（{len(missing)} 项） ---")
        for dependency in sorted(missing, key=lambda item: (item.name, item.version)):
            lines.append(describe(dependency))
        lines.append("")
    if pointer:
        lines.append(f"--- 2.2 只有许可标识/SPDX 指针（{len(pointer)} 项） ---")
        for dependency in sorted(pointer, key=lambda item: (item.name, item.version)):
            lines.append(describe(dependency))
        lines.append("")

    needed: dict[str, list[Dependency]] = {}
    for dependency in missing + pointer:
        needed.setdefault(choose_license(dependency.license), []).append(dependency)
    lines.append("--- 2.3 按所选许可转载的完整正文 ---")
    lines.append("")
    for license_id in sorted(needed, key=family_order):
        packages = needed[license_id]
        lines.append(f"===== {license_id}（用于 {len(packages)} 个包） =====")
        lines.append("各包版权行见 2.1/2.2 的列表；以下为该项目所选许可的完整正文。")
        lines.append("")
        body = canonical_body(unique, license_id)
        if body is None:
            lines.append(f"（未找到 {license_id} 的完整文本，待复核）")
            unresolved.extend(
                f"{dependency.name} {dependency.version}" for dependency in packages
            )
        else:
            lines.append(body)
        lines.append("")

    lines.append("=" * 78)
    lines.append("3. 生成与复核说明")
    lines.append("=" * 78)
    lines.append("")
    lines.append(f"- Rust registry 包：{sum(1 for item in dependencies if item.ecosystem == 'Rust')} 个")
    lines.append(
        f"- 生产 JavaScript 依赖：{sum(1 for item in dependencies if item.ecosystem == 'JavaScript')} 个"
    )
    lines.append(f"- 自带许可文件的依赖：{len(dependencies) - len(missing)} 个")
    lines.append(f"- 去重后的许可文本：{len(unique)} 份")
    lines.append(f"- 包内无许可文件：{len(missing)} 个")
    lines.append(f"- 只有许可标识/SPDX 指针、无完整正文：{len(pointer)} 个")
    lines.append(f"- 以上按所选许可补全：{len(missing) + len(pointer)} 个")
    lines.append("- 双许可包同时收录其自带的全部许可文本；使用方可择一。")
    lines.append("- 缺失文件的包按项目所选许可补全；版权行来自包元数据，建议发布前人工复核。")
    lines.append("- 本文件不是法律意见。")
    if rust_error:
        lines.append(f"- Rust 采集异常：{rust_error}")
    if js_error:
        lines.append(f"- JavaScript 采集异常：{js_error}")
    if unresolved:
        lines.append(f"- 待复核项：{len(unresolved)}；" + "；".join(sorted(set(unresolved))[:20]))
    lines.append("")

    stats = {
        "rust": sum(1 for item in dependencies if item.ecosystem == "Rust"),
        "js": sum(1 for item in dependencies if item.ecosystem == "JavaScript"),
        "with_files": len(dependencies) - len(missing),
        "missing": len(missing),
        "pointer": len(pointer),
        "unique_texts": len(unique),
        "unresolved": len(unresolved),
    }
    return "\n".join(lines) + "\n", stats, unresolved


def txt_rows(rows: list[tuple[str, str, str]]) -> str:
    return "\n".join(f"  {name} {version} — {license_id}" for name, version, license_id in rows)


def metadata_sources() -> str:
    lines = []
    for name in ("collectibles", "region_counters", "reference_sets"):
        data = json.loads((REPO / "data" / "metadata" / f"{name}.json").read_text(encoding="utf-8"))
        states: dict[str, int] = {}
        for source in data.get("sources", {}).values():
            state = source.get("redistributable", "unknown")
            states[state] = states.get(state, 0) + 1
        summary = "、".join(f"{k}×{v}" for k, v in sorted(states.items()))
        lines.append(f"  data/metadata/{name}.json：{summary}")
    return "\n".join(lines) + "\n"


def main() -> int:
    version = app_version()
    rust_deps, rust_rows, rust_error = collect_rust()
    js_deps, js_rows, js_error = collect_js()
    dependencies = rust_deps + js_deps

    licenses_text, stats, unresolved = build_licenses(dependencies, rust_error, js_error)
    LICENSES_OUT.write_text(licenses_text, encoding="utf-8", newline="\n")

    mit = MIT_TEXT.read_text(encoding="utf-8").strip() if MIT_TEXT.exists() else "（MIT 全文缺失，待复核）"
    lucide = LUCIDE_TEXT.read_text(encoding="utf-8").strip() if LUCIDE_TEXT.exists() else "（ISC 全文缺失，待复核）"

    js_note = f"；{js_error}" if js_error else ""
    notices = f"""SOTTR Explorer — 第三方组件与许可通知（离线副本）
版本：{version}    生成：apps/desktop/scripts/build_notices.py

支持范围：Windows Steam 普通战役存档的只读分析；全收集（100%）目录与缺失清单。

产品许可：代码采用 MIT（全文见仓库 LICENSE）。地图底图、游戏数据与下列第三方组件
不在 MIT 覆盖范围内，各自保留原权利与条款；完整说明见仓库 THIRD_PARTY_NOTICES.md。

完整依赖许可全文：THIRD_PARTY_LICENSES.txt（应用内「关于」页可展开查看）。
本次生成：Rust {stats['rust']} 个、JavaScript {stats['js']} 个依赖；自带许可文件 {stats['with_files']} 个；
去重后 {stats['unique_texts']} 份许可文本；包内无许可文件 {stats['missing']} 个、
仅有许可指针 {stats['pointer']} 个（已按项目所选许可补全正文）；待复核 {stats['unresolved']} 项。

来源许可状态（含用户决策记录）：
{metadata_sources()}  - `pending` 条目均为仅链接或哈希引用：攻略正文、图片、视频与存档字节不随产品分发
  - 2026-10-17 用户确认：商业游戏数据（本地提取与派生事实）可随交付版发布
  - 2026-10-31 用户确认：ASL-SOTTR 的 43 组区域计数映射定性为事实性数据，可随产品分发
    （仅转录偏移与计数，未复制上游源代码；经独立交叉验证——内存 stride 272 字节与
    存档 stride 104 字节交叉推导、公开样本佐证、图鉴与游戏计数 43/43 吻合）
  - TrRebootModTools 字典与列表模板：MIT（全文见下）
  - 游戏名称、地图美术与相关商标归 Square Enix / Crystal Dynamics 及原权利人所有；
    本项目为只读、非官方工具，与上述公司无从属关系

运行依赖：Windows WebView2 Runtime（系统 Evergreen 组件；Windows 11 与多数 Windows 10
已预装，缺失时需从 Microsoft 官方获取安装）。

分发声明：本构建未做代码签名；Windows SmartScreen 可能提示“未知发布者”，
不声称已通过 SmartScreen。

TrRebootModTools 许可全文（内部 ID 名称字典与列表模板来源；MIT）：
----------------------------------------------------------------
{mit}
----------------------------------------------------------------

Lucide 许可全文（应用图标几何与界面图标库；ISC）：
----------------------------------------------------------------
{lucide}
----------------------------------------------------------------

Rust 依赖（name version — license，共 {len(rust_rows)} 项）：
{txt_rows(rust_rows) if not rust_error else '  ' + rust_error}
JavaScript 依赖（prod，共 {len(js_rows)} 项{js_note}）：
{txt_rows(js_rows) if not js_error else '  ' + js_error}
"""
    OUT.write_text(notices, encoding="utf-8", newline="\n")
    print(
        f"written {OUT.name} ({OUT.stat().st_size} bytes) and "
        f"{LICENSES_OUT.name} ({LICENSES_OUT.stat().st_size} bytes; "
        f"rust={stats['rust']}, js={stats['js']}, unique={stats['unique_texts']}, "
        f"missing={stats['missing']}, pointer={stats['pointer']}, "
        f"unresolved={stats['unresolved']})"
    )
    if rust_error or js_error or unresolved:
        print("WARNING: 许可材料不完整，见文件第 3 节与上面的错误信息。", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
