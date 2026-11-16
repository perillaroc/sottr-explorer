<script setup lang="ts">
// 应用外壳（SX-015）：顶栏 + 侧栏 + 主工作区 + 详情（常驻/抽屉/页面替换）。
// 布局按可用内容宽度分档（800/1100/1440），详情方式与焦点行为随档位切换。
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { ArrowLeft, CircleAlert, CircleCheck, Info, TriangleAlert, X } from "@lucide/vue";
import { provideStore } from "./app/inject";
import { t, tFormat } from "./app/i18n";
import { createAppStore, type AppStore, type LayoutMode } from "./app/store";
import {
  DETAIL_COMPACT_WIDTH,
  DETAIL_DRAWER_WIDTH,
  DETAIL_PANE_WIDTH,
} from "./app/prefs";
import { productionMetadata } from "./data/metadata";
import { demoObservedIds } from "./data/atlas";
import { createDemoSource } from "./data/mock";
import { TauriDataSource, isTauriRuntime } from "./data/tauri";
import type { DataSource } from "./data/source";
import AppSidebar from "./components/AppSidebar.vue";
import AppTopBar from "./components/AppTopBar.vue";
import ExportDialog from "./components/ExportDialog.vue";
import PlanExportDialog from "./components/PlanExportDialog.vue";
import ShareReportDialog from "./components/ShareReportDialog.vue";
import MockBanner from "./components/MockBanner.vue";
import SlotPicker from "./components/SlotPicker.vue";
import OverviewPage from "./components/OverviewPage.vue";
import FullCollectionPage from "./components/FullCollectionPage.vue";
import PlanPage from "./components/PlanPage.vue";
import CollectionPage from "./components/CollectionPage.vue";
import MapPage from "./components/MapPage.vue";
import DiagnosticsPage from "./components/DiagnosticsPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
import AboutPage from "./components/AboutPage.vue";
import CollectionDetail from "./components/CollectionDetail.vue";
import CoverageNotice from "./components/CoverageNotice.vue";
import DetailWindow from "./components/DetailWindow.vue";
import UiButton from "./components/ui/UiButton.vue";
import UiDialog from "./components/ui/UiDialog.vue";
import UiDrawer from "./components/ui/UiDrawer.vue";
import UiIconButton from "./components/ui/UiIconButton.vue";
import UiResizeHandle from "./components/ui/UiResizeHandle.vue";

const props = withDefaults(
  defineProps<{
    source?: DataSource;
    /** 测试用：直接注入已构建的 store（避免重复创建）。 */
    store?: AppStore;
    /** 测试用：直接指定有效宽度，避免依赖窗口尺寸。 */
    layoutWidth?: number;
  }>(),
  { source: undefined, store: undefined, layoutWidth: undefined },
);

/**
 * 选择数据源（SX-016）：
 * - Tauri WebView 内 → `TauriDataSource`（真实命令）与生产元数据；
 * - `?mock=1`（仅开发/演示）强制模拟数据源，界面常驻“模拟数据”横幅；
 * - 纯浏览器（`pnpm dev` 直接打开）没有 IPC，退化为模拟数据源。
 */
function createDefaultSource(): DataSource {
  const params =
    typeof window !== "undefined" ? new URLSearchParams(window.location.search) : new URLSearchParams();
  const forceMock = params.get("mock") === "1";
  if (isTauriRuntime() && !forceMock) {
    return new TauriDataSource();
  }
  return createDemoSource({ failGuideOpen: params.get("mockLinkFail") === "1" });
}

const source = props.source ?? createDefaultSource();
const store =
  props.store ??
  createAppStore(source, {
    layoutWidth: props.layoutWidth,
    metadata: source.kind === "tauri" ? productionMetadata : undefined,
    // 模拟模式使用固定观察集合，让演示与测试可复现；真实模式读取当前报告。
    observedIds: source.kind === "mock" ? () => demoObservedIds : undefined,
  });
provideStore(store);

const root = ref<HTMLElement | null>(null);
let observer: ResizeObserver | null = null;

