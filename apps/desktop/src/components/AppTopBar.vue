<script setup lang="ts">
// 顶栏（SX-015）：应用名、当前文件与切换、全局分析状态、手动刷新、更多菜单。
// 全局分析状态靠近文件选择器，明确“正在分析哪个文件”。
import { computed } from "vue";
import { ChevronDown, Menu, RefreshCw } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { copy } from "../app/labels";
import { fileNameOf, formatSnapshotTime } from "../app/format";
import UiButton from "./ui/UiButton.vue";
import UiIconButton from "./ui/UiIconButton.vue";
import UiMenu from "./ui/UiMenu.vue";
import type { MenuItem } from "./ui/types";

const store = useStore();

const currentName = computed(() => {
  const path = store.state.selectedPath;
  return path ? fileNameOf(path) : t("未选择文件", "No file selected");
});

const hasResult = computed(() => store.currentSlot.value?.result !== null);

const statusText = computed(() => {
  const analysis = store.state.analysis;
  if (analysis) {
    const name = fileNameOf(analysis.path);
    return analysis.kind === "refresh"
      ? t(`正在刷新 ${name}（保留上次结果）`, `Refreshing ${name} (previous result kept)`)
      : t(`正在分析 ${name}…`, `Analyzing ${name}…`);
  }
  if (store.state.lastCancelledPath === store.state.selectedPath && store.state.selectedPath) {
    return t("已取消", "Cancelled");
  }
  const slot = store.currentSlot.value;
  if (slot?.result) {
    if (slot.result.freshness === "stale") {
      return t(`旧快照（${formatSnapshotTime(slot.result.snapshot_time)}）`, `Stale snapshot (${formatSnapshotTime(slot.result.snapshot_time)})`);
    }
    return t(`分析完成（快照 ${formatSnapshotTime(slot.result.snapshot_time)}）`, `Analysis complete (snapshot ${formatSnapshotTime(slot.result.snapshot_time)})`);
  }
  if (slot?.startError) {
    return t(`分析失败：${slot.startError.code}`, `Analysis failed: ${slot.startError.code}`);
  }
  return t("尚未分析", "Not analyzed yet");
});

const moreItems = computed<MenuItem[]>(() => [
  store.canExport.value
    ? { id: "export", label: t("导出当前快照（JSON/CSV）…", "Export current snapshot (JSON/CSV)…") }
    : {
        id: "export",
        label: t("导出当前快照（JSON/CSV）…", "Export current snapshot (JSON/CSV)…"),
        disabled: true,
        reason: store.state.analysis
          ? t("分析/刷新进行中：完成后再导出", "Analysis/refresh in progress: export after it finishes")
          : t("没有可导出的报告：请先分析当前槽位", "No report to export: analyze the current slot first"),
      },
  { id: "rescan", label: t("重新查找默认目录", "Rescan default folders") },
  { id: "diagnostics", label: t("诊断", "Diagnostics") },
  { id: "settings", label: t("设置与关于", "Settings & About") },
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
      :label="t('打开导航菜单', 'Open navigation menu')"
      :pressed="store.state.navOverlayOpen"
      @click="store.actions.toggleNav()"
    >
      <Menu :size="18" aria-hidden="true" />
    </UiIconButton>
    <UiButton
      v-else
      size="sm"
      variant="ghost"
      :title="store.state.navCollapsed ? t('展开导航', 'Expand navigation') : t('折叠导航', 'Collapse navigation')"
      @click="store.actions.toggleNav()"
    >
      <Menu :size="16" aria-hidden="true" />
      <span class="sr-only">{{ store.state.navCollapsed ? t("展开导航", "Expand navigation") : t("折叠导航", "Collapse navigation") }}</span>
    </UiButton>

    <div class="flex min-w-0 items-baseline gap-2">
      <span class="text-sm font-semibold whitespace-nowrap">{{ copy.appName }}</span>
      <span class="hidden text-xs text-muted sm:inline">{{ copy.subtitle }}</span>
    </div>

    <div class="ml-2 flex min-w-0 items-center gap-2">
      <span class="text-[13px] text-muted">{{ t("当前：", "Current:") }}</span>
      <span class="max-w-48 truncate text-[13px]" data-testid="current-file" :title="store.state.selectedPath ?? ''">
        {{ currentName }}
      </span>
      <UiButton size="sm" :title="t('切换存档（打开导入页）', 'Switch save (open import page)')" @click="store.actions.openImport()">{{ t("切换", "Switch") }}</UiButton>
    </div>

    <div class="flex-1" />

    <p class="max-w-64 truncate text-[13px] text-muted" aria-live="polite" data-testid="analysis-status">
      {{ statusText }}
    </p>
    <UiButton
      v-if="store.state.analysis"
      size="sm"
      variant="danger"
      :title="t('取消当前分析（不产生新报告）', 'Cancel the current analysis (no new report)')"
      @click="store.actions.cancelAnalysis()"
    >
      {{ t("取消", "Cancel") }}
    </UiButton>
    <UiButton
      size="sm"
      :disabled="!hasResult || store.state.analysis !== null"
      :title="hasResult ? t('手动刷新（F5）', 'Refresh manually (F5)') : t('尚未分析，无法刷新', 'Not analyzed yet; cannot refresh')"
      @click="store.actions.refresh()"
    >
      <RefreshCw :size="16" aria-hidden="true" />
      {{ t("刷新", "Refresh") }}
    </UiButton>
    <UiMenu :label="t('更多', 'More')" :items="moreItems" @select="onMore">
      <span class="inline-flex items-center gap-1">{{ t("更多", "More") }}<ChevronDown :size="14" aria-hidden="true" /></span>
    </UiMenu>
  </header>
</template>
