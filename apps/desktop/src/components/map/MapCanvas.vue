<script setup lang="ts">
// 可交互地图（2026-10-31 地图查询与详情增强·阶段二）：
// 底图 + 按类别着色的收集品标记；已收集实心、未收集描边；选中外圈并置顶。
// 缩放 1×–4×（滚轮以指针为锚点、按钮、双击）、缩放后拖拽平移；标记不随缩放变大。
// 键盘/读屏的等价路径是「清单」标签页：标记不进入 Tab 序列（tabindex=-1）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { RotateCcw, ZoomIn, ZoomOut } from "@lucide/vue";
import { t, tFormat } from "../../app/i18n";
import { categoryClass, categoryDotClass } from "../../app/category-style";
import { categoryIcon } from "../../app/category-icon";
import { categoryLabel } from "../../app/labels";
import type { MapEntry } from "../../app/map-view";
import { basemapImageUrl, projectToImage, type Basemap } from "../../data/basemaps";

const props = defineProps<{
  basemap: Basemap;
  /** 当前筛选后要绘制的条目。 */
  entries: MapEntry[];
  selectedId: string | null;
  /** 区域 · 子关卡，用于无障碍概述。 */
  label: string;
}>();

const emit = defineEmits<{ select: [id: string | null] }>();

const MIN_ZOOM = 1;
const MAX_ZOOM = 4;
const DRAG_THRESHOLD = 4;

const container = ref<HTMLElement | null>(null);
const imageElement = ref<HTMLImageElement | null>(null);
const viewport = ref({ width: 0, height: 0 });
const zoom = ref(1);
const pan = ref({ x: 0, y: 0 });
const imageFailed = ref(false);

let observer: ResizeObserver | null = null;

const imageUrl = computed(() => basemapImageUrl(props.basemap));
const usable = computed(
  () => imageUrl.value !== null && !imageFailed.value && props.basemap.status !== "unverified",
);

const errorText = computed(() => {
  if (props.basemap.status === "unverified") {
    return t("该子关卡底图未经验证，地图视图不显示。", "The basemap for this sub-level is unverified, so it is not shown.");
  }
  if (!imageUrl.value) {
    return t(
      "底图素材未随本次构建配置，地图视图不可用。",
      "Basemap images are not configured in this build, so the map view is unavailable.",
    );
  }
  return t("底图加载失败。", "The basemap image failed to load.");
});

const fit = computed(() => {
  const { width, height } = viewport.value;
  const imageWidth = props.basemap.image_width;
  const imageHeight = props.basemap.image_height;
  if (!width || !height || imageWidth <= 0 || imageHeight <= 0) {
    return 0;
  }
  return Math.min(width / imageWidth, height / imageHeight);
});

const canvasWidth = computed(() => props.basemap.image_width * fit.value);
const canvasHeight = computed(() => props.basemap.image_height * fit.value);

const layerStyle = computed(() => ({
  width: `${canvasWidth.value}px`,
  height: `${canvasHeight.value}px`,
  transform: `translate(${pan.value.x}px, ${pan.value.y}px) scale(${zoom.value})`,
  transformOrigin: "0 0",
}));

function clampPan(point: { x: number; y: number }, scale = zoom.value): { x: number; y: number } {
  const vw = viewport.value.width;
  const vh = viewport.value.height;
  const scaledWidth = canvasWidth.value * scale;
  const scaledHeight = canvasHeight.value * scale;
  const axis = (value: number, scaled: number, available: number): number => {
    if (scaled <= available) {
      return (available - scaled) / 2;
    }
    return Math.min(0, Math.max(available - scaled, value));
  };
  return { x: axis(point.x, scaledWidth, vw), y: axis(point.y, scaledHeight, vh) };
}

function centerPan(scale = zoom.value): { x: number; y: number } {
  return {
    x: (viewport.value.width - canvasWidth.value * scale) / 2,
    y: (viewport.value.height - canvasHeight.value * scale) / 2,
  };
}

function resetView(): void {
  zoom.value = MIN_ZOOM;
  pan.value = clampPan(centerPan(MIN_ZOOM), MIN_ZOOM);
}

function measure(): void {
  const element = container.value;
  if (!element) {
    return;
  }
  const rect = element.getBoundingClientRect();
  viewport.value = { width: rect.width, height: rect.height };
}

onMounted(() => {
  measure();
  if (typeof ResizeObserver !== "undefined" && container.value) {
    observer = new ResizeObserver(() => measure());
    observer.observe(container.value);
  }
  resetView();
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  // 断开 src，让解码后的位图可回收。
  imageElement.value?.removeAttribute("src");
});

// 容器尺寸变化：重算 fit，保持相对位置并钳制平移范围。
watch(fit, (value, previous) => {
  if (!value) {
    return;
  }
  if (!previous) {
    resetView();
    return;
  }
  pan.value = clampPan(pan.value);
});

// 切换底图：重置缩放/平移与加载失败状态。
watch(
  () => props.basemap.basemap_id,
  () => {
    imageFailed.value = false;
    resetView();
  },
);

