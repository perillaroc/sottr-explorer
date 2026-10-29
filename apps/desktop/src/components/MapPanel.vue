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

const props = defineProps<{
  /** 同一子关卡内的条目（含当前条目）。 */
  items: PlanItem[];
  selectedId: string | null;
  /** 该子关卡的基底层记录；没有则为 null。 */
  basemap: Basemap | null;
}>();

const imageFailed = ref(false);

watch(
  () => props.basemap?.basemap_id,
  () => {
    imageFailed.value = false;
  },
);

const imageUrl = computed(() => (props.basemap ? basemapImageUrl(props.basemap) : null));

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

const ariaLabel = computed(() =>
  tFormat(
    `同一子关卡内 {n} 个收集品在地图底图上的位置`,
    `Positions of {n} collectibles on the sub-level map`,
    { n: dots.value.length },
  ),
);

const caption = computed(() => {
  if (mode.value === "map" && props.basemap) {
    const approximate = props.basemap.status === "approximate";
    return {
      main: approximate
        ? t(
            "底图点位为近似位置，按该子关卡的变换绘制。",
            "Map markers are approximate positions drawn with this sub-level's transform.",
          )
        : t(
            "底图点位按该子关卡已校准的变换绘制。",
            "Map markers are drawn with this sub-level's calibrated transform.",
          ),
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
  >
    <div
      v-if="mode === 'map' && basemap && imageUrl"
      class="relative mx-auto w-full max-w-[320px] overflow-hidden rounded-[6px] border border-line bg-raised"
      :style="{ aspectRatio: `${basemap.image_width} / ${basemap.image_height}` }"
    >
      <img
        :src="imageUrl"
        :alt="ariaLabel"
        class="absolute inset-0 h-full w-full object-fill"
        data-testid="basemap-image"
        @error="imageFailed = true"
      />
      <span
        v-for="dot in dots"
        :key="dot.id"
        class="absolute h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-[1.5px]"
        :class="
          dot.collected
            ? 'border-[var(--accent-strong)] bg-[var(--accent-strong)]'
            : 'border-[var(--muted)] bg-transparent'
        "
        :style="{ left: `${dot.left}%`, top: `${dot.top}%` }"
        :data-collected="dot.collected"
        :data-selected="dot.selected"
        data-testid="basemap-dot"
      >
        <span
          v-if="dot.selected"
          class="pointer-events-none absolute -inset-[3px] rounded-full border-2 border-[var(--accent-strong)]"
        />
        <title>{{ dot.label }}（{{ dot.collected ? t("已收集", "collected") : t("未收集", "uncollected") }}）</title>
      </span>
    </div>
    <PositionScatter v-else :items="items" :selected-id="selectedId" />
    <figcaption v-if="mode === 'map'" class="text-xs text-muted">
      {{ caption.main }}
      <template v-if="caption.legend"> {{ caption.legend }}</template>
      <span class="text-caution">{{ caption.caution }}</span>
    </figcaption>
  </figure>
</template>
