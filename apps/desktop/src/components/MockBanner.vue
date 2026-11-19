<script setup lang="ts">
// 模拟数据横幅（SX-015）：所有假数据页面显著标注；演示视口宽度也在此说明。
// 阶段二：更紧凑的 12px 行高；窄窗改用短句，避免双语长文占满首屏。
import { computed } from "vue";
import { FlaskConical } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormatPair } from "../app/i18n";
import { copy } from "../app/labels";

const store = useStore();

const message = computed(() =>
  store.layout.value === "narrow"
    ? t("mockBanner.mock-data-demo-input-not-a-real")
    : copy.mockBanner,
);
</script>

<template>
  <div
    class="flex flex-wrap items-center gap-1.5 border-b border-caution/40 bg-caution-bg px-3 py-1 text-[12px] leading-4 text-caution"
    role="status"
    data-testid="mock-banner"
  >
    <FlaskConical :size="14" class="shrink-0" aria-hidden="true" />
    <span>{{ message }}</span>
    <span v-if="store.state.mockWidth" class="rounded-control border border-caution/40 px-1.5">
      {{ tFormatPair(`mockBanner.demo-viewport-width-px`, { v0: store.state.mockWidth }) }}
    </span>
  </div>
</template>
