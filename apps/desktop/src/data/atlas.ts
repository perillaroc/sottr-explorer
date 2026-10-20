// 收集图鉴（SX-022）：图鉴数据视图 + 按区域收集计划（镜像 `sottr_core::atlas`）。
//
// 规则必须与 Rust 核心逐条一致（黄金测试 `tests/atlas-plan.spec.ts` 对照 CLI 输出）：
// - 状态两态：观察集合内 = `collected`，否则 `not_collected`；
// - 必收 / 可选 / 未闭合由 `required_for_100` 表达，界面不自行推断；
// - 特殊区域（`_` 前缀与 `croft_manor`）单独成组，不计入主区域合计；
// - 位置是关卡内归一化坐标，**未校准**，只作相对参照；
// - 不在图鉴内的观察 ID 单列，不混入任何区域。
//
// 该模块只做数据整理与展示组合，不重新解释游戏数据：区域绑定、子类别与名称
// 全部来自版本化数据集 `data/catalog/collectibles_catalog.json`。

import raw from "../../../../data/catalog/collectibles_catalog.json";
import { localizedName, t } from "../app/i18n";

export interface AtlasPosition {
  x: number;
  y: number;
  space: string;
}

export interface AtlasItem {
  kind: "story" | "nonstory";
  name: string | null;
  name_en: string | null;
  name_zh: string | null;
  name_source: string | null;
  collection?: string;
  type?: number;
  marker?: string | null;
  is_dlc: boolean;
  required_for_100: boolean | null;
  region_hash: string | null;
  region_slug: string | null;
  category: string;
  subcategory: string | null;
  unit: number | null;
  position: AtlasPosition | null;
  acquisition_note: string | null;
  shared_id?: boolean;
  shared_names?: string[];
}

export interface AtlasSubcategory {
  id: string;
  name: string;
  count: number;
  acquisition_rule_key: string;
}

export interface AtlasCategory {
  id: string;
  name: string;
  count: number;
  required_for_100: boolean | null;
  required_count: number;
  acquisition_rule_key: string;
  note?: string;
  subcategories?: AtlasSubcategory[];
}

export interface AtlasAcquisitionRule {
  text_zh: string;
  text_en: string;
  source: string;
  limitations?: string[];
}

export interface AtlasUnit {
  zone: string | null;
  display_en: string | null;
  display_zh: string | null;
  name_source: string;
  region_slug: string | null;
}

export interface AtlasRegionBinding {
  signature_distance: number;
  runner_up_distance: number | null;
  signature: Record<string, number>;
  source_targets: Record<string, number>;
}

export interface AtlasRegion {
  region_hash: string;
  slug: string | null;
  name_en: string | null;
  name_zh: string | null;
  name_source: string | null;
  note?: string | null;
  is_dlc: boolean;
  counts_toward_percent: boolean;
  item_count: number;
  binding: AtlasRegionBinding | null;
  challenge_tombs: number[];
  campsites: number[];
  challenges: number[];
  missions: number[];
}

export interface AtlasCollection {
  name: string;
  is_dlc: boolean;
  count: number;
  required_for_100: boolean;
}

export interface Atlas {
  catalog_schema_version: string;
  catalog_version: string;
  generated_from: Record<string, unknown>;
  totals: {
    items: number;
    story: number;
    nonstory: number;
    required_for_100: number;
    dlc: number;
    with_position: number;
    with_region: number;
  };
  acquisition_rules: Record<string, AtlasAcquisitionRule>;
  categories: AtlasCategory[];
  collections: AtlasCollection[];
  units: Record<string, AtlasUnit>;
  regions: AtlasRegion[];
  items: Record<string, AtlasItem>;
}

/** 生产图鉴（仓库数据集，只读）。 */
export const productionAtlas = raw as unknown as Atlas;

/**
 * 模拟模式 / 测试用的固定观察集合：按目录顺序每 3 条取 2 条“已收集”。
 * 只用于让演示与断言可复现，不代表任何真实存档。
 */
export const demoObservedIds: string[] = Object.keys(productionAtlas.items).filter(
  (_, index) => index % 3 !== 0,
);

