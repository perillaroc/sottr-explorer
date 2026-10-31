<script setup lang="ts">
// 面板宽度调整手柄（2026-10-31 详情拖动）：贴在面板左边缘（宽 6px，向外扩 4px 热区）。
// 指针拖动、键盘 ←/→（Shift 加速）/Home/End、双击复位。
// 拖动过程中只发 `resize`（父级实时预览，不写偏好）；交互结束发 `commit`（父级写偏好）。
import { computed, onBeforeUnmount, ref } from "vue";

const props = defineProps<{
  /** 当前宽度（px）。 */
  width: number;
  min: number;
  /** 最大宽度：函数形式，调用时按视口/布局取值。 */
  max: () => number;
  /** 双击（或键盘复位）回到的宽度。 */
  defaultWidth: number;
  /** aria-label，如「调整详情宽度」。 */
  label: string;
}>();

const emit = defineEmits<{ resize: [number]; commit: [number] }>();

const dragging = ref(false);
let startX = 0;
let startWidth = 0;
let lastWidth = 0;
let previousUserSelect = "";
let activePointer: number | null = null;

const maxValue = computed(() => Math.max(props.min, Math.round(props.max())));
const currentValue = computed(() => clamp(props.width));

function clamp(value: number): number {
  if (!Number.isFinite(value)) {
    return props.defaultWidth;
  }
  return Math.min(maxValue.value, Math.max(props.min, Math.round(value)));
}

function releaseBody(): void {
  document.body.style.userSelect = previousUserSelect;
}

function beginDrag(event: PointerEvent): void {
  if (event.button !== 0 || dragging.value) {
    return;
  }
  event.preventDefault();
  dragging.value = true;
  startX = event.clientX;
  startWidth = clamp(props.width);
  lastWidth = startWidth;
  previousUserSelect = document.body.style.userSelect;
  // 拖动期间禁用文本选择，避免整页被“刷蓝”。
  document.body.style.userSelect = "none";
  const target = event.currentTarget as HTMLElement | null;
  if (target && typeof target.setPointerCapture === "function") {
    try {
      target.setPointerCapture(event.pointerId);
      activePointer = event.pointerId;
    } catch {
      activePointer = null;
    }
  }
}

function moveDrag(event: PointerEvent): void {
  if (!dragging.value) {
    return;
  }
  event.preventDefault();
  // 面板在右侧：指针左移 → 面板变宽。
  lastWidth = clamp(startWidth + (startX - event.clientX));
  emit("resize", lastWidth);
}

function endDrag(event: PointerEvent): void {
  if (!dragging.value) {
    return;
  }
  dragging.value = false;
  releaseBody();
  const target = event.currentTarget as HTMLElement | null;
  if (activePointer !== null && target && typeof target.releasePointerCapture === "function") {
    try {
      target.releasePointerCapture(activePointer);
    } catch {
      // 指针捕获已随指针抬起释放。
    }
  }
  activePointer = null;
  emit("resize", lastWidth);
  emit("commit", lastWidth);
}

function onKeydown(event: KeyboardEvent): void {
  const step = event.shiftKey ? 64 : 16;
  let next: number | null = null;
  if (event.key === "ArrowLeft") {
    next = currentValue.value + step;
  } else if (event.key === "ArrowRight") {
    next = currentValue.value - step;
  } else if (event.key === "Home") {
    next = props.min;
  } else if (event.key === "End") {
    next = maxValue.value;
  }
  if (next === null) {
    return;
  }
  event.preventDefault();
  const width = clamp(next);
  emit("resize", width);
  emit("commit", width);
}

function onDoubleClick(event: MouseEvent): void {
  event.preventDefault();
  const width = clamp(props.defaultWidth);
  emit("resize", width);
  emit("commit", width);
}

onBeforeUnmount(() => {
  if (dragging.value) {
    dragging.value = false;
    releaseBody();
  }
});
</script>

<template>
  <div
    role="separator"
    aria-orientation="vertical"
    :aria-label="label"
    :aria-valuemin="min"
    :aria-valuemax="maxValue"
    :aria-valuenow="currentValue"
    :aria-valuetext="`${currentValue}px`"
    tabindex="0"
    class="group absolute inset-y-0 -left-1 z-20 w-[10px] cursor-col-resize touch-none select-none"
    :class="dragging ? 'cursor-col-resize' : ''"
    @pointerdown="beginDrag"
    @pointermove="moveDrag"
    @pointerup="endDrag"
    @pointercancel="endDrag"
    @dblclick="onDoubleClick"
    @keydown="onKeydown"
  >
    <span
      class="pointer-events-none absolute inset-y-0 left-1 w-0.5 rounded-full bg-transparent transition-colors group-hover:bg-accent group-focus-visible:bg-accent"
      :class="dragging ? 'bg-accent' : ''"
      aria-hidden="true"
    />
  </div>
</template>
