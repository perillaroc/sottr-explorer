<script setup lang="ts">
// 关卡内相对位置散点（SX-022）：
// - 坐标来自游戏数据的关卡归一化空间，**未校准**为地图投影，不是导航点位；
// - 只表达“同一子关卡内这些收集品的相对位置”，不画真实地图底图（版权与校准双重风险）；
// - 已收集实心、未收集空心，当前条目加外圈。
import { computed } from "vue";
import { t } from "../app/i18n";
import type { PlanItem } from "../data/atlas";

const props = defineProps<{
  /** 同一子关卡内的条目（含当前条目）。 */
  items: PlanItem[];
  selectedId: string | null;
}>();

const SIZE = 180;
const PAD = 12;

interface Point {
  id: string;
  label: string;
  x: number;
  y: number;
  collected: boolean;
  selected: boolean;
}

const points = computed<Point[]>(() =>
  props.items
    .filter((item) => item.position)
    .map((item) => ({
      id: item.id,
      label: item.display_name,
      x: PAD + item.position!.x * (SIZE - PAD * 2),
      // 图鉴坐标的 y 向下增大（原点在取景框西北角），与 SVG 的 y 同向，不做翻转。
      // 旧版按「y 向上」做了 1 - y，会让散点上下镜像。
      y: PAD + item.position!.y * (SIZE - PAD * 2),
      collected: item.status === "collected",
      selected: item.id === props.selectedId,
    })),
);

const hasPoints = computed(() => points.value.length > 0);
</script>

<template>
  <figure class="space-y-1" data-testid="position-scatter">
    <svg
      v-if="hasPoints"
      :viewBox="`0 0 ${SIZE} ${SIZE}`"
      class="w-full max-w-[220px] rounded-[6px] border border-line bg-raised"
      role="img"
      :aria-label="t(`同一子关卡内 ${points.length} 个收集品的相对位置（未校准）`, `Relative positions of ${points.length} collectibles in the same sub-level (uncalibrated)`)"
    >
      <line
        v-for="tick in [0.25, 0.5, 0.75]"
        :key="`grid-${tick}`"
        :x1="PAD + tick * (SIZE - PAD * 2)"
        :x2="PAD + tick * (SIZE - PAD * 2)"
        :y1="PAD"
        :y2="SIZE - PAD"
        stroke="var(--border)"
        stroke-width="0.5"
        stroke-dasharray="2 3"
      />
      <line
        v-for="tick in [0.25, 0.5, 0.75]"
        :key="`grid-y-${tick}`"
        :x1="PAD"
        :x2="SIZE - PAD"
        :y1="PAD + tick * (SIZE - PAD * 2)"
        :y2="PAD + tick * (SIZE - PAD * 2)"
        stroke="var(--border)"
        stroke-width="0.5"
        stroke-dasharray="2 3"
      />
      <g v-for="point in points" :key="point.id">
        <circle
          v-if="point.selected"
          :cx="point.x"
          :cy="point.y"
          r="9"
          fill="none"
          stroke="var(--accent-strong)"
          stroke-width="2"
        />
        <circle
          :cx="point.x"
          :cy="point.y"
          r="4.5"
          :fill="point.collected ? 'var(--accent-strong)' : 'transparent'"
          :stroke="point.collected ? 'var(--accent-strong)' : 'var(--muted)'"
          stroke-width="1.6"
        >
          <title>{{ point.label }}（{{ point.collected ? t("已收集", "collected") : t("未收集", "uncollected") }}）</title>
        </circle>
      </g>
      <text :x="PAD" :y="SIZE - 2" class="fill-[var(--muted)]" font-size="8">0</text>
      <text :x="SIZE - PAD - 6" :y="SIZE - 2" class="fill-[var(--muted)]" font-size="8">1</text>
    </svg>
    <figcaption class="text-xs text-muted">
      {{ t("实心 = 已收集，空心 = 未收集；同一子关卡内的相对位置。", "Filled = collected, hollow = uncollected; relative positions within the same sub-level.") }}
      <span class="text-caution">{{ t("坐标未校准，不是导航点位。", "Coordinates are uncalibrated and are not navigation waypoints.") }}</span>
    </figcaption>
  </figure>
</template>
