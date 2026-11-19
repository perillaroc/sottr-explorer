// 进度增量与历史（P1-3）：同一次会话内的两次分析 → 总进度 / 区域 / 条目差异。
//
// 边界：
// - 只做比较与整理，不重新判定收集状态：观察集合、图鉴状态沿用 `buildPlan`；
// - 对比对象是**当前存档在本次会话中的两次分析**；历史只存内存，不落盘、不含路径。
//   磁盘持久化受 D5 约束，本轮不实现。
// - 不可比时不推断增量：不同输入文件、非战役上下文、物品表不完整都给出明确原因，
//   新增/丢失列表保持为空，避免把“看不到”当成“没有变化”。
// - 差分是确定的：观察集合先归一化并排序，新增/丢失按区域顺序、再按条目 ID 排序。

import type { Report } from "../contract/report";
import { buildPlan, productionAtlas, type Atlas, type CollectionPlan } from "../data/atlas";
import { t, localizedName } from "./i18n";
import { categoryLabel } from "./labels";

export type ProgressMode = "campaign" | "unknown" | "profile" | "unsupported";

/** 一次分析的可比较摘要（只含条目 ID 与计数，不含绝对路径）。 */
export interface ProgressSnapshot {
  /** 输入引用（文件名；不含目录）。 */
  inputRef: string;
  snapshotId: string;
  snapshotTime: string;
  mode: ProgressMode;
  /** 报告自带的物品表完整性标记；false 时不推断新增/丢失。 */
  inventoryComplete: boolean;
  /** 归一化后属于图鉴的已观测 ID（升序）。 */
  observed: string[];
  /** 目录外观察 ID 数（不进入条目差异，只做说明）。 */
  observedNotInCatalog: number;
  mainCollected: number;
  mainTotal: number;
  requiredCollected: number;
  requiredTotal: number;
}

export type ProgressDeltaStatus =
  | "comparable"
  | "different_save"
  | "not_campaign"
  | "inventory_incomplete";

export interface ProgressItemDelta {
  id: string;
  name: string;
  category: string;
  categoryLabel: string;
  regionSlug: string | null;
  regionLabel: string;
  required: boolean | null;
}

export interface ProgressRegionDelta {
  slug: string;
  label: string;
  kind: "main" | "special";
  before: number;
  after: number;
  total: number;
  delta: number;
}

export interface ProgressDelta {
  status: ProgressDeltaStatus;
  comparable: boolean;
  before: ProgressSnapshot;
  after: ProgressSnapshot;
  /** 主区域收集品总数差（after − before）；不可比时为 0。 */
  totalDelta: number;
  /** 其中必收条目的差；不可比时为 0。 */
  requiredDelta: number;
  added: ProgressItemDelta[];
  removed: ProgressItemDelta[];
  /** 变化的区域（before ≠ after），按图鉴区域顺序。 */
  regions: ProgressRegionDelta[];
}

/** 每个槽位在会话内保留的分析次数上限。 */
export const MAX_PROGRESS_HISTORY = 20;

/** 从报告与观察集合生成快照摘要（计数来自 `buildPlan`，与界面一致）。 */
export function progressSnapshotOf(
  identity: Pick<ProgressSnapshot, "inputRef" | "snapshotId" | "snapshotTime">,
  report: Report,
  observedIds: Iterable<string>,
  atlas: Atlas = productionAtlas,
): ProgressSnapshot {
  const plan = buildPlan(atlas, observedIds);
  const observed: string[] = [];
  let requiredCollected = 0;
  let requiredTotal = 0;
  for (const region of plan.regions) {
    for (const group of region.groups) {
      for (const item of group.items) {
        if (item.required_for_100 === true) {
          requiredTotal += 1;
        }
        if (item.status === "collected") {
          observed.push(item.id);
          if (item.required_for_100 === true) {
            requiredCollected += 1;
          }
        }
      }
    }
  }
  observed.sort();
  return {
    inputRef: identity.inputRef,
    snapshotId: identity.snapshotId,
    snapshotTime: identity.snapshotTime,
    mode: report.context.mode,
    inventoryComplete: report.observations.inventory.inventory_complete,
    observed,
    observedNotInCatalog: plan.observed_not_in_catalog.length,
    mainCollected: plan.main_collected,
    mainTotal: plan.main_total,
    requiredCollected,
    requiredTotal,
  };
}

function regionLabels(atlas: Atlas): Map<string, string> {
  const labels = new Map<string, string>();
  for (const region of atlas.regions) {
    if (region.slug) {
      labels.set(
        region.slug,
        localizedName(region.name_zh, region.name_en, region.slug),
      );
    }
  }
  return labels;
}

