<script setup lang="ts">
// 概览（SX-022）：收集进度摘要 + 各区域进度 + 入口。
// 区域进度是图鉴条目数之比，不是游戏内地图百分比（口径未闭合，见 research/game-data/notes/reconciliation.md）。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
import { copy } from "../app/labels";
import { formatPercent, formatSnapshotTime } from "../app/format";
import { countPlan, regionName } from "../app/collection";
import CoverageNotice from "./CoverageNotice.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";

const store = useStore();

const report = computed(() => store.report.value);
const slot = computed(() => store.currentSlot.value);
const plan = computed(() => store.plan.value);
const counts = computed(() => store.planMainCounts.value);
const specialCounts = computed(() => countPlan(store.plan.value, "special"));
const overall = computed(() => report.value?.observations.overall_completion ?? null);
const overallText = computed(() =>
  overall.value?.value === null || overall.value === null ? null : formatPercent(overall.value.value),
);

const mainRegions = computed(() => plan.value.regions.filter((region) => region.kind === "main"));
const specialRegions = computed(() => plan.value.regions.filter((region) => region.kind === "special"));

function percent(collected: number, total: number): number {
  return total === 0 ? 0 : Math.round((collected / total) * 100);
}

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
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-6" aria-labelledby="overview-title">
    <div class="mx-auto max-w-5xl space-y-4">
      <header class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h1 id="overview-title" class="text-lg font-semibold">{{ t("概览", "Overview") }}</h1>
        <p class="text-[13px] text-muted" data-testid="overview-context">
          <template v-if="slot && slot.result">
            {{ t(`当前文件：${slot.name} · 快照 ${formatSnapshotTime(slot.result.snapshot_time)}`, `Current file: ${slot.name} · snapshot ${formatSnapshotTime(slot.result.snapshot_time)}`) }}
          </template>
          <template v-else>{{ copy.noReport }}</template>
        </p>
      </header>

      <UiEmptyState v-if="!report" :title="copy.noReport" :description="copy.noReportHelp">
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <div class="rounded-[10px] border border-line bg-surface px-4 py-3">
            <p class="text-xs text-muted">{{ t("主区域收集品", "Main-region collectibles") }}</p>
            <p v-if="store.planAvailable.value" class="mt-1 flex items-baseline gap-1" data-testid="collection-summary">
              <span class="text-2xl font-semibold tabular-nums">{{ counts.collected }}</span>
              <span class="text-sm text-muted">/ {{ counts.all }}</span>
            </p>
            <p v-else class="mt-1 text-[13px] text-muted" data-testid="collection-summary">
              {{ t("不适用", "Not applicable") }}
            </p>
            <p class="mt-0.5 text-xs text-muted">{{ t("已收集", "Collected") }}</p>
          </div>
          <div class="rounded-[10px] border border-line bg-surface px-4 py-3">
            <p class="text-xs text-muted">{{ t("未收集", "Uncollected") }}</p>
            <p class="mt-1 flex items-baseline gap-2">
              <span v-if="store.planAvailable.value" class="text-2xl font-semibold tabular-nums" data-testid="uncollected-value">{{ counts.uncollected }} <span class="text-sm font-normal text-muted">{{ t("项", "items") }}</span></span>
              <span v-else class="text-[13px] text-muted" data-testid="uncollected-value">{{ t("不适用", "Not applicable") }}</span>
            </p>
            <p class="mt-0.5 text-xs text-muted">{{ t(`特殊区域另计 ${specialCounts.uncollected} 项`, `Special regions add ${specialCounts.uncollected}`) }}</p>
          </div>
          <div class="rounded-[10px] border border-line bg-surface px-4 py-3">
            <p class="text-xs text-muted">{{ t("游戏内完成度", "In-game completion") }}</p>
            <p class="mt-1 flex items-baseline gap-2">
              <span v-if="overallText" class="text-2xl font-semibold tabular-nums" data-testid="overall-value">{{ overallText }}</span>
              <span v-else class="text-[13px]" data-testid="overall-value">{{ tFormat("不可用（{reason}）", "Unavailable ({reason})", { reason: overall?.reason ?? t("原因见诊断", "see Diagnostics for the reason") }) }}</span>
              <UiBadge tone="caution" :kind-label="t('证据', 'Evidence')">{{ t("推定", "Estimated") }}</UiBadge>
            </p>
            <p class="mt-0.5 text-xs text-muted">{{ t("社区校准候选", "Community-calibrated candidate") }}</p>
          </div>
        </div>

        <CoverageNotice
          v-if="store.isStale.value"
          tone="warning"
          test-id="stale-banner"
          :title="copy.stale"
          :lines="[
            t(`快照时间：${formatSnapshotTime(slot?.result?.snapshot_time ?? null)}`, `Snapshot time: ${formatSnapshotTime(slot?.result?.snapshot_time ?? null)}`),
            t(`刷新错误：${slot?.result?.last_refresh_error?.code ?? '未知'}`, `Refresh error: ${slot?.result?.last_refresh_error?.code ?? 'unknown'}`),
            t('请刷新后再核对收集状态。', 'Refresh before checking collection status again.'),
          ]"
        />

        <section v-if="store.planAvailable.value" class="space-y-2">
          <h2 class="text-[13px] font-medium text-muted">{{ t("区域进度", "Region progress") }}</h2>
          <ul class="space-y-1.5" data-testid="region-progress">
            <li v-for="region in mainRegions" :key="region.slug">
              <button
                type="button"
                class="w-full cursor-pointer rounded-[8px] border border-line bg-surface px-3 py-1.5 text-left transition-colors hover:border-accent/60 hover:bg-raised/60"
                :data-testid="`overview-region-${region.slug}`"
                @click="openRegion(region.slug)"
              >
                <div class="flex items-baseline justify-between gap-2 text-[13px]">
                  <span class="text-ink">{{ regionName(region) }}</span>
                  <span class="tabular-nums text-muted">{{ region.collected }}/{{ region.total }}</span>
                </div>
                <div class="mt-1 h-1 w-full overflow-hidden rounded-full bg-raised">
                  <div class="h-full rounded-full bg-accent" :style="{ width: `${percent(region.collected, region.total)}%` }" />
                </div>
              </button>
            </li>
          </ul>

          <details class="pt-1">
            <summary class="cursor-pointer text-[13px] text-muted">
              {{ t(`特殊区域（${specialRegions.length}）`, `Special regions (${specialRegions.length})`) }}
            </summary>
            <ul class="mt-1 space-y-1">
              <li v-for="region in specialRegions" :key="region.slug" class="flex items-baseline gap-2 text-[13px]">
                <button
                  type="button"
                  class="cursor-pointer text-accent-strong hover:underline"
                  @click="openRegion(region.slug)"
                >
                  {{ regionName(region) }}
                </button>
                <span class="tabular-nums text-muted">{{ region.collected }}/{{ region.total }}</span>
              </li>
            </ul>
          </details>
        </section>

        <section class="flex flex-wrap items-center gap-2">
          <UiButton variant="primary" data-testid="view-missing" @click="openAllUncollected">
            {{ t("查看未收集清单", "View uncollected list") }}
          </UiButton>
          <UiButton @click="store.actions.setStatusFilter('all'); store.actions.setView('regions')">
            {{ t("浏览全部收集品", "Browse all collectibles") }}
          </UiButton>
        </section>
      </template>
    </div>
  </section>
</template>
