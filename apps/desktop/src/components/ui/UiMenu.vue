<script setup lang="ts">
// 下拉菜单（SX-015）：Esc 关闭并回到触发按钮；点击外部关闭；禁用项给出原因。
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import UiButton from "./UiButton.vue";
import type { MenuItem } from "./types";

const props = withDefaults(
  defineProps<{
    label: string;
    items: MenuItem[];
    align?: "start" | "end";
  }>(),
  { align: "end" },
);

const emit = defineEmits<{ select: [id: string] }>();

const open = ref(false);
const trigger = ref<{ $el?: HTMLElement } | null>(null);
const popup = ref<HTMLElement | null>(null);

function toggle(): void {
  open.value = !open.value;
}

function close(refocus = true): void {
  if (!open.value) {
    return;
  }
  open.value = false;
  if (refocus) {
    trigger.value?.$el?.focus?.();
  }
}

function onSelect(item: MenuItem): void {
  if (item.disabled) {
    return;
  }
  emit("select", item.id);
  close();
}

function onPointerDown(event: PointerEvent): void {
  const target = event.target as Node | null;
  if (target && (popup.value?.contains(target) || trigger.value?.$el?.contains(target))) {
    return;
  }
  close(false);
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    close();
  }
}

watch(open, (value) => {
  if (value) {
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeydown, true);
    void nextTick(() => {
      popup.value?.querySelector<HTMLElement>("button:not([disabled])")?.focus();
    });
  } else {
    document.removeEventListener("pointerdown", onPointerDown, true);
    document.removeEventListener("keydown", onKeydown, true);
  }
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onPointerDown, true);
  document.removeEventListener("keydown", onKeydown, true);
});

defineExpose({ close, toggle });
</script>

<template>
  <div class="relative">
    <UiButton
      ref="trigger"
      size="sm"
      :title="label"
      aria-haspopup="menu"
      :aria-expanded="open"
      @click="toggle"
    >
      <slot name="trigger">{{ label }}</slot>
    </UiButton>
    <div
      v-if="open"
      ref="popup"
      role="menu"
      :aria-label="label"
      class="absolute z-30 mt-1 min-w-48 rounded-[8px] border border-line bg-surface p-1 shadow-[var(--shadow)]"
      :class="align === 'end' ? 'right-0' : 'left-0'"
    >
      <button
        v-for="item in props.items"
        :key="item.id"
        type="button"
        role="menuitem"
        :disabled="item.disabled"
        :title="item.disabled ? item.reason : undefined"
        class="block w-full cursor-pointer rounded-[6px] px-2 py-1.5 text-left text-sm text-ink hover:bg-raised disabled:cursor-not-allowed disabled:opacity-60"
        @click="onSelect(item)"
      >
        {{ item.label }}
        <span v-if="item.disabled && item.reason" class="block text-xs text-muted">{{ item.reason }}</span>
      </button>
    </div>
  </div>
</template>
