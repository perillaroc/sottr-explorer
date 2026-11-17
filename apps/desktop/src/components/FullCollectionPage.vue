<script setup lang="ts">
// 全收集（100%）页面（SX-021 阶段 C2b；2026-11-05 界面美化阶段三）：
// - 数据来自内嵌目录（data/catalog）与当前存档的观察集合；
// - 规则与 sottr-core::missing / CLI missing 一致（C2a 黄金测试保证）：
//   必收类别按必收子集给逐项缺失清单；可选类别只给计数；未闭合类别不显示缺失结论；
//   共用同一 ID 的条目按 ID 计数并列出全部名称；目录外观察 ID 单列。
import { computed } from "vue";
import { ChevronDown } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
import { categoryLabel, copy } from "../app/labels";
import { categoryClass, categoryDotClass } from "../app/category-style";
import { pagePaddingClass } from "../app/layout";
import { collectionDisplayName, guidanceCoverage, guidanceScopeForId } from "../app/guidance";
import { groupDisplayName, regionName } from "../app/collection";
import {
  computeMissing,
  dlcCollectionProgress,
  productionCatalog,
  type CategoryMissing,
  type DlcCollectionProgress,
  type MissingItem,
} from "../data/catalog";
import { optionalReferenceStatus } from "../data/optional-references";
import AppPageHeader from "./AppPageHeader.vue";
import CoverageNotice from "./CoverageNotice.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiProgressBar from "./ui/UiProgressBar.vue";

const store = useStore();

const layout = computed(() => store.layout.value);
const pageX = computed(() => pagePaddingClass(layout.value));
const isNarrow = computed(() => layout.value === "narrow");

/** 必收类别（100% 口径）在计划里的区域分布。 */
const requiredRegionRows = computed(() => {
  const rows: { slug: string; label: string; kind: "main" | "special"; cells: { id: string; label: string; collected: number; total: number }[]; collected: number; total: number }[] = [];
  for (const region of store.plan.value.regions) {
    const cells = region.groups
      .filter((group) => group.required_for_100 === true)
      .map((group) => ({
        id: `${group.category}:${group.subcategory ?? ""}`,
        label: groupDisplayName(group),
        collected: group.collected,
        total: group.total,
      }));
    if (cells.length === 0) {
      continue;
    }
    const collected = cells.reduce((sum, cell) => sum + cell.collected, 0);
    const total = cells.reduce((sum, cell) => sum + cell.total, 0);
    rows.push({ slug: region.slug, label: regionName(region), kind: region.kind, cells, collected, total });
  }
  return rows;
});

function openRegion(slug: string, category: string): void {
  store.actions.setRegion(slug);
  store.actions.setCategory(category.split(":")[0]);
  store.actions.setStatusFilter("missing");
  store.actions.setView("regions");
}

const missing = computed(() => {
  const report = store.report.value;
  if (!report) {
    return null;
  }
  const ids = report.observations.inventory.records.map((record) => record.internal_id);
  return computeMissing(productionCatalog, ids);
});

const required = computed<CategoryMissing[]>(
  () => missing.value?.categories.filter((category) => category.required_for_100 === true) ?? [],
);
const optional = computed<CategoryMissing[]>(
  () => missing.value?.categories.filter((category) => category.required_for_100 === false) ?? [],
);
const unresolved = computed<CategoryMissing[]>(
  () => missing.value?.categories.filter((category) => category.required_for_100 === null) ?? [],
);

/** DLC 故事条目按 7 个包分组；仅展示，不进入必收 352。 */
const dlcGroups = computed<DlcCollectionProgress[]>(() => {
  const report = store.report.value;
  if (!report) {
    return [];
  }
  const ids = report.observations.inventory.records.map((record) => record.internal_id);
  return dlcCollectionProgress(productionCatalog, ids);
});
const dlcTotals = computed(() => {
  const totals = { total: 0, observed: 0 };
  for (const group of dlcGroups.value) {
    totals.total += group.total;
    totals.observed += group.observed;
  }
  return totals;
});

const requiredTotals = computed(() => {
  const totals = { total: 0, observed: 0, missing: 0 };
  for (const category of required.value) {
    totals.total += category.total;
    totals.observed += category.observed;
    totals.missing += category.missing ?? 0;
  }
  return totals;
});

function missingLabel(item: MissingItem): string {
  return item.name ?? item.id;
}

