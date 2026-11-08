<script setup lang="ts">
// 覆盖/降级/过期提示：改变可用性的告警必须就地可见（不藏进详情）。
// 视觉：16px 状态图标 + 标题 + 说明/动作；错误不只靠红色边框表达。
import { computed } from "vue";
import { CircleAlert, Info, TriangleAlert } from "@lucide/vue";

const props = withDefaults(
  defineProps<{
    tone?: "info" | "warning" | "danger";
    title: string;
    lines?: string[];
    /** 便于测试/定位：默认 coverage-notice。 */
    testId?: string;
  }>(),
  { tone: "info", lines: () => [], testId: "coverage-notice" },
);

const icon = computed(() =>
  props.tone === "danger" ? CircleAlert : props.tone === "warning" ? TriangleAlert : Info,
);

const classes = computed(() =>
  props.tone === "danger"
    ? "border-danger/40 bg-danger-bg text-danger"
    : props.tone === "warning"
      ? "border-caution/40 bg-caution-bg text-caution"
      : "border-info/30 bg-info-bg text-info",
);
</script>

<template>
  <div
    class="rounded-section border px-3 py-2.5 text-[13px]"
    :class="classes"
    :data-testid="testId"
  >
    <div class="flex items-start gap-2">
      <component :is="icon" :size="16" :stroke-width="2" class="mt-0.5 shrink-0" aria-hidden="true" />
      <div class="min-w-0 flex-1">
        <p class="font-medium">{{ title }}</p>
        <ul v-if="lines.length" class="mt-1 space-y-0.5 text-ink">
          <li v-for="line in lines" :key="line">{{ line }}</li>
        </ul>
        <div v-if="$slots.default" class="mt-2 flex flex-wrap items-center gap-2">
          <slot />
        </div>
      </div>
    </div>
  </div>
</template>
