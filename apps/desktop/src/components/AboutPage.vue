<script setup lang="ts">
// 关于（2026-10-31 从「设置与关于」拆出）：应用与数据版本、支持范围、许可与快捷键。
// 设置页只放可调项；解释性、归属性与数据来源信息集中在这里。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { supportedCategories } from "../app/catalog";
import { basemapDatasetVersion } from "../data/basemaps";
// SX-018：许可通知全文随前端内嵌（构建期内嵌，离线可读）。
import noticesText from "../../../../THIRD_PARTY_NOTICES.txt?raw";

const store = useStore();

const report = computed(() => store.report.value);
const categories = computed(() => (report.value ? supportedCategories(store.metadata, report.value) : []));

const versionRows = computed(() => [
  { label: t("应用版本", "App version"), value: "0.1.0" },
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
    <div class="mx-auto max-w-3xl space-y-6 px-6 py-6">
      <header class="space-y-0.5">
        <h1 id="about-title" class="text-lg font-semibold">{{ t("关于", "About") }}</h1>
        <p class="text-[13px] text-muted">SOTTR Explorer 0.1.0 · {{ t("只读存档分析", "Read-only save analysis") }}</p>
      </header>

      <section class="space-y-2" data-testid="about-data">
        <h2 class="text-[13px] font-medium text-muted">{{ t("数据与范围", "Data & scope") }}</h2>
        <dl class="divide-y divide-line/60 overflow-hidden rounded-[10px] border border-line bg-surface text-[13px]">
          <div v-for="row in versionRows" :key="row.label" class="flex flex-wrap items-baseline gap-3 px-4 py-2">
            <dt class="w-28 shrink-0 text-muted">{{ row.label }}</dt>
            <dd class="mono min-w-0 break-all">{{ row.value }}</dd>
          </div>
        </dl>
        <p class="text-[13px]">
          {{ t("支持范围：普通/稀有贮藏点、石碑、地下墓穴宝藏的参考遗漏推定；已验证坐标 0 项；区域百分比暂不支持。", "Supported scope: reference-gap inference for survival/rare caches, monoliths and crypt treasures; 0 verified coordinates; region percentages are not supported yet.") }}
        </p>
        <p v-if="categories.length" class="text-[13px]">
          {{ t("当前报告支持类别", "Categories supported by the current report") }}：
          {{ categories.map((category) => category.label).join(t("、", ", ")) }}
        </p>
        <p class="text-[13px] text-muted">
          {{ t("本地只读：无账号、无遥测、无后台上传；导出默认分享脱敏。", "Local and read-only: no accounts, telemetry or background uploads; exports are share-redacted by default.") }}
        </p>
      </section>

      <section class="space-y-2" data-testid="about-notices">
        <h2 class="text-[13px] font-medium text-muted">{{ t("许可", "Licenses") }}</h2>
        <ul class="space-y-1 text-[13px]">
          <li>{{ t("代码采用 MIT（根目录 LICENSE）；地图底图与游戏数据不受 MIT 覆盖；第三方来源与许可见仓库 THIRD_PARTY_NOTICES.md。", "The code is MIT (see LICENSE); map basemaps and game data are not covered by MIT; see THIRD_PARTY_NOTICES.md for third-party sources and licenses.") }}</li>
          <li>
            {{ t("交付形态：单文件免安装 exe（Windows），依赖系统 WebView2 Runtime；本构建未做代码签名，SmartScreen 可能提示未知发布者。", "Delivery: a single portable Windows exe that requires the system WebView2 Runtime; this build is not code-signed, so SmartScreen may warn about an unknown publisher.") }}
          </li>
          <li>
            {{ t("地图底图：vi7ual（spriters-resource 投稿 502127 / 502132 / 502133），仅按非商业用途随包分发；版权归原权利人，商业用途需另行取得许可。", "Map basemaps: vi7ual (spriters-resource submissions 502127 / 502132 / 502133), distributed for non-commercial use only; copyright remains with the original rights holders and commercial use requires separate permission.") }}
          </li>
        </ul>
        <details class="rounded-[10px] border border-line bg-surface px-4 py-2">
          <summary class="cursor-pointer text-[13px] text-accent-strong">
            {{ t("第三方组件与许可通知（离线全文）", "Third-party components and license notices (offline full text)") }}
          </summary>
          <pre class="mt-2 max-h-80 overflow-auto whitespace-pre-wrap text-[11px] leading-4 text-muted">{{ noticesText }}</pre>
        </details>
      </section>

      <section class="space-y-2">
        <h2 class="text-[13px] font-medium text-muted">{{ t("快捷键", "Keyboard shortcuts") }}</h2>
        <dl class="divide-y divide-line/60 overflow-hidden rounded-[10px] border border-line bg-surface text-[13px]">
          <div v-for="shortcut in shortcuts" :key="shortcut.keys" class="flex items-baseline gap-4 px-4 py-2">
            <dt class="mono w-24 shrink-0">{{ shortcut.keys }}</dt>
            <dd class="min-w-0">{{ shortcut.action }}</dd>
          </div>
        </dl>
      </section>
    </div>
  </section>
</template>
