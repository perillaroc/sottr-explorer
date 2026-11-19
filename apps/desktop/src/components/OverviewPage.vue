<script setup lang="ts">
// 概览（SX-022；2026-11-05 界面美化阶段三）：收集进度摘要 + 区域索引 + 入口。
// 区域进度是图鉴条目数之比，不是游戏内地图百分比（口径未闭合，见 research/game-data/notes/reconciliation.md）。
import { computed } from "vue";
import { ChevronDown } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat, tFormatPair } from "../app/i18n";
import { copy } from "../app/labels";
import { formatPercent, formatSnapshotTime } from "../app/format";
import { countPlan, regionName } from "../app/collection";
import { pagePaddingClass, regionGridClass, summaryGridClass } from "../app/layout";
import AppPageHeader from "./AppPageHeader.vue";
import CoverageNotice from "./CoverageNotice.vue";
import RegionProgressRow from "./RegionProgressRow.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";

const store = useStore();

const report = computed(() => store.report.value);
const slot = computed(() => store.currentSlot.value);
const plan = computed(() => store.plan.value);
const layout = computed(() => store.layout.value);
const counts = computed(() => store.planMainCounts.value);
const specialCounts = computed(() => countPlan(store.plan.value, "special"));
const overall = computed(() => report.value?.observations.overall_completion ?? null);
const overallText = computed(() =>
  overall.value?.value === null || overall.value === null ? null : formatPercent(overall.value.value),
);

const mainRegions = computed(() => plan.value.regions.filter((region) => region.kind === "main"));
const specialRegions = computed(() => plan.value.regions.filter((region) => region.kind === "special"));

const pageX = computed(() => pagePaddingClass(layout.value));
const summaryClass = computed(() => summaryGridClass(layout.value));
const regionGrid = computed(() => regionGridClass(layout.value));

// 进度增量（P1-3）：当前槽位在本次会话中的多次分析。
const progressHistory = computed(() => store.progressHistoryForCurrent.value);
const progress = computed(() => store.progressComparison.value);
const progressLatest = computed(() => progressHistory.value[progressHistory.value.length - 1] ?? null);
/** 基线可来自其他槽位（用于槽位对比；不同输入文件会明确标为不可比）。 */
const progressBaselineChoices = computed(() =>
  store.progressBaselineOptions.value.map((snapshot) => ({
    id: snapshot.snapshotId,
    label: tFormat("overviewPage.text",  {
      name: snapshot.inputRef,
      time: formatSnapshotTime(snapshot.snapshotTime),
    }),
  })),
);

const progressStatusText = computed(() => {
  const delta = progress.value;
  if (!delta) {
    return null;
  }
  switch (delta.status) {
    case "different_save":
      return tFormat("overviewPage.the-two-analyses-come-from-different-saves", 
        { before: delta.before.inputRef, after: delta.after.inputRef });
    case "not_campaign":
      return t("overviewPage.a-non-campaign-context-is-involved-unknown");
    case "inventory_incomplete":
      return t("overviewPage.the-inventory-table-is-incomplete-type28-degradation");
    default:
      return null;
  }
});

function signed(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}

function deltaClass(value: number): string {
  if (value > 0) {
    return "text-success";
  }
  return value < 0 ? "text-danger" : "text-muted";
}

function onBaselineChange(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  store.actions.setProgressBaseline(value === "" ? null : value);
}

const contextText = computed(() => {
  if (slot.value?.result) {
    return tFormat("overviewPage.current-file-snapshot", 
      {
        name: slot.value.name,
        time: formatSnapshotTime(slot.value.result.snapshot_time),
      });
  }
  return copy.noReport;
});

function openRegion(slug: string): void {
  store.actions.setRegion(slug);
  store.actions.setStatusFilter("missing");
  store.actions.setView("regions");
}

function openAllUncollected(): void {
  store.actions.setRegion("all");
  store.actions.setStatusFilter("missing");
  store.actions.setView("regions");
}