function regionOrder(plan: CollectionPlan): Map<string, number> {
  const order = new Map<string, number>();
  plan.regions.forEach((region, index) => order.set(region.slug, index));
  return order;
}

function itemDeltaOf(id: string, atlas: Atlas, labels: Map<string, string>): ProgressItemDelta {
  const item = atlas.items[id];
  const slug = item?.region_slug ?? null;
  return {
    id,
    name: item ? localizedName(item.name_zh, item.name_en ?? item.name, id) : id,
    category: item?.category ?? "unknown",
    categoryLabel: categoryLabel(item?.category ?? null, null),
    regionSlug: slug,
    regionLabel: slug ? (labels.get(slug) ?? slug) : t("progress.unknown-region"),
    required: item?.required_for_100 ?? null,
  };
}

function sortDeltas(
  deltas: ProgressItemDelta[],
  order: Map<string, number>,
): ProgressItemDelta[] {
  return [...deltas].sort((left, right) => {
    const leftRegion = left.regionSlug === null ? Number.MAX_SAFE_INTEGER : (order.get(left.regionSlug) ?? Number.MAX_SAFE_INTEGER);
    const rightRegion = right.regionSlug === null ? Number.MAX_SAFE_INTEGER : (order.get(right.regionSlug) ?? Number.MAX_SAFE_INTEGER);
    if (leftRegion !== rightRegion) {
      return leftRegion - rightRegion;
    }
    return left.id < right.id ? -1 : left.id > right.id ? 1 : 0;
  });
}

/** 计算两次分析的差异；不可比时只返回原因与两侧摘要。 */
export function computeProgressDelta(
  before: ProgressSnapshot,
  after: ProgressSnapshot,
  atlas: Atlas = productionAtlas,
): ProgressDelta {
  const base: ProgressDelta = {
    status: "comparable",
    comparable: true,
    before,
    after,
    totalDelta: 0,
    requiredDelta: 0,
    added: [],
    removed: [],
    regions: [],
  };

  if (before.mode !== "campaign" || after.mode !== "campaign") {
    return { ...base, status: "not_campaign", comparable: false };
  }
  if (before.inputRef !== after.inputRef) {
    return { ...base, status: "different_save", comparable: false };
  }
  if (!before.inventoryComplete || !after.inventoryComplete) {
    return { ...base, status: "inventory_incomplete", comparable: false };
  }

  const beforeSet = new Set(before.observed);
  const afterSet = new Set(after.observed);
  const allPlan = buildPlan(atlas, []);
  const order = regionOrder(allPlan);
  const labels = regionLabels(atlas);

  const added = sortDeltas(
    after.observed.filter((id) => !beforeSet.has(id)).map((id) => itemDeltaOf(id, atlas, labels)),
    order,
  );
  const removed = sortDeltas(
    before.observed.filter((id) => !afterSet.has(id)).map((id) => itemDeltaOf(id, atlas, labels)),
    order,
  );

  const regions: ProgressRegionDelta[] = [];
  for (const region of allPlan.regions) {
    const ids = region.groups.flatMap((group) => group.items.map((item) => item.id));
    const beforeCount = ids.filter((id) => beforeSet.has(id)).length;
    const afterCount = ids.filter((id) => afterSet.has(id)).length;
    if (beforeCount === afterCount) {
      continue;
    }
    regions.push({
      slug: region.slug,
      label: localizedName(region.name_zh, region.name_en, region.slug),
      kind: region.kind,
      before: beforeCount,
      after: afterCount,
      total: ids.length,
      delta: afterCount - beforeCount,
    });
  }

  return {
    ...base,
    totalDelta: after.mainCollected - before.mainCollected,
    requiredDelta: after.requiredCollected - before.requiredCollected,
    added,
    removed,
    regions,
  };
}

/** 追加一次快照：同一快照 ID 不重复记录；超出上限丢弃最旧。 */
export function appendProgressSnapshot(
  history: ProgressSnapshot[],
  snapshot: ProgressSnapshot,
): ProgressSnapshot[] {
  if (history.some((entry) => entry.snapshotId === snapshot.snapshotId)) {
    return history;
  }
  const next = [...history, snapshot];
  return next.length > MAX_PROGRESS_HISTORY ? next.slice(next.length - MAX_PROGRESS_HISTORY) : next;
}

/** 默认对比基线：最近一次之前的那次分析（没有则 `null`）。 */
export function defaultBaselineId(history: ProgressSnapshot[]): string | null {
  return history.length >= 2 ? history[history.length - 2].snapshotId : null;
}
