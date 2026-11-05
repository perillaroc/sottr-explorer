// 必收条目的「下落」索引（v0.2.0）：
// - 地图：有条目坐标，且所属 unit 有底图；
// - 文字：条目带合集 / 子关卡 / 逐项注记。
// 统计只读图鉴与底图数据集，不依赖当前存档状态；用于全收集页的覆盖行与缺失清单标记。
import { productionAtlas, type Atlas } from "../data/atlas";
import { basemapForUnit } from "../data/basemaps";
import { localizedName } from "./i18n";

export type GuidanceScope = "map" | "item" | "collection" | "unit" | "none";

export interface GuidanceCoverage {
  required: number;
  mapped: number;
  item: number;
  collection: number;
  unit: number;
  uncovered: number;
}

/** 全量必收指引覆盖（静态：与当前存档的收集状态无关）。 */
export function guidanceCoverage(atlas: Atlas = productionAtlas): GuidanceCoverage {
  const stats: GuidanceCoverage = {
    required: 0,
    mapped: 0,
    item: 0,
    collection: 0,
    unit: 0,
    uncovered: 0,
  };
  for (const item of Object.values(atlas.items)) {
    if (item.required_for_100 !== true) {
      continue;
    }
    stats.required += 1;
    if (item.position && item.unit !== null && item.unit !== undefined && basemapForUnit(item.unit)) {
      stats.mapped += 1;
      continue;
    }
    if (item.acquisition_note) {
      switch (item.acquisition_note_scope) {
        case "collection":
          stats.collection += 1;
          break;
        case "unit":
          stats.unit += 1;
          break;
        default:
          stats.item += 1;
      }
    } else {
      stats.uncovered += 1;
    }
  }
  return stats;
}

/** 单条必收条目的下落类别；非必收或未知 ID 返回 `none`。 */
export function guidanceScopeForId(id: string, atlas: Atlas = productionAtlas): GuidanceScope {
  const item = atlas.items[id];
  if (!item) {
    return "none";
  }
  if (item.position && item.unit !== null && item.unit !== undefined && basemapForUnit(item.unit)) {
    return "map";
  }
  if (item.acquisition_note) {
    if (item.acquisition_note_scope === "collection") {
      return "collection";
    }
    if (item.acquisition_note_scope === "unit") {
      return "unit";
    }
    return "item";
  }
  return "none";
}

/** 合集显示名：优先图鉴里的中英显示名，回退内部合集名。 */
export function collectionDisplayName(name: string, atlas: Atlas = productionAtlas): string {
  const entry = atlas.collections.find((collection) => collection.name === name);
  return localizedName(entry?.display_zh, entry?.display_en, name);
}
