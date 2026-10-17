<script setup lang="ts">
// 应用外壳（SX-015）：顶栏 + 侧栏 + 主工作区 + 详情（常驻/抽屉/页面替换）。
// 布局按可用内容宽度分档（800/1100/1440），详情方式与焦点行为随档位切换。
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { ArrowLeft, X } from "@lucide/vue";
import { provideStore } from "./app/inject";
import { createAppStore, type AppStore, type LayoutMode } from "./app/store";
import { productionMetadata } from "./data/metadata";
import { demoObservedIds } from "./data/atlas";
import { createDemoSource } from "./data/mock";
import { TauriDataSource, isTauriRuntime } from "./data/tauri";
import type { DataSource } from "./data/source";
import AppSidebar from "./components/AppSidebar.vue";
import AppTopBar from "./components/AppTopBar.vue";
import ExportDialog from "./components/ExportDialog.vue";
import MockBanner from "./components/MockBanner.vue";
import SlotPicker from "./components/SlotPicker.vue";
import OverviewPage from "./components/OverviewPage.vue";
import FullCollectionPage from "./components/FullCollectionPage.vue";
import CollectionPage from "./components/CollectionPage.vue";
import DiagnosticsPage from "./components/DiagnosticsPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
import CollectionDetail from "./components/CollectionDetail.vue";
import UiButton from "./components/ui/UiButton.vue";
import UiDrawer from "./components/ui/UiDrawer.vue";
import UiIconButton from "./components/ui/UiIconButton.vue";

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
    case "regions":
      return CollectionPage;
    case "diagnostics":
      return DiagnosticsPage;
    case "settings":
      return SettingsPage;
    default:
      return SlotPicker;
  }
});

const selectedEntry = computed(() => store.selectedEntry.value);

const showPersistentPane = computed(
  () => layout.value === "wide" && store.state.detailOpen && selectedEntry.value !== null && store.state.view !== "import",
);
const showDrawer = computed(
  () =>
    (layout.value === "medium" || layout.value === "compact") &&
    store.state.detailOpen &&
    selectedEntry.value !== null &&
    store.state.view !== "import",
);
const showNarrowDetail = computed(
  () => layout.value === "narrow" && store.state.detailOpen && selectedEntry.value !== null && store.state.view !== "import",
);

const drawerWidthClass = computed(() =>
  layout.value === "compact" ? "w-[min(720px,calc(100vw-24px))]" : "w-[400px] max-w-[calc(100vw-24px)]",
);

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
      store.actions.setView("regions");
      store.actions.requestSearchFocus();
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

    <div class="flex min-h-0 flex-1">
      <AppSidebar v-if="layout !== 'narrow' && !showNarrowDetail" :expanded="sidebarExpanded" />

      <div class="relative flex min-h-0 min-w-0 flex-1">
        <main class="flex min-h-0 min-w-0 flex-1 flex-col">
          <div
            v-if="store.state.notice"
            class="flex items-start gap-2 border-b border-line bg-raised px-3 py-1.5 text-[13px]"
            :class="
              store.state.notice.kind === 'error'
                ? 'text-danger'
                : store.state.notice.kind === 'warning'
                  ? 'text-caution'
                  : store.state.notice.kind === 'success'
                    ? 'text-success'
                    : 'text-info'
            "
            role="status"
            aria-live="polite"
            data-testid="notice"
          >
            <span class="min-w-0 flex-1">{{ store.state.notice.text }}</span>
            <UiIconButton label="关闭提示" @click="store.actions.dismissNotice()">
              <X :size="14" aria-hidden="true" />
            </UiIconButton>
          </div>

          <template v-if="showNarrowDetail && selectedEntry">
            <header class="flex h-12 shrink-0 items-center gap-2 border-b border-line px-3">
              <UiButton size="sm" variant="ghost" data-testid="detail-back" @click="store.actions.closeDetail()">
                <ArrowLeft :size="16" aria-hidden="true" />
                返回清单
              </UiButton>
              <span class="truncate text-xs text-muted">{{ store.currentSlot.value?.name ?? "" }}</span>
            </header>
            <div class="scroll-area min-h-0 flex-1 px-4 py-3">
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
          class="w-[400px] shrink-0 border-l border-line bg-surface"
          aria-label="条目详情"
          data-testid="detail-pane"
        >
          <div class="scroll-area h-full px-4 py-3">
            <CollectionDetail
              :region="selectedEntry.region"
              :group="selectedEntry.group"
              :item="selectedEntry.item"
              mode="pane"
            />
          </div>
        </aside>
      </div>
    </div>

    <!-- 浮层导航（中等/紧凑/窄窗）：不挤窄清单；点击背景或选择后关闭 -->
    <div v-if="showNavOverlay" class="absolute inset-x-0 top-12 bottom-0 z-30 flex bg-[var(--overlay)]" @click.self="store.actions.setNavOverlay(false)">
      <div class="flex h-full flex-col bg-surface shadow-[var(--shadow)]">
        <AppSidebar :expanded="true" />
      </div>
    </div>

    <UiDrawer
      v-if="showDrawer && selectedEntry"
      :open="true"
      :title="store.currentSlot.value?.name ? `详情 · ${store.currentSlot.value.name}` : '详情'"
      :width-class="drawerWidthClass"
      @close="store.actions.closeDetail()"
    >
      <CollectionDetail
        :region="selectedEntry.region"
        :group="selectedEntry.group"
        :item="selectedEntry.item"
        mode="drawer"
      />
    </UiDrawer>

    <!-- 导出对话框（SX-016）：任何视图/布局下都可打开；模态焦点规则同抽屉 -->
    <ExportDialog />
  </div>
</template>
