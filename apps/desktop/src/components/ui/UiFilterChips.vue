<script setup lang="ts">
// 筛选按钮组（状态/类别等）：aria-pressed 表达选中，计数来自明确范围。
// variant="segmented" 用于状态筛选（同一浅底槽内分段）；variant="chips" 用于类别筛选。
import type { ChipOption } from "./types";

const props = withDefaults(
  defineProps<{
    options: ChipOption[];
    modelValue: string;
    ariaLabel: string;
    variant?: "chips" | "segmented";
  }>(),
  { variant: "chips" },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

function itemClass(option: ChipOption): string {
  const selected = props.modelValue === option.value;
  if (props.variant === "segmented") {
    return [
      "ui-interactive inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-[4px] border px-2.5 text-[13px] leading-5 disabled:cursor-not-allowed disabled:opacity-60",
      selected
        ? "border-line bg-surface font-medium text-ink"
        : "border-transparent text-muted enabled:hover:bg-surface/70 enabled:hover:text-ink",
    ].join(" ");
  }
  return [
    "ui-interactive inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-control border px-2.5 text-[13px] leading-5 disabled:cursor-not-allowed disabled:opacity-60",
    selected
      ? "border-accent bg-accent-bg font-medium text-accent-strong"
      : "border-line bg-surface text-ink enabled:hover:bg-raised",
  ].join(" ");
}
</script>

<template>
  <div
    role="group"
    :aria-label="ariaLabel"
    :class="
      props.variant === 'segmented'
        ? 'inline-flex flex-wrap items-center gap-0.5 rounded-control border border-line bg-raised p-0.5'
        : 'flex flex-wrap items-center gap-1'
    "
  >
    <button
      v-for="option in props.options"
      :key="option.value"
      type="button"
      :aria-pressed="props.modelValue === option.value"
      :disabled="option.disabled"
      :title="option.title"
      :class="itemClass(option)"
      @click="emit('update:modelValue', option.value)"
    >
      <span>{{ option.label }}</span>
      <span v-if="option.count !== undefined" class="text-xs tabular-nums text-muted">{{ option.count }}</span>
    </button>
  </div>
</template>
