# 数据管线

产品里的三套数据集来自不同输入，由不同脚本生成，被不同层消费。这张图是唯一的汇总入口；
每个数据集的细节见对应的`data/*/README.md`（GitHub）。

## 血缘

```text
玩家存档 *.dat
└─ research/save-inspector（Python v0.2 参照实现）
   └─ 观察 ID / 区域计数候选
      └─ data/metadata/build_metadata.py ──► data/metadata/*.json

游戏安装包 bigfile*.tiger
└─ research/game-data/extract_*（仓库外提取物）
   ├─ reconcile_* ─────────────────────► research/game-data/notes/*
   └─ build_derived_tables.py ──────────► research/game-data/data/*.json
                                            └─ data/catalog/build_catalog.py
                                               ──► data/catalog/collectibles_catalog.json

地图图集（仓库外）
└─ 开发工作区的地图处理脚本（不随仓库分发）
   ──► data/map/maps/*.webp + data/map/basemaps.json
```

## 数据集所有权

| 数据集 | 生成器 | 输入 | 消费者 | 契约 | 进产品包 |
| --- | --- | --- | --- | --- | --- |
| `data/catalog/collectibles_catalog.json` | `data/catalog/build_catalog.py` | 研究字典 + `research/game-data/data/*` + 仓库外提取物 | 前端 `atlas.ts`、CLI `plan`/`missing`、Rust 测试 | `schemas/catalog.schema.json` | 是（Vite 构建期 import） |
| `data/metadata/{collectibles,region_counters,reference_sets}.json` | `data/metadata/build_metadata.py` | `research/save-inspector/data/*` + 图鉴 | Rust `session.rs`（编译期嵌入）、CLI `--metadata`、测试 | `schemas/metadata.schema.json` | 是（`include_str!`） |
| `data/map/basemaps.json` + `data/map/maps/*.webp` | 开发工作区的地图处理脚本 | 地图图集 + 游戏内地图定义 | 前端、Rust 加载器 | `schemas/basemap.schema.json` | 是（42 张，约 48 MB） |
| `research/game-data/data/*.json` | `research/game-data/build_derived_tables.py` | 仓库外提取物 | 仅 `build_catalog.py` | 无独立 schema | 否 |
| `research/save-inspector/data/*.json` | 研究包转录 | 研究交接包 | `build_metadata.py`、`build_catalog.py` | 无独立 schema | 否 |
| `fixtures/contract/*` | 手工维护 | — | Rust 契约测试、Node 校验、前端模拟数据 | `schemas/*.schema.json` | 否 |

`data/metadata/manifest.json` 记录输入与输出的 SHA-256；图鉴本身也作为元数据的输入被记入清单，
防止两份收集品数据无声漂移。

## 产品包的边界

只有以下路径进入发布物：

- `data/catalog/collectibles_catalog.json`
- `data/metadata/{collectibles,region_counters,reference_sets}.json`
- `data/map/**`

`data/` 下的生成脚本、README，以及整个 `research/` 都不进包。原始商业资源、玩家存档与提取物
留在仓库外，由 `.gitignore` 的 `*.dat`、`samples/`、`analysis*/` 与私有工作区规则保护。

## 复现

需要仓库外的提取物与游戏安装目录；命令以各数据目录 README 为准：

```powershell
py -3 research/game-data/build_derived_tables.py <work_dir> <trreboot_tools_dir>
py -3 data/catalog/build_catalog.py <work_dir> --check
py -3 data/metadata/build_metadata.py --check
```

校验分三层：

```powershell
cargo test -p sottr-core --test metadata_dataset
cargo test -p sottr-core --test basemap_dataset
cargo test -p sottr-core --test catalog_dataset
cd apps/desktop
pnpm contract:check
pnpm contract:images
```

`--check` 必须与仓库文件一致；任何不一致都意味着生成器、输入或产物三者之一发生了漂移。

## 来源与许可

- 名称字典：TrRebootModTools（MIT）；
- 区域计数事实：ASL-SOTTR（事实性数据，已确认可随产品分发）；
- 参考存档与攻略页：Stella，仅引用链接与哈希，不打包内容；
- 地图底图：vi7ual，非商业用途；
- 游戏数据事实：项目所有者已确认可随交付版发布。

完整声明见[来源与许可](/data/sources)与[第三方声明](/legal/third-party)；
提取与对账过程见[研究记录](/data/research/)。
