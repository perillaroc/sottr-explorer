<script setup lang="ts">
// 模态抽屉（SX-015）：焦点进入首个动作、焦点限制在抽屉内、Esc 关闭、
// 关闭后焦点回到触发条目；打开时锁定背景滚动。
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { X } from "@lucide/vue";
import { t } from "../../app/i18n";
import UiIconButton from "./UiIconButton.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    /** 宽度类（未提供 `width` 时使用）：右侧抽屉默认 400px。 */
    widthClass?: string;
    /** 动态宽度（px）：提供时用内联宽度，仍保留响应式 max-width（不再依赖固定宽度类）。 */
    width?: number;
  }>(),
  { widthClass: "w-[400px] max-w-[calc(100vw-24px)]", width: undefined },
);

const emit = defineEmits<{ close: [] }>();

const container = ref<HTMLElement | null>(null);
let lastFocused: HTMLElement | null = null;
let scrollLocked = false;

function focusableElements(): HTMLElement[] {
  if (!container.value) {
    return [];
  }
  return Array.from(
    container.value.querySelectorAll<HTMLElement>(
      'summary, a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  );
}

function focusFirst(): void {
  const elements = focusableElements();
  (elements[0] ?? container.value)?.focus();
}

function onKeydown(event: KeyboardEvent): void {
  if (!props.open) {
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    emit("close");
    return;
  }
  if (event.key !== "Tab") {
    return;
  }
  const elements = focusableElements();
  if (elements.length === 0) {
    event.preventDefault();
    container.value?.focus();
    return;
  }
  const first = elements[0];
  const last = elements[elements.length - 1];
  const active = document.activeElement as HTMLElement | null;
  if (!active || !container.value?.contains(active)) {
    event.preventDefault();
    first.focus();
    return;
  }
  if (event.shiftKey && active === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}

function activate(): void {
  lastFocused = (document.activeElement as HTMLElement | null) ?? null;
  if (!scrollLocked) {
    document.body.style.overflow = "hidden";
    scrollLocked = true;
  }
  document.addEventListener("keydown", onKeydown, true);
  void nextTick(focusFirst);
}

function deactivate(): void {
  document.removeEventListener("keydown", onKeydown, true);
  if (scrollLocked) {
    document.body.style.overflow = "";
    scrollLocked = false;
  }
  lastFocused?.focus?.();
  lastFocused = null;
}

watch(
  () => props.open,
  (open, wasOpen) => {
    if (open && !wasOpen) {
      activate();
    } else if (!open && wasOpen) {
      deactivate();
    }
  },
);

onMounted(() => {
  if (props.open) {
    activate();
  }
});

onBeforeUnmount(() => {
  // 父级用 v-if 卸载抽屉时不会再触发 open 的 watch：这里统一收尾（焦点恢复、
  // 事件移除、滚动解锁），与 deactivate 幂等。
  deactivate();
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-40 flex justify-end bg-[var(--overlay)]"
      @click.self="emit('close')"
    >
      <section
        ref="container"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        tabindex="-1"
        :class="[
          'relative flex h-full min-w-0 flex-col border-l border-line bg-surface shadow-[var(--shadow)]',
          width === undefined ? widthClass : 'max-w-[calc(100vw-24px)]',
        ]"
        :style="width === undefined ? undefined : { width: `${width}px` }"
      >
        <header class="flex h-12 shrink-0 items-center justify-between gap-2 border-b border-line px-4">
          <h2 class="truncate text-base font-medium">{{ title }}</h2>
          <UiIconButton :label="t('关闭详情（Esc）', 'Close details (Esc)')" @click="emit('close')">
            <X :size="16" aria-hidden="true" />
          </UiIconButton>
        </header>
        <!-- 调整宽度手柄（阶段一）：绝对定位在抽屉左边缘，放在头部之后使焦点先进入关闭按钮。 -->
        <slot name="handle" />
        <div class="scroll-area min-h-0 flex-1 px-4 py-3">
          <slot />
        </div>
      </section>
    </div>
  </Teleport>
</template>
