# -*- coding: utf-8 -*-
"""SX-022 图鉴数据基础库：故事条目全字段、区域绑定、单位命名、本地化对齐。

只读消费已提取的游戏资源负载（`research/s019` 的提取产物）与两个 MIT 列表模板，
产出可随产品分发的事实性映射。原始商业资源不进入本仓库。

对象布局（与 `research/s019/README.md` 记录一致，SX-022 补齐故事条目字段）：

- CollectibleStory（组）步长 168：isDLC@4、组名 ref@16、条目 ref@152、条目数@160
- StoryCollectibleItem 步长 496、NonStoryCollectibleItem 步长 240，**共享同一前缀**：
  id u32@0、metricName ref@8、type i32@16、regionId u32@32、unit u32@36、
  elevation i32@72、coordinateX f32@76、coordinateY f32@80、instanceZ f32@84、
  mapMarkerIcon ref@232
- StoryCollectibleItem 另有：nameText ref@272、资源路径 ref@296
"""
from __future__ import annotations

import re
import struct
from collections import Counter
from pathlib import Path

STORY_GROUP_STRIDE = 168
STORY_ITEM_STRIDE = 496
NONSTORY_GROUP_STRIDE = 32
NONSTORY_ITEM_STRIDE = 240

#: 故事条目 type @16 -> 子类别（SX-022 实测：0=遗物 108、1=文献 131、3=壁画 63）
STORY_TYPE_SUBCATEGORY = {0: "relics", 1: "documents", 3: "murals"}

#: 非故事条目 type @16 -> 类别（沿用 SX-019/SX-020 口径）
NONSTORY_TYPE_CATEGORY = {
    2: "survival_caches",
    4: "monoliths",
    5: "explorer_backpacks",
    6: "crypt_treasures",
    7: "archivist_maps",
    8: "strongboxes",
    9: "challenge_tombs",
    11: "crypt_entrances",
    1: "test_items",
}

#: 存档区域计数器类别 -> 图鉴类别（区域绑定签名的比对维度）
COUNTER_TO_CATEGORY = {
    "relics": "relics",
    "documents": "documents",
    "murals": "murals",
    "survival_caches": "survival_caches",
    "monoliths": "monoliths",
    "crypts": "crypt_treasures",
    "strongboxes": "strongboxes",
    "challenge_tombs": "challenge_tombs",
}

#: 参与区域签名比对的维度（顺序稳定，用于生成可复核的签名字符串）
SIGNATURE_DIMENSIONS = (
    "relics",
    "documents",
    "murals",
    "survival_caches",
    "monoliths",
    "crypt_treasures",
    "strongboxes",
)

#: 未校准的坐标空间标识
POSITION_SPACE = "unit_normalized_unverified"


def _string(dtp, ref_pos):
    target = dtp.ref_target(ref_pos)
    return dtp.string_at(target) if target is not None else None


def parse_story_items(dtp) -> list[dict]:
    """解析故事收集品合集 → 逐条全字段（含 unit / 坐标 / 子类别 / 英文名）。"""
    root = dtp.ref_target(dtp.body_pos)
    story_target = dtp.ref_target(root)
    if story_target is None:
        raise ValueError("story collection root is not an internal ref")
    group_count = dtp.u32(root + 8)

    rows: list[dict] = []
    for k in range(group_count):
        base = story_target + k * STORY_GROUP_STRIDE
        group = _string(dtp, base + 16)
        is_dlc = bool(dtp.u8(base + 4))
        items_target = dtp.ref_target(base + 152)
        count = dtp.u32(base + 160)
        if items_target is None:
            continue
        for j in range(count):
            it = items_target + j * STORY_ITEM_STRIDE
            type_id = dtp.i32(it + 16)
            rows.append({
                "id": f"{dtp.u32(it):08X}",
                "metric_name": _string(dtp, it + 8),
                "type": type_id,
                "region_hash": f"{dtp.u32(it + 32):08X}",
                "unit": dtp.u32(it + 36),
                "coordinate_x": dtp.f32(it + 76),
                "coordinate_y": dtp.f32(it + 80),
                "marker": _string(dtp, it + 232),
                "name_en": _string(dtp, it + 272),
                "asset_path": _string(dtp, it + 296),
                "collection": group,
                "is_dlc": is_dlc,
            })
    return rows