function browseAll(): void {
  store.actions.setStatusFilter("all");
  store.actions.setView("regions");
}
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="overview-title">
    <div class="mx-auto w-full max-w-[1120px] pb-8" :class="pageX">
      <AppPageHeader
        title-id="overview-title"
        :title="t('appSidebar.overview')"
        :context="contextText"
      >
        <template #actions>
          <UiButton
            v-if="report && store.planAvailable.value"
            data-testid="open-share-report"
            @click="store.actions.openShareReportDialog()"
          >
            {{ t("overviewPage.share-report") }}
          </UiButton>
          <UiButton
            v-if="report && store.planAvailable.value"
            variant="primary"
            data-testid="view-missing"
            @click="openAllUncollected"
          >
            {{ t("overviewPage.view-uncollected-list") }}
          </UiButton>
        </template>
      </AppPageHeader>

      <UiEmptyState v-if="!report" :title="copy.noReport" :description="copy.noReportHelp">
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("common.open-a-save") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <section class="ui-panel grid" :class="summaryClass" data-testid="overview-summary">
          <div class="px-4 py-3">
            <p class="ui-meta">{{ t("shareReport.main-region-collectibles") }}</p>
            <p v-if="store.planAvailable.value" class="mt-1 flex items-baseline gap-1" data-testid="collection-summary">
              <span class="ui-stat-large">{{ counts.collected }}</span>
              <span class="text-[18px] leading-[38px] text-muted">/ {{ counts.all }}</span>
            </p>
            <p v-else class="mt-1 text-[13px] text-muted" data-testid="collection-summary">
              {{ t("overviewPage.not-applicable") }}
            </p>
            <p class="mt-0.5 ui-meta">{{ t("common.collected") }}</p>
          </div>

          <div class="px-4 py-3">
            <p class="ui-meta">{{ t("common.uncollected") }}</p>
            <p class="mt-1" data-testid="uncollected-value">
              <span v-if="store.planAvailable.value" class="ui-stat-large text-accent-strong">
                {{ counts.uncollected }} <span class="text-sm font-normal">{{ t("overviewPage.items") }}</span>
              </span>
              <span v-else class="text-[13px] text-muted">{{ t("overviewPage.not-applicable") }}</span>
            </p>
            <p class="mt-0.5 ui-meta">
              {{ tFormatPair(`overviewPage.special-regions-add`, { v0: specialCounts.uncollected }) }}
            </p>
          </div>

          <div class="px-4 py-3">
            <p class="ui-meta">{{ t("shareReport.in-game-completion") }}</p>
            <p class="mt-1 flex flex-wrap items-baseline gap-2">
              <span v-if="overallText" class="ui-stat-large" data-testid="overall-value">{{ overallText }}</span>
              <span v-else class="text-[13px]" data-testid="overall-value">
                {{ tFormat("overviewPage.unavailable",  { reason: overall?.reason ?? t("overviewPage.see-diagnostics-for-the-reason") }) }}
              </span>
              <UiBadge tone="caution" :kind-label="t('overviewPage.evidence')">{{ t("overviewPage.estimated") }}</UiBadge>
            </p>
            <p class="mt-0.5 ui-meta">{{ t("overviewPage.community-calibrated-candidate") }}</p>
          </div>
        </section>

        <UiEmptyState
          v-if="!store.planAvailable.value"
          class="mt-4"
          :title="t('common.profile-files-have-no-collectible-list')"
          :description="t('overviewPage.overall-completion-still-follows-the-report-collectible')"
        >
          <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("common.choose-a-save-file") }}</UiButton>
        </UiEmptyState>

        <CoverageNotice
          v-if="store.isStale.value"
          class="mt-4"
          tone="warning"
          test-id="stale-banner"
          :title="copy.stale"
          :lines="[
            tFormatPair(`common.snapshot-time`, { v0: formatSnapshotTime(slot?.result?.snapshot_time ?? null) }),
            tFormatPair(`exportDialog.refresh-error`, { v0: slot?.result?.last_refresh_error?.code ?? '未知' }),
            t('common.refresh-before-checking-collection-status-again'),
          ]"
        />

        <section v-if="store.planAvailable.value" class="mt-6" data-testid="progress-delta">
          <div class="mb-2 flex flex-wrap items-center gap-2">
            <h2 class="ui-section-title">{{ t("overviewPage.progress-delta") }}</h2>
            <UiBadge tone="info" :kind-label="t('overviewPage.scope')">{{ t("overviewPage.this-session") }}</UiBadge>
          </div>

          <div class="ui-panel px-4 py-3">
            <p v-if="progressBaselineChoices.length === 0" class="text-[13px] text-muted" data-testid="progress-empty">
              {{ t("overviewPage.no-previous-analysis-in-this-session-yet") }}
            </p>

            <template v-else>
              <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-[13px]">
                <label class="flex items-center gap-2 text-muted">
                  {{ t("overviewPage.baseline") }}
                  <select
                    class="h-9 max-w-[260px] cursor-pointer rounded-control border border-line bg-surface px-2 text-[13px] text-ink"
                    :value="store.progressBaselineId.value ?? ''"
                    data-testid="progress-baseline"
                    @change="onBaselineChange"
                  >
                    <option v-if="store.progressBaselineId.value === null" value="">
                      {{ t("overviewPage.choose-an-analysis") }}
                    </option>
                    <option v-for="choice in progressBaselineChoices" :key="choice.id" :value="choice.id">
                      {{ choice.label }}
                    </option>
                  </select>
                </label>
                <span class="text-muted" data-testid="progress-latest">
                  {{ tFormat("overviewPage.latest",  { time: formatSnapshotTime(progressLatest?.snapshotTime ?? null) }) }}
                </span>
              </div>

              <p
                v-if="progressHistory.length < 2 && !progress"
                class="mt-2 text-[13px] text-muted"
                data-testid="progress-single"
              >
                {{ t("overviewPage.this-save-has-been-analyzed-once-only") }}
              </p>

              <p v-if="progressStatusText" class="mt-2 text-[13px] text-caution" data-testid="progress-status">
                {{ progressStatusText }}
              </p>

              <template v-if="progress">
                <div class="mt-3 flex flex-wrap gap-x-8 gap-y-2" data-testid="progress-totals">
                  <div>
                    <p class="ui-meta">{{ t("shareReport.main-region-collectibles") }}</p>
                    <p class="mt-0.5 text-[15px] tabular-nums">
                      {{ progress.before.mainCollected }} → {{ progress.after.mainCollected }}
                      <span
                        v-if="progress.comparable"
                        class="ml-1 font-semibold"
                        :class="deltaClass(progress.totalDelta)"
                      >{{ signed(progress.totalDelta) }}</span>
                    </p>
                  </div>
                  <div>
                    <p class="ui-meta">{{ t("common.required-among-them") }}</p>
                    <p class="mt-0.5 text-[15px] tabular-nums">
                      {{ progress.before.requiredCollected }} → {{ progress.after.requiredCollected }}
                      <span
                        v-if="progress.comparable"
                        class="ml-1 font-semibold"
                        :class="deltaClass(progress.requiredDelta)"
                      >{{ signed(progress.requiredDelta) }}</span>
                    </p>
                  </div>
                  <template v-if="progress.comparable">
                    <div>
                      <p class="ui-meta">{{ t("overviewPage.added-items") }}</p>
                      <p class="mt-0.5 text-[15px] font-semibold tabular-nums text-success">{{ progress.added.length }}</p>
                    </div>
                    <div>
                      <p class="ui-meta">{{ t("overviewPage.no-longer-present") }}</p>
                      <p class="mt-0.5 text-[15px] font-semibold tabular-nums" :class="progress.removed.length > 0 ? 'text-caution' : 'text-muted'">
                        {{ progress.removed.length }}
                      </p>
                    </div>
                  </template>
                </div>

                <template v-if="progress.comparable">
                  <div v-if="progress.regions.length > 0" class="mt-3" data-testid="progress-regions">
                  <p class="text-xs font-medium text-muted">{{ t("overviewPage.region-deltas") }}</p>
                  <ul class="mt-1 space-y-1">
                    <li
                      v-for="region in progress.regions"
                      :key="region.slug"
                      class="flex flex-wrap items-baseline gap-x-2 text-[13px]"
                    >
                      <span class="text-ink">{{ region.label }}</span>
                      <span class="tabular-nums text-muted">{{ region.before }} → {{ region.after }}</span>
                      <span class="font-medium tabular-nums" :class="deltaClass(region.delta)">{{ signed(region.delta) }}</span>
                      <span v-if="region.kind === 'special'" class="text-xs text-muted">{{ t("overviewPage.special") }}</span>
                    </li>
                  </ul>
                </div>

                <div class="mt-3 grid gap-3" :class="layout === 'wide' || layout === 'medium' ? 'grid-cols-2' : 'grid-cols-1'">
                  <details v-if="progress.added.length > 0" class="ui-disclosure" open data-testid="progress-added">
                    <summary class="flex cursor-pointer items-center gap-2 text-[13px]">
                      {{ tFormat("overviewPage.added-items-2",  { count: progress.added.length }) }}
                      <ChevronDown class="ui-chevron" :size="14" aria-hidden="true" />
                    </summary>
                    <ul class="mt-1 max-h-56 space-y-1 overflow-auto">
                      <li v-for="item in progress.added" :key="item.id" class="flex flex-wrap items-baseline gap-x-2 text-[13px]" data-testid="progress-added-item">
                        <span class="text-ink">{{ item.name }}</span>
                        <span class="text-xs text-muted">{{ item.categoryLabel }}</span>
                        <span class="text-xs text-muted">{{ item.regionLabel }}</span>
                        <span class="mono text-xs text-muted">{{ item.id }}</span>
                      </li>
                    </ul>
                  </details>

                  <details v-if="progress.removed.length > 0" class="ui-disclosure" data-testid="progress-removed">
                    <summary class="flex cursor-pointer items-center gap-2 text-[13px]">
                      {{ tFormat("overviewPage.no-longer-present-2",  { count: progress.removed.length }) }}
                      <ChevronDown class="ui-chevron" :size="14" aria-hidden="true" />
                    </summary>
                    <ul class="mt-1 max-h-56 space-y-1 overflow-auto">
                      <li v-for="item in progress.removed" :key="item.id" class="flex flex-wrap items-baseline gap-x-2 text-[13px]" data-testid="progress-removed-item">
                        <span class="text-ink">{{ item.name }}</span>
                        <span class="text-xs text-muted">{{ item.categoryLabel }}</span>
                        <span class="text-xs text-muted">{{ item.regionLabel }}</span>
                        <span class="mono text-xs text-muted">{{ item.id }}</span>
                      </li>
                    </ul>
                    <p class="mt-1 text-xs text-muted">
                      {{ t("overviewPage.no-longer-present-may-come-from") }}
                    </p>
                  </details>
                </div>
                </template>
              </template>
            </template>

            <p class="mt-2 text-xs text-muted">
              {{ t("overviewPage.history-stays-in-this-session-s-memory") }}
            </p>
          </div>
        </section>

        <section v-if="store.planAvailable.value" class="mt-6 space-y-3">
          <h2 class="ui-section-title">{{ t("overviewPage.region-index") }}</h2>
          <div class="grid gap-3" :class="regionGrid" data-testid="region-progress">
            <RegionProgressRow
              v-for="region in mainRegions"
              :key="region.slug"
              :slug="region.slug"
              :name="regionName(region)"
              :collected="region.collected"
              :total="region.total"
              :test-id="`overview-region-${region.slug}`"
              @open="openRegion"
            />
          </div>

          <details class="ui-disclosure pt-1">
            <summary class="flex cursor-pointer items-center gap-2 text-[13px] text-muted">
              {{ tFormatPair(`overviewPage.special-regions`, { v0: specialRegions.length }) }}
              <ChevronDown class="ui-chevron" :size="14" aria-hidden="true" />
            </summary>
            <div class="mt-2 grid gap-3" :class="regionGrid">
              <RegionProgressRow
                v-for="region in specialRegions"
                :key="region.slug"
                :slug="region.slug"
                :name="regionName(region)"
                :collected="region.collected"
                :total="region.total"
                :test-id="`overview-special-${region.slug}`"
                @open="openRegion"
              />
            </div>
          </details>
        </section>

        <section v-if="store.planAvailable.value" class="mt-6 flex flex-wrap items-center gap-2">
          <UiButton @click="browseAll">{{ t("overviewPage.browse-all-collectibles") }}</UiButton>
        </section>
      </template>
    </div>
  </section>
</template>
