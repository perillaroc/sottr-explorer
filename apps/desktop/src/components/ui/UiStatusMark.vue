<script setup lang="ts">
// 收集状态图形（2026-11-05 界面美化阶段一）：已收集 = 带勾选圆形，
// 未收集 = 空心圆形，未知 = 虚线圆形 + 问号，不自行猜测业务状态。
// 默认通过 role="img" + aria-label 暴露可访问文字；调用方已有可见状态文字时
// 传 decorative 避免读屏重复播报。
import { computed } from "vue";
import { Check } from "@lucide/vue";
import { t } from "../../app/i18n";

const props = withDefaults(
  defineProps<{
    status: "collected" | "uncollected" | "unknown";
    /** 覆盖默认的可访问 / 悬停文字。 */
    label?: string;
    /** 图形仅作装饰，由相邻文字表达状态。 */
    decorative?: boolean;
  }>(),
  { label: undefined, decorative: false },
);

const kind = computed<"collected" | "uncollected" | "unknown">(() => {
  if (props.status === "collected") {
    return "collected";
  }
  if (props.status === "uncollected") {
    return "uncollected";
  }
  return "unknown";
});

const text = computed(() =>
  props.label ??
  (kind.value === "collected"
    ? t("已收集", "Collected")
    : kind.value === "uncollected"
      ? t("未收集", "Uncollected")
      : t("未知", "Unknown")),
);
</script>

<template>
  <span
    :class="['ui-status-mark', `ui-status-mark--${kind}`]"
    :role="decorative ? undefined : 'img'"
    :aria-label="decorative ? undefined : text"
    :aria-hidden="decorative ? 'true' : undefined"
    :title="decorative ? undefined : text"
    :data-status="kind"
  >
    <Check v-if="kind === 'collected'" :size="11" :stroke-width="3" aria-hidden="true" />
    <span v-else-if="kind === 'unknown'" aria-hidden="true">?</span>
  </span>
</template>