const layout = computed<LayoutMode>(() => store.layout.value);
const pageComponent = computed(() => {
  switch (store.state.view) {
    case "overview":
      return OverviewPage;
    case "full":
      return FullCollectionPage;
    case "plan":
      return PlanPage;
    case "regions":
      return CollectionPage;
    case "map":
      return MapPage;
    case "diagnostics":
      return DiagnosticsPage;
    case "settings":
      return SettingsPage;
    case "about":
      return AboutPage;
    default:
      return SlotPicker;
  }
});

const selectedEntry = computed(() => store.selectedEntry.value);

// 存档监视提示（P1-4）：只提示“是否刷新”，不自动分析。
const inputWatchAlert = computed(() => {
  switch (store.state.inputWatch.status) {
    case "changed":
      return {
        tone: "warning" as const,
        title: t("当前存档已更新", "The current save has changed"),
        lines: [
          t("检测到存档文件发生变化；是否刷新？应用不会自动分析。", "A change was detected. Refresh now? The app never analyzes automatically."),
        ],
        action: "refresh" as const,
      };
    case "missing":
      return {
        tone: "danger" as const,
        title: t("当前存档已不在原位置", "The current save is no longer at its path"),
        lines: [
          t("文件可能被移动、删除，或仍在原子替换中；请重新选择存档文件。", "The file may have been moved, deleted, or is still being replaced atomically. Choose the save file again."),
        ],
        action: "reselect" as const,
      };
    case "unreadable":
      return {
        tone: "warning" as const,
        title: t("当前存档暂时不可读", "The current save is temporarily unreadable"),
        lines: [
          t("可能正在保存或被其他程序占用；应用会继续只读重试，不会自动刷新。", "It may be saving or locked by another program. The app keeps retrying read-only and never refreshes automatically."),
          tFormat("原因码：{code}", "Code: {code}", { code: store.state.inputWatch.code ?? t("未知", "unknown") }),
        ],
        action: "retry" as const,
      };
    default:
      return null;
  }
});

// 全局通知条（阶段二）：状态图标 + 文本 + 关闭，保持 live region；不使用自动消失 toast。
const noticeTone = computed(() => {
  switch (store.state.notice?.kind) {
    case "error":
      return "danger";
    case "warning":
      return "caution";
    case "success":
      return "success";
    default:
      return "info";
  }
});

const noticeIcon = computed(() => {
  switch (noticeTone.value) {
    case "danger":
      return CircleAlert;
    case "caution":
      return TriangleAlert;
    case "success":
      return CircleCheck;
    default:
      return Info;
  }
});

const noticeClass = computed(() => {
  switch (noticeTone.value) {
    case "danger":
      return "border-danger/40 bg-danger-bg text-danger";
    case "caution":
      return "border-caution/40 bg-caution-bg text-caution";
    case "success":
      return "border-success/40 bg-success-bg text-success";
    default:
      return "border-info/30 bg-info-bg text-info";
  }
});

const detailViewActive = computed(
  () => store.state.view === "overview" || store.state.view === "full" || store.state.view === "regions",
);

const showPersistentPane = computed(
  () => layout.value === "wide" && detailViewActive.value && store.state.detailOpen && selectedEntry.value !== null,
);
const showDrawer = computed(
  () =>
    (layout.value === "medium" || layout.value === "compact") &&
    detailViewActive.value &&
    store.state.detailOpen &&
    selectedEntry.value !== null,
);
const showNarrowDetail = computed(
  () => layout.value === "narrow" && detailViewActive.value && store.state.detailOpen && selectedEntry.value !== null,
);

// ---------------------------------------------------------------------------
// 详情宽度（阶段一）：左边缘手柄拖动/键盘调整，宽度写入偏好；拖动期间只在本地
// 预览（draft），松手（commit）才落偏好，避免拖动中频繁写 localStorage。
// ---------------------------------------------------------------------------

