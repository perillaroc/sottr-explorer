<script setup lang="ts">
// 空状态：说明“为什么没有内容”，与“未发现遗漏”等业务结论区分。
withDefaults(
  defineProps<{
    title: string;
    description?: string;
    tone?: "neutral" | "warning" | "danger";
  }>(),
  { description: undefined, tone: "neutral" },
);
</script>

<template>
  <div
    class="flex flex-col items-start gap-2 rounded-[8px] border border-dashed px-4 py-3 text-sm"
    :class="
      tone === 'danger'
        ? 'border-danger/50 bg-danger-bg text-danger'
        : tone === 'warning'
          ? 'border-caution/50 bg-caution-bg text-caution'
          : 'border-line bg-raised text-ink'
    "
  >
    <p class="font-medium">{{ title }}</p>
    <p v-if="description" class="text-[13px] text-muted">{{ description }}</p>
    <div v-if="$slots.default" class="flex flex-wrap items-center gap-2">
      <slot />
    </div>
  </div>
</template>
