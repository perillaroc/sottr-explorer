// 全收集目录与缺失清单（SX-021 阶段 C2a）：
// - 目录数据由 data/catalog/build_catalog.py 生成（Vite 直接内嵌，与元数据同一做法）；
// - computeMissing 是 sottr-core::missing 的镜像实现，规则必须逐条一致：
//   必收类别给逐项清单（计数与清单按必收子集，DLC 与非必收不混入）、可选类别只给计数、
//   未闭合类别无缺失结论、共用 ID 附全部名称、未知 ID 单列不混入缺失；
// - 一致性由 tests/catalog-missing.spec.ts 以 Rust CLI 生成的黄金输出验证。
import rawCatalog from "../../../../data/catalog/collectibles_catalog.json";

/** 目录条目。 */
export interface CatalogItem {
  kind: "story" | "nonstory";
  name: string | null;
  collection?: string;
  type?: number;
  marker?: string;
  is_dlc: boolean;
  required_for_100: boolean | null;
  region_hash?: string;
  region_slug?: string | null;
  unit?: number | null;
  category: string;
  shared_id?: boolean;
  shared_names?: string[];
}

/** 目录类别。 */
export interface CatalogCategory {
  id: string;
  name: string;
  count: number;
  required_for_100: boolean | null;
  required_count: number;
  note?: string;
}

/** 全收集目录。 */
export interface Catalog {
  catalog_schema_version: string;
  catalog_version: string;
  totals: {
    items: number;
    story: number;
    nonstory: number;
    required_for_100: number;
    dlc: number;
  };
  categories: CatalogCategory[];
  collections: unknown[];
  regions: unknown[];
  items: Record<string, CatalogItem>;
}

/** 真实模式使用的全收集目录（仓库数据集，只读）。 */
export const productionCatalog = rawCatalog as unknown as Catalog;

/** 未闭合类别不生成缺失清单。 */
export const REASON_UNRESOLVED = "未闭合类别：不生成缺失清单（T22）";
/** 可选类别不生成逐项缺失清单。 */
export const REASON_OPTIONAL = "可选类别：不在 100% 必需范围，不生成逐项清单";

/** 单条缺失。 */
export interface MissingItem {
  id: string;
  name: string | null;
  collection?: string;
  shared_names?: string[];
}

/** 类别缺失进度。 */
export interface CategoryMissing {
  id: string;
  name: string;
  required_for_100: boolean | null;
  total: number;
  observed: number;
  missing?: number;
  reason?: string;
  items?: MissingItem[];
}

/** 全收集缺失报告。 */
export interface MissingReport {
  catalog_version: string;
  observed_items: number;
  observed_not_in_catalog: string[];
  categories: CategoryMissing[];
}

function normalizeObserved(id: string): string | null {
  const trimmed = id.trim();
  const body = trimmed.startsWith("0x") || trimmed.startsWith("0X") ? trimmed.slice(2) : null;
  if (body !== null && body.length === 8 && /^[0-9a-fA-F]{8}$/.test(body)) {
    return `0x${body.toUpperCase()}`;
  }
  return null;
}

