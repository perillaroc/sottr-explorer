<script setup lang="ts">
// 设置（2026-10-31 从「设置与关于」拆出；2026-11-05 界面美化阶段五）：
// 只放可调项，不放说明性文本。数据版本、支持范围与许可等归属信息在「关于」页。
// 版面：一个工作面分「显示 / 布局 / 外部链接」三个分区；标签 128px、控件左对齐，
// 窄窗标签独占一行；单选保留原生 radio，视觉为分段选择。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { t, type LanguagePreference } from "../app/i18n";
import { LANGUAGE_OPTIONS } from "../app/locales";
import { pagePaddingClass } from "../app/layout";
import type { LinkOpenMode, SearchEngine } from "../app/prefs";
import type { ThemePreference } from "../app/theme";
import AppPageHeader from "./AppPageHeader.vue";
import UiButton from "./ui/UiButton.vue";

const store = useStore();

const layout = computed(() => store.layout.value);
const pageX = computed(() => pagePaddingClass(layout.value));
const narrow = computed(() => layout.value === "narrow");

// 语言选项来自注册表：新增语言只需改 locales/index.ts，这里不写死。
const languageOptions = computed<{ value: LanguagePreference; label: string; hint: string }[]>(() =>
  LANGUAGE_OPTIONS.map((option) => ({
    value: option.value,
    label: option.labelKey ? t(option.labelKey) : (option.label ?? option.value),
    hint: t(option.hintKey),
  })),
);

const themeOptions = computed<{ value: ThemePreference; label: string; hint: string }[]>(() => [
  { value: "system", label: t("settingsPage.system"), hint: t("settingsPage.follow-the-system-light-dark-setting") },
  { value: "light", label: t("settingsPage.light"), hint: t("settingsPage.always-use-the-light-theme") },
  { value: "dark", label: t("settingsPage.dark"), hint: t("settingsPage.always-use-the-dark-theme") },
]);

const searchEngineOptions = computed<{ value: SearchEngine; label: string }[]>(() => [
  { value: "bing", label: "Bing" },
  { value: "google", label: "Google" },
  { value: "baidu", label: t("settingsPage.baidu") },
]);

const linkOpenModeOptions = computed<{ value: LinkOpenMode; label: string; hint: string }[]>(() => [
  {
    value: "open",
    label: t("settingsPage.open-browser"),
    hint: t("settingsPage.open-in-the-system-default-browser"),
  },
  {
    value: "copy",
    label: t("settingsPage.copy-link-only"),
    hint: t("settingsPage.copy-the-address-to-the-clipboard-only"),
  },
]);

const languageHint = computed(
  () => languageOptions.value.find((option) => option.value === store.state.language)?.hint ?? "",
);
const themeHint = computed(
  () => themeOptions.value.find((option) => option.value === store.state.theme)?.hint ?? "",
);
const linkOpenHint = computed(
  () => linkOpenModeOptions.value.find((option) => option.value === store.state.linkOpenMode)?.hint ?? "",
);

const rowClass = computed(() =>
  narrow.value ? "flex flex-col items-start gap-2 py-3" : "flex flex-wrap items-center gap-x-3 gap-y-2 py-2.5",
);
const labelClass = computed(() =>
  narrow.value ? "shrink-0 text-[13px] font-medium" : "w-32 shrink-0 text-[13px] font-medium",
);

const segmentGroup =
  "inline-flex flex-wrap gap-0.5 rounded-control border border-line bg-raised p-1";

