<script setup lang="ts">
// 侧栏（SX-015）：主视图 + 区域快捷入口；折叠时只显示图标（aria-label 保留）。
// 设置与关于拆成两个入口；区域按主区域/特殊区域分组，特殊区域不再逐行挂“特殊”标签。
import { computed } from "vue";
import {
  Activity,
  FolderOpen,
  Info,
  LayoutDashboard,
  ListTree,
  MapPinned,
  Settings,
  Trophy,
} from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
import { regionName } from "../app/collection";

const props = withDefaults(defineProps<{ expanded: boolean }>(), { expanded: true });

const store = useStore();

const hasResult = computed(() => store.currentSlot.value?.result !== null);
const mainRegions = computed(() =>
  store.plan.value.regions
    .filter((region) => region.kind === "main")
    .map((region) => ({
      id: region.slug,
      label: regionName(region),
      collected: region.collected,
      total: region.total,
    })),
);
const specialRegions = computed(() =>
  store.plan.value.regions
    .filter((region) => region.kind === "special")
    .map((region) => ({
      id: region.slug,
      label: regionName(region),
      collected: region.collected,
      total: region.total,
    })),
);

const baseItem =
  "flex w-full items-center gap-2 rounded-[6px] px-2 py-[7px] text-left text-[13px] disabled:cursor-not-allowed disabled:opacity-50";

function itemClass(active: boolean): string {
  return `${baseItem} ${active ? "bg-accent-bg text-accent-strong" : "text-ink hover:bg-raised"}`;
}

function go(view: "import" | "overview" | "full" | "regions" | "map" | "diagnostics" | "settings" | "about"): void {
  store.actions.setView(view);
}

function goRegion(region: string): void {
  store.actions.setRegion(region);
  store.actions.setView("regions");
}
</script>

