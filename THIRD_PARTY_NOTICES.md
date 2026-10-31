# 第三方通知与许可声明

> 本文件只记录许可、来源与归属，不记录开发进度。
> 随应用分发的离线全文由 `apps/desktop/scripts/build_notices.py` 生成到
> [`THIRD_PARTY_NOTICES.txt`](THIRD_PARTY_NOTICES.txt)，并在「设置与关于」页内展示。
> 各数据集的结构、生成方式与验收细节见对应目录的 README。

## 1. 产品许可

SOTTR Explorer 的源代码采用 **MIT**，全文见根目录 [`LICENSE`](LICENSE)。

MIT **不覆盖**以下内容，它们各自保留原权利与条款，不因本仓库的 MIT 许可而被再许可：

| 范围 | 说明 |
| --- | --- |
| `data/map/maps/`、`data/map/basemaps.json` | 地图底图，作者 vi7ual；见第 5 节 |
| `data/catalog/`、`data/metadata/` | 《古墓丽影：暗影》游戏数据事实；见第 6 节 |
| `licenses/`、`THIRD_PARTY_NOTICES.txt` | 第三方组件与数据来源；见第 3、4 节 |

「Shadow of the Tomb Raider」及相关名称、美术与商标归 Square Enix、Crystal Dynamics
及各自权利人所有。本项目是只读、非官方的存档分析工具，与上述公司无从属关系，
也未获其背书。

## 2. 数据集与来源

| 数据集 | 位置 | 详细说明 |
| --- | --- | --- |
| 全收集图鉴 | `data/catalog/` | [`data/catalog/README.md`](data/catalog/README.md) |
| 收集品元数据（ID、区域计数、参考集合） | `data/metadata/` | [`data/metadata/README.md`](data/metadata/README.md) |
| 真实地图基底层 | `data/map/` | [`data/map/README.md`](data/map/README.md) |

来源分三类：

- **工具来源**：TrRebootModTools（MIT）的内部 ID 字典与列表模板。
- **事实来源**：游戏自身数据（`locals.bin`、存档字节、gameflow）中提取的 ID、名称、
  坐标与计数；社区来源（ASL-SOTTR、Stella、3DM、游民星空、B 站）只转录事实或引用链接。
- **研究材料**：私人存档、提取物与完整研究包位于仓库外，不随产品分发。

社区来源的逐条状态（链接、固定提交、`redistributable` 标记）记录在各数据文件的
`sources` 字段中，机器可读；本文件不再重复。

## 3. ASL-SOTTR 区域计数：事实性数据（已确认）

`data/metadata/region_counters.json` 的 43 条映射记录「存档哪个字节偏移存放哪个区域的
哪个类别计数」，来源是 Atorizil/ASL-SOTTR 的 `SOTTR.asl`（固定提交
`b758ee890eac30d9e3b3477999a30f8f9e442f00`，第 256–344 行）。

**上游未声明许可证。本仓库的定性：转录的事实数据，不构成上游代码的再分发。**

理由链：

1. **只转录数据，不复制代码**：仓库内只有 43 组「区域 / 类别 → 偏移」的数值与其
   上游名称拼写，没有任何 ASL 脚本源码或结构。
2. **独立交叉验证，而非照抄**：偏移由内存 stride 272 字节与存档 stride 104 字节
   交叉推导；公开的早期与 100% 存档样本佐证；图鉴逐行对账时与游戏自带计数
   43/43 完全吻合。
3. **上游本身未记录存档偏移**：ASL 脚本面向内存读取，存档偏移是本项目独立推导的
   结论，不是上游的表达式或创意。

因此 2026-10-31 由项目所有者确认，将其状态由 `pending` 改为 `yes`（可随产品分发），
并在 `build_metadata.py` 中记录该结论。若将来上游声明许可证，按更严格的条款复核，
不影响现有结论。

## 4. 依赖组件

依赖的完整清单（包名、版本、`license` 字段）只在随应用分发的
[`THIRD_PARTY_NOTICES.txt`](THIRD_PARTY_NOTICES.txt) 中维护，由脚本生成：

- Rust registry 包 **461 个**（不含本仓库 workspace 包），全部带 `license` 字段；
- **生产** JavaScript 依赖 **27 个**，全部带 `license` 字段。

仓库不再另存依赖清单副本，避免与通知正文不一致。开发依赖不随产品分发，不在通知范围内。

需要单独留意的许可：

