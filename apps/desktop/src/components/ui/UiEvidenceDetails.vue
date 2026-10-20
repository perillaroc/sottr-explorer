<script setup lang="ts">
// 原始证据折叠层（默认折叠；键盘可用；长内容允许换行）。
import { tFormat } from "../../app/i18n";

withDefaults(
  defineProps<{
    summary: string;
    /** 追加在概要里的状态说明（例如“全部候选名/偏移/来源”）。 */
    hint?: string;
  }>(),
  { hint: undefined },
);
</script>

<template>
  <details class="rounded-[8px] border border-line bg-raised/60 px-3 py-2">
    <summary class="cursor-pointer text-[13px] font-medium text-ink">
      {{ summary }}
      <span v-if="hint" class="ml-1 text-xs text-muted">{{ tFormat("（{hint}）", " ({hint})", { hint }) }}</span>
    </summary>
    <div class="mt-2 space-y-1 text-[13px] break-words text-muted">
      <slot />
    </div>
  </details>
</template>
