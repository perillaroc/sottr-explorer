# sottr-explorer（SOTTR Explorer）

Shadow of the Tomb Raider（Windows Steam 普通战役存档）本地只读分析工具。
按已确认开发方案组织：Rust 解析核心 + 独立 CLI + Tauri/Vue 桌面端；Python v0.2 研究程序作为回归参照保留。

## 现状

契约阶段：研究基线（SX-001）、仓库骨架（SX-002）与版本化接口契约 1.0.0（SX-003）已完成；
解析核心（G1：SX-004 至 SX-007）与 G2（SX-008 至 SX-011：版本化元数据、同核心分析 CLI、
Python v0.2 差分回归、JSON/CSV 导出与分享脱敏）已实现；**G2 出口已于 2026-10-15 评审满足**
（附交接限制：稳定快照与竞态保护按 SX-013、Windows 路径策略按 SX-012、桌面接线按 SX-014）。
G3 已开始：**SX-012 完成**（共享平台适配层 `crates/sottr-platform`：Known Folder
目录发现与路径策略，CLI 已接线，Windows 实机测试通过）；**SX-013 完成**（稳定快照读取与
输出提交：句柄约束读取、读取前后指纹/摘要复核、独占新目标与受控临时文件，CLI 已接线）；
**SX-014 完成**（Tauri 最小权限命令层：选择输入、列候选、分析、取消、导出、解析/打开核验链接；
权限清单与 CSP、IPC 集成测试与权限/CSP 静态审查）；**SX-015 完成**（契约驱动的界面外壳：
槽位选择、概览、区域与收集、详情、诊断、设置；视觉令牌与明暗主题；加载/空/错误/降级/过期/
取消状态；模拟数据演示与交互验收记录）；**SX-016 完成**（真实接线与端到端：前端改用
`TauriDataSource` 调用真实命令；导出对话框（JSON/CSV、分享脱敏、目标新目录、过期标记）；
攻略链接解析/打开与失败恢复（复制已核验链接）；详情补齐表位置/参考集/组计数与来源链接；
端到端测试（假后端 IPC 全流程 + Rust 真实夹具分析→导出→链接解析 + 真实输出绑定界面）。
G4 已开始：**SX-019 完成**（游戏资源独立提取：非故事收集品 168 唯一 ID、26 区域、94 单位地图，
与私有存档观察 129/168 交叉验证；DTP 文本键关联未闭合项已记录；见 `research/s019/`）；
**SX-020 完成**（全收集口径对账：故事 260+42/DLC、挑战古墓 9、挑战 15+1 收敛；
整体完成度否定“纯收集计数”假设；约束交付 SX-021；见 `research/s020/`）；
**SX-021 完成**（全收集目录与缺失清单：目录源数据 A、核心加载 B、CLI `missing` C1、
界面页 C2a/C2b 与跨实现黄金一致性、T22 验收 D；见 `data/catalog/`）；
**SX-017 完成**（跨层验收与回归：自动化全绿、单机实测性能记录；DPI/WebView2 手工项
保持待验证；见 `private/s017_acceptance.log`）。
**SX-022 完成**（图鉴数据基础与按区域收集计划：故事条目全字段与子类别、区域哈希→ASL slug
唯一绑定（签名距离 0）、65 个子关卡命名、302/302 故事条目官方中文名（locals 双表下标对齐）、
关卡内相对位置、类别获取方式；`catalog` 升到 schema 2.0.0；新增 `sottr-core::atlas` 与
CLI `plan`；区域页/详情/概览/全收集页改为消费图鉴。端到端验收：图鉴「区域×子类别」
与存档自带区域计数器 **43/43 行完全吻合**；见 `research/s022/README.md`）。

- 基线复现记录：[docs/baseline.md](docs/baseline.md)
- 接口契约与夹具说明：[docs/contracts.md](docs/contracts.md)
- 元数据数据集（来源、生成与许可状态）：[data/metadata/README.md](data/metadata/README.md)
- 全收集图鉴（区域 / 子关卡 / 中英名 / 相对位置 / 获取方式）：[data/catalog/README.md](data/catalog/README.md)
- 图鉴数据基础研究（区域绑定、本地化对齐与验收）：[research/s022/README.md](research/s022/README.md)
- 开发方案与任务状态在开发工作区 `plan/` 目录维护，不在本仓库

