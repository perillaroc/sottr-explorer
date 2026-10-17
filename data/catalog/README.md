# 全收集目录（SX-021 阶段 A）

`collectibles_catalog.json`：按“全收集（成就/100% 完成度）”口径整理的收集品目录源数据，
供后续阶段（元数据管道接入、CLI/界面缺失清单）消费。事实来源、数字对账与门禁决策见
`research/s019/findings.md`、`research/s020/README.md` 与 `../metadata/README.md`。

## 结构

| 字段 | 说明 |
| --- | --- |
| `totals` | 条目总数 470（故事 302 + 非故事 168）、必收 352、DLC 42 |
| `categories` | 类别清单：必收（artifacts 253、monoliths 12、survival_caches 77、crypt_treasures 10）、可选（backpacks 15、maps 19、strongboxes 8、crypt_entrances 9、gym 8、test 2、croft_manor 7）、`challenge_tombs` 为 `null`（未闭合，见下） |
| `collections` | 44 个故事合集（35 基础 + 7 DLC；`Prologue` 组 7 条不要求） |
| `regions` | 26 个区域的组成（挑战古墓 9、营地、挑战、任务目标列表；任务目标语义未闭合，不得用于缺失） |
| `items` | 每条：名称（MIT 字典优先）、类别、region 哈希、DLC/必收标记；共用 ID 见 `shared_id`/`shared_names` |

## 生成与校验

```powershell
# 需要仓库外的提取物（见 research/s019/README.md）：
py -3 data/catalog/build_catalog.py <work_dir>          # 写入/更新
py -3 data/catalog/build_catalog.py <work_dir> --check  # 校验与现有文件一致
```

自检（生成器内置）：故事 302、非故事 168、故事必收 253；输出需通过
`schemas/catalog.schema.json` 契约。

核心加载（阶段 B 已完成）：`sottr_core::catalog::Catalog::from_json` 以严格模式加载并校验
（拒绝未知字段；稳定错误码；`crates/sottr-core/tests/catalog_dataset.rs` 覆盖真实数据与
非法输入）。目录作为**独立版本化数据集**（`catalog_version`）随产品数据分发，
不并入 `data/metadata/` 三文件组（冻结契约保持不变）。

## CLI 用法（阶段 C1 已完成）

```powershell
sottr-cli analyze --input <存档目录> --context campaign --output <分析目录> --metadata data/metadata
sottr-cli missing --catalog data/catalog/collectibles_catalog.json `
    --analysis <分析目录>/analysis.json --output <新目录> [--input save1.dat]
```

输出 `missing.json`：按类别进度与逐项缺失。规则（与 T22 一致）：必收类别给逐项清单
（缺失=未在所选存档观察集合中出现；**计数与清单均按必收子集，DLC 与非必收条目不混入**）；
可选类别只给计数；未闭合类别（如挑战古墓）不出缺失结论；共用同一 ID 的条目按 ID
计数并附全部名称；观察集合中不在目录的 ID 单列。

前端接入（阶段 C2a 已完成）：`apps/desktop/src/data/catalog.ts` 内嵌本目录并由
`computeMissing` 计算缺失（与 `sottr-core::missing` 规则一致；`tests/catalog-missing.spec.ts`
以 CLI 黄金输出 `tests/fixtures/missing/expected.json` 对照）。界面组件接入见阶段 C2b。

## 已知未闭合项（按 T22 不进入缺失结论）

1. **挑战古墓 ID 共用**：12 个古墓名共享 9 个唯一 ID，其中 5 个名共用占位 ID
   `0xC9D62DEE`（Pillars/Gift/Gate/Xolotl/Fire_Mountain），gym 中 2 个名共用
   `0x98564B9D`；按 ID 无法逐项判定，类别整体标记 `null`。界面展示可按名称列出，
   但“缺失”只能按共用 ID 观察，不得拆分假点。
2. **任务目标（missions）**：20 条与外部“8 条支线”不符，语义未定，不展示为缺失。
3. **区域映射**：目录使用游戏 region 哈希；与产品既有区域别名（ASL 计数 slug）的
   对应关系尚未建立（待阶段 B 用计数对账建立）。
4. **故事/非故事子类别**：故事条目未细分 relic/document/mural（iconType 为空），
   按合集组展示；细分留待后续研究。

## 许可与门禁

用户已确认（2026-10-17）商业游戏数据可随交付版发布，许可不作为阻塞；名称来源
TrRebootModTools `tr11collectibles.bt`（MIT，见 `THIRD_PARTY_LICENSES.md`）。
