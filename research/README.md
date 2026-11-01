# 研究资料（research）

产品数据的事实来源与可复现脚本。原始商业资源、玩家存档与提取产物一律留在仓库外
（`sottr-explorer-devel/private/`），仓库里只保留脚本、派生映射与结论。

## 两个包

| 包 | 读什么 | 产出 | 谁在消费 |
| --- | --- | --- | --- |
| [`save-inspector/`](save-inspector/README.md) | 玩家存档（`*.dat`） | 解压/哈希/差异报告；名称与区域计数字典 | `cargo test -p sottr-core --test parity` 的参照实现；`data/metadata/build_metadata.py`、`data/catalog/build_catalog.py` 的字典输入 |
| [`game-data/`](game-data/README.md) | 游戏安装包（`bigfile*.tiger`） | 容器格式库、提取脚本、口径对账、区域/名称/位置派生表 | `data/catalog/build_catalog.py` 的解析库与派生表输入；`data/catalog/README.md` 与多处代码注释引用的证据 |

## 流水线

```text
玩家存档 ──save-inspector──► 观察 ID / 区域计数器 ──► data/metadata/*.json

游戏安装包 ──game-data/extract_*──► 原始提取物（仓库外）
                                     ├─ reconcile ─────────► notes/reconciliation.md 结论
                                     └─ build_derived_tables ─► game-data/data/*.json
                                                                 └─► data/catalog/collectibles_catalog.json
```

`game-data/notes/` 四篇记录按顺序阅读：

1. [`extraction.md`](game-data/notes/extraction.md)：环境、输入、容器格式、脚本与运行示例
2. [`findings.md`](game-data/notes/findings.md)：提取到的事实、假说与未解问题
3. [`reconciliation.md`](game-data/notes/reconciliation.md)：全收集口径对账与否定结果
4. [`derived-tables.md`](game-data/notes/derived-tables.md)：派生表口径、43 行验收与未闭合项

## 复现与校验

所有脚本都需要仓库外的提取物与游戏安装目录，命令见各包 README。数据产物可用
`--check` 校验与仓库内文件一致：

```powershell
py -3 data/metadata/build_metadata.py --check
py -3 data/catalog/build_catalog.py <work_dir> --check
```

## 许可边界

- 脚本与派生映射随仓库分发（代码 MIT，见根目录 `LICENSE`）；
- 名称与 zone 命名来自 TrRebootModTools（MIT）；
- 区域计数器来源目标来自 ASL-SOTTR（事实性数据，见 `THIRD_PARTY_NOTICES.md`）；
- 原始商业资源转储与玩家存档不入库（`.gitignore` 的 `*.dat`、`samples/`、`analysis*/`、`private/`）。
