<script setup lang="ts">
// 区域与收集（SX-022；2026-11-05 界面美化阶段四）：以收集图鉴为准。
// - “全部区域”视图：每个区域的已收集/总数与进度，点击进入区域；
// - 区域视图：按类别分组的逐项清单（默认只显示未收集），可搜索、筛选、看位置与获取方式；
// - 有底图的区域/条目可直接跳到地图查询并定位；
// - 状态只有两态：已收集 / 未收集（来自存档观察集合，已用存档区域计数器逐行验证）；
// - 特殊区域（跨区域挑战古墓、全局区域、克劳馥庄园）单独成组，不计入主区域合计。
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { MapPin, X } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat, localizedName } from "../app/i18n";
import { copy } from "../app/labels";
import { categoriesOfRegion, categoryNameOf, groupDisplayName, regionName } from "../app/collection";
import { categoryIcon } from "../app/category-icon";
import { categoryClass, categoryDotClass } from "../app/category-style";
import { pagePaddingClass, regionGridClass } from "../app/layout";
import { regionHasBasemap } from "../app/map-view";
import { shortId } from "../app/format";
import type { PlanGroup, PlanItem, PlanRegion } from "../data/atlas";
import AppPageHeader from "./AppPageHeader.vue";
import CoverageNotice from "./CoverageNotice.vue";
import RegionProgressRow from "./RegionProgressRow.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiFilterChips from "./ui/UiFilterChips.vue";
import UiIconButton from "./ui/UiIconButton.vue";
import UiSearchInput from "./ui/UiSearchInput.vue";
import UiStatusMark from "./ui/UiStatusMark.vue";
import type { ChipOption } from "./ui/types";

const store = useStore();
const searchInput = ref<InstanceType<typeof UiSearchInput> | null>(null);
const listContainer = ref<HTMLElement | null>(null);

const plan = computed(() => store.plan.value);
const region = computed(() => store.state.region);
const regionPlan = computed<PlanRegion | null>(() => store.planRegion.value);
const groups = computed<PlanGroup[]>(() => store.planGroups.value);
const counts = computed(() => store.planCountsForRegion.value);
const totalCounts = computed(() => store.planMainCounts.value);

const mainRegions = computed(() => plan.value.regions.filter((entry) => entry.kind === "main"));
const specialRegions = computed(() => plan.value.regions.filter((entry) => entry.kind === "special"));
const regionGrid = computed(() => regionGridClass(store.layout.value));
const pageX = computed(() => pagePaddingClass(store.layout.value));

const isRegionView = computed(() => region.value !== "all" && region.value !== "unknown");
const regionMapAvailable = computed(
  () => isRegionView.value && regionHasBasemap(plan.value, region.value),
);

const regionTitle = computed(() => {
  if (region.value === "all") {
    return t("全部区域", "All regions");
  }
  if (region.value === "unknown") {
    return t("目录外记录", "Off-catalog records");
  }
  return regionPlan.value ? regionName(regionPlan.value) : region.value;
});

const summaryText = computed(() => {
  if (region.value === "all") {
    return t(
      `主区域 ${totalCounts.value.collected}/${totalCounts.value.all} 已收集`,
      `Main regions ${totalCounts.value.collected}/${totalCounts.value.all} collected`,
    );
  }
  if (region.value === "unknown") {
    return t(
      `目录外记录 ${plan.value.observed_not_in_catalog.length} 项`,
      `${plan.value.observed_not_in_catalog.length} off-catalog records`,
    );
  }
  if (!isRegionView.value) {
    return "";
  }
  return t(
    `${counts.value.collected}/${counts.value.all} · 未收集 ${counts.value.uncollected} 项`,
    `${counts.value.collected}/${counts.value.all} · ${counts.value.uncollected} uncollected`,
  );
});

const statusOptions = computed<ChipOption[]>(() => [
  { value: "missing", label: t("未收集", "Uncollected"), count: store.planFilterCounts.value.missing },
  { value: "observed", label: t("已收集", "Collected"), count: store.planFilterCounts.value.observed },
  {
    value: "all",
    label: t("全部", "All"),
    count: store.planFilterCounts.value.all,
  },
]);

const categoryOptions = computed<ChipOption[]>(() => {
  const options: ChipOption[] = [{ value: "", label: t("全部类别", "All categories") }];
  if (regionPlan.value) {
    for (const category of categoriesOfRegion(regionPlan.value)) {
      options.push({ value: category.id, label: categoryLabelOf(category.id), count: category.count });
    }
  }
  return options;
});

