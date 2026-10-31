# SOTTR Explorer

《古墓丽影：暗影》（Shadow of the Tomb Raider，Windows Steam 版）的**本地只读**存档分析工具。
打开你的存档，看清还差哪些收集品、它们在哪、怎么拿——原文件不会被改动一个字节。

## 它能做什么

- **概览**：当前存档的整体进度、各区域完成情况。
- **全收集（100%）**：按必收口径列出还缺的条目，区分故事收集品与非故事收集品、DLC 与挑战古墓。
- **区域与收集**：按区域 → 类别 → 条目逐层查看，标出已收集 / 未收集、所属子关卡、获取方式。
- **详情**：每个条目的攻略外链（图文与视频）、同一子关卡内的相对位置；有底图的子关卡直接画在地图上。
- **诊断**：每条状态判断背后的原始证据与数据来源，结论可追溯。

所有判断都基于"存档里出现了哪些 ID"，不是估算；不确定的地方会明确写"未知"，不会用 0 或空清单冒充结论。

## 怎么用

1. 启动应用，选择存档文件（默认扫描 `文档\Shadow of the Tomb Raider\*.dat`，也可以手动挑选）。
2. 等待分析完成，在左侧切换「概览 / 全收集 / 区域与收集」。
3. 点任意条目查看详情：怎么获取、外部攻略、在同一子关卡中的相对位置。

存档是只读打开的：工具不会写回、修复或覆盖原文件，也不会替你挑选或删除文件。

### 关于位置与地图

- 位置是**关卡内相对位置**，不是世界坐标，也不是导航路线。
- 42 个子关卡配有游戏内地图底图。点位按每张图自己的变换绘制：其中 7 个单位经过抽样验证，
  界面标「已校准」并给出中位误差与 P95；其余标「近似位置」。
- 没有底图的区域继续显示散点，并明确标注「坐标未校准」。
- 底图只在你展开「同一子关卡内的相对位置」时才加载。

## 隐私与安全

- 不联网：不加载任何网页，不上传存档，无账号与遥测。外部攻略链接交给系统浏览器打开。
- 只读：不写回、不修复、不改名、不删除你的存档。
- 输入输出只支持本地卷，拒绝 UNC 与设备路径；导出必须写到不存在的新目录，不会覆盖已有文件。
- 私人存档与研究数据不在本仓库内。

## 数据与来源

- 全收集图鉴（区域 / 子关卡 / 中英名 / 获取方式 / 相对位置）：[data/catalog/README.md](data/catalog/README.md)
- 真实地图基底层（42 个单位、坐标与状态口径）：[data/map/README.md](data/map/README.md)
- 收集品元数据：[data/metadata/README.md](data/metadata/README.md)
- 第三方来源与许可：[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)
- 地图底图来自 spriters-resource 作者 **vi7ual**（投稿 502127 / 502132 / 502133，2025-12-18 提交），
  站点声明素材供 personal projects and non-commercial work；本项目按非商业用途随包分发，
  版权仍归原权利人。

## 许可

代码采用 **MIT**，见 [LICENSE](LICENSE)。该许可不覆盖地图底图（`data/map/`）与
游戏数据事实（`data/catalog/`、`data/metadata/`），它们各自保留原权利；第三方组件与
来源的完整声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

「Shadow of the Tomb Raider」及相关商标归 Square Enix / Crystal Dynamics 所有；
本项目是只读、非官方工具，与上述公司无从属关系。

## 面向开发者

### 需要

- Rust（stable）
- Node.js + pnpm
- Windows（存档发现与稳定快照读取依赖 Windows API）

### 目录

| 路径 | 说明 |
| --- | --- |
| `crates/sottr-core/` | 只读解析与领域核心（纯库：不访问 UI、网络或文件系统） |
| `crates/sottr-platform/` | 平台适配层：目录发现、路径策略、稳定快照读取与输出提交 |
| `crates/sottr-cli/` | 命令行：`analyze` / `export` / `plan` / `missing` |
| `apps/desktop/` | Tauri 2 后端 + Vue 3 / Vite / Tailwind 前端 |
| `data/` | 图鉴、元数据与地图基底层（含契约与说明） |
| `schemas/`、`fixtures/contract/` | 接口 JSON Schema 与契约夹具 |
| `docs/contracts.md` | 契约与变换格式的正式定义 |

### 构建与检查

```powershell
cargo build                         # 核心、CLI 与平台适配层
cargo test                          # 全量测试
cargo clippy --all-targets -- -D warnings

cd apps/desktop
pnpm install
pnpm build                          # vue-tsc 类型检查 + vite 构建
pnpm test                           # 组件 / 状态 / 端到端测试
pnpm contract:check                 # 契约校验 + 类型生成 + vue-tsc
pnpm contract:images                # 底图清单：存在性、字节数、SHA-256、WebP 尺寸
pnpm dev                            # 开发服务器（浏览器内为模拟数据演示）
```

### 命令行

```powershell
# 只读批量分析（输出目录必须不存在）
cargo run -p sottr-cli -- analyze --input <存档目录> --context campaign --output <新目录> --metadata data/metadata

# 导出 JSON + CSV（--share 去除绝对路径）
cargo run -p sottr-cli -- export --input <存档文件> --context campaign --output <新目录> --metadata data/metadata --format both --share

# 按区域的收集计划
cargo run -p sottr-cli -- plan --catalog data/catalog/collectibles_catalog.json --analysis <分析目录>/analysis.json --output <新目录>
```

契约与数据集的正式定义见 [docs/contracts.md](docs/contracts.md)。
