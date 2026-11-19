<script setup lang="ts">
// 搜索输入：36px 高、6px 圆角；受控组件，提供 focus() 供 Ctrl+F 使用；
// 空值时显示清除按钮，左侧搜索图标不挤压输入文字。
import { ref } from "vue";
import { Search, X } from "@lucide/vue";
import { t, tFormatPair } from "../../app/i18n";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    ariaLabel: string;
    disabled?: boolean;
  }>(),
  { placeholder: undefined, disabled: false },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const input = ref<HTMLInputElement | null>(null);

function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}

function clear(): void {
  emit("update:modelValue", "");
  input.value?.focus();
}

function focus(): void {
  input.value?.focus();
}

defineExpose({ focus });
</script>

<template>
  <div class="relative flex items-center">
    <Search :size="16" class="pointer-events-none absolute left-2.5 text-muted" aria-hidden="true" />
    <input
      ref="input"
      type="search"
      :value="props.modelValue"
      :placeholder="props.placeholder ?? t('uiSearchInput.search')"
      :aria-label="ariaLabel"
      :disabled="disabled"
      class="ui-interactive h-9 w-full rounded-control border border-line bg-surface pr-9 pl-9 text-[13px] leading-5 text-ink placeholder:text-muted enabled:hover:border-accent/50 disabled:opacity-60"
      @input="onInput"
    />
    <button
      v-if="props.modelValue"
      type="button"
      :aria-label="tFormatPair(`uiSearchInput.clear-search`, { v0: props.modelValue })"
      :title="t('uiSearchInput.clear-search-2')"
      class="ui-interactive absolute right-1 inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-[4px] text-muted enabled:hover:bg-raised enabled:hover:text-ink"
      @click="clear"
    >
      <X :size="16" aria-hidden="true" />
    </button>
  </div>
</template>
