<script setup lang="ts">
// 基础按钮（SX-015）：36px 高、6px 圆角；禁用时给出原因由调用方就地说明。
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "md" | "sm";
    disabled?: boolean;
    type?: "button" | "submit";
    title?: string;
  }>(),
  { variant: "secondary", size: "md", disabled: false, type: "button", title: undefined },
);

const classes = computed(() => [
  "inline-flex items-center justify-center gap-1.5 rounded-[6px] font-normal transition-colors select-none",
  props.size === "sm" ? "h-8 px-2.5 text-[13px]" : "h-9 px-3 text-sm",
  props.variant === "primary"
    ? "bg-accent text-accent-contrast hover:bg-accent-strong"
    : props.variant === "danger"
      ? "bg-danger text-white hover:opacity-90"
      : props.variant === "ghost"
        ? "text-ink hover:bg-raised"
        : "border border-line bg-surface text-ink hover:bg-raised",
  props.disabled ? "cursor-not-allowed opacity-60 hover:bg-surface" : "cursor-pointer",
]);
</script>

<template>
  <button :type="type" :disabled="disabled" :title="title" :class="classes">
    <slot />
  </button>
</template>
