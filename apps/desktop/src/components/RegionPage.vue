<script setup lang="ts">
// 区域与收集（SX-015）：区域标题与计数 → 覆盖/降级提示 → 搜索与筛选 → 结果清单。
// 筛选计数来自当前报告与明确范围；“全部”仅指本页支持的收集品集合。
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useStore } from "../app/inject";
import { copy, regionLabel } from "../app/labels";
import { missingByRegion, PRECISION_OPTIONS, EVIDENCE_OPTIONS, type StatusFilter } from "../app/catalog";
import { regionsOf } from "../data/metadata";
import CollectibleList from "./CollectibleList.vue";
import CoverageNotice from "./CoverageNotice.vue";
import RegionCounts from "./RegionCounts.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiFilterChips from "./ui/UiFilterChips.vue";
import UiSearchInput from "./ui/UiSearchInput.vue";
import type { ChipOption } from "./ui/types";

const store = useStore();
const searchInput = ref<InstanceType<typeof UiSearchInput> | null>(null);
const listContainer = ref<HTMLElement | null>(null);

const report = computed(() => store.report.value);
const rows = computed(() => store.rows.value);
const counts = computed(() => store.filterCounts.value);
const region = computed(() => store.state.region);

const regionOptions = computed(() =>
  regionsOf(store.metadata).map((item) => ({
    id: item.region_id,
    label: regionLabel(item.region_id, item.display_name),
  })),
);

const statusOptions = computed<ChipOption[]>(() => [
  { value: "missing", label: "参考遗漏推定", count: counts.value.missing },
  { value: "observed", label: "已观察", count: counts.value.observed },
  { value: "unknown", label: "状态未知", count: counts.value.unknown },
  { value: "all", label: "全部", count: counts.value.all, title: "“全部”仅指本页当前支持的收集品集合" },
]);

const categoryOptions = computed<ChipOption[]>(() => {
  const base: ChipOption[] = [{ value: "", label: "全部类别" }];
  for (const categoryId of report.value?.coverage.supported_category_ids ?? []) {
    const entry = store.metadata.categories[categoryId] ?? null;
    base.push({ value: categoryId, label: entry?.display_name ?? categoryId });
  }
  return base;
});

const evidenceOptions = computed<ChipOption[]>(() => [
  { value: "", label: "全部依据" },
  ...EVIDENCE_OPTIONS.map((option) => ({ value: option.value, label: option.label })),
]);

const precisionOptions = computed<ChipOption[]>(() => [
  { value: "", label: "全部精度" },
  ...PRECISION_OPTIONS.map((option) => ({ value: option.value, label: option.label })),
]);

const regionTitle = computed(() =>
  region.value === "all"
    ? "全部区域"
    : region.value === "unknown"
      ? "区域未知"
      : regionLabel(region.value, store.metadata.regions[region.value]?.display_name ?? null),
);

const activeFilterSummary = computed(() => {
  const parts: string[] = [];
  const statusLabels: Record<StatusFilter, string> = {
    missing: "参考遗漏推定",
    observed: "已观察",
    unknown: "状态未知",
    all: "全部",
  };
  parts.push(`状态：${statusLabels[store.state.filters.status]}`);
  if (store.state.filters.category) {
    parts.push(`类别：${store.metadata.categories[store.state.filters.category]?.display_name ?? store.state.filters.category}`);
  }
  if (store.state.filters.evidence) {
    parts.push(`依据：${store.state.filters.evidence}`);
  }
  if (store.state.filters.precision) {
    parts.push(`定位：${store.state.filters.precision}`);
  }
  if (store.state.search) {
    parts.push(`搜索：${store.state.search}`);
  }
  return parts.join(" · ");
});

const showRegionSelector = computed(() => store.layout.value === "compact" || store.layout.value === "narrow");

/** 空清单文案：区分“搜索/筛选无结果”“支持范围内确实没有遗漏”“区域未知无清单”。 */
const emptyTitle = computed(() => {
  if (store.state.search) {
    return copy.emptyFiltered;
  }
  if (store.state.filters.status === "missing" && store.missingAvailable.value && region.value === "all") {
    return copy.emptyMissing;
  }
  if (region.value === "unknown") {
    return copy.emptyUnknownRegion;
  }
  return copy.emptyFiltered;
});

const remainingHint = computed(() => {
  if (!report.value || region.value === "all") {
    return null;
  }
  const byRegion = missingByRegion(store.metadata, report.value);
  const inRegion = region.value === "unknown" ? (byRegion.unknown ?? 0) : (byRegion[region.value] ?? 0);
  const rest = store.missingTotal.value - inRegion;
  if (rest <= 0) {
    return null;
  }
  return `当前区域 ${inRegion} 项；其余 ${rest} 项请在全部区域查看。`;
});

function onSelectRow(id: string): void {
  if (store.layout.value === "narrow" && listContainer.value) {
    store.actions.setListScrollTop(listContainer.value.scrollTop);
  }
  store.actions.openRow(id);
}

