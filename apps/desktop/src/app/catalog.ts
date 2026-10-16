// 界面派生视图（SX-015）：把报告与元数据整理成清单行、区域计数单元格与诊断分组。
//
// 设计边界：
// - 不重新计算领域推定：遗漏/门禁一律读取报告中的 assessment/location 结论；
//   本模块只做筛选、折叠、命名与展示映射；
// - 未知与不支持显式区分：缺读数 → unknown（附原因），无映射 → unsupported；
//   超来源目标 → conflict（保留原始读数，不裁剪）。
import type { AssessedItem, InventoryRecord, LocationHint, Report } from "../contract/report";
import type { MetadataDataset } from "../contract/metadata";
import { COUNTER_CELL_SPECS, counterKey, REFERENCE_CATEGORY_ORDER } from "../data/metadata";
import { businessStatus, categoryLabel, itemEvidenceLevel, regionLabel } from "./labels";

export type RegionScope = "all" | "unknown" | string;
export type StatusFilter = "missing" | "observed" | "unknown" | "all";

export interface FilterState {
  status: StatusFilter;
  category: string | null;
  evidence: string | null;
  precision: string | null;
}

export const DEFAULT_FILTERS: FilterState = {
  status: "missing",
  category: null,
  evidence: null,
  precision: null,
};

export interface GroupRow {
  kind: "group";
  id: string;
  location: LocationHint;
  /** 当前筛选范围内该组的参考遗漏推定条目数（不复制为多个精确条目）。 */
  missingCount: number;
  /** 报告中的组成员数。 */
  memberCount: number;
  regionId: string | null;
}

export interface ItemRow {
  kind: "item";
  id: string;
  item: AssessedItem;
  location: LocationHint | null;
  regionId: string | null;
}

export type CollectibleRow = GroupRow | ItemRow;

/** 逐项 ID → 所属定位线索（组线索优先）。 */
export function locationsByMember(report: Report): Map<string, LocationHint> {
  const map = new Map<string, LocationHint>();
  const locations = [...(report.locations ?? [])].sort((a, b) => {
    const rank = (location: LocationHint) => (location.precision === "guide_group" ? 0 : 1);
    return rank(a) - rank(b);
  });
  for (const location of locations) {
    for (const id of location.member_ids) {
      if (!map.has(id)) {
        map.set(id, location);
      }
    }
  }
  return map;
}

/** 条目区域：元数据区域优先，其次所在定位线索的区域；未知为 null。 */
export function regionOfItem(
  metadata: MetadataDataset,
  report: Report,
  id: string,
  locations?: Map<string, LocationHint>,
): string | null {
  const fromMetadata = metadata.items[id]?.region_id ?? null;
  if (fromMetadata) {
    return fromMetadata;
  }
  const location = locations?.get(id) ?? locationsByMember(report).get(id);
  return location?.region_id ?? null;
}

export interface DisplayName {
  text: string;
  kind: "localized" | "curated" | "fallback";
  /** 人工简名或回退说明（必须可见，不能伪装成正式名称）。 */
  note: string | null;
  aliases: string[];
}

/** 显示名称：有来源的本地化名 → 人工简名（注明）→ “类别（名称待确认）· 短 ID”。 */
export function displayNameOf(
  metadata: MetadataDataset,
  id: string,
  shortIdOf: (value: string) => string,
): DisplayName {
  const item = metadata.items[id] ?? null;
  const category = categoryLabel(item?.category_id ?? null, metadata.categories[item?.category_id ?? ""]?.display_name ?? null);
  const aliases = item?.aliases ?? [];
  if (item?.display_name && item.name_kind !== "unknown") {
    if (item.name_kind === "curated_short") {
      return { text: item.display_name, kind: "curated", note: "人工简名，不是游戏正式名称", aliases };
    }
    return { text: item.display_name, kind: "localized", note: null, aliases };
  }
  return { text: `${category}（名称待确认）`, kind: "fallback", note: `短 ID ${shortIdOf(id)}`, aliases };
}

function searchHaystack(
  metadata: MetadataDataset,
  report: Report,
  id: string,
  locations: Map<string, LocationHint>,
): string {
  const item = metadata.items[id] ?? null;
  const display = displayNameOf(metadata, id, (value) => value);
  const region = regionOfItem(metadata, report, id, locations);
  const parts = [
    display.text,
    id,
    id.replace(/^0x/i, ""),
    ...(item?.aliases ?? []),
    item?.display_name ?? "",
    item?.category_id ?? "",
    categoryLabel(item?.category_id ?? null, null),
    regionLabel(region, region ? (metadata.regions[region]?.display_name ?? null) : null),
  ];
  return parts.filter(Boolean).join(" ").toLowerCase();
}