def parse_nonstory_items(dtp) -> list[dict]:
    """解析非故事收集品 → 逐条全字段（含 unit / 坐标 / 地图标记）。"""
    root = dtp.ref_target(dtp.body_pos)
    ns_ref = root + 32
    ns_target = dtp.ref_target(ns_ref)
    if ns_target is None:
        raise ValueError("non-story collection root is not an internal ref")
    group_count = dtp.u32(ns_ref + 8)

    rows: list[dict] = []
    for g in range(group_count):
        base = ns_target + g * NONSTORY_GROUP_STRIDE
        is_dlc = bool(dtp.u8(base + 4))
        items_target = dtp.ref_target(base + 16)
        count = dtp.u32(base + 24)
        if items_target is None:
            continue
        for j in range(count):
            it = items_target + j * NONSTORY_ITEM_STRIDE
            rows.append({
                "id": f"{dtp.u32(it):08X}",
                "metric_name": _string(dtp, it + 8),
                "type": dtp.i32(it + 16),
                "region_hash": f"{dtp.u32(it + 32):08X}",
                "unit": dtp.u32(it + 36),
                "coordinate_x": dtp.f32(it + 76),
                "coordinate_y": dtp.f32(it + 80),
                "marker": _string(dtp, it + 232),
                "is_dlc": is_dlc,
            })
    return rows


def load_zone_names(template_path: Path) -> dict[int, str]:
    """从 MIT 列表模板 `tr11zones.bt` 读 unit id -> zone 名。"""
    pattern = re.compile(r'case (\d+): return "([^"]+)"')
    names: dict[int, str] = {}
    if not Path(template_path).exists():
        return names
    text = Path(template_path).read_text(encoding="utf-8", errors="replace")
    for match in pattern.finditer(text):
        names[int(match.group(1))] = match.group(2)
    return names


def load_locals(path: Path) -> list[tuple[str, str]]:
    """读取 locals.bin：`i32 languageId, i32 numStrings, numStrings×8 键偏移表`。

    返回**按下标顺序**的 (key, value) 列表；下标即两语言表对齐的依据。
    缺项的键偏移为 0，该位置返回 ("", "")。
    """
    data = Path(path).read_bytes()
    _language_id, count = struct.unpack_from("<ii", data, 0)
    entries: list[tuple[str, str]] = []
    for i in range(count):
        key_pos = struct.unpack_from("<i", data, 8 + 8 * i)[0]
        if key_pos <= 0:
            entries.append(("", ""))
            continue
        key_end = data.index(b" ", key_pos)
        key = data[key_pos:key_end].decode("utf-8", "replace")
        value_end = data.index(b"\0", key_end + 1)
        value = data[key_end + 1:value_end].decode("utf-8", "replace")
        entries.append((key, value))
    return entries


def align_locales(en_entries, zh_entries) -> dict[str, str]:
    """英/简中两表按**下标**对齐：英文文本 -> 中文文本。

    SX-022 实测：两表 numStrings 相同（39661），同下标键一致（抽样 290 个下标，
    仅 7 处中文表缺项）。因此 DTP 里的英文明文可直接换成官方中文。
    只保留键一致且双方非空的条目；英文重复文本取首个非空中文。
    """
    mapping: dict[str, str] = {}
    keys_mismatch = 0
    for (en_key, en_val), (zh_key, zh_val) in zip(en_entries, zh_entries):
        if not en_key or not zh_key:
            continue
        if en_key != zh_key:
            keys_mismatch += 1
            continue
        if en_val and zh_val and en_val not in mapping:
            mapping[en_val] = zh_val
    mapping["__keys_mismatch__"] = str(keys_mismatch)
    return mapping


