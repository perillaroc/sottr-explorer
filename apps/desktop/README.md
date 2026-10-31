# SOTTR Explorer 桌面端（Tauri 2 + Vue 3 + TypeScript + Vite + Tailwind 4）

SX-014 提供最小权限命令层（后端）；SX-015 实现契约驱动的界面外壳与状态机；
**SX-016 完成真实接线与端到端**：前端在 Tauri 内使用 `TauriDataSource` 调用真实命令，
补齐导出对话框（JSON/CSV、分享脱敏、目标新目录、过期标记）、攻略链接解析/打开与失败恢复、
详情证据（表位置/参考集/组计数/来源链接）与端到端测试。纯浏览器开发（`pnpm dev`）没有 IPC，
自动退化为模拟数据源并显著标注“模拟数据”。

2026-10-31 增加三块能力：**详情面板可拖动调宽**（宽度随界面偏好持久化、双击/键盘可复位）、
**地图查询视图**（底图 + 类别图标标记、地图与「详情/清单」右栏联动、缩放平移）、
**详情「在新窗口中打开」**（Tauri 独立原生窗口；浏览器模式用 `window.open`；
窗口创建失败退回应用内模态浮层）。

```powershell
pnpm install        # 安装依赖并生成/更新 pnpm-lock.yaml
pnpm dev            # 前端开发服务器（http://127.0.0.1:1420；浏览器内为模拟数据演示）
pnpm build          # vue-tsc 类型检查（含 tests/）+ vite 构建
pnpm test           # 组件/状态/端到端测试（Vitest + jsdom；SX-015/SX-016）
pnpm contract:check # 契约夹具校验 + 由 schemas/ 生成 src/contract 类型 + 类型检查
pnpm tauri dev      # 桌面调试（真实命令；需要 WebView2 与 MSVC 工具链）
```

## 数据源与演示开关

| 场景 | 数据源 | 说明 |
| --- | --- | --- |
| `pnpm tauri dev` / 打包运行 | `TauriDataSource`（`src/data/tauri.ts`） | 真实命令；元数据用 `productionMetadata`（与后端内嵌文件一致） |
| 浏览器 `pnpm dev` | `MockDataSource`（`src/data/mock.ts`） | 契约夹具 + 演示输入；界面常驻“模拟数据”横幅 |
| Tauri 内 `?mock=1` | MockDataSource | 演示/排查用；仍显示模拟横幅 |
| 任何模拟模式 `?mockWidth=800..1440` | — | 按“有效内容宽度”约束应用做宽度验收 |
| 模拟模式 `?mockLinkFail=1` | — | 演示“系统浏览器打开失败”的错误与恢复入口 |

模拟数据源覆盖：完整、非零组、表不完整（异常 type28）、资料文件、被占用后重试、
分析失败、刷新失败（stale）、取消、导出（新目录/已存在/重叠/非法名称）、链接解析
（无 URL 来源）与打开失败。

模拟模式下的地图使用随包底图（`data/map/maps/*.webp`），收集状态来自固定演示观察集合；
详情「在新窗口中打开」在浏览器里打开新标签页（`#popup=detail`），被拦截时自动退回应用内浮层。

## 结构（SX-015 / SX-016 与 2026-10-31 增强）

| 路径 | 说明 |
| --- | --- |
| `src/App.vue` | 应用外壳：顶栏 + 侧栏 + 主工作区 + 详情（宽屏常驻 / 抽屉 / 窄窗页面替换）、快捷键、数据源选择、详情窗口兜底浮层 |
| `src/app/store.ts` | 单一 reactive store：request_id 隔离、切槽撤销提交资格、刷新失败 stale、筛选回退、导出与链接动作状态、地图选择/筛选、详情弹窗兜底、偏好持久化 |
| `src/app/catalog.ts` | 界面派生视图：清单行（组行折叠）、区域计数单元格、诊断分组、表位置/参考集/组计数查询 |
| `src/app/map-view.ts` | 地图视图派生：底图目录、当前图条目/分组/筛选、无坐标与无底图统计、全量覆盖统计 |
| `src/app/category-icon.ts` | 图鉴类别 → Lucide 图标（未知回退 `MapPin`） |
| `src/app/detail-window.ts` | 详情弹窗 payload（版本 1）、从计划构建、打开/读取/订阅更新、窗口识别 |
| `src/data/source.ts` | 数据源抽象（discover/list/analyze/cancel/export/resolve/open 与 DTO） |
| `src/data/tauri.ts` | 真实命令映射（camelCase 参数；命令错误 `{code,message}`） |
| `src/data/mock.ts` | 模拟实现（含导出路径校验与 `OUTPUT_EXISTS` 恢复语义） |
| `src/data/basemaps.ts` | 底图数据集索引（按 unit/区域查找、图片地址、坐标投影） |
| `src/components/ExportDialog.vue` | 导出对话框：格式、分享脱敏、目标新目录、成功/失败/过期三态与恢复 |
| `src/components/CollectionDetail.vue` | 详情 store 连接层（区域/类别文案、直链与搜索地址、外链动作、弹出窗口） |
| `src/components/ItemDetailView.vue` | 详情纯展示层（主窗口与弹窗共用）：状态/徽标、如何获取、外部参考、相对位置 |
| `src/components/DetailWindow.vue` | 详情弹窗内容：读取 payload、同子关卡清单、外链打开/复制、缺失/版本/读取失败状态 |
| `src/components/MapPage.vue` | 地图查询页：区域/子关卡选择、独立筛选、覆盖与校准提示、地图 + 右栏 |
| `src/components/map/MapCanvas.vue` | 可交互地图：类别图标标记、已收集/未收集/选中、滚轮/按钮/双击缩放、拖拽平移 |
| `src/components/map/MapRail.vue` | 「详情 / 清单」标签页（各自保留滚动位置）、清单底部无坐标/无底图说明 |
| `src/components/map/MapLegend.vue` | 类别图标图例，兼类别筛选 |
| `src/components/ui/UiResizeHandle.vue` | 面板宽度手柄（指针拖动、←/→、Home/End、双击复位） |
| `src/components/ui/UiDialog.vue` | 居中模态（与抽屉相同的焦点规则：进入/限制/Esc/恢复） |
| `tests/` | Vitest：tauri-source（命令映射）、export、map-view/map-canvas/map-page、detail-window、end-to-end（假后端 IPC 全流程）、real-data（真实输出绑定，环境变量触发）等 |
| `src-tauri/` | 命令层（11 条命令、两个 capability、CSP 与 IPC/权限集成测试） |

