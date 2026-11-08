<script setup lang="ts">
// 顶栏（2026-11-05 界面美化阶段二）：56px 三段结构——品牌 / 当前存档 / 状态与动作。
// 1440 以上完整显示；1100–1439 隐藏副标题、压缩状态；800–1099 文件名可截断；
// <800 刷新与更多改为图标按钮。文件名、切换入口、刷新状态与取消动作始终保留。
import { computed } from "vue";
import {
  ChevronDown,
  CircleAlert,
  CircleCheck,
  CircleDashed,
  Menu,
  RefreshCw,
  TriangleAlert,
} from "@lucide/vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { copy } from "../app/labels";
import { fileNameOf, formatSnapshotTime } from "../app/format";
import AppBrandMark from "./AppBrandMark.vue";
import UiButton from "./ui/UiButton.vue";
import UiIconButton from "./ui/UiIconButton.vue";
import UiMenu from "./ui/UiMenu.vue";
import type { MenuItem } from "./ui/types";

const store = useStore();

const layout = computed(() => store.layout.value);
const isNarrow = computed(() => layout.value === "narrow");
const isCompactStatus = computed(() => layout.value === "compact" || layout.value === "narrow");
const showSubtitle = computed(() => layout.value === "wide");
const showDividers = computed(() => layout.value !== "narrow");

const currentName = computed(() => {
  const path = store.state.selectedPath;
  return path ? fileNameOf(path) : t("未选择文件", "No file selected");
});

const fileMaxClass = computed(() => {
  switch (layout.value) {
    case "wide":
      return "max-w-56";
    case "medium":
      return "max-w-48";
    case "compact":
      return "max-w-36";
    default:
      return "max-w-28";
  }
});

const statusMaxClass = computed(() => {
  switch (layout.value) {
    case "wide":
      return "max-w-[288px]";
    case "medium":
      return "max-w-[176px]";
    default:
      return "max-w-[128px]";
  }
});

const hasResult = computed(() => store.currentSlot.value?.result != null);

type StatusKind = "analysis" | "cancelled" | "stale" | "complete" | "failed" | "idle";

const statusKind = computed<StatusKind>(() => {
  if (store.state.analysis) {
    return "analysis";
  }
  if (store.state.lastCancelledPath === store.state.selectedPath && store.state.selectedPath) {
    return "cancelled";
  }
  const slot = store.currentSlot.value;
  if (slot?.result) {
    return slot.result.freshness === "stale" ? "stale" : "complete";
  }
  if (slot?.startError) {
    return "failed";
  }
  return "idle";
});

/** 完整状态（含快照时间），放在 title 中供鼠标 / 辅助技术读取。 */
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

/** 当前档位下的简短状态；完整信息保留在 title。 */
const statusShort = computed(() => {
  switch (statusKind.value) {
    case "analysis":
      return store.state.analysis?.kind === "refresh" ? t("刷新中", "Refreshing") : t("分析中", "Analyzing");
    case "cancelled":
      return t("已取消", "Cancelled");
    case "stale":
      return t("旧快照", "Stale snapshot");
    case "complete":
      return isCompactStatus.value ? t("完成", "Done") : t("分析完成", "Analysis complete");
    case "failed":
      return isCompactStatus.value ? t("失败", "Failed") : t("分析失败", "Analysis failed");
    default:
      return isCompactStatus.value ? t("未分析", "Not analyzed") : t("尚未分析", "Not analyzed yet");
  }
});

const statusIcon = computed(() => {
  switch (statusKind.value) {
    case "analysis":
      return RefreshCw;
    case "stale":
      return TriangleAlert;
    case "complete":
      return CircleCheck;
    case "failed":
      return CircleAlert;
    default:
      return CircleDashed;
  }
});

const statusToneClass = computed(() => {
  switch (statusKind.value) {
    case "analysis":
      return "text-info";
    case "stale":
      return "text-caution";
    case "complete":
      return "text-success";
    case "failed":
      return "text-danger";
    default:
      return "text-muted";
  }
});

