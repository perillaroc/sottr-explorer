// 详情弹出窗口（2026-10-31 地图查询与详情增强·阶段三）。
//
// 边界：
// - payload 是打开时的**只读快照**：不含完整路径、原始存档内容、证据或诊断数据；
// - Tauri 内由 Rust 命令存入进程内存（不落盘），弹窗通过命令读取、用事件接收更新；
// - 纯浏览器开发/演示用 `sessionStorage`（仅模拟数据，关闭标签页即清除）与
//   `window.open` + `postMessage`，不写 localStorage；
// - 窗口识别：Tauri 用窗口标签 `detail`，浏览器用 `#popup=detail`，不解析动态 URL。
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Atlas, CollectionPlan, PlanItem } from "../data/atlas";
import { basemapForUnit, type Basemap } from "../data/basemaps";
import { isTauriRuntime } from "../data/tauri";
import { buildSearchUrl, textSearchQuery, videoSearchQuery } from "./external-links";
import { categoryNameOf } from "./collection";
import { subcategoryLabel } from "./labels";
import { localizedName, t } from "./i18n";
import type { LinkOpenMode, SearchEngine } from "./prefs";
import type { DirectLink } from "./store";

/** payload 结构版本；版本不符时弹窗显示「请重新打开」。 */
export const DETAIL_WINDOW_VERSION = 1;
/** 浏览器模式的 sessionStorage 键（不落 localStorage）。 */
export const DETAIL_WINDOW_STORAGE_KEY = "sottr-explorer.detail-window.v1";
/** 主窗口通知弹窗更新 payload 的事件名（与 Rust `DETAIL_PAYLOAD_EVENT` 一致）。 */
export const DETAIL_PAYLOAD_EVENT = "detail-payload-updated";
/** 浏览器弹窗识别锚点。 */
export const DETAIL_POPUP_HASH = "#popup=detail";

export interface DetailWindowItem {
  id: string;
  displayName: string;
  nameEn?: string;
  nameZh?: string;
  internalName?: string;
  status: "collected" | "not_collected";
  category: string;
  categoryLabel: string;
  subgroupLabel: string | null;
  isDlc: boolean;
  requiredFor100: boolean | null;
  acquisitionText: string;
  acquisitionSource: "item_note" | "category_rule";
  position?: { x: number; y: number };
  sameUnitRemaining?: number;
  sharedNames?: string[];
  searchTextUrl: string;
  searchVideoUrl: string;
  directLinks: { label: string; url: string }[];
}

export interface DetailWindowPayload {
  version: 1;
  /** 显示用来源：文件名 + 快照时间（不含完整路径）。 */
  slotName: string;
  snapshotTime: string | null;
  /** 报告是否为过期快照（界面显示警告）。 */
  stale: boolean;
  /** 外链打开方式（打开浏览器 / 仅复制），与主窗口设置一致。 */
  linkOpenMode: LinkOpenMode;
  regionName: string;
  unitLabel: string;
  selectedId: string;
  /** 同子关卡的全部条目（含选中项），用于弹窗内清单与相对位置底图。 */
  items: DetailWindowItem[];
  basemap: Basemap | null;
}

/** 构建 payload 所需的数据（由 store 或测试提供，避免反向依赖 store 类型）。 */
export interface DetailWindowSource {
  plan: CollectionPlan;
  atlas: Atlas;
  slotName: string;
  snapshotTime: string | null;
  stale: boolean;
  searchEngine: SearchEngine;
  linkOpenMode: LinkOpenMode;
  directLinksForItem(item: PlanItem): DirectLink[];
}

export interface PlanEntryRef {
  region: { slug: string; name_zh?: string; name_en?: string };
  group: { category: string; name: string; subcategory?: string };
  item: PlanItem;
}

function unitLabelOf(item: PlanItem): string {
  const unit = item.unit;
  if (!unit) {
    return "—";
  }
  return localizedName(unit.display_zh, unit.display_en, unit.zone ?? t(`子关卡 ${unit.unit}`, `Sub-level ${unit.unit}`));
}

function itemPayload(
  source: DetailWindowSource,
  regionLabel: string,
  group: { category: string; name: string; subcategory?: string },
  item: PlanItem,
): DetailWindowItem {
  const categoryLabel = categoryNameOf(source.atlas, group.category);
  const groupLabel = subcategoryLabel(group.subcategory ?? null) ?? categoryLabel;
  return {
    id: item.id,
    displayName: item.display_name,
    nameEn: item.name_en,
    nameZh: item.name_zh,
    internalName: item.internal_name,
    status: item.status,
    category: group.category,
    categoryLabel,
    subgroupLabel: groupLabel === categoryLabel ? null : groupLabel,
    isDlc: item.is_dlc,
    requiredFor100: item.required_for_100,
    acquisitionText: item.acquisition_text,
    acquisitionSource: item.acquisition_source,
    position: item.position ? { x: item.position.x, y: item.position.y } : undefined,
    sameUnitRemaining: item.same_unit_remaining,
    sharedNames: item.shared_names,
    searchTextUrl: buildSearchUrl(source.searchEngine, textSearchQuery(item, regionLabel)),
    searchVideoUrl: buildSearchUrl(source.searchEngine, videoSearchQuery(item, regionLabel)),
    directLinks: source.directLinksForItem(item).map((link) => ({
      label: link.medium === "video" ? t("视频页", "Video page") : t("攻略页", "Guide page"),
      url: link.url,
    })),
  };
}