## 布局档位与详情宽度

- 布局按“有效内容宽度”分档：≥1440 宽屏（详情常驻）、1100–1439 中等（右侧抽屉）、800–1099 紧凑（近全宽抽屉）、<800 窄窗（详情页面替换）。
- 常驻详情与两种抽屉都提供左边缘拖动手柄：拖动实时预览、松手写入偏好；←/→ 每次 16px（Shift 64px）、Home/End 到边界、双击复位。
- 宽度偏好：`detailPaneWidth` 400 / `detailDrawerWidth` 400 / `detailCompactWidth` 720 / `mapRailWidth` 420（范围 320–900；地图右栏 320–600）；设置页「恢复默认布局」一并复位。
- 窄窗详情为整页替换，不参与宽度偏好。

## 地图查询视图

- 入口：侧栏「地图查询」（需要已分析报告）。一张底图 = 一个子关卡，不做跨子关卡拼图。
- 页头固定说明覆盖范围（当前数据：42 张底图 · 24/65 个子关卡 · 291/470 条可上图），并显示校准状态（7 张 `calibrated` / 35 张 `approximate`）与过期快照警告。
- 标记：类别图标 + 已收集（实心）/未收集（描边）/选中（外圈置顶）；点击标记选中并自动切到右栏「详情」，点击清单项选中并聚焦地图、停留在「清单」。
- 缩放/平移：滚轮以指针为锚点 1×–4×、按钮 +/−/重置、双击放大；缩放后拖拽平移；标记不随缩放变大。键盘/读屏等价路径是「清单」标签页。
- 筛选：状态（默认全部）、类别（图例即筛选）、搜索；与区域清单互不影响，地图区域选择不回写区域清单。
- 响应式：≥1100 双栏（右栏可拖动调宽）；800–1099 右栏为右侧浮层；<800 地图 + 折叠清单，条目详情用地图页自己的全宽浮层。
- 底图来源：vi7ual（个人非商业使用，见 `THIRD_PARTY_NOTICES.md`）。

## 详情弹出窗口

- 详情头部「在新窗口中打开」：Tauri 内创建/复用固定标签 `detail` 的原生窗口（560×780，最小 380×420）；浏览器开发模式用 `window.open`，被拦截或创建失败时退回应用内模态浮层。
- 弹窗内容为打开时的只读快照：选中条目 + 同子关卡清单 + 外部参考；窗口内可切换条目，窗口标题固定、不含条目名。
- 数据传递：Tauri 内 payload 只存 Rust 进程内存（≤2 MiB、必须是 JSON 对象），弹窗经命令读取、用 `detail-payload-updated` 事件接收更新；浏览器模式用 `sessionStorage` + `postMessage`（仅模拟数据，关闭标签页即清除）。
- 生命周期：关闭详情窗口不取消主窗口在飞分析；关闭主窗口会一并关闭详情窗口并退出应用。
- 限制：payload 是打开时快照；主窗口刷新后弹窗不自动更新，需重新打开；弹窗不切换主题/语言/搜索引擎（按打开时状态生成）。

## 端到端测试

1. **命令层（Rust，真实夹具）**：`SOTTR_DESKTOP_E2E_INPUT`/`_OUT`（必需）、
   `_EXPORT_DIR`（新目录，触发分析→导出断言）、`_RESOLVE_SOURCE`（默认
   `src:stella-san-juan`）后运行
   `cargo test -p sottr-desktop --test ipc_commands real_fixture_end_to_end -- --nocapture`。
2. **界面全流程（jsdom）**：`tests/end-to-end.spec.ts` 用真实 `TauriDataSource` +
   脚本化假后端走“选择副本→分析→查漏→组线索→打开/复制链接→刷新（含失败）→导出（含重名恢复）”；
   断言命令参数、快照绑定、假点数为 0、原始 present 不显示为已收集。
3. **真实输出绑定**：把步骤 1 产出的 DesktopResult JSON 路径设入
   `SOTTR_DESKTOP_E2E_RESULT` 后运行 `pnpm test`，真实 Rust 输出会渲染进真实组件
   （未设置时跳过并在日志说明）。

## 当前边界

- 不加载任何外部页面、不发起网络请求；攻略链接只经后端校验后交给系统浏览器。
- 前端不做解析或领域计算：遗漏、门禁与定位精度全部读取报告结论（契约 1.0.0）。
- 地图点位只是同子关卡内的**相对位置参考**，不是导航路线；多数底图是近似校准（全局误差估计）。
- 地图覆盖不完整：291/470 条可上图；3 个特殊区域与 41 个无底图子关卡请在区域清单查看。
- 详情弹窗 payload 为打开时快照（不含完整路径、证据与诊断数据）；刷新后需重新打开。
- 真实 WebView2 内的原生选择器/外部浏览器/动态窗口/CSP 强制与 Windows DPI、对比度、
  辅助技术实测属于 SX-017；应用图标已更新（SX-018）。
