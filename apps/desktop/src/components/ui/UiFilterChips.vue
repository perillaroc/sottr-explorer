<script setup lang="ts">
// 筛选按钮组（状态/类别等）：aria-pressed 表达选中，计数来自明确范围。
import type { ChipOption } from "./types";

const props = withDefaults(
  defineProps<{
    options: ChipOption[];
    modelValue: string;
    ariaLabel: string;
  }>(),
  {},
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <div role="group" :aria-label="ariaLabel" class="flex flex-wrap items-center gap-1">
    <button
      v-for="option in props.options"
      :key="option.value"
      type="button"
      :aria-pressed="props.modelValue === option.value"
      :disabled="option.disabled"
      :title="option.title"
      class="inline-flex h-8 cursor-pointer items-center gap-1 rounded-[6px] border px-2.5 text-[13px] disabled:cursor-not-allowed disabled:opacity-60"
      :class="
        props.modelValue === option.value
          ? 'border-accent bg-accent-bg text-accent'
          : 'border-line bg-surface text-ink hover:bg-raised'
      "
      @click="emit('update:modelValue', option.value)"
    >
      <span>{{ option.label }}</span>
      <span v-if="option.count !== undefined" class="text-xs text-muted">{{ option.count }}</span>
    </button>
  </div>
</template>
