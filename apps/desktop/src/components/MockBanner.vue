<script setup lang="ts">
// 模拟数据横幅（SX-015）：所有假数据页面显著标注；演示视口宽度也在此说明。
// 阶段二：更紧凑的 12px 行高；窄窗改用短句，避免双语长文占满首屏。
import { computed } from "vue";
import { FlaskConical } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { copy } from "../app/labels";

const store = useStore();

const message = computed(() =>
  store.layout.value === "narrow"
    ? t("模拟数据：演示输入，不代表真实存档。", "Mock data: demo input, not a real save.")
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
      {{ t(`演示视口宽度 ${store.state.mockWidth}px`, `Demo viewport width ${store.state.mockWidth}px`) }}
    </span>
  </div>
</template>