function categoryName(id: string, fallback: string): string {
  return categoryLabel(id, fallback);
}

/** 全量必收指引覆盖（静态，与当前存档状态无关）。 */
const guidance = guidanceCoverage();
const guidanceCovered =
  guidance.mapped + guidance.item + guidance.collection + guidance.unit;

function guidanceLabel(id: string): string {
  switch (guidanceScopeForId(id)) {
    case "map":
      return t("地图", "Map");
    case "collection":
      return t("合集", "Collection");
    case "unit":
      return t("子关卡", "Sub-level");
    case "item":
      return t("指引", "Guide");
    default:
      return "";
  }
}
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="full-collection-title" data-testid="full-collection">
    <div class="mx-auto w-full max-w-[1120px] pb-8" :class="pageX">
      <AppPageHeader
        title-id="full-collection-title"
        :title="t('全收集（100%）', 'Full collection (100%)')"
        :description="t('按必收子集列出缺失清单；地图 / 合集 / 子关卡是指引来源，不代表地图覆盖全部必收。', 'Missing items are listed for the required subset; map / collection / sub-level are guidance sources, not proof that the map covers all required items.')"
      />

      <CoverageNotice
        v-if="store.isStale.value"
        class="mt-4"
        tone="warning"
        test-id="stale-banner"
        :title="copy.stale"
        :lines="[t('请刷新后再核对收集状态。', 'Refresh before checking collection status again.')]"
      />

      <UiEmptyState
        v-if="!missing"
        :title="t('先分析一个存档文件，再查看全收集缺失清单。', 'Analyze a save file first to view the full-collection missing list.')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
      </UiEmptyState>

      <UiEmptyState
        v-else-if="!store.planAvailable.value"
        :title="t('资料文件没有收集品清单', 'Profile files have no collectible list')"
        :description="t('全收集缺失清单只适用于普通战役存档；请选择存档文件。', 'The full-collection missing list only applies to campaign saves; choose a save file.')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("选择存档文件", "Choose a save file") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <!-- 必收工作带：旧金色只用于此处的关键进度与细线。 -->
        <section class="rounded-section border border-gold/40 bg-gold-bg px-4 py-3" data-testid="required-band">
          <div class="flex flex-wrap items-end gap-x-8 gap-y-3">
            <div>
              <p class="text-xs text-gold">{{ t("必收", "Required") }}</p>
              <p class="mt-0.5 text-[22px] leading-[30px] font-semibold tabular-nums text-gold">
                {{ requiredTotals.observed }}/{{ requiredTotals.total }}
              </p>
            </div>
            <div>
              <p class="text-xs text-gold">{{ t("缺失", "Missing") }}</p>
              <p class="mt-0.5 text-[22px] leading-[30px] font-semibold tabular-nums text-gold">
                {{ requiredTotals.missing }}
              </p>
            </div>
            <div class="min-w-56 flex-1" data-testid="guidance-coverage">
              <p class="text-xs text-gold">
                {{ t("必收指引", "Required guidance") }} {{ guidanceCovered }}/{{ guidance.required }}
              </p>
              <UiProgressBar
                class="mt-1"
                :value="guidanceCovered"
                :total="guidance.required"
                tone="gold"
                :label="t('必收指引覆盖', 'Required guidance coverage')"
              />
              <p class="mt-1 text-xs text-muted">
                {{ tFormat(
                  "地图 {mapped} · 合集 {collection} · 子关卡 {unit}",
                  "map {mapped} · collection {collection} · sub-level {unit}",
                  { mapped: guidance.mapped, collection: guidance.collection, unit: guidance.unit },
                ) }}
              </p>
            </div>
          </div>
          <p v-if="missing.observed_not_in_catalog.length > 0" class="mt-2 text-xs text-muted">
            {{ t(`目录外 ID ${missing.observed_not_in_catalog.length}（不计入必收）`, `Off-catalog IDs ${missing.observed_not_in_catalog.length} (not part of required)`) }}
          </p>
        </section>

        <h2 class="mt-6 mb-2 ui-section-title">{{ t("必收类别 · 按区域", "Required categories · by region") }}</h2>
        <p v-if="isNarrow" class="mb-1 text-xs text-muted">
          {{ t("左右滚动查看完整矩阵。", "Scroll horizontally to see the full matrix.") }}
        </p>
        <div class="ui-panel overflow-x-auto">
          <table class="w-full min-w-[720px] border-collapse text-[13px]" data-testid="required-region-matrix">
            <thead>
              <tr class="border-b border-line text-left text-xs text-muted">
                <th class="h-9 px-3 font-medium">{{ t("区域", "Region") }}</th>
                <th class="h-9 px-3 font-medium">{{ t("必收进度", "Required progress") }}</th>
                <th class="h-9 px-3 font-medium">{{ t("类别（已收集/总数）", "Categories (collected/total)") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in requiredRegionRows"
                :key="row.slug"
                class="border-b border-line/60 align-top last:border-b-0"
                :data-testid="`required-region-${row.slug}`"
              >
                <td class="px-3 py-2">
                  <span class="text-ink">{{ row.label }}</span>
                  <span v-if="row.kind === 'special'" class="ml-1 rounded-control bg-raised px-1.5 py-0.5 text-xs text-muted">
                    {{ t("特殊", "Special") }}
                  </span>
                </td>
                <td class="px-3 py-2">
                  <div class="w-28">
                    <span class="text-xs tabular-nums text-muted">{{ row.collected }}/{{ row.total }}</span>
                    <UiProgressBar
                      class="mt-1"
                      :value="row.collected"
                      :total="row.total"
                      :label="tFormat('{region} 必收进度', '{region} required progress', { region: row.label })"
                    />
                  </div>
                </td>
                <td class="px-3 py-2">
                  <div class="flex flex-wrap gap-1.5">
                    <button
                      v-for="cell in row.cells"
                      :key="cell.id"
                      type="button"
                      class="ui-interactive inline-flex cursor-pointer items-baseline gap-1 rounded-control border border-line bg-surface px-2 py-1 text-xs text-accent-strong enabled:hover:bg-raised"
                      @click="openRegion(row.slug, cell.id)"
                    >
                      {{ cell.label }}
                      <span class="tabular-nums text-muted">{{ cell.collected }}/{{ cell.total }}</span>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <h2 class="mt-6 mb-2 ui-section-title">{{ t("必收类别", "Required categories") }}</h2>
        <ul class="space-y-2">
          <li v-for="category in required" :key="category.id" :data-testid="`category-${category.id}`">
            <details class="ui-disclosure ui-panel overflow-hidden" :data-testid="`missing-toggle-${category.id}`">
              <summary class="flex cursor-pointer flex-wrap items-center gap-2 px-3 py-2.5 text-[13px]">
                <span class="cat-dot" :class="categoryDotClass(category.id)" aria-hidden="true"></span>
                <span class="font-medium" :class="categoryClass(category.id)">{{ categoryName(category.id, category.name) }}</span>
                <span class="text-xs tabular-nums text-muted">{{ category.observed }}/{{ category.total }}</span>
                <UiBadge v-if="(category.missing ?? 0) > 0" tone="accent">
                  {{ t(`缺失 ${category.missing}`, `Missing ${category.missing}`) }}
                </UiBadge>
                <span v-else class="text-xs text-muted">{{ t("已全部观察到", "All observed") }}</span>
                <span
                  v-if="(category.items?.length ?? 0) > 0"
                  class="ml-auto inline-flex items-center gap-1 text-xs text-accent-strong"
                >
                  {{ t(`查看缺失清单（${category.items?.length}）`, `View missing list (${category.items?.length})`) }}
                  <ChevronDown class="ui-chevron" :size="16" aria-hidden="true" />
                </span>
              </summary>
              <ul v-if="(category.items?.length ?? 0) > 0" class="max-h-72 overflow-auto border-t border-line">
                <li
                  v-for="item in category.items"
                  :key="item.id"
                  class="grid grid-cols-[minmax(0,1fr)_auto] gap-x-3 gap-y-1 border-b border-line/50 px-3 py-2 text-xs last:border-b-0"
                  data-testid="missing-item"
                >
                  <span class="min-w-0 text-[13px] text-ink">{{ missingLabel(item) }}</span>
                  <span class="mono text-xs text-muted">{{ item.id }}</span>
                  <div class="col-span-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
                    <span v-if="item.collection">
                      {{ tFormat("合集：{value}", "Collection: {value}", { value: collectionDisplayName(item.collection) }) }}
                    </span>
                    <span v-if="guidanceLabel(item.id)" class="inline-flex items-center rounded-control bg-raised px-1.5 py-0.5">
                      {{ tFormat("指引：{value}", "Guide: {value}", { value: guidanceLabel(item.id) }) }}
                    </span>
                    <span v-if="item.shared_names">
                      {{ tFormat("共用 ID：{names}（按 ID 计数，不拆分判定）", "Shared ID: {names} (counted by ID; not split for assessment)", { names: item.shared_names.join(" / ") }) }}
                    </span>
                  </div>
                </li>
              </ul>
            </details>
          </li>
        </ul>

        <h2 class="mt-6 mb-2 ui-section-title">{{ t("可选类别", "Optional categories") }}</h2>
        <p class="mb-2 text-xs text-muted">
          {{ t(
            "可选类别不进入必收 352。只有能对上存档区域计数器的类别给出总数；其余只显示已观察数，参考数标“未知”。",
            "Optional categories are not part of the required 352. A total is shown only when it matches save region counters; otherwise only the observed count is shown and the reference count is marked unknown.",
          ) }}
        </p>
        <ul class="space-y-1.5">
          <li
            v-for="category in optional"
            :key="category.id"
            class="ui-panel flex flex-wrap items-center gap-2 px-3 py-2.5 text-[13px]"
            :data-testid="`category-${category.id}`"
          >
            <span class="cat-dot" :class="categoryDotClass(category.id)" aria-hidden="true"></span>
            <span class="text-ink">{{ categoryName(category.id, category.name) }}</span>
            <template v-if="optionalReferenceStatus(category.id) === 'verified'">
              <span class="text-xs tabular-nums text-muted">{{ category.observed }}/{{ category.total }}</span>
              <span class="text-xs text-muted">· {{ t("区域计数器已对账", "matched to region counters") }}</span>
            </template>
            <span
              v-else-if="optionalReferenceStatus(category.id) === 'unknown'"
              class="text-xs tabular-nums text-muted"
            >
              {{ tFormat("已观察 {n} · 参考数未知", "Observed {n} · reference count unknown", { n: category.observed }) }}
            </span>
            <span v-else class="text-xs tabular-nums text-muted">
              {{ tFormat("仅展示 {n} 条 · 不计入必收", "Display only · {n} items · not counted", { n: category.observed }) }}
            </span>
          </li>
        </ul>

        <h2 class="mt-6 mb-2 ui-section-title">{{ t("DLC 故事条目（不计入 100%）", "DLC story items (not part of 100%)") }}</h2>
        <p class="mb-2 text-xs text-muted">
          {{ tFormat(
            "按 7 个 DLC 包分组，DLC 合计 {observed}/{total}；仅展示，不进入必收 352 或全收集结论。",
            "Grouped into 7 DLC packs; {observed}/{total} observed in total. Display only: not part of the required 352 or the full-collection result.",
            { observed: dlcTotals.observed, total: dlcTotals.total },
          ) }}
        </p>
        <ul class="space-y-1.5" data-testid="dlc-groups">
          <li
            v-for="group in dlcGroups"
            :key="group.id"
            class="ui-panel grid grid-cols-[4.5rem_minmax(0,1fr)_auto] items-center gap-3 px-3 py-2.5 text-[13px]"
            :data-testid="`dlc-group-${group.label}`"
          >
            <span class="text-ink">{{ group.label }}</span>
            <UiProgressBar
              :value="group.observed"
              :total="group.total"
              :label="tFormat('{label} 故事条目进度', '{label} story item progress', { label: group.label })"
            />
            <span class="text-xs tabular-nums text-muted">{{ group.observed }}/{{ group.total }}</span>
          </li>
        </ul>

        <h2 class="mt-6 mb-2 ui-section-title">{{ t("未闭合类别", "Unresolved categories") }}</h2>
        <ul class="space-y-1.5">
          <li
            v-for="category in unresolved"
            :key="category.id"
            class="ui-panel flex flex-wrap items-center gap-2 px-3 py-2.5 text-[13px]"
            :data-testid="`category-${category.id}`"
          >
            <span class="cat-dot" :class="categoryDotClass(category.id)" aria-hidden="true"></span>
            <span class="text-ink">{{ categoryName(category.id, category.name) }}</span>
            <span class="text-xs tabular-nums text-muted">{{ category.observed }}/{{ category.total }}</span>
          </li>
        </ul>
      </template>
    </div>
  </section>
</template>
