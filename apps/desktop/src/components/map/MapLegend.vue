<script setup lang="ts">
// 类别图例 + 类别筛选（阶段二）：图标与名称成对出现（不单靠颜色/形状表意）；
// 点击某类别即筛选地图与清单，再点「全部类别」清除筛选。
import { categoryIcon } from "../../app/category-icon";
import { categoryClass } from "../../app/category-style";

export interface MapLegendOption {
  /** 空串表示「全部类别」。 */
  category: string;
  label: string;
  count?: number;
}

withDefaults(
  defineProps<{
    options: MapLegendOption[];
    selected: string | null;
    ariaLabel: string;
  }>(),
  {},
);

const emit = defineEmits<{ select: [category: string | null] }>();
</script>

<template>
  <div role="group" :aria-label="ariaLabel" class="flex flex-nowrap items-center gap-1 overflow-x-auto pb-0.5">
    <button
      v-for="option in options"
      :key="option.category || 'all'"
      type="button"
      :aria-pressed="(selected ?? '') === option.category"
      :title="option.label"
      class="ui-interactive inline-flex h-8 shrink-0 cursor-pointer items-center gap-1 rounded-control border px-2.5 text-[13px] leading-5"
      :class="
        (selected ?? '') === option.category
          ? 'border-accent bg-accent-bg font-medium text-accent-strong'
          : 'border-line bg-surface text-ink hover:bg-raised'
      "
      :data-testid="`map-category-${option.category || 'all'}`"
      @click="emit('select', option.category === '' ? null : option.category)"
    >
      <component
        :is="categoryIcon(option.category || null)"
        v-if="option.category"
        :size="14"
        :class="categoryClass(option.category)"
        aria-hidden="true"
      />
      <span :class="option.category ? categoryClass(option.category) : ''">{{ option.label }}</span>
      <span v-if="option.count !== undefined" class="text-xs tabular-nums text-muted">{{ option.count }}</span>
    </button>
  </div>
</template>
