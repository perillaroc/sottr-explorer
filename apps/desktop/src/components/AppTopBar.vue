<script setup lang="ts">
// 顶栏（SX-015）：应用名、当前文件与切换、全局分析状态、手动刷新、更多菜单。
// 全局分析状态靠近文件选择器，明确“正在分析哪个文件”。
import { computed } from "vue";
import { ChevronDown, Menu, RefreshCw } from "@lucide/vue";import { useStore } from "../app/inject";
import { copy } from "../app/labels";
import { fileNameOf, formatSnapshotTime } from "../app/format";
import UiButton from "./ui/UiButton.vue";
import UiIconButton from "./ui/UiIconButton.vue";
import UiMenu from "./ui/UiMenu.vue";
import type { MenuItem } from "./ui/types";

const store = useStore();

const currentName = computed(() => {
  const path = store.state.selectedPath;
  return path ? fileNameOf(path) : "未选择文件";
});

const hasResult = computed(() => store.currentSlot.value?.result !== null);

const statusText = computed(() => {
  const analysis = store.state.analysis;
  if (analysis) {
    const name = fileNameOf(analysis.path);
    return analysis.kind === "refresh" ? `正在刷新 ${name}（保留上次结果）` : `正在分析 ${name}…`;
  }
  if (store.state.lastCancelledPath === store.state.selectedPath && store.state.selectedPath) {
    return "已取消";
  }
  const slot = store.currentSlot.value;
  if (slot?.result) {
    if (slot.result.freshness === "stale") {
      return `旧快照（${formatSnapshotTime(slot.result.snapshot_time)}）`;
    }
    return `分析完成（快照 ${formatSnapshotTime(slot.result.snapshot_time)}）`;
  }
  if (slot?.startError) {
    return `分析失败：${slot.startError.code}`;
  }
  return "尚未分析";
});

const moreItems = computed<MenuItem[]>(() => [
  store.canExport.value
    ? { id: "export", label: "导出当前快照（JSON/CSV）…" }
    : {
        id: "export",
        label: "导出当前快照（JSON/CSV）…",
        disabled: true,
        reason: store.state.analysis
          ? "分析/刷新进行中：完成后再导出"
          : "没有可导出的报告：请先分析当前槽位",
      },
  { id: "rescan", label: "重新查找默认目录" },
  { id: "diagnostics", label: "诊断" },
  { id: "settings", label: "设置与关于" },
]);

function onMore(id: string): void {
  switch (id) {
    case "export":
      store.actions.openExportDialog();
      break;
    case "rescan":
      void store.actions.rescan();
      break;
    case "diagnostics":
      store.actions.setView("diagnostics");
      break;
    case "settings":
      store.actions.setView("settings");
      break;
    default:
      break;
  }
}
</script>

<template>
  <header class="flex h-12 shrink-0 items-center gap-2 border-b border-line bg-surface px-3">
    <UiIconButton
      v-if="store.layout.value === 'narrow'"
      label="打开导航菜单"
      :pressed="store.state.navOverlayOpen"
      @click="store.actions.toggleNav()"
    >
      <Menu :size="18" aria-hidden="true" />
    </UiIconButton>
    <UiButton
      v-else
      size="sm"
      variant="ghost"
      :title="store.state.navCollapsed ? '展开导航' : '折叠导航'"
      @click="store.actions.toggleNav()"
    >
      <Menu :size="16" aria-hidden="true" />
      <span class="sr-only">{{ store.state.navCollapsed ? "展开导航" : "折叠导航" }}</span>
    </UiButton>

    <div class="flex min-w-0 items-baseline gap-2">
      <span class="text-sm font-semibold whitespace-nowrap">{{ copy.appName }}</span>
      <span class="hidden text-xs text-muted sm:inline">{{ copy.subtitle }}</span>
    </div>

    <div class="ml-2 flex min-w-0 items-center gap-2">
      <span class="text-[13px] text-muted">当前：</span>
      <span class="max-w-48 truncate text-[13px]" data-testid="current-file" :title="store.state.selectedPath ?? ''">
        {{ currentName }}
      </span>
      <UiButton size="sm" title="切换存档（打开导入页）" @click="store.actions.openImport()">切换</UiButton>
    </div>

    <div class="flex-1" />

    <p class="max-w-64 truncate text-[13px] text-muted" aria-live="polite" data-testid="analysis-status">
      {{ statusText }}
    </p>
    <UiButton
      v-if="store.state.analysis"
      size="sm"
      variant="danger"
      title="取消当前分析（不产生新报告）"
      @click="store.actions.cancelAnalysis()"
    >
      取消
    </UiButton>
    <UiButton
      size="sm"
      :disabled="!hasResult || store.state.analysis !== null"
      :title="hasResult ? '手动刷新（F5）' : '尚未分析，无法刷新'"
      @click="store.actions.refresh()"
    >
      <RefreshCw :size="16" aria-hidden="true" />
      刷新
    </UiButton>
    <UiMenu label="更多" :items="moreItems" @select="onMore">
      <span class="inline-flex items-center gap-1">更多<ChevronDown :size="14" aria-hidden="true" /></span>
    </UiMenu>
  </header>
</template>
