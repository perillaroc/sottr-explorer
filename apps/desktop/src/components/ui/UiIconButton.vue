<script setup lang="ts">
// 图标按钮：32×32 点击区、6px 圆角，必须提供可读名称（aria-label + title）。
withDefaults(
  defineProps<{
    label: string;
    disabled?: boolean;
    pressed?: boolean;
  }>(),
  { disabled: false, pressed: undefined },
);
</script>

<template>
  <button
    type="button"
    :aria-label="label"
    :title="label"
    :disabled="disabled"
    :aria-pressed="pressed"
    class="ui-interactive inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-control text-ink enabled:hover:bg-raised enabled:active:bg-raised disabled:cursor-not-allowed disabled:opacity-60"
  >
    <slot />
  </button>
</template>
