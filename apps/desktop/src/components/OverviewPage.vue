<script setup lang="ts">
// 概览（SX-022）：收集进度摘要 + 各区域进度 + 入口。
// 区域进度是图鉴条目数之比，不是游戏内地图百分比（口径未闭合，见 research/s020）。
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
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="overview-title">
    <div class="mx-auto max-w-4xl space-y-4">
      <header class="space-y-1">
        <h1 id="overview-title" class="text-xl font-semibold">{{ t("概览", "Overview") }}</h1>
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
        <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
          <div class="rounded-[8px] border border-line bg-surface px-3 py-2">
            <p class="text-xs text-muted">{{ t("主区域收集品", "Main-region collectibles") }}</p>
            <p v-if="store.planAvailable.value" class="mt-0.5 flex flex-wrap items-baseline gap-1.5" data-testid="collection-summary">
              <span class="text-lg font-semibold tabular-nums">{{ counts.collected }}/{{ counts.all }}</span>
              <span class="text-[13px] text-muted">{{ t("已收集", "collected") }}</span>
            </p>
            <p v-else class="mt-0.5 text-[13px] text-muted" data-testid="collection-summary">
              {{ t("不适用（资料文件没有收集品清单）", "Not applicable (profile files have no collectible list)") }}
            </p>
            <p class="mt-0.5 text-xs text-muted">
              {{ t("9 个主区域合计（不含特殊区域）；含可选与未闭合类别。", "Total across 9 main regions (excluding special regions); includes optional and unresolved categories.") }}
            </p>
          </div>
          <div class="rounded-[8px] border border-line bg-surface px-3 py-2">
            <p class="text-xs text-muted">{{ t("未收集", "Uncollected") }}</p>
            <p class="mt-0.5 flex flex-wrap items-baseline gap-1.5">
              <span v-if="store.planAvailable.value" class="text-lg font-semibold tabular-nums" data-testid="uncollected-value">{{ counts.uncollected }} {{ t("项", "items") }}</span>
              <span v-else class="text-[13px] text-muted" data-testid="uncollected-value">{{ t("不适用", "Not applicable") }}</span>
              <UiBadge tone="caution" :kind-label="t('状态', 'Status')">{{ t("待收集", "To collect") }}</UiBadge>
            </p>
            <p class="mt-0.5 text-xs text-muted">
              {{ t(`按区域与类别分组，见「区域与收集」；特殊区域另计 ${specialCounts.uncollected} 项。`, `Grouped by region and category in “Regions & Collection”; special regions add ${specialCounts.uncollected} items.`) }}
            </p>
          </div>
          <div class="rounded-[8px] border border-line bg-surface px-3 py-2">
            <p class="text-xs text-muted">{{ t("游戏内整体完成度字段", "In-game overall completion field") }}</p>
            <p class="mt-0.5 flex flex-wrap items-baseline gap-1.5">
              <span v-if="overallText" class="text-lg font-semibold tabular-nums" data-testid="overall-value">{{ overallText }}</span>
              <span v-else class="text-[13px]" data-testid="overall-value">{{ tFormat("不可用（{reason}）", "Unavailable ({reason})", { reason: overall?.reason ?? t("原因见诊断", "see Diagnostics for the reason") }) }}</span>
              <UiBadge tone="caution" :kind-label="t('证据', 'Evidence')">{{ t("推定", "Estimated") }}</UiBadge>
            </p>
            <p class="mt-0.5 text-xs text-muted">
              {{ t("社区校准候选，与本工具的收集品计数口径不同，不能互相换算。", "A community-calibrated candidate; its basis differs from this tool's collectible counts and the two cannot be converted.") }}
            </p>
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
            t('旧结果只反映上一次读取；请刷新后再核对收集状态。', 'The previous result reflects only the last read; refresh before checking collection status again.'),
          ]"
        />

        <CoverageNotice tone="info" test-id="coverage-banner" :title="t('收集状态口径', 'Collection status basis')" :lines="[copy.coverageScope]">
          <span class="text-xs">
            {{ t(`图鉴 ${plan.catalog_version} · 共 ${plan.main_total} 项主区域收集品 · 目录外观察记录 ${plan.observed_not_in_catalog.length} 个（不计入收集品）`, `Catalog ${plan.catalog_version} · ${plan.main_total} main-region collectibles · ${plan.observed_not_in_catalog.length} off-catalog observed records (not counted as collectibles)`) }}
          </span>
        </CoverageNotice>

        <section v-if="store.planAvailable.value" class="space-y-2">
          <h2 class="text-base font-medium">{{ t("区域进度", "Region progress") }}</h2>
          <p class="text-xs text-muted">
            {{ t("进度 = 本区域已收集 / 图鉴条目数；不是游戏内地图完成度百分比。", "Progress = collected in this region / catalog item count; it is not the in-game map completion percentage.") }}
          </p>
          <ul class="space-y-1.5" data-testid="region-progress">
            <li v-for="region in mainRegions" :key="region.slug">
              <button
                type="button"
                class="w-full cursor-pointer rounded-[6px] border border-line bg-surface px-3 py-1.5 text-left hover:bg-raised"
                :data-testid="`overview-region-${region.slug}`"
                @click="openRegion(region.slug)"
              >
                <div class="flex items-baseline justify-between gap-2 text-[13px]">
                  <span class="text-ink">{{ regionName(region) }}</span>
                  <span class="tabular-nums text-muted">{{ region.collected }}/{{ region.total }}</span>
                </div>
                <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-raised">
                  <div class="h-full rounded-full bg-accent-strong" :style="{ width: `${percent(region.collected, region.total)}%` }" />
                </div>
              </button>
            </li>
          </ul>

          <details class="pt-1">
            <summary class="cursor-pointer text-[13px] text-muted">
              {{ t(`特殊区域（${specialRegions.length}）：跨区域挑战古墓 / 全局区域 / 庄园`, `Special regions (${specialRegions.length}): cross-region challenge tombs / global region / manor`) }}
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
