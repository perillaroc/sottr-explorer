<script setup lang="ts">
// 覆盖/降级/过期提示：改变可用性的告警必须就地可见（不藏进详情）。
withDefaults(
  defineProps<{
    tone?: "info" | "warning" | "danger";
    title: string;
    lines?: string[];
    /** 便于测试/定位：默认 coverage-notice。 */
    testId?: string;
  }>(),
  { tone: "info", lines: () => [], testId: "coverage-notice" },
);
</script>

<template>
  <div
    class="rounded-[8px] border px-3 py-2 text-[13px]"
    :class="
      tone === 'danger'
        ? 'border-danger/40 bg-danger-bg text-danger'
        : tone === 'warning'
          ? 'border-caution/40 bg-caution-bg text-caution'
          : 'border-info/30 bg-info-bg text-info'
    "
    :data-testid="testId"
  >
    <p class="font-medium">{{ title }}</p>
    <ul v-if="lines.length" class="mt-1 space-y-0.5">
      <li v-for="line in lines" :key="line">{{ line }}</li>
    </ul>
    <div v-if="$slots.default" class="mt-2 flex flex-wrap items-center gap-2">
      <slot />
    </div>
  </div>
</template>