export type ItemStatus = "collected" | "not_collected";
export type RegionKind = "main" | "special";

export interface PlanUnit {
  unit: number;
  zone: string | null;
  display_zh: string | null;
  display_en: string | null;
  name_source: string;
}

export interface PlanItem {
  id: string;
  display_name: string;
  name_en?: string;
  name_zh?: string;
  name_source?: string;
  internal_name?: string;
  collection?: string;
  status: ItemStatus;
  is_dlc: boolean;
  required_for_100: boolean | null;
  unit?: PlanUnit;
  position?: AtlasPosition;
  acquisition_text: string;
  acquisition_source: "item_note" | "category_rule";
  location_hint: string;
  same_unit_remaining?: number;
  shared_names?: string[];
}

export interface PlanGroup {
  category: string;
  subcategory?: string;
  name: string;
  required_for_100: boolean | null;
  collected: number;
  total: number;
  acquisition_rule_key: string;
  items: PlanItem[];
}

export interface PlanRegion {
  slug: string;
  region_hash: string;
  name_zh?: string;
  name_en?: string;
  name_source?: string;
  kind: RegionKind;
  note?: string;
  collected: number;
  total: number;
  groups: PlanGroup[];
}

export interface CollectionPlan {
  catalog_version: string;
  main_collected: number;
  main_total: number;
  regions: PlanRegion[];
  observed_not_in_catalog: string[];
}

/** 与核心 `missing::normalize_observed` 一致：`0x` + 8 位十六进制 → 大写。 */
export function normalizeObserved(id: string): string | null {
  const trimmed = id.trim();
  let body: string;
  if (trimmed.startsWith("0x") || trimmed.startsWith("0X")) {
    body = trimmed.slice(2);
  } else {
    return null;
  }
  if (body.length !== 8 || !/^[0-9a-fA-F]{8}$/.test(body)) {
    return null;
  }
  return `0x${body.toUpperCase()}`;
}

export function regionKind(slug: string): RegionKind {
  return slug.startsWith("_") || slug === "croft_manor" ? "special" : "main";
}

function unitOf(atlas: Atlas, unit: number | null): PlanUnit | undefined {
  if (!unit) {
    return undefined;
  }
  const entry = atlas.units[String(unit)];
  if (!entry) {
    return undefined;
  }
  return {
    unit,
    zone: entry.zone,
    display_zh: entry.display_zh,
    display_en: entry.display_en,
    name_source: entry.name_source,
  };
}

export function displayNameOfItem(item: AtlasItem, id: string): string {
  return localizedName(item.name_zh, item.name_en ?? item.name, id);
}

function locationHint(
  regionName: string,
  unit: PlanUnit | undefined,
  position: AtlasPosition | undefined,
  othersUncollected: number | undefined,
): string {
  const parts = [regionName];
  const unitName = unit ? localizedName(unit.display_zh, unit.display_en, unit.zone ?? "") : null;
  if (unitName && !parts.includes(unitName)) {
    parts.push(unitName);
  }
  let text = parts.join(" · ");
  if (position) {
    text += t(
      `；关卡内相对位置约 (${position.x.toFixed(2)}, ${position.y.toFixed(2)})（未校准，仅供同关卡内相对参照）`,
      `; in-level relative position ≈ (${position.x.toFixed(2)}, ${position.y.toFixed(2)}) (uncalibrated; relative reference within the same level only)`,
    );
  }
  if (othersUncollected !== undefined && othersUncollected > 0) {
    text += t(
      `；该子关卡还有 ${othersUncollected} 项未收集`,
      `; ${othersUncollected} more uncollected in this sub-level`,
    );
  }
  return text;
}

/** 特殊区域的英文说明（图鉴数据只提供中文 note）。 */
const REGION_NOTES_EN: Record<string, string> = {
  _challenge_tombs:
    "Aggregate challenge-tomb region in game data (regionProgressionIndex=999, hideOnMap=1); it has no save counter and is shown per sub-level.",
  _special:
    "Special region in game data outside the 26-region main table; includes cross-region items such as Path of the Stars and Lara's Notebook.",
  croft_manor: "Prologue manor area; does not count toward 100%.",
};

