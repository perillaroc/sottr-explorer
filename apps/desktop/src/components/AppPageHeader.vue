<script setup lang="ts">
// 页面骨架（2026-11-05 界面美化阶段二）：统一标题、上下文、说明与页面级动作。
// 不管理 store、不改变路由；筛选工具条通过默认插槽放在标题下方。
withDefaults(
  defineProps<{
    /** 页面名，每页一个。 */
    title: string;
    /** 当前区域 / 存档 / 快照等上下文。 */
    context?: string;
    /** 仅必要的业务口径说明。 */
    description?: string;
    /** 标题 id，供页面 section 的 aria-labelledby 使用。 */
    titleId?: string;
  }>(),
  { context: undefined, description: undefined, titleId: undefined },
);
</script>

<template>
  <header class="ui-page-header" data-testid="page-header">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="min-w-0">
        <p v-if="context" class="ui-meta mb-1 truncate" :title="context" data-testid="page-context">
          {{ context }}
        </p>
        <h1 :id="titleId" class="ui-page-title line-clamp-2 break-words" data-testid="page-title">{{ title }}</h1>
        <p
          v-if="description"
          class="mt-1 max-w-[76ch] text-[13px] leading-5 text-muted"
          data-testid="page-description"
        >
          {{ description }}
        </p>
      </div>
      <div v-if="$slots.actions" class="flex min-w-0 flex-wrap items-center gap-2">
        <slot name="actions" />
      </div>
    </div>
    <div v-if="$slots.default" class="mt-4">
      <slot />
    </div>
  </header>
</template>