## 目录结构

| 路径 | 说明 |
| --- | --- |
| `crates/sottr-core/` | 只读解析与领域核心（纯库：不访问 UI、网络或文件系统；依赖 serde、serde_json 与 flate2） |
| `crates/sottr-core/tests/parity/` | Python v0.2 差分回归（SX-010）：规范化适配器、私有/公开夹具清单与差异报告 |
| `crates/sottr-cli/` | 命令行入口（`analyze` 批量分析；`export` 导出信封与 CSV，支持分享脱敏） |
| `crates/sottr-platform/` | 平台适配层（SX-012/SX-013/SX-014）：Windows Known Folder 目录发现、有界枚举、路径策略、稳定快照读取（句柄约束与读取前后校验）、输出提交（独占新目标/受控临时文件）与快照时钟；CLI 与桌面后端共用，零第三方依赖 |
| `apps/desktop/` | Tauri 2 后端 + Vue 3/Vite/Tailwind 4 前端（pnpm）。后端命令层（SX-014）：`discover_saves`/`list_candidates`/`select_input`/`analyze_file`/`cancel_analysis`/`export_report`/`resolve_guide_link`/`open_guide_link`；权限清单 `capabilities/default.json` + `permissions/sottr-commands.toml`；CSP 与导航门禁在 `tauri.conf.json`/`src/navigation.rs`。前端（SX-015/SX-016）：外壳/槽位选择/概览/区域与收集/详情/诊断/设置 + 视觉令牌 + 真实数据源 `TauriDataSource`（`src/data/tauri.ts`）与模拟数据源；导出对话框与攻略链接动作；组件/状态/端到端测试（`tests/`，80 项 + 1 项真实输出绑定（需环境变量）） |
| `data/metadata/` | 版本化收集品元数据（SX-008）：三个数据集文件、manifest 与转换脚本 |
| `data/catalog/` | 全收集图鉴（SX-021 建目录；SX-022 schema 2.0.0：区域 / 子关卡 / 中英名 / 相对位置 / 获取方式）：生成脚本、契约与说明 |
| `research/s022/` | 图鉴数据基础研究（SX-022）：区域绑定、子关卡命名、本地化对齐与验收脚本（无商业数据转储） |
| `research/python-v0.2/` | 研究基线 Python 程序与字典副本（回归参照，无私人样本） |
| `research/s019/` | G4 研究记录（SX-019）：游戏资源与本地化关联的独立提取脚本及发现（无商业数据转储） |
| `research/s020/` | G4 验证记录（SX-020）：全收集口径逐类对账、否定结果与 SX-021 约束（含脚本） |
| `schemas/` | 报告、批量、桌面信封、导出与元数据 JSON Schema（契约 1.0.0） |
| `fixtures/contract/` | 契约夹具：合法 / 非法 / 表示规则 / Python 适配示例 |
| `docs/` | 基线记录与契约文档 |

## 构建与检查

需要 Rust（stable；SX-002 执行时验证 rustc/cargo 1.98.1）与 Node.js + pnpm（验证 Node 22 / pnpm 10.33）。

```powershell
cargo build                    # 默认成员：核心、CLI 与平台适配层（无 Tauri 依赖）
cargo test
cargo tree -p sottr-core       # 检查：核心依赖（serde、serde_json、flate2；无 Tauri/网络/文件发现）
cargo check -p sottr-desktop   # 桌面端后端（Tauri 2；首次编译较慢）
cargo test -p sottr-platform   # 平台适配层：目录发现/路径策略/稳定快照/输出提交（含 Windows 实机：Known Folder、链接与重解析点、ACL、长路径、占用与路径替换）
cargo test -p sottr-desktop    # 桌面命令层：IPC 集成测试（MockRuntime）+ 权限/CSP 静态审查（SX-014）

# 只读批量分析（输出目录必须不存在；profile.dat 自动按资料文件处理）
cargo run -p sottr-cli -- analyze --input <样本目录> --context campaign `
    --output <新目录> --metadata data/metadata

