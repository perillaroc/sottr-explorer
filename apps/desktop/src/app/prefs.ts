// 界面偏好持久化（SX-015）：只保存展示偏好，不保存任何存档内容或报告数据。
//
// 记忆范围（开发方案“信息架构与应用外壳”）：主题、导航折叠、视图与筛选偏好；
// 切槽不继承旧槽位的分析结果，只继承这些偏好。
import type { FilterState } from "./catalog";
import { DEFAULT_FILTERS, type RegionScope, type StatusFilter } from "./catalog";
import type { LanguagePreference } from "./i18n";
import type { ThemePreference } from "./theme";

export type ViewId = "import" | "overview" | "full" | "regions" | "diagnostics" | "settings";

/** 外部搜索链接使用的搜索引擎（默认 Bing）。 */
export type SearchEngine = "bing" | "google" | "baidu";

/** 外链的打开方式：交给系统浏览器，或只复制链接。 */
export type LinkOpenMode = "open" | "copy";

/** 可拖动面板的宽度范围（拖动钳制与偏好解析共用同一组数字）。 */
export interface WidthLimits {
  min: number;
  max: number;
  default: number;
}

/** 宽屏常驻详情：默认 400px，硬上限 720px。 */
export const DETAIL_PANE_WIDTH: WidthLimits = { min: 320, max: 720, default: 400 };
/** 1100–1439px 抽屉：默认 400px。 */
export const DETAIL_DRAWER_WIDTH: WidthLimits = { min: 320, max: 900, default: 400 };
/** 800–1099px 紧凑抽屉：保留近全宽的默认值 720px。 */
export const DETAIL_COMPACT_WIDTH: WidthLimits = { min: 320, max: 900, default: 720 };
/** 地图查询右栏（阶段二使用）：默认 420px，上限 600px。 */
export const MAP_RAIL_WIDTH: WidthLimits = { min: 320, max: 600, default: 420 };

export interface UiPreferences {
  /** 界面显示语言（默认中文）。 */
  language: LanguagePreference;
  theme: ThemePreference;
  navCollapsed: boolean;
  view: ViewId;
  region: RegionScope;
  filters: FilterState;
  searchEngine: SearchEngine;
  linkOpenMode: LinkOpenMode;
  /** 宽屏常驻详情宽度（px）。 */
  detailPaneWidth: number;
  /** 1100–1439px 抽屉宽度（px）。 */
  detailDrawerWidth: number;
  /** 800–1099px 紧凑抽屉宽度（px）。 */
  detailCompactWidth: number;
  /** 地图查询右栏宽度（px）。 */
  mapRailWidth: number;
}

const STORAGE_KEY = "sottr-explorer.ui.v1";

const DEFAULT_PREFERENCES: UiPreferences = {
  language: "zh",
  theme: "system",
  navCollapsed: false,
  view: "import",
  region: "all",
  filters: { ...DEFAULT_FILTERS },
  searchEngine: "bing",
  linkOpenMode: "open",
  detailPaneWidth: DETAIL_PANE_WIDTH.default,
  detailDrawerWidth: DETAIL_DRAWER_WIDTH.default,
  detailCompactWidth: DETAIL_COMPACT_WIDTH.default,
  mapRailWidth: MAP_RAIL_WIDTH.default,
};

function storage(): Storage | null {
  try {
    return typeof window !== "undefined" && window.localStorage ? window.localStorage : null;
  } catch {
    return null;
  }
}

export function loadPreferences(): UiPreferences {
  const store = storage();
  if (!store) {
    return { ...DEFAULT_PREFERENCES, filters: { ...DEFAULT_PREFERENCES.filters } };
  }
  try {
    const raw = store.getItem(STORAGE_KEY);
    if (!raw) {
      return { ...DEFAULT_PREFERENCES, filters: { ...DEFAULT_PREFERENCES.filters } };
    }
    const parsed = JSON.parse(raw) as Partial<UiPreferences>;
    const filters: FilterState = {
      status: (parsed.filters?.status as StatusFilter) ?? DEFAULT_FILTERS.status,
      category: parsed.filters?.category ?? null,
      evidence: parsed.filters?.evidence ?? null,
      precision: parsed.filters?.precision ?? null,
    };
    return {
      language: isLanguagePreference(parsed.language) ? parsed.language : DEFAULT_PREFERENCES.language,
      theme: parsed.theme === "light" || parsed.theme === "dark" ? parsed.theme : "system",
      navCollapsed: parsed.navCollapsed === true,
      view: isViewId(parsed.view) ? parsed.view : DEFAULT_PREFERENCES.view,
      region: typeof parsed.region === "string" && parsed.region.length > 0 ? parsed.region : "all",
      filters,
      searchEngine: isSearchEngine(parsed.searchEngine)
        ? parsed.searchEngine
        : DEFAULT_PREFERENCES.searchEngine,
      linkOpenMode: isLinkOpenMode(parsed.linkOpenMode)
        ? parsed.linkOpenMode
        : DEFAULT_PREFERENCES.linkOpenMode,
      detailPaneWidth: widthPreference(parsed.detailPaneWidth, DETAIL_PANE_WIDTH),
      detailDrawerWidth: widthPreference(parsed.detailDrawerWidth, DETAIL_DRAWER_WIDTH),
      detailCompactWidth: widthPreference(parsed.detailCompactWidth, DETAIL_COMPACT_WIDTH),
      mapRailWidth: widthPreference(parsed.mapRailWidth, MAP_RAIL_WIDTH),
    };
  } catch {
    return { ...DEFAULT_PREFERENCES, filters: { ...DEFAULT_PREFERENCES.filters } };
  }
}

export function savePreferences(preferences: UiPreferences): void {
  const store = storage();
  if (!store) {
    return;
  }
  try {
    store.setItem(STORAGE_KEY, JSON.stringify(preferences));
  } catch {
    // 存储不可用时静默：偏好不是必要数据。
  }
}

export function clearPreferences(): void {
  storage()?.removeItem(STORAGE_KEY);
}

/** 宽度偏好：合法数字钳制到安全范围；缺失/非数字回退默认值。 */
export function widthPreference(value: unknown, limits: WidthLimits): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return limits.default;
  }
  return Math.min(limits.max, Math.max(limits.min, Math.round(value)));
}

function isViewId(value: unknown): value is ViewId {
  return value === "import" || value === "overview" || value === "full" || value === "regions" || value === "diagnostics" || value === "settings";
}

function isLanguagePreference(value: unknown): value is LanguagePreference {
  return value === "zh" || value === "en" || value === "both";
}

function isSearchEngine(value: unknown): value is SearchEngine {
  return value === "bing" || value === "google" || value === "baidu";
}

function isLinkOpenMode(value: unknown): value is LinkOpenMode {
  return value === "open" || value === "copy";
}
