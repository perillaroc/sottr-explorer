// 应用状态（SX-015）：Vue reactive 单一 store（不引入状态管理依赖）。
//
// 关键规则（开发方案“状态流与恢复规则”）：
// - 每次分析分配 request_id；只有“当前请求”的结果可以提交（取消、切槽或
//   晚到的旧结果一律丢弃），保证 A 槽结果不会出现在 B 槽标题下；
// - 刷新失败时保留旧报告并标记 stale（契约信封携带旧报告与刷新错误码）；
//   陈旧报告不能显示为“当前遗漏”；
// - 筛选偏好跨槽保留，但每次切槽重新校验可用性（无效筛选回退并说明原因）；
// - 界面不重新计算领域推定：遗漏、门禁与定位精度全部读取报告结论。
import { computed, reactive, ref, type ComputedRef } from "vue";
import type { DesktopResult } from "../contract/desktop-result";
import type { Report } from "../contract/report";
import type { MetadataDataset } from "../contract/metadata";
import type {
  AnalyzeRequest,
  CandidateFileDto,
  DataSource,
  DiscoverOutcomeDto,
  InputError,
  Purpose,
} from "../data/source";
import { demoMetadata } from "../data/metadata";
import {
  buildRows,
  filterCounts as computeFilterCounts,
  missingCapabilityAvailable,
  missingCount as computeMissingCount,
  observedCount as computeObservedCount,
  unknownCount as computeUnknownCount,
  type CollectibleRow,
  type FilterState,
  type RegionScope,
  type StatusFilter,
} from "./catalog";
import { fileNameOf, shortId } from "./format";
import { applyTheme, onSystemThemeChange, type ThemePreference } from "./theme";
import { loadPreferences, savePreferences, type UiPreferences, type ViewId } from "./prefs";

export type LayoutMode = "wide" | "medium" | "compact" | "narrow";

export interface SlotEntry {
  path: string;
  name: string;
  /** 已提交的结果（current 或 stale）；null 表示尚未成功分析。 */
  result: DesktopResult | null;
  /** 首次分析失败（尚未产生报告）；成功后清空。 */
  startError: InputError | null;
}

export interface Notice {
  kind: "info" | "success" | "warning" | "error";
  text: string;
}

export interface AnalysisState {
  requestId: string;
  path: string;
  kind: "start" | "refresh";
  startedAt: number;
}

export interface AppState {
  booted: boolean;
  /** 数据源类型：mock 时界面显著标注模拟数据。 */
  sourceKind: "mock" | "tauri";
  discovery: DiscoverOutcomeDto | null;
  discoveryError: InputError | null;
  activeRoot: string | null;
  candidates: CandidateFileDto[];
  loadingCandidates: boolean;
  selectedPath: string | null;
  /** 分析用途必须由用户显式确认（未选时禁用开始分析）。 */
  purpose: Purpose | null;
  analysis: AnalysisState | null;
  slots: Record<string, SlotEntry>;
  lastCancelledPath: string | null;
  view: ViewId;
  region: RegionScope;
  filters: FilterState;
  moreFiltersOpen: boolean;
  search: string;
  selectedRowId: string | null;
  layoutWidth: number;
  navCollapsed: boolean;
  navOverlayOpen: boolean;
  detailOpen: boolean;
  theme: ThemePreference;
  notice: Notice | null;
  /** 模拟模式下的演示视口宽度（?mockWidth=800），用于宽度验收；null 为跟随窗口。 */
  mockWidth: number | null;
  listScrollTop: number;
  /** Ctrl+F 焦点请求计数：区域页监听并聚焦搜索框。 */
  searchFocusToken: number;
}

export interface CreateStoreOptions {
  metadata?: MetadataDataset;
  /** 是否读写 localStorage（测试传 false）。 */
  persist?: boolean;
  /** 初始视口宽度（测试直接指定，避免依赖窗口尺寸）。 */
  layoutWidth?: number;
}

export function layoutFor(width: number): LayoutMode {
  if (width >= 1440) {
    return "wide";
  }
  if (width >= 1100) {
    return "medium";
  }
  if (width >= 800) {
    return "compact";
  }
  return "narrow";
}

