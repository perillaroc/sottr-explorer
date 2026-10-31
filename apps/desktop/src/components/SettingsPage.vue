<script setup lang="ts">
// 设置与关于（SX-015；2026-10-20 增加语言模式）：
// 只保留必要展示项（语言、主题、布局恢复、版本与范围、许可证）。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { t, type LanguagePreference } from "../app/i18n";
import { copy } from "../app/labels";
import { supportedCategories } from "../app/catalog";
import type { LinkOpenMode, SearchEngine } from "../app/prefs";
import type { ThemePreference } from "../app/theme";
import UiButton from "./ui/UiButton.vue";
// SX-018：许可通知全文随前端内嵌（构建期内嵌，离线可读）。
import noticesText from "../../../../THIRD_PARTY_NOTICES.txt?raw";

const store = useStore();

const languageOptions: { value: LanguagePreference; label: string; hint: string }[] = [
  { value: "zh", label: "中文", hint: "仅显示中文" },
  { value: "en", label: "English", hint: "Show English only" },
  { value: "both", label: "中英文对照 / Bilingual", hint: "中文与英文并列显示" },
];

const themeOptions = computed<{ value: ThemePreference; label: string }[]>(() => [
  { value: "system", label: t("跟随系统", "Follow system") },
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
    hint: t("只把地址复制到剪贴板，不打开浏览器", "Copy the address to the clipboard without opening a browser"),
  },
]);

/** 单选组样式（与主题组同一套令牌）。 */
function radioClass(active: boolean): string {
  const base = "inline-flex cursor-pointer items-center gap-2 rounded-[6px] border px-3 py-1.5 text-[13px]";
  return active ? `${base} border-accent bg-accent-bg text-accent-strong` : `${base} border-line bg-surface text-ink`;
}

const report = computed(() => store.report.value);
const categories = computed(() => (report.value ? supportedCategories(store.metadata, report.value) : []));

