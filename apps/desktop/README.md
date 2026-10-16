# SOTTR Explorer 桌面端（Tauri 2 + Vue 3 + TypeScript + Vite + Tailwind 4）

SX-014 提供最小权限命令层（后端）；SX-015 实现契约驱动的界面外壳与状态机；
**SX-016 完成真实接线与端到端**：前端在 Tauri 内使用 `TauriDataSource` 调用真实命令，
补齐导出对话框（JSON/CSV、分享脱敏、目标新目录、过期标记）、攻略链接解析/打开与失败恢复、
详情证据（表位置/参考集/组计数/来源链接）与端到端测试。纯浏览器开发（`pnpm dev`）没有 IPC，
自动退化为模拟数据源并显著标注“模拟数据”。

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

## 结构（SX-015 / SX-016）

| 路径 | 说明 |
| --- | --- |
| `src/App.vue` | 应用外壳：顶栏 + 侧栏 + 主工作区 + 详情（宽屏常驻 / 抽屉 / 窄窗页面替换）、快捷键、数据源选择 |
| `src/app/store.ts` | 单一 reactive store：request_id 隔离、切槽撤销提交资格、刷新失败 stale、筛选回退、导出与链接动作状态、偏好持久化 |
| `src/app/catalog.ts` | 界面派生视图：清单行（组行折叠）、区域计数单元格、诊断分组、表位置/参考集/组计数查询 |
| `src/data/source.ts` | 数据源抽象（discover/list/analyze/cancel/export/resolve/open 与 DTO） |
| `src/data/tauri.ts` | 真实命令映射（camelCase 参数；命令错误 `{code,message}`） |
| `src/data/mock.ts` | 模拟实现（含导出路径校验与 `OUTPUT_EXISTS` 恢复语义） |
| `src/components/ExportDialog.vue` | 导出对话框：格式、分享脱敏、目标新目录、成功/失败/过期三态与恢复 |
| `src/components/ItemDetail.vue` | 详情：状态/依据/定位、去哪里找、攻略动作（打开/复制已核验链接/复制线索）、表位置/参考集/组计数/来源链接、折叠证据 |
| `src/components/ui/UiDialog.vue` | 居中模态（与抽屉相同的焦点规则：进入/限制/Esc/恢复） |
| `tests/` | Vitest：tauri-source（命令映射）、export、end-to-end（假后端 IPC 全流程）、real-data（真实输出绑定，环境变量触发）等 |
| `src-tauri/` | SX-014 命令层（8 条命令、权限清单、CSP 与 IPC 集成测试） |

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
- 无已验证坐标不绘制任何点位；已验证坐标仍为 0 项。
- 真实 WebView2 内的原生选择器/外部浏览器/CSP 强制与 Windows DPI、对比度、
  辅助技术实测属于 SX-017；应用图标仍为模板占位（SX-018）。
