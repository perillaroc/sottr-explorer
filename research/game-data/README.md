# 游戏安装数据（game-data）

从 Shadow of the Tomb Raider（Steam，Definitive Edition）安装包的 `bigfile*.tiger`
归档中提取、对账并生成图鉴派生表。全程只读；原始资源与提取物留在仓库外
`private/s019_work/`。

## 文件

| 文件 | 作用 |
| --- | --- |
| [`sottr_dtp.py`](sottr_dtp.py) | 容器读取库：TIGER v5、CDRM、DRM v23、DTP 对象图、FNV-1a 64 |
| [`sottr_atlas.py`](sottr_atlas.py) | 图鉴数据基础库：故事/非故事条目全字段、区域绑定、单位命名、本地化对齐 |
| [`extract_collectibleinfo.py`](extract_collectibleinfo.py) | 提取步骤 1：`globalcollectibleinfo.drm`，逐归档家族比对负载 |
| [`parse_collectibleinfo.py`](parse_collectibleinfo.py) | 提取步骤 2：解析非故事收集品全集 |
| [`extract_gameflow_locals.py`](extract_gameflow_locals.py) | 提取步骤 3：`globalgameflow.drm`（26 区域、94 单位地图）与 `locals.bin`（英/简中） |
| [`extract_full_collection.py`](extract_full_collection.py) | 故事收集品全量与区域组成列表 |
| [`crosscheck_saves.py`](crosscheck_saves.py) | CLI 观察 ID vs DTP 全集、866 名称字典交叉对账 |
| [`reconcile_full_collection.py`](reconcile_full_collection.py) | 全收集口径对账：故事分组、区域组成、单位地图、整体百分比 |
| [`build_derived_tables.py`](build_derived_tables.py) | 生成 `data/` 下的五张派生表 |
| [`validate_region_counters.py`](validate_region_counters.py) | 验收：图鉴 ↔ 存档区域计数器逐行对账（43 行） |
| [`data/`](data/) | 随产品分发的派生表：区域绑定、区域名、单位名、条目名、对齐统计 |

## 记录

- [`notes/extraction.md`](notes/extraction.md)：环境、输入、容器格式、脚本与运行示例（原 SX-019）
- [`notes/findings.md`](notes/findings.md)：提取到的事实、假说与未解问题（原 SX-019）
- [`notes/reconciliation.md`](notes/reconciliation.md)：全收集口径对账与否定结果（原 SX-020）
- [`notes/derived-tables.md`](notes/derived-tables.md)：派生表口径、43 行验收与未闭合项（原 SX-022）

## 复现顺序

仓库根目录执行；`$W` 是仓库外的工作目录（含 `extracted/`），`$T` 是 TrRebootModTools 目录。

```powershell
$W = "<work_dir>"
$T = "<TrRebootModTools 目录>"

# 提取（需要游戏安装目录）
py -3 research/game-data/extract_collectibleinfo.py "<游戏目录>" "$W/extracted"
py -3 research/game-data/parse_collectibleinfo.py "$W/extracted/dtp_0360E8.bin" "$W/extracted/collectibleinfo_update3.json"
py -3 research/game-data/extract_gameflow_locals.py "<游戏目录>" "$W/extracted"

# 全收集口径（故事全量 + 区域组成 + 对账）
py -3 research/game-data/extract_full_collection.py "$W" "$W/extracted"
py -3 research/game-data/reconcile_full_collection.py "$W" "$W/extracted"

# 派生表 → 图鉴 → 验收
py -3 research/game-data/build_derived_tables.py "$W" "$T"
py -3 data/catalog/build_catalog.py "$W"
py -3 research/game-data/validate_region_counters.py `
    data/catalog/collectibles_catalog.json "$W/cli_analyze/analysis.json" save1.dat
```

`build_derived_tables.py` 省略第三个参数时写入本目录的 `data/`；
`data/catalog/build_catalog.py --check` 用于确认现有图鉴可复现。
