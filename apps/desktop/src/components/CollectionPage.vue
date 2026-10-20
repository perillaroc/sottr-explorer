<script setup lang="ts">
// 区域与收集（SX-022）：以收集图鉴为准。
// - “全部区域”视图：每个区域的已收集/总数与进度，点击进入区域；
// - 区域视图：按类别分组的逐项清单（默认只显示未收集），可搜索、筛选、看位置与获取方式；
// - 状态只有两态：已收集 / 未收集（来自存档观察集合，已用存档区域计数器逐行验证）；
// - 特殊区域（跨区域挑战古墓、全局区域、克劳馥庄园）单独成组，不计入主区域合计；
// - 区域进度分母是该区域图鉴条目数，不是游戏内地图百分比。
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useStore } from "../app/inject";
import { t, tFormat, localizedName } from "../app/i18n";
import { copy } from "../app/labels";
import { categoriesOfRegion, categoryNameOf, countRegion, groupDisplayName, regionName } from "../app/collection";
import { categoryClass, categoryDotClass } from "../app/category-style";
import { shortId } from "../app/format";
import type { PlanGroup, PlanRegion } from "../data/atlas";
import CoverageNotice from "./CoverageNotice.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiFilterChips from "./ui/UiFilterChips.vue";
import UiSearchInput from "./ui/UiSearchInput.vue";
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

const regionTitle = computed(() => {
  if (region.value === "all") {
    return t("全部区域", "All regions");
  }
  if (region.value === "unknown") {
    return t("目录外观察记录", "Off-catalog observed records");
  }
  return regionPlan.value ? regionName(regionPlan.value) : region.value;
});

