// 界面偏好持久化（SX-015）：只保存展示偏好，不保存任何存档内容或报告数据。
//
// 记忆范围（开发方案“信息架构与应用外壳”）：主题、导航折叠、视图与筛选偏好；
// 切槽不继承旧槽位的分析结果，只继承这些偏好。
import type { FilterState } from "./catalog";
import { DEFAULT_FILTERS, type RegionScope, type StatusFilter } from "./catalog";
import type { ThemePreference } from "./theme";

export type ViewId = "import" | "overview" | "regions" | "diagnostics" | "settings";

export interface UiPreferences {
  theme: ThemePreference;
  navCollapsed: boolean;
  view: ViewId;
  region: RegionScope;
  filters: FilterState;
}

const STORAGE_KEY = "sottr-explorer.ui.v1";

const DEFAULT_PREFERENCES: UiPreferences = {
  theme: "system",
  navCollapsed: false,
  view: "import",
  region: "all",
  filters: { ...DEFAULT_FILTERS },
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
      theme: parsed.theme === "light" || parsed.theme === "dark" ? parsed.theme : "system",
      navCollapsed: parsed.navCollapsed === true,
      view: isViewId(parsed.view) ? parsed.view : DEFAULT_PREFERENCES.view,
      region: typeof parsed.region === "string" && parsed.region.length > 0 ? parsed.region : "all",
      filters,
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

function isViewId(value: unknown): value is ViewId {
  return value === "import" || value === "overview" || value === "regions" || value === "diagnostics" || value === "settings";
}
