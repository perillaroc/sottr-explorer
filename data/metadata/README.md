# 元数据数据集（SX-008）

版本化收集品元数据。应用只读加载这三个 JSON 文件，**不执行其中任何内容**、
不自动联网更新；同一个数据集可用于多个槽位，更新名称或攻略链接不需要改动
二进制解析器。

| 文件 | 内容 |
| --- | --- |
| `collectibles.json` | 866 个内部 ID 条目（全部候选名称保留为 `aliases`）与 9 个类别 |
| `region_counters.json` | 9 个区域与 43 项稀疏区域计数映射 |
| `reference_sets.json` | 参考集合（逐类别参考 ID）与 3 个圣胡安攻略组 |
| `manifest.json` | 输入与输出内容哈希清单（漂移检测用） |
| `build_metadata.py` | 转换脚本（开发期工具；不进入产品运行路径） |

Schema 为 `schemas/metadata.schema.json`（契约 1.0.0）；字段语义与拒绝规则
见 `docs/contracts.md`。三个数据文件作为**一组**加载（加载器与 Rust 测试均按
一组校验）；纯净的缺失状态返回空元数据包，不产生任何硬编码兜底推定。

## 来源与证据边界

- 内部 ID 与候选名称：TrRebootModTools（MIT，固定提交
  `a79c48b75f1ffaaf623dc73a625211bf98ff0504`）的字典转录（研究包
  `tool/data/collectible_id_candidates.json`，本仓库副本在 `research/python-v0.2/data/`）。
- 区域计数偏移与目标：Atorizil/ASL-SOTTR（未声明许可证，固定提交
  `b758ee890eac30d9e3b3477999a30f8f9e442f00`）的事实转录与独立比对；
  43 项稀疏映射，缺项不是 0。
- 参考集合：Stella 攻略站两个作者标注 100% 参考槽位（Medium 与
  NG+/Deadly Obsession）共同且相等的四组集合；压缩文件 SHA-256 记录在
  `reference_sets.json` 的 `source_save_hashes`（存档字节不随产品分发）。
- 攻略组与区域线索：Stella 圣胡安攻略页与详图页的组级引用（仅链接与事实，
  不打包攻略内容）。

名称策略：`aliases` 是内部候选名称，不是游戏显示名称；`display_name` 在无来源
本地化名称时为 `null`，人工简名以 `name_kind: "curated_short"` 标明。中文名未知
就保持 `null`，不把内部名片段伪装成游戏名称。

定位策略：仅记录区域线索与攻略组成员关系；**精确坐标映射为 0，不产生假点**；
不建立内部 ID 到攻略编号的一对一映射（例如圣胡安贮藏点组 8 项只给一个组级线索）。

## 重新生成

```powershell
py -3 data/metadata/build_metadata.py          # 生成三个数据文件与 manifest.json
py -3 data/metadata/build_metadata.py --check  # 校验现有文件与 manifest 一致
```

脚本只从 `research/python-v0.2/data/` 的三个研究数据文件与脚本内显式记录的
固定事实（参考存档哈希、圣胡安/帕伊提提归属、攻略组与来源表）转换，并做数量与
交叉一致性校验（866 ID / 871 名称 / 43 计数 / 65、12、12、10 参考；作用域
合计 77、12、10 与参考 ID 数一致）。任何校验失败即退出非零、不写文件。

## 许可状态（发布前复核）

| 来源 | 状态 |
| --- | --- |
| TrRebootModTools 字典 | MIT；许可全文见 `research/python-v0.2/data/THIRD_PARTY_LICENSE.txt` |
| ASL-SOTTR 计数事实 | 上游未声明许可证；标注 `pending`，发布包含需 SX-018 再分发复核 |
| Stella 参考存档与攻略页 | 标注 `pending`；仅引用链接与哈希，不打包其内容 |
| 开发方案文档与内部研究记录 | 仅转录事实结论；不随产品分发 |

`redistributable` 非 `yes` 的条目不得直接进入发布包，直到许可确认或数据替换
（见 `THIRD_PARTY_LICENSES.md`）。