const statusOptions = computed<ChipOption[]>(() => [
  { value: "missing", label: t("未收集", "Uncollected"), count: store.planFilterCounts.value.missing },
  { value: "observed", label: t("已收集", "Collected"), count: store.planFilterCounts.value.observed },
  {
    value: "all",
    label: t("全部", "All"),
    count: store.planFilterCounts.value.all,
    title: t(
      "“全部”指本图鉴收录的收集品，不含服装 / 技能等非收集品记录",
      "“All” means collectibles in this catalog; outfits, skills and other non-collectible records are excluded",
    ),
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

function categoryLabelOf(categoryId: string): string {
  return categoryNameOf(store.atlas, categoryId);
}

/** 分组自己的名字（故事类别按子类别分组）；与类别名相同时不重复成行。 */
function subgroupLabelOf(group: PlanGroup): string | null {
  const name = groupDisplayName(group);
  return name === categoryLabelOf(group.category) ? null : name;
}

function groupPercent(region: PlanRegion): number {
  const entry = countRegion(region);
  return entry.all === 0 ? 0 : Math.round((entry.collected / entry.all) * 100);
}

function openRegion(slug: string): void {
  store.actions.setRegion(slug);
}

function onSelectRow(id: string): void {
  if (store.layout.value === "narrow" && listContainer.value) {
    store.actions.setListScrollTop(listContainer.value.scrollTop);
  }
  store.actions.openRow(id);
}

const activeFilterSummary = computed(() => {
  const parts: string[] = [];
  const statusLabels: Record<string, string> = {
    missing: t("未收集", "Uncollected"),
    observed: t("已收集", "Collected"),
    all: t("全部", "All"),
  };
  parts.push(tFormat("状态：{value}", "Status: {value}", { value: statusLabels[store.state.filters.status] ?? store.state.filters.status }));
  if (store.state.filters.category) {
    parts.push(tFormat("类别：{value}", "Category: {value}", { value: categoryLabelOf(store.state.filters.category) }));
  }
  if (store.state.search) {
    parts.push(tFormat("搜索：{value}", "Search: {value}", { value: store.state.search }));
  }
  return parts.join(" · ");
});

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
    <header class="shrink-0 space-y-2 px-6 pt-5 pb-3">
      <div class="flex flex-wrap items-baseline gap-2">
        <h1 id="collection-title" class="text-xl font-semibold">{{ t("区域与收集", "Regions & Collection") }} · {{ regionTitle }}</h1>
        <span class="text-[13px] text-muted">
          {{ t(`主区域 ${totalCounts.collected}/${totalCounts.all} 已收集 · 特殊区域单列 · 图鉴 ${plan.catalog_version}`, `Main regions ${totalCounts.collected}/${totalCounts.all} collected · special regions listed separately · catalog ${plan.catalog_version}`) }}
        </span>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <label class="text-[13px] text-muted" for="region-select">{{ t("区域", "Region") }}</label>
        <select
          id="region-select"
          class="h-9 rounded-[6px] border border-line bg-surface px-2 text-[13px]"
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
          <option value="unknown">{{ t(`目录外观察记录（${plan.observed_not_in_catalog.length}）`, `Off-catalog observed records (${plan.observed_not_in_catalog.length})`) }}</option>
        </select>
      </div>

      <UiEmptyState v-if="!store.report.value" :title="copy.noReport" :description="copy.noReportHelp">
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
      </UiEmptyState>
      <UiEmptyState
        v-else-if="!store.planAvailable.value"
        :title="t('资料文件没有收集品清单', 'Profile files have no collectible list')"
        :description="t('profile.dat 是资料文件，只做容器观察；请选择 save*.dat 存档查看区域收集状态。', 'profile.dat is a profile file and is only observed as a container; choose a save*.dat file to view regional collection status.')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("选择存档文件", "Choose a save file") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <CoverageNotice
          v-if="store.isStale.value"
          tone="warning"
          test-id="stale-banner"
          :title="copy.stale"
          :lines="[t('旧结果只反映上一次读取；收集状态可能已变化，请刷新后再核对。', 'The previous result reflects only the last read; collection status may have changed — refresh before checking again.')]"
        />
        <CoverageNotice
          tone="info"
          test-id="coverage-banner"
          :title="t('收集状态来自存档物品表', 'Collection status comes from the save inventory table')"
          :lines="[
            t('“已收集”= 该 ID 出现在当前存档的物品表中；区域计数已与存档自带的区域计数器逐行核对。', '“Collected” means the ID appears in the current save inventory table; regional counts were checked line by line against the save’s own region counters.'),
            t('区域进度是图鉴条目数之比，不是游戏内地图完成度百分比。', 'Region progress is a ratio of catalog item counts, not the in-game map completion percentage.'),
          ]"
        />

        <template v-if="region !== 'all' && region !== 'unknown'">
          <div class="flex flex-wrap items-center gap-2">
            <span class="text-[13px] text-ink">
              {{ t(`已收集 ${counts.collected}/${counts.all}`, `Collected ${counts.collected}/${counts.all}`) }}
            </span>
            <span v-if="counts.uncollected > 0" class="rounded-[6px] bg-accent-bg px-2 py-0.5 text-[13px] text-accent-strong">
              {{ t(`未收集 ${counts.uncollected}`, `Uncollected ${counts.uncollected}`) }}
            </span>
            <span v-else class="text-[13px] text-muted">{{ t("本区域已全部收集", "Everything in this region is collected") }}</span>
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <div class="w-64 max-w-full">
              <UiSearchInput
                ref="searchInput"
                :model-value="store.state.search"
                :placeholder="t('搜索名称 / 内部名 / ID', 'Search name / internal name / ID')"
                :ariaLabel="t('搜索名称或 ID', 'Search by name or ID')"
                @update:model-value="store.actions.setSearch($event)"
              />
            </div>
            <UiFilterChips
              :options="statusOptions"
              :model-value="store.state.filters.status"
              :ariaLabel="t('按收集状态筛选', 'Filter by collection status')"
              @update:model-value="store.actions.setStatusFilter($event as 'missing' | 'observed' | 'all')"
            />
            <UiFilterChips
              :options="categoryOptions"
              :model-value="store.state.filters.category ?? ''"
              :ariaLabel="t('按类别筛选', 'Filter by category')"
              @update:model-value="store.actions.setCategory($event === '' ? null : $event)"
            />
            <UiButton size="sm" variant="ghost" @click="store.actions.clearFilters()">{{ t("清除筛选", "Clear filters") }}</UiButton>
            <span class="text-xs text-muted">{{ activeFilterSummary }}</span>
          </div>
        </template>
      </template>
    </header>

    <div ref="listContainer" class="scroll-area min-h-0 flex-1 px-6 pb-6" data-testid="region-list">
      <template v-if="store.report.value && store.planAvailable.value">
        <!-- 全部区域：区域卡片 -->
        <div v-if="region === 'all'" class="space-y-4" data-testid="region-overview">
          <section class="space-y-2">
            <h2 class="text-base font-medium">{{ t("主区域", "Main regions") }}</h2>
            <div class="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-3">
              <button
                v-for="entry in mainRegions"
                :key="entry.slug"
                type="button"
                class="cursor-pointer rounded-[8px] border border-line bg-surface px-3 py-2 text-left hover:bg-raised"
                :data-testid="`region-card-${entry.slug}`"
                @click="openRegion(entry.slug)"
              >
                <div class="flex items-baseline justify-between gap-2">
                  <span class="font-medium text-ink">{{ regionName(entry) }}</span>
                  <span class="tabular-nums text-[13px] text-muted">{{ entry.collected }}/{{ entry.total }}</span>
                </div>
                <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-raised">
                  <div
                    class="h-full rounded-full bg-accent-strong"
                    :style="{ width: `${groupPercent(entry)}%` }"
                  />
                </div>
                <p class="mt-1 text-xs text-muted">
                  <span v-if="entry.total - entry.collected > 0">{{ t(`未收集 ${entry.total - entry.collected} 项`, `${entry.total - entry.collected} uncollected`) }}</span>
                  <span v-else>{{ t("已全部收集", "All collected") }}</span>
                </p>
              </button>
            </div>
          </section>

          <section class="space-y-2">
            <h2 class="text-base font-medium">{{ t("特殊区域", "Special regions") }}</h2>
            <p class="text-xs text-muted">
              {{ t("这些区域没有存档区域计数器，单独成组，不计入主区域合计。", "These regions have no save region counters; they are listed separately and excluded from the main-region total.") }}
            </p>
            <div class="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-3">
              <button
                v-for="entry in specialRegions"
                :key="entry.slug"
                type="button"
                class="cursor-pointer rounded-[8px] border border-line bg-surface px-3 py-2 text-left hover:bg-raised"
                :data-testid="`region-card-${entry.slug}`"
                @click="openRegion(entry.slug)"
              >
                <div class="flex items-baseline justify-between gap-2">
                  <span class="font-medium text-ink">{{ regionName(entry) }}</span>
                  <span class="tabular-nums text-[13px] text-muted">{{ entry.collected }}/{{ entry.total }}</span>
                </div>
                <p v-if="entry.note" class="mt-1 text-xs text-muted">{{ entry.note }}</p>
              </button>
            </div>
          </section>

          <p v-if="plan.observed_not_in_catalog.length > 0" class="text-xs text-muted">
            {{ t(`存档里另有 ${plan.observed_not_in_catalog.length} 个不属于收集品的观察 ID（服装 / 技能 / 装备等），见「诊断」；它们不进入任何区域计数。`, `The save also contains ${plan.observed_not_in_catalog.length} observed IDs that are not collectibles (outfits, skills, gear, etc.); see Diagnostics. They are not counted in any region.`) }}
          </p>
        </div>

        <!-- 目录外观察记录 -->
        <div v-else-if="region === 'unknown'" class="space-y-2" data-testid="unknown-records">
          <p class="text-[13px] text-muted">
            {{ t("以下 ID 出现在存档中，但不属于全收集图鉴（多为服装、技能与装备）。按原始身份列出，不推断收集状态。", "The IDs below appear in the save but are not part of the full-collection catalog (mostly outfits, skills and gear). They are listed by raw identity; collection status is not inferred.") }}
          </p>
          <p v-if="plan.observed_not_in_catalog.length === 0" class="text-[13px] text-muted">{{ t("没有目录外观察记录。", "No off-catalog observed records.") }}</p>
          <ul v-else class="grid grid-cols-1 gap-1 md:grid-cols-2 xl:grid-cols-3">
            <li v-for="id in plan.observed_not_in_catalog" :key="id" class="mono text-[12px] text-muted">
              {{ id }}
            </li>
          </ul>
        </div>

        <!-- 区域清单 -->
        <template v-else>
          <div v-if="groups.length === 0">
            <UiEmptyState :title="emptyTitle" :description="tFormat('当前范围：{value}', 'Current scope: {value}', { value: `${regionTitle} · ${activeFilterSummary}` })">
              <UiButton v-if="store.state.search" size="sm" @click="store.actions.setSearch('')">{{ t("清空搜索", "Clear search") }}</UiButton>
              <UiButton size="sm" @click="store.actions.clearFilters()">{{ t("清除筛选", "Clear filters") }}</UiButton>
            </UiEmptyState>
          </div>

          <section v-for="group in groups" :key="`${group.category}-${group.subcategory ?? ''}`" class="mb-4">
            <!-- 分类单独成行：第一行是着色类别名 + 计数 + 口径，第二行才是该组的子类别。 -->
            <header class="mb-1 space-y-0.5">
              <div class="flex flex-wrap items-baseline gap-2">
                <h2 class="text-[15px] font-medium" :class="categoryClass(group.category)">
                  {{ categoryLabelOf(group.category) }}
                </h2>
                <span class="text-xs text-muted">{{ group.collected }}/{{ group.total }}</span>
                <UiBadge v-if="group.required_for_100 === null" tone="neutral" :kind-label="t('口径', 'Scope')">{{ t("未闭合", "Unresolved") }}</UiBadge>
                <UiBadge v-else-if="group.required_for_100 === false" tone="neutral" :kind-label="t('口径', 'Scope')">{{ t("可选", "Optional") }}</UiBadge>
              </div>
              <p v-if="subgroupLabelOf(group)" class="text-xs text-muted" data-testid="group-subcategory">
                {{ tFormat("子类别：{value}", "Subcategory: {value}", { value: subgroupLabelOf(group) ?? "" }) }}
              </p>
            </header>
            <ul class="divide-y divide-line/70">
              <li v-for="item in group.items" :key="item.id">
                <button
                  type="button"
                  class="flex w-full flex-wrap items-start gap-2 rounded-[6px] px-2 py-1.5 text-left hover:bg-raised"
                  :class="store.state.selectedRowId === item.id ? 'bg-accent-bg' : ''"
                  :data-testid="`row-${item.id}`"
                  @click="onSelectRow(item.id)"
                >
                  <span class="min-w-0 flex-1">
                    <span class="line-clamp-2 block break-words">{{ item.display_name }}</span>
                    <span class="mt-0.5 block text-xs text-muted">
                      <span
                        class="cat-dot mr-1.5 align-middle"
                        :class="categoryDotClass(group.category)"
                        :title="categoryLabelOf(group.category)"
                        aria-hidden="true"
                      ></span>
                      <template v-if="item.unit">
                        {{ localizedName(item.unit.display_zh, item.unit.display_en, item.unit.zone ?? "") }}
                        ·
                      </template>
                      <span class="mono">{{ shortId(item.id) }}</span>
                      <template v-if="item.is_dlc"> · DLC</template>
                    </span>
                  </span>
                  <span class="ml-auto flex shrink-0 flex-wrap items-center gap-1">
                    <UiBadge
                      :tone="item.status === 'collected' ? 'success' : 'caution'"
                      :kind-label="t('状态', 'Status')"
                    >
                      {{ item.status === "collected" ? t("已收集", "Collected") : t("未收集", "Uncollected") }}
                    </UiBadge>
                    <UiBadge v-if="item.position" tone="info" :kind-label="t('位置', 'Location')">{{ t("相对位置", "Relative position") }}</UiBadge>
                  </span>
                </button>
              </li>
            </ul>
          </section>
        </template>
      </template>
    </div>
  </section>
</template>
