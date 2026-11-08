<script setup lang="ts">
// 进度条（2026-11-05 界面美化阶段一）：区域 / 必收进度共用。
// - 常规 4px、主进度 6px（size="md"）；
// - 轨道 raised，主区域 accent，必收 gold-fill；
// - 数值钳制在 [0, total]，total ≤ 0 / NaN 时按 0% 处理；
// - role="progressbar" 暴露百分比与「已收集 / 总数」可访问文本。
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    value: number;
    total: number;
    /** 可访问名称，如「主区域收集进度」。 */
    label: string;
    tone?: "accent" | "gold";
    size?: "sm" | "md";
    /** 便于测试 / 定位。 */
    testId?: string;
  }>(),
  { tone: "accent", size: "sm", testId: undefined },
);

const safeTotal = computed(() => (Number.isFinite(props.total) && props.total > 0 ? props.total : 0));

const safeValue = computed(() => {
  if (!Number.isFinite(props.value) || safeTotal.value === 0) {
    return 0;
  }
  return Math.min(safeTotal.value, Math.max(0, props.value));
});

const percent = computed(() =>
  safeTotal.value === 0 ? 0 : (safeValue.value / safeTotal.value) * 100,
);

const valueText = computed(() => `${safeValue.value} / ${safeTotal.value}`);
</script>

<template>
  <div
    role="progressbar"
    :aria-label="label"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-valuenow="Math.round(percent)"
    :aria-valuetext="valueText"
    class="ui-progress"
    :class="{ 'ui-progress--md': size === 'md' }"
    :data-testid="testId"
  >
    <div
      class="ui-progress__fill"
      :class="{ 'ui-progress__fill--gold': tone === 'gold' }"
      :style="{ width: `${percent}%` }"
    />
  </div>
</template>
