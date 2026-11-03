# 构建

## 需要

- Rust（stable，工具链见 `rust-toolchain.toml`）；
- Node.js 22 与 pnpm 10；
- Windows：存档发现与稳定快照读取依赖 Windows API；
- 桌面端还需要 WebView2 与 MSVC 工具链。

## Rust

```powershell
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

## 桌面端

```powershell
cd apps/desktop
pnpm install
pnpm build          # vue-tsc 类型检查 + vite 构建
pnpm test           # 组件 / 状态 / 端到端测试
pnpm tauri build    # 单文件便携 exe（不生成安装包）：target/release/sottr-desktop.exe
pnpm dev            # 开发服务器 http://127.0.0.1:51888（浏览器内为模拟数据）
pnpm tauri dev      # 桌面调试（真实命令；需要 WebView2）
```

浏览器模式没有 IPC，会自动退化为模拟数据源并在界面上常驻“模拟数据”横幅。

## 契约

```powershell
cd apps/desktop
pnpm contract:check   # schema 校验 + 生成 src/contract 类型 + vue-tsc
pnpm contract:images  # 底图存在性、字节数、SHA-256、WebP 尺寸
```

`contract:images` 单独成命令、不并入 `contract:check`：图片体积大且可外置。

## 文档站

```powershell
cd docs
pnpm install
pnpm docs:dev       # 本地预览
pnpm docs:check     # 错误码与术语枚举一致性
pnpm docs:build     # 先跑一致性校验，再输出 docs/.vitepress/dist
pnpm docs:preview   # 预览构建产物
```

Netlify 使用同一套命令：仓库根 `netlify.toml` 把 base 设为 `docs`，发布目录为
`.vitepress/dist`。构建默认会把死链和一致性错误当失败，因此文档链接、错误码或枚举值
改动后必须重新构建。

## 常见问题

- `pnpm tauri dev` 启动失败：先确认 WebView2 运行时与 MSVC 工具链；
- 契约类型报错：先跑 `pnpm contract:types`，不要手改 `src/contract/*.d.ts`；
- 底图 404：确认 `data/map/maps/*.webp` 存在，或设置 `BASEMAP_IMAGE_ROOT` 指向外置目录；
- 数据哈希校验失败：确认文本文件使用 LF（仓库 `.gitattributes` 已强制 `eol=lf`）；
  不要把 `data/**/*.json` 保存成 CRLF，否则内容 SHA-256 会变化；
- 差分回归全部跳过：检查 `SOTTR_SAMPLE_DIR` / `SOTTR_PUBLIC_DIR` 是否设置。
