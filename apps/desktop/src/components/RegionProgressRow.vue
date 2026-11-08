<script setup lang="ts">
// 区域进度入口（2026-11-05 界面美化阶段三）：概览与“全部区域”共用。
// 名称 + 已收集/总数 + 细进度条 + 未收集数量；整行可点击，完成时显示文字与勾选，
// 不只把进度条变绿。地图按钮是可选的第二动作，业务动作由父级通过事件执行。
import { computed } from "vue";
import { Check, MapPin } from "@lucide/vue";
import { t, tFormat } from "../app/i18n";
import UiProgressBar from "./ui/UiProgressBar.vue";

const props = withDefaults(
  defineProps<{
    slug: string;
    name: string;
    collected: number;
    total: number;
    /** 主按钮的 data-testid；不同页面沿用既有定位。 */
    testId?: string;
    /** 是否提供“在地图中查看”按钮。 */
    mapAvailable?: boolean;
    mapTestId?: string;
  }>(),
  { testId: undefined, mapAvailable: false, mapTestId: undefined },
);

const emit = defineEmits<{
  open: [slug: string];
  openMap: [slug: string];
}>();

const remaining = computed(() => Math.max(0, props.total - props.collected));
const complete = computed(() => props.total > 0 && remaining.value === 0);
const progressLabel = computed(() =>
  tFormat("{region} 收集进度", "{region} collection progress", { region: props.name }),
);
const mapLabel = computed(() =>
  tFormat("在地图中查看 {region}", "View {region} on the map", { region: props.name }),
);

const remainingLabel = computed(() => {
  if (props.total === 0) {
    return t("无条目", "No items");
  }
  return remaining.value > 0
    ? t(`未收集 ${remaining.value} 项`, `${remaining.value} uncollected`)
    : t("已全部收集", "All collected");
});
</script>

<template>
  <div
    class="flex items-stretch overflow-hidden rounded-section border border-line bg-surface transition-colors hover:border-accent/60"
  >
    <button
      type="button"
      class="ui-interactive flex min-w-0 flex-1 cursor-pointer flex-col gap-2 px-4 py-3 text-left enabled:hover:bg-raised/60"
      :data-testid="testId"
      @click="emit('open', slug)"
    >
      <div class="flex items-baseline justify-between gap-2">
        <span class="truncate text-[14px] font-medium text-ink">{{ name }}</span>
        <span class="shrink-0 text-xs tabular-nums text-muted">{{ collected }}/{{ total }}</span>
      </div>
      <UiProgressBar :value="collected" :total="total" :label="progressLabel" />
      <div class="flex items-center justify-between gap-2 text-xs">
        <span class="text-muted">{{ remainingLabel }}</span>
        <Check v-if="complete" :size="14" class="shrink-0 text-success" aria-hidden="true" />
      </div>
    </button>
    <button
      v-if="mapAvailable"
      type="button"
      class="ui-interactive flex w-10 shrink-0 cursor-pointer items-center justify-center border-l border-line text-muted enabled:hover:bg-raised enabled:hover:text-accent-strong"
      :aria-label="mapLabel"
      :title="t('在地图中查看', 'View on the map')"
      :data-testid="mapTestId"
      @click="emit('openMap', slug)"
    >
      <MapPin :size="15" aria-hidden="true" />
    </button>
  </div>
</template>