/** 详情拉开时给主内容保留的最小宽度。 */
const MAIN_MIN_WIDTH = 320;
const SIDEBAR_RAIL_WIDTH = 64;
const SIDEBAR_EXPANDED_WIDTH = 224;

const sidebarWidth = computed(() =>
  layout.value === "wide" && !store.state.navCollapsed ? SIDEBAR_EXPANDED_WIDTH : SIDEBAR_RAIL_WIDTH,
);

function clampWidth(value: number, min: number, max: number): number {
  const upper = Math.max(min, max);
  return Math.min(upper, Math.max(min, Math.round(value)));
}

/** 常驻详情上限：视口可用宽度 − 侧栏 − 主内容最小宽度，且不超过硬上限。 */
function paneMaxWidth(): number {
  const available = store.state.layoutWidth - sidebarWidth.value - MAIN_MIN_WIDTH;
  return Math.max(DETAIL_PANE_WIDTH.min, Math.min(DETAIL_PANE_WIDTH.max, available));
}

/** 抽屉上限：紧凑档允许接近全宽（保留 24px 遮罩边距），其余档给主内容留 320px。 */
function drawerMaxWidth(): number {
  const reserved = layout.value === "compact" ? 24 : MAIN_MIN_WIDTH;
  return Math.max(
    DETAIL_DRAWER_WIDTH.min,
    Math.min(DETAIL_DRAWER_WIDTH.max, store.state.layoutWidth - reserved),
  );
}

const paneDraft = ref<number | null>(null);
const drawerDraft = ref<number | null>(null);

const detailPaneWidth = computed(() =>
  clampWidth(paneDraft.value ?? store.state.detailPaneWidth, DETAIL_PANE_WIDTH.min, paneMaxWidth()),
);

const drawerDefaultWidth = computed(() =>
  layout.value === "compact" ? DETAIL_COMPACT_WIDTH.default : DETAIL_DRAWER_WIDTH.default,
);

const detailDrawerWidth = computed(() => {
  const stored = layout.value === "compact" ? store.state.detailCompactWidth : store.state.detailDrawerWidth;
  return clampWidth(drawerDraft.value ?? stored, DETAIL_DRAWER_WIDTH.min, drawerMaxWidth());
});

function commitPaneWidth(width: number): void {
  paneDraft.value = null;
  store.actions.setDetailPaneWidth(width);
}

function commitDrawerWidth(width: number): void {
  drawerDraft.value = null;
  if (layout.value === "compact") {
    store.actions.setDetailCompactWidth(width);
  } else {
    store.actions.setDetailDrawerWidth(width);
  }
}

const sidebarExpanded = computed(() => layout.value === "wide" && !store.state.navCollapsed);
const showNavOverlay = computed(() => store.state.navOverlayOpen && layout.value !== "wide");

const rootStyle = computed(() =>
  store.state.mockWidth
    ? {
        maxWidth: `${store.state.mockWidth}px`,
        margin: "0 auto",
        // 用 outline（不影响布局尺寸）标注演示视口，保证“有效内容宽度”与标注一致。
        outline: "1px dashed var(--border)",
      }
    : {},
);

function updateWidth(): void {
  if (props.layoutWidth !== undefined) {
    store.actions.setLayoutWidth(props.layoutWidth);
    return;
  }
  const element = root.value;
  const width = element?.clientWidth ?? window.innerWidth ?? 1280;
  store.actions.setLayoutWidth(width > 0 ? width : 1280);
}

function onKeydown(event: KeyboardEvent): void {
  const target = event.target as HTMLElement | null;
  const inEditable =
    target !== null && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "o") {
    event.preventDefault();
    void store.actions.pickInput("save-file");
    return;
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
    if (!inEditable) {
      event.preventDefault();
      if (store.state.view === "map") {
        store.actions.requestMapSearchFocus();
      } else {
        store.actions.setView("regions");
        store.actions.requestSearchFocus();
      }
    }
    return;
  }
  if (event.key === "F5") {
    event.preventDefault();
    void store.actions.refresh();
    return;
  }
  if (event.key === "Escape" && showNarrowDetail.value) {
    event.preventDefault();
    store.actions.closeDetail();
  }
}

