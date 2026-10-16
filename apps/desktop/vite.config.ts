import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// 应用根与仓库根：契约夹具在仓库 `fixtures/contract/`（模拟数据源直接引用，
// 保持夹具唯一来源）；开发服务器需要显式允许该目录。
const appRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("../../", import.meta.url));

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [vue(), tailwindcss()],

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
}));
