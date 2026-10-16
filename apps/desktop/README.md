# SOTTR Explorer 桌面端（Tauri 2 + Vue 3 + TypeScript + Vite + Tailwind 4）

SX-014 提供最小权限命令层（后端）；**SX-015 实现契约驱动的界面外壳与模拟数据演示**
（槽位选择、概览、区域与收集、详情、诊断、设置；加载/空/错误/降级/过期/取消状态）。
真实分析接线（Tauri 命令 ↔ 界面）在 SX-016：当前所有页面显著标注“模拟数据”。

```powershell
pnpm install        # 安装依赖并生成/更新 pnpm-lock.yaml
pnpm dev            # 仅前端开发服务器（http://localhost:1420）
pnpm build          # vue-tsc 类型检查（含 tests/）+ vite 构建
pnpm test           # 组件/状态测试（Vitest + jsdom；SX-015 验收）
pnpm contract:check # 契约夹具校验 + 由 schemas/ 生成 src/contract 类型 + 类型检查
pnpm tauri dev      # 桌面调试（需要 WebView2 与 MSVC 工具链）
```

## 结构（SX-015）

| 路径 | 说明 |
| --- | --- |
| `src/App.vue` | 应用外壳：顶栏 + 侧栏 + 主工作区 + 详情（宽屏常驻 / 抽屉 / 窄窗页面替换）、快捷键 |
| `src/app/store.ts` | 单一 reactive store：request_id 隔离、切槽撤销提交资格、刷新失败 stale、筛选回退、偏好持久化 |
| `src/app/catalog.ts` | 界面派生视图：清单行（组行折叠）、区域计数单元格（未知/暂不支持/计数待核对）、诊断分组、未分类记录 |
| `src/app/labels.ts` | 中文文案与三组概念（业务状态 / 证据依据 / 定位精度）标签；组门禁措辞 |
| `src/data/` | 数据源抽象（`source.ts`）与模拟实现（`mock*.ts`：契约夹具 + 演示输入 + 真实元数据合并） |
| `src/components/` | 外观组件（ui/）与业务组件（SlotPicker、OverviewPage、RegionPage、CollectibleList、ItemDetail、DiagnosticsPage、SettingsPage 等） |
| `src/styles.css` | 视觉令牌（明暗主题由 `<html data-theme>` 驱动）与基础样式 |
| `tests/` | Vitest 组件/状态测试（55 项；含键盘、焦点、宽度档位、状态隔离） |
| `src-tauri/` | SX-014 命令层（权限清单、CSP、Tauri 命令与 IPC 集成测试） |

## 模拟数据与演示开关

- 模拟数据源 `MockDataSource`：契约夹具（`fixtures/contract/valid/*`）与真实元数据
  （`data/metadata/*`）组合；场景覆盖完整、非零组、表不完整、资料文件、被占用后重试、
  分析失败、刷新失败（stale）、取消。界面顶部常驻“模拟数据”横幅。
- `?mockWidth=800`（仅模拟模式）：把应用内容宽度限制为指定值并在横幅标注，
  用于有效宽度（800/1100/1280/1440 与更窄）的人工/自动验收；布局档位按
  **内容宽度** 计算：≥1440 常驻详情、1100–1439 抽屉、800–1099 近全宽抽屉、<800 单列。
- 主题偏好、导航折叠、视图与筛选偏好保存在 `localStorage`（键 `sottr-explorer.ui.v1`）；
  不保存任何存档内容或报告数据。

## 当前边界

- 不加载任何外部页面、不发起网络请求；攻略链接由 SX-016 的后端校验后交系统浏览器。
- 前端不做解析或领域计算：遗漏、门禁与定位精度全部读取报告结论（契约 1.0.0）。
- 空白地图/装饰性定位针不渲染；已验证坐标 0 项时只显示文字线索。
- 应用图标为模板占位，发布前（SX-018）替换；真实 Windows DPI/对比度/辅助技术验收在 SX-017。
