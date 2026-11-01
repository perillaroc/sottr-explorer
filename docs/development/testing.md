# 测试

## 测试矩阵

| 套件 | 命令 | 依赖 | 说明 |
| --- | --- | --- | --- |
| 核心单元 | `cargo test -p sottr-core` | 无 | 解析、计数、评估、定位门禁、导出 |
| 契约夹具 | `cargo test -p sottr-core --test contract_fixtures` | `fixtures/contract/` | 合法夹具通过、非法夹具被拒 |
| 数据集 | `cargo test -p sottr-core --test {metadata_dataset,basemap_dataset,catalog_dataset,atlas_plan,group_gate}` | 仓库内数据 | 数量、哈希、交叉一致性、黄金输出 |
| 差分回归 | `cargo test -p sottr-core --test parity` | `SOTTR_SAMPLE_DIR` / `SOTTR_PUBLIC_DIR` | 与 Python v0.2 逐层对照 |
| 全量 Rust | `cargo test` | 视套件 | 包含平台层与 CLI |
| 前端 | `cd apps/desktop && pnpm test` | jsdom | 组件、状态、端到端（假后端 IPC） |
| 契约校验 | `cd apps/desktop && pnpm contract:check` | Node | schema 校验 + 类型生成 + `vue-tsc` |
| 图片校验 | `cd apps/desktop && pnpm contract:images` | 底图文件 | 存在性、字节数、SHA-256、WebP 尺寸 |
| Python 参照 | `python -m unittest discover -s research/save-inspector/tests -v` | 标准库 | 29 项，含可选真实样本 |
| 文档站 | `cd docs && pnpm docs:build` | Node | 构建失败包含死链 |

## 跳过不是通过

差分回归与真实样本测试依赖环境变量；缺少夹具时会**安静跳过并记录覆盖缺口**，不记为通过。
CI 或发布记录应把这些缺口写出来，而不是只看“0 failed”。

常用变量：

- `SOTTR_SAMPLE_DIR`：私有五文件；
- `SOTTR_PUBLIC_DIR`：按来源恢复的公开样本；
- `SOTTR_PARITY_OUT_DIR`：报告与原始输出基目录（默认 `target/tmp/parity`）；
- `SOTTR_PYTHON`：解释器；
- `SOTTR_DESKTOP_E2E_INPUT` / `_OUT` / `_EXPORT_DIR` / `_RESOLVE_SOURCE` / `_RESULT`：桌面端端到端。

## 黄金输出

`atlas_plan` 用 CLI 输出对照前端 `src/data/atlas.ts` 的镜像实现，逐字段比较。任何一侧改动
计划逻辑都必须同时更新黄金输出；不要把差异“归一化”掉。

## 新增夹具

1. 先改 `schemas/*.schema.json` 与契约说明；
2. 在 `fixtures/contract/{valid,invalid,representation,adapted}/` 增加用例；
3. 在 Rust 测试与 `apps/desktop/scripts/contract/validate.mjs` 中登记；
4. 跑 `cargo test -p sottr-core --test contract_fixtures` 与 `pnpm contract:check`。

## 不随仓库分发的样本

公开 20 样本需按来源恢复，私有五文件只在开发工作站；样本字节不提交进仓库。校准与恢复清单见
`crates/sottr-core/tests/parity/fixtures/*.json`。