/** 计算缺失报告（与 sottr-core::missing 规则一致）。 */
export function computeMissing(catalog: Catalog, observedIds: Iterable<string>): MissingReport {
  const normalized = new Set<string>();
  const notInCatalog: string[] = [];
  for (const raw of observedIds) {
    const id = normalizeObserved(raw);
    if (id === null) {
      notInCatalog.push(raw.trim().toUpperCase());
      continue;
    }
    if (!(id in catalog.items)) {
      notInCatalog.push(id);
    }
    normalized.add(id);
  }
  notInCatalog.sort();
  const observedNotInCatalog = notInCatalog.filter((id, index) => index === 0 || id !== notInCatalog[index - 1]);

  let observedItems = 0;
  for (const id of normalized) {
    if (id in catalog.items) {
      observedItems += 1;
    }
  }

  const entriesByCategory = new Map<string, Array<[string, CatalogItem]>>();
  for (const [id, item] of Object.entries(catalog.items)) {
    const list = entriesByCategory.get(item.category) ?? [];
    list.push([id, item]);
    entriesByCategory.set(item.category, list);
  }

  const categories: CategoryMissing[] = catalog.categories.map((category) => {
    const entries = entriesByCategory.get(category.id) ?? [];
    if (category.required_for_100 === true) {
      const requiredEntries = entries.filter(([, item]) => item.required_for_100 === true);
      const observed = requiredEntries.filter(([id]) => normalized.has(id)).length;
      const items: MissingItem[] = requiredEntries
        .filter(([id]) => !normalized.has(id))
        .map(([id, item]) => {
          const missing: MissingItem = { id, name: item.name };
          if (item.collection !== undefined) {
            missing.collection = item.collection;
          }
          if (item.shared_names !== undefined) {
            missing.shared_names = item.shared_names;
          }
          return missing;
        })
        .sort((left, right) => (left.id < right.id ? -1 : left.id > right.id ? 1 : 0));
      return {
        id: category.id,
        name: category.name,
        required_for_100: true,
        total: requiredEntries.length,
        observed,
        missing: items.length,
        items,
      };
    }
    const observed = entries.filter(([id]) => normalized.has(id)).length;
    if (category.required_for_100 === false) {
      return {
        id: category.id,
        name: category.name,
        required_for_100: false,
        total: entries.length,
        observed,
        missing: entries.length - observed,
        reason: REASON_OPTIONAL,
      };
    }
    return {
      id: category.id,
      name: category.name,
      required_for_100: null,
      total: entries.length,
      observed,
      reason: REASON_UNRESOLVED,
    };
  });

  return {
    catalog_version: catalog.catalog_version,
    observed_items: observedItems,
    observed_not_in_catalog: observedNotInCatalog,
    categories,
  };
}

/** 一个 DLC 故事包（7 组）的进度。 */
export interface DlcCollectionProgress {
  /** 合集键，如 `DLC01_COLLECT_STORY`。 */
  id: string;
  /** 短标签，如 `DLC01`。 */
  label: string;
  total: number;
  observed: number;
}

/** 按合集把 DLC 故事条目分组；DLC 不进入必收 352，也不参与全收集结论。 */
export function dlcCollectionProgress(
  catalog: Catalog,
  observedIds: Iterable<string>,
): DlcCollectionProgress[] {
  const normalized = new Set<string>();
  for (const raw of observedIds) {
    const id = normalizeObserved(raw);
    if (id !== null) {
      normalized.add(id);
    }
  }
  const totals = new Map<string, { total: number; observed: number }>();
  for (const [id, item] of Object.entries(catalog.items)) {
    if (!item.is_dlc) {
      continue;
    }
    const collection = item.collection ?? "DLC_UNKNOWN";
    const row = totals.get(collection) ?? { total: 0, observed: 0 };
    row.total += 1;
    if (normalized.has(id)) {
      row.observed += 1;
    }
    totals.set(collection, row);
  }
  return [...totals.entries()]
    .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
    .map(([id, row]) => ({
      id,
      label: dlcLabel(id),
      total: row.total,
      observed: row.observed,
    }));
}

/** `DLC01_COLLECT_STORY` → `DLC01`；无法识别时保留原键。 */
function dlcLabel(collection: string): string {
  const match = /^DLC(\d+)/i.exec(collection);
  return match === null ? collection : `DLC${match[1]}`;
}

/** 图鉴口径说明（P2-5）：特殊区域与挑战古墓共用 ID 的静态统计。 */
export interface CatalogScopeNotes {
  specialTotal: number;
  specialWithUnit: number;
  specialWithoutUnit: number;
  challengeTombIds: number;
  challengeTombNames: number;
  sharedChallengeId: string | null;
  sharedChallengeNames: string[];
}

export function catalogScopeNotes(catalog: Catalog): CatalogScopeNotes {
  const special = Object.values(catalog.items).filter(
    (item) => item.region_slug === "_special",
  );
  const challengeTombs = Object.entries(catalog.items).filter(
    ([, item]) => item.category === "challenge_tombs",
  );
  const shared = challengeTombs.find(([, item]) => item.shared_id === true);
  const sharedNames = shared?.[1].shared_names ?? [];
  return {
    specialTotal: special.length,
    specialWithUnit: special.filter((item) => item.unit != null && item.unit !== 0).length,
    specialWithoutUnit: special.filter((item) => item.unit == null || item.unit === 0).length,
    challengeTombIds: challengeTombs.length,
    challengeTombNames: challengeTombs.length - (shared === undefined ? 0 : 1) + sharedNames.length,
    sharedChallengeId: shared?.[0] ?? null,
    sharedChallengeNames: sharedNames,
  };
}
