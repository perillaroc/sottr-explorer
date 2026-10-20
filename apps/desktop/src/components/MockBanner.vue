<script setup lang="ts">
// 模拟数据横幅（SX-015）：所有假数据页面显著标注；演示视口宽度也在此说明。
import { FlaskConical } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { copy } from "../app/labels";

const store = useStore();
</script>

<template>
  <div
    class="flex flex-wrap items-center gap-2 border-b border-caution/40 bg-caution-bg px-3 py-1.5 text-[13px] text-caution"
    role="status"
    data-testid="mock-banner"
  >
    <FlaskConical :size="16" aria-hidden="true" />
    <span>{{ copy.mockBanner }}</span>
    <span v-if="store.state.mockWidth" class="rounded-[6px] border border-caution/40 px-1.5">
      {{ t(`演示视口宽度 ${store.state.mockWidth}px`, `Demo viewport width ${store.state.mockWidth}px`) }}
    </span>
  </div>
</template>
