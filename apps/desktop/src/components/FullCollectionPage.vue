<script setup lang="ts">
// 全收集（100%）页面（SX-021 阶段 C2b）：
// - 数据来自内嵌目录（data/catalog）与当前存档的观察集合；
// - 规则与 sottr-core::missing / CLI missing 一致（C2a 黄金测试保证）：
//   必收类别按必收子集给逐项缺失清单；可选类别只给计数；未闭合类别不显示缺失结论；
//   共用同一 ID 的条目按 ID 计数并列出全部名称；目录外观察 ID 单列。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
import { categoryLabel } from "../app/labels";
import { collectionDisplayName, guidanceCoverage, guidanceScopeForId } from "../app/guidance";
import { groupDisplayName, regionName } from "../app/collection";
import {
  computeMissing,
  productionCatalog,
  type CategoryMissing,
  type MissingItem,
} from "../data/catalog";

const store = useStore();

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
  <section class="scroll-area min-h-0 flex-1 px-6 py-6" data-testid="full-collection">
    <div class="mx-auto max-w-5xl">
      <header class="mb-4 flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h1 class="text-lg font-semibold text-ink">{{ t("全收集（100%）", "Full collection (100%)") }}</h1>
        <span v-if="missing" class="text-[13px] text-muted">
          {{ t(`必收 ${requiredTotals.observed}/${requiredTotals.total} · 缺失 ${requiredTotals.missing}`, `Required ${requiredTotals.observed}/${requiredTotals.total} · missing ${requiredTotals.missing}`) }}
        </span>
      </header>

      <p v-if="missing" class="mb-3 text-[12px] text-muted" data-testid="guidance-coverage">
        {{ tFormat(
          "必收指引 {covered}/{total}（地图 {mapped} · 合集 {collection} · 子关卡 {unit}）",
          "Required guidance {covered}/{total} (map {mapped} · collection {collection} · sub-level {unit})",
          {
            covered: guidanceCovered,
            total: guidance.required,
            mapped: guidance.mapped,
            collection: guidance.collection,
            unit: guidance.unit,
          },
        ) }}
      </p>

      <p v-if="!missing" class="rounded-[8px] border border-line bg-surface px-3 py-4 text-[13px] text-muted">
        {{ t("先分析一个存档文件，再查看全收集缺失清单。", "Analyze a save file first to view the full-collection missing list.") }}
      </p>

      <template v-else>
        <div class="mb-4 flex flex-wrap items-center gap-3 rounded-[8px] border border-line bg-surface px-4 py-2 text-[13px]">
          <span class="text-ink">{{ t(`必收合计：${requiredTotals.observed}/${requiredTotals.total}`, `Required total: ${requiredTotals.observed}/${requiredTotals.total}`) }}</span>
          <span class="rounded-[999px] bg-accent-bg px-2 py-0.5 text-accent-strong">{{ t(`缺失 ${requiredTotals.missing}`, `Missing ${requiredTotals.missing}`) }}</span>
          <span v-if="missing.observed_not_in_catalog.length > 0" class="text-muted">
            {{ t(`目录外 ID ${missing.observed_not_in_catalog.length}`, `Off-catalog IDs ${missing.observed_not_in_catalog.length}`) }}
          </span>
        </div>

        <h2 class="mb-2 text-[13px] font-medium text-muted">{{ t("必收类别 · 按区域", "Required categories · by region") }}</h2>
        <div class="mb-5 overflow-x-auto rounded-[8px] border border-line bg-surface px-3">
        <table class="w-full min-w-[640px] border-collapse text-[12px]" data-testid="required-region-matrix">
          <thead>
            <tr class="border-b border-line text-left text-muted">
              <th class="py-1 pr-2 font-medium">{{ t("区域", "Region") }}</th>
              <th class="py-1 pr-2 font-medium">{{ t("必收进度", "Required progress") }}</th>
              <th class="py-1 font-medium">{{ t("类别（已收集/总数）", "Categories (collected/total)") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="row in requiredRegionRows"
              :key="row.slug"
              class="border-b border-line/60 align-top"
              :data-testid="`required-region-${row.slug}`"
            >
              <td class="py-1 pr-2">
                <span class="text-ink">{{ row.label }}</span>
                <span v-if="row.kind === 'special'" class="ml-1 text-[10px] text-muted">{{ t("特殊", "Special") }}</span>
              </td>
              <td class="py-1 pr-2 tabular-nums text-muted">{{ row.collected }}/{{ row.total }}</td>
              <td class="py-1">
                <span v-for="(cell, index) in row.cells" :key="cell.id">
                  <span v-if="index > 0" class="text-muted"> · </span>
                  <button
                    type="button"
                    class="cursor-pointer text-accent-strong hover:underline"
                    @click="openRegion(row.slug, cell.id)"
                  >
                    {{ cell.label }} {{ cell.collected }}/{{ cell.total }}
                  </button>
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2 class="mb-2 text-[13px] font-medium text-muted">{{ t("必收类别", "Required categories") }}</h2>
      <ul class="mb-5 space-y-1.5">
        <li
          v-for="category in required"
          :key="category.id"
          class="rounded-[8px] border border-line bg-surface px-3 py-2"
          :data-testid="`category-${category.id}`"
        >
          <div class="flex flex-wrap items-center gap-2 text-[13px]">
            <span class="text-ink">{{ categoryName(category.id, category.name) }}</span>
            <span class="text-muted">{{ category.observed }}/{{ category.total }}</span>
            <span v-if="(category.missing ?? 0) > 0" class="rounded-[6px] bg-accent-bg px-2 py-0.5 text-accent-strong">
              {{ t(`缺失 ${category.missing}`, `Missing ${category.missing}`) }}
            </span>
            <span v-else class="text-muted">{{ t("已全部观察到", "All observed") }}</span>
          </div>
          <details
            v-if="(category.items?.length ?? 0) > 0"
            class="mt-1.5"
            :data-testid="`missing-toggle-${category.id}`"
          >
            <summary class="cursor-pointer text-[12px] text-accent-strong">{{ t(`展开缺失清单（${category.items?.length}）`, `Expand missing list (${category.items?.length})`) }}</summary>
            <ul class="mt-1 max-h-72 space-y-0.5 overflow-auto pl-3 text-[12px] leading-5">
              <li
                v-for="item in category.items"
                :key="item.id"
                class="text-muted"
                data-testid="missing-item"
              >
                <span class="text-ink">{{ missingLabel(item) }}</span>
                <span class="ml-2 font-mono text-[11px]">{{ item.id }}</span>
                <span v-if="item.collection" class="ml-2">
                  {{ tFormat("合集：{value}", "Collection: {value}", { value: collectionDisplayName(item.collection) }) }}
                </span>
                <span
                  v-if="guidanceLabel(item.id)"
                  class="ml-2 rounded-[4px] bg-raised px-1.5 py-0.5 text-[10px] text-muted"
                >
                  {{ guidanceLabel(item.id) }}
                </span>
                <span v-if="item.shared_names" class="ml-2">
                  {{ tFormat("共用 ID：{names}（按 ID 计数，不拆分判定）", "Shared ID: {names} (counted by ID; not split for assessment)", { names: item.shared_names.join(" / ") }) }}
                </span>
              </li>
            </ul>
          </details>
        </li>
      </ul>

      <h2 class="mb-2 text-[13px] font-medium text-muted">{{ t("可选类别", "Optional categories") }}</h2>
      <ul class="mb-5 space-y-1.5">
        <li
          v-for="category in optional"
          :key="category.id"
          class="flex flex-wrap items-center gap-2 rounded-[8px] border border-line bg-surface px-3 py-2 text-[13px]"
          :data-testid="`category-${category.id}`"
        >
          <span class="text-ink">{{ categoryName(category.id, category.name) }}</span>
          <span class="text-muted">{{ category.observed }}/{{ category.total }}</span>
        </li>
      </ul>

      <h2 class="mb-2 text-[13px] font-medium text-muted">{{ t("未闭合类别", "Unresolved categories") }}</h2>
      <ul class="space-y-1.5">
        <li
          v-for="category in unresolved"
          :key="category.id"
          class="flex flex-wrap items-center gap-2 rounded-[8px] border border-line bg-surface px-3 py-2 text-[13px]"
          :data-testid="`category-${category.id}`"
        >
          <span class="text-ink">{{ categoryName(category.id, category.name) }}</span>
          <span class="text-muted">{{ category.observed }}/{{ category.total }}</span>
        </li>
      </ul>
      </template>
    </div>
  </section>
</template>
