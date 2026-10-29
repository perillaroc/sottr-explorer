<script setup lang="ts">
// 底图面板（P3-01）：在「同一子关卡内的相对位置」段落里替换散点。
//
// 显示底图的条件（P3-02）：该子关卡有基底层记录、状态不是 unverified、
// 且配置了外置图片素材。任一条不满足就回退到 PositionScatter——
// 图片加载失败同样回退，不显示破图。
//
// 点位坐标来自基底层矩阵（首版为 u = x·W、v = y·H），不做任何前端拟合。
import { computed, ref, watch } from "vue";
import { t, tFormat } from "../app/i18n";
import type { PlanItem } from "../data/atlas";
import { basemapImageUrl, projectToImage, resolveBasemapMode, type Basemap } from "../data/basemaps";
import PositionScatter from "./PositionScatter.vue";

const props = withDefaults(
  defineProps<{
    /** 同一子关卡内的条目（含当前条目）。 */
    items: PlanItem[];
    selectedId: string | null;
    /** 该子关卡的基底层记录；没有则为 null。 */
    basemap: Basemap | null;
    /** 所在段落是否已展开；折叠时不挂载图片，避免提前下载。 */
    active?: boolean;
  }>(),
  // Boolean 属性缺省会被 Vue 转成 false，这里显式默认成 true（独立使用时立即加载）。
  { active: true },
);

const imageFailed = ref(false);

watch(
  () => props.basemap?.basemap_id,
  () => {
    imageFailed.value = false;
  },
);

const imageUrl = computed(() => (props.basemap ? basemapImageUrl(props.basemap) : null));

/** 展开且地址可用时才挂载 `<img>`；折叠状态不发起图片请求。 */
const shouldLoadImage = computed(() => props.active && imageUrl.value !== null);

const resolved = computed(() => resolveBasemapMode(props.basemap, imageUrl.value, imageFailed.value));
const mode = computed(() => resolved.value.mode);

interface Dot {
  id: string;
  label: string;
  left: number;
  top: number;
  collected: boolean;
  selected: boolean;
}

const dots = computed<Dot[]>(() => {
  const entry = props.basemap;
  if (!entry || mode.value !== "map") {
    return [];
  }
  const points: Dot[] = [];
  for (const item of props.items) {
    if (!item.position) {
      continue;
    }
    const pixel = projectToImage(entry, item.position.x, item.position.y);
    if (!pixel) {
      continue;
    }
    points.push({
      id: item.id,
      label: item.display_name,
      left: (pixel.u / entry.image_width) * 100,
      top: (pixel.v / entry.image_height) * 100,
      collected: item.status === "collected",
      selected: item.id === props.selectedId,
    });
  }
  return points;
});

/** 底图的最大高度（像素）；宽度按可用宽度与图片比例自适应。 */
const MAX_HEIGHT = 320;

/**
 * 面板内边距之外的可用宽度由父容器决定，这里只负责「不超宽、不超高」：
 * 宽度取 `min(100%, 最大高度 × 宽高比)`，保证任意比例（含 2:1 宽图）都按高度上限收敛。
 */
const mapBoxStyle = computed(() => {
  const entry = props.basemap;
  if (!entry || entry.image_height <= 0) {
    return {};
  }
  const ratio = entry.image_width / entry.image_height;
  return {
    aspectRatio: `${entry.image_width} / ${entry.image_height}`,
    maxWidth: `min(100%, ${Math.round(MAX_HEIGHT * ratio)}px)`,
  };
});

/** 当前条目最后绘制，避免被相邻点位盖住外圈。 */
const orderedDots = computed<Dot[]>(() => [
  ...dots.value.filter((dot) => !dot.selected),
  ...dots.value.filter((dot) => dot.selected),
]);

const ariaLabel = computed(() =>
  tFormat(
    `同一子关卡内 {n} 个收集品在地图底图上的位置`,
    `Positions of {n} collectibles on the sub-level map`,
    { n: dots.value.length },
  ),
);

/** 误差像素值保留一位小数；缺失时给占位符而不是编造数字。 */
function formatPx(value: number | null | undefined): string {
  return typeof value === "number" && Number.isFinite(value) ? value.toFixed(1) : "—";
}

const caption = computed(() => {
  if (mode.value === "map" && props.basemap) {
    const entry = props.basemap;
    const median = formatPx(entry.median_error_px);
    const p95 = formatPx(entry.p95_error_px);
    const estimated = entry.error_source === "global_estimate";
    // 只有 status = calibrated 才允许出现「已校准」字样。
    const main =
      entry.status === "calibrated"
        ? tFormat(
            "已校准 · 中位误差 {median} px，P95 {p95} px",
            "Calibrated · median {median} px, P95 {p95} px",
            { median, p95 },
          )
        : tFormat(
            "近似位置 · 中位误差 {median} px{estimated}",
            "Approximate · median {median} px{estimated}",
            { median, estimated: estimated ? t("（全局估计）", " (global estimate)") : "" },
          );
    return {
      main,
      legend: t("实心 = 已收集，空心 = 未收集。", "Filled = collected, hollow = uncollected."),
      caution: t("不是导航路线。", "Not a navigation route."),
    };
  }
  return {
    main: t(
      "实心 = 已收集，空心 = 未收集；同一子关卡内的相对位置。",
      "Filled = collected, hollow = uncollected; relative positions within the same sub-level.",
    ),
    legend: "",
    caution: t("坐标未校准，不是导航点位。", "Coordinates are uncalibrated and are not navigation waypoints."),
  };
});
</script>

<template>
  <figure
    class="space-y-1"
    data-testid="map-panel"
    :data-mode="mode"
    :data-reason="resolved.reason"
    :data-status="mode === 'map' && basemap ? basemap.status : null"
  >
    <div
      v-if="mode === 'map' && basemap"
      class="relative mx-auto w-full overflow-hidden rounded-[6px] border border-line bg-raised"
      :style="mapBoxStyle"
      data-testid="basemap-frame"
    >
      <template v-if="shouldLoadImage && imageUrl">
        <img
          :src="imageUrl"
          :alt="ariaLabel"
          class="absolute inset-0 h-full w-full object-fill"
          data-testid="basemap-image"
          @error="imageFailed = true"
        />
        <span
          v-for="dot in orderedDots"
          :key="dot.id"
          class="absolute h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-[1.5px]"
          :class="[
            dot.collected
              ? 'border-[var(--accent-strong)] bg-[var(--accent-strong)]'
              : 'border-[var(--muted)] bg-transparent',
            dot.selected ? 'z-10' : 'z-0',
          ]"
          :style="{ left: `${dot.left}%`, top: `${dot.top}%` }"
          :data-collected="dot.collected"
          :data-selected="dot.selected"
          data-testid="basemap-dot"
        >
        <span
          v-if="dot.selected"
          class="pointer-events-none absolute -inset-[4px] rounded-full border-2 border-[var(--accent-strong)]"
          data-testid="basemap-dot-ring"
        />
        <title>{{ dot.label }}（{{ dot.collected ? t("已收集", "collected") : t("未收集", "uncollected") }}）</title>
        </span>
      </template>
    </div>
    <PositionScatter v-else :items="items" :selected-id="selectedId" />
    <figcaption v-if="mode === 'map'" class="text-xs text-muted">
      {{ caption.main }}
      <template v-if="caption.legend"> {{ caption.legend }}</template>
      <span class="text-caution">{{ caption.caution }}</span>
    </figcaption>
  </figure>
</template>
