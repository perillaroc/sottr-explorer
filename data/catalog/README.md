# 全收集图鉴（SX-021 阶段 A；SX-022 schema 2.0.0）

`collectibles_catalog.json`：按“全收集（成就/100% 完成度）”口径整理的收集品图鉴，
也是界面「区域与收集」与 CLI `plan` 的数据源。

事实来源、对账与门禁决策见 `research/s019/findings.md`、`research/s020/README.md`、
`research/s022/README.md` 与 `../metadata/README.md`。

## 结构

| 字段 | 说明 |
| --- | --- |
| `totals` | 条目 470（故事 302 + 非故事 168）、必收 352、DLC 42、带位置 425、已确定区域 470 |
| `acquisition_rules` | 获取方式规则表（`key → {text_zh, text_en, source, limitations}`）；**人工整理**，标 `curated` |
| `categories` | 类别清单：必收（artifacts 295、monoliths 12、survival_caches 77、crypt_treasures 10）、可选（explorer_backpacks 15、archivist_maps 19、strongboxes 8、crypt_entrances 9、gym_training 8、test_items 2、croft_manor_extra 7）、`challenge_tombs` 为 `null`（未闭合）。每个类别带 `acquisition_rule_key`；`artifacts` 与 `croft_manor_extra` 另有 `subcategories`（relics/documents/murals，各带自己的规则键） |
| `collections` | 44 个故事合集（35 基础 + 7 DLC；`Prologue` 组 7 条不要求） |
| `units` | 子关卡命名：`unit_id → {zone, display_en, display_zh, name_source, region_slug}`（65 条） |
| `regions` | 26 个游戏区域 + 特殊区域；每条带 `slug` / `name_en` / `name_zh` / `item_count` / `binding`（签名匹配证据）与挑战古墓、营地、挑战、任务目标列表（`missions` 语义未定，不用于缺失） |
| `items` | 每条：内部候选名 `name`、游戏中英名 `name_en`/`name_zh`、类别、子类别、`region_hash`/`region_slug`、`unit`、`position`（关卡内相对位置，**未校准**）、`is_dlc`、必收标记、`acquisition_note`（逐项注记，可空）；共用 ID 见 `shared_id`/`shared_names` |

## 生成与校验

```powershell
# 需要仓库外的提取物（见 research/s019/README.md 与 research/s022/README.md）：
py -3 research/s022/scripts/build_derived_tables.py <work_dir> <trreboot_tools_dir>
py -3 data/catalog/build_catalog.py <work_dir>          # 写入/更新
py -3 data/catalog/build_catalog.py <work_dir> --check  # 校验与现有文件一致
```

自检（生成器内置）：故事 302、非故事 168、故事必收 253、全部条目有区域、
302 条故事条目有官方中文名、全部 unit 有 zone 名。输出同时通过
`schemas/catalog.schema.json` 契约（`pnpm contract:validate`）。

## 验收（最强的一条）

```powershell
py -3 research/s022/scripts/validate_region_counters.py `
    data/catalog/collectibles_catalog.json <work_dir>/cli_analyze/analysis.json save1.dat
```

把图鉴的「区域 × 子类别」总数与已收集数，与**存档自带的区域计数器**
逐行对账：**43/43 行完全吻合**（另 2 行属克劳馥庄园，该区域没有计数器）。
这条同时验证区域绑定、子类别推导、DLC 排除、非故事类别映射与状态判定。

## 核心与 CLI

- 加载：`sottr_core::catalog::Catalog::from_json`（严格模式：拒绝未知字段、
  稳定错误码、校验区域 slug、坐标范围、获取规则引用与各项计数）。
- 按区域计划：`sottr_core::atlas::plan`（纯函数：图鉴 + 观察集合 → 区域/类别逐项状态、
  子关卡、相对位置与获取方式）。前端 `src/data/atlas.ts` 是它的镜像实现，
  由 `tests/atlas-plan.spec.ts` 以 CLI 黄金输出逐字段对照。

```powershell
sottr-cli analyze --input <存档目录> --context campaign --output <分析目录> --metadata data/metadata
sottr-cli plan --catalog data/catalog/collectibles_catalog.json `
    --analysis <分析目录>/analysis.json --output <新目录> [--input save1.dat]
sottr-cli missing --catalog data/catalog/collectibles_catalog.json `
    --analysis <分析目录>/analysis.json --output <新目录> [--input save1.dat]
```

`plan` 输出 `plan.json`：每个区域的收集品清单、已收集/未收集、所属子关卡、
关卡内相对位置（未校准）与获取方式；特殊区域单独成组，不计入主区域合计。
`missing` 仍按类别回答「100% 必收还差哪些」（规则与 T22 一致，未改动）。

## 已知未闭合项（按 T22 不进入缺失结论）

1. **挑战古墓 ID 共用**：12 个古墓名共享 9 个唯一 ID，其中 5 个名共用占位 ID
   `0xC9D62DEE`（Pillars/Gift/Gate/Xolotl/Fire_Mountain），gym 中 2 个名共用
   `0x98564B9D`；按 ID 无法逐项判定，类别整体标记 `null`。界面按名称列出，
   但“缺失”只按共用 ID 观察，不拆分假点。
2. **任务目标（missions）**：20 条与外部「8 条支线」不符，语义未定，不展示为缺失。
3. **坐标未校准**：只作关卡内相对位置，不做地图投影、不画底图。
4. **获取方式**：`acquisition_rules` 是人工整理的类别通用说明；
   `items[].acquisition_note` 用于逐项注记（SX-025 补齐重点条目）。
5. **三个特殊区域**（`_challenge_tombs` / `_special` / `croft_manor`）无存档区域计数器，
   单独成组，不参与主区域进度。

## 许可与门禁

产品代码采用 MIT（根目录 `LICENSE`），**不覆盖本图鉴等游戏数据事实**。图鉴中的
名称、区域、坐标与计数是《古墓丽影：暗影》的数据事实；项目所有者已确认
（2026-10-17）可随交付版发布，该确认与 SSOTR 计数事实的定性（2026-10-31）都记录在
`THIRD_PARTY_NOTICES.md` 第 3、6 节。

名称来源 TrRebootModTools `tr11collectibles.bt`、zone 来源 `tr11zones.bt`（均 MIT，
全文见 `licenses/TrRebootModTools-MIT.txt`）；中英名来自游戏 `locals.bin` 的表内对齐
（见 `research/s022/README.md`）。
