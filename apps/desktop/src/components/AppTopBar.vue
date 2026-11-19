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
import { t, tFormatPair } from "../app/i18n";
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
  return path ? fileNameOf(path) : t("appTopBar.no-file-selected");
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
      ? tFormatPair(`appTopBar.refreshing-previous-result-kept`, { v0: name })
      : tFormatPair(`appTopBar.analyzing`, { v0: name });
  }
  if (store.state.lastCancelledPath === store.state.selectedPath && store.state.selectedPath) {
    return t("appTopBar.cancelled");
  }
  const slot = store.currentSlot.value;
  if (slot?.result) {
    if (slot.result.freshness === "stale") {
      return tFormatPair(`appTopBar.stale-snapshot`, { v0: formatSnapshotTime(slot.result.snapshot_time) });
    }
    return tFormatPair(`appTopBar.analysis-complete-snapshot`, { v0: formatSnapshotTime(slot.result.snapshot_time) });
  }
  if (slot?.startError) {
    return tFormatPair(`appTopBar.analysis-failed`, { v0: slot.startError.code });
  }
  return t("appTopBar.not-analyzed-yet");
});

/** 当前档位下的简短状态；完整信息保留在 title。 */
const statusShort = computed(() => {
  switch (statusKind.value) {
    case "analysis":
      return store.state.analysis?.kind === "refresh" ? t("appTopBar.refreshing") : t("appTopBar.analyzing-2");
    case "cancelled":
      return t("appTopBar.cancelled");
    case "stale":
      return t("appTopBar.stale-snapshot-2");
    case "complete":
      return isCompactStatus.value ? t("appTopBar.done") : t("appTopBar.analysis-complete");
    case "failed":
      return isCompactStatus.value ? t("appTopBar.failed") : t("appTopBar.analysis-failed-2");
    default:
      return isCompactStatus.value ? t("appTopBar.not-analyzed") : t("appTopBar.not-analyzed-yet");
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
    ? t("appTopBar.refresh-manually-f5")
    : t("appTopBar.not-analyzed-yet-cannot-refresh"),
);

const moreItems = computed<MenuItem[]>(() => [
  store.canExport.value
    ? { id: "export", label: t("appTopBar.export-current-snapshot-json-csv") }
    : {
        id: "export",
        label: t("appTopBar.export-current-snapshot-json-csv"),
        disabled: true,
        reason: store.state.analysis
          ? t("appTopBar.analysis-refresh-in-progress-export-after")
          : t("appTopBar.no-report-to-export-analyze-the-current"),
      },
  { id: "rescan", label: t("appTopBar.rescan-default-folders") },
  { id: "diagnostics", label: t("common.diagnostics") },
  { id: "settings", label: t("common.settings") },
  { id: "about", label: t("common.about") },
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
        :label="t('appTopBar.open-navigation-menu')"
        :pressed="store.state.navOverlayOpen"
        @click="store.actions.toggleNav()"
      >
        <Menu :size="18" aria-hidden="true" />
      </UiIconButton>
      <UiButton
        v-else
        size="sm"
        variant="ghost"
        :title="store.state.navCollapsed ? t('appTopBar.expand-navigation') : t('appTopBar.collapse-navigation')"
        @click="store.actions.toggleNav()"
      >
        <Menu :size="18" aria-hidden="true" />
        <span class="sr-only">{{ store.state.navCollapsed ? t("appTopBar.expand-navigation") : t("appTopBar.collapse-navigation") }}</span>
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
    <div class="flex min-w-0 items-center gap-0.5" role="group" :aria-label="t('appTopBar.current-save')">
      <span
        class="truncate text-[13px] font-medium"
        :class="fileMaxClass"
        data-testid="current-file"
        :title="store.state.selectedPath ?? ''"
      >{{ currentName }}</span>
      <UiIconButton
        :label="t('appTopBar.switch-save-open-import-page')"
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
        :title="t('appTopBar.cancel-the-current-analysis-no-new-report')"
        @click="store.actions.cancelAnalysis()"
      >
        {{ t("common.cancel") }}
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
        {{ t("app.refresh") }}
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
      <UiMenu :label="t('appTopBar.more')" :items="moreItems" @select="onMore">
        <template #trigger>
          <span v-if="!isNarrow" class="inline-flex items-center gap-1">
            {{ t("appTopBar.more") }}<ChevronDown :size="14" aria-hidden="true" />
          </span>
          <template v-else>
            <ChevronDown :size="16" aria-hidden="true" />
            <span class="sr-only">{{ t("appTopBar.more") }}</span>
          </template>
        </template>
      </UiMenu>
    </div>
  </header>
</template>