function compareItems(
  left: [string, AtlasItem],
  right: [string, AtlasItem],
): number {
  const key = (item: AtlasItem): [number, number, number] => [
    item.unit ?? 0,
    item.position?.y ?? -1,
    item.position?.x ?? -1,
  ];
  const a = key(left[1]);
  const b = key(right[1]);
  for (let index = 0; index < a.length; index += 1) {
    if (a[index] !== b[index]) {
      return a[index] - b[index];
    }
  }
  return left[0] < right[0] ? -1 : left[0] > right[0] ? 1 : 0;
}

/** 计算按区域的收集计划（镜像 `sottr_core::atlas::plan`）。 */
export function buildPlan(atlas: Atlas, observedIds: Iterable<string>): CollectionPlan {
  const observed = new Set<string>();
  const notInCatalog: string[] = [];
  for (const rawId of observedIds) {
    const normalized = normalizeObserved(rawId);
    if (normalized === null) {
      notInCatalog.push(String(rawId).trim().toUpperCase());
    } else {
      if (!(normalized in atlas.items)) {
        notInCatalog.push(normalized);
      }
      observed.add(normalized);
    }
  }
  notInCatalog.sort();
  const observedNotInCatalog = [...new Set(notInCatalog)];

  // 区域 → 条目
  const byRegion = new Map<string, [string, AtlasItem][]>();
  for (const [id, item] of Object.entries(atlas.items)) {
    const slug = item.region_slug;
    if (!slug) {
      continue;
    }
    const bucket = byRegion.get(slug) ?? [];
    bucket.push([id, item]);
    byRegion.set(slug, bucket);
  }

  // 同子关卡内未收集数（跨类别）
  const unitRemaining = new Map<string, number>();
  for (const [slug, entries] of byRegion) {
    for (const [id, item] of entries) {
      if (observed.has(id)) {
        continue;
      }
      if (item.unit) {
        const key = `${slug}:${item.unit}`;
        unitRemaining.set(key, (unitRemaining.get(key) ?? 0) + 1);
      }
    }
  }

  // 主区域按图鉴区域顺序，特殊区域在后
  const ordered: { kind: RegionKind; slug: string; regionHash: string }[] = [];
  for (const region of atlas.regions) {
    const slug = region.slug;
    if (!slug || !byRegion.has(slug)) {
      continue;
    }
    ordered.push({ kind: regionKind(slug), slug, regionHash: region.region_hash });
  }
  ordered.sort((left, right) => (left.kind === right.kind ? 0 : left.kind === "main" ? -1 : 1));

  const regions: PlanRegion[] = [];
  let mainCollected = 0;
  let mainTotal = 0;

  for (const { kind, slug, regionHash } of ordered) {
    const entries = byRegion.get(slug) ?? [];
    const record = atlas.regions.find((region) => region.region_hash === regionHash);
    const nameZh = record?.name_zh ?? undefined;
    const nameEn = record?.name_en ?? undefined;
    const regionName = localizedName(nameZh, nameEn, slug);

    const buckets = new Map<string, { category: string; subcategory: string | null; items: [string, AtlasItem][] }>();
    for (const [id, item] of entries) {
      const key = `${item.category}\u0000${item.subcategory ?? ""}`;
      const bucket = buckets.get(key) ?? { category: item.category, subcategory: item.subcategory, items: [] };
      bucket.items.push([id, item]);
      buckets.set(key, bucket);
    }

    const groups: PlanGroup[] = [];
    let regionCollected = 0;
    // 与核心 BTreeMap<(category, subcategory)> 的顺序一致：类别升序，None 在 Some 之前。
    const orderedBuckets = [...buckets.values()].sort((left, right) => {
      if (left.category !== right.category) {
        return left.category < right.category ? -1 : 1;
      }
      const a = left.subcategory ?? "";
      const b = right.subcategory ?? "";
      return a === b ? 0 : a < b ? -1 : 1;
    });
    for (const bucket of orderedBuckets) {
      const category = atlas.categories.find((entry) => entry.id === bucket.category);
      const sub = bucket.subcategory
        ? category?.subcategories?.find((entry) => entry.id === bucket.subcategory)
        : undefined;
      const ruleKey = sub?.acquisition_rule_key ?? category?.acquisition_rule_key ?? "";
      const rule = atlas.acquisition_rules[ruleKey];
      const ruleText = rule ? t(rule.text_zh, rule.text_en) : "";
      // 分组名保持目录原始值（与 Rust 核心/CLI 黄金输出一致）；界面按类别/子类别 id
      // 做双语映射，见 `app/collection.ts` 的 `groupDisplayName`。
      const groupName = sub?.name ?? category?.name ?? bucket.category;

      bucket.items.sort(compareItems);
      const items: PlanItem[] = [];
      let collected = 0;
      for (const [id, item] of bucket.items) {
        const isCollected = observed.has(id);
        if (isCollected) {
          collected += 1;
        }
        const status: ItemStatus = isCollected ? "collected" : "not_collected";
        const unit = unitOf(atlas, item.unit);
        const total = item.unit ? (unitRemaining.get(`${slug}:${item.unit}`) ?? undefined) : undefined;
        const othersUncollected =
          total === undefined
            ? undefined
            : status === "not_collected"
              ? Math.max(total - 1, 0)
              : total;
        const note = item.acquisition_note ?? null;
        items.push({
          id,
          display_name: displayNameOfItem(item, id),
          name_en: item.name_en ?? undefined,
          name_zh: item.name_zh ?? undefined,
          name_source: item.name_source ?? undefined,
          internal_name: item.name ?? undefined,
          collection: item.collection,
          status,
          is_dlc: item.is_dlc,
          required_for_100: item.required_for_100,
          unit,
          position: item.position ?? undefined,
          acquisition_text: note ?? ruleText,
          acquisition_source: note !== null ? "item_note" : "category_rule",
          location_hint: locationHint(regionName, unit, item.position ?? undefined, othersUncollected),
          same_unit_remaining: othersUncollected,
          shared_names: item.shared_names,
        });
      }
      regionCollected += collected;
      groups.push({
        category: bucket.category,
        subcategory: bucket.subcategory ?? undefined,
        name: groupName,
        required_for_100: category?.required_for_100 ?? null,
        collected,
        total: items.length,
        acquisition_rule_key: ruleKey,
        items,
      });
    }

    const regionTotal = groups.reduce((sum, group) => sum + group.items.length, 0);
    if (kind === "main") {
      mainCollected += regionCollected;
      mainTotal += regionTotal;
    }
    regions.push({
      slug,
      region_hash: regionHash,
      name_zh: nameZh,
      name_en: nameEn,
      name_source: record?.name_source ?? undefined,
      kind,
      note: record?.note
        ? localizedName(record.note, record.slug ? REGION_NOTES_EN[record.slug] : undefined, record.note)
        : undefined,
      collected: regionCollected,
      total: regionTotal,
      groups,
    });
  }

  return {
    catalog_version: atlas.catalog_version,
    main_collected: mainCollected,
    main_total: mainTotal,
    regions,
    observed_not_in_catalog: observedNotInCatalog,
  };
}

/** 区域显示名（当前语言；缺失回退另一种语言）。 */
export function regionDisplayName(region: PlanRegion): string {
  return localizedName(region.name_zh, region.name_en, region.slug);
}

/** 计划内全部条目（用于搜索/诊断）。 */
export function planItems(plan: CollectionPlan): PlanItem[] {
  return plan.regions.flatMap((region) => region.groups.flatMap((group) => group.items));
}

/** 从计划里按 ID 取条目及其所属区域/分组。 */
export function findPlanItem(
  plan: CollectionPlan,
  id: string,
): { region: PlanRegion; group: PlanGroup; item: PlanItem } | null {
  for (const region of plan.regions) {
    for (const group of region.groups) {
      const item = group.items.find((entry) => entry.id === id);
      if (item) {
        return { region, group, item };
      }
    }
  }
  return null;
}
