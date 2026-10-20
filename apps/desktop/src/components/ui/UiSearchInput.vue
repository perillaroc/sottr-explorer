<script setup lang="ts">
// 搜索输入：受控组件；提供 focus() 供 Ctrl+F 使用；空值时显示清除按钮。
import { ref } from "vue";
import { X } from "@lucide/vue";
import { t } from "../../app/i18n";

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
    <input
      ref="input"
      type="search"
      :value="props.modelValue"
      :placeholder="props.placeholder ?? t('搜索', 'Search')"
      :aria-label="ariaLabel"
      :disabled="disabled"
      class="h-9 w-full rounded-[6px] border border-line bg-surface pr-8 pl-3 text-sm text-ink placeholder:text-muted disabled:opacity-60"
      @input="onInput"
    />
    <button
      v-if="props.modelValue"
      type="button"
      :aria-label="t(`清除搜索：${props.modelValue}`, `Clear search: ${props.modelValue}`)"
      :title="t('清除搜索', 'Clear search')"
      class="absolute right-1 inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-[6px] text-muted hover:bg-raised hover:text-ink"
      @click="clear"
    >
      <X :size="16" aria-hidden="true" />
    </button>
  </div>
</template>