const shortcuts = computed(() => [
  { keys: "Ctrl+O", action: t("打开文件（选择存档）", "Open a file (choose a save)") },
  { keys: "Ctrl+F", action: t("聚焦当前清单搜索", "Focus the current list search") },
  { keys: "F5", action: t("刷新当前槽位（手动）", "Refresh the current slot (manual)") },
  { keys: "Esc", action: t("关闭菜单或详情抽屉", "Close a menu or the details drawer") },
  {
    keys: "Tab / Shift+Tab",
    action: t(
      "顶栏 → 导航 → 工具栏 → 清单 → 详情",
      "Top bar → navigation → toolbar → list → details",
    ),
  },
]);
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="settings-title">
    <div class="mx-auto max-w-3xl space-y-4">
      <h1 id="settings-title" class="text-xl font-semibold">{{ t("设置与关于", "Settings & About") }}</h1>

      <section class="space-y-2" data-testid="language-settings">
        <h2 class="text-base font-medium">{{ t("语言", "Language") }}</h2>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" :aria-label="t('界面语言', 'Interface language')">
          <label
            v-for="option in languageOptions"
            :key="option.value"
            :class="radioClass(store.state.language === option.value)"
            :title="option.hint"
          >
            <input
              type="radio"
              name="language"
              :value="option.value"
              :checked="store.state.language === option.value"
              :data-testid="`language-${option.value}`"
              @change="store.actions.setLanguage(option.value)"
            />
            {{ option.label }}
          </label>
        </div>
        <p class="text-xs text-muted">
          {{ t("界面文字默认中文；图鉴名称、区域与获取说明按当前语言显示；分析报告中的诊断/限制文本保留数据源原文。", "Interface text defaults to Chinese; catalog names, regions and acquisition notes follow the selected language; diagnostic/limitation text in reports keeps the data source's original wording.") }}
        </p>
      </section>

      <section class="space-y-2">
        <h2 class="text-base font-medium">{{ t("主题", "Theme") }}</h2>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" :aria-label="t('主题', 'Theme')">
          <label
            v-for="option in themeOptions"
            :key="option.value"
            :class="radioClass(store.state.theme === option.value)"
          >
            <input
              type="radio"
              name="theme"
              :value="option.value"
              :checked="store.state.theme === option.value"
              :data-testid="`theme-${option.value}`"
              @change="store.actions.setTheme(option.value)"
            />
            {{ option.label }}
          </label>
        </div>
        <p class="text-xs text-muted">
          {{ t("令牌与对比度目标见开发方案；Windows 高对比度/DPI 实测由 SX-017 记录。", "See the development plan for tokens and contrast targets; Windows high-contrast/DPI checks are recorded in SX-017.") }}
        </p>
      </section>

      <section class="space-y-2">
        <h2 class="text-base font-medium">{{ t("布局", "Layout") }}</h2>
        <div class="flex flex-wrap items-center gap-2">
          <UiButton size="sm" @click="store.actions.resetLayout()">{{ t("恢复默认布局", "Restore default layout") }}</UiButton>
          <span class="text-xs text-muted">
            {{ t("导航展开、详情常驻；首版不提供窗格拖拽调宽。", "Navigation expanded and details persistent; pane resizing by dragging is not available in the first release.") }}
          </span>
        </div>
      </section>

      <section class="space-y-2" data-testid="external-links-settings">
        <h2 class="text-base font-medium">{{ t("外部链接", "External links") }}</h2>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" :aria-label="t('搜索引擎', 'Search engine')">
          <label
            v-for="option in searchEngineOptions"
            :key="option.value"
            :class="radioClass(store.state.searchEngine === option.value)"
          >
            <input
              type="radio"
              name="search-engine"
              :value="option.value"
              :checked="store.state.searchEngine === option.value"
              :data-testid="`search-engine-${option.value}`"
              @change="store.actions.setSearchEngine(option.value)"
            />
            {{ option.label }}
          </label>
        </div>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" :aria-label="t('打开方式', 'Open mode')">
          <label
            v-for="option in linkOpenModeOptions"
            :key="option.value"
            :class="radioClass(store.state.linkOpenMode === option.value)"
            :title="option.hint"
          >
            <input
              type="radio"
              name="link-open-mode"
              :value="option.value"
              :checked="store.state.linkOpenMode === option.value"
              :data-testid="`link-open-${option.value}`"
              @change="store.actions.setLinkOpenMode(option.value)"
            />
            {{ option.label }}
          </label>
        </div>
        <p class="text-xs text-muted">
          {{ t("详情页的攻略/搜索链接按此设置处理；「仅复制链接」时不会唤起浏览器。应用内始终不加载网页。", "Guide/search links on the details page follow this setting; “Copy link only” does not launch a browser. The app never loads web pages in-app.") }}
        </p>
      </section>

      <section class="space-y-2">
        <h2 class="text-base font-medium">{{ t("数据与范围", "Data & scope") }}</h2>
        <ul class="space-y-1 text-[13px]">
          <li>{{ t("应用版本：0.1.0（模拟数据界面，SX-015）", "App version: 0.1.0 (mock-data interface, SX-015)") }}</li>
          <li>{{ t("元数据版本", "Metadata version") }}：{{ store.metadata.metadata_version }}（{{ store.metadata.namespace }}）</li>
          <li>
            {{ t("已支持范围：普通/稀有贮藏点、石碑、地下墓穴宝藏的参考遗漏推定；已验证坐标 0 项；区域百分比暂不支持。", "Supported scope: reference-gap inference for survival/rare caches, monoliths and crypt treasures; 0 verified coordinates; region percentages are not supported yet.") }}
          </li>
          <li v-if="categories.length">
            {{ t("当前报告支持类别", "Categories supported by the current report") }}：{{ categories.map((category) => category.label).join(t("、", ", ")) }}
          </li>
          <li>{{ t("许可证：代码采用 MIT（根目录 LICENSE）；地图底图与游戏数据不受 MIT 覆盖；第三方来源与许可见仓库 THIRD_PARTY_NOTICES.md。", "License: the code is MIT (see LICENSE); map basemaps and game data are not covered by MIT; see THIRD_PARTY_NOTICES.md in the repository for third-party sources and licenses.") }}</li>
          <li>{{ t("本地只读：无账号、无遥测、无后台上传；导出默认分享脱敏（接线在 SX-016）。", "Local and read-only: no accounts, telemetry or background uploads; exports are share-redacted by default (wired in SX-016).") }}</li>
        </ul>
        <p class="rounded-[6px] border border-caution/40 bg-caution-bg px-3 py-2 text-xs text-caution">
          {{ copy.mockBanner }}
        </p>
      </section>

      <section class="space-y-2" data-testid="about-notices">
        <h2 class="text-base font-medium">{{ t("关于与许可", "About & licenses") }}</h2>
        <ul class="space-y-1 text-[13px]">
          <li>
            {{ t("交付形态：单文件免安装 exe（Windows）；运行依赖系统 WebView2 Runtime（Windows 11 与多数 Windows 10 已预装，缺失时从 Microsoft 官方获取）。", "Delivery: a single portable exe (Windows); it requires the system WebView2 Runtime (preinstalled on Windows 11 and most Windows 10; obtain it from Microsoft if missing).") }}
          </li>
          <li>{{ t("本构建未做代码签名：SmartScreen 可能提示“未知发布者”；未声称已通过 SmartScreen。", "This build is not code-signed: SmartScreen may warn about an “unknown publisher”; no SmartScreen approval is claimed.") }}</li>
          <li>
            {{ t("代码采用 MIT；地图底图、游戏数据事实与第三方组件各自保留原权利，完整声明见下方离线通知全文与仓库 THIRD_PARTY_NOTICES.md。", "The code is MIT; map basemaps, game data facts and third-party components keep their own terms — see the offline notices below and THIRD_PARTY_NOTICES.md in the repository.") }}
          </li>
          <li>
            {{ t("地图底图：vi7ual（spriters-resource 投稿 502127 / 502132 / 502133），站点声明素材供 personal projects and non-commercial work；版权归原权利人，商业用途需另行取得许可。", "Map basemaps: vi7ual (spriters-resource submissions 502127 / 502132 / 502133), published for personal projects and non-commercial work; copyright remains with the original rights holders and commercial use requires separate permission.") }}
          </li>
        </ul>
        <details class="rounded-[6px] border border-line bg-surface px-3 py-2">
          <summary class="cursor-pointer text-[13px] text-accent-strong">
            {{ t("第三方组件与许可通知（离线全文）", "Third-party components and license notices (offline full text)") }}
          </summary>
          <pre class="mt-2 max-h-80 overflow-auto whitespace-pre-wrap text-[11px] leading-4 text-muted">{{ noticesText }}</pre>
        </details>
      </section>

      <section class="space-y-2">
        <h2 class="text-base font-medium">{{ t("快捷键", "Keyboard shortcuts") }}</h2>
        <ul class="space-y-1 text-[13px]">
          <li v-for="shortcut in shortcuts" :key="shortcut.keys">
            <span class="mono">{{ shortcut.keys }}</span>：{{ shortcut.action }}
          </li>
        </ul>
      </section>
    </div>
  </section>
</template>
