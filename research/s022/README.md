# SX-022 研究记录：图鉴数据基础（区域 / 子关卡 / 中英名 / 位置 / 获取方式）

把 SX-019/SX-020 已提取的游戏数据，闭合成**可按区域展示、可判定收集状态、可给出查找线索**
的图鉴。产出为事实性映射与版本化数据集；原始商业资源仍只在本地读取、不进仓库。

## 环境与输入

| 项 | 值 |
| --- | --- |
| 输入（仓库外） | `private/s019_work/extracted/`：`dtp_0360E8.bin`（globalcollectibleinfo，update3）、`gameflow_dtp_0360F4.bin`、`locals_english.bin`、`locals_simplechinese.bin` |
| 列表模板（MIT） | `private/s019_tools/TrRebootModTools/Templates/010 Editor/tr/tr11/lists/tr11zones.bt` |
| 区域计数器来源目标 | `data/metadata/region_counters.json`（ASL-SOTTR，社区脚本） |
| 观察集（验收） | `private/s019_work/cli_analyze/analysis.json`（5 个私人战役槽位的 CLI 批量分析） |
| 观察日期 | 2026-10-17 |

## 结论（均已复核）

### 1. 区域哈希 → ASL slug：唯一双射，签名距离 0

用「目录的区域×类别计数」对「存档区域计数器的来源目标」做 L1 最近邻匹配。
9 个区域全部唯一命中、距离 0，次优解距离 ≥ 3：

| 区域哈希 | slug | 目录计数（遗物/文献/壁画/贮藏/石碑/墓穴/保险箱） |
| --- | --- | --- |
| C5B30CDB | paititi | 28/42/23/34/7/5/3 |
| 838BAE8C | kuwaq_yaku | 4/18/10/14/3/2/2 |
| 8D2E7B5C | mission_of_san_juan | 9/27/4/8/1/1/1 |
| D65AE5D8 | peruvian_jungle | 9/13/7/9/1/2/1 |
| B06345B4 | cenote | 5/6/0/8/0/0/0 |
| B02CF2FD | cozumel | 2/1/2/3/0/0/1 |
| C92877A7 | porvenir_oil_fields | 1/4/1/0/0/0/0 |
| 8C0D7019 | trial_of_the_eagle | 0/1/2/1/0/0/0 |
| 96135833 | head_of_the_serpent | 0/0/1/0/0/0/0 |

特殊区域（无计数器，单独成组，不参与主区域进度）：

| 哈希 | slug | 说明 |
| --- | --- | --- |
| 26694CD8 | `_challenge_tombs` | 挑战古墓聚合区域（`regionProgressionIndex=999`、`hideOnMap=1`） |
| 29E50C12 | `_special` | 不在 26 区主表内；Path of the Stars / Lara's Notebook 等跨区域条目 |
| 669D3CB3 | `croft_manor` | 克劳馥庄园（Prologue，不参与 100%） |

证据全文：`research/s022/data/region_bindings.json`。

### 2. 故事条目结构：与 nonStory 共享前缀，子类别可还原

SX-019 只从故事条目取了 4 个字段（496 字节结构）。实测同一结构在相同偏移上有：

| 偏移 | 字段 | 证据 |
| --- | --- | --- |
| 16 | `type` | 0 = 遗物（108）、1 = 文献（131）、3 = 壁画（63） |
| 32 / 36 | `regionId` / `unit` | 取值集合与 nonStory 完全一致 |
| 76 / 80 | `coordinateX` / `coordinateY` | 302 条全部落在 [0, 1] |
| 232 | `mapMarkerIcon` | `Relic` 70 / `Document` 131 / `Mural` 63 / 无 38 |
| 272 | 英文名明文 | 如 `HIDDEN CITY RIDDLE` |
| 296 | 资源路径 | 如 `Collectibles\tr11_relics\Relic_Hidden_City_Riddle` |

**注意**：`type` 才是权威子类别；38 条 `mapMarkerIcon` 为空的故事条目仍然是遗物
（其中 24 条属 `_special`），不能按图标判定。

### 3. 端到端验收：43/43 行与存档区域计数器完全吻合

`scripts/validate_region_counters.py` 用「区域绑定 + 子类别 + DLC 排除 + 观察=已收集」
重算，与**存档自带的区域计数器**逐行比对：

```text
paititi               relics           28 / 28 目标，26 / 26 读数  OK
paititi               survival_caches  34 / 34 目标，32 / 32 读数  OK
mission_of_san_juan   7 个类别全部     目标吻合，读数 0 / 0        OK
...  matched rows: 43 OK / 0 MISMATCH / 2 no counter（croft_manor 无计数器）
```

