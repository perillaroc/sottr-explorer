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
  CommandErrorDto,
  DataSource,
  DiscoverOutcomeDto,
  ExportFormat,
  ExportOutcomeDto,
  InputError,
  PickKind,
  Purpose,
} from "../data/source";
import { toCommandError } from "../data/errors";
import { copyText } from "./clipboard";
import { mediumOfUrl, type LinkMedium } from "./external-links";
import { demoMetadata, productionMetadata } from "../data/metadata";
import type { FilterState, RegionScope, StatusFilter } from "./catalog";
import {
  buildPlan,
  demoObservedIds,
  findPlanItem,
  productionAtlas,
  type Atlas,
  type CollectionPlan,
  type PlanGroup,
  type PlanItem,
  type PlanRegion,
} from "../data/atlas";
import {
  countPlan,
  countRegion,
  filterRegionGroups,
  groupFilters,
  type CollectionCounts,
} from "./collection";
import { fileNameOf } from "./format";
import { applyTheme, onSystemThemeChange, type ThemePreference } from "./theme";
import {
  loadPreferences,
  savePreferences,
  type LinkOpenMode,
  type SearchEngine,
  type UiPreferences,
  type ViewId,
} from "./prefs";

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

/** 导出对话框状态（SX-016）：JSON/CSV、分享脱敏、目标新目录与结果/错误。 */
export interface ExportState {
  open: boolean;
  format: ExportFormat;
  share: boolean;
  /** 目标位置的父目录（用户选择或手动输入）。 */
  parent: string | null;
  /** 目标新目录名（导出不覆盖：同名目录已存在时报 OUTPUT_EXISTS）。 */
  folder: string;
  status: "edit" | "exporting" | "success" | "error";
  outcome: ExportOutcomeDto | null;
  error: CommandErrorDto | null;
  /** 失败后可一键采用的替代目录名（如 OUTPUT_EXISTS）。 */
  suggestedFolder: string | null;
}

/**
 * 外链动作状态（解析/打开/复制；错误保留目标便于重试）。
 *
 * `key` 标识发起动作的按钮：直链为 `guideLinkKey`，合成外链为 `externalLinkKey`。
 */
export interface LinkActionState {
  key: string;
  kind: "open" | "copy";
  status: "resolving" | "opening" | "opened" | "copied" | "error";
  url: string | null;
  error: CommandErrorDto | null;
}

/** 直链按钮的目标标识（定位线索 + 来源）。 */
export function guideLinkKey(locationId: string, sourceRef: string): string {
  return `guide:${locationId}:${sourceRef}`;
}

/** 合成外链按钮的目标标识（地址本身）。 */
export function externalLinkKey(url: string): string {
  return `url:${url}`;
}

/** 动作目标：直链（定位 + 来源，URL 由后端从元数据解析）或合成地址（界面已算出）。 */
type LinkTarget =
  | { kind: "guide"; label: string; snapshotId: string; locationId: string; sourceRef: string }
  | { kind: "url"; label: string; url: string };

function linkTargetKey(target: LinkTarget): string {
  return target.kind === "guide"
    ? guideLinkKey(target.locationId, target.sourceRef)
    : externalLinkKey(target.url);
}

/** 可直接打开的参考链接（来自报告的位置线索 + 元数据来源）。 */
export interface DirectLink {
  locationId: string;
  sourceRef: string;
  url: string;
  medium: LinkMedium;
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
  /** 外部搜索链接使用的搜索引擎。 */
  searchEngine: SearchEngine;
  /** 外链打开方式：交给系统浏览器或只复制链接。 */
  linkOpenMode: LinkOpenMode;
  notice: Notice | null;
  /** 模拟模式下的演示视口宽度（?mockWidth=800），用于宽度验收；null 为跟随窗口。 */
  mockWidth: number | null;
  listScrollTop: number;
  /** Ctrl+F 焦点请求计数：区域页监听并聚焦搜索框。 */
  searchFocusToken: number;
  /** 导出对话框（SX-016）。 */
  exportState: ExportState;
  /** 详情内的攻略链接动作（SX-016；null 表示无进行中的动作）。 */
  linkAction: LinkActionState | null;
}

