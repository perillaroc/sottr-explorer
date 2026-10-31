<script setup lang="ts">
// 设置（2026-10-31 从「设置与关于」拆出）：只放可调项，不放说明性文本。
// 数据版本、支持范围与许可等归属信息在「关于」页。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { t, type LanguagePreference } from "../app/i18n";
import type { LinkOpenMode, SearchEngine } from "../app/prefs";
import type { ThemePreference } from "../app/theme";
import UiButton from "./ui/UiButton.vue";

const store = useStore();

const languageOptions: { value: LanguagePreference; label: string; hint: string }[] = [
  { value: "zh", label: t("中文", "Chinese"), hint: "仅显示中文" },
  { value: "en", label: "English", hint: "Show English only" },
  { value: "both", label: t("中英对照", "Bilingual"), hint: "中文与英文并列显示" },
];

const themeOptions = computed<{ value: ThemePreference; label: string }[]>(() => [
  { value: "system", label: t("跟随系统", "System") },
  { value: "light", label: t("浅色", "Light") },
  { value: "dark", label: t("深色", "Dark") },
]);

const searchEngineOptions = computed<{ value: SearchEngine; label: string }[]>(() => [
  { value: "bing", label: "Bing" },
  { value: "google", label: "Google" },
  { value: "baidu", label: t("百度", "Baidu") },
]);

const linkOpenModeOptions = computed<{ value: LinkOpenMode; label: string; hint: string }[]>(() => [
  {
    value: "open",
    label: t("打开浏览器", "Open browser"),
    hint: t("交给系统默认浏览器打开", "Open in the system default browser"),
  },
  {
    value: "copy",
    label: t("仅复制链接", "Copy link only"),
    hint: t("只把地址复制到剪贴板", "Copy the address to the clipboard only"),
  },
]);

const segmentGroup = "inline-flex flex-wrap gap-1 rounded-[8px] border border-line bg-raised p-1";

function segmentClass(active: boolean): string {
  const base =
    "inline-flex cursor-pointer items-center rounded-[6px] px-3 py-1 text-[13px] transition-colors focus-within:ring-2 focus-within:ring-[var(--focus)]";
  return active ? `${base} bg-surface text-ink shadow-[0_1px_2px_rgba(15,23,42,0.14)]` : `${base} text-muted hover:text-ink`;
}
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="settings-title">
    <div class="mx-auto max-w-3xl space-y-4 px-6 py-6">
      <h1 id="settings-title" class="text-lg font-semibold">{{ t("设置", "Settings") }}</h1>

      <section class="divide-y divide-line/60 overflow-hidden rounded-[10px] border border-line bg-surface">
        <div class="flex flex-wrap items-center gap-3 px-4 py-3" data-testid="language-settings">
          <span class="w-24 shrink-0 text-[13px] font-medium">{{ t("语言", "Language") }}</span>
          <div :class="segmentGroup" role="radiogroup" :aria-label="t('界面语言', 'Interface language')">
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
        </div>

        <div class="flex flex-wrap items-center gap-3 px-4 py-3">
          <span class="w-24 shrink-0 text-[13px] font-medium">{{ t("主题", "Theme") }}</span>
          <div :class="segmentGroup" role="radiogroup" :aria-label="t('主题', 'Theme')">
            <label
              v-for="option in themeOptions"
              :key="option.value"
              :class="segmentClass(store.state.theme === option.value)"
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
        </div>

        <div class="flex flex-wrap items-center gap-3 px-4 py-3">
          <span class="w-24 shrink-0 text-[13px] font-medium">{{ t("布局", "Layout") }}</span>
          <UiButton size="sm" data-testid="reset-layout" @click="store.actions.resetLayout()">
            {{ t("恢复默认布局", "Restore default layout") }}
          </UiButton>
        </div>

        <div class="flex flex-wrap items-center gap-3 px-4 py-3" data-testid="external-links-settings">
          <span class="w-24 shrink-0 text-[13px] font-medium">{{ t("搜索引擎", "Search engine") }}</span>
          <div :class="segmentGroup" role="radiogroup" :aria-label="t('搜索引擎', 'Search engine')">
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

        <div class="flex flex-wrap items-center gap-3 px-4 py-3">
          <span class="w-24 shrink-0 text-[13px] font-medium">{{ t("链接打开方式", "Link behavior") }}</span>
          <div :class="segmentGroup" role="radiogroup" :aria-label="t('打开方式', 'Open mode')">
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
        </div>
      </section>
    </div>
  </section>
</template>