这一条同时验证：区域绑定、故事子类别、DLC 排除、非故事类别映射，
以及**「存档观察集合 = 已收集集合」**。

### 4. 子关卡命名：65/65 可解析

`tr11zones.bt` 的 `GetZoneName(zoneId)` 覆盖全部 65 个被引用的 unit
（`243→pa_hub_paititi`、`318→sm_spanish_mission_village`、`2468→ct_serpents_hollow`…）。
显示名优先取 `regionname_*` 官方区域名，其余用人工补全表（标 `curated`）。

### 5. 中英名：locals 双表按**下标**对齐

`locals.bin` 是扁平表：`i32 languageId, i32 numStrings, numStrings×8 键偏移表`。
英/简中两表 `numStrings` 同为 39661，**同下标键相同**（对齐 37625 个键，
键冲突 0；中文缺项处跳过）。于是：

- 故事条目：`<metricName小写>_title_<guid>` 前缀命中 **302/302**，
  例如 `HIDDEN CITY RIDDLE → 隐秘之城的谜团`、`SILVER BOX OF IX CHEL → 伊希切尔的银盒子`；
- 区域名：42 个 `regionname_*` 全部可读（`PAITITI → 帕依提提`、`CROFT MANOR → 克劳馥庄园`…）。

这推翻了 SX-019「DTP 64 位键 → locals 键哈希未闭合」的阻塞：不需要破解哈希，
用**文本/前缀直接定位下标**即可。

### 6. 覆盖度：470 条就是完整收集品全集

save1 观察 517 个 ID，其中 358 个属于图鉴；其余 159 个经 866 字典逐条核对
**全部是服装 / 技能 / 装备**（`Lara_Explorer`、`DLC_Outfit_13`、`Stealth_01`…），
不是收集品。因此图鉴不存在「目录外收集品」。

### 7. 坐标空间

168 条非故事条目中 164 条落在 [0, 1]（4 个 gym 训练场条目越界，已丢弃）；
302 条故事条目全部落在 [0, 1]。判定为**关卡内归一化坐标**，但**未校准**为地图投影，
产品里只作相对位置散点，并明确标注未校准。

## 产出与脚本

| 文件 | 作用 |
| --- | --- |
| `scripts/sottr_atlas.py` | 库：故事/非故事全字段解析、zone 命名、locals 读取与对齐、区域签名匹配 |
| `scripts/build_derived_tables.py` | 生成 `research/s022/data/{region_bindings,region_names,unit_names,item_names,locale_summary}.json` |
| `scripts/validate_region_counters.py` | 验收：图鉴 ↔ 存档区域计数器逐行对账（43 行） |
| `data/catalog/build_catalog.py` | 生成 schema 2.0.0 图鉴 `data/catalog/collectibles_catalog.json` |

复现：

```powershell
$W = "<work_dir>"                                  # 含 extracted/ 的目录
$T = "<TrRebootModTools 目录>"
py -3 research/s022/scripts/build_derived_tables.py $W $T
py -3 data/catalog/build_catalog.py $W
py -3 research/s022/scripts/validate_region_counters.py `
    data/catalog/collectibles_catalog.json $W/cli_analyze/analysis.json save1.dat
py -3 data/catalog/build_catalog.py $W --check      # 可复现性
```

## 未闭合项（不阻塞，产品内如实标注）

1. **挑战古墓共用 ID**：12 个古墓名共用 9 个 ID（`0xC9D62DEE`×5、`0x98564B9D`×2），
   按 ID 计数、展示全部名称，不拆分假点。
2. **`_special` / `_challenge_tombs` 的宿主区域**：游戏数据把它们归到聚合区域；
   产品保持原样，用子关卡名（各古墓 / 各关卡）表达实际位置，不改写为宿主区域。
3. **坐标未校准**：不做地图投影、不画底图。
4. **获取方式文案**为人工整理的类别通用规则（`source: curated`），
   逐项注记字段 `items[].acquisition_note` 留给 SX-025 补齐重点条目。
5. **区域名键的语义**：`regionname_pa_paititi02` 等取的是该区域主显示名；
   26 区中未承载收集品的区域不展示。

## 许可与门禁

用户已确认（2026-10-17）商业游戏数据可随交付版发布，许可不作为阻塞。
名称与 zone 来自 TrRebootModTools（MIT，见 `THIRD_PARTY_LICENSES.md`）。
派生表只含事实性映射，不含原始资源转储。
