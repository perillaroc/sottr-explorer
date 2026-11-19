<script setup lang="ts">
// 地图查询视图（2026-10-31 地图查询与详情增强·阶段二）：
// 以底图为主，按类别使用不同图标绘制收集品；右侧「详情 / 清单」标签页联动。
//
// 边界（方案 §2.1）：一张底图 = 一个子关卡，不做跨子关卡拼图；点位只是同子关卡内的
// 相对位置参考，不是导航；筛选独立于区域清单，`mapRegion` 不回写 `state.region`。
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { ChevronDown, X } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat, tFormatPair } from "../app/i18n";
import { copy } from "../app/labels";
import { categoryNameOf } from "../app/collection";
import { pagePaddingClass } from "../app/layout";
import { MAP_RAIL_WIDTH } from "../app/prefs";
import {
  defaultMapBasemap,
  filterMapEntries,
  mapEntriesForBasemap,
  mapGroups,
  mapOptions,
  mapOptionsForRegion,
  unmappedInRegion,
  unplacedInBasemap,
  type MapBasemapOption,
  type MapEntry,
  type MapFilters,
} from "../app/map-view";
import CollectionDetail from "./CollectionDetail.vue";
import AppPageHeader from "./AppPageHeader.vue";
import MapCanvas from "./map/MapCanvas.vue";
import MapLegend, { type MapLegendOption } from "./map/MapLegend.vue";
import MapRail from "./map/MapRail.vue";
import CoverageNotice from "./CoverageNotice.vue";
import UiButton from "./ui/UiButton.vue";
import UiDrawer from "./ui/UiDrawer.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiFilterChips from "./ui/UiFilterChips.vue";
import UiIconButton from "./ui/UiIconButton.vue";
import UiResizeHandle from "./ui/UiResizeHandle.vue";
import UiSearchInput from "./ui/UiSearchInput.vue";
import type { ChipOption } from "./ui/types";

const store = useStore();

const canvas = ref<InstanceType<typeof MapCanvas> | null>(null);
const searchInput = ref<InstanceType<typeof UiSearchInput> | null>(null);
const railOverlayOpen = ref(false);
const narrowDetailOpen = ref(false);
const railDraft = ref<number | null>(null);
/** 当前地图上落在底图留白区的条目（由 MapCanvas 采样底图 alpha 得出）。 */
const offMapIds = ref<string[]>([]);

const plan = computed(() => store.plan.value);
const atlas = store.atlas;
const layout = computed(() => store.layout.value);
const pageX = computed(() => pagePaddingClass(layout.value));
const ready = computed(() => store.report.value !== null && store.planAvailable.value);

const inlineRail = computed(() => layout.value === "wide" || layout.value === "medium");
const overlayRail = computed(() => layout.value === "compact");
const narrow = computed(() => layout.value === "narrow");

const options = computed<MapBasemapOption[]>(() => mapOptions(plan.value, atlas));

/** 区域选择：只列有底图的区域（「全部区域」不作为地图选择值）。 */
const regionChoices = computed(() => {
  const seen = new Map<string, string>();
  for (const option of options.value) {
    if (!seen.has(option.regionSlug)) {
      seen.set(option.regionSlug, option.regionName);
    }
  }
  return [...seen.entries()].map(([slug, name]) => ({ slug, name }));
});

const currentOption = computed<MapBasemapOption | null>(() => {
  const id = store.state.mapBasemapId;
  return (
    options.value.find((option) => option.basemapId === id) ??
    defaultMapBasemap(plan.value, atlas, store.state.mapRegion)
  );
});

const basemapChoices = computed(() =>
  mapOptionsForRegion(plan.value, atlas, store.state.mapRegion),
);

const entries = computed<MapEntry[]>(() =>
  currentOption.value ? mapEntriesForBasemap(plan.value, currentOption.value.basemap) : [],
);

const filters = computed<MapFilters>(() => ({
  status: store.state.mapStatus,
  category: store.state.mapCategory,
  search: store.state.mapSearch,
}));

