# SOTTR Explorer 桌面端（Tauri 2 + Vue 3 + TypeScript + Vite + Tailwind 4）

脚手架状态（SX-002）：可构建与启动占位界面，尚无分析接线。

```powershell
pnpm install        # 安装依赖并生成/更新 pnpm-lock.yaml
pnpm dev            # 仅前端开发服务器（http://localhost:1420）
pnpm build          # vue-tsc 类型检查 + vite 构建
pnpm tauri dev      # 桌面调试（需要 WebView2 与 MSVC 工具链）
```

说明：

- 应用图标为模板占位，发布前（SX-018）替换为正式图标
- CSP 暂未设置（模板默认 null），由 SX-014 按最小权限要求配置
- 前端当前不加载任何外部页面、不发起网络请求
- 界面与状态由 SX-015/SX-016 按冻结契约实现；本目录不做解析或领域计算