function matchesStatus(item: AssessedItem, status: StatusFilter): boolean {
  const missing = item.assessment === "likely_missing" || item.assessment === "verified_uncollected";
  switch (status) {
    case "missing":
      return missing;
    case "observed":
      return item.observation === "present" && !missing;
    case "unknown":
      return !missing && item.observation !== "present";
    default:
      return true;
  }
}

/** 参考遗漏入口是否可用（能力矩阵；界面不凭差集自行判断）。 */
export function missingCapabilityAvailable(report: Report): boolean {
  return report.capabilities.reference_difference.status === "supported";
}

/** 参考遗漏推定条目数（仅四个内部参考类别；不是全游戏遗漏数）。 */
export function missingCount(report: Report): number {
  return report.assessments.filter(
    (item) => item.assessment === "likely_missing" || item.assessment === "verified_uncollected",
  ).length;
}

export function observedCount(report: Report): number {
  return report.assessments.filter(
    (item) =>
      item.observation === "present" &&
      item.assessment !== "likely_missing" &&
      item.assessment !== "verified_uncollected",
  ).length;
}

export function unknownCount(report: Report): number {
  return report.assessments.filter(
    (item) =>
      item.observation !== "present" &&
      item.assessment !== "likely_missing" &&
      item.assessment !== "verified_uncollected",
  ).length;
}

/** 区域 → 参考遗漏推定条目数（用于“其余 N 项请在全部区域查看”）。 */
export function missingByRegion(metadata: MetadataDataset, report: Report): Record<string, number> {
  const locations = locationsByMember(report);
  const counts: Record<string, number> = {};
  for (const item of report.assessments) {
    if (item.assessment !== "likely_missing" && item.assessment !== "verified_uncollected") {
      continue;
    }
    const region = regionOfItem(metadata, report, item.internal_id, locations) ?? "unknown";
    counts[region] = (counts[region] ?? 0) + 1;
  }
  return counts;
}

export interface BuildRowsArgs {
  report: Report;
  metadata: MetadataDataset;
  region: RegionScope;
  filters: FilterState;
  search: string;
  shortIdOf: (value: string) => string;
}

/** 主页面清单行：遗漏视图把同组多个遗漏成员折叠为一条组行。 */
export function buildRows(args: BuildRowsArgs): CollectibleRow[] {
  const { report, metadata, region, filters, search, shortIdOf } = args;
  const locations = locationsByMember(report);
  const query = search.trim().toLowerCase();

  const inRegion = (id: string): boolean => {
    if (region === "all") {
      return true;
    }
    const resolved = regionOfItem(metadata, report, id, locations);
    return region === "unknown" ? resolved === null : resolved === region;
  };

  const items = report.assessments.filter((item) => {
    if (!matchesStatus(item, filters.status) || !inRegion(item.internal_id)) {
      return false;
    }
    const metadataItem = metadata.items[item.internal_id] ?? null;
    if (filters.category && metadataItem?.category_id !== filters.category) {
      return false;
    }
    if (filters.evidence && itemEvidenceLevel(item) !== filters.evidence) {
      return false;
    }
    const location = locations.get(item.internal_id) ?? null;
    const precision = location?.precision ?? "none";
    if (filters.precision && precision !== filters.precision) {
      return false;
    }
    if (query && !searchHaystack(metadata, report, item.internal_id, locations).includes(query)) {
      return false;
    }
    return true;
  });

  const itemRow = (item: AssessedItem): ItemRow => ({
    kind: "item",
    id: item.internal_id,
    item,
    location: locations.get(item.internal_id) ?? null,
    regionId: regionOfItem(metadata, report, item.internal_id, locations),
  });

  const rows: CollectibleRow[] = [];
  if (filters.status === "missing") {
    const grouped = new Map<string, ItemRow[]>();
    for (const item of items) {
      const location = locations.get(item.internal_id) ?? null;
      const groupId = location?.precision === "guide_group" && location.guide_group_id ? location.location_id : null;
      if (groupId && location) {
        grouped.set(groupId, [...(grouped.get(groupId) ?? []), itemRow(item)]);
      } else {
        rows.push(itemRow(item));
      }
    }
    for (const [groupId, members] of grouped) {
      const location = members[0].location as LocationHint;
      rows.push({
        kind: "group",
        id: groupId,
        location,
        missingCount: members.length,
        memberCount: location.member_ids.length,
        regionId: location.region_id,
      });
    }
  } else {
    for (const item of items) {
      rows.push(itemRow(item));
    }
  }

  return sortRows(metadata, rows, shortIdOf);
}