function onImageError(event: Event): void {
  const target = event.target as HTMLImageElement | null;
  if (!target || target.getAttribute("src") !== imageUrl.value) {
    return;
  }
  imageFailed.value = true;
}

function applyZoom(next: number, anchorX: number, anchorY: number): void {
  const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next));
  if (clamped === zoom.value) {
    pan.value = clampPan(pan.value);
    return;
  }
  const ratio = clamped / zoom.value;
  const current = pan.value;
  pan.value = clampPan(
    {
      x: anchorX - (anchorX - current.x) * ratio,
      y: anchorY - (anchorY - current.y) * ratio,
    },
    clamped,
  );
  zoom.value = clamped;
}

function onWheel(event: WheelEvent): void {
  if (!usable.value || !fit.value) {
    return;
  }
  event.preventDefault();
  const rect = container.value?.getBoundingClientRect();
  if (!rect) {
    return;
  }
  const factor = event.deltaY < 0 ? 1.15 : 1 / 1.15;
  applyZoom(zoom.value * factor, event.clientX - rect.left, event.clientY - rect.top);
}

function zoomBy(factor: number): void {
  if (!fit.value) {
    return;
  }
  applyZoom(zoom.value * factor, viewport.value.width / 2, viewport.value.height / 2);
}

function onDoubleClick(event: MouseEvent): void {
  if (!usable.value || !fit.value) {
    return;
  }
  if ((event.target as HTMLElement | null)?.closest("[data-map-marker='true']")) {
    return;
  }
  const rect = container.value?.getBoundingClientRect();
  if (!rect) {
    return;
  }
  applyZoom(zoom.value * 1.6, event.clientX - rect.left, event.clientY - rect.top);
}

// ---------------------------------------------------------------------------
// 拖拽平移（仅缩放后可用；移动不超过阈值视为点击空白）
// ---------------------------------------------------------------------------

interface DragState {
  startX: number;
  startY: number;
  panX: number;
  panY: number;
  moved: boolean;
  pointerId: number;
}

let drag: DragState | null = null;
let suppressClick = false;

