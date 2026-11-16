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

// 进度增量（P1-3）：当前槽位在本次会话中的多次分析。
const progressHistory = computed(() => store.progressHistoryForCurrent.value);
const progress = computed(() => store.progressComparison.value);
const progressLatest = computed(() => progressHistory.value[progressHistory.value.length - 1] ?? null);
/** 基线可来自其他槽位（用于槽位对比；不同输入文件会明确标为不可比）。 */
const progressBaselineChoices = computed(() =>
  store.progressBaselineOptions.value.map((snapshot) => ({
    id: snapshot.snapshotId,
    label: tFormat("{name} · {time}", "{name} · {time}", {
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
      return tFormat(
        "两次分析来自不同存档（{before} / {after}），不计算增量。",
        "The two analyses come from different saves ({before} / {after}); no delta is computed.",
        { before: delta.before.inputRef, after: delta.after.inputRef },
      );
    case "not_campaign":
      return t(
        "含非战役上下文（unknown / profile / unsupported），不计算增量。",
        "A non-campaign context is involved (unknown / profile / unsupported); no delta is computed.",
      );
    case "inventory_incomplete":
      return t(
        "物品表不完整（type28 降级），不推断新增或丢失。",
        "The inventory table is incomplete (type28 degradation); additions and losses are not inferred.",
      );
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

        <section v-if="store.planAvailable.value" class="mt-6" data-testid="progress-delta">
          <div class="mb-2 flex flex-wrap items-center gap-2">
            <h2 class="ui-section-title">{{ t("进度增量", "Progress delta") }}</h2>
            <UiBadge tone="info" :kind-label="t('范围', 'Scope')">{{ t("本次会话", "This session") }}</UiBadge>
          </div>

          <div class="ui-panel px-4 py-3">
            <p v-if="progressBaselineChoices.length === 0" class="text-[13px] text-muted" data-testid="progress-empty">
              {{ t("本会话还没有上一次分析。刷新或重新分析同一存档后，这里会显示与上一次的差异。", "No previous analysis in this session yet. Refresh or analyze the same save again to see the difference from the previous run.") }}
            </p>

            <template v-else>
              <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-[13px]">
                <label class="flex items-center gap-2 text-muted">
                  {{ t("对比基线", "Baseline") }}
                  <select
                    class="h-9 max-w-[260px] cursor-pointer rounded-control border border-line bg-surface px-2 text-[13px] text-ink"
                    :value="store.progressBaselineId.value ?? ''"
                    data-testid="progress-baseline"
                    @change="onBaselineChange"
                  >
                    <option v-if="store.progressBaselineId.value === null" value="">
                      {{ t("（选择一次分析）", "(choose an analysis)") }}
                    </option>
                    <option v-for="choice in progressBaselineChoices" :key="choice.id" :value="choice.id">
                      {{ choice.label }}
                    </option>
                  </select>
                </label>
                <span class="text-muted" data-testid="progress-latest">
                  {{ tFormat("→ 最新一次：{time}", "→ Latest: {time}", { time: formatSnapshotTime(progressLatest?.snapshotTime ?? null) }) }}
                </span>
              </div>

              <p
                v-if="progressHistory.length < 2 && !progress"
                class="mt-2 text-[13px] text-muted"
                data-testid="progress-single"
              >
                {{ t("同一存档只分析了一次；可选择其他槽位的快照作对照，或再次分析同一存档。", "This save has been analyzed once only. Pick another slot’s snapshot as a baseline, or analyze the same save again.") }}
              </p>

              <p v-if="progressStatusText" class="mt-2 text-[13px] text-caution" data-testid="progress-status">
                {{ progressStatusText }}
              </p>

              <template v-if="progress">
                <div class="mt-3 flex flex-wrap gap-x-8 gap-y-2" data-testid="progress-totals">
                  <div>
                    <p class="ui-meta">{{ t("主区域收集品", "Main-region collectibles") }}</p>
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
                    <p class="ui-meta">{{ t("其中必收", "Required among them") }}</p>
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
                      <p class="ui-meta">{{ t("新增条目", "Added items") }}</p>
                      <p class="mt-0.5 text-[15px] font-semibold tabular-nums text-success">{{ progress.added.length }}</p>
                    </div>
                    <div>
                      <p class="ui-meta">{{ t("不再出现", "No longer present") }}</p>
                      <p class="mt-0.5 text-[15px] font-semibold tabular-nums" :class="progress.removed.length > 0 ? 'text-caution' : 'text-muted'">
                        {{ progress.removed.length }}
                      </p>
                    </div>
                  </template>
                </div>

                <template v-if="progress.comparable">
                  <div v-if="progress.regions.length > 0" class="mt-3" data-testid="progress-regions">
                  <p class="text-xs font-medium text-muted">{{ t("区域增量", "Region deltas") }}</p>
                  <ul class="mt-1 space-y-1">
                    <li
                      v-for="region in progress.regions"
                      :key="region.slug"
                      class="flex flex-wrap items-baseline gap-x-2 text-[13px]"
                    >
                      <span class="text-ink">{{ region.label }}</span>
                      <span class="tabular-nums text-muted">{{ region.before }} → {{ region.after }}</span>
                      <span class="font-medium tabular-nums" :class="deltaClass(region.delta)">{{ signed(region.delta) }}</span>
                      <span v-if="region.kind === 'special'" class="text-xs text-muted">{{ t("（特殊区域）", "(special)") }}</span>
                    </li>
                  </ul>
                </div>

                <div class="mt-3 grid gap-3" :class="layout === 'wide' || layout === 'medium' ? 'grid-cols-2' : 'grid-cols-1'">
                  <details v-if="progress.added.length > 0" class="ui-disclosure" open data-testid="progress-added">
                    <summary class="flex cursor-pointer items-center gap-2 text-[13px]">
                      {{ tFormat("新增条目（{count}）", "Added items ({count})", { count: progress.added.length }) }}
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
                      {{ tFormat("不再出现（{count}）", "No longer present ({count})", { count: progress.removed.length }) }}
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
                      {{ t("“不再出现”可能是物品表读取降级或对比了不同存档；不据此判断收集丢失。", "“No longer present” may come from a degraded inventory read or a comparison across saves; it does not imply lost progress.") }}
                    </p>
                  </details>
                </div>
                </template>
              </template>
            </template>

            <p class="mt-2 text-xs text-muted">
              {{ t("历史只保留在本次会话内存中（条目 ID 与计数），不写入磁盘，也不含存档路径。", "History stays in this session’s memory (item IDs and counts). It is not written to disk and contains no save paths.") }}
            </p>
          </div>
        </section>

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