<template>
  <nav
    class="flex h-full flex-col border-r border-line bg-surface"
    :class="props.expanded ? 'w-[216px]' : 'w-14'"
    :aria-label="t('主导航', 'Main navigation')"
    data-testid="app-sidebar"
  >
    <ul class="flex-1 space-y-0.5 overflow-y-auto p-1.5">
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'import')"
          :aria-current="store.state.view === 'import' ? 'page' : undefined"
          :title="t('打开存档', 'Open save')"
          @click="go('import')"
        >
          <FolderOpen :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("打开存档", "Open save") }}</span>
          <span v-else class="sr-only">{{ t("打开存档", "Open save") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'overview')"
          :disabled="!hasResult"
          :title="hasResult ? t('概览', 'Overview') : t('概览：先分析一个存档文件', 'Overview: analyze a save file first')"
          :aria-current="store.state.view === 'overview' ? 'page' : undefined"
          @click="go('overview')"
        >
          <LayoutDashboard :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("概览", "Overview") }}</span>
          <span v-else class="sr-only">{{ t("概览", "Overview") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'full')"
          :disabled="!hasResult"
          :title="hasResult ? t('全收集（100%）', 'Full collection (100%)') : t('全收集（100%）：先分析一个存档文件', 'Full collection (100%): analyze a save file first')"
          :aria-current="store.state.view === 'full' ? 'page' : undefined"
          @click="go('full')"
        >
          <Trophy :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("全收集（100%）", "Full collection (100%)") }}</span>
          <span v-else class="sr-only">{{ t("全收集（100%）", "Full collection (100%)") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'regions')"
          :disabled="!hasResult"
          :title="hasResult ? t('区域与收集', 'Regions & Collection') : t('区域与收集：先分析一个存档文件', 'Regions & Collection: analyze a save file first')"
          :aria-current="store.state.view === 'regions' ? 'page' : undefined"
          @click="go('regions')"
        >
          <ListTree :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("区域与收集", "Regions & Collection") }}</span>
          <span v-else class="sr-only">{{ t("区域与收集", "Regions & Collection") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'map')"
          :disabled="!hasResult"
          :title="hasResult ? t('地图查询', 'Map') : t('地图查询：先分析一个存档文件', 'Map: analyze a save file first')"
          :aria-current="store.state.view === 'map' ? 'page' : undefined"
          @click="go('map')"
        >
          <MapPinned :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("地图查询", "Map") }}</span>
          <span v-else class="sr-only">{{ t("地图查询", "Map") }}</span>
        </button>
      </li>

      <template v-if="props.expanded">
        <li class="mt-3 px-2 text-[11px] font-medium tracking-wide text-muted">{{ t("区域", "Regions") }}</li>
        <li>
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === 'all')"
            :disabled="!hasResult"
            :title="hasResult ? t('全部区域', 'All regions') : t('区域列表：先分析一个存档文件', 'Region list: analyze a save file first')"
            @click="goRegion('all')"
          >
            {{ t("全部区域", "All regions") }}
          </button>
        </li>
        <li v-for="region in mainRegions" :key="region.id">
          <button
            type="button"
            class="flex w-full items-center gap-2"
            :class="itemClass(store.state.view === 'regions' && store.state.region === region.id)"
            :disabled="!hasResult"
            :title="hasResult ? tFormat('{region}：已收集 {collected}/{total}', '{region}: collected {collected}/{total}', { region: region.label, collected: region.collected, total: region.total }) : t('区域列表：先分析一个存档文件', 'Region list: analyze a save file first')"
            @click="goRegion(region.id)"
          >
            <span class="min-w-0 flex-1 truncate text-left">{{ region.label }}</span>
            <span class="shrink-0 tabular-nums text-xs text-muted">{{ region.collected }}/{{ region.total }}</span>
          </button>
        </li>

        <li class="mt-3 px-2 text-[11px] font-medium tracking-wide text-muted">{{ t("特殊区域", "Special") }}</li>
        <li v-for="region in specialRegions" :key="region.id">
          <button
            type="button"
            class="flex w-full items-center gap-2"
            :class="itemClass(store.state.view === 'regions' && store.state.region === region.id)"
            :disabled="!hasResult"
            :title="hasResult ? tFormat('{region}：已收集 {collected}/{total}', '{region}: collected {collected}/{total}', { region: region.label, collected: region.collected, total: region.total }) : t('区域列表：先分析一个存档文件', 'Region list: analyze a save file first')"
            @click="goRegion(region.id)"
          >
            <span class="min-w-0 flex-1 truncate text-left">{{ region.label }}</span>
            <span class="shrink-0 tabular-nums text-xs text-muted">{{ region.collected }}/{{ region.total }}</span>
          </button>
        </li>
        <li>
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === 'unknown')"
            :disabled="!hasResult"
            :title="hasResult ? t('目录外观察记录（服装 / 技能等，不计入收集品）', 'Off-catalog records (outfits, skills, etc.; not collectibles)') : t('区域列表：先分析一个存档文件', 'Region list: analyze a save file first')"
            @click="goRegion('unknown')"
          >
            {{ t(`目录外记录（${store.plan.value.observed_not_in_catalog.length}）`, `Off-catalog records (${store.plan.value.observed_not_in_catalog.length})`) }}
          </button>
        </li>
      </template>
    </ul>

    <ul class="space-y-0.5 border-t border-line p-1.5">
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'diagnostics')"
          :disabled="!hasResult"
          :title="hasResult ? t('诊断', 'Diagnostics') : t('诊断：先分析一个存档文件', 'Diagnostics: analyze a save file first')"
          :aria-current="store.state.view === 'diagnostics' ? 'page' : undefined"
          @click="go('diagnostics')"
        >
          <Activity :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("诊断", "Diagnostics") }}</span>
          <span v-else class="sr-only">{{ t("诊断", "Diagnostics") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'settings')"
          :aria-current="store.state.view === 'settings' ? 'page' : undefined"
          :title="t('设置', 'Settings')"
          @click="go('settings')"
        >
          <Settings :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("设置", "Settings") }}</span>
          <span v-else class="sr-only">{{ t("设置", "Settings") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'about')"
          :aria-current="store.state.view === 'about' ? 'page' : undefined"
          :title="t('关于', 'About')"
          @click="go('about')"
        >
          <Info :size="16" aria-hidden="true" />
          <span v-if="props.expanded">{{ t("关于", "About") }}</span>
          <span v-else class="sr-only">{{ t("关于", "About") }}</span>
        </button>
      </li>
    </ul>
  </nav>
</template>
