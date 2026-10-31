# -*- coding: utf-8 -*-
"""许可通知生成：汇总依赖许可、内嵌来源与声明，输出 THIRD_PARTY_NOTICES.txt。

该文件随应用分发，并在「设置与关于」页内展示；仓库不再另存依赖清单。
依赖工具：`cargo metadata`（Rust 依赖许可标识）与
`pnpm licenses list --prod --json`（生产 JavaScript 依赖许可标识）；
失败时对应小节标注为“待复核”，不虚构内容。

用法（仓库根目录）：
  py -3 apps/desktop/scripts/build_notices.py
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
OUT = REPO / "THIRD_PARTY_NOTICES.txt"
MIT_TEXT = REPO / "licenses" / "TrRebootModTools-MIT.txt"
LUCIDE_TEXT = REPO / "licenses" / "lucide-ISC.txt"


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


def rust_packages() -> tuple[list[tuple[str, str, str]], str]:
    """返回 registry 包（排除本仓库 workspace 包）与失败说明。"""
    code, stdout, stderr = run(
        ["cargo", "metadata", "--format-version", "1", "--manifest-path", "Cargo.toml"], REPO
    )
    if code != 0:
        return [], f"cargo metadata 失败，待复核：{stderr.strip()[:200]}"
    metadata = json.loads(stdout)
    rows = []
    for package in metadata.get("packages", []):
        if not package.get("source"):
            continue
        rows.append(
            (
                package["name"],
                package["version"],
                package.get("license") or "UNKNOWN",
            )
        )
    rows.sort()
    return rows, ""


def js_packages() -> tuple[list[tuple[str, str, str]], str]:
    """返回生产依赖 (name, versions, license) 列表；优先 pnpm，失败时回退 node_modules。"""
    code, stdout, stderr = run(
        ["pnpm", "licenses", "list", "--prod", "--json"], REPO / "apps" / "desktop"
    )
    if code == 0 and stdout.strip():
        data = json.loads(stdout)
        rows = []
        if isinstance(data, dict):
            for license_id, packages in data.items():
                for package in packages:
                    rows.append(
                        (
                            package.get("name", "?"),
                            ", ".join(package.get("versions", [])) or "?",
                            license_id,
                        )
                    )
        rows.sort()
        return rows, ""
    rows, error = js_from_node_modules()
    return rows, f"pnpm licenses 不可用，按 node_modules 的 package.json 采集：{error}"


def js_from_node_modules() -> tuple[list[tuple[str, str, str]], str]:
    root = REPO / "apps" / "desktop" / "node_modules"
    if not root.is_dir():
        return [], "node_modules 缺失"
    try:
        manifest = json.loads(
            (REPO / "apps" / "desktop" / "package.json").read_text(encoding="utf-8")
        )
    except (OSError, json.JSONDecodeError):
        manifest = {}
    prod_names = set(manifest.get("dependencies", {}).keys())

    rows = []
    for entry in sorted(root.iterdir()):
        packages = []
        if entry.name.startswith("@") and entry.is_dir():
            packages = sorted(entry.iterdir())
        elif entry.is_dir() and entry.name != ".bin":
            packages = [entry]
        for package in packages:
            manifest_path = package / "package.json"
            if not manifest_path.exists():
                continue
            try:
                data = json.loads(manifest_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                continue
            name = data.get("name", package.name)
            if name not in prod_names:
                continue
            rows.append((name, data.get("version", "?"), data.get("license") or "UNKNOWN"))
    rows.sort()
    return rows, ""


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
    version = json.loads((REPO / "apps" / "desktop" / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8")).get("version", "?")
    rust_rows, rust_error = rust_packages()
    js_rows, js_error = js_packages()
    mit = MIT_TEXT.read_text(encoding="utf-8").strip() if MIT_TEXT.exists() else "（MIT 全文缺失，待复核）"
    lucide = LUCIDE_TEXT.read_text(encoding="utf-8").strip() if LUCIDE_TEXT.exists() else "（ISC 全文缺失，待复核）"

    js_note = f"；{js_error}" if js_error else ""
    notices = f"""SOTTR Explorer — 第三方组件与许可通知（离线副本）
版本：{version}    生成：apps/desktop/scripts/build_notices.py

支持范围：Windows Steam 普通战役存档的只读分析；全收集（100%）目录与缺失清单。

产品许可：代码采用 MIT（全文见仓库 LICENSE）。地图底图、游戏数据与下列第三方组件
不在 MIT 覆盖范围内，各自保留原权利与条款；完整说明见仓库 THIRD_PARTY_NOTICES.md。

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
    OUT.write_text(notices, encoding="utf-8")
    print(
        f"written {OUT.name} ({OUT.stat().st_size} bytes; rust={len(rust_rows)}, js={len(js_rows)})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