const filtersActive = computed(
  () =>
    store.state.search.trim().length > 0 ||
    store.state.filters.category !== null ||
    store.state.filters.status !== "missing",
);

function categoryLabelOf(categoryId: string): string {
  return categoryNameOf(store.atlas, categoryId);
}

/** 分组自己的名字（故事类别按子类别分组）；与类别名相同时不重复成行。 */
function subgroupLabelOf(group: PlanGroup): string | null {
  const name = groupDisplayName(group);
  return name === categoryLabelOf(group.category) ? null : name;
}

function openRegion(slug: string): void {
  store.actions.setRegion(slug);
}

function openRegionMap(slug: string): void {
  store.actions.openRegionInMap(slug);
}

/** 条目能否跳地图：所在子关卡有底图。 */
function canOpenMap(item: PlanItem): boolean {
  return store.mapTargetForItem(item) !== null;
}

function openItemMap(item: PlanItem): void {
  store.actions.openInMap(item.id);
}

function onSelectRow(id: string): void {
  if (store.layout.value === "narrow" && listContainer.value) {
    store.actions.setListScrollTop(listContainer.value.scrollTop);
  }
  store.actions.openRow(id);
}

function isSelected(id: string): boolean {
  return store.state.selectedRowId === id;
}

/** 图鉴状态只有 collected / not_collected；转化为状态图形使用的三态。 */
function statusMarkOf(item: PlanItem): "collected" | "uncollected" | "unknown" {
  if (item.status === "collected") {
    return "collected";
  }
  if (item.status === "not_collected") {
    return "uncollected";
  }
  return "unknown";
}

function rowClass(id: string): string {
  return isSelected(id) ? "ui-row-selected" : "hover:bg-raised";
}

function nameClass(item: PlanItem): string {
  if (isSelected(item.id)) {
    return "font-medium text-ink";
  }
  return item.status === "collected" && store.state.filters.status === "all" ? "text-muted" : "text-ink";
}

function unitLabelOf(item: PlanItem): string | null {
  const unit = item.unit;
  if (!unit) {
    return null;
  }
  return localizedName(unit.display_zh, unit.display_en, unit.zone ?? "");
}

const emptyTitle = computed(() => {
  if (store.state.search) {
    return t("没有匹配的收集品", "No matching collectibles");
  }
  if (store.state.filters.status === "missing") {
    return t("本区域已全部收集", "Everything in this region is collected");
  }
  return t("当前筛选没有条目", "No items match the current filters");
});

onMounted(() => {
  void nextTick(() => {
    if (listContainer.value) {
      listContainer.value.scrollTop = store.state.listScrollTop;
    }
  });
});

