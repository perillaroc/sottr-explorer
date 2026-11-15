# 全收集图鉴

`data/catalog/collectibles_catalog.json` 是按“全收集（成就 / 100% 完成度）”口径整理的收集品图鉴，
也是界面「区域与收集」与 CLI `plan` / `missing` 的数据源。

## 规模

- 条目 470（故事 302 + 非故事 168）；
- 必收 352，DLC 42，带位置 409，已确定区域 470；
- 65 个命名单位、26 个主区域，另有三个特殊区域；
- 42 个故事合集，其中 Prologue 组 7 条不要求。

## 结构

| 字段 | 说明 |
| --- | --- |
| `totals` | 上述计数 |
| `acquisition_rules` | 获取方式规则表，人工整理并标 `curated` |
| `categories` | 必收：artifacts 295、monoliths 12、survival_caches 77、crypt_treasures 10；可选：explorer_backpacks 15、archivist_maps 19、strongboxes 8、crypt_entrances 9、gym_training 8、test_items 2、croft_manor_extra 7；`challenge_tombs` 为 `null`（ID 共用未闭合） |
| `collections` | 故事合集 |
| `units` | 子关卡命名（65 条） |
| `regions` | 区域、绑定证据、挑战古墓/营地/挑战/任务目标列表 |
| `items` | 内部名、游戏中英名、类别/子类别、区域、单位、归一化位置、DLC 与必收标记、逐项注记 |

## 逐条攻略

`data/catalog/guides.json` 是逐条攻略映射的创作数据集：按条目 ID 记录来源、来源页定位、
粒度与核对状态。只有 `scope = item` 且 `confidence = verified` 的引用会在
`build_metadata.py` 构建时合并为元数据 `exact_guide_item`，界面据此在详情里显示
「攻略页 · 定位」。当前已收录 301 条 verified item 映射；`py -3 data/catalog/check_guides.py`
校验结构、来源与覆盖计数；加 `--worklist` 可打印仍缺逐条链接的必收条目。

## 位置口径

图鉴坐标是**每单位独立**的归一化位置（`[0,1]`，原点西北、y 向下）。它本身不带校准状态；
是否已校准由[地图基底](/data/map)按单位标注。

## 生成与校验

```powershell
py -3 research/game-data/build_derived_tables.py <work_dir> <trreboot_tools_dir>
py -3 data/catalog/build_catalog.py <work_dir>          # 写入
py -3 data/catalog/build_catalog.py <work_dir> --check  # 核对现有文件
```

生成器内置自检：故事 302、非故事 168、故事必收 253、全部条目有区域、302 条故事条目有官方
中文名、全部 unit 有 zone 名。输出同时通过 `schemas/catalog.schema.json`。

## 最强的一条验收

把图鉴的「区域 × 子类别」总数与已收集数，与存档自带的区域计数器逐行对账：**43/43 行完全吻合**
（另 2 行属克劳馥庄园，该区域没有计数器）。这条同时验证区域绑定、子类别推导、DLC 排除、
非故事类别映射与状态判定。

## 未闭合项

1. 挑战古墓 ID 共用：12 个古墓名共享 9 个唯一 ID，其中 5 个名共用占位 ID，无法逐项判定；
2. 任务目标语义未定，不展示为缺失；
3. 获取方式是类别通用说明，逐项注记只覆盖重点条目；
4. 三个特殊区域没有存档区域计数器，单独成组。

完整说明见
[data/catalog/README.md](https://github.com/perillaroc/sottr-explorer/blob/main/data/catalog/README.md)。