function sortRows(
  metadata: MetadataDataset,
  rows: CollectibleRow[],
  shortIdOf: (value: string) => string,
): CollectibleRow[] {
  const regionOrder = Object.keys(metadata.regions).sort();
  const categoryOrder = [...REFERENCE_CATEGORY_ORDER];
  const rowKey = (row: CollectibleRow) => {
    const regionId = row.kind === "group" ? row.regionId : row.regionId;
    const categoryId =
      row.kind === "group"
        ? (metadata.guide_groups[row.location.guide_group_id ?? ""]?.category_ids[0] ?? "")
        : (metadata.items[row.id]?.category_id ?? "");
    const name =
      row.kind === "group"
        ? (metadata.guide_groups[row.location.guide_group_id ?? ""]?.display_name ?? row.location.guide_group_id ?? "")
        : displayNameOf(metadata, row.id, shortIdOf).text;
    return {
      regionRank: regionId ? regionOrder.indexOf(regionId) : regionOrder.length,
      categoryRank: categoryOrder.indexOf(categoryId as (typeof REFERENCE_CATEGORY_ORDER)[number]),
      name,
      id: row.id,
    };
  };
  return [...rows].sort((a, b) => {
    const left = rowKey(a);
    const right = rowKey(b);
    return (
      left.regionRank - right.regionRank ||
      categoryRank(left.categoryRank) - categoryRank(right.categoryRank) ||
      left.name.localeCompare(right.name, "zh-CN") ||
      left.id.localeCompare(right.id)
    );
  });
}

function categoryRank(rank: number): number {
  return rank === -1 ? Number.MAX_SAFE_INTEGER : rank;
}

/** 定位精度筛选选项（含“无定位”）。 */
export const PRECISION_OPTIONS = [
  { value: "region", label: "区域线索" },
  { value: "guide_group", label: "攻略组" },
  { value: "guide_item", label: "精确条目" },
  { value: "verified_coordinates", label: "已验证坐标" },
  { value: "none", label: "无定位" },
] as const;

export const EVIDENCE_OPTIONS = [
  { value: "observed", label: "原始观察" },
  { value: "community_correlated", label: "社区关联" },
  { value: "game_validated", label: "游戏内验证" },
] as const;

/** 区域计数单元格（概览与区域页共用）。 */
export interface CountCell {
  key: string;
  categoryId: string;
  label: string;
  kind: "value" | "unknown" | "unsupported" | "conflict";
  value: number | null;
  target: number | null;
  rawValue: number | null;
  reason: string | null;
  mergedNote: string | null;
}

export function regionCells(
  metadata: MetadataDataset,
  report: Report | null,
  regionId: string,
): CountCell[] {
  return COUNTER_CELL_SPECS.map((spec) => {
    const key = counterKey(regionId, spec.counterCategoryId);
    const mapping = metadata.region_counters[key];
    const label = categoryLabel(spec.categoryId, metadata.categories[spec.categoryId]?.display_name ?? null);
    if (!mapping) {
      return {
        key,
        categoryId: spec.categoryId,
        label,
        kind: "unsupported" as const,
        value: null,
        target: null,
        rawValue: null,
        reason: "该区域/类别没有已映射的区域计数（43 项稀疏映射之外）",
        mergedNote: null,
      };
    }
    const reading = report?.observations.region_counters.find((counter) => counter.counter_key === key) ?? null;
    if (!reading) {
      return {
        key,
        categoryId: spec.categoryId,
        label,
        kind: "unknown" as const,
        value: null,
        target: mapping.source_total,
        rawValue: null,
        reason: "已映射计数，但本次快照未提供读数（未知不是 0）",
        mergedNote: spec.mergedNote ?? null,
      };
    }
    if (!reading.within_source_target || reading.value === null) {
      return {
        key,
        categoryId: spec.categoryId,
        label,
        kind: "conflict" as const,
        value: null,
        target: reading.source_target,
        rawValue: reading.raw_value,
        reason: reading.reason ?? "原始读数超出来源目标，保留原始值",
        mergedNote: spec.mergedNote ?? null,
      };
    }
    return {
      key,
      categoryId: spec.categoryId,
      label,
      kind: "value" as const,
      value: reading.value,
      target: reading.source_target,
      rawValue: reading.raw_value,
      reason: null,
      mergedNote: spec.mergedNote ?? null,
    };
  });
}

