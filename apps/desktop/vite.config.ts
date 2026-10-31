import { defineConfig } from "vitest/config";
import { loadEnv, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { createReadStream, existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// 应用根与仓库根：契约夹具在仓库 `fixtures/contract/`（模拟数据源直接引用，
// 保持夹具唯一来源）；开发服务器需要显式允许该目录。
const appRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("../../", import.meta.url));

/** 随包分发的底图目录（D4 = A）；`image_root.kind = bundled` 时由它提供 `/basemaps/*`。 */
const bundledBasemapDir = path.join(repoRoot, "data/map/maps");

/**
 * 把底图挂到 `/basemaps/maps/*`：开发期直接读目录，构建期作为 asset 输出。
 *
 * `BASEMAP_IMAGE_ROOT` 可在本地覆盖来源目录（例如尚未入库的外置素材），
 * 两者都没有时返回 null，前端拿到 null 并回退散点。
 */
function basemapImages(dir: string | null, emit: boolean): Plugin | null {
  if (!dir) {
    return null;
  }
  const base = path.resolve(dir);
  const prefix = "/basemaps/maps/";
  return {
    name: "sottr-basemap-images",
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
    generateBundle() {
      if (!emit || !existsSync(base)) {
        return;
      }
      for (const name of readdirSync(base)) {
        if (!name.endsWith(".webp")) {
          continue;
        }
        this.emitFile({
          type: "asset",
          fileName: `basemaps/maps/${name}`,
          source: readFileSync(path.join(base, name)),
        });
      }
    },
  };
}

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const fileEnv = loadEnv(mode, appRoot, "");
  const override = process.env.BASEMAP_IMAGE_ROOT ?? fileEnv.BASEMAP_IMAGE_ROOT;
  // 测试环境固定走「未配置素材」分支：避免本机目录让用例依赖外部文件。
  const imageDir = mode === "test" ? null : (override ?? (existsSync(bundledBasemapDir) ? bundledBasemapDir : null));
  const plugin = basemapImages(imageDir, !override);

  return {
    plugins: [vue(), tailwindcss(), ...(plugin ? [plugin] : [])],

    define: {
      __BASEMAP_IMAGE_BASE__: JSON.stringify(imageDir ? "/basemaps" : null),
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