watch(
  () => store.state.searchFocusToken,
  () => {
    searchInput.value?.focus();
  },
);
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col" aria-labelledby="collection-title">
    <header class="shrink-0 border-b border-line/70 pb-3" :class="pageX">
      <div class="mx-auto w-full max-w-[1120px]">
        <AppPageHeader
          title-id="collection-title"
          :title="regionTitle"
          :context="t('区域与收集', 'Regions & Collection')"
          :description="summaryText"
        >
          <template #actions>
            <UiButton
              v-if="regionMapAvailable"
              size="sm"
              data-testid="region-map-open"
              :title="t('在地图查询中查看该区域', 'View this region on the map')"
              @click="openRegionMap(region)"
            >
              <MapPin :size="15" aria-hidden="true" />
              {{ t("地图", "Map") }}
            </UiButton>
          </template>

          <div class="rounded-section bg-raised/60 p-2">
            <div class="flex flex-wrap items-center gap-2">
              <select
                id="region-select"
                class="h-9 max-w-[220px] cursor-pointer rounded-control border border-line bg-surface px-2 text-[13px]"
                :aria-label="t('区域', 'Region')"
                :value="region"
                data-testid="region-select"
                @change="openRegion(($event.target as HTMLSelectElement).value)"
              >
                <option value="all">{{ t("全部区域", "All regions") }}</option>
                <optgroup :label="t('主区域', 'Main regions')">
                  <option v-for="entry in mainRegions" :key="entry.slug" :value="entry.slug">
                    {{ regionName(entry) }} ({{ entry.collected }}/{{ entry.total }})
                  </option>
                </optgroup>
                <optgroup :label="t('特殊区域', 'Special regions')">
                  <option v-for="entry in specialRegions" :key="entry.slug" :value="entry.slug">
                    {{ regionName(entry) }} ({{ entry.collected }}/{{ entry.total }})
                  </option>
                </optgroup>
                <option value="unknown">{{ t(`目录外记录（${plan.observed_not_in_catalog.length}）`, `Off-catalog (${plan.observed_not_in_catalog.length})`) }}</option>
              </select>

              <template v-if="isRegionView">
                <div class="w-60 max-w-full">
                  <UiSearchInput
                    ref="searchInput"
                    :model-value="store.state.search"
                    :placeholder="t('搜索名称 / 内部名 / ID', 'Search name / internal name / ID')"
                    :ariaLabel="t('搜索名称或 ID', 'Search by name or ID')"
                    @update:model-value="store.actions.setSearch($event)"
                  />
                </div>
                <UiFilterChips
                  variant="segmented"
                  :options="statusOptions"
                  :model-value="store.state.filters.status"
                  :ariaLabel="t('按收集状态筛选', 'Filter by collection status')"
                  @update:model-value="store.actions.setStatusFilter($event as 'missing' | 'observed' | 'all')"
                />
              </template>
            </div>

            <div v-if="isRegionView" class="mt-2 flex flex-wrap items-center gap-1">
              <UiFilterChips
                :options="categoryOptions"
                :model-value="store.state.filters.category ?? ''"
                :ariaLabel="t('按类别筛选', 'Filter by category')"
                @update:model-value="store.actions.setCategory($event === '' ? null : $event)"
              />
              <UiIconButton
                v-if="filtersActive"
                :label="t('清除筛选', 'Clear filters')"
                data-testid="clear-filters"
                @click="store.actions.clearFilters()"
              >
                <X :size="15" aria-hidden="true" />
              </UiIconButton>
            </div>
          </div>
        </AppPageHeader>
      </div>
    </header>

    <div ref="listContainer" class="scroll-area min-h-0 flex-1" data-testid="region-list">
      <div class="mx-auto w-full max-w-[1120px] pb-8 pt-2" :class="pageX">
        <UiEmptyState v-if="!store.report.value" :title="copy.noReport" :description="copy.noReportHelp">
          <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
        </UiEmptyState>

        <UiEmptyState
          v-else-if="!store.planAvailable.value"
          :title="t('资料文件没有收集品清单', 'Profile files have no collectible list')"
        >
          <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("选择存档文件", "Choose a save file") }}</UiButton>
        </UiEmptyState>

        <template v-else>
          <CoverageNotice
            v-if="store.isStale.value"
            class="mb-3 mt-2"
            tone="warning"
            test-id="stale-banner"
            :title="copy.stale"
            :lines="[t('请刷新后再核对收集状态。', 'Refresh before checking collection status again.')]"
          />

          <!-- 全部区域：区域入口 -->
          <div v-if="region === 'all'" class="space-y-6 pt-2" data-testid="region-overview">
            <section class="space-y-2">
              <h2 class="ui-section-title">{{ t("主区域", "Main regions") }}</h2>
              <div class="grid gap-3" :class="regionGrid">
                <RegionProgressRow
                  v-for="entry in mainRegions"
                  :key="entry.slug"
                  :slug="entry.slug"
                  :name="regionName(entry)"
                  :collected="entry.collected"
                  :total="entry.total"
                  :test-id="`region-card-${entry.slug}`"
                  :map-available="regionHasBasemap(plan, entry.slug)"
                  :map-test-id="`region-map-${entry.slug}`"
                  @open="openRegion"
                  @open-map="openRegionMap"
                />
              </div>
            </section>

            <section class="space-y-2">
              <h2 class="ui-section-title">{{ t("特殊区域", "Special regions") }}</h2>
              <div class="grid gap-3" :class="regionGrid">
                <RegionProgressRow
                  v-for="entry in specialRegions"
                  :key="entry.slug"
                  :slug="entry.slug"
                  :name="regionName(entry)"
                  :collected="entry.collected"
                  :total="entry.total"
                  :test-id="`region-card-${entry.slug}`"
                  :map-available="regionHasBasemap(plan, entry.slug)"
                  :map-test-id="`region-map-${entry.slug}`"
                  @open="openRegion"
                  @open-map="openRegionMap"
                />
              </div>
            </section>
          </div>

          <!-- 目录外观察记录 -->
          <div v-else-if="region === 'unknown'" class="space-y-2 pt-4" data-testid="unknown-records">
            <p v-if="plan.observed_not_in_catalog.length === 0" class="text-[13px] text-muted">{{ t("没有目录外观察记录。", "No off-catalog observed records.") }}</p>
            <ul v-else class="grid grid-cols-1 gap-1 md:grid-cols-2">
              <li v-for="id in plan.observed_not_in_catalog" :key="id" class="mono text-[12px] text-muted">
                {{ id }}
              </li>
            </ul>
          </div>

          <!-- 区域清单 -->
          <div v-else class="pb-8">
            <div v-if="groups.length === 0" class="pt-4">
              <UiEmptyState :title="emptyTitle">
                <UiButton v-if="store.state.search" size="sm" @click="store.actions.setSearch('')">{{ t("清空搜索", "Clear search") }}</UiButton>
                <UiButton size="sm" @click="store.actions.clearFilters()">{{ t("清除筛选", "Clear filters") }}</UiButton>
              </UiEmptyState>
            </div>

            <section v-for="group in groups" :key="`${group.category}-${group.subcategory ?? ''}`">
              <header
                class="sticky top-0 z-10 flex flex-wrap items-baseline gap-x-2 gap-y-0.5 border-b border-line bg-surface py-2"
                data-testid="group-header"
              >
                <component :is="categoryIcon(group.category)" :size="16" class="shrink-0" :class="categoryClass(group.category)" aria-hidden="true" />
                <h2 class="text-[15px] font-semibold" :class="categoryClass(group.category)">
                  {{ categoryLabelOf(group.category) }}
                </h2>
                <span class="text-xs tabular-nums text-muted">{{ group.collected }}/{{ group.total }}</span>
                <span v-if="group.required_for_100 === false" class="text-xs text-muted">· {{ t("可选", "Optional") }}</span>
                <span v-else-if="group.required_for_100 === null" class="text-xs text-muted">· {{ t("未闭合", "Unresolved") }}</span>
                <span v-if="subgroupLabelOf(group)" class="text-xs text-muted" data-testid="group-subcategory">
                  · {{ subgroupLabelOf(group) }}
                </span>
              </header>
              <ul>
                <li
                  v-for="item in group.items"
                  :key="item.id"
                  class="flex items-stretch border-b border-line/50 transition-colors last:border-b-0"
                  :class="rowClass(item.id)"
                >
                  <button
                    type="button"
                    class="flex min-h-14 min-w-0 flex-1 cursor-pointer items-start gap-3 px-3 py-2 text-left"
                    :data-testid="`row-${item.id}`"
                    @click="onSelectRow(item.id)"
                  >
                    <UiStatusMark :status="statusMarkOf(item)" decorative class="mt-1" />
                    <span class="min-w-0 flex-1">
                      <span
                        class="line-clamp-2 block break-words text-[14px] leading-5"
                        :class="nameClass(item)"
                      >{{ item.display_name }}</span>
                      <span class="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-xs text-muted">
                        <span
                          class="cat-dot"
                          :class="categoryDotClass(group.category)"
                          :title="categoryLabelOf(group.category)"
                          aria-hidden="true"
                        ></span>
                        <template v-if="unitLabelOf(item)">
                          <span class="min-w-0 break-words">{{ unitLabelOf(item) }}</span>
                          <span aria-hidden="true">·</span>
                        </template>
                        <span class="mono">{{ shortId(item.id) }}</span>
                        <template v-if="item.is_dlc">
                          <span aria-hidden="true">·</span>
                          <span>DLC</span>
                        </template>
                      </span>
                    </span>
                    <span
                      class="shrink-0 self-center text-xs whitespace-nowrap"
                      :class="item.status === 'collected' ? 'text-success' : 'text-caution'"
                    >
                      {{ item.status === "collected" ? t("已收集", "Collected") : t("未收集", "Uncollected") }}
                    </span>
                  </button>
                  <UiIconButton
                    v-if="canOpenMap(item)"
                    :label="tFormat('在地图中查看：{name}', 'View on the map: {name}', { name: item.display_name })"
                    :data-testid="`item-map-${item.id}`"
                    class="my-2 mr-2 self-center text-muted hover:text-accent-strong"
                    @click="openItemMap(item)"
                  >
                    <MapPin :size="15" aria-hidden="true" />
                  </UiIconButton>
                </li>
              </ul>
            </section>
          </div>
        </template>
      </div>
    </div>
  </section>
</template>
