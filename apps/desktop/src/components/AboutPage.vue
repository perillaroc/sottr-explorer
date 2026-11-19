<script setup lang="ts">
// 关于（2026-10-31 从「设置与关于」拆出；2026-11-05 界面美化阶段五）：
// 应用与数据版本、支持范围、许可与快捷键；设置页只放可调项。
// 页头使用 32px 罗盘品牌标记；不添加大幅品牌 hero。
import { computed, ref } from "vue";
import { ChevronDown } from "@lucide/vue";
import { getVersion } from "@tauri-apps/api/app";
import { useStore } from "../app/inject";
import { t, tFormatPair } from "../app/i18n";
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
  { label: t("aboutPage.app-version"), value: appVersion.value },
  { label: t("aboutPage.metadata-version"), value: `${store.metadata.metadata_version}（${store.metadata.namespace}）` },
  { label: t("aboutPage.catalog-version"), value: store.atlas.catalog_version },
  { label: t("aboutPage.basemap-dataset"), value: basemapDatasetVersion },
]);

const shortcuts = computed(() => [
  { keys: "Ctrl+O", action: t("aboutPage.open-a-file-choose-a-save") },
  { keys: "Ctrl+F", action: t("aboutPage.focus-the-current-list-search") },
  { keys: "F5", action: t("aboutPage.refresh-the-current-slot") },
  { keys: "Esc", action: t("aboutPage.close-a-menu-or-the-details-drawer") },
]);
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="about-title">
    <div class="mx-auto w-full max-w-[960px] pb-8" :class="pageX">
      <header class="ui-page-header">
        <div class="flex items-start gap-3">
          <AppBrandMark :size="32" class="mt-0.5 shrink-0" />
          <div class="min-w-0">
            <p class="ui-meta mb-1">{{ t("common.about") }}</p>
            <h1 id="about-title" class="ui-page-title">SOTTR Explorer</h1>
            <p class="mt-1 text-[13px] leading-5 text-muted">
              {{ tFormatPair(`aboutPage.version-read-only-save-analysis`, { v0: appVersion }) }}
            </p>
          </div>
        </div>
      </header>

      <div class="mt-6 space-y-6">
        <section data-testid="about-data">
          <h2 class="ui-section-title">{{ t("aboutPage.data-scope") }}</h2>
          <dl class="ui-panel mt-2 divide-y divide-line/60 text-[13px] leading-5">
            <div v-for="row in versionRows" :key="row.label" class="flex flex-wrap items-baseline gap-x-3 gap-y-1 px-4 py-2.5">
              <dt class="w-28 shrink-0 text-muted">{{ row.label }}</dt>
              <dd class="mono min-w-0 flex-1 break-all">{{ row.value }}</dd>
            </div>
          </dl>
          <p class="mt-2 text-[13px] leading-5">
            {{ t("aboutPage.supported-scope-reference-gap-inference-for-survival") }}
          </p>
          <p v-if="categories.length" class="mt-1 text-[13px] leading-5">
            {{ t("aboutPage.categories-supported-by-the-current-report") }}：
            {{ categories.map((category) => category.label).join(t("common.list-separator")) }}
          </p>
          <p class="mt-1 text-[13px] leading-5 text-muted">
            {{ t("aboutPage.local-and-read-only-no-accounts-telemetry") }}
          </p>
        </section>

        <section data-testid="about-notices">
          <h2 class="ui-section-title">{{ t("aboutPage.licenses") }}</h2>
          <ul class="ui-panel mt-2 divide-y divide-line/60 text-[13px] leading-5">
            <li class="px-4 py-2.5">
              {{ t("aboutPage.the-code-is-mit-see-license-map") }}
            </li>
            <li class="px-4 py-2.5">
              {{ t("aboutPage.delivery-a-single-portable-windows-exe") }}
            </li>
            <li class="px-4 py-2.5">
              {{ t("aboutPage.map-basemaps-vi7ual-spriters-resource-submissions-502127") }}
            </li>
          </ul>

          <details class="ui-disclosure ui-panel mt-2 px-4 py-2">
            <summary class="flex cursor-pointer items-center justify-between gap-2 text-[13px] font-medium text-accent-strong">
              {{ t("aboutPage.third-party-components-and-license-notices-offline") }}
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
              {{ t("aboutPage.full-dependency-license-texts-offline-grouped") }}
              <ChevronDown class="ui-chevron" :size="16" aria-hidden="true" />
            </summary>
            <p v-if="licensesLoading" class="mt-2 text-xs text-muted">
              {{ t("aboutPage.loading-full-license-texts") }}
            </p>
            <p v-else-if="licensesError" class="mt-2 text-xs text-danger" role="alert">
              {{ t("aboutPage.could-not-load-full-license-texts") }}{{ licensesError }}
            </p>
            <pre
              v-else-if="licensesText"
              class="mono mt-2 max-h-96 overflow-auto whitespace-pre-wrap break-words text-[12px] leading-[1.55] text-muted"
              data-testid="license-full-text"
            >{{ licensesText }}</pre>
          </details>
        </section>

        <section>
          <h2 class="ui-section-title">{{ t("aboutPage.keyboard-shortcuts") }}</h2>
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
