# 发布

## 版本字段

应用版本只有一个来源：`Cargo.toml` 的 `[workspace.package].version`。Tauri 配置不写
`version`，构建时回退到 `src-tauri/Cargo.toml`（即工作区版本）；前端由 Vite 在构建期注入
同一值，打包后「关于」页再用 Tauri `getVersion()` 读取。发版只改这一行，跑
`cargo check --workspace` 更新 `Cargo.lock`；发布工作流会校验 tag 与 Cargo 版本一致。

| 字段 | 作用 | 变更规则 |
| --- | --- | --- |
| `tool_version` | 应用构建号 | 跟随 `Cargo.toml` 的 workspace 版本，无需单独修改 |
| `schema_version` | 报告契约版本，当前固定 `1.0.0` | 字段含义、枚举或单位变化即升级 |
| `catalog_version` | 图鉴数据集版本（当前 2.1.0，schema 2.1.0） | 条目、区域、坐标或指引口径变化即升级 |
| `metadata_version` | 元数据数据集版本 | 名称、计数映射或参考集合变化即升级 |
| `reference_set_version` | 参考集合版本 | 参考 ID 集合变化即升级 |

数据与契约版本独立于应用版本；应用升版时不需要改这些字段。

未知版本写成 `null` 并在 `diagnostics` 说明原因，不虚构构建号。

## 分发形态

当前版本线（v0.x）只提供单文件便携 exe，不生成 MSI/NSIS 安装包：

- `apps/desktop/src-tauri/tauri.conf.json` 的 `bundle.active` 为 `false`，
  `pnpm tauri build` 只输出 `target/release/sottr-desktop.exe`；
- 产物未签名，Windows SmartScreen 会提示，发布说明必须写明；
- 运行需要 WebView2：Windows 11 已内置，Windows 10 可能需单独安装；
- 推送 `v*` tag 会触发 `.github/workflows/release.yml`：在 `windows-latest` 构建单文件 exe，
  生成 SHA-256，并创建或更新 GitHub Release；也可手动运行该工作流只检查构建；
- 工作流使用内置 `GITHUB_TOKEN`（`contents: write`），不需要额外 secrets；若仓库策略限制
  默认 token 权限，需在 Settings → Actions 中允许写权限；
- 日常检查由 `.github/workflows/ci.yml` 负责，发布工作流只负责构建与上传。

## 依赖审计

发布前运行 `cargo audit`，并在 `apps/desktop` 与 `docs` 各运行 `pnpm audit`：

- 高危/严重项必须处理；无法升级的项要写明范围（例如仅开发期工具、仅 Linux 目标）与理由；
- 只做 patch/minor 升级；Tauri、Vue、Vite、Tailwind 等大版本单独评估；
- 依赖变化后重新生成许可文件（见发布前检查第 8 步）。

## 发布前检查

1. `cargo test`、`cargo clippy --all-targets -- -D warnings`；
2. `cd apps/desktop && pnpm install && pnpm build && pnpm test`；
3. `pnpm contract:check` 与 `pnpm contract:images`；
4. `py -3 data/metadata/build_metadata.py --check`；
5. `py -3 data/catalog/check_guidance.py`（必收指引覆盖 352/352）；
6. `py -3 data/catalog/check_guides.py`（逐条攻略映射结构与覆盖计数）；
7. `py -3 data/catalog/build_catalog.py <work_dir> --check`（需要仓库外提取物）；
8. `py -3 apps/desktop/scripts/build_notices.py`，确认 `THIRD_PARTY_NOTICES.md` 与生成的两份
   txt 一致；
9. `cd docs && pnpm docs:build`，确认无死链与一致性错误；
10. 准备 GitHub Release 的版本说明，确认 `README.md` 的下载链接；
11. 打并推送 `v*` tag，由 GitHub Actions 构建并创建 Release（产物含 `.exe` 与 `.sha256`）；
   文档站由 Netlify 从 `main` 构建。

## 不随包分发

- `research/` 下的脚本、派生表与研究笔记；
- 玩家存档、公开样本、游戏提取物；
- 开发工作区中的私有工作内容；
- `data/` 下的生成脚本、`guidance.json`（只参与构建，合并结果在图鉴内）与 README；
- `fixtures/contract/` 与 `schemas/`（开发期校验用）。

## 许可

代码采用 MIT；该许可不覆盖 `data/map/` 的地图底图与 `data/catalog/`、`data/metadata/` 中的
游戏数据事实。第三方组件与来源的完整声明见[第三方声明](/legal/third-party)。

站点发布不使用客户端分析脚本，避免与“不联网、无遥测”的产品声明冲突。