def region_counter_targets(report: dict) -> dict[tuple[str, str], int]:
    """从分析报告的 region_counters 取 (slug, 图鉴类别) -> 来源目标数。"""
    targets: dict[tuple[str, str], int] = {}
    for counter in report["observations"]["region_counters"]:
        category = COUNTER_TO_CATEGORY.get(counter["category_id"])
        if category is None:
            continue
        targets[(counter["region_id"], category)] = counter["source_target"]
    return targets


def region_counter_values(report: dict) -> dict[tuple[str, str], int]:
    """从分析报告的 region_counters 取 (slug, 图鉴类别) -> 存档读数（已收集数）。"""
    values: dict[tuple[str, str], int] = {}
    for counter in report["observations"]["region_counters"]:
        category = COUNTER_TO_CATEGORY.get(counter["category_id"])
        if category is None:
            continue
        values[(counter["region_id"], category)] = counter["value"]
    return values


def item_signature(items) -> dict[str, Counter]:
    """区域哈希 -> 类别计数签名（仅计入非 DLC 条目，与存档计数器口径一致）。"""
    signatures: dict[str, Counter] = {}
    for item in items:
        if item.get("is_dlc"):
            continue
        category = item.get("signature_category")
        if not category:
            continue
        signatures.setdefault(item["region_hash"], Counter())[category] += 1
    return signatures


def bind_regions(items, targets: dict[tuple[str, str], int], exclude: set[str] | None = None):
    """区域哈希 → ASL slug：按类别计数签名做 L1 最近邻，返回绑定与全部距离。

    只在「存档计数器有目标数」的区域（`targets` 里出现过的 slug）之间匹配；
    `exclude` 用于跳过无计数器的特殊区域（跨区域挑战古墓、全局区域、庄园）。
    每个 slug 只允许被一个哈希占用，避免多对一。
    """
    exclude = exclude or set()
    slug_dims: dict[str, Counter] = {}
    for (slug, category), target in targets.items():
        slug_dims.setdefault(slug, Counter())[category] = target

    signatures = item_signature(items)
    bindings: dict[str, dict] = {}
    taken: dict[str, str] = {}

    candidates = []
    for region_hash, counts in signatures.items():
        if region_hash in exclude:
            continue
        scored = []
        for slug, dims in slug_dims.items():
            distance = sum(abs(counts.get(dim, 0) - dims.get(dim, 0))
                           for dim in SIGNATURE_DIMENSIONS)
            scored.append((distance, slug))
        scored.sort()
        candidates.append((scored[0][0], region_hash, scored))

    # 距离升序贪心，保证每个 slug 至多绑定一个哈希
    candidates.sort()
    for distance, region_hash, scored in candidates:
        best_distance, best_slug = scored[0]
        if best_slug in taken:
            free = [(d, s) for d, s in scored if s not in taken]
            if not free:
                continue
            best_distance, best_slug = free[0]
        taken[best_slug] = region_hash
        runner_up = [(d, s) for d, s in scored if s != best_slug]
        signature = signatures[region_hash]
        bindings[region_hash] = {
            "region_slug": best_slug,
            "signature_distance": best_distance,
            "runner_up_distance": runner_up[0][0] if runner_up else None,
            "signature": {dim: signature.get(dim, 0) for dim in SIGNATURE_DIMENSIONS},
            "targets": {dim: slug_dims[best_slug].get(dim, 0) for dim in SIGNATURE_DIMENSIONS},
        }
    return bindings


def load_source_targets_from_metadata(metadata_dir: Path) -> dict[tuple[str, str], int]:
    """无存档时仍可绑定：从 `data/metadata/region_counters.json` 取来源目标数。"""
    data = __import__("json").loads(
        (Path(metadata_dir) / "region_counters.json").read_text(encoding="utf-8"))
    targets: dict[tuple[str, str], int] = {}
    for counter in data.get("region_counters", {}).values():
        category = COUNTER_TO_CATEGORY.get(counter["category_id"])
        if category is None:
            continue
        targets[(counter["region_id"], category)] = counter["source_total"]
    return targets
