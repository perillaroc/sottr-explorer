# SOTTR Explorer 桌面端（Tauri 2 + Vue 3 + TypeScript + Vite + Tailwind 4）

Tauri 最小权限命令层 + Vue 界面：存档分析、全收集、区域清单、收集计划、地图查询、诊断、
设置/关于，以及导出与分享。纯浏览器开发（`pnpm dev`）没有 IPC，自动退化为模拟数据源并
标注“模拟数据”。

## 功能概览

- **详情**：可拖动调宽的详情面板（宽度持久化、双击/键盘复位）；可在独立原生窗口打开，
  创建失败时退回应用内浮层。
- **地图查询**：底图 + 类别图标标记，地图与「详情 / 清单」右栏联动，1×–4× 缩放与平移。
- **必收指引**：无坐标条目用合集注记、无底图子关卡用子关卡注记；详情显示合集与指引标签，
  全收集页显示 352/352 覆盖。
- **收集计划**：只列未收集条目，按区域 → 子关卡 → 类别聚合，支持筛选与排序；导出可打印的
  `plan.html` 与 `plan.csv`（新目录、分享脱敏可选）。
- **进度增量**：对比同一存档在本次会话内的两次分析；历史只存内存，不写磁盘；不可比时明确说明。
- **存档监视**：只读轮询当前存档指纹（约 500ms），更新 / 替换 / 占用 / 删除时提示刷新；
  不自动分析、不写盘、不锁定。
- **分享报告**：自包含 `share.html`（总进度、区域进度、必收缺失），不含路径、用户名、哈希、
  快照时间或地图缩略图。