# 单文件导出（导出信封 report.json + 逐项 report.csv；--share 去除绝对路径）
cargo run -p sottr-cli -- export --input <存档文件> --context campaign `
    --output <新目录> --metadata data/metadata --format both --share

# 按区域的收集计划（图鉴 + 存档观察集合 → 逐区域/逐项状态、子关卡、相对位置与获取方式）
cargo run -p sottr-cli -- plan --catalog data/catalog/collectibles_catalog.json `
    --analysis <分析目录>/analysis.json --output <新目录> [--input save1.dat]

# Python v0.2 差分回归（可选夹具；未设置时安静跳过并记录覆盖缺口）
$env:SOTTR_SAMPLE_DIR = "<私有五文件目录>"     # profile.dat + save1/save202/save204/save206
$env:SOTTR_PUBLIC_DIR = "<公开样本目录>"       # 按来源恢复的 20 个公开战役样本
cargo test -p sottr-core --test parity

py -3 data/metadata/build_metadata.py --check   # 元数据与 manifest 哈希一致

cd apps/desktop
pnpm install
pnpm build                     # vue-tsc 类型检查（含 tests/）+ vite 构建
pnpm test                      # 组件/状态/端到端测试：Vitest + jsdom（SX-015/SX-016：键盘、焦点、宽度、状态隔离、导出与链接流程、假后端 IPC 全流程）
#   真实输出绑定（可选）：$env:SOTTR_DESKTOP_E2E_RESULT = "<Rust 端到端 DesktopResult JSON>" 后重跑
pnpm contract:validate         # 契约夹具与 data/metadata 真实数据：合法通过 / 非法拒绝 / u64 表示边界
pnpm contract:document -- --schema batch --file <analysis.json>   # 复核 CLI 输出
pnpm contract:types            # 由 schemas/ 重新生成 src/contract/*.d.ts
pnpm contract:check            # 上述校验 + 类型生成 + vue-tsc 联合检查
pnpm dev                       # 前端开发服务器（http://127.0.0.1:1420；浏览器内为模拟数据演示）
#   ?mock=1 可在 Tauri 内强制模拟数据源；?mockWidth=800..1440 模拟“有效内容宽度”；
#   ?mockLinkFail=1 演示“系统浏览器打开失败”的恢复路径（均仅模拟模式）
```

Python 研究参照（仅标准库，Python 3.10+）：

```powershell
cd research/python-v0.2
$env:SOTTR_SAMPLE_DIR = "<私有样本目录>"   # 未设置时真实夹具测试跳过
py -3 -m unittest discover -s tests -v
```

## 安全边界

- 只读：任何路径都不写回、修复或覆盖原存档；默认无账号、遥测与后台上传
- `sottr-core` 不接收路径、不访问文件系统；目录发现、路径策略、稳定快照与输出提交在 `crates/sottr-platform`（SX-012/SX-013，CLI 与桌面后端共用）
- 输入以稳定快照读取：Windows 读取期间由系统拒绝其他写入/删除/改名，读取前后校验指纹与内容摘要；被占用返回可重试的 `INPUT_UNREADABLE`，读取中变化返回 `INPUT_CHANGED`（手动词刷新，不监视文件）
- 输入与输出只支持本地卷；UNC 与设备命名空间路径明确拒绝；输出必须是不存在的新目录且与输入不重叠（规范路径比较）；输出用独占新目标与受控临时文件提交，符号链接/重解析点被拒绝，失败或取消不留“完整报告”假象
- 未知状态显式 `unknown`，能力不适用显式 `unsupported`；不用 0 或空清单冒充结论
- 桌面命令层（SX-014）只开放完成“选择输入 → 列候选 → 分析 → 取消 → 导出 → 打开核验链接”所需命令；前端权限为显式最小集（`core:app/event/path/window` 默认权限 + 7 条应用命令，未开放 opener/dialog/shell/文件系统/图像/菜单/托盘/资源/WebView 查询），WebView 不加载外部攻略页面（CSP + 导航门禁），攻略链接由后端校验 http/https 后交给系统外部浏览器
- 私人存档、私人输出与完整研究包不在本仓库；见 `.gitignore` 的私人材料规则

## 许可与来源

- 产品许可证尚未选择（仓库暂不含 `LICENSE`，属刻意状态，需单独决策）
- 第三方内容来源、许可与待处理项见 [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)
- 精确依赖版本由 `Cargo.lock` 与 `apps/desktop/pnpm-lock.yaml` 固定
