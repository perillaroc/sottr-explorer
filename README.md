# sottr-explorer（SOTTR Explorer）

Shadow of the Tomb Raider（Windows Steam 普通战役存档）本地只读分析工具。
按已确认开发方案组织：Rust 解析核心 + 独立 CLI + Tauri/Vue 桌面端；Python v0.2 研究程序作为回归参照保留。

## 现状

脚手架阶段：仓库骨架（SX-002）与研究基线（SX-001）已完成；版本化契约（SX-003）尚未冻结，
当前不提供可用的分析功能，界面为占位骨架。

- 基线复现记录：[docs/baseline.md](docs/baseline.md)
- 开发方案与任务状态在开发工作区 `plan/` 目录维护，不在本仓库

## 目录结构

| 路径 | 说明 |
| --- | --- |
| `crates/sottr-core/` | 只读解析与领域核心（纯库：不访问 UI、网络或文件系统；零第三方依赖） |
| `crates/sottr-cli/` | 命令行入口（`analyze` 等命令在 SX-009 实现） |
| `apps/desktop/` | Tauri 2 后端 + Vue 3/Vite/Tailwind 4 前端（pnpm） |
| `research/python-v0.2/` | 研究基线 Python 程序与字典副本（回归参照，无私人样本） |
| `docs/` | 基线记录与后续契约文档 |
| `schemas/` | 报告与元数据 JSON Schema（SX-003 起） |

## 构建与检查

需要 Rust（stable；SX-002 执行时验证 rustc/cargo 1.98.1）与 Node.js + pnpm（验证 Node 22 / pnpm 10.33）。

```powershell
cargo build                    # 默认成员：核心与 CLI（无 Tauri 依赖）
cargo test
cargo tree -p sottr-core       # 检查：核心无第三方依赖
cargo check -p sottr-desktop   # 桌面端后端（Tauri 2；首次编译较慢）

cd apps/desktop
pnpm install
pnpm build                     # vue-tsc 类型检查 + vite 构建
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
