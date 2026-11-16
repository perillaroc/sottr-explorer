<script setup lang="ts">
// 收集计划页（P1-2）：回答“这次去哪里收”，与全收集页的“还差什么”分工。
//
// - 只列未收集条目，层级固定为 区域 → 子关卡 → 类别；
// - 状态、名称与获取方式全部来自 `buildPlan` / 图鉴（CLI `plan` 的镜像），
//   本页只做筛选、分组与跳转；
// - 筛选（仅必收 / 全部、区域、类别）与区域清单、地图页互不影响；
// - 每条可直接打开地图（有底图时）、详情或已核对的逐条攻略；
// - 导出为可打印 HTML / CSV 清单，写入新目录（脱敏可选）。
import { computed } from "vue";
import { ExternalLink, ListChecks, MapPinned, Printer } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
import { pagePaddingClass } from "../app/layout";
import { copy } from "../app/labels";
import type { ChecklistItem, PlanSort } from "../app/plan-checklist";
import { buildChecklist } from "../app/plan-checklist";
import { basemapForUnit } from "../data/basemaps";
import AppPageHeader from "./AppPageHeader.vue";
import CoverageNotice from "./CoverageNotice.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiFilterChips from "./ui/UiFilterChips.vue";

const store = useStore();

const layout = computed(() => store.layout.value);
const pageX = computed(() => pagePaddingClass(layout.value));
const isNarrow = computed(() => layout.value === "narrow");
const checklist = computed(() => store.planChecklist.value);

const scopeOptions = computed(() => [
  { value: "required", label: t("仅必收", "Required only") },
  { value: "all", label: t("全部未收集", "All uncollected") },
]);

const categoryOptions = computed(() => [
  {
    value: "",
    // “全部类别”的计数与类别筛选无关（清空类别后的总未收集数）。
    label: t("全部类别", "All categories"),
    count: buildChecklist(store.plan.value, {
      scope: store.state.planScope,
      region: store.state.planRegion,
      category: null,
      sort: store.state.planSort,
    }).items,
  },
  ...store.planCategoryOptions.value.map((option) => ({
    value: option.id,
    label: option.label,
    count: option.count,
  })),
]);

const sortOptions = computed<{ value: PlanSort; label: string }[]>(() => [
  { value: "unit", label: t("关卡顺序", "Sub-level order") },
  { value: "remaining", label: t("未收集数", "Uncollected count") },
]);

function onRegionChange(event: Event): void {
  store.actions.setPlanRegion((event.target as HTMLSelectElement).value);
}

function onSortChange(event: Event): void {
  store.actions.setPlanSort((event.target as HTMLSelectElement).value as PlanSort);
}

function onCategoryChange(value: string): void {
  store.actions.setPlanCategory(value === "" ? null : value);
}

function openDetail(regionSlug: string, item: ChecklistItem): void {
  store.actions.setRegion(regionSlug);
  store.actions.openRow(item.id);
  store.actions.setView("regions");
}

function openMap(item: ChecklistItem): void {
  store.actions.openInMap(item.id);
}

function firstGuideLink(item: ChecklistItem) {
  return store.directLinksForItem(item.item)[0] ?? null;
}

function guideLabel(item: ChecklistItem): string {
  const link = firstGuideLink(item);
  if (!link) {
    return t("攻略页", "Guide");
  }
  return link.guideItem ? tFormat("攻略页 · {locator}", "Guide · {locator}", { locator: link.guideItem }) : t("攻略页", "Guide");
}

function openGuide(item: ChecklistItem): void {
  const link = firstGuideLink(item);
  if (!link) {
    return;
  }
  void store.actions.openGuideLink(link.locationId, link.sourceRef, guideLabel(item));
}

function onExport(): void {
  store.actions.openPlanExportDialog();
}