export function createAppStore(source: DataSource, options: CreateStoreOptions = {}) {
  const metadata = options.metadata ?? demoMetadata;
  const persist = options.persist !== false;
  const preferences = persist ? loadPreferences() : null;

  const state = reactive<AppState>({
    booted: false,
    sourceKind: source.kind,
    discovery: null,
    discoveryError: null,
    activeRoot: null,
    candidates: [],
    loadingCandidates: false,
    selectedPath: null,
    purpose: null,
    analysis: null,
    slots: {},
    lastCancelledPath: null,
    view: preferences?.view ?? "import",
    region: preferences?.region ?? "all",
    filters: { ...(preferences?.filters ?? { status: "missing", category: null, evidence: null, precision: null }) },
    moreFiltersOpen: false,
    search: "",
    selectedRowId: null,
    layoutWidth: options.layoutWidth ?? 1280,
    navCollapsed: preferences?.navCollapsed ?? false,
    navOverlayOpen: false,
    detailOpen: true,
    theme: preferences?.theme ?? "system",
    notice: null,
    mockWidth: null,
    listScrollTop: 0,
    searchFocusToken: 0,
  });

  let requestCounter = 0;
  const systemThemeListener = ref<(() => void) | null>(null);

  function persistNow(): void {
    if (!persist) {
      return;
    }
    const snapshot: UiPreferences = {
      theme: state.theme,
      navCollapsed: state.navCollapsed,
      view: state.view,
      region: state.region,
      filters: { ...state.filters },
    };
    savePreferences(snapshot);
  }

  function notify(kind: Notice["kind"], text: string): void {
    state.notice = { kind, text };
  }

  function newRequestId(): string {
    requestCounter += 1;
    return `req-${Date.now().toString(36)}-${requestCounter}`;
  }

  function slotOf(path: string): SlotEntry {
    const existing = state.slots[path];
    if (existing) {
      return existing;
    }
    const created: SlotEntry = { path, name: fileNameOf(path), result: null, startError: null };
    state.slots[path] = created;
    return created;
  }

  const layout = computed<LayoutMode>(() => layoutFor(state.layoutWidth));

  const currentSlot = computed<SlotEntry | null>(() => {
    if (!state.selectedPath) {
      return null;
    }
    return state.slots[state.selectedPath] ?? null;
  });

  const report = computed<Report | null>(() => currentSlot.value?.result?.report ?? null);
  const isStale = computed<boolean>(() => currentSlot.value?.result?.freshness === "stale");
  const refreshing = computed<boolean>(
    () => state.analysis?.kind === "refresh" && state.analysis.path === state.selectedPath,
  );
  const missingAvailable = computed<boolean>(() => (report.value ? missingCapabilityAvailable(report.value) : false));

  const rows = computed<CollectibleRow[]>(() => {
    if (!report.value) {
      return [];
    }
    return buildRows({
      report: report.value,
      metadata,
      region: state.region,
      filters: state.filters,
      search: state.search,
      shortIdOf: shortId,
    });
  });

  const filterCounts = computed<Record<StatusFilter, number>>(() =>
    report.value ? computeFilterCounts(metadata, report.value, state.region) : { missing: 0, observed: 0, unknown: 0, all: 0 },
  );

  const missingTotal = computed(() => (report.value ? computeMissingCount(report.value) : 0));
  const observedTotal = computed(() => (report.value ? computeObservedCount(report.value) : 0));
  const unknownTotal = computed(() => (report.value ? computeUnknownCount(report.value) : 0));

  // -------------------------------------------------------------------------
  // 请求隔离与提交
  // -------------------------------------------------------------------------

  async function beginAnalyze(path: string, kind: "start" | "refresh"): Promise<void> {
    if (state.analysis) {
      return;
    }
    const request: AnalyzeRequest = {
      requestId: newRequestId(),
      path,
      purpose: state.purpose ?? "unknown",
    };
    state.analysis = { requestId: request.requestId, path, kind, startedAt: Date.now() };
    const result = await source.analyze(request);
    commitResult(request.requestId, kind, result);
  }

  function commitResult(requestId: string, kind: "start" | "refresh", result: DesktopResult): void {
    // 只有当前请求可以提交：取消后到达、切槽后到达或已被新请求替换的结果一律丢弃。
    if (!state.analysis || state.analysis.requestId !== requestId) {
      return;
    }
    const path = state.analysis.path;
    state.analysis = null;
    const slot = slotOf(path);

    // 取消：无结果信封（freshness=none，report/error 均为 null）→ 保留原视图。
    if (result.report === null && result.error === null) {
      state.lastCancelledPath = path;
      notify("info", "已取消：保留原视图，未产生新报告。");
      return;
    }

    // 成功或过期（携带报告）：提交为当前槽位结果。
    if (result.report !== null && result.freshness !== "none") {
      slot.result = result;
      slot.startError = null;
      if (result.freshness === "stale") {
        notify("warning", "刷新失败：已保留旧快照并标记过期（未反映本次读取）。");
      } else if (kind === "start") {
        state.view = "overview";
      }
      ensureFilterValid();
      return;
    }

    // 无报告的错误：刷新失败且有旧报告时保留旧报告（契约应给出 stale，这里作为兜底）。
    const error = result.error;
    if (error) {
      if (slot.result) {
        notify("warning", `刷新失败：${error.message}（保留上次结果）`);
      } else {
        slot.startError = error;
      }
    }
  }

  /** 无效筛选回退（如参考差集不可用时“参考遗漏推定”不可用）。 */
  function ensureFilterValid(): void {
    if (!report.value) {
      return;
    }
    if (state.filters.status === "missing" && !missingCapabilityAvailable(report.value)) {
      state.filters.status = "all";
      notify("warning", "本次无法判断遗漏：参考差集能力不可用，已切换到“全部”条目。");
    }
    if (state.selectedRowId && !rows.value.some((row) => row.id === state.selectedRowId)) {
      state.selectedRowId = null;
    }
  }

  // -------------------------------------------------------------------------
  // 发现与候选
  // -------------------------------------------------------------------------

  async function boot(): Promise<void> {
    if (state.booted) {
      return;
    }
    state.booted = true;
    applyTheme(state.theme);
    systemThemeListener.value = onSystemThemeChange(() => {
      if (state.theme === "system") {
        applyTheme("system");
      }
    });
    await discover();
  }

  async function discover(): Promise<void> {
    state.discoveryError = null;
    try {
      const discovery = await source.discover();
      state.discovery = discovery;
      const firstRoot = discovery.game_roots[0] ?? null;
      if (firstRoot) {
        await openRoot(firstRoot);
      } else {
        state.activeRoot = null;
        state.candidates = [];
      }
    } catch (error) {
      state.discoveryError = { code: "DISCOVERY_FAILED", message: String(error), retryable: true };
    }
  }

  async function rescan(): Promise<void> {
    await discover();
    notify("info", "已重新查找默认目录（模拟：演示候选）。");
  }

  async function openRoot(root: string): Promise<void> {
    state.activeRoot = root;
    state.loadingCandidates = true;
    try {
      const listing = await source.listCandidates(root);
      state.candidates = listing.files;
    } finally {
      state.loadingCandidates = false;
    }
  }

  function selectCandidate(path: string): void {
    state.selectedPath = path;
    state.lastCancelledPath = null;
  }

  function setPurpose(purpose: Purpose): void {
    state.purpose = purpose;
  }

  async function pickInput(kind: "file" | "directory"): Promise<void> {
    const picked = await source.pick(kind);
    if (!picked) {
      // 取消系统文件选择器：返回原页，不显示错误。
      return;
    }
    if (kind === "directory") {
      await openRoot(picked);
      state.view = "import";
      return;
    }
    selectCandidate(picked);
    if (!state.candidates.some((candidate) => candidate.path === picked)) {
      const name = fileNameOf(picked);
      state.candidates = [
        ...state.candidates,
        {
          name,
          path: picked,
          size_bytes: null,
          modified_millis: null,
          matches_observed_names: /^save\d+\.dat$/i.test(name) || name.toLowerCase() === "profile.dat",
          status: "ready",
          detail: null,
          note: "手动选择（模拟）",
        },
      ];
    }
    state.view = "import";
  }

  // -------------------------------------------------------------------------
  // 分析、取消、刷新、切槽
  // -------------------------------------------------------------------------

  async function startAnalysis(): Promise<void> {
    if (state.analysis) {
      return;
    }
    if (!state.selectedPath) {
      notify("warning", "请先选择要分析的存档文件。");
      return;
    }
    if (!state.purpose) {
      notify("warning", "请先确认这个文件用于：普通战役存档 / 不确定，仅查看可用观察。");
      return;
    }
    const slot = slotOf(state.selectedPath);
    slot.startError = null;
    await beginAnalyze(slot.path, "start");
  }

  function cancelAnalysis(): void {
    const analysis = state.analysis;
    if (!analysis) {
      return;
    }
    source.cancel(analysis.requestId);
    state.analysis = null;
    state.lastCancelledPath = analysis.path;
    notify("info", "已取消：保留原视图，未产生新报告。");
  }

  async function refresh(): Promise<void> {
    const slot = currentSlot.value;
    if (!slot || !slot.result) {
      notify("warning", "当前没有可刷新的报告。");
      return;
    }
    await beginAnalyze(slot.path, "refresh");
  }

  function switchSlot(path: string): void {
    if (state.analysis && state.analysis.path !== path) {
      // 切槽先撤销旧请求的结果提交资格，并请求取消后端任务。
      source.cancel(state.analysis.requestId);
      state.analysis = null;
      notify("info", "已撤销之前的分析请求（切槽）。");
    }
    state.selectedPath = path;
    state.selectedRowId = null;
    state.lastCancelledPath = null;
    ensureFilterValid();
    if (state.slots[path]?.result) {
      // 已分析过的槽位：回到上一视图（筛选已重新校验）。
      if (state.view === "import") {
        state.view = "overview";
      }
    } else {
      state.view = "import";
    }
  }

  function openImport(): void {
    state.view = "import";
  }

  // -------------------------------------------------------------------------
  // 视图与筛选
  // -------------------------------------------------------------------------

  function setView(view: ViewId): void {
    state.view = view;
    state.navOverlayOpen = false;
    persistNow();
  }

  function setRegion(region: RegionScope): void {
    state.region = region;
    state.selectedRowId = null;
    persistNow();
  }

  function setStatusFilter(status: StatusFilter): void {
    state.filters.status = status;
    state.selectedRowId = null;
    persistNow();
  }

  function setCategory(category: string | null): void {
    state.filters.category = category;
    state.selectedRowId = null;
    persistNow();
  }

  function setEvidenceFilter(value: string | null): void {
    state.filters.evidence = value;
    state.selectedRowId = null;
    persistNow();
  }

  function setPrecisionFilter(value: string | null): void {
    state.filters.precision = value;
    state.selectedRowId = null;
    persistNow();
  }

  function clearFilters(): void {
    state.filters = { status: "all", category: null, evidence: null, precision: null };
    state.search = "";
    state.selectedRowId = null;
    persistNow();
  }

  function toggleMoreFilters(): void {
    state.moreFiltersOpen = !state.moreFiltersOpen;
  }

  function setSearch(query: string): void {
    state.search = query;
  }

  function openRow(id: string): void {
    state.selectedRowId = id;
    state.detailOpen = true;
  }

  function closeDetail(): void {
    state.detailOpen = false;
    state.selectedRowId = null;
  }

  // -------------------------------------------------------------------------
  // 布局与展示偏好
  // -------------------------------------------------------------------------

  function toggleNav(): void {
    if (layout.value === "wide") {
      state.navCollapsed = !state.navCollapsed;
      persistNow();
      return;
    }
    // 中等/紧凑/窄窗：展开为内容上方的浮层，不挤窄清单。
    state.navOverlayOpen = !state.navOverlayOpen;
  }

  function setNavOverlay(open: boolean): void {
    state.navOverlayOpen = open;
  }

  function setTheme(preference: ThemePreference): void {
    state.theme = preference;
    applyTheme(preference);
    persistNow();
  }

  function resetLayout(): void {
    state.navCollapsed = false;
    state.detailOpen = true;
    persistNow();
    notify("info", "已恢复默认布局（导航展开、详情常驻）。");
  }

  function setLayoutWidth(width: number): void {
    state.layoutWidth = Math.max(320, Math.round(width));
  }

  function setListScrollTop(value: number): void {
    state.listScrollTop = value;
  }

  function setMockWidth(width: number | null): void {
    state.mockWidth = width;
  }

  /** Ctrl+F：请求区域页聚焦搜索框。 */
  function requestSearchFocus(): void {
    state.searchFocusToken += 1;
  }

  function dismissNotice(): void {
    state.notice = null;
  }

  function dispose(): void {
    systemThemeListener.value?.();
    systemThemeListener.value = null;
  }

  const store = {
    state,
    metadata,
    layout,
    currentSlot,
    report,
    isStale,
    refreshing,
    missingAvailable,
    rows,
    filterCounts,
    missingTotal,
    observedTotal,
    unknownTotal,
    metadataVersion: metadata.metadata_version,
    actions: {
      boot,
      rescan,
      openRoot,
      selectCandidate,
      setPurpose,
      pickInput,
      startAnalysis,
      cancelAnalysis,
      refresh,
      switchSlot,
      openImport,
      setView,
      setRegion,
      setStatusFilter,
      setCategory,
      setEvidenceFilter,
      setPrecisionFilter,
      clearFilters,
      toggleMoreFilters,
      setSearch,
      openRow,
      closeDetail,
      toggleNav,
      setNavOverlay,
      setTheme,
      resetLayout,
      setLayoutWidth,
      setListScrollTop,
      setMockWidth,
      dismissNotice,
      requestSearchFocus,
      dispose,
      persistNow,
    },
  };
  return store;
}

export type AppStore = ReturnType<typeof createAppStore>;
export type { ComputedRef };
