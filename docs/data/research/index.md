# 研究记录

产品里的名称、区域、单位、坐标与计数，来自两条研究线：玩家存档的只读解析参照，以及游戏安装包
的提取与对账。这里收录公开整理版；原始提取数据、存档与研究包留在仓库外，不随产品分发。

::: tip 这些页面是什么
研究记录描述“事实是怎么得到的”，不是产品契约。产品字段的含义与状态以
[契约 1.0.0](/contracts)和[数据集说明](/data/)为准。
:::

## 两条研究线

| 研究线 | 读什么 | 产出 | 谁在消费 |
| --- | --- | --- | --- |
| [存档解析参照](https://github.com/perillaroc/sottr-explorer/tree/main/research/save-inspector) | 玩家存档 `*.dat` | 解压、哈希、差异报告；名称与区域计数字典 | 差分回归参照；`build_metadata.py`、`build_catalog.py` 的字典输入 |
| [游戏数据提取](https://github.com/perillaroc/sottr-explorer/tree/main/research/game-data) | 游戏安装包 `bigfile*.tiger` | 容器格式库、提取脚本、口径对账、区域/名称/位置派生表 | `data/catalog/build_catalog.py` 的解析库与派生表输入 |

## 公开页面

| 页面 | 内容 |
| --- | --- |
| [研究基线（SX-001）](/data/research/baseline) | 交接包校验、测试复现、五文件分析与历史输出对比 |
| [提取与容器格式（SX-019）](/data/research/extraction) | TIGER / CDRM / DRM / DTP 格式与提取脚本 |
| [研究发现（SX-019）](/data/research/findings) | 提取到的事实、假说与未解问题 |
| [全收集对账（SX-020）](/data/research/reconciliation) | 100% 口径对账与否定结果 |
| [整体完成度百分比（P3-1）](/data/research/completion) | 0x28 的公式结构、已验证部分与未闭合项 |
| [DTP 64 位文本键（P3-2）](/data/research/text-keys) | 文本键映射的否证结果与产品的替代路径 |
| [派生表口径（SX-022）](/data/research/derived-tables) | 区域、单位、中英名、位置与 43 行验收 |

## 公开版做了什么处理

- 开发工作区的私有路径替换为 `<game_dir>`、`<work_dir>`、`<trreboot_tools_dir>` 等占位符；
- 存档字符串、提取物、验证日志、私有研究包不进入仓库与站点；
- 技术事实、脚本方法、对账数字与结论保持原样。

本页与下列四篇公开页是研究记录的正文。产品数据集与契约为最终口径。

## 研究边界

- 全程只读打开游戏文件与存档；不修改、不写回；
- 原始商业资源只在本地读取，仓库与站点不包含资源转储；
- 名称与 zone 命名来自 TrRebootModTools（MIT）；区域计数事实来自 ASL-SOTTR；
- 研究结论中标注为“假说”的内容不得直接进入产品图层。

来源与许可的完整说明见[来源与许可](/data/sources)与[第三方声明](/legal/third-party)。
