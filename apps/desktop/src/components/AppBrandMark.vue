<script setup lang="ts">
// 品牌罗盘符号（2026-11-05 界面美化阶段二）：复用 Lucide Compass 几何，
// 不替换应用图标文件；品牌名由相邻文字提供，图形本身对读屏隐藏。
import { Compass } from "@lucide/vue";

withDefaults(defineProps<{ size?: number }>(), { size: 24 });
</script>

<template>
  <Compass :size="size" :stroke-width="1.75" class="text-accent" aria-hidden="true" />
</template>