function segmentClass(active: boolean): string {
  const base =
    "ui-interactive inline-flex h-7 cursor-pointer items-center rounded-[4px] border px-3 text-[13px] leading-5 focus-within:ring-2 focus-within:ring-[var(--focus)]";
  return active
    ? `${base} border-line bg-surface font-medium text-ink shadow-sm`
    : `${base} border-transparent text-muted hover:text-ink`;
}
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="settings-title">
    <div class="mx-auto w-full max-w-[840px] pb-8" :class="pageX">
      <AppPageHeader title-id="settings-title" :title="t('common.settings')" />

      <div class="ui-panel mt-4">
        <section class="border-b border-line px-4 py-3">
          <h2 class="ui-section-title">{{ t("settingsPage.display") }}</h2>
          <div class="mt-1 divide-y divide-line/50">
            <div :class="rowClass" data-testid="language-settings">
              <span :class="labelClass">{{ t("settingsPage.language") }}</span>
              <div class="min-w-0 flex-1">
                <div :class="segmentGroup" role="radiogroup" :aria-label="t('settingsPage.interface-language')">
                  <label
                    v-for="option in languageOptions"
                    :key="option.value"
                    :class="segmentClass(store.state.language === option.value)"
                    :title="option.hint"
                  >
                    <input
                      type="radio"
                      name="language"
                      class="sr-only"
                      :value="option.value"
                      :checked="store.state.language === option.value"
                      :data-testid="`language-${option.value}`"
                      @change="store.actions.setLanguage(option.value)"
                    />
                    {{ option.label }}
                  </label>
                </div>
                <p class="mt-1 text-xs text-muted" data-testid="language-hint">{{ languageHint }}</p>
              </div>
            </div>

            <div :class="rowClass">
              <span :class="labelClass">{{ t("settingsPage.theme") }}</span>
              <div class="min-w-0 flex-1">
                <div :class="segmentGroup" role="radiogroup" :aria-label="t('settingsPage.theme')">
                  <label
                    v-for="option in themeOptions"
                    :key="option.value"
                    :class="segmentClass(store.state.theme === option.value)"
                    :title="option.hint"
                  >
                    <input
                      type="radio"
                      name="theme"
                      class="sr-only"
                      :value="option.value"
                      :checked="store.state.theme === option.value"
                      :data-testid="`theme-${option.value}`"
                      @change="store.actions.setTheme(option.value)"
                    />
                    {{ option.label }}
                  </label>
                </div>
                <p class="mt-1 text-xs text-muted" data-testid="theme-hint">{{ themeHint }}</p>
              </div>
            </div>
          </div>
        </section>

        <section class="border-b border-line px-4 py-3">
          <h2 class="ui-section-title">{{ t("settingsPage.layout") }}</h2>
          <div class="mt-1">
            <div :class="rowClass">
              <span :class="labelClass">{{ t("settingsPage.navigation-details") }}</span>
              <div class="min-w-0 flex-1 space-y-1">
                <UiButton size="sm" data-testid="reset-layout" @click="store.actions.resetLayout()">
                  {{ t("settingsPage.restore-default-layout") }}
                </UiButton>
                <p class="text-xs text-muted">
                  {{ t("settingsPage.restores-the-expanded-navigation-detail-width") }}
                </p>
              </div>
            </div>
          </div>
        </section>

        <section class="px-4 py-3" data-testid="external-links-settings">
          <h2 class="ui-section-title">{{ t("settingsPage.external-links") }}</h2>
          <div class="mt-1 divide-y divide-line/50">
            <div :class="rowClass">
              <span :class="labelClass">{{ t("settingsPage.search-engine") }}</span>
              <div class="min-w-0 flex-1">
                <div :class="segmentGroup" role="radiogroup" :aria-label="t('settingsPage.search-engine')">
                  <label
                    v-for="option in searchEngineOptions"
                    :key="option.value"
                    :class="segmentClass(store.state.searchEngine === option.value)"
                  >
                    <input
                      type="radio"
                      name="search-engine"
                      class="sr-only"
                      :value="option.value"
                      :checked="store.state.searchEngine === option.value"
                      :data-testid="`search-engine-${option.value}`"
                      @change="store.actions.setSearchEngine(option.value)"
                    />
                    {{ option.label }}
                  </label>
                </div>
              </div>
            </div>

            <div :class="rowClass">
              <span :class="labelClass">{{ t("settingsPage.link-behavior") }}</span>
              <div class="min-w-0 flex-1">
                <div :class="segmentGroup" role="radiogroup" :aria-label="t('settingsPage.open-mode')">
                  <label
                    v-for="option in linkOpenModeOptions"
                    :key="option.value"
                    :class="segmentClass(store.state.linkOpenMode === option.value)"
                    :title="option.hint"
                  >
                    <input
                      type="radio"
                      name="link-open-mode"
                      class="sr-only"
                      :value="option.value"
                      :checked="store.state.linkOpenMode === option.value"
                      :data-testid="`link-open-${option.value}`"
                      @change="store.actions.setLinkOpenMode(option.value)"
                    />
                    {{ option.label }}
                  </label>
                </div>
                <p class="mt-1 text-xs text-muted" data-testid="link-open-hint">{{ linkOpenHint }}</p>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  </section>
</template>
