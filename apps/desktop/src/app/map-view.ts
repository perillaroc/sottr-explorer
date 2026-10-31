// 地图查询视图的派生数据（2026-10-31 地图查询与详情增强·阶段二）。
//
// 只做投影：把当前收集计划（`buildPlan`，随存档状态变化）与只读底图数据集
// `data/map/basemaps.json` 拼成地图视图模型；不重新解释收集状态，不校准，
// 不改写任何数据集。规则：
// - 一条底图记录 = 一张图，覆盖一个或多个 unit（当前数据是 1:1）；
// - 底图 `region_id` 不在当前计划的区域里（如数据集里的 `unnamed`）不展示；
// - 条目必须 `item.unit` 在 `basemap.unit_ids` 内且 `position` 存在，才画在地图上；
// - 排序保持计划顺序（区域 → 类别 → 子类别 → 条目），不按名称重排。
import { localizedName, t } from "./i18n";
import { categoryLabel, subcategoryLabel } from "./labels";
import { itemHaystack } from "./collection";
import { allBasemaps, basemapForUnit, basemapsForRegion, type Basemap } from "../data/basemaps";
import type { Atlas, CollectionPlan, PlanGroup, PlanItem, PlanRegion } from "../data/atlas";
import type { StatusFilter } from "./catalog";

/** 地图视图的一条底图选项（区域 + 子关卡两级选择器的第二级）。 */
export interface MapBasemapOption {
  basemap: Basemap;
  basemapId: string;
  regionSlug: string;
  regionName: string;
  unitIds: number[];
  /** 子关卡显示名（图鉴双语名；无目录单位回退 level_drm / map_path / 子关卡 N）。 */
  unitLabel: string;
  /** 有坐标的目录条目数。 */
  itemCount: number;
  collected: number;
  uncollected: number;
  status: Basemap["status"];
}

/** 地图视图的一条收集品（携带所属区域/分组，详情与清单共用）。 */
export interface MapEntry {
  region: PlanRegion;
  group: PlanGroup;
  item: PlanItem;
}

/** 地图视图的独立筛选状态（与区域清单互不影响）。 */
export interface MapFilters {
  status: StatusFilter;
  category: string | null;
  search: string;
}

/** 清单标签页的分组（类别 + 子类别；顺序与计划一致）。 */
export interface MapGroupView {
  category: string;
  subcategory: string | null;
  label: string;
  items: MapEntry[];
  collected: number;
  total: number;
}

/** 全量覆盖统计（供页头固定说明）。 */
export interface MapCoverage {
  /** 底图数据集记录数。 */
  basemaps: number;
  /** 目录中有条目的子关卡数。 */
  units: number;
  /** 其中已有底图的子关卡数。 */
  unitsWithBasemap: number;
  /** 可画到地图上的条目数（有底图 unit + 有坐标）。 */
  mappable: number;
  /** 计划中的条目总数。 */
  items: number;
}

function unitLabelOf(atlas: Atlas, basemap: Basemap): string {
  const labels: string[] = [];
  for (const unit of basemap.unit_ids) {
    const entry = atlas.units[String(unit)];
    const label = entry ? localizedName(entry.display_zh, entry.display_en, entry.zone ?? "") : "";
    labels.push(
      label ||
        basemap.level_drm ||
        basemap.map_path ||
        t(`子关卡 ${unit}`, `Sub-level ${unit}`),
    );
  }
  return labels.join(" · ");
}

/** 某张底图上的条目（保持计划中的区域/分组/条目顺序）。 */
export function mapEntriesForBasemap(plan: CollectionPlan, basemap: Basemap): MapEntry[] {
  const units = new Set(basemap.unit_ids);
  const entries: MapEntry[] = [];
  for (const region of plan.regions) {
    for (const group of region.groups) {
      for (const item of group.items) {
        if (!item.position || !item.unit || !units.has(item.unit.unit)) {
          continue;
        }
        entries.push({ region, group, item });
      }
    }
  }
  return entries;
}

function optionFor(
  plan: CollectionPlan,
  atlas: Atlas,
  basemap: Basemap,
  region: PlanRegion,
): MapBasemapOption {
  const entries = mapEntriesForBasemap(plan, basemap);
  const collected = entries.filter((entry) => entry.item.status === "collected").length;
  return {
    basemap,
    basemapId: basemap.basemap_id,
    regionSlug: region.slug,
    regionName: localizedName(region.name_zh, region.name_en, region.slug),
    unitIds: [...basemap.unit_ids],
    unitLabel: unitLabelOf(atlas, basemap),
    itemCount: entries.length,
    collected,
    uncollected: entries.length - collected,
    status: basemap.status,
  };
}

function regionOrder(plan: CollectionPlan): Map<string, number> {
  return new Map(plan.regions.map((region, index) => [region.slug, index]));
}

function minUnit(basemap: Basemap): number {
  return Math.min(...basemap.unit_ids);
}

/**
 * 当前计划可见的全部底图选项：区域按计划顺序，同区域按最小 unit 升序。
 * `itemCount === 0` 的底图仍保留（避免「图不见了」）。
 */