const refreshLabel = computed(() =>
  hasResult.value
    ? t("手动刷新（F5）", "Refresh manually (F5)")
    : t("尚未分析，无法刷新", "Not analyzed yet; cannot refresh"),
);

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
  { id: "settings", label: t("设置", "Settings") },
  { id: "about", label: t("关于", "About") },
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
    case "about":
      store.actions.setView("about");
      break;
    default:
      break;
  }
}
</script>

<template>
  <header
    class="flex h-14 shrink-0 items-center gap-2 border-b border-line bg-surface px-3"
    data-testid="app-topbar"
  >
    <!-- 左：导航 + 品牌 -->
    <div class="flex min-w-0 shrink-0 items-center gap-2">
      <UiIconButton
        v-if="isNarrow"
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
        <Menu :size="18" aria-hidden="true" />
        <span class="sr-only">{{ store.state.navCollapsed ? t("展开导航", "Expand navigation") : t("折叠导航", "Collapse navigation") }}</span>
      </UiButton>
      <AppBrandMark :size="24" />
      <div class="flex min-w-0 items-baseline gap-1.5">
        <span class="text-[15px] font-semibold whitespace-nowrap">{{ copy.appName }}</span>
        <span v-if="showSubtitle" class="whitespace-nowrap text-xs text-muted" data-testid="brand-subtitle">
          {{ copy.subtitle }}
        </span>
      </div>
    </div>

    <span v-if="showDividers" class="h-5 w-px shrink-0 bg-line" aria-hidden="true"></span>

    <!-- 中：当前存档 -->
    <div class="flex min-w-0 items-center gap-0.5" role="group" :aria-label="t('当前存档', 'Current save')">
      <span
        class="truncate text-[13px] font-medium"
        :class="fileMaxClass"
        data-testid="current-file"
        :title="store.state.selectedPath ?? ''"
      >{{ currentName }}</span>
      <UiIconButton
        :label="t('切换存档（打开导入页）', 'Switch save (open import page)')"
        @click="store.actions.openImport()"
      >
        <ChevronDown :size="16" aria-hidden="true" />
      </UiIconButton>
    </div>

    <div class="flex-1" />

    <!-- 右：状态 + 动作 -->
    <div
      class="flex min-w-0 items-center gap-2"
      :class="showDividers ? 'border-l border-line pl-3' : ''"
    >
      <span
        class="inline-flex min-w-0 items-center gap-1.5 text-[12px]"
        :class="[statusToneClass, statusMaxClass]"
        :title="statusText"
        aria-live="polite"
        aria-atomic="true"
        data-testid="analysis-status"
      >
        <component :is="statusIcon" :size="14" :stroke-width="2" class="shrink-0" aria-hidden="true" />
        <span class="truncate">{{ statusShort }}</span>
      </span>
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
        v-if="!isNarrow"
        size="sm"
        :disabled="!hasResult || store.state.analysis !== null"
        :title="refreshLabel"
        data-testid="topbar-refresh"
        @click="store.actions.refresh()"
      >
        <RefreshCw :size="16" aria-hidden="true" />
        {{ t("刷新", "Refresh") }}
      </UiButton>
      <UiIconButton
        v-else
        :label="refreshLabel"
        :disabled="!hasResult || store.state.analysis !== null"
        data-testid="topbar-refresh"
        @click="store.actions.refresh()"
      >
        <RefreshCw :size="16" aria-hidden="true" />
      </UiIconButton>
      <UiMenu :label="t('更多', 'More')" :items="moreItems" @select="onMore">
        <template #trigger>
          <span v-if="!isNarrow" class="inline-flex items-center gap-1">
            {{ t("更多", "More") }}<ChevronDown :size="14" aria-hidden="true" />
          </span>
          <template v-else>
            <ChevronDown :size="16" aria-hidden="true" />
            <span class="sr-only">{{ t("更多", "More") }}</span>
          </template>
        </template>
      </UiMenu>
    </div>
  </header>
</template>
