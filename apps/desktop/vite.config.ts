import { defineConfig } from "vitest/config";
import { loadEnv, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { createReadStream, existsSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// 应用根与仓库根：契约夹具在仓库 `fixtures/contract/`（模拟数据源直接引用，
// 保持夹具唯一来源）；开发服务器需要显式允许该目录。
const appRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("../../", import.meta.url));

/**
 * 开发期把 `/basemaps/<path>` 映射到外置图片目录（不复制、不改 fs.allow）。
 *
 * 真实地图底图是外置素材（D4 未决）：只有显式设置 BASEMAP_IMAGE_ROOT 时才提供，
 * 且只用于开发服务器；未设置时前端拿到 null 并回退到散点。
 * 取值优先级：进程环境变量 → `.env.local` 等 Vite 环境文件。
 */
function basemapImageServer(root: string | undefined): Plugin | null {
  if (!root) {
    return null;
  }
  const base = path.resolve(root);
  const prefix = "/basemaps/";
  return {
    name: "sottr-basemap-images",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const url = req.url ?? "";
        if (!url.startsWith(prefix)) {
          return next();
        }
        const relative = decodeURIComponent(url.slice(prefix.length).split("?")[0]);
        const file = path.resolve(base, relative);
        if (!file.startsWith(base + path.sep) || !existsSync(file) || !statSync(file).isFile()) {
          res.statusCode = 404;
          res.end("not found");
          return;
        }
        res.setHeader("Content-Type", "image/webp");
        createReadStream(file).pipe(res);
      });
    },
  };
}

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const fileEnv = loadEnv(mode, appRoot, "");
  // 测试环境固定走「未配置素材」分支：避免本机 .env.local 让用例依赖外部文件。
  const basemapRoot =
    mode === "test" ? undefined : (process.env.BASEMAP_IMAGE_ROOT ?? fileEnv.BASEMAP_IMAGE_ROOT);
  const basemapPlugin = basemapImageServer(basemapRoot);

  return {
    plugins: [vue(), tailwindcss(), ...(basemapPlugin ? [basemapPlugin] : [])],

    define: {
      __BASEMAP_IMAGE_BASE__: JSON.stringify(basemapRoot ? "/basemaps" : null),
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      fs: {
        allow: [appRoot, repoRoot],
      },
      hmr: host
        ? {
            protocol: "ws",
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
    },

    // 组件测试（SX-015）：jsdom 环境；夹具 JSON 与契约类型直接消费。
    test: {
      environment: "jsdom",
      include: ["tests/**/*.spec.ts"],
      setupFiles: ["tests/setup.ts"],
      restoreMocks: true,
    },
  };
});