export interface CreateStoreOptions {
  metadata?: MetadataDataset;
  /** 图鉴数据集（默认为仓库生产数据集）。 */
  atlas?: Atlas;
  /**
   * 观察集合来源：默认读取当前报告的物品表。
   * 模拟模式与测试可传入固定集合，使演示与断言可复现。
   */
  observedIds?: () => string[] | null;
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
    searchEngine: preferences?.searchEngine ?? "bing",
    linkOpenMode: preferences?.linkOpenMode ?? "open",
    notice: null,
    mockWidth: null,
    listScrollTop: 0,
    searchFocusToken: 0,
    exportState: {
      open: false,
      format: "both",
      share: true,
      parent: null,
      folder: "",
      status: "edit",
      outcome: null,
      error: null,
      suggestedFolder: null,
    },
    linkAction: null,
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
      searchEngine: state.searchEngine,
      linkOpenMode: state.linkOpenMode,
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

  // -------------------------------------------------------------------------
  // 收集图鉴与按区域计划（SX-022）
  // -------------------------------------------------------------------------

  const atlas = options.atlas ?? productionAtlas;
  // 模拟模式使用固定观察集合：演示与测试可复现；真实模式读取当前报告的物品表。
  const observedSource = options.observedIds ?? (source.kind === "mock" ? () => demoObservedIds : undefined);

  const observedIds = computed<string[]>(() => {
    if (!report.value || report.value.context.mode === "profile") {
      return [];
    }
    const injected = observedSource?.();
    if (injected) {
      return injected;
    }
    return (report.value?.observations.inventory.records ?? []).map((record) => record.internal_id);
  });

  /** 资料文件（profile.dat）没有物品表：不产生收集状态，界面显示“不适用”。 */
  const planAvailable = computed(() => report.value !== null && report.value.context.mode !== "profile");

  const plan = computed<CollectionPlan>(() =>
    planAvailable.value
      ? buildPlan(atlas, observedIds.value)
      : {
          catalog_version: atlas.catalog_version,
          main_collected: 0,
          main_total: 0,
          regions: [],
          observed_not_in_catalog: [],
        },
  );

  const planCounts = computed<CollectionCounts>(() => countPlan(plan.value));
  /** 主区域合计（不含特殊区域），用于“全部区域”摘要。 */
  const planMainCounts = computed<CollectionCounts>(() => countPlan(plan.value, "main"));

  const planRegion = computed<PlanRegion | null>(() => {
    if (state.region === "all" || state.region === "unknown") {
      return null;
    }
    return plan.value.regions.find((region) => region.slug === state.region) ?? null;
  });

  const planGroups = computed<PlanGroup[]>(() => {
    const region = planRegion.value;
    if (!region) {
      return [];
    }
    return filterRegionGroups(region, groupFilters(state.filters, state.search));
  });

  const planCountsForRegion = computed<CollectionCounts>(() =>
    planRegion.value ? countRegion(planRegion.value) : planCounts.value,
  );

  const planFilterCounts = computed<Record<StatusFilter, number>>(() => {
    if (state.region === "all" || state.region === "unknown") {
      return {
        missing: planCounts.value.uncollected,
        observed: planCounts.value.collected,
        unknown: 0,
        all: planCounts.value.all,
      };
    }
    const region = planRegion.value;
    if (!region) {
      return { missing: 0, observed: 0, unknown: 0, all: 0 };
    }
    const source: PlanItem[] = region.groups.flatMap((group) => group.items);
    return {
      missing: source.filter((item) => item.status === "not_collected").length,
      observed: source.filter((item) => item.status === "collected").length,
      unknown: 0,
      all: source.length,
    };
  });

  /** 当前选中的计划条目（详情面板数据源）。 */
  const selectedEntry = computed(() => {
    const id = state.selectedRowId;
    if (!id) {
      return null;
    }
    return findPlanItem(plan.value, id);
  });

  /** 计划内条目总数（用于说明“全部”的范围）。 */
  const planItemCount = computed(() => planCounts.value.all);

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