const filteredEntries = computed(() => filterMapEntries(entries.value, filters.value));
const groups = computed(() => mapGroups(entries.value, filters.value));

/** 有筛选时才显示「清除筛选」，避免空态按钮挤压工具头。 */
const filtersActive = computed(
  () =>
    store.state.mapStatus !== "all" ||
    store.state.mapCategory !== null ||
    store.state.mapSearch.trim().length > 0,
);

const statusOptions = computed<ChipOption[]>(() => [
  {
    value: "missing",
    label: t("common.uncollected"),
    count: entries.value.filter((entry) => entry.item.status === "not_collected").length,
  },
  {
    value: "observed",
    label: t("common.collected"),
    count: entries.value.filter((entry) => entry.item.status === "collected").length,
  },
  {
    value: "all",
    label: t("collectionPage.all"),
    count: entries.value.length,
  },
]);

const categoryOptions = computed<MapLegendOption[]>(() => {
  const counts = new Map<string, number>();
  for (const entry of entries.value) {
    counts.set(entry.group.category, (counts.get(entry.group.category) ?? 0) + 1);
  }
  const list: MapLegendOption[] = [
    { category: "", label: t("common.all-categories"), count: entries.value.length },
  ];
  for (const [category, count] of counts) {
    list.push({ category, label: categoryNameOf(atlas, category), count });
  }
  return list;
});

const calibrationShort = computed(() => {
  const option = currentOption.value;
  if (!option) {
    return "";
  }
  return option.basemap.status === "calibrated" ? t("mapPage.calibrated") : t("mapPage.approximate");
});

const calibrationTitle = computed(() => {
  const option = currentOption.value;
  if (!option) {
    return "";
  }
  const basemap = option.basemap;
  const formatPx = (value: number | null | undefined) =>
    typeof value === "number" && Number.isFinite(value) ? value.toFixed(1) : "—";
  return basemap.status === "calibrated"
    ? tFormat("mapPage.median-error-px-p95-px",  {
        median: formatPx(basemap.median_error_px),
        p95: formatPx(basemap.p95_error_px),
      })
    : tFormat("mapPage.median-error-px",  {
        median: formatPx(basemap.median_error_px),
      });
});

const basemapLabel = (option: MapBasemapOption): string => {
  const status = option.status === "calibrated" ? t("mapPage.calibrated-2") : t("mapPage.approximate-2");
  return tFormat("mapPage.text",  {
    unit: option.unitLabel,
    collected: option.collected,
    total: option.itemCount,
    status,
  });
};

const canvasLabel = computed(() =>
  currentOption.value
    ? `${currentOption.value.regionName} · ${currentOption.value.unitLabel}`
    : t("common.map"),
);

/** 页头主标题：当前区域 · 子关卡；没有可用底图时退回页面名。 */
const mapTitle = computed(() =>
  ready.value && currentOption.value ? canvasLabel.value : t("appSidebar.map"),
);

const unplaced = computed(() =>
  currentOption.value ? unplacedInBasemap(plan.value, currentOption.value.basemap) : 0,
);
const unmapped = computed(() =>
  store.state.mapRegion ? unmappedInRegion(plan.value, store.state.mapRegion) : 0,
);

/** 窄窗的全宽详情浮层：仅当前底图条目可打开。 */
const selectedMapEntry = computed<MapEntry | null>(
  () => entries.value.find((entry) => entry.item.id === store.state.selectedRowId) ?? null,
);

const railMaxWidth = () =>
  Math.max(MAP_RAIL_WIDTH.min, Math.min(MAP_RAIL_WIDTH.max, store.state.layoutWidth - 480));

const railWidth = computed(() => {
  const stored = railDraft.value ?? store.state.mapRailWidth;
  return Math.min(railMaxWidth(), Math.max(MAP_RAIL_WIDTH.min, Math.round(stored)));
});

