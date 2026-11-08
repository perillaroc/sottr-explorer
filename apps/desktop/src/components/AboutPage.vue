<script setup lang="ts">
// 关于（2026-10-31 从「设置与关于」拆出；2026-11-05 界面美化阶段五）：
// 应用与数据版本、支持范围、许可与快捷键；设置页只放可调项。
// 页头使用 32px 罗盘品牌标记；不添加大幅品牌 hero。
import { computed, ref } from "vue";
import { ChevronDown } from "@lucide/vue";
import { getVersion } from "@tauri-apps/api/app";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { pagePaddingClass } from "../app/layout";
import { supportedCategories } from "../app/catalog";
import { basemapDatasetVersion } from "../data/basemaps";
import { isTauriRuntime } from "../data/tauri";
import AppBrandMark from "./AppBrandMark.vue";
// SX-018 / 2026-10-31：许可通知与依赖许可全文随前端内嵌（构建期内嵌，离线可读）。
// 全文约 800 KB，展开时再动态载入，避免主包与首屏变重。
import noticesText from "../../../../THIRD_PARTY_NOTICES.txt?raw";

const store = useStore();

const layout = computed(() => store.layout.value);
const pageX = computed(() => pagePaddingClass(layout.value));

// 应用版本的唯一来源是 Cargo workspace 版本：构建期由 Vite 注入 __APP_VERSION__；
// 打包运行时再用 Tauri 的 getVersion() 覆盖，浏览器开发/测试保留构建期值。
const appVersion = ref(__APP_VERSION__);
if (isTauriRuntime()) {
  void getVersion()
    .then((value) => {
      appVersion.value = value;
    })
    .catch(() => {
      // 保持构建期注入值
    });
}

const licensesText = ref<string | null>(null);
const licensesLoading = ref(false);
const licensesError = ref<string | null>(null);

async function loadLicenses(): Promise<void> {
  if (licensesText.value || licensesLoading.value) {
    return;
  }
  licensesLoading.value = true;
  licensesError.value = null;
  try {
    const module = await import("../../../../THIRD_PARTY_LICENSES.txt?raw");
    licensesText.value = module.default;
  } catch (error) {
    licensesError.value = error instanceof Error ? error.message : String(error);
  } finally {
    licensesLoading.value = false;
  }
}

function onLicensesToggle(event: Event): void {
  if ((event.currentTarget as HTMLDetailsElement).open) {
    void loadLicenses();
  }
}

const report = computed(() => store.report.value);
const categories = computed(() => (report.value ? supportedCategories(store.metadata, report.value) : []));

const versionRows = computed(() => [
  { label: t("应用版本", "App version"), value: appVersion.value },
  { label: t("元数据版本", "Metadata version"), value: `${store.metadata.metadata_version}（${store.metadata.namespace}）` },
  { label: t("图鉴版本", "Catalog version"), value: store.atlas.catalog_version },
  { label: t("底图数据集", "Basemap dataset"), value: basemapDatasetVersion },
]);

