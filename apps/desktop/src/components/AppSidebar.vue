<script setup lang="ts">
// 侧栏（2026-11-05 界面美化阶段二）：展开 224px / 折叠 64px；一级导航 40px、
// 区域入口 36px；选中为 3px 左侧标记 + accent-bg + accent-strong。
// 区域按主区域/特殊区域分组，分组标题 12px 正常大小写，底部诊断/设置/关于固定。
import { computed } from "vue";
import {
  Activity,
  ClipboardList,
  FolderOpen,
  Info,
  LayoutDashboard,
  ListTree,
  MapPinned,
  Settings,
  Trophy,
} from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat, tFormatPair } from "../app/i18n";
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

function go(view: "import" | "overview" | "full" | "plan" | "regions" | "map" | "diagnostics" | "settings" | "about"): void {
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
    :aria-label="t('appSidebar.main-navigation')"
    data-testid="app-sidebar"
  >
    <ul class="flex-1 space-y-0.5 overflow-y-auto px-1.5 pt-3 pb-1.5">
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'import')"
          :aria-current="store.state.view === 'import' ? 'page' : undefined"
          :title="t('appSidebar.open-save')"
          @click="go('import')"
        >
          <FolderOpen :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("appSidebar.open-save") }}</span>
          <span v-else class="sr-only">{{ t("appSidebar.open-save") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'overview')"
          :disabled="!hasResult"
          :title="hasResult ? t('appSidebar.overview') : t('appSidebar.overview-analyze-a-save-file-first')"
          :aria-current="store.state.view === 'overview' ? 'page' : undefined"
          @click="go('overview')"
        >
          <LayoutDashboard :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("appSidebar.overview") }}</span>
          <span v-else class="sr-only">{{ t("appSidebar.overview") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'full')"
          :disabled="!hasResult"
          :title="hasResult ? t('appSidebar.full-collection-100') : t('appSidebar.full-collection-100-analyze-a-save-file')"
          :aria-current="store.state.view === 'full' ? 'page' : undefined"
          @click="go('full')"
        >
          <Trophy :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("appSidebar.full-collection-100") }}</span>
          <span v-else class="sr-only">{{ t("appSidebar.full-collection-100") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'plan')"
          :disabled="!hasResult"
          :title="hasResult ? t('appSidebar.collection-plan-where-to-go-next') : t('appSidebar.collection-plan-analyze-a-save-file-first')"
          :aria-current="store.state.view === 'plan' ? 'page' : undefined"
          @click="go('plan')"
        >
          <ClipboardList :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("appSidebar.collection-plan") }}</span>
          <span v-else class="sr-only">{{ t("appSidebar.collection-plan") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'regions')"
          :disabled="!hasResult"
          :title="hasResult ? t('appSidebar.regions-collection') : t('appSidebar.regions-collection-analyze-a-save-file-first')"
          :aria-current="store.state.view === 'regions' ? 'page' : undefined"
          @click="go('regions')"
        >
          <ListTree :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("appSidebar.regions-collection") }}</span>
          <span v-else class="sr-only">{{ t("appSidebar.regions-collection") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'map')"
          :disabled="!hasResult"
          :title="hasResult ? t('appSidebar.map') : t('appSidebar.map-analyze-a-save-file-first')"
          :aria-current="store.state.view === 'map' ? 'page' : undefined"
          @click="go('map')"
        >
          <MapPinned :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("appSidebar.map") }}</span>
          <span v-else class="sr-only">{{ t("appSidebar.map") }}</span>
        </button>
      </li>

      <template v-if="props.expanded">
        <li :class="groupLabel">{{ t("appSidebar.regions") }}</li>
        <li>
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === 'all', true)"
            :disabled="!hasResult"
            :title="hasResult ? t('common.all-regions') : t('appSidebar.region-list-analyze-a-save-file-first')"
            @click="goRegion('all')"
          >
            {{ t("common.all-regions") }}
          </button>
        </li>
        <li v-for="region in mainRegions" :key="region.id">
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === region.id, true)"
            :disabled="!hasResult"
            :title="hasResult ? tFormat('appSidebar.collected',  { region: region.label, collected: region.collected, total: region.total }) : t('appSidebar.region-list-analyze-a-save-file-first')"
            @click="goRegion(region.id)"
          >
            <span class="min-w-0 flex-1 truncate text-left">{{ region.label }}</span>
            <span class="shrink-0 text-xs tabular-nums text-muted">{{ region.collected }}/{{ region.total }}</span>
          </button>
        </li>

        <li :class="groupLabel">{{ t("common.special") }}</li>
        <li v-for="region in specialRegions" :key="region.id">
          <button
            type="button"
            :class="itemClass(store.state.view === 'regions' && store.state.region === region.id, true)"
            :disabled="!hasResult"
            :title="hasResult ? tFormat('appSidebar.collected',  { region: region.label, collected: region.collected, total: region.total }) : t('appSidebar.region-list-analyze-a-save-file-first')"
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
            :title="hasResult ? t('appSidebar.off-catalog-records-outfits-skills-etc') : t('appSidebar.region-list-analyze-a-save-file-first')"
            @click="goRegion('unknown')"
          >
            {{ tFormatPair(`appSidebar.off-catalog-records`, { v0: store.plan.value.observed_not_in_catalog.length }) }}
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
          :title="hasResult ? t('common.diagnostics') : t('appSidebar.diagnostics-analyze-a-save-file-first')"
          :aria-current="store.state.view === 'diagnostics' ? 'page' : undefined"
          @click="go('diagnostics')"
        >
          <Activity :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("common.diagnostics") }}</span>
          <span v-else class="sr-only">{{ t("common.diagnostics") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'settings')"
          :aria-current="store.state.view === 'settings' ? 'page' : undefined"
          :title="t('common.settings')"
          @click="go('settings')"
        >
          <Settings :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("common.settings") }}</span>
          <span v-else class="sr-only">{{ t("common.settings") }}</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'about')"
          :aria-current="store.state.view === 'about' ? 'page' : undefined"
          :title="t('common.about')"
          @click="go('about')"
        >
          <Info :size="18" class="shrink-0" aria-hidden="true" />
          <span v-if="props.expanded" class="min-w-0 flex-1 truncate">{{ t("common.about") }}</span>
          <span v-else class="sr-only">{{ t("common.about") }}</span>
        </button>
      </li>
    </ul>
  </nav>
</template>