const overlayRailWidth = computed(() =>
  Math.max(MAP_RAIL_WIDTH.min, Math.min(railWidth.value, store.state.layoutWidth - 24)),
);

const narrowDetailWidth = computed(() =>
  Math.max(MAP_RAIL_WIDTH.min, store.state.layoutWidth - 24),
);

function commitRailWidth(width: number): void {
  railDraft.value = null;
  store.actions.setMapRailWidth(width);
}

function onRegionChange(event: Event): void {
  const slug = (event.target as HTMLSelectElement).value;
  store.actions.setMapRegion(slug);
}

function onBasemapChange(event: Event): void {
  store.actions.setMapBasemap((event.target as HTMLSelectElement).value);
}

function clearMapFilters(): void {
  store.actions.setMapStatus("all");
  store.actions.setMapCategory(null);
  store.actions.setMapSearch("");
}

function openRegionList(): void {
  if (store.state.mapRegion) {
    store.actions.setRegion(store.state.mapRegion);
  }
  store.actions.setView("regions");
}

/** 底图留白区点位变化：只用于图下说明，不影响筛选与计数。 */
function onOffMap(ids: string[]): void {
  offMapIds.value = ids;
}

/** 点击地图图标：选中并自动切到「详情」；窄窗打开全宽详情浮层。 */
function onMarkerSelect(id: string | null): void {
  if (id === null) {
    store.actions.closeDetail();
    return;
  }
  store.actions.openRow(id);
  if (narrow.value) {
    narrowDetailOpen.value = true;
    return;
  }
  store.actions.setMapRailTab("detail");
  if (overlayRail.value) {
    railOverlayOpen.value = true;
  }
}

/** 点击清单项：选中并让地图聚焦，保持在「清单」标签。 */
function onListSelect(id: string): void {
  store.actions.openRow(id);
  canvas.value?.focusItem(id);
}

function openNarrowDetail(id: string): void {
  store.actions.openRow(id);
  narrowDetailOpen.value = true;
}

function onRailTab(tab: "detail" | "list"): void {
  store.actions.setMapRailTab(tab);
}

/** 从区域清单跳转过来时，把选中标记带入可视区并保持右栏详情可见。 */
function focusSelected(): void {
  const id = store.state.selectedRowId;
  if (!id) {
    return;
  }
  void nextTick(() => canvas.value?.focusItem(id));
}

watch(() => store.state.mapFocusToken, focusSelected);

watch(
  () => store.state.mapSearchFocusToken,
  () => searchInput.value?.focus(),
);

