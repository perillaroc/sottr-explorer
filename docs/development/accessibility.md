# 辅助技术与无障碍

> 键盘、读屏与高对比度用户的支持范围、检查清单与验证方式。

## 支持声明

- **键盘**：所有操作都可用键盘完成；地图的平移/缩放另有指针操作，但右栏「清单」标签页
  提供等价的键盘与读屏路径。
- **焦点**：打开抽屉/对话框时焦点进入；Tab 循环；Esc 关闭并回到触发元素；窄窗详情为页面
  替换，进入标题、返回恢复列表滚动。
- **名称与标签**：可交互控件都有可访问名称；单选框/复选项由 `<label>` 包裹；图标
  `aria-hidden`；状态与地图标记带 `aria-label`。
- **对比度**：设计令牌在明暗主题下对正文/辅助/强调/状态/类别色均 ≥4.5:1
  （`tests/theme-contrast.spec.ts`）。
- **高对比度**：`@media (forced-colors: active)` 下选中态改用系统 `Highlight` 轮廓；
  焦点环使用 `outline`，不依赖阴影。
- **动效**：`prefers-reduced-motion: reduce` 时关闭过渡与动画。

## 六条主流程检查清单

| 流程 | 检查点 |
| --- | --- |
| 导航 | 首个 Tab 出现「跳到正文」，Enter 后焦点落到 `main`；侧栏按钮有名称，当前项 `aria-current="page"` |
| 区域清单 | 列表项可用 Tab/方向键到达；行状态有文字或 `aria-label`，不只靠颜色；筛选器有标签 |
| 详情 | 打开后焦点进入；常驻面板不是模态；抽屉/对话框 Tab 循环、Esc 关闭并恢复焦点 |
| 对话框 | 导出/计划导出/详情弹窗 `role="dialog"` + 名称；关闭按钮、取消/确认有名称 |
| 地图清单 | 右栏「详情 / 清单」标签可切换；清单项可读、可选中；标记 `aria-label` 含名称与收集状态；缩放按钮有名称 |
| 导出 | 格式、脱敏、目录选择都可用键盘；成功/失败/过期状态由 `role="status"` 播报；只写到新目录 |

## 自动检查

```bash
cd apps/desktop
pnpm test          # accessibility / layout-focus / theme-contrast 等
pnpm build         # vue-tsc + 产物构建
```

- `tests/accessibility.spec.ts`：跨页结构不变量（跳到正文、单一 main、每页一个 h1、
  控件名称、表单标签、对话框命名、地图标记名称、状态 live region、forced-colors 样式）。
- `tests/layout-focus.spec.ts`：焦点陷阱、Esc 恢复、窄窗页面替换、快捷键。
- `tests/theme-contrast.spec.ts`：明暗主题令牌对比度与 Tailwind 映射。

未引入 axe-core：jsdom 无法计算颜色对比与布局规则，且会新增运行时依赖；结构项由
`accessibility.spec.ts` 覆盖，颜色由令牌测试与浏览器抽查覆盖。

浏览器端可用开发模式做补充抽查：`?mock=1` 下逐页检查可访问名称、文本对比度与
`prefers-reduced-motion`；高对比度需要系统设置，见下。

## 人工检查

1. **Narrator（Windows 11）**：启动应用 → 第一个 Tab 是否读出“跳到正文” → 依次走导航、
   区域清单、详情、对话框、地图清单、导出；确认状态提示会被播报且不打断当前朗读。
2. **高对比度**：设置 → 辅助功能 → 对比度主题（如“夜空/沙漠”）；确认：
   - 焦点环可见；
   - 侧栏当前项、清单选中行、地图右栏选中标签有 `Highlight` 轮廓；
   - 状态标记与地图标记仍有形状/文字区分；
   - 4× 缩放时底图与标记仍可辨。
3. 记录检查结果；发现的问题标注“已修 / 不支持 / 待办”，不支持项同步到[已知限制](limitations.md)。

## 当前限制

- 地图平移只有指针操作；键盘与读屏请使用「清单」标签页。
- 高对比度下底图本身按系统强制色显示（不做 `forced-color-adjust: none`），
  底图细节可能少于常规主题。
- 真实 Narrator 与系统 DPI 需在 Windows 会话中按上面的清单检查。

## 关联文档

- [界面导览](/guide/interface)
- [测试](testing.md)
- [已知限制](limitations.md)
- [决策记录](decisions/index.md)