const unitHasMap = (unit: number | null): boolean => unit !== null && basemapForUnit(unit) !== null;
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="collection-plan-title" data-testid="plan-page">
    <div class="mx-auto w-full max-w-[1120px] pb-8" :class="pageX">
      <AppPageHeader
        title-id="collection-plan-title"
        :title="t('收集计划', 'Collection plan')"
        :description="t('只列还没收集的条目，按区域 → 子关卡 → 类别聚合；回答“这次去哪里收”。全收集页回答“还差什么”。', 'Lists uncollected items grouped by region → sub-level → category: where to go next. The full-collection page answers what is still missing.')"
      >
        <template #actions>
          <UiButton data-testid="plan-export" :disabled="!store.planAvailable.value" @click="onExport">
            <Printer :size="16" aria-hidden="true" />
            {{ t("导出清单", "Export checklist") }}
          </UiButton>
        </template>
      </AppPageHeader>

      <CoverageNotice
        v-if="store.isStale.value"
        class="mt-4"
        tone="warning"
        test-id="plan-stale-banner"
        :title="copy.stale"
        :lines="[t('请刷新后再生成收集计划。', 'Refresh before generating a collection plan.')]"
      />

      <UiEmptyState
        v-if="!store.report.value"
        :title="t('先分析一个存档文件，再生成收集计划。', 'Analyze a save file first to build a collection plan.')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
      </UiEmptyState>

      <UiEmptyState
        v-else-if="!store.planAvailable.value"
        :title="t('资料文件没有收集品清单', 'Profile files have no collectible list')"
        :description="t('收集计划只适用于普通战役存档；请选择存档文件。', 'The collection plan only applies to campaign saves; choose a save file.')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("选择存档文件", "Choose a save file") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <section class="mt-4 rounded-section border border-gold/40 bg-gold-bg px-4 py-3" data-testid="plan-summary">
          <div class="flex flex-wrap items-end gap-x-8 gap-y-3">
            <div>
              <p class="text-xs text-gold">{{ t("未收集（当前筛选）", "Uncollected (filtered)") }}</p>
              <p class="mt-0.5 text-[22px] leading-[30px] font-semibold tabular-nums text-gold">
                {{ checklist.items }}
              </p>
            </div>
            <div>
              <p class="text-xs text-gold">{{ t("其中必收", "Required among them") }}</p>
              <p class="mt-0.5 text-[22px] leading-[30px] font-semibold tabular-nums text-gold">
                {{ checklist.required }}
              </p>
            </div>
            <div>
              <p class="text-xs text-gold">{{ t("涉及区域", "Regions") }}</p>
              <p class="mt-0.5 text-[22px] leading-[30px] font-semibold tabular-nums text-gold">
                {{ checklist.regionCount }}
              </p>
            </div>
            <p class="min-w-56 flex-1 text-xs text-muted">
              {{ t("清单只列未收集条目；已收集的条目视为完成，不再出现在这次计划里。", "Only uncollected items are listed; collected items count as done and leave the plan.") }}
              <span class="block">{{ store.planScopeText.value }}</span>
            </p>
          </div>
        </section>

        <div class="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2">
          <UiFilterChips
            :options="scopeOptions"
            :model-value="store.state.planScope"
            :ariaLabel="t('清单口径', 'Checklist scope')"
            variant="segmented"
            @update:model-value="store.actions.setPlanScope($event as 'required' | 'all')"
          />
          <label class="flex items-center gap-2 text-[13px] text-muted">
            {{ t("区域", "Region") }}
            <select
              class="h-9 max-w-[200px] cursor-pointer rounded-control border border-line bg-surface px-2 text-[13px] text-ink"
              :value="store.state.planRegion"
              data-testid="plan-region-select"
              @change="onRegionChange"
            >
              <option value="all">{{ t("全部区域", "All regions") }}</option>
              <option v-for="region in store.planRegionOptions.value" :key="region.slug" :value="region.slug">
                {{ region.label }}{{ region.count > 0 ? `（${region.count}）` : "" }}
              </option>
            </select>
          </label>
          <label class="flex items-center gap-2 text-[13px] text-muted">
            {{ t("排序", "Sort") }}
            <select
              class="h-9 cursor-pointer rounded-control border border-line bg-surface px-2 text-[13px] text-ink"
              :value="store.state.planSort"
              data-testid="plan-sort"
              @change="onSortChange"
            >
              <option v-for="option in sortOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
            </select>
          </label>
        </div>

        <div class="mt-2 flex flex-wrap items-center gap-2" data-testid="plan-category-chips">
          <UiFilterChips
            :options="categoryOptions"
            :model-value="store.state.planCategory ?? ''"
            :ariaLabel="t('类别', 'Category')"
            @update:model-value="onCategoryChange"
          />
        </div>

        <UiEmptyState
          v-if="checklist.regions.length === 0"
          class="mt-6"
          :title="t('当前筛选下没有未收集条目。', 'No uncollected items under the current filters.')"
          :description="t('可以放宽口径（全部未收集）或清空区域/类别筛选。', 'Widen the scope (all uncollected) or clear the region/category filters.')"
        >
          <UiButton
            v-if="store.state.planScope === 'required'"
            @click="store.actions.setPlanScope('all')"
          >{{ t("改为全部未收集", "Switch to all uncollected") }}</UiButton>
          <UiButton
            v-else
            @click="store.actions.setPlanRegion('all'); store.actions.setPlanCategory(null)"
          >{{ t("清空筛选", "Clear filters") }}</UiButton>
        </UiEmptyState>

        <template v-else>
          <p v-if="isNarrow" class="mt-3 text-xs text-muted">
            {{ t("窄窗下条目按单列排列；按钮仍可用 Tab 访问。", "In narrow windows items stack in one column; all buttons remain reachable with Tab.") }}
          </p>

          <section
            v-for="region in checklist.regions"
            :key="region.slug"
            class="mt-6"
            :data-testid="`plan-region-${region.slug}`"
          >
            <div class="mb-2 flex flex-wrap items-center gap-2">
              <h2 class="ui-section-title">{{ region.label }}</h2>
              <UiBadge tone="accent">{{ t(`未收集 ${region.remaining}`, `${region.remaining} uncollected`) }}</UiBadge>
              <UiBadge v-if="region.kind === 'special'" tone="neutral">{{ t("特殊区域", "Special") }}</UiBadge>
            </div>

            <div class="space-y-3">
              <section
                v-for="unit in region.units"
                :key="`${region.slug}:${unit.unit ?? 'none'}`"
                class="ui-panel overflow-hidden"
                :data-testid="`plan-unit-${unit.unit ?? 'none'}`"
              >
                <header class="flex flex-wrap items-center gap-2 border-b border-line bg-raised/50 px-3 py-1.5">
                  <span class="text-[13px] font-medium text-ink">{{ unit.label }}</span>
                  <span class="text-xs tabular-nums text-muted">
                    {{ t(`${unit.remaining} 项未收集`, `${unit.remaining} uncollected`) }}
                  </span>
                  <span v-if="unitHasMap(unit.unit)" class="inline-flex items-center gap-1 text-xs text-muted">
                    <MapPinned :size="13" aria-hidden="true" />
                    {{ t("有底图", "Basemap") }}
                  </span>
                </header>

                <div v-for="group in unit.groups" :key="`${group.category}:${group.subcategory ?? ''}`" class="border-b border-line/60 last:border-b-0">
                  <p class="px-3 pt-2 text-xs font-medium text-muted">{{ group.label }}</p>
                  <ul>
                    <li
                      v-for="item in group.items"
                      :key="item.id"
                      class="flex flex-wrap items-start gap-x-3 gap-y-1 px-3 py-2"
                      :data-testid="`plan-item-${item.id}`"
                    >
                      <div class="min-w-0 flex-1">
                        <div class="flex flex-wrap items-center gap-1.5 text-[13px]">
                          <span class="text-ink">{{ item.name }}</span>
                          <UiBadge :tone="item.required === true ? 'accent' : 'neutral'">
                            {{ item.required === true ? t("必收", "Required") : item.required === false ? t("可选", "Optional") : t("未闭合", "Unresolved") }}
                          </UiBadge>
                          <UiBadge v-if="item.isDlc" tone="info">DLC</UiBadge>
                          <span class="mono text-xs text-muted">{{ item.id }}</span>
                        </div>
                        <p class="mt-0.5 text-xs leading-5 text-muted">{{ item.locationHint }}</p>
                        <p class="mt-0.5 line-clamp-2 text-xs leading-5 text-muted" :title="item.acquisitionText">
                          {{ item.acquisitionText }}
                        </p>
                      </div>
                      <div class="flex shrink-0 flex-wrap items-center gap-1.5">
                        <span v-if="item.hasBasemap" class="inline-flex items-center gap-1 rounded-control bg-raised px-1.5 py-0.5 text-xs text-muted">
                          <MapPinned :size="12" aria-hidden="true" />
                          {{ t("底图", "Map") }}
                        </span>
                        <span v-if="item.hasText" class="inline-flex items-center rounded-control bg-raised px-1.5 py-0.5 text-xs text-muted">
                          <ListChecks :size="12" aria-hidden="true" />
                          {{ t("文字下落", "Text") }}
                        </span>
                        <UiButton v-if="item.hasBasemap" size="sm" variant="ghost" @click="openMap(item)">
                          {{ t("地图", "Map") }}
                        </UiButton>
                        <UiButton size="sm" variant="ghost" @click="openDetail(region.slug, item)">
                          {{ t("详情", "Details") }}
                        </UiButton>
                        <UiButton v-if="firstGuideLink(item)" size="sm" variant="ghost" @click="openGuide(item)">
                          <ExternalLink :size="14" aria-hidden="true" />
                          {{ guideLabel(item) }}
                        </UiButton>
                      </div>
                    </li>
                  </ul>
                </div>
              </section>
            </div>
          </section>
        </template>
      </template>
    </div>
  </section>
</template>