onMounted(() => {
  store.actions.ensureMapView();
  focusSelected();
});
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col" aria-labelledby="map-title">
    <header class="shrink-0 border-b border-line/70 pb-2.5" :class="pageX">
      <AppPageHeader title-id="map-title" :title="mapTitle" :context="t('appSidebar.map')">
        <template #actions>
          <template v-if="ready && currentOption">
            <span class="text-[13px] tabular-nums text-ink" data-testid="map-counts">
              {{ tFormatPair(`mapPage.collected-shown`, { v0: currentOption.collected, v1: currentOption.itemCount, v2: filteredEntries.length }) }}
            </span>
            <span
              class="cursor-help text-xs text-muted underline decoration-dotted underline-offset-2"
              :title="calibrationTitle"
              data-testid="map-calibration"
            >{{ calibrationShort }}</span>
          </template>
          <span class="text-xs text-muted">底图 · vi7ual</span>
        </template>

        <div v-if="ready" class="rounded-section bg-raised/60 p-2">
          <div class="flex flex-wrap items-center gap-2">
            <select
              id="map-region-select"
              class="h-9 max-w-[180px] cursor-pointer rounded-control border border-line bg-surface px-2 text-[13px]"
              :aria-label="t('common.region')"
              :value="store.state.mapRegion ?? ''"
              data-testid="map-region-select"
              @change="onRegionChange"
            >
              <option v-for="region in regionChoices" :key="region.slug" :value="region.slug">
                {{ region.name }}
              </option>
            </select>

            <select
              id="map-basemap-select"
              class="h-9 max-w-[280px] cursor-pointer rounded-control border border-line bg-surface px-2 text-[13px]"
              :aria-label="t('common.sub-level-2')"
              :value="store.state.mapBasemapId ?? ''"
              data-testid="map-basemap-select"
              @change="onBasemapChange"
            >
              <option v-for="option in basemapChoices" :key="option.basemapId" :value="option.basemapId">
                {{ basemapLabel(option) }}
              </option>
            </select>

            <div class="w-56 max-w-full">
              <UiSearchInput
                ref="searchInput"
                :model-value="store.state.mapSearch"
                :placeholder="t('mapPage.search-name-internal-name-id-sub-level')"
                :ariaLabel="t('mapPage.search-collectibles-on-the-map')"
                :disabled="!ready"
                @update:model-value="store.actions.setMapSearch($event)"
              />
            </div>
            <UiFilterChips
              variant="segmented"
              :options="statusOptions"
              :model-value="store.state.mapStatus"
              :ariaLabel="t('mapPage.filter-by-collection-status-map')"
              @update:model-value="store.actions.setMapStatus($event as 'missing' | 'observed' | 'all')"
            />
            <UiIconButton
              v-if="filtersActive"
              :label="t('common.clear-filters')"
              data-testid="map-clear-filters"
              @click="clearMapFilters"
            >
              <X :size="15" aria-hidden="true" />
            </UiIconButton>
          </div>
          <div class="mt-2">
            <MapLegend
              :options="categoryOptions"
              :selected="store.state.mapCategory"
              :ariaLabel="t('mapPage.category-legend-and-filter')"
              @select="store.actions.setMapCategory($event)"
            />
          </div>
        </div>

        <CoverageNotice
          v-if="ready && store.isStale.value"
          class="mt-2"
          tone="warning"
          test-id="map-stale"
          :title="copy.stale"
          :lines="[t('common.refresh-before-checking-collection-status-again')]"
        />
      </AppPageHeader>
    </header>

    <div v-if="!ready || !currentOption" class="pt-2" :class="pageX">
      <UiEmptyState v-if="!store.report.value" :title="copy.noReport" :description="copy.noReportHelp">
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("common.open-a-save") }}</UiButton>
      </UiEmptyState>
      <UiEmptyState
        v-else-if="!store.planAvailable.value"
        :title="t('common.profile-files-have-no-collectible-list')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("common.choose-a-save-file") }}</UiButton>
      </UiEmptyState>
      <UiEmptyState
        v-else
        :title="t('mapPage.no-basemap-is-available-for-this-region')"
      >
        <UiButton size="sm" @click="openRegionList">{{ t("mapPage.view-in-regional-list") }}</UiButton>
      </UiEmptyState>
    </div>

    <div v-else class="flex min-h-0 flex-1 flex-col">
      <div class="relative flex min-h-0 flex-1">
        <div class="relative flex min-h-0 flex-1 flex-col">
          <div class="relative flex min-h-0 flex-1">
            <MapCanvas
              ref="canvas"
              :basemap="currentOption.basemap"
              :entries="filteredEntries"
              :selected-id="store.state.selectedRowId"
              :label="canvasLabel"
              @select="onMarkerSelect"
              @off-map="onOffMap"
            >
              <template #error>
                <UiButton size="sm" @click="openRegionList">{{ t("mapPage.view-in-regional-list") }}</UiButton>
              </template>
            </MapCanvas>
            <div
              v-if="entries.length === 0"
              class="pointer-events-none absolute inset-0 flex items-center justify-center p-4"
              data-testid="map-empty-items"
            >
              <p class="rounded-[6px] border border-line bg-surface/95 px-3 py-2 text-[13px]">
                {{ t("mapPage.this-sub-level-has-no-collectibles") }}
              </p>
            </div>
          </div>

          <p
            v-if="offMapIds.length > 0"
            class="shrink-0 border-t border-line bg-surface px-4 py-1.5 text-xs text-muted"
            data-testid="map-off-map-note"
          >
            {{
              tFormatPair(`mapPage.marker-s-fall-in-blank-areas`, { v0: offMapIds.length })
            }}
          </p>

          <details
            v-if="narrow"
            class="ui-disclosure max-h-[45%] shrink-0 overflow-auto border-t border-line bg-surface"
            data-testid="map-narrow-list"
          >
            <summary class="flex min-h-11 cursor-pointer items-center justify-between gap-2 px-4 py-2 text-[13px] font-medium">
              {{ tFormatPair(`mapPage.list`, { v0: entries.length }) }}
              <ChevronDown class="ui-chevron" :size="16" aria-hidden="true" />
            </summary>
            <div class="h-[min(320px,45vh)]">
              <MapRail
                :entries="entries"
                :groups="groups"
                :selected-id="store.state.selectedRowId"
                tab="list"
                :unplaced="unplaced"
                :unmapped="unmapped"
                list-only
                show-row-detail
                @select="onListSelect"
                @open-detail="openNarrowDetail"
                @open-region="openRegionList"
                @clear-filters="clearMapFilters"
              />
            </div>
          </details>
        </div>

        <aside
          v-if="inlineRail"
          class="relative shrink-0 border-l border-line"
          :style="{ width: `${railWidth}px` }"
          :aria-label="t('mapPage.map-details-and-list')"
          data-testid="map-rail-inline"
        >
          <UiResizeHandle
            data-testid="map-rail-resize"
            :width="railWidth"
            :min="MAP_RAIL_WIDTH.min"
            :max="railMaxWidth"
            :default-width="MAP_RAIL_WIDTH.default"
            :label="t('mapPage.resize-the-map-panel-drag-arrow-keys')"
            @resize="railDraft = $event"
            @commit="commitRailWidth"
          />
          <MapRail
            class="h-full"
            :entries="entries"
            :groups="groups"
            :selected-id="store.state.selectedRowId"
            :tab="store.state.mapRailTab"
            :unplaced="unplaced"
            :unmapped="unmapped"
            @update:tab="onRailTab"
            @select="onListSelect"
            @open-region="openRegionList"
            @clear-filters="clearMapFilters"
          />
        </aside>

        <button
          v-if="overlayRail"
          type="button"
          class="ui-interactive absolute top-2 left-2 z-20 inline-flex h-9 cursor-pointer items-center gap-1 rounded-control border border-line bg-surface px-3 text-[13px] shadow-sm hover:bg-raised"
          data-testid="map-open-rail"
          @click="railOverlayOpen = true"
        >
          {{ t("mapPage.details-list") }}
        </button>
      </div>
    </div>

    <UiDrawer
      v-if="overlayRail && railOverlayOpen && currentOption"
      :open="true"
      :title="t('mapPage.details-list')"
      :width="overlayRailWidth"
      flush
      @close="railOverlayOpen = false"
    >
      <MapRail
        class="h-full"
        :entries="entries"
        :groups="groups"
        :selected-id="store.state.selectedRowId"
        :tab="store.state.mapRailTab"
        :unplaced="unplaced"
        :unmapped="unmapped"
        @update:tab="onRailTab"
        @select="onListSelect"
        @open-region="openRegionList"
        @clear-filters="clearMapFilters"
      />
    </UiDrawer>

    <UiDrawer
      v-if="narrow && narrowDetailOpen && selectedMapEntry"
      :open="true"
      :title="selectedMapEntry.item.display_name"
      :width="narrowDetailWidth"
      @close="narrowDetailOpen = false"
    >
      <CollectionDetail
        :region="selectedMapEntry.region"
        :group="selectedMapEntry.group"
        :item="selectedMapEntry.item"
        mode="drawer"
        :show-position="false"
      />
    </UiDrawer>
  </section>
</template>
