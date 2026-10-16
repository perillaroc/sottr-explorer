<script setup lang="ts">
// 模态对话框（SX-016）：与 UiDrawer 相同的焦点规则——打开后焦点进入首个动作、
// 限制在对话框内、Esc 关闭、关闭后焦点回到触发元素；打开时锁定背景滚动。
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { X } from "@lucide/vue";
import UiIconButton from "./UiIconButton.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    /** 宽度类：导出对话框默认 520px，不超过视口减 24px。 */
    widthClass?: string;
  }>(),
  { widthClass: "w-[520px] max-w-[calc(100vw-24px)]" },
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
  // 父级用 v-if 卸载对话框时不会再触发 open 的 watch：统一在这里收尾。
  deactivate();
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-40 flex items-center justify-center bg-[var(--overlay)] p-3"
      @click.self="emit('close')"
    >
      <section
        ref="container"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        tabindex="-1"
        :class="[
          'flex max-h-[calc(100vh-48px)] flex-col rounded-[12px] border border-line bg-surface shadow-[var(--shadow)]',
          widthClass,
        ]"
        data-testid="export-dialog"
      >
        <header class="flex h-12 shrink-0 items-center justify-between gap-2 border-b border-line px-4">
          <h2 class="truncate text-base font-medium">{{ title }}</h2>
          <UiIconButton label="关闭（Esc）" @click="emit('close')">
            <X :size="16" aria-hidden="true" />
          </UiIconButton>
        </header>
        <div class="scroll-area min-h-0 flex-1 overflow-auto px-4 py-3">
          <slot />
        </div>
        <footer v-if="$slots.footer" class="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-line px-4 py-2.5">
          <slot name="footer" />
        </footer>
      </section>
    </div>
  </Teleport>
</template>