```powershell
pnpm install        # 安装依赖并生成/更新 pnpm-lock.yaml
pnpm dev            # 前端开发服务器（http://127.0.0.1:51888；浏览器内为模拟数据演示）
pnpm build          # vue-tsc 类型检查（含 tests/）+ vite 构建
pnpm test           # 组件/状态/端到端测试（Vitest + jsdom）
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
分析失败、刷新失败（stale）、取消、导出（新目录/已存在/重叠/非法名称）、
计划清单导出（大小上限/分享模式路径扫描）与打开导出目录、链接解析
（无 URL 来源）与打开失败。

模拟模式下的地图使用随包底图（`data/map/maps/*.webp`），收集状态来自固定演示观察集合；
详情「在新窗口中打开」在浏览器里打开新标签页（`#popup=detail`），被拦截时自动退回应用内浮层。

## 结构

| 路径 | 说明 |
| --- | --- |
| `src/styles.css` | 视觉令牌（明暗主题、类别色、表面、字阶、控件与动效）与共享表面 / 状态 / 进度 / 折叠样式 |
| `src/App.vue` | 应用外壳：顶栏 + 侧栏 + 主工作区 + 详情（宽屏常驻 / 抽屉 / 窄窗页面替换）、快捷键、数据源选择、详情窗口兜底浮层 |
| `src/app/layout.ts` | 页面水平内边距与区域索引 / 摘要栅格口径（按 `store.layout`，不写视口 media query） |
| `src/app/locales/` | 文案资源：`zh-CN.ts`（权威键表）、`en.ts`、`index.ts` 注册表（语言列表与设置页选项） |
| `src/app/i18n.ts` | 语言偏好、`t` / `tFormat` / `tFormatPair` 键查找、英文缺失回退与记录、`localizedName` |
| `src/app/store.ts` | 单一 reactive store：request_id 隔离、切槽撤销提交资格、刷新失败 stale、筛选回退、导出与链接动作状态、地图选择/筛选、详情弹窗兜底、偏好持久化 |
| `src/app/catalog.ts` | 界面派生视图：清单行（组行折叠）、区域计数单元格、诊断分组、表位置/参考集/组计数查询 |
| `src/app/map-view.ts` | 地图视图派生：底图目录、当前图条目/分组/筛选、无坐标与无底图统计、全量覆盖统计 |
| `src/app/guidance.ts` | 必收指引派生：352/352 覆盖统计、单条下落类别、合集显示名 |
| `src/app/category-icon.ts` | 图鉴类别 → Lucide 图标（未知回退 `MapPin`） |
| `src/app/detail-window.ts` | 详情弹窗 payload（版本 1）、从计划构建、打开/读取/订阅更新、窗口识别 |
| `src/data/source.ts` | 数据源抽象（discover/list/analyze/cancel/export/resolve/open 与 DTO） |
| `src/data/tauri.ts` | 真实命令映射（camelCase 参数；命令错误 `{code,message}`） |
| `src/data/mock.ts` | 模拟实现（含导出路径校验与 `OUTPUT_EXISTS` 恢复语义） |
| `src/data/basemaps.ts` | 底图数据集索引（按 unit/区域查找、图片地址、坐标投影） |
| `src/components/AppPageHeader.vue` | 页面骨架：主标题、上下文、说明、页面级动作与工具条插槽 |
| `src/components/AppBrandMark.vue` | 罗盘品牌标记（Lucide `Compass`，不新增图片资产） |
| `src/components/RegionProgressRow.vue` | 概览与全部区域共用的区域进度入口 |
| `src/components/ExportDialog.vue` | 导出对话框：格式、分享脱敏、目标新目录、成功/失败/过期三态与恢复 |
| `src/components/CollectionDetail.vue` | 详情 store 连接层（区域/类别文案、直链与搜索地址、外链动作、弹出窗口） |
| `src/components/ItemDetailView.vue` | 详情纯展示层（主窗口与弹窗共用）：状态/徽标、合集、如何获取/指引、外部参考、相对位置 |
| `src/components/DetailWindow.vue` | 详情弹窗内容：读取 payload、同子关卡清单、外链打开/复制、缺失/版本/读取失败状态 |
| `src/components/MapPage.vue` | 地图查询页：区域/子关卡选择、独立筛选、覆盖与校准提示、地图 + 右栏 |
| `src/components/map/MapCanvas.vue` | 可交互地图：类别图标标记、已收集/未收集/选中、滚轮/按钮/双击缩放、拖拽平移 |
| `src/components/map/MapRail.vue` | 「详情 / 清单」标签页（各自保留滚动位置）、清单底部无坐标/无底图说明 |
| `src/components/map/MapLegend.vue` | 类别图标图例，兼类别筛选 |
| `src/components/ui/UiResizeHandle.vue` | 面板宽度手柄（指针拖动、←/→、Home/End、双击复位） |
| `src/components/ui/UiStatusMark.vue` | 收集状态图形（已收集实心勾 / 未收集空心 / 未知虚线），配文字或 `aria-label`，不单靠颜色 |
| `src/components/ui/UiProgressBar.vue` | 可访问进度条（4px / 6px、accent / 旧金、`role="progressbar"` 与 `aria-valuetext`） |
| `src/components/ui/UiDialog.vue` | 居中模态（与抽屉相同的焦点规则：进入/限制/Esc/恢复） |
| `tests/` | Vitest：tauri-source（命令映射）、export、map-view/map-canvas/map-page、detail-window、end-to-end（假后端 IPC 全流程）、real-data（真实输出绑定，环境变量触发）等 |
| `src-tauri/` | 命令层（11 条命令、两个 capability、CSP 与 IPC/权限集成测试） |

## 视觉结构

- **令牌**：`src/styles.css` 定义 `data-theme="light|dark"` 的表面、文字、主操作、旧金、状态与 12 个类别色，并映射为 Tailwind 主题；类别色按新表面微调明度，对比度由 `tests/theme-contrast.spec.ts` 校验。
- **表面**：页面 `canvas` 无外框；工作面 `surface`；筛选槽 / hover / 折叠区 `raised`；只有菜单、抽屉、对话框使用阴影。
- **字阶**：页面标题 22/30、详情标题 20/28、区块标题 15/22、正文 14/22、辅助 13/20、元信息 12/18；ID 与 hash 使用等宽字体并允许断行。
- **选中语言**：导航与清单行使用「3px 左侧标记 + `accent-bg` + 文字加重」；地图点位使用 2px 外环 + 4px 偏移；收集状态另用 `UiStatusMark` 的形状 + 文字，不只靠颜色。
- **页面结构**：清单 / 地图工具头固定，其余页面随内容滚动；`AppPageHeader` 统一标题、上下文、说明与页面级动作。内容宽度：设置 840px，诊断 / 关于 960px，表格型概览与清单最多 1120px，地图不设固定最大宽度。
- **地图**：画布使用 `--map-stage`，底图原色显示、不加滤镜；工具条在右下角（32px 按钮组 + 缩放读数）；右栏「详情 / 清单」44px 标签栏，选中为底部 2px accent。
- **设置 / 关于**：设置按「显示 / 布局 / 外部链接」分区，标签 128px、窄窗独占一行，原生 radio 视觉为 36/28px 分段选择；关于页使用 32px 罗盘、键值面板与 12px 等宽许可长文（仍按需动态加载）。

## 布局档位与详情宽度

- 布局按“有效内容宽度”分档：≥1440 宽屏（详情常驻）、1100–1439 中等（右侧抽屉）、800–1099 紧凑（近全宽抽屉）、<800 窄窗（详情页面替换）。
- 常驻详情与两种抽屉都提供左边缘拖动手柄：拖动实时预览、松手写入偏好；←/→ 每次 16px（Shift 64px）、Home/End 到边界、双击复位。
- 宽度偏好：`detailPaneWidth` 400 / `detailDrawerWidth` 400 / `detailCompactWidth` 720 / `mapRailWidth` 420（范围 320–900；地图右栏 320–600）；设置页「恢复默认布局」一并复位。
- 窄窗详情为整页替换，不参与宽度偏好。

## 地图查询视图

- 入口：侧栏「地图查询」（需要已分析报告）。一张底图 = 一个子关卡，不做跨子关卡拼图。
- 页头固定说明覆盖范围（当前数据：42 张底图 · 25/65 个子关卡 · 294/470 条可上图），并显示校准状态（18 张 `calibrated` / 24 张 `approximate`）与过期快照警告。
- 标记：类别图标 + 已收集（实心）/未收集（描边）/选中（外圈置顶）；点击标记选中并自动切到右栏「详情」，点击清单项选中并聚焦地图、停留在「清单」。
- 缩放/平移：滚轮以指针为锚点 1×–4×、右下角工具条 +/−/重置与缩放读数、双击放大；缩放后拖拽平移；标记不随缩放变大。键盘/读屏等价路径是「清单」标签页。
- 筛选：状态（默认全部）、类别（图例即筛选）、搜索；与区域清单互不影响，地图区域选择不回写区域清单。
- 响应式：≥1100 双栏（右栏可拖动调宽）；800–1099 右栏为右侧浮层；<800 地图 + 折叠清单，条目详情用地图页自己的全宽浮层。
- 底图来源：vi7ual（个人非商业使用，见 `THIRD_PARTY_NOTICES.md`）。

## 详情弹出窗口

- 详情头部「在新窗口中打开」：Tauri 内创建/复用固定标签 `detail` 的原生窗口（560×780，最小 380×420）；浏览器开发模式用 `window.open`，被拦截或创建失败时退回应用内模态浮层。
- 弹窗内容为打开时的只读快照：选中条目 + 同子关卡清单 + 外部参考；窗口内可切换条目，窗口标题固定、不含条目名。
- 数据传递：Tauri 内 payload 只存 Rust 进程内存（≤2 MiB、必须是 JSON 对象），弹窗经命令读取、用 `detail-payload-updated` 事件接收更新；浏览器模式用 `sessionStorage` + `postMessage`（仅模拟数据，关闭标签页即清除）。
- 生命周期：关闭详情窗口不取消主窗口在飞分析；关闭主窗口会一并关闭详情窗口并退出应用。
- 限制：payload 是打开时快照；主窗口刷新后弹窗不自动更新，需重新打开；弹窗不切换主题/语言/搜索引擎（按打开时状态生成）。

## 辅助技术

- **跳到正文**：首个 Tab 焦点是 `#main-content` 链接，`main` 带 `tabindex="-1"`。
- **名称与标签**：可交互控件都有可访问名称；单选框/复选项由 `<label>` 包裹；图标 `aria-hidden`。
- **焦点**：抽屉/对话框 `role="dialog"` + 名称，焦点进入、Tab 循环、Esc 关闭并回到触发元素。
- **对比度与高对比度**：焦点环为 2px `outline`；`@media (forced-colors: active)` 下当前导航项、
  选中清单行与选中地图标签改用系统 `Highlight` 轮廓。
- **地图**：标记与缩放按钮有名称；键盘/读屏等价路径是右栏「清单」标签页。
- **回归**：`tests/accessibility.spec.ts`、`layout-focus.spec.ts`、`theme-contrast.spec.ts`；
  Narrator 与系统高对比度的人工步骤见 `docs/development/accessibility.md`。

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
- 地图覆盖不完整：294/470 条可上图；其余子关卡与特殊区域请在区域清单查看。
- 详情弹窗 payload 为打开时快照（不含完整路径、证据与诊断数据）；刷新后需重新打开。
- 真实 WebView2 的动态详情窗口、关闭联动与偏好重启已验收；原生文件选择器、外部浏览器打开、
  DPI、Narrator 与高对比度未做正式人工验收，自动化覆盖见 `docs/development/accessibility.md`。