/** 从当前计划构建弹窗 payload（只读快照；不含路径、证据与诊断数据）。 */
export function buildDetailWindowPayload(
  source: DetailWindowSource,
  entry: PlanEntryRef,
): DetailWindowPayload {
  const regionLabel = localizedName(
    entry.region.name_zh,
    entry.region.name_en,
    entry.region.slug,
  );
  const unit = entry.item.unit?.unit;
  const items: DetailWindowItem[] = [];
  if (unit === undefined) {
    items.push(itemPayload(source, regionLabel, entry.group, entry.item));
  } else {
    for (const region of source.plan.regions) {
      const label = localizedName(region.name_zh, region.name_en, region.slug);
      for (const group of region.groups) {
        for (const item of group.items) {
          if (item.unit?.unit === unit) {
            items.push(itemPayload(source, label, group, item));
          }
        }
      }
    }
    if (items.length === 0) {
      items.push(itemPayload(source, regionLabel, entry.group, entry.item));
    }
  }
  return {
    version: DETAIL_WINDOW_VERSION,
    slotName: source.slotName,
    snapshotTime: source.snapshotTime,
    stale: source.stale,
    linkOpenMode: source.linkOpenMode,
    regionName: regionLabel,
    unitLabel: unitLabelOf(entry.item),
    selectedId: entry.item.id,
    items,
    basemap: unit === undefined ? null : basemapForUnit(unit),
  };
}

/** payload 解析失败的原因（弹窗按原因显示不同说明）。 */
export type DetailPayloadErrorKind = "missing" | "invalid" | "version";

export class DetailPayloadError extends Error {
  constructor(readonly kind: DetailPayloadErrorKind) {
    super(kind);
    this.name = "DetailPayloadError";
  }
}

/** 解析并校验 payload（版本与必要字段；不做结构深校验）。 */
export function parseDetailWindowPayload(raw: string | null): DetailWindowPayload {
  if (!raw) {
    throw new DetailPayloadError("missing");
  }
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    throw new DetailPayloadError("invalid");
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new DetailPayloadError("invalid");
  }
  const record = value as Record<string, unknown>;
  if (record.version !== DETAIL_WINDOW_VERSION) {
    throw new DetailPayloadError("version");
  }
  if (typeof record.selectedId !== "string" || !Array.isArray(record.items)) {
    throw new DetailPayloadError("invalid");
  }
  return value as DetailWindowPayload;
}

let popupWindow: Window | null = null;

/** 清除浏览器弹窗引用（测试用；生产在用户关闭弹窗后由 `closed` 判断）。 */
export function resetDetailWindowHandle(): void {
  popupWindow = null;
}

/**
 * 打开/复用详情窗口。
 * Tauri 内调用 Rust 命令；浏览器写 sessionStorage 后 `window.open`（被拦截时抛错）。
 */
export async function openDetailWindow(payload: DetailWindowPayload): Promise<void> {
  const json = JSON.stringify(payload);
  if (isTauriRuntime()) {
    await invoke("open_detail_window", { payload: json });
    return;
  }

  try {
    sessionStorage.setItem(DETAIL_WINDOW_STORAGE_KEY, json);
  } catch {
    // 存储不可用时仍尝试打开；弹窗会显示「没有可显示的详情」。
  }

  if (popupWindow && !popupWindow.closed) {
    popupWindow.focus();
    popupWindow.postMessage({ type: DETAIL_PAYLOAD_EVENT, payload }, location.origin);
    return;
  }

  const url = `${location.pathname}${location.search}${DETAIL_POPUP_HASH}`;
  popupWindow = window.open(url, "_blank", "popup,width=560,height=780");
  if (!popupWindow) {
    throw new Error(t("无法打开新窗口（可能被浏览器拦截）", "Cannot open a new window (it may be blocked)"));
  }
}

/** 读取最近一次 payload 的原始 JSON（Tauri 命令或 sessionStorage）。 */
export async function readDetailWindowRaw(): Promise<string | null> {
  if (isTauriRuntime()) {
    return (await invoke<string | null>("read_detail_window_payload")) ?? null;
  }
  try {
    return sessionStorage.getItem(DETAIL_WINDOW_STORAGE_KEY);
  } catch {
    return null;
  }
}

/** 订阅主窗口的 payload 更新；返回取消订阅函数。 */
export async function subscribeDetailWindowUpdates(
  callback: (payload: DetailWindowPayload) => void,
): Promise<() => void> {
  if (isTauriRuntime()) {
    const unlisten: UnlistenFn = await listen<string>(DETAIL_PAYLOAD_EVENT, (event) => {
      try {
        callback(parseDetailWindowPayload(event.payload));
      } catch {
        // 无法解析的更新不覆盖已有内容。
      }
    });
    return unlisten;
  }

  const handler = (event: MessageEvent): void => {
    if (event.origin !== location.origin || event.source !== window.opener) {
      return;
    }
    const data = event.data as { type?: string; payload?: unknown } | null;
    if (!data || data.type !== DETAIL_PAYLOAD_EVENT) {
      return;
    }
    try {
      callback(parseDetailWindowPayload(JSON.stringify(data.payload ?? null)));
    } catch {
      // 同上：忽略非法更新。
    }
  };
  window.addEventListener("message", handler);
  return () => window.removeEventListener("message", handler);
}

/** 当前页面是否是详情弹窗（Tauri 窗口标签 / 浏览器锚点）。 */
export function isDetailPopupWindow(): boolean {
  if (isTauriRuntime()) {
    try {
      return getCurrentWindow().label === "detail";
    } catch {
      return false;
    }
  }
  if (location.hash.startsWith(DETAIL_POPUP_HASH)) {
    return true;
  }
  try {
    return new URLSearchParams(location.search).get("popup") === "detail";
  } catch {
    return false;
  }
}