function onPointerDown(event: PointerEvent): void {
  if (event.button !== 0 || zoom.value <= MIN_ZOOM) {
    return;
  }
  const target = event.target as HTMLElement | null;
  if (target?.closest("[data-map-marker='true']")) {
    return;
  }
  drag = {
    startX: event.clientX,
    startY: event.clientY,
    panX: pan.value.x,
    panY: pan.value.y,
    moved: false,
    pointerId: event.pointerId,
  };
  (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
}

function onPointerMove(event: PointerEvent): void {
  if (!drag) {
    return;
  }
  const dx = event.clientX - drag.startX;
  const dy = event.clientY - drag.startY;
  if (!drag.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) {
    return;
  }
  drag.moved = true;
  pan.value = clampPan({ x: drag.panX + dx, y: drag.panY + dy });
}

function onPointerUp(event: PointerEvent): void {
  if (!drag) {
    return;
  }
  suppressClick = drag.moved;
  (event.currentTarget as HTMLElement | null)?.releasePointerCapture?.(drag.pointerId);
  drag = null;
}

function onCanvasClick(event: MouseEvent): void {
  if ((event.target as HTMLElement | null)?.closest("[data-map-marker='true']")) {
    return;
  }
  if (suppressClick) {
    suppressClick = false;
    return;
  }
  emit("select", null);
}

// ---------------------------------------------------------------------------
// 标记
// ---------------------------------------------------------------------------

interface Marker {
  id: string;
  entry: MapEntry;
  left: number;
  top: number;
  collected: boolean;
  selected: boolean;
  category: string;
  label: string;
  ariaLabel: string;
}

const markers = computed<Marker[]>(() => {
  if (!fit.value) {
    return [];
  }
  const result: Marker[] = [];
  for (const entry of props.entries) {
    const position = entry.item.position;
    if (!position) {
      continue;
    }
    const pixel = projectToImage(props.basemap, position.x, position.y);
    if (!pixel) {
      continue;
    }
    const collected = entry.item.status === "collected";
    const category = entry.group.category;
    const categoryName = categoryLabel(category, entry.group.name);
    result.push({
      id: entry.item.id,
      entry,
      left: pixel.u * fit.value,
      top: pixel.v * fit.value,
      collected,
      selected: entry.item.id === props.selectedId,
      category,
      label: categoryName,
      ariaLabel: `${entry.item.display_name}，${categoryName}，${collected ? t("已收集", "collected") : t("未收集", "uncollected")}`,
    });
  }
  return result;
});

/** 选中项最后绘制，避免被相邻点位盖住外圈。 */
const orderedMarkers = computed(() => [
  ...markers.value.filter((marker) => !marker.selected),
  ...markers.value.filter((marker) => marker.selected),
]);

const ariaLabel = computed(() =>
  tFormat("{label}：{count} 个收集品标记", "{label}: {count} collectible markers", {
    label: props.label,
    count: markers.value.length,
  }),
);

function markerStyle(marker: Marker): Record<string, string> {
  return {
    left: `${marker.left}px`,
    top: `${marker.top}px`,
    transform: `translate(-50%, -50%) scale(${1 / zoom.value})`,
    zIndex: marker.selected ? "30" : "1",
  };
}

/** 把标记带入可视区（清单点击调用）；不改变缩放。 */
function focusItem(id: string): void {
  const marker = markers.value.find((entry) => entry.id === id);
  if (!marker || !fit.value) {
    return;
  }
  const margin = 24;
  const x = pan.value.x + marker.left * zoom.value;
  const y = pan.value.y + marker.top * zoom.value;
  const next = { ...pan.value };
  if (x < margin || x > viewport.value.width - margin) {
    next.x = viewport.value.width / 2 - marker.left * zoom.value;
  }
  if (y < margin || y > viewport.value.height - margin) {
    next.y = viewport.value.height / 2 - marker.top * zoom.value;
  }
  pan.value = clampPan(next);
}

defineExpose({ focusItem });
</script>

<template>
  <div
    ref="container"
    class="relative h-full min-h-0 flex-1 touch-none select-none overflow-hidden bg-raised"
    role="group"
    :aria-label="ariaLabel"
    data-testid="map-canvas"
    :data-zoom="zoom"
    @click="onCanvasClick"
    @dblclick="onDoubleClick"
    @wheel="onWheel"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
  >
    <template v-if="usable">
      <div class="absolute top-0 left-0 will-change-transform" :style="layerStyle" data-testid="map-layer">
        <img
          :key="imageUrl ?? ''"
          ref="imageElement"
          :src="imageUrl ?? ''"
          :alt="ariaLabel"
          decoding="async"
          draggable="false"
          class="block max-w-none object-fill"
          :style="{ width: `${canvasWidth}px`, height: `${canvasHeight}px` }"
          data-testid="map-image"
          @error="onImageError"
        />
        <button
          v-for="marker in orderedMarkers"
          :key="marker.id"
          type="button"
          tabindex="-1"
          data-map-marker="true"
          :data-testid="`map-marker-${marker.id}`"
          :data-collected="marker.collected"
          :data-selected="marker.selected"
          :data-category="marker.category"
          :aria-label="marker.ariaLabel"
          :aria-pressed="marker.selected"
          :title="marker.entry.item.display_name"
          class="absolute flex cursor-pointer items-center justify-center rounded-full shadow-[0_1px_2px_rgba(0,0,0,0.35)]"
          :class="[
            marker.selected ? 'h-[26px] w-[26px]' : 'h-6 w-6',
            marker.collected
              ? `border-[1.5px] border-[var(--surface)] ${categoryDotClass(marker.category)} text-[var(--canvas)]`
              : `border-2 border-current bg-surface ${categoryClass(marker.category)}`,
          ]"
          :style="markerStyle(marker)"
          @click.stop="emit('select', marker.id)"
          @dblclick.stop
        >
          <component :is="categoryIcon(marker.category)" :size="14" aria-hidden="true" />
          <span
            v-if="marker.selected"
            class="pointer-events-none absolute -inset-[3px] rounded-full border-2 border-[var(--accent)]"
            data-testid="map-marker-ring"
            aria-hidden="true"
          />
        </button>
      </div>

      <div class="absolute top-2 right-2 z-40 flex flex-col gap-1 rounded-[6px] border border-line bg-surface/95 p-1 shadow">
        <button
          type="button"
          :aria-label="t('放大', 'Zoom in')"
          :title="t('放大', 'Zoom in')"
          :disabled="zoom >= 4"
          class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-[4px] text-ink hover:bg-raised disabled:cursor-not-allowed disabled:opacity-50"
          data-testid="map-zoom-in"
          @click="zoomBy(1.35)"
        >
          <ZoomIn :size="16" aria-hidden="true" />
        </button>
        <button
          type="button"
          :aria-label="t('缩小', 'Zoom out')"
          :title="t('缩小', 'Zoom out')"
          :disabled="zoom <= 1"
          class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-[4px] text-ink hover:bg-raised disabled:cursor-not-allowed disabled:opacity-50"
          data-testid="map-zoom-out"
          @click="zoomBy(1 / 1.35)"
        >
          <ZoomOut :size="16" aria-hidden="true" />
        </button>
        <button
          type="button"
          :aria-label="t('重置缩放（1×）', 'Reset zoom (1×)')"
          :title="t('重置缩放（1×）', 'Reset zoom (1×)')"
          :disabled="zoom <= 1"
          class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-[4px] text-xs text-ink hover:bg-raised disabled:cursor-not-allowed disabled:opacity-50"
          data-testid="map-zoom-reset"
          @click="resetView"
        >
          <RotateCcw :size="14" aria-hidden="true" />
        </button>
      </div>
    </template>

    <div v-else class="flex h-full items-center justify-center p-4 text-center">
      <div class="max-w-sm space-y-2">
        <p class="text-[13px] text-caution">{{ errorText }}</p>
        <slot name="error" />
      </div>
    </div>
  </div>
</template>
