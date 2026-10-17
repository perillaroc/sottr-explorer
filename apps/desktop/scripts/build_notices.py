# -*- coding: utf-8 -*-
"""SX-018 许可通知生成：汇总内嵌来源、依赖许可与声明，输出 THIRD_PARTY_NOTICES.txt。

依赖工具：`cargo metadata`（Rust 依赖许可标识）与 `pnpm licenses list --prod --json`
（JS 依赖许可标识）；失败时对应小节标注为“待复核”，不虚构内容。

用法（仓库根目录）：
  py -3 apps/desktop/scripts/build_notices.py
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
OUT = REPO / "THIRD_PARTY_NOTICES.txt"
MIT_TEXT = REPO / "research" / "python-v0.2" / "data" / "THIRD_PARTY_LICENSE.txt"


def run(args: list[str], cwd: Path) -> tuple[int, str, str]:
    try:
        proc = subprocess.run(
            args,
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            shell=False,
        )
        return proc.returncode, proc.stdout, proc.stderr
    except OSError as error:
        return 127, "", str(error)


def rust_section() -> tuple[str, int]:
    code, stdout, stderr = run(
        ["cargo", "metadata", "--format-version", "1", "--manifest-path", "Cargo.toml"], REPO
    )
    if code != 0:
        return f"（cargo metadata 失败，待复核：{stderr.strip()[:200]}）\n", 0
    metadata = json.loads(stdout)
    rows = []
    for package in metadata.get("packages", []):
        license_id = package.get("license") or "UNKNOWN"
        rows.append(f"  {package['name']} {package['version']} — {license_id}")
    rows.sort()
    return "\n".join(rows) + "\n", len(rows)


def js_section() -> tuple[str, int]:
    code, stdout, stderr = run(
        ["pnpm", "licenses", "list", "--prod", "--json"], REPO / "apps" / "desktop"
    )
    if code != 0 or not stdout.strip():
        return f"（pnpm licenses 失败，待复核：{stderr.strip()[:200]}）\n", 0
    data = json.loads(stdout)
    rows = []
    if isinstance(data, dict):
        for license_id, packages in sorted(data.items()):
            for package in packages:
                name = package.get("name", "?")
                versions = ", ".join(package.get("versions", []))
                rows.append(f"  {name} {versions} — {license_id}")
    return "\n".join(rows) + "\n", len(rows)


def js_from_node_modules() -> tuple[str, int]:
    root = REPO / "apps" / "desktop" / "node_modules"
    if not root.is_dir():
        return "（node_modules 缺失，待复核）\n", 0
    rows = []
    for entry in sorted(root.iterdir()):
        packages = []
        if entry.name.startswith("@") and entry.is_dir():
            packages = sorted(entry.iterdir())
        elif entry.is_dir() and entry.name != ".bin":
            packages = [entry]
        for package in packages:
            manifest = package / "package.json"
            if not manifest.exists():
                continue
            try:
                data = json.loads(manifest.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                continue
            name = data.get("name", package.name)
            version = data.get("version", "?")
            license_id = data.get("license") or "UNKNOWN"
            rows.append(f"  {name} {version} — {license_id}")
    rows.sort()
    return "\n".join(rows) + "\n", len(rows)


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
    version = json.loads((REPO / "apps" / "desktop" / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8")).get("version", "?")
    rust_rows, rust_count = rust_section()
    js_rows, js_count = js_section()
    js_source = None
    if js_count == 0:
        js_rows, js_count = js_from_node_modules()
        js_source = "按本地 node_modules 的 package.json 采集（pnpm licenses 不可用）"
    mit = MIT_TEXT.read_text(encoding="utf-8").strip() if MIT_TEXT.exists() else "（MIT 全文缺失，待复核）"

    notices = f"""SOTTR Explorer — 第三方组件与许可通知（离线副本）
版本：{version}    生成：SX-018（apps/desktop/scripts/build_notices.py）

支持范围：Windows Steam 普通战役存档的只读分析；全收集（100%）目录与缺失清单。
产品许可状态：尚未选择（保留所有权利；选择许可证为单独决策）。

来源许可状态（含用户决策记录）：
{metadata_sources()}  - 2026-10-17 用户确认：商业游戏数据（本地提取与派生事实）可随交付版发布
  - 2026-10-17 用户确认：社区来源（ASL-SOTTR 计数事实、Stella 引用等 pending 项）一并豁免复核
  - TrRebootModTools 字典与列表模板：MIT（全文见下）
  - 本地游戏资源提取物：仅本机读取，含派生事实；原始素材不随包分发

运行依赖：Windows WebView2 Runtime（系统 Evergreen 组件；Windows 11 与多数 Windows 10
已预装，缺失时需从 Microsoft 官方获取安装）。

分发声明：本构建未做代码签名；Windows SmartScreen 可能提示“未知发布者”，
不声称已通过 SmartScreen。

TrRebootModTools 许可全文（内部 ID 名称字典与列表模板来源；MIT）：
----------------------------------------------------------------
{mit}
----------------------------------------------------------------

Rust 依赖（name version — license，共 {rust_count} 项）：
{rust_rows}
JavaScript 依赖（prod，共 {js_count} 项{('；' + js_source) if js_source else ''}）：
{js_rows}
"""
    OUT.write_text(notices, encoding="utf-8")
    print(f"written {OUT} ({OUT.stat().st_size} bytes; rust={rust_count}, js={js_count})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
