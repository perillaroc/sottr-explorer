// 元数据视图（SX-015 模拟数据源）：
// - 合并仓库 `data/metadata/` 三个真实数据集（与桌面后端内嵌的同一组文件，
//   与 CLI `--metadata` 同源），保持区域/类别/攻略组/计数映射/来源的真实形状；
// - 叠加契约夹单元数据中的“有来源的名称”（人工简名），用于演示显示名回退规则
//   （真实数据集当前所有条目的 display_name 均为 null，界面必须走回退文案）。
//
// 该模块只做数据整理，不计算领域推定：门禁结论一律读取报告中的
// group_actionability，界面不凭计数自行下结论。
import collectiblesJson from "../../../../data/metadata/collectibles.json";
import countersJson from "../../../../data/metadata/region_counters.json";
import referenceSetsJson from "../../../../data/metadata/reference_sets.json";
import type { MetadataDataset, MetadataItem, Region } from "../contract/metadata";
import { metadataFixture } from "./fixtures";

function mergeItems(base: MetadataDataset["items"], overlay: MetadataDataset["items"]): MetadataDataset["items"] {
  const merged: MetadataDataset["items"] = { ...base };
  for (const [id, item] of Object.entries(overlay)) {
    const existing = merged[id];
    merged[id] = existing
      ? { ...existing, display_name: item.display_name, name_kind: item.name_kind, display_language: item.display_language }
      : item;
  }
  return merged;
}

function mergeRegions(base: MetadataDataset["regions"], overlay: MetadataDataset["regions"]): MetadataDataset["regions"] {
  const merged: MetadataDataset["regions"] = { ...base };
  for (const [id, region] of Object.entries(overlay)) {
    const existing = merged[id];
    // 真实数据集里的 curated_short 名称优先；仅当基础条目缺失或名称为未知时用夹具补齐。
    if (!existing || !existing.display_name || existing.name_kind === "unknown") {
      merged[id] = existing ? { ...existing, display_name: region.display_name, name_kind: region.name_kind } : region;
    }
  }
  return merged;
}

const collectibles = collectiblesJson as unknown as MetadataDataset;
const counters = countersJson as unknown as MetadataDataset;
const referenceSets = referenceSetsJson as unknown as MetadataDataset;

/** 合并后的元数据集（模拟数据源与界面共享；不可变使用）。 */
export const demoMetadata: MetadataDataset = {
  schema_version: collectibles.schema_version,
  metadata_version: collectibles.metadata_version,
  namespace: collectibles.namespace,
  items: mergeItems(collectibles.items, metadataFixture.items),
  categories: { ...collectibles.categories, ...metadataFixture.categories },
  regions: mergeRegions(counters.regions, metadataFixture.regions),
  guide_groups: { ...referenceSets.guide_groups },
  reference_sets: { ...referenceSets.reference_sets },
  region_counters: { ...counters.region_counters },
  sources: { ...collectibles.sources },
};

/** 界面支持的逐项参考类别（参考差集范围；顺序即显示顺序）。 */
export const REFERENCE_CATEGORY_ORDER = [
  "survival_caches",
  "rare_resource_caches",
  "monoliths",
  "crypt_treasures",
] as const;

/** 区域计数展示单元格：贮藏点合并普通/稀有（保留内部分类，见 `counts_toward`）。 */
export interface CounterCellSpec {
  /** 展示用稳定键（`caches` 合并，其余等于参考类别 ID）。 */
  key: string;
  /** 展示用参考类别（coverage 中的类别 ID）。 */
  categoryId: string;
  /**
   * 计数映射使用的类别（存档字段命名与参考类别不同：
   * 参考类别 `crypt_treasures` 对应计数类别 `crypts`）。
   */
  counterCategoryId: string;
  /** 强制合并展示时的说明。 */
  mergedNote?: string;
}

export const COUNTER_CELL_SPECS: CounterCellSpec[] = [
  {
    key: "caches",
    categoryId: "survival_caches",
    counterCategoryId: "survival_caches",
    mergedNote: "普通与稀有贮藏点合并显示，内部保留两个类别",
  },
  { key: "monoliths", categoryId: "monoliths", counterCategoryId: "monoliths" },
  { key: "crypt_treasures", categoryId: "crypt_treasures", counterCategoryId: "crypts" },
];

/** 键为 `counter:<region>:<category>`。 */
export function counterKey(regionId: string, categoryId: string): string {
  return `counter:${regionId}:${categoryId}`;
}

/** 区域列表（按元数据键排序；用于侧栏与概览）。 */
export function regionsOf(metadata: MetadataDataset): Region[] {
  return Object.values(metadata.regions).sort((a, b) => a.region_id.localeCompare(b.region_id));
}

/** 条目的类别（未知返回 null）。 */
export function categoryOf(metadata: MetadataDataset, id: string): string | null {
  return metadata.items[id]?.category_id ?? null;
}

/** 条目的区域（元数据或位置线索；未知返回 null）。 */
export function regionOf(metadata: MetadataDataset, id: string): string | null {
  return metadata.items[id]?.region_id ?? null;
}

/** 参考集合中的成员判定（按参考类别）。 */
export function isReferenceMember(metadata: MetadataDataset, id: string, categories: string[]): boolean {
  for (const referenceSet of Object.values(metadata.reference_sets)) {
    for (const categoryId of categories) {
      const ids = referenceSet.reference_ids[categoryId];
      if (ids && ids.includes(id)) {
        return true;
      }
    }
  }
  return false;
}

/** 元数据条目（不存在返回 null：界面必须显示 ID 回退）。 */
export function itemOf(metadata: MetadataDataset, id: string): MetadataItem | null {
  return metadata.items[id] ?? null;
}
