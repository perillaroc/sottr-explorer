/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

/**
 * 基底层图片的浏览器可访问前缀。
 * 由 vite.config.ts 在设置了 BASEMAP_IMAGE_ROOT 时注入 `/basemaps`，否则为 null
 * （未配置外置素材时界面回退到散点）。
 */
declare const __BASEMAP_IMAGE_BASE__: string | null;