  /** 切槽/分析后重新校验偏好：区域 slug 必须仍存在于图鉴；无效值回退到“全部区域”。 */
  function ensureFilterValid(): void {
    if (state.region !== "all" && state.region !== "unknown"
      && !plan.value.regions.some((region) => region.slug === state.region)) {
      state.region = "all";
      notify("info", "原区域在当前图鉴中不存在，已切换到“全部区域”。");
    }
    if (state.selectedRowId && !selectedEntry.value) {
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

  async function pickInput(kind: PickKind): Promise<void> {
    const picked = await source.pick(kind);
    if (!picked) {
      // 取消系统文件选择器：返回原页，不显示错误。
      return;
    }
    if (kind === "save-directory") {
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
  // 导出（SX-016）：JSON/CSV、分享脱敏、目标新目录；绑定当前快照
  // -------------------------------------------------------------------------

  /** 导出是否可用：必须有当前报告且没有在飞分析（报告未完成不能导出）。 */
  const canExport = computed<boolean>(
    () => (currentSlot.value?.result ?? null) !== null && state.analysis === null,
  );

  /** 目标完整路径（父目录 + 新目录名）。 */
  const exportTargetPath = computed<string | null>(() => {
    const { parent, folder } = state.exportState;
    if (!parent || folder.trim().length === 0) {
      return null;
    }
    return `${parent.replace(/[\\/]+$/, "")}\\${folder.trim()}`;
  });

  /** 默认目录名：本地时间戳，避免与既有导出目录同名。 */
  function defaultExportFolder(): string {
    const now = new Date();
    const pad = (value: number) => String(value).padStart(2, "0");
    const stamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
    return `sottr-explorer-${stamp}`;
  }

  function openExportDialog(): void {
    if (!state.selectedPath || !currentSlot.value?.result) {
      notify("warning", "没有可导出的报告：请先分析当前槽位。");
      return;
    }
    if (state.analysis) {
      notify("warning", "分析/刷新进行中：完成后再导出，避免导出与快照不一致。");
      return;
    }
    state.exportState = {
      open: true,
      format: "both",
      // 默认分享脱敏；用户取消勾选才导出完整本地信息。
      share: true,
      parent: state.exportState.parent,
      folder: state.exportState.folder || defaultExportFolder(),
      status: "edit",
      outcome: null,
      error: null,
      suggestedFolder: null,
    };
  }

  function closeExportDialog(): void {
    // 取消导出：静默返回，不改动任何已有产物。
    state.exportState.open = false;
  }

  function setExportFormat(format: ExportFormat): void {
    state.exportState.format = format;
  }

  function setExportShare(share: boolean): void {
    state.exportState.share = share;
  }

  function setExportFolder(folder: string): void {
    state.exportState.folder = folder;
  }

  function setExportParent(parent: string | null): void {
    state.exportState.parent = parent;
  }

  async function pickExportParent(): Promise<void> {
    const picked = await source.pick("export-directory");
    if (picked) {
      state.exportState.parent = picked;
    }
    // 用户取消选择器：静默返回，不改动表单。
  }

  async function runExport(): Promise<void> {
    const slot = currentSlot.value;
    const result = slot?.result ?? null;
    if (!result || !result.snapshot_id) {
      notify("warning", "没有可导出的报告：请先分析当前槽位。");
      return;
    }
    if (state.analysis) {
      notify("warning", "分析/刷新进行中，暂不能导出。");
      return;
    }
    const target = exportTargetPath.value;
    if (!target) {
      state.exportState.status = "error";
      state.exportState.error = {
        code: "INVALID_PARAMS",
        message: "请先选择目标位置并填写新目录名",
      };
      return;
    }
    state.exportState.status = "exporting";
    state.exportState.error = null;
    state.exportState.outcome = null;
    try {
      const outcome = await source.exportReport({
        snapshotId: result.snapshot_id,
        inputRef: result.input_ref,
        outputDir: target,
        format: state.exportState.format,
        share: state.exportState.share,
      });
      state.exportState.status = "success";
      state.exportState.outcome = outcome;
      state.exportState.suggestedFolder = null;
      notify(
        outcome.freshness === "stale" ? "warning" : "success",
        outcome.freshness === "stale"
          ? `已导出旧快照（含过期标记）：${outcome.artifacts.map((artifact) => artifact.path).join("；")}`
          : `已导出：${outcome.artifacts.map((artifact) => artifact.path).join("；")}`,
      );
    } catch (error) {
      const commandError = toCommandError(error);
      state.exportState.status = "error";
      state.exportState.error = commandError;
      if (commandError.code === "OUTPUT_EXISTS") {
        state.exportState.suggestedFolder = defaultExportFolder();
      }
    }
  }

  /** 采用建议的新目录名后回到编辑状态（重试路径）。 */
  function applySuggestedExportFolder(): void {
    if (!state.exportState.suggestedFolder) {
      return;
    }
    state.exportState.folder = state.exportState.suggestedFolder;
    state.exportState.suggestedFolder = null;
    state.exportState.error = null;
    state.exportState.status = "edit";
  }

  // -------------------------------------------------------------------------
  // 外部链接：直链绑定当前快照（后端按定位 + 来源解析 URL），合成外链直接用地址；
  // 动作按 `state.linkOpenMode` 分派打开或仅复制，失败保留目标以便重试
  // -------------------------------------------------------------------------

  const linkActionFor = (key: string): LinkActionState | null => {
    const action = state.linkAction;
    if (!action || action.key !== key) {
      return null;
    }
    return action;
  };

  async function runLinkAction(target: LinkTarget): Promise<void> {
    const key = linkTargetKey(target);
    const mode = state.linkOpenMode;
    state.linkAction = {
      key,
      kind: mode,
      status: mode === "open" ? "opening" : "resolving",
      url: target.kind === "url" ? target.url : null,
      error: null,
    };
    try {
      let url: string;
      if (target.kind === "guide") {
        const request = {
          snapshotId: target.snapshotId,
          locationId: target.locationId,
          sourceRef: target.sourceRef,
        };
        url =
          mode === "open"
            ? (await source.openGuideLink(request)).url
            : (await source.resolveGuideLink(request)).url;
      } else if (mode === "open") {
        await source.openExternalUrl(target.url);
        url = target.url;
      } else {
        url = target.url;
      }
      if (mode === "open") {
        state.linkAction = { key, kind: mode, status: "opened", url, error: null };
        notify("success", `已在系统浏览器打开：${target.label}（WebView 不加载网页）`);
      } else {
        const copied = await copyText(url);
        state.linkAction = { key, kind: mode, status: "copied", url, error: null };
        notify(
          copied ? "success" : "warning",
          copied
            ? `已复制链接：${target.label}（不含存档内容或私人路径）`
            : "复制失败：可在下方选择链接文本手动复制",
        );
      }
    } catch (error) {
      state.linkAction = { key, kind: mode, status: "error", url: null, error: toCommandError(error) };
    }
  }

  /**
   * 打开/复制某条定位来源的直链（按钮标识为 `guideLinkKey`）。
   * 需要当前报告的快照：URL 由后端从该快照的元数据解析，前端不提交地址。
   */
  function openGuideLink(locationId: string, sourceRef: string, label: string): Promise<void> {
    const result = currentSlot.value?.result ?? null;
    if (!result?.snapshot_id || !result.report) {
      notify("warning", "没有可用的报告：请先分析当前槽位。");
      return Promise.resolve();
    }
    return runLinkAction({
      kind: "guide",
      label,
      snapshotId: result.snapshot_id,
      locationId,
      sourceRef,
    });
  }

  /** 打开/复制界面合成的外链（按钮标识为 `externalLinkKey`）；不绑定快照。 */
  function openExternalLink(url: string, label: string): Promise<void> {
    return runLinkAction({ kind: "url", label, url });
  }

  /**
   * 当前条目可用的直链候选：报告里成员含该条目的位置线索 × 其来源中**有 URL** 的那些。
   * 没有 URL 的来源（如书面记录）不产生候选——界面据此隐藏对应按钮，不置灰。
   */
  function directLinksForItem(item: PlanItem): DirectLink[] {
    const locations = report.value?.locations ?? [];
    const links: DirectLink[] = [];
    const seen = new Set<string>();
    for (const location of locations) {
      if (!location.member_ids.includes(item.id)) {
        continue;
      }
      for (const sourceRef of location.source_refs) {
        const url = productionMetadata.sources[sourceRef]?.url ?? null;
        if (!url || seen.has(url)) {
          continue;
        }
        seen.add(url);
        links.push({ locationId: location.location_id, sourceRef, url, medium: mediumOfUrl(url) });
      }
    }
    return links;
  }

  function clearLinkAction(): void {
    state.linkAction = null;
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

  function setSearchEngine(engine: SearchEngine): void {
    state.searchEngine = engine;
    persistNow();
  }

  function setLinkOpenMode(mode: LinkOpenMode): void {
    state.linkOpenMode = mode;
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
    atlas,
    layout,
    currentSlot,
    report,
    isStale,
    refreshing,
    plan,
    planAvailable,
    planCounts,
    planMainCounts,
    planRegion,
    planGroups,
    planCountsForRegion,
    planFilterCounts,
    planItemCount,
    selectedEntry,
    metadataVersion: metadata.metadata_version,
    canExport,
    exportTargetPath,
    /** 某外链按钮当前的动作状态（无则 null）；键由 `guideLinkKey` / `externalLinkKey` 给出。 */
    linkActionFor,
    /** 某条目可用的直链候选（无地址的来源不出现）。 */
    directLinksForItem,
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
      openExportDialog,
      closeExportDialog,
      setExportFormat,
      setExportShare,
      setExportFolder,
      setExportParent,
      pickExportParent,
      runExport,
      applySuggestedExportFolder,
      openGuideLink,
      openExternalLink,
      clearLinkAction,
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
      setSearchEngine,
      setLinkOpenMode,
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
