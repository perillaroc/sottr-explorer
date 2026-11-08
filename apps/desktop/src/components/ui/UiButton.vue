<script setup lang="ts">
// 基础按钮（SX-015）：36px / 32px 高、6px 圆角；禁用时给出原因由调用方就地说明。
// 视觉令牌阶段（2026-11-05 界面美化）：主按钮 hover 使用 --accent-hover，
// 危险按钮使用 --danger-contrast / --danger-hover，保证明暗主题下文字对比度。
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
  "ui-interactive inline-flex items-center justify-center gap-1.5 rounded-control font-medium leading-5 select-none",
  props.size === "sm" ? "h-8 px-2.5 text-[13px]" : "h-9 px-3 text-[13px]",
  props.variant === "primary"
    ? "bg-accent text-accent-contrast enabled:hover:bg-accent-hover enabled:active:bg-accent-hover"
    : props.variant === "danger"
      ? "bg-danger text-danger-contrast enabled:hover:bg-danger-hover enabled:active:bg-danger-hover"
      : props.variant === "ghost"
        ? "text-ink enabled:hover:bg-raised enabled:active:bg-raised"
        : "border border-line bg-surface text-ink enabled:hover:bg-raised enabled:active:bg-raised",
  props.disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer",
]);
</script>

<template>
  <button :type="type" :disabled="disabled" :title="title" :class="classes">
    <slot />
  </button>
</template>
