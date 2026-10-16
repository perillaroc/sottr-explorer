<script setup lang="ts">
// 概览（SX-015）：一行摘要 + 区域入口；推论标签不丢失；覆盖提示固定可见。
import { computed } from "vue";
import { AlertTriangle, Info } from "@lucide/vue";
import { useStore } from "../app/inject";
import { copy, evidenceLabel } from "../app/labels";
import { formatPercent, formatSnapshotTime } from "../app/format";
import { supportedCategories, verifiedCoordinateCount } from "../app/catalog";
import RegionCounts from "./RegionCounts.vue";
import CoverageNotice from "./CoverageNotice.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";

const store = useStore();

const report = computed(() => store.report.value);
const slot = computed(() => store.currentSlot.value);
const overall = computed(() => report.value?.observations.overall_completion ?? null);
const overallText = computed(() => (overall.value?.value === null || overall.value === null ? null : formatPercent(overall.value.value)));
const missingAvailable = computed(() => store.missingAvailable.value);
const coordinateCount = computed(() => (report.value ? verifiedCoordinateCount(report.value) : 0));
const categories = computed(() => (report.value ? supportedCategories(store.metadata, report.value) : []));
const locationStatus = computed(() => report.value?.capabilities.location.status ?? "unsupported");
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="overview-title">
    <div class="mx-auto max-w-4xl space-y-4">
      <header class="space-y-1">
        <h1 id="overview-title" class="text-xl font-semibold">概览</h1>
        <p class="text-[13px] text-muted" data-testid="overview-context">
          <template v-if="slot && slot.result">
            当前文件：{{ slot.name }} · 快照 {{ formatSnapshotTime(slot.result.snapshot_time) }}
          </template>
          <template v-else>{{ copy.noReport }}</template>
        </p>
      </header>

      <UiEmptyState v-if="!report" :title="copy.noReport" :description="copy.noReportHelp">
        <UiButton variant="primary" @click="store.actions.setView('import')">打开存档</UiButton>
      </UiEmptyState>

      <template v-else>
        <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
          <div class="rounded-[8px] border border-line bg-surface px-3 py-2">
            <p class="text-xs text-muted">整体进度推定</p>
            <p class="mt-0.5 flex flex-wrap items-baseline gap-1.5" data-testid="overall-summary">
              <span v-if="overallText" class="text-lg font-semibold tabular-nums" data-testid="overall-value">{{ overallText }}</span>
              <span v-else class="text-[13px]" data-testid="overall-value">不可用（{{ overall?.reason ?? "原因见诊断" }}）</span>
              <UiBadge tone="caution" kind-label="证据">推定</UiBadge>
              <UiBadge tone="neutral" kind-label="依据">{{ overall ? evidenceLabel(overall.evidence_level) : "未知" }}</UiBadge>
            </p>
            <p class="mt-0.5 text-xs text-muted">全局字段的社区校准候选；未反映剩余比例构成。</p>
          </div>
          <div class="rounded-[8px] border border-line bg-surface px-3 py-2">
            <p class="text-xs text-muted">支持范围内参考遗漏</p>
            <p class="mt-0.5 flex flex-wrap items-baseline gap-1.5">
              <span class="text-lg font-semibold tabular-nums" data-testid="missing-value">{{ store.missingTotal.value }} 项</span>
              <UiBadge tone="caution" kind-label="状态">推定</UiBadge>
            </p>
            <p class="mt-0.5 text-xs text-muted">仅四个内部参考类别；不是“全游戏还差 N 项”。</p>
          </div>
          <div class="rounded-[8px] border border-line bg-surface px-3 py-2">
            <p class="text-xs text-muted">定位能力</p>
            <p class="mt-0.5 text-[13px]" data-testid="location-value">
              已验证坐标 {{ coordinateCount }} 项；区域/攻略组线索{{ locationStatus === "supported" ? "可用" : locationStatus === "partial" ? "部分可用" : "不可用" }}
            </p>
            <p class="mt-0.5 text-xs text-muted">不绘制精确点位；攻略组线索不冒充条目编号。</p>
          </div>
        </div>

        <CoverageNotice
          v-if="store.isStale.value"
          tone="warning"
          test-id="stale-banner"
          :title="copy.stale"
          :lines="[
            `快照时间：${formatSnapshotTime(slot?.result?.snapshot_time ?? null)}`,
            `刷新错误：${slot?.result?.last_refresh_error?.code ?? '未知'}：${slot?.result?.last_refresh_error?.message ?? ''}`,
            '旧证据可读，但不能当作当前遗漏；旧结果导出必须携带过期标记。',
          ]"
        />
        <CoverageNotice
          v-else-if="!missingAvailable"
          tone="warning"
          test-id="degraded-banner"
          :title="copy.missingUnavailable"
          :lines="[copy.missingUnavailableHelp, `原因码：${(report.coverage.unavailable_reason_codes ?? []).join('、') || '见诊断'}`]"
        />

        <!-- 覆盖提示在主流程中始终可见（与状态提示并存，不互相替代）。 -->
        <CoverageNotice tone="info" test-id="coverage-banner" :title="copy.coverage" :lines="[copy.coverageScope]">
          <span class="text-xs">受支持类别：{{ categories.map((category) => category.label).join("、") || "无" }}</span>
        </CoverageNotice>

        <section class="space-y-2">
          <h2 class="text-base font-medium">区域入口</h2>
          <RegionCounts :region="null" compact />
        </section>

        <section class="flex flex-wrap items-center gap-2">
          <UiButton
            variant="primary"
            :disabled="!missingAvailable"
            :title="missingAvailable ? '查看参考遗漏（应用遗漏筛选）' : `${copy.missingUnavailable}：${copy.missingUnavailableHelp}`"
            data-testid="view-missing"
            @click="store.actions.setStatusFilter('missing'); store.actions.setView('regions')"
          >
            查看参考遗漏
          </UiButton>
          <UiButton @click="store.actions.setStatusFilter('all'); store.actions.setView('regions')">
            浏览受支持的收集品
          </UiButton>
          <span v-if="!missingAvailable" class="flex items-center gap-1 text-xs text-caution">
            <AlertTriangle :size="14" aria-hidden="true" />
            遗漏入口已禁用：{{ copy.missingUnavailableHelp }}
          </span>
          <span v-else class="flex items-center gap-1 text-xs text-muted">
            <Info :size="14" aria-hidden="true" />
            未分类原始记录请在诊断页查看。
          </span>
        </section>
      </template>
    </div>
  </section>
</template>
