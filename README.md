# sottr-explorer（SOTTR Explorer）

Shadow of the Tomb Raider（Windows Steam 普通战役存档）本地只读分析工具。
按已确认开发方案组织：Rust 解析核心 + 独立 CLI + Tauri/Vue 桌面端；Python v0.2 研究程序作为回归参照保留。

## 现状

契约阶段：研究基线（SX-001）、仓库骨架（SX-002）与版本化接口契约 1.0.0（SX-003）已完成；
解析核心（G1：SX-004 至 SX-007）、版本化元数据（G2：SX-008）、同核心分析 CLI
（G2：SX-009）、Python v0.2 差分回归（G2：SX-010）与 JSON/CSV 导出及分享脱敏
（G2：SX-011）已实现；G2 出口与界面仍为占位骨架，待后续任务评审/接入。

- 基线复现记录：[docs/baseline.md](docs/baseline.md)
- 接口契约与夹具说明：[docs/contracts.md](docs/contracts.md)
- 元数据数据集（来源、生成与许可状态）：[data/metadata/README.md](data/metadata/README.md)
- 开发方案与任务状态在开发工作区 `plan/` 目录维护，不在本仓库

## 目录结构

| 路径 | 说明 |
| --- | --- |
| `crates/sottr-core/` | 只读解析与领域核心（纯库：不访问 UI、网络或文件系统；依赖 serde、serde_json 与 flate2） |
| `crates/sottr-core/tests/parity/` | Python v0.2 差分回归（SX-010）：规范化适配器、私有/公开夹具清单与差异报告 |
| `crates/sottr-cli/` | 命令行入口（`analyze` 批量分析；`export` 导出信封与 CSV，支持分享脱敏） |
| `apps/desktop/` | Tauri 2 后端 + Vue 3/Vite/Tailwind 4 前端（pnpm） |
| `data/metadata/` | 版本化收集品元数据（SX-008）：三个数据集文件、manifest 与转换脚本 |
| `research/python-v0.2/` | 研究基线 Python 程序与字典副本（回归参照，无私人样本） |
| `schemas/` | 报告、批量、桌面信封、导出与元数据 JSON Schema（契约 1.0.0） |
| `fixtures/contract/` | 契约夹具：合法 / 非法 / 表示规则 / Python 适配示例 |
| `docs/` | 基线记录与契约文档 |

## 构建与检查

需要 Rust（stable；SX-002 执行时验证 rustc/cargo 1.98.1）与 Node.js + pnpm（验证 Node 22 / pnpm 10.33）。

```powershell
cargo build                    # 默认成员：核心与 CLI（无 Tauri 依赖）
cargo test
cargo tree -p sottr-core       # 检查：核心依赖（serde、serde_json、flate2；无 Tauri/网络/文件发现）
cargo check -p sottr-desktop   # 桌面端后端（Tauri 2；首次编译较慢）

# 只读批量分析（输出目录必须不存在；profile.dat 自动按资料文件处理）
cargo run -p sottr-cli -- analyze --input <样本目录> --context campaign `
    --output <新目录> --metadata data/metadata

# 单文件导出（导出信封 report.json + 逐项 report.csv；--share 去除绝对路径）
cargo run -p sottr-cli -- export --input <存档文件> --context campaign `
    --output <新目录> --metadata data/metadata --format both --share

# Python v0.2 差分回归（可选夹具；未设置时安静跳过并记录覆盖缺口）
$env:SOTTR_SAMPLE_DIR = "<私有五文件目录>"     # profile.dat + save1/save202/save204/save206
$env:SOTTR_PUBLIC_DIR = "<公开样本目录>"       # 按来源恢复的 20 个公开战役样本
cargo test -p sottr-core --test parity

py -3 data/metadata/build_metadata.py --check   # 元数据与 manifest 哈希一致

cd apps/desktop
pnpm install
pnpm build                     # vue-tsc 类型检查 + vite 构建
pnpm contract:validate         # 契约夹具与 data/metadata 真实数据：合法通过 / 非法拒绝 / u64 表示边界
pnpm contract:document -- --schema batch --file <analysis.json>   # 复核 CLI 输出
pnpm contract:types            # 由 schemas/ 重新生成 src/contract/*.d.ts
pnpm contract:check            # 上述校验 + 类型生成 + vue-tsc 联合检查
```

Python 研究参照（仅标准库，Python 3.10+）：

```powershell
cd research/python-v0.2
$env:SOTTR_SAMPLE_DIR = "<私有样本目录>"   # 未设置时真实夹具测试跳过
py -3 -m unittest discover -s tests -v
```

## 安全边界

- 只读：任何路径都不写回、修复或覆盖原存档；默认无账号、遥测与后台上传
- `sottr-core` 不接收路径、不访问文件系统；平台路径发现与稳定快照由适配层承担（SX-012/SX-013）
- 未知状态显式 `unknown`，能力不适用显式 `unsupported`；不用 0 或空清单冒充结论
- 私人存档、私人输出与完整研究包不在本仓库；见 `.gitignore` 的私人材料规则

## 许可与来源

- 产品许可证尚未选择（仓库暂不含 `LICENSE`，属刻意状态，需单独决策）
- 第三方内容来源、许可与待处理项见 [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)
- 精确依赖版本由 `Cargo.lock` 与 `apps/desktop/pnpm-lock.yaml` 固定