- **MPL-2.0（Rust 5 个）**：`cssparser`、`cssparser-macros`、`dtoa-short`、`option-ext`、
  `selectors`；JavaScript 另有 2 个。弱著佐权，只覆盖其自身源文件。按 MPL-2.0 §3.2，
  以目标代码分发时须告知接收者如何取得对应源代码：上述包可按 `name` 与 `version`
  在 <https://crates.io> 与 <https://www.npmjs.com> 取得原始源代码包。- **Unicode-3.0（Rust 18 个）**：`icu_*`、`litemap`、`tinystr`、`writeable`、`yoke`、
  `zerofrom`、`zerotrie`、`zerovec` 等，允许再分发，需保留其版权与许可声明。
- **BSL/其他**：`BlueOak-1.0.0`（JavaScript 5 个）、`CC0-1.0`、`Python-2.0`、
  `Zlib`、`0BSD`、`Unlicense`、`ISC`、`BSD-2/3-Clause`、`MIT-0` 均为宽松许可；
  `brotli` 为 `BSD-3-Clause AND MIT`，两者条件同时适用。
- **LGPL-2.1-or-later（可选）**：`r-efi` 标注
  `MIT OR Apache-2.0 OR LGPL-2.1-or-later`，本项目按 MIT / Apache-2.0 使用。

重新生成（仓库根目录，需要 `cargo` 与 `pnpm`）：

```powershell
py -3 apps/desktop/scripts/build_notices.py
```

脚本调用 `cargo metadata` 与 `pnpm licenses list --json`；任一失败时对应小节标注
「待复核」，不虚构内容。

## 5. 地图底图

底图来自 spriters-resource 的三份投稿，作者 **vi7ual**（2025-12-18 提交）：

| 投稿 | 标题 | 覆盖 |
| --- | --- | --- |
| 502127 | Map (1/3) | 24590–24607 |
| 502132 | Map (2/3) | 24608–24622（标题按同批投稿推断） |
| 502133 | Map (3/3) | 24623–24637（标题按同批投稿推断） |

站点 About 页（<https://www.spriters-resource.com/page/about/>）写明素材供
「personal projects and non-commercial work」。本项目据此以**非商业**方式随包分发
处理后的 WebP（`data/map/maps/`，42 张，约 48 MB），`sources[*].redistributable`
记为 `allowed`；原始 PNG 不入库。

素材版权仍归原权利人。**商业用途需另行取得许可**，MIT 许可不延伸至这些图片。

## 6. 游戏数据事实

`data/catalog/` 与 `data/metadata/` 中的名称、区域、坐标与计数是《古墓丽影：暗影》
的数据事实，由本机游戏文件与存档提取、整理。项目所有者已于 2026-10-17 确认这些
数据可随交付版发布。

该确认只涉及事实数据的再分发，不构成对游戏素材、美术或商标的许可。游戏内中英名称
来自 `locals.bin` 的表内对齐，不打包任何本地化文件或原始资源。

## 7. 脚手架与图标

- 项目骨架由 create-tauri-app 4.7.5（Tauri Programme，MIT / Apache-2.0 双许可）生成。
- `apps/desktop/src-tauri/icons/` 当前仍是模板默认图标（Tauri 标识），仅用于本地
  脚手架；**发布前必须替换为自行设计的应用图标**。
- 界面不加载自带字体：`styles.css` 只用 CSS 变量引用系统字体，无 `@font-face`。

## 8. 研究来源（未复制代码）

- **TrRebootModTools**（<https://github.com/arcusmaximus/TrRebootModTools>，固定提交
  `a79c48b75f1ffaaf623dc73a625211bf98ff0504`）：内部 ID 名称字典按 MIT 转录，
  许可全文见 [`licenses/TrRebootModTools-MIT.txt`](licenses/TrRebootModTools-MIT.txt)；
  二进制/列表模板（`tr11zones`、`tr11collectibles`）只作对象布局与命名格式参考。
- **CDCE.TIGER.Tool**（<https://github.com/Ekey/CDCE.TIGER.Tool>，固定提交
  `4c02ae12d83894b348f49d094bbdee7fbf45526d`）：只作容器格式交叉核对（源码阅读）；
  上游未声明许可证，**未复制源码，不进入产品与发布包**。
- 社区攻略与视频（Stella、3DM、游民星空、B 站）：只记录链接、哈希与事实线索，
  不打包攻略正文、图片或视频。

## 9. 发布前待办

- 替换 `apps/desktop/src-tauri/icons/` 的 16 个模板图标。
- 随二进制发布各依赖的许可全文（当前仓库只提供清单与 TrRebootModTools 全文；
  MPL-2.0 覆盖的包按第 4 节给出源代码获取方式）。打包脚本可在
  `build_notices.py` 基础上扩展。
- 每次发布前重跑 `build_notices.py`，并确认 `data/*/README.md` 的来源状态无变化。
