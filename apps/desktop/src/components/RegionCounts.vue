<script setup lang="ts">
// 区域计数（SX-015）：只显示已支持的计数单元格；缺项显示“未知/暂不支持”及原因，
// 绝不补成 0；计数超来源目标显示“计数待核对”；不生成整体区域百分比。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { copy, regionLabel } from "../app/labels";
import { formatCount } from "../app/format";
import { regionCells, type CountCell } from "../app/catalog";
import { regionsOf } from "../data/metadata";

const props = withDefaults(
  defineProps<{
    /** 单个区域；null 表示列出全部区域（概览用）。 */
    region: string | null;
    compact?: boolean;
  }>(),
  { compact: false },
);

const store = useStore();

const regionIds = computed(() =>
  props.region === null ? regionsOf(store.metadata).map((region) => region.region_id) : [props.region],
);

function cells(regionId: string): CountCell[] {
  return regionCells(store.metadata, store.report.value, regionId);
}

function cellText(cell: CountCell): string {
  switch (cell.kind) {
    case "value":
      return formatCount(cell.value, cell.target);
    case "conflict":
      return copy.countConflict;
    case "unknown":
      return copy.unknownCell;
    default:
      return copy.unsupportedCell;
  }
}

function cellTitle(cell: CountCell): string {
  const parts = [`${cell.label}：${cellText(cell)}`];
  if (cell.kind === "value" && cell.target !== null) {
    parts.push(`来源目标 ${cell.target}（社区脚本；未证明适用所有版本/DLC）`);
  }
  if (cell.kind === "conflict" && cell.rawValue !== null) {
    parts.push(`原始读数 ${cell.rawValue}（不裁剪到目标值）`);
  }
  if (cell.reason) {
    parts.push(cell.reason);
  }
  if (cell.mergedNote) {
    parts.push(cell.mergedNote);
  }
  return parts.join("；");
}

function labelOf(regionId: string): string {
  return regionLabel(regionId, store.metadata.regions[regionId]?.display_name ?? null);
}
</script>

<template>
  <div class="space-y-1.5" data-testid="region-counts">
    <div
      v-for="regionId in regionIds"
      :key="regionId"
      class="flex flex-wrap items-baseline gap-x-3 gap-y-1 rounded-[6px] px-2 py-1.5 text-[13px]"
      :class="regionId === store.state.region && props.region === null ? 'bg-accent-bg' : 'bg-raised'"
    >
      <span class="min-w-24 font-medium">{{ labelOf(regionId) }}</span>
      <span
        v-for="cell in cells(regionId)"
        :key="cell.key"
        class="text-muted"
        :title="cellTitle(cell)"
        :data-cell="`${regionId}:${cell.categoryId}:${cell.kind}`"
      >
        {{ cell.label }}
        <span
          :class="
            cell.kind === 'value'
              ? 'text-ink'
              : cell.kind === 'conflict'
                ? 'text-caution'
                : cell.kind === 'unknown'
                  ? 'text-muted'
                  : 'text-muted'
          "
          >{{ cellText(cell) }}</span
        >
      </span>
    </div>
    <p class="text-xs text-muted">
      区域计数为社区关联；本区域百分比暂不支持。缺项显示“{{ copy.unknownCell }} / {{ copy.unsupportedCell }}”及原因，不补成 0。
    </p>
  </div>
</template>
