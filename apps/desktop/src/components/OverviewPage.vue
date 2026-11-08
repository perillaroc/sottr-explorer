<script setup lang="ts">
// 概览（SX-022；2026-11-05 界面美化阶段三）：收集进度摘要 + 区域索引 + 入口。
// 区域进度是图鉴条目数之比，不是游戏内地图百分比（口径未闭合，见 research/game-data/notes/reconciliation.md）。
import { computed } from "vue";
import { ChevronDown } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
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

const contextText = computed(() => {
  if (slot.value?.result) {
    return tFormat(
      "当前文件：{name} · 快照 {time}",
      "Current file: {name} · snapshot {time}",
      {
        name: slot.value.name,
        time: formatSnapshotTime(slot.value.result.snapshot_time),
      },
    );
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
        :title="t('概览', 'Overview')"
        :context="contextText"
      >
        <template #actions>
          <UiButton
            v-if="report && store.planAvailable.value"
            variant="primary"
            data-testid="view-missing"
            @click="openAllUncollected"
          >
            {{ t("查看未收集清单", "View uncollected list") }}
          </UiButton>
        </template>
      </AppPageHeader>

      <UiEmptyState v-if="!report" :title="copy.noReport" :description="copy.noReportHelp">
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <section class="ui-panel grid" :class="summaryClass" data-testid="overview-summary">
          <div class="px-4 py-3">
            <p class="ui-meta">{{ t("主区域收集品", "Main-region collectibles") }}</p>
            <p v-if="store.planAvailable.value" class="mt-1 flex items-baseline gap-1" data-testid="collection-summary">
              <span class="ui-stat-large">{{ counts.collected }}</span>
              <span class="text-[18px] leading-[38px] text-muted">/ {{ counts.all }}</span>
            </p>
            <p v-else class="mt-1 text-[13px] text-muted" data-testid="collection-summary">
              {{ t("不适用", "Not applicable") }}
            </p>
            <p class="mt-0.5 ui-meta">{{ t("已收集", "Collected") }}</p>
          </div>

          <div class="px-4 py-3">
            <p class="ui-meta">{{ t("未收集", "Uncollected") }}</p>
            <p class="mt-1" data-testid="uncollected-value">
              <span v-if="store.planAvailable.value" class="ui-stat-large text-accent-strong">
                {{ counts.uncollected }} <span class="text-sm font-normal">{{ t("项", "items") }}</span>
              </span>
              <span v-else class="text-[13px] text-muted">{{ t("不适用", "Not applicable") }}</span>
            </p>
            <p class="mt-0.5 ui-meta">
              {{ t(`特殊区域另计 ${specialCounts.uncollected} 项`, `Special regions add ${specialCounts.uncollected}`) }}
            </p>
          </div>

          <div class="px-4 py-3">
            <p class="ui-meta">{{ t("游戏内完成度", "In-game completion") }}</p>
            <p class="mt-1 flex flex-wrap items-baseline gap-2">
              <span v-if="overallText" class="ui-stat-large" data-testid="overall-value">{{ overallText }}</span>
              <span v-else class="text-[13px]" data-testid="overall-value">
                {{ tFormat("不可用（{reason}）", "Unavailable ({reason})", { reason: overall?.reason ?? t("原因见诊断", "see Diagnostics for the reason") }) }}
              </span>
              <UiBadge tone="caution" :kind-label="t('证据', 'Evidence')">{{ t("推定", "Estimated") }}</UiBadge>
            </p>
            <p class="mt-0.5 ui-meta">{{ t("社区校准候选", "Community-calibrated candidate") }}</p>
          </div>
        </section>

        <UiEmptyState
          v-if="!store.planAvailable.value"
          class="mt-4"
          :title="t('资料文件没有收集品清单', 'Profile files have no collectible list')"
          :description="t('整体完成度仍按报告显示；收集品计数与区域索引不适用。', 'Overall completion still follows the report; collectible counts and the region index do not apply.')"
        >
          <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("选择存档文件", "Choose a save file") }}</UiButton>
        </UiEmptyState>

        <CoverageNotice
          v-if="store.isStale.value"
          class="mt-4"
          tone="warning"
          test-id="stale-banner"
          :title="copy.stale"
          :lines="[
            t(`快照时间：${formatSnapshotTime(slot?.result?.snapshot_time ?? null)}`, `Snapshot time: ${formatSnapshotTime(slot?.result?.snapshot_time ?? null)}`),
            t(`刷新错误：${slot?.result?.last_refresh_error?.code ?? '未知'}`, `Refresh error: ${slot?.result?.last_refresh_error?.code ?? 'unknown'}`),
            t('请刷新后再核对收集状态。', 'Refresh before checking collection status again.'),
          ]"
        />

        <section v-if="store.planAvailable.value" class="mt-6 space-y-3">
          <h2 class="ui-section-title">{{ t("区域索引", "Region index") }}</h2>
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
              {{ t(`特殊区域（${specialRegions.length}）`, `Special regions (${specialRegions.length})`) }}
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
          <UiButton @click="browseAll">{{ t("浏览全部收集品", "Browse all collectibles") }}</UiButton>
        </section>
      </template>
    </div>
  </section>
</template>