onMounted(async () => {
  if (source.kind === "mock" && typeof window !== "undefined") {
    const mockWidth = Number(new URLSearchParams(window.location.search).get("mockWidth"));
    if (Number.isFinite(mockWidth) && mockWidth > 0) {
      store.actions.setMockWidth(mockWidth);
    }
  }
  // 等样式（演示视口宽度）应用后再测量，避免用旧宽度计算布局档位。
  await nextTick();
  updateWidth();
  if (!props.layoutWidth && typeof ResizeObserver !== "undefined" && root.value) {
    observer = new ResizeObserver(() => updateWidth());
    observer.observe(root.value);
  } else if (!props.layoutWidth) {
    window.addEventListener("resize", updateWidth);
  }
  window.addEventListener("keydown", onKeydown);
  await store.actions.boot();
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  window.removeEventListener("resize", updateWidth);
  window.removeEventListener("keydown", onKeydown);
  store.actions.dispose();
});
</script>

<template>
  <div
    ref="root"
    class="relative mx-auto flex h-full min-h-0 flex-col overflow-hidden bg-canvas text-ink"
    :style="rootStyle"
    data-testid="app-root"
    :data-layout="layout"
  >
    <MockBanner v-if="store.state.sourceKind === 'mock'" />
    <AppTopBar />

    <div class="relative flex min-h-0 flex-1">
      <AppSidebar v-if="layout !== 'narrow' && !showNarrowDetail" :expanded="sidebarExpanded" />

      <div class="relative flex min-h-0 min-w-0 flex-1">
        <main class="flex min-h-0 min-w-0 flex-1 flex-col">
          <CoverageNotice
            v-if="inputWatchAlert"
            class="mx-3 mt-3"
            :tone="inputWatchAlert.tone"
            test-id="input-watch-alert"
            :title="inputWatchAlert.title"
            :lines="inputWatchAlert.lines"
          >
            <UiButton
              v-if="inputWatchAlert.action === 'refresh'"
              size="sm"
              variant="primary"
              data-testid="input-watch-refresh"
              @click="store.actions.refreshFromWatch()"
            >{{ t("刷新", "Refresh") }}</UiButton>
            <UiButton
              v-if="inputWatchAlert.action === 'reselect'"
              size="sm"
              variant="primary"
              data-testid="input-watch-reselect"
              @click="store.actions.reselectFromWatch()"
            >{{ t("重新选择", "Choose another file") }}</UiButton>
            <UiButton
              size="sm"
              variant="ghost"
              data-testid="input-watch-dismiss"
              @click="store.actions.dismissInputWatch()"
            >{{ t("稍后", "Later") }}</UiButton>
          </CoverageNotice>

          <div
            v-if="store.state.notice"
            class="flex items-start gap-2 border-b px-4 py-1.5 text-[13px]"
            :class="noticeClass"
            role="status"
            aria-live="polite"
            data-testid="notice"
          >
            <component :is="noticeIcon" :size="16" :stroke-width="2" class="mt-0.5 shrink-0" aria-hidden="true" />
            <span class="min-w-0 flex-1">{{ store.state.notice.text }}</span>
            <UiIconButton :label="t('关闭提示', 'Dismiss notice')" @click="store.actions.dismissNotice()">
              <X :size="14" aria-hidden="true" />
            </UiIconButton>
          </div>

          <template v-if="showNarrowDetail && selectedEntry">
            <header class="flex h-12 shrink-0 items-center gap-2 border-b border-line px-4">
              <UiButton size="sm" variant="ghost" data-testid="detail-back" @click="store.actions.closeDetail()">
                <ArrowLeft :size="16" aria-hidden="true" />
                {{ t("返回清单", "Back to list") }}
              </UiButton>
              <span class="truncate text-xs text-muted">{{ store.currentSlot.value?.name ?? "" }}</span>
            </header>
            <div class="scroll-area min-h-0 flex-1 px-4 py-4">
              <CollectionDetail
                :region="selectedEntry.region"
                :group="selectedEntry.group"
                :item="selectedEntry.item"
                mode="page"
              />
            </div>
          </template>
          <component v-else :is="pageComponent" />
        </main>

        <aside
          v-if="showPersistentPane && selectedEntry"
          class="relative shrink-0 border-l border-line bg-surface"
          :style="{ width: `${detailPaneWidth}px` }"
          :aria-label="t('条目详情', 'Item details')"
          data-testid="detail-pane"
        >
          <UiResizeHandle
            data-testid="detail-pane-resize"
            :width="detailPaneWidth"
            :min="DETAIL_PANE_WIDTH.min"
            :max="paneMaxWidth"
            :default-width="DETAIL_PANE_WIDTH.default"
            :label="t('调整详情宽度（拖动、方向键或双击复位）', 'Resize details (drag, arrow keys, or double-click to reset)')"
            @resize="paneDraft = $event"
            @commit="commitPaneWidth"
          />
          <div class="scroll-area h-full px-5 py-4">
            <CollectionDetail
              :region="selectedEntry.region"
              :group="selectedEntry.group"
              :item="selectedEntry.item"
              mode="pane"
            />
          </div>
        </aside>
      </div>

      <!-- 浮层导航（中等/紧凑/窄窗）：相对内容行定位，不覆盖顶栏 / 模拟横幅；
           点击背景或选择后关闭。 -->
      <div
        v-if="showNavOverlay"
        class="absolute inset-0 z-30 flex bg-[var(--overlay)]"
        data-testid="nav-overlay"
        @click.self="store.actions.setNavOverlay(false)"
      >
        <div class="flex h-full flex-col bg-surface shadow-[var(--shadow)]">
          <AppSidebar :expanded="true" />
        </div>
      </div>
    </div>

    <UiDrawer
      v-if="showDrawer && selectedEntry"
      :open="true"
      :title="store.currentSlot.value?.name ? t(`详情 · ${store.currentSlot.value.name}`, `Details · ${store.currentSlot.value.name}`) : t('详情', 'Details')"
      :width="detailDrawerWidth"
      @close="store.actions.closeDetail()"
    >
      <template #handle>
        <UiResizeHandle
          data-testid="detail-drawer-resize"
          :width="detailDrawerWidth"
          :min="DETAIL_DRAWER_WIDTH.min"
          :max="drawerMaxWidth"
          :default-width="drawerDefaultWidth"
          :label="t('调整详情宽度（拖动、方向键或双击复位）', 'Resize details (drag, arrow keys, or double-click to reset)')"
          @resize="drawerDraft = $event"
          @commit="commitDrawerWidth"
        />
      </template>
      <CollectionDetail
        :region="selectedEntry.region"
        :group="selectedEntry.group"
        :item="selectedEntry.item"
        mode="drawer"
      />
    </UiDrawer>

    <!-- 导出对话框（SX-016）：任何视图/布局下都可打开；模态焦点规则同抽屉 -->
    <ExportDialog />
    <!-- 收集计划清单导出（P1-2）：只从计划页打开，写入新目录 -->
    <PlanExportDialog />
    <!-- 分享报告（P1-5）：自包含 HTML，固定分享口径 -->
    <ShareReportDialog />

    <!-- 详情独立窗口失败时的应用内兜底（阶段三）：同一份只读 payload，模态焦点规则同对话框 -->
    <UiDialog
      v-if="store.state.detailWindowFallback.open && store.state.detailWindowFallback.payload"
      :open="true"
      :title="t('收集品详情', 'Item details')"
      width-class="w-[640px] max-w-[calc(100vw-24px)]"
      test-id="detail-fallback-dialog"
      @close="store.actions.closeDetailWindowFallback()"
    >
      <DetailWindow
        mode="inline"
        :payload="store.state.detailWindowFallback.payload"
        @close="store.actions.closeDetailWindowFallback()"
      />
    </UiDialog>
  </div>
</template>
