<script setup lang="ts">
// 侧栏（2026-11-05 界面美化阶段二）：展开 224px / 折叠 64px；一级导航 40px、
// 区域入口 36px；选中为 3px 左侧标记 + accent-bg + accent-strong。
// 区域按主区域/特殊区域分组，分组标题 12px 正常大小写，底部诊断/设置/关于固定。
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

const hasResult = computed(() => store.currentSlot.value?.result != null);
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

const navItem =
  "ui-interactive flex h-10 w-full items-center gap-2.5 rounded-control px-2.5 text-left text-[13px] disabled:cursor-not-allowed disabled:opacity-50";
const regionItem =
  "ui-interactive flex h-9 w-full items-center gap-2 rounded-control px-2.5 text-left text-[13px] disabled:cursor-not-allowed disabled:opacity-50";
const groupLabel = "mt-3 border-t border-line px-2 pt-2 text-xs font-medium text-muted";

function itemClass(active: boolean, region = false): string {
  const base = region ? regionItem : navItem;
  const state = active
    ? "ui-row-selected font-medium text-accent-strong"
    : "text-ink enabled:hover:bg-raised";
  const collapsed = !props.expanded && !region ? "justify-center px-0" : "";
  return `${base} ${state} ${collapsed}`;
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
    :class="props.expanded ? 'w-[224px]' : 'w-16'"
    :aria-label="t('主导航', 'Main navigation')"
    data-testid="app-sidebar"
  >
    <ul class="flex-1 space-y-0.5 overflow-y-auto px-1.5 pt-3 pb-1.5">
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'import')"
          :aria-current="store.state.view === 'import' ? 'page' : undefined"
          :title="t('打开存档', 'Open save')"
          @click="go('import')"
        >
          <FolderOpen :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("打开存档", "Open save") }}</span>
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
          <LayoutDashboard :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("概览", "Overview") }}</span>
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
          <Trophy :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("全收集（100%）", "Full collection (100%)") }}</span>
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
          <ListTree :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("区域与收集", "Regions & Collection") }}</span>
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
          <MapPinned :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("地图查询", "Map") }}</span>
          <span v-else class="sr-only">{{ t("地图查询", "Map") }}</span>
        </button>
      </li>

      <template v-if="props.expanded">
        <li :class="groupLabel">{{ t("区域", "Regions") }}</li>
        <li>
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === 'all', true)"
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
            :class="itemClass(store.state.view === 'regions' && store.state.region === region.id, true)"
            :disabled="!hasResult"
            :title="hasResult ? tFormat('{region}：已收集 {collected}/{total}', '{region}: collected {collected}/{total}', { region: region.label, collected: region.collected, total: region.total }) : t('区域列表：先分析一个存档文件', 'Region list: analyze a save file first')"
            @click="goRegion(region.id)"
          >
            <span class="min-w-0 flex-1 truncate text-left">{{ region.label }}</span>
            <span class="shrink-0 text-xs tabular-nums text-muted">{{ region.collected }}/{{ region.total }}</span>
          </button>
        </li>

        <li :class="groupLabel">{{ t("特殊区域", "Special") }}</li>
        <li v-for="region in specialRegions" :key="region.id">
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === region.id, true)"
            :disabled="!hasResult"
            :title="hasResult ? tFormat('{region}：已收集 {collected}/{total}', '{region}: collected {collected}/{total}', { region: region.label, collected: region.collected, total: region.total }) : t('区域列表：先分析一个存档文件', 'Region list: analyze a save file first')"
            @click="goRegion(region.id)"
          >
            <span class="min-w-0 flex-1 truncate text-left">{{ region.label }}</span>
            <span class="shrink-0 text-xs tabular-nums text-muted">{{ region.collected }}/{{ region.total }}</span>
          </button>
        </li>
        <li>
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === 'unknown', true)"
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
          <Activity :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("诊断", "Diagnostics") }}</span>
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
          <Settings :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("设置", "Settings") }}</span>
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
          <Info :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("关于", "About") }}</span>
          <span v-else class="sr-only">{{ t("关于", "About") }}</span>
        </button>
      </li>
    </ul>
  </nav>
</template>
