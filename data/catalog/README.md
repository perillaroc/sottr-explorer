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