function selectRegion(value: string): void {
  store.actions.setRegion(value);
}

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
  <section class="flex min-h-0 flex-1 flex-col" aria-labelledby="regions-title">
    <header class="shrink-0 space-y-2 px-6 pt-5 pb-3">
      <div class="flex flex-wrap items-center gap-2">
        <h1 id="regions-title" class="text-xl font-semibold">区域与收集 · {{ regionTitle }}</h1>
        <span v-if="report" class="text-[13px] text-muted">范围：参考类别 {{ report.coverage.supported_category_ids.length }} 个；全范围参考遗漏 {{ store.missingTotal.value }} 项</span>
      </div>

      <div v-if="showRegionSelector" class="flex flex-wrap items-center gap-2">
        <label class="text-[13px] text-muted" for="region-select">区域</label>
        <select
          id="region-select"
          class="h-9 rounded-[6px] border border-line bg-surface px-2 text-[13px]"
          :value="region"
          data-testid="region-select"
          @change="selectRegion(($event.target as HTMLSelectElement).value)"
        >
          <option value="all">全部区域</option>
          <option v-for="option in regionOptions" :key="option.id" :value="option.id">{{ option.label }}</option>
          <option value="unknown">区域未知</option>
        </select>
      </div>

      <RegionCounts v-if="report && region !== 'all' && region !== 'unknown'" :region="region" compact />

      <UiEmptyState v-if="!report" :title="copy.noReport" :description="copy.noReportHelp">
        <UiButton variant="primary" @click="store.actions.setView('import')">打开存档</UiButton>
      </UiEmptyState>

      <template v-else>
        <CoverageNotice
          v-if="store.isStale.value"
          tone="warning"
          test-id="stale-banner"
          :title="copy.stale"
          :lines="['原证据可读；不显示为当前遗漏；旧结果导出必须携带过期标记。']"
        />
        <CoverageNotice
          v-else-if="!store.missingAvailable.value"
          tone="warning"
          test-id="degraded-banner"
          :title="copy.missingUnavailable"
          :lines="[copy.missingUnavailableHelp]"
        />
        <!-- 覆盖提示在主流程中始终可见（与状态提示并存）。 -->
        <CoverageNotice tone="info" test-id="coverage-banner" :title="copy.coverage" />

        <div class="flex flex-wrap items-center gap-2">
          <div class="w-64 max-w-full">
            <UiSearchInput
              ref="searchInput"
              :model-value="store.state.search"
              placeholder="搜索名称或 ID（含候选内部名）"
              :ariaLabel="'搜索名称或 ID'"
              @update:model-value="store.actions.setSearch($event)"
            />
          </div>
          <UiFilterChips
            :options="statusOptions"
            :model-value="store.state.filters.status"
            :ariaLabel="'按状态筛选'"
            @update:model-value="store.actions.setStatusFilter($event as StatusFilter)"
          />
          <UiFilterChips
            :options="categoryOptions"
            :model-value="store.state.filters.category ?? ''"
            :ariaLabel="'按类别筛选'"
            @update:model-value="store.actions.setCategory($event === '' ? null : $event)"
          />
          <UiButton size="sm" variant="ghost" :aria-expanded="store.state.moreFiltersOpen" @click="store.actions.toggleMoreFilters()">
            更多筛选
          </UiButton>
          <UiButton size="sm" variant="ghost" @click="store.actions.clearFilters()">清除筛选</UiButton>
          <span class="text-xs text-muted">{{ activeFilterSummary }}</span>
        </div>

        <div v-if="store.state.moreFiltersOpen" class="flex flex-wrap items-center gap-3 rounded-[8px] border border-line bg-raised/60 px-3 py-2">
          <div class="flex items-center gap-2">
            <span class="text-xs text-muted">证据依据</span>
            <UiFilterChips
              :options="evidenceOptions"
              :model-value="store.state.filters.evidence ?? ''"
              :ariaLabel="'按证据依据筛选'"
              @update:model-value="store.actions.setEvidenceFilter($event === '' ? null : $event)"
            />
          </div>
          <div class="flex items-center gap-2">
            <span class="text-xs text-muted">定位精度</span>
            <UiFilterChips
              :options="precisionOptions"
              :model-value="store.state.filters.precision ?? ''"
              :ariaLabel="'按定位精度筛选'"
              @update:model-value="store.actions.setPrecisionFilter($event === '' ? null : $event)"
            />
          </div>
        </div>
      </template>
    </header>

    <div ref="listContainer" class="scroll-area min-h-0 flex-1 px-6 pb-6" data-testid="region-list">
      <template v-if="report">
        <CollectibleList v-if="rows.length > 0" :rows="rows" :selected-id="store.state.selectedRowId" @select="onSelectRow" />

        <UiEmptyState
          v-else
          :title="emptyTitle"
          :description="`当前范围：${regionTitle} · ${activeFilterSummary}`"
        >
          <UiButton v-if="store.state.search" size="sm" @click="store.actions.setSearch('')">清空搜索</UiButton>
          <UiButton size="sm" @click="store.actions.clearFilters()">清除筛选</UiButton>
          <UiButton v-if="store.state.filters.status !== 'all'" size="sm" variant="ghost" @click="store.actions.setStatusFilter('all')">
            查看全部条目
          </UiButton>
        </UiEmptyState>

        <p v-if="remainingHint" class="mt-3 text-xs text-muted" data-testid="remaining-hint">
          {{ remainingHint }}
          <button type="button" class="cursor-pointer text-accent hover:underline" @click="store.actions.setRegion('all')">查看全部区域</button>
        </p>
      </template>
    </div>
  </section>
</template>