const shortcuts = computed(() => [
  { keys: "Ctrl+O", action: t("打开文件（选择存档）", "Open a file (choose a save)") },
  { keys: "Ctrl+F", action: t("聚焦当前清单搜索", "Focus the current list search") },
  { keys: "F5", action: t("刷新当前槽位", "Refresh the current slot") },
  { keys: "Esc", action: t("关闭菜单或详情抽屉", "Close a menu or the details drawer") },
]);
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="about-title">
    <div class="mx-auto w-full max-w-[960px] pb-8" :class="pageX">
      <header class="ui-page-header">
        <div class="flex items-start gap-3">
          <AppBrandMark :size="32" class="mt-0.5 shrink-0" />
          <div class="min-w-0">
            <p class="ui-meta mb-1">{{ t("关于", "About") }}</p>
            <h1 id="about-title" class="ui-page-title">SOTTR Explorer</h1>
            <p class="mt-1 text-[13px] leading-5 text-muted">
              {{ t(`版本 ${appVersion} · 只读存档分析`, `Version ${appVersion} · read-only save analysis`) }}
            </p>
          </div>
        </div>
      </header>

      <div class="mt-6 space-y-6">
        <section data-testid="about-data">
          <h2 class="ui-section-title">{{ t("数据与范围", "Data & scope") }}</h2>
          <dl class="ui-panel mt-2 divide-y divide-line/60 text-[13px] leading-5">
            <div v-for="row in versionRows" :key="row.label" class="flex flex-wrap items-baseline gap-x-3 gap-y-1 px-4 py-2.5">
              <dt class="w-28 shrink-0 text-muted">{{ row.label }}</dt>
              <dd class="mono min-w-0 flex-1 break-all">{{ row.value }}</dd>
            </div>
          </dl>
          <p class="mt-2 text-[13px] leading-5">
            {{ t("支持范围：普通/稀有贮藏点、石碑、地下墓穴宝藏的参考遗漏推定；已验证坐标 0 项；区域百分比暂不支持。", "Supported scope: reference-gap inference for survival/rare caches, monoliths and crypt treasures; 0 verified coordinates; region percentages are not supported yet.") }}
          </p>
          <p v-if="categories.length" class="mt-1 text-[13px] leading-5">
            {{ t("当前报告支持类别", "Categories supported by the current report") }}：
            {{ categories.map((category) => category.label).join(t("、", ", ")) }}
          </p>
          <p class="mt-1 text-[13px] leading-5 text-muted">
            {{ t("本地只读：无账号、无遥测、无后台上传；导出默认分享脱敏。", "Local and read-only: no accounts, telemetry or background uploads; exports are share-redacted by default.") }}
          </p>
        </section>

        <section data-testid="about-notices">
          <h2 class="ui-section-title">{{ t("许可", "Licenses") }}</h2>
          <ul class="ui-panel mt-2 divide-y divide-line/60 text-[13px] leading-5">
            <li class="px-4 py-2.5">
              {{ t("代码采用 MIT（根目录 LICENSE）；地图底图与游戏数据不受 MIT 覆盖；第三方来源、依赖清单与许可全文可在下方离线展开。", "The code is MIT (see LICENSE); map basemaps and game data are not covered by MIT; third-party sources, dependency inventory and full license texts can be opened offline below.") }}
            </li>
            <li class="px-4 py-2.5">
              {{ t("交付形态：单文件免安装 exe（Windows），依赖系统 WebView2 Runtime；本构建未做代码签名，SmartScreen 可能提示未知发布者。", "Delivery: a single portable Windows exe that requires the system WebView2 Runtime; this build is not code-signed, so SmartScreen may warn about an unknown publisher.") }}
            </li>
            <li class="px-4 py-2.5">
              {{ t("地图底图：vi7ual（spriters-resource 投稿 502127 / 502132 / 502133），仅按非商业用途随包分发；版权归原权利人，商业用途需另行取得许可。", "Map basemaps: vi7ual (spriters-resource submissions 502127 / 502132 / 502133), distributed for non-commercial use only; copyright remains with the original rights holders and commercial use requires separate permission.") }}
            </li>
          </ul>

          <details class="ui-disclosure ui-panel mt-2 px-4 py-2">
            <summary class="flex cursor-pointer items-center justify-between gap-2 text-[13px] font-medium text-accent-strong">
              {{ t("第三方组件与许可通知（离线全文）", "Third-party components and license notices (offline full text)") }}
              <ChevronDown class="ui-chevron" :size="16" aria-hidden="true" />
            </summary>
            <pre class="mono mt-2 max-h-80 overflow-auto whitespace-pre-wrap break-words text-[12px] leading-[1.55] text-muted">{{ noticesText }}</pre>
          </details>

          <details
            class="ui-disclosure ui-panel mt-2 px-4 py-2"
            data-testid="about-license-full"
            @toggle="onLicensesToggle"
          >
            <summary class="flex cursor-pointer items-center justify-between gap-2 text-[13px] font-medium text-accent-strong">
              {{ t("依赖许可全文（离线，按包分组）", "Full dependency license texts (offline, grouped by package)") }}
              <ChevronDown class="ui-chevron" :size="16" aria-hidden="true" />
            </summary>
            <p v-if="licensesLoading" class="mt-2 text-xs text-muted">
              {{ t("正在载入许可全文…", "Loading full license texts…") }}
            </p>
            <p v-else-if="licensesError" class="mt-2 text-xs text-danger" role="alert">
              {{ t("无法载入许可全文：", "Could not load full license texts: ") }}{{ licensesError }}
            </p>
            <pre
              v-else-if="licensesText"
              class="mono mt-2 max-h-96 overflow-auto whitespace-pre-wrap break-words text-[12px] leading-[1.55] text-muted"
              data-testid="license-full-text"
            >{{ licensesText }}</pre>
          </details>
        </section>

        <section>
          <h2 class="ui-section-title">{{ t("快捷键", "Keyboard shortcuts") }}</h2>
          <dl class="ui-panel mt-2 divide-y divide-line/60 text-[13px] leading-5">
            <div v-for="shortcut in shortcuts" :key="shortcut.keys" class="flex items-baseline gap-4 px-4 py-2">
              <dt class="mono w-24 shrink-0">{{ shortcut.keys }}</dt>
              <dd class="min-w-0">{{ shortcut.action }}</dd>
            </div>
          </dl>
        </section>
      </div>
    </div>
  </section>
</template>