export function mapOptions(plan: CollectionPlan, atlas: Atlas): MapBasemapOption[] {
  const order = regionOrder(plan);
  const options: MapBasemapOption[] = [];
  for (const basemap of allBasemaps()) {
    const region = plan.regions.find((entry) => entry.slug === basemap.region_id);
    if (!region) {
      continue;
    }
    options.push(optionFor(plan, atlas, basemap, region));
  }
  options.sort((left, right) => {
    const leftOrder = order.get(left.regionSlug) ?? Number.MAX_SAFE_INTEGER;
    const rightOrder = order.get(right.regionSlug) ?? Number.MAX_SAFE_INTEGER;
    if (leftOrder !== rightOrder) {
      return leftOrder - rightOrder;
    }
    const diff = minUnit(left.basemap) - minUnit(right.basemap);
    return diff !== 0 ? diff : left.basemapId.localeCompare(right.basemapId);
  });
  return options;
}

/** 某区域的底图选项（区域不存在或无底图时为空）。 */
export function mapOptionsForRegion(
  plan: CollectionPlan,
  atlas: Atlas,
  regionSlug: string | null,
): MapBasemapOption[] {
  if (!regionSlug) {
    return [];
  }
  return mapOptions(plan, atlas).filter((option) => option.regionSlug === regionSlug);
}

/** 区域默认底图：优先有带坐标条目的，其次第一张；无底图时为 null。 */
export function defaultMapBasemap(
  plan: CollectionPlan,
  atlas: Atlas,
  regionSlug: string | null,
): MapBasemapOption | null {
  const options = regionSlug ? mapOptionsForRegion(plan, atlas, regionSlug) : mapOptions(plan, atlas);
  return options.find((option) => option.itemCount > 0) ?? options[0] ?? null;
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

/** 按地图视图的独立筛选过滤条目（保持计划顺序）。 */
export function filterMapEntries(entries: MapEntry[], filters: MapFilters): MapEntry[] {
  const query = filters.search.trim().toLowerCase();
  return entries.filter(({ group, item }) => {
    if (filters.category && group.category !== filters.category) {
      return false;
    }
    if (!matchesStatus(item, filters.status)) {
      return false;
    }
    return query.length === 0 || itemHaystack(item, group).includes(query);
  });
}

/**
 * 清单标签页的分组：按类别 + 子类别聚合，顺序按计划中出现顺序；
 * 组的 collected/total 保持整组真实计数（筛选只影响显示的条目），与区域清单一致。
 */
export function mapGroups(entries: MapEntry[], filters: MapFilters): MapGroupView[] {
  const visible = new Set(filterMapEntries(entries, filters).map((entry) => entry.item.id));
  const order: string[] = [];
  const groups = new Map<string, MapGroupView>();
  for (const entry of entries) {
    const key = `${entry.group.category}\u0000${entry.group.subcategory ?? ""}`;
    let view = groups.get(key);
    if (!view) {
      view = {
        category: entry.group.category,
        subcategory: entry.group.subcategory ?? null,
        label: subcategoryLabel(entry.group.subcategory ?? null) ?? categoryLabel(entry.group.category, entry.group.name),
        items: [],
        collected: 0,
        total: 0,
      };
      groups.set(key, view);
      order.push(key);
    }
    view.total += 1;
    if (entry.item.status === "collected") {
      view.collected += 1;
    }
    if (visible.has(entry.item.id)) {
      view.items.push(entry);
    }
  }
  const result: MapGroupView[] = [];
  for (const key of order) {
    const view = groups.get(key);
    if (view && view.items.length > 0) {
      result.push(view);
    }
  }
  return result;
}

/** 属于该图 unit 但没有坐标的条目数（本图底部说明）。 */
export function unplacedInBasemap(plan: CollectionPlan, basemap: Basemap): number {
  const units = new Set(basemap.unit_ids);
  let count = 0;
  for (const region of plan.regions) {
    for (const group of region.groups) {
      for (const item of group.items) {
        if (item.unit && units.has(item.unit.unit) && !item.position) {
          count += 1;
        }
      }
    }
  }
  return count;
}

/** 区域内没有底图的条目数（所属 unit 完全没有底图，或没有 unit）。 */
export function unmappedInRegion(plan: CollectionPlan, regionSlug: string): number {
  const region = plan.regions.find((entry) => entry.slug === regionSlug);
  if (!region) {
    return 0;
  }
  let count = 0;
  for (const group of region.groups) {
    for (const item of group.items) {
      if (!item.unit || !basemapForUnit(item.unit.unit)) {
        count += 1;
      }
    }
  }
  return count;
}

/** 区域是否有任何底图记录。 */
export function regionHasBasemap(plan: CollectionPlan, regionSlug: string): boolean {
  return (
    plan.regions.some((region) => region.slug === regionSlug) && basemapsForRegion(regionSlug).length > 0
  );
}

/** 全量覆盖统计：底图数据集范围 + 当前计划的可上图数量。 */
export function mapCoverage(plan: CollectionPlan, atlas: Atlas): MapCoverage {
  let items = 0;
  let mappable = 0;
  for (const region of plan.regions) {
    for (const group of region.groups) {
      for (const item of group.items) {
        items += 1;
        if (item.unit && item.position && basemapForUnit(item.unit.unit)) {
          mappable += 1;
        }
      }
    }
  }
  // 子关卡总数取图鉴 units（含没有条目的 unit 0）；有底图的按数据集 unit 与图鉴交集计。
  const basemapUnits = new Set(allBasemaps().flatMap((basemap) => basemap.unit_ids));
  const unitsWithBasemap = [...basemapUnits].filter(
    (unit) => atlas.units[String(unit)] !== undefined,
  ).length;
  return {
    basemaps: allBasemaps().length,
    units: Object.keys(atlas.units).length,
    unitsWithBasemap,
    mappable,
    items,
  };
}
