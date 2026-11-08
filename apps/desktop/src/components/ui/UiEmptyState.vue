<script setup lang="ts">
// 空状态：说明“为什么没有内容”，与“未发现遗漏”等业务结论区分。
// 视觉：可选 32px 线性图标 + 标题 + 说明 + 下一步操作，不使用大号灰盒。
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
  <div class="flex max-w-[72ch] flex-col items-start gap-2 py-6 text-sm">
    <span v-if="$slots.icon" class="text-muted" aria-hidden="true">
      <slot name="icon" />
    </span>
    <p
      class="ui-section-title"
      :class="tone === 'danger' ? 'text-danger' : tone === 'warning' ? 'text-caution' : 'text-ink'"
    >
      {{ title }}
    </p>
    <p v-if="description" class="text-[13px] leading-5 text-muted">{{ description }}</p>
    <div v-if="$slots.default" class="mt-1 flex flex-wrap items-center gap-2">
      <slot />
    </div>
  </div>
</template>