/** 其他类别的计数读数（仅计数，无逐项状态）——参考类别之外的映射读数。 */
export function otherCounterReadings(metadata: MetadataDataset, report: Report, regionId: string) {
  const referenceCategories = new Set<string>(REFERENCE_CATEGORY_ORDER);
  return report.observations.region_counters
    .filter((counter) => counter.region_id === regionId && counter.category_id && !referenceCategories.has(counter.category_id))
    .map((counter) => ({
      key: counter.counter_key,
      label: categoryLabel(counter.category_id, metadata.categories[counter.category_id]?.display_name ?? null),
      value: counter.value,
      target: counter.source_target,
    }));
}

/** 已验证坐标条目数（当前预期为 0；不生成假点位）。 */
export function verifiedCoordinateCount(report: Report): number {
  return report.locations.filter((location) => location.precision === "verified_coordinates").length;
}

/** 未分类/未知非零 ID 的原始记录（诊断用；不计入收集品数）。 */
export function unclassifiedRecords(metadata: MetadataDataset, report: Report): InventoryRecord[] {
  return report.observations.inventory.records.filter((record) => {
    const item = metadata.items[record.internal_id] ?? null;
    return item === null || item.category_id === null;
  });
}

/** 诊断分组（按“输入与解析 / 支持能力 / 计数与参考 / 定位与元数据”）。 */export interface DiagnosticGroup {
  title: string;
  items: Report["diagnostics"];
}

export function diagnosticGroups(report: Report): DiagnosticGroup[] {
  const groups: Record<string, Report["diagnostics"]> = {
    "输入与解析": [],
    "支持能力": [],
    "计数与参考": [],
    "定位与元数据": [],
  };
  for (const diagnostic of report.diagnostics) {
    switch (diagnostic.scope) {
      case "input":
      case "container":
      case "sections":
        groups["输入与解析"].push(diagnostic);
        break;
      case "inventory":
        groups["支持能力"].push(diagnostic);
        break;
      case "counters":
      case "reference_difference":
        groups["计数与参考"].push(diagnostic);
        break;
      default:
        groups["定位与元数据"].push(diagnostic);
        break;
    }
  }
  return Object.entries(groups)
    .filter(([, items]) => items.length > 0)
    .map(([title, items]) => ({ title, items }));
}

/** 覆盖提示里列出的受支持类别（来自报告 coverage，不补零）。 */
export function supportedCategories(metadata: MetadataDataset, report: Report): { id: string; label: string }[] {
  return report.coverage.supported_category_ids.map((id) => ({
    id,
    label: categoryLabel(id, metadata.categories[id]?.display_name ?? null),
  }));
}

/** 状态筛选计数（当前区域范围内，不含搜索与更多筛选；用于筛选徽标）。 */
export function filterCounts(
  metadata: MetadataDataset,
  report: Report,
  region: RegionScope,
): Record<StatusFilter, number> {
  const locations = locationsByMember(report);
  const counts: Record<StatusFilter, number> = { missing: 0, observed: 0, unknown: 0, all: 0 };
  for (const item of report.assessments) {
    if (region !== "all") {
      const resolved = regionOfItem(metadata, report, item.internal_id, locations);
      if (region === "unknown" ? resolved !== null : resolved !== region) {
        continue;
      }
    }
    counts.all += 1;
    if (item.assessment === "likely_missing" || item.assessment === "verified_uncollected") {
      counts.missing += 1;
    } else if (item.observation === "present") {
      counts.observed += 1;
    } else {
      counts.unknown += 1;
    }
  }
  return counts;
}

/** 状态标签便捷导出（组件与测试共用）。 */
export { businessStatus };

/** 按稳定 ID 取行（详情面板在筛选变化后仍可保持选中条目）。 */
export function rowById(metadata: MetadataDataset, report: Report, id: string): CollectibleRow | null {
  const locations = locationsByMember(report);
  const item = report.assessments.find((entry) => entry.internal_id === id) ?? null;
  if (item) {
    return {
      kind: "item",
      id,
      item,
      location: locations.get(id) ?? null,
      regionId: regionOfItem(metadata, report, id, locations),
    };
  }
  const location = report.locations.find((entry) => entry.location_id === id) ?? null;
  if (location) {
    const missingCount = location.member_ids.filter((member) => {
      const entry = report.assessments.find((assessment) => assessment.internal_id === member) ?? null;
      return entry?.assessment === "likely_missing" || entry?.assessment === "verified_uncollected";
    }).length;
    return {
      kind: "group",
      id,
      location,
      missingCount,
      memberCount: location.member_ids.length,
      regionId: location.region_id,
    };
  }
  return null;
}
