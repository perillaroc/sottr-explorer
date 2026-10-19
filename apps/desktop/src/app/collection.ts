// 区域与收集的派生视图（SX-022）：把收集计划整理成区域摘要、筛选后的分组与清单行。
//
// 只做筛选、排序与计数，不重新解释收集状态：状态一律来自 `buildPlan`（与 Rust 核心
// 的 `sottr_core::atlas` 黄金一致）。筛选值沿用 `StatusFilter`：
// `missing` = 未收集（存档观察集合中没有），`observed` = 已收集，`all` = 全部。
import type { Atlas, CollectionPlan, PlanGroup, PlanItem, PlanRegion } from "../data/atlas";
import { regionDisplayName } from "../data/atlas";
import type { FilterState, StatusFilter } from "./catalog";

export interface CollectionCounts {
  all: number;
  collected: number;
  uncollected: number;
}

export function countItems(items: PlanItem[]): CollectionCounts {
  let collected = 0;
  for (const item of items) {
    if (item.status === "collected") {
      collected += 1;
    }
  }
  return { all: items.length, collected, uncollected: items.length - collected };
}

export function countRegion(region: PlanRegion): CollectionCounts {
  return countItems(region.groups.flatMap((group) => group.items));
}

export function countPlan(plan: CollectionPlan, kind?: "main" | "special"): CollectionCounts {
  const regions = kind ? plan.regions.filter((region) => region.kind === kind) : plan.regions;
  return countItems(regions.flatMap((region) => region.groups.flatMap((group) => group.items)));
}

/** 区域中文名（中文 → 英文 → slug）。 */
export function regionName(region: PlanRegion): string {
  return regionDisplayName(region);
}

/** 类别显示名（图鉴类别名；未知回退 id）。主清单分组头与详情徽标共用。 */
export function categoryNameOf(atlas: Atlas, categoryId: string): string {
  return atlas.categories.find((entry) => entry.id === categoryId)?.name ?? categoryId;
}

function itemHaystack(item: PlanItem, group: PlanGroup): string {
  const unit = item.unit ? [item.unit.display_zh, item.unit.display_en, item.unit.zone] : [];
  return [
    item.display_name,
    item.name_en,
    item.name_zh,
    item.internal_name,
    item.id,
    item.id.replace(/^0x/i, ""),
    item.collection,
    group.name,
    ...unit,
    ...(item.shared_names ?? []),
  ]
    .filter((value): value is string => typeof value === "string" && value.length > 0)
    .join(" ")
    .toLowerCase();
}

function matchesStatus(item: PlanItem, status: StatusFilter): boolean {
  if (status === "missing") {
    return item.status === "not_collected";
  }
  if (status === "observed") {
    return item.status === "collected";
  }
  return true;
}

export interface GroupFilters {
  status: StatusFilter;
  category: string | null;
  search: string;
}

/**
 * 按状态 / 类别 / 搜索过滤区域内的分组。
 * 空分组（全部被筛掉）不返回，避免出现“0 项”的假分组。
 */
export function filterRegionGroups(region: PlanRegion, filters: GroupFilters): PlanGroup[] {
  const query = filters.search.trim().toLowerCase();
  const groups: PlanGroup[] = [];
  for (const group of region.groups) {
    if (filters.category && group.category !== filters.category) {
      continue;
    }
    const items = group.items.filter((item) => {
      if (!matchesStatus(item, filters.status)) {
        return false;
      }
      return query.length === 0 || itemHaystack(item, group).includes(query);
    });
    if (items.length === 0) {
      continue;
    }
    // 分组进度保持该组的真实总数，不随筛选变化：筛选只影响显示的条目。
    groups.push({ ...group, items });
  }
  return groups;
}

/** 当前区域可用的类别筛选（按出现顺序）。 */
export function categoriesOfRegion(region: PlanRegion): { id: string; name: string; count: number }[] {
  const seen = new Map<string, { id: string; name: string; count: number }>();
  for (const group of region.groups) {
    const entry = seen.get(group.category) ?? { id: group.category, name: group.category, count: 0 };
    entry.count += group.items.length;
    seen.set(group.category, entry);
  }
  return [...seen.values()];
}

/** 全部区域的类别名（取第一处出现的分组名，故事类别按子类别展示）。 */
export function groupLabel(group: PlanGroup): string {
  return group.name;
}

/** 区域列表：主区域在前、特殊区域在后（计划已排序，这里只做透传并附名称）。 */
export function regionOptions(plan: CollectionPlan): { slug: string; label: string; kind: "main" | "special" }[] {
  return plan.regions.map((region) => ({
    slug: region.slug,
    label: regionName(region),
    kind: region.kind,
  }));
}

/** 从筛选状态构造 `GroupFilters`（忽略与区域页无关的 evidence/precision）。 */
export function groupFilters(filters: FilterState, search: string): GroupFilters {
  return { status: filters.status, category: filters.category, search };
}

/** 计划里全部条目的 ID（用于“目录外观察 ID”之外的全量集合）。 */
export function planItemIds(plan: CollectionPlan): string[] {
  return plan.regions.flatMap((region) => region.groups.flatMap((group) => group.items.map((item) => item.id)));
}
