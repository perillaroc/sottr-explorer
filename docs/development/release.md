# 发布

## 版本字段

| 字段 | 作用 | 变更规则 |
| --- | --- | --- |
| `schema_version` | 报告契约版本，当前固定 `1.0.0` | 字段含义、枚举或单位变化即升级 |
| `catalog_version` | 图鉴数据集版本（当前 schema 2.0.0） | 条目、区域或坐标口径变化即升级 |
| `metadata_version` | 元数据数据集版本 | 名称、计数映射或参考集合变化即升级 |
| `reference_set_version` | 参考集合版本 | 参考 ID 集合变化即升级 |
| `tool_version` | 应用构建号 | 每次发布更新 |

未知版本写成 `null` 并在 `diagnostics` 说明原因，不虚构构建号。

## 分发形态

v0.1.x 只提供单文件便携 exe，不生成 MSI/NSIS 安装包：

- `apps/desktop/src-tauri/tauri.conf.json` 的 `bundle.active` 为 `false`，
  `pnpm tauri build` 只输出 `target/release/sottr-desktop.exe`；
- 产物未签名，Windows SmartScreen 会提示，发布说明必须写明；
- 运行需要 WebView2：Windows 11 已内置，Windows 10 可能需单独安装；
- 仓库没有 CI，打 tag 不会自动构建；在本机构建后手工创建 GitHub Release 并上传 exe。

## 发布前检查

1. `cargo test`、`cargo clippy --all-targets -- -D warnings`；
2. `cd apps/desktop && pnpm install && pnpm build && pnpm test`；
3. `pnpm contract:check` 与 `pnpm contract:images`；
4. `py -3 data/metadata/build_metadata.py --check`；
5. `py -3 data/catalog/build_catalog.py <work_dir> --check`（需要仓库外提取物）；
6. `py -3 apps/desktop/scripts/build_notices.py`，确认 `THIRD_PARTY_NOTICES.md` 与生成的两份
   txt 一致；
7. `cd docs && pnpm docs:build`，确认无死链与一致性错误；
8. 准备 GitHub Release 的版本说明，确认 `README.md` 的下载链接；
9. 打 tag、在本机构建 `pnpm tauri build`，手工创建 GitHub Release 并上传 exe；
   文档站由 Netlify 从 `main` 构建。

## 不随包分发

- `research/` 下的脚本、派生表与研究笔记；
- 玩家存档、公开样本、游戏提取物；
- 开发工作区中的私有工作内容；
- `data/` 下的生成脚本与 README；
- `fixtures/contract/` 与 `schemas/`（开发期校验用）。

## 许可

代码采用 MIT；该许可不覆盖 `data/map/` 的地图底图与 `data/catalog/`、`data/metadata/` 中的
游戏数据事实。第三方组件与来源的完整声明见[第三方声明](/legal/third-party)。

站点发布不使用客户端分析脚本，避免与“不联网、无遥测”的产品声明冲突。
