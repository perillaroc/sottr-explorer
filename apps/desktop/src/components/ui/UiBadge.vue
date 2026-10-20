<script setup lang="ts">
// 文本徽标：形状/符号 + 文字，不只靠颜色区分（业务状态 / 依据 / 定位共用外观）。
import { computed } from "vue";
import { labelColon } from "../../app/i18n";

const props = withDefaults(
  defineProps<{
    tone?: "caution" | "info" | "neutral" | "success" | "danger" | "accent";
    /** 简短分类（如“状态”“依据”“定位”），供屏幕阅读器理解语义。 */
    kindLabel?: string;
  }>(),
  { tone: "neutral", kindLabel: undefined },
);

const glyph = computed(() => {
  switch (props.tone) {
    case "caution":
      return "◆";
    case "info":
      return "●";
    case "success":
      return "✓";
    case "danger":
      return "!";
    case "accent":
      return "▸";
    default:
      return "○";
  }
});

const classes = computed(() => {
  const base = "inline-flex items-center gap-1 rounded-[6px] border px-1.5 py-0.5 text-xs leading-5";
  switch (props.tone) {
    case "caution":
      return `${base} border-caution/40 bg-caution-bg text-caution`;
    case "info":
      return `${base} border-info/40 bg-info-bg text-info`;
    case "success":
      return `${base} border-success/40 bg-success-bg text-success`;
    case "danger":
      return `${base} border-danger/40 bg-danger-bg text-danger`;
    case "accent":
      return `${base} border-accent/40 bg-accent-bg text-accent-strong`;
    default:
      return `${base} border-line bg-raised text-muted`;
  }
});
</script>

<template>
  <span :class="classes">
    <span aria-hidden="true">{{ glyph }}</span>
    <span v-if="kindLabel" class="sr-only">{{ kindLabel }}{{ labelColon() }}</span>
    <slot />
  </span>
</template>
