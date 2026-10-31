# 第三方组件与许可清单

> 状态说明：本清单记录仓库内第三方内容的来源与许可状态。产品自身许可证尚未选择（见文末）。

## 1. 内部 ID 名称字典（TrRebootModTools）

- 内容：`research/python-v0.2/data/collectible_id_candidates.json`（866 个不同 ID / 871 个名称）
- 来源：<https://github.com/arcusmaximus/TrRebootModTools>
- 固定提交：`a79c48b75f1ffaaf623dc73a625211bf98ff0504`
- 许可：MIT。许可全文随数据保存：`research/python-v0.2/data/THIRD_PARTY_LICENSE.txt`

## 2. 区域计数映射数据（社区来源、独立转录）

- 内容：`research/python-v0.2/data/region_counter_candidates.json`、`research/python-v0.2/data/reference_category_ids.json`
- 来源事实：Atorizil/ASL-SOTTR（社区自动分章脚本），固定提交 `b758ee890eac30d9e3b3477999a30f8f9e442f00`
- 说明：仅独立转录的事实与本地交叉推导；未复制上游源代码；上游未声明许可证
- 状态：**待处理（分发前复核）**——发布包（SX-018）不得包含该数据，直至再分发许可确认或数据替换为独立来源；本地开发与研究使用不受影响

## 3. 公开参考存档（Stella 攻略站）

- 未随本仓库分发；仅在本地只读研究中使用
- 来源、下载归档与哈希见研究包 `tool/evidence/public_sources.json`（仓库外）

## 4. 元数据数据集（`data/metadata/`，SX-008）

- 内容：866 个内部 ID 条目、43 项区域计数映射、参考集合与攻略组（详见 `data/metadata/README.md`）
- 来源与状态：
  - TrRebootModTools 字典转录：MIT（许可全文随 `research/python-v0.2/data/THIRD_PARTY_LICENSE.txt`）
  - ASL-SOTTR 计数事实（独立转录与比对）：**待处理**——上游未声明许可证；发布包含需 SX-018 再分发复核
  - Stella 参考存档哈希与攻略组链接：**待处理**——仅引用链接与哈希，不打包存档或攻略内容
  - 开发方案文档与内部研究记录：仅转录事实结论，`redistributable: no`，不随产品分发
- 加载器拒绝执行元数据中的任何脚本或命令；数据文件不包含私人路径、原始存档或商业资源

## 5. Rust 与 JavaScript 依赖

- 精确版本见 `Cargo.lock` 与 `apps/desktop/pnpm-lock.yaml`
- 依赖许可审计与发布包许可通知在 SX-018 验证；当前不宣称已完成审计
- `crates/sottr-core` 发布依赖仅 serde、serde_json 与 flate2（MIT/Apache-2.0 许可族）；
  `serde_json` 自 SX-008 起用于元数据加载（此前为 dev-dependency，无新增第三方包）
- 元数据内容哈希使用核心内置的最小 SHA-256 实现（无第三方哈希依赖）

## 6. 脚手架模板与占位图标

- 项目骨架由 create-tauri-app 4.7.5（Tauri Programme，MIT / Apache-2.0 双许可）生成
- `apps/desktop/src-tauri/icons/` 为模板默认占位图标（Tauri 标识），仅用于本地脚手架；发布前（SX-018）必须替换为自行设计的应用图标

## 7. 研究包与私人材料

- 真实存档样本、私人分析输出、完整研究包与基线复跑输出均在仓库外私有目录
- `research/python-v0.2/` 为授权交接包内程序的副本（SX-001 基线，见 `docs/baseline.md`）

## 8. 产品许可证

尚未选择。仓库暂不包含 `LICENSE` 文件；选择许可证与公开发布为单独决策，未获授权前不执行。

## 9. SX-019 研究来源（游戏资源提取，2026-10-17）

- TrRebootModTools（<https://github.com/arcusmaximus/TrRebootModTools>），固定提交
  `a79c48b75f1ffaaf623dc73a625211bf98ff0504`：其二进制模板与列表模板（`tr11zones`、
  `tr11collectibles`）用作对象布局与单位命名的格式参考（MIT，字典来源见第 1 节同源仓库）；
  未复制源码进入本仓库
- CDCE.TIGER.Tool（<https://github.com/Ekey/CDCE.TIGER.Tool>），固定提交
  `4c02ae12d83894b348f49d094bbdee7fbf45526d`：仅作容器格式交叉核对（源码阅读）；
  **上游未声明许可证**，未复制源码，不进入产品与发布包
- 本地游戏资源（tiger/drm/locals.bin）提取结果：原始数据与映射候选表留在仓库外私有目录；
  事实性映射（ID→区域/单位/放置/坐标）是否随交付版发布按方案 G4 出口的
  “证据与再分发门禁逐类复核”执行，当前不声明已完成
- 方法与发现详见 `research/s019/README.md`、`research/s019/findings.md`

## 10. 真实地图底图（`data/map/`，2026-10-31）

- 来源：spriters-resource 作者 **vi7ual** 的三份投稿（2025-12-18 提交），共 44 张地图贴图：
  `502127`「Map (1/3)」24590–24607、`502132`「Map (2/3)」24608–24622、
  `502133`「Map (3/3)」24623–24637（后两个标题按同批投稿推断）
- 站点 About 页（<https://www.spriters-resource.com/page/about/>）说明：
  「These resources are collected for **personal projects and non-commercial work**.」
- 本项目的处理：按**非商业用途随包分发**，`sources[*].redistributable = allowed`；
  处理后的 WebP 位于 `data/map/maps/`（约 48 MB），原始 PNG 留在开发工作区未入库
- 素材版权归原权利人；**商业用途需另行取得许可**
- 记录与推导见工作区 `doc/2026-10-19-真实地图/真实地图方案.md` 的 P0-05 / P0-06 与
  `data/map/README.md`
