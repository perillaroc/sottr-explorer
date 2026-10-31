<script setup lang="ts">
// 地图查询视图（2026-10-31 地图查询与详情增强·阶段二）：
// 以底图为主，按类别使用不同图标绘制收集品；右侧「详情 / 清单」标签页联动。
//
// 边界（方案 §2.1）：一张底图 = 一个子关卡，不做跨子关卡拼图；点位只是同子关卡内的
// 相对位置参考，不是导航；筛选独立于区域清单，`mapRegion` 不回写 `state.region`。
import { computed, onMounted, ref, watch } from "vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
import { copy } from "../app/labels";
import { categoryNameOf } from "../app/collection";
import { MAP_RAIL_WIDTH } from "../app/prefs";
import {
  defaultMapBasemap,
  filterMapEntries,
  mapCoverage,
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
import MapCanvas from "./map/MapCanvas.vue";
import MapLegend, { type MapLegendOption } from "./map/MapLegend.vue";
import MapRail from "./map/MapRail.vue";
import CoverageNotice from "./CoverageNotice.vue";
import UiButton from "./ui/UiButton.vue";
import UiDrawer from "./ui/UiDrawer.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiFilterChips from "./ui/UiFilterChips.vue";
import UiResizeHandle from "./ui/UiResizeHandle.vue";
import UiSearchInput from "./ui/UiSearchInput.vue";
import type { ChipOption } from "./ui/types";

const store = useStore();

const canvas = ref<InstanceType<typeof MapCanvas> | null>(null);
const searchInput = ref<InstanceType<typeof UiSearchInput> | null>(null);
const railOverlayOpen = ref(false);
const narrowDetailOpen = ref(false);
const railDraft = ref<number | null>(null);

const plan = computed(() => store.plan.value);
const atlas = store.atlas;
const layout = computed(() => store.layout.value);
const ready = computed(() => store.report.value !== null && store.planAvailable.value);

const inlineRail = computed(() => layout.value === "wide" || layout.value === "medium");
const overlayRail = computed(() => layout.value === "compact");
const narrow = computed(() => layout.value === "narrow");

const options = computed<MapBasemapOption[]>(() => mapOptions(plan.value, atlas));
const coverage = computed(() => mapCoverage(plan.value, atlas));

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

const statusOptions = computed<ChipOption[]>(() => [
  {
    value: "all",
    label: t("全部", "All"),
    count: entries.value.length,
  },
  {
    value: "missing",
    label: t("未收集", "Uncollected"),
    count: entries.value.filter((entry) => entry.item.status === "not_collected").length,
  },
  {
    value: "observed",
    label: t("已收集", "Collected"),
    count: entries.value.filter((entry) => entry.item.status === "collected").length,
  },
]);

const categoryOptions = computed<MapLegendOption[]>(() => {
  const counts = new Map<string, number>();
  for (const entry of entries.value) {
    counts.set(entry.group.category, (counts.get(entry.group.category) ?? 0) + 1);
  }
  const list: MapLegendOption[] = [
    { category: "", label: t("全部类别", "All categories"), count: entries.value.length },
  ];
  for (const [category, count] of counts) {
    list.push({ category, label: categoryNameOf(atlas, category), count });
  }
  return list;
});

const coverageText = computed(() =>
  tFormat(
    "底图 {basemaps} 张 · {unitsWithBasemap}/{units} 个子关卡 · {mappable}/{items} 条可上图",
    "{basemaps} basemaps · {unitsWithBasemap}/{units} sub-levels · {mappable}/{items} items mappable",
    {
      basemaps: coverage.value.basemaps,
      unitsWithBasemap: coverage.value.unitsWithBasemap,
      units: coverage.value.units,
      mappable: coverage.value.mappable,
      items: coverage.value.items,
    },
  ),
);

const calibrationText = computed(() => {
  const option = currentOption.value;
  if (!option) {
    return "";
  }
  const basemap = option.basemap;
  const formatPx = (value: number | null | undefined) =>
    typeof value === "number" && Number.isFinite(value) ? value.toFixed(1) : "—";
  // 只有 status = calibrated 才允许出现「已校准」字样。
  if (basemap.status === "calibrated") {
    return tFormat("已校准 · 中位误差 {median} px，P95 {p95} px", "Calibrated · median {median} px, P95 {p95} px", {
      median: formatPx(basemap.median_error_px),
      p95: formatPx(basemap.p95_error_px),
    });
  }
  return tFormat(
    "近似位置 · 中位误差 {median} px{estimated}",
    "Approximate · median {median} px{estimated}",
    {
      median: formatPx(basemap.median_error_px),
      estimated: basemap.error_source === "global_estimate" ? t("（全局估计）", " (global estimate)") : "",
    },
  );
});

const basemapLabel = (option: MapBasemapOption): string => {
  const status = option.status === "calibrated" ? t("已校准", "calibrated") : t("近似", "approximate");
  return tFormat("{unit} · {collected}/{total} · {status}", "{unit} · {collected}/{total} · {status}", {
    unit: option.unitLabel,
    collected: option.collected,
    total: option.itemCount,
    status,
  });
};

const canvasLabel = computed(() =>
  currentOption.value
    ? `${currentOption.value.regionName} · ${currentOption.value.unitLabel}`
    : t("地图", "Map"),
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

watch(
  () => store.state.mapSearchFocusToken,
  () => searchInput.value?.focus(),
);

onMounted(() => store.actions.ensureMapView());
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col" aria-labelledby="map-title">
    <header class="shrink-0 space-y-2 px-4 pt-4 pb-2">
      <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h1 id="map-title" class="text-xl font-semibold">{{ t("地图查询", "Map") }}</h1>
        <template v-if="ready && currentOption">
          <span class="text-[13px] text-ink" data-testid="map-counts">
            {{ t(`已收集 ${currentOption.collected}/${currentOption.itemCount} · 当前筛选 ${filteredEntries.length} 项`, `Collected ${currentOption.collected}/${currentOption.itemCount} · ${filteredEntries.length} shown`) }}
          </span>
          <span class="text-xs text-muted" data-testid="map-calibration">{{ calibrationText }}</span>
        </template>
        <span class="text-xs text-muted" data-testid="map-coverage">{{ coverageText }}</span>
      </div>

      <template v-if="ready">
        <div class="flex flex-wrap items-center gap-2">
          <label class="text-[13px] text-muted" for="map-region-select">{{ t("区域", "Region") }}</label>
          <select
            id="map-region-select"
            class="h-9 max-w-[220px] rounded-[6px] border border-line bg-surface px-2 text-[13px]"
            :value="store.state.mapRegion ?? ''"
            data-testid="map-region-select"
            @change="onRegionChange"
          >
            <option v-for="region in regionChoices" :key="region.slug" :value="region.slug">
              {{ region.name }}
            </option>
          </select>

          <label class="text-[13px] text-muted" for="map-basemap-select">{{ t("子关卡", "Sub-level") }}</label>
          <select
            id="map-basemap-select"
            class="h-9 max-w-[320px] rounded-[6px] border border-line bg-surface px-2 text-[13px]"
            :value="store.state.mapBasemapId ?? ''"
            data-testid="map-basemap-select"
            @change="onBasemapChange"
          >
            <option v-for="option in basemapChoices" :key="option.basemapId" :value="option.basemapId">
              {{ basemapLabel(option) }}
            </option>
          </select>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <div class="w-56 max-w-full">
            <UiSearchInput
              ref="searchInput"
              :model-value="store.state.mapSearch"
              :placeholder="t('搜索名称 / 内部名 / ID / 子关卡', 'Search name / internal name / ID / sub-level')"
              :ariaLabel="t('搜索地图上的收集品', 'Search collectibles on the map')"
              :disabled="!ready"
              @update:model-value="store.actions.setMapSearch($event)"
            />
          </div>
          <UiFilterChips
            :options="statusOptions"
            :model-value="store.state.mapStatus"
            :ariaLabel="t('按收集状态筛选（地图）', 'Filter by collection status (map)')"
            @update:model-value="store.actions.setMapStatus($event as 'missing' | 'observed' | 'all')"
          />
          <UiButton size="sm" variant="ghost" data-testid="map-clear-filters" @click="clearMapFilters">
            {{ t("清除筛选", "Clear filters") }}
          </UiButton>
        </div>
        <MapLegend
          :options="categoryOptions"
          :selected="store.state.mapCategory"
          :ariaLabel="t('类别图例与筛选', 'Category legend and filter')"
          @select="store.actions.setMapCategory($event)"
        />
        <CoverageNotice
          v-if="store.isStale.value"
          tone="warning"
          test-id="map-stale"
          :title="copy.stale"
          :lines="[t('旧结果只反映上一次读取；收集状态可能已变化，请刷新后再核对。', 'The previous result reflects only the last read; collection status may have changed — refresh before checking again.')]"
        />
      </template>
    </header>

    <UiEmptyState
      v-if="!store.report.value"
      class="mx-4"
      :title="copy.noReport"
      :description="copy.noReportHelp"
    >
      <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
    </UiEmptyState>
    <UiEmptyState
      v-else-if="!store.planAvailable.value"
      class="mx-4"
      :title="t('资料文件没有收集品清单', 'Profile files have no collectible list')"
      :description="t('profile.dat 只做容器观察；请选择 save*.dat 存档查看地图上的收集品。', 'profile.dat is only observed as a container; choose a save*.dat file to view collectibles on the map.')"
    >
      <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("选择存档文件", "Choose a save file") }}</UiButton>
    </UiEmptyState>
    <UiEmptyState
      v-else-if="!currentOption"
      class="mx-4"
      :title="t('该区域暂无可用的地图底图', 'No basemap is available for this region')"
      :description="t('特殊区域与无底图子关卡不在地图中；请在区域清单查看这些条目。', 'Special regions and sub-levels without basemaps are not in the map; see the regional list instead.')"
    >
      <UiButton size="sm" @click="openRegionList">{{ t("在区域清单中查看", "View in regional list") }}</UiButton>
    </UiEmptyState>

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
            >
              <template #error>
                <UiButton size="sm" @click="openRegionList">{{ t("在区域清单中查看", "View in regional list") }}</UiButton>
              </template>
            </MapCanvas>
            <div
              v-if="entries.length === 0"
              class="pointer-events-none absolute inset-0 flex items-center justify-center p-4"
              data-testid="map-empty-items"
            >
              <p class="rounded-[6px] border border-line bg-surface/95 px-3 py-2 text-[13px]">
                {{ t("本子关卡没有带坐标的收集品。", "This sub-level has no collectibles with coordinates.") }}
              </p>
            </div>
          </div>

          <details
            v-if="narrow"
            class="max-h-[45%] shrink-0 overflow-auto border-t border-line"
            data-testid="map-narrow-list"
          >
            <summary class="cursor-pointer px-4 py-2 text-[13px]">
              {{ t(`清单 ${entries.length}`, `List ${entries.length}`) }}
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
          :aria-label="t('地图详情与清单', 'Map details and list')"
          data-testid="map-rail-inline"
        >
          <UiResizeHandle
            data-testid="map-rail-resize"
            :width="railWidth"
            :min="MAP_RAIL_WIDTH.min"
            :max="railMaxWidth"
            :default-width="MAP_RAIL_WIDTH.default"
            :label="t('调整右栏宽度（拖动、方向键或双击复位）', 'Resize the map panel (drag, arrow keys, or double-click to reset)')"
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
          class="absolute top-2 left-2 z-20 inline-flex h-9 cursor-pointer items-center gap-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] shadow hover:bg-raised"
          data-testid="map-open-rail"
          @click="railOverlayOpen = true"
        >
          {{ t("详情与清单", "Details & list") }}
        </button>
      </div>

      <footer class="shrink-0 border-t border-line px-4 py-1 text-xs text-muted" data-testid="map-footer">
        {{ t("底图来源：vi7ual（个人非商业使用）", "Basemaps: vi7ual (personal, non-commercial use)") }}
        ·
        {{ t("特殊区域与无底图子关卡不在地图中，请在区域清单查看。", "Special regions and sub-levels without basemaps are not in the map; see the regional list.") }}
      </footer>
    </div>

    <UiDrawer
      v-if="overlayRail && railOverlayOpen && currentOption"
      :open="true"
      :title="t('详情与清单', 'Details & list')"
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
