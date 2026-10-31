// 真实地图基底层（P3-01）：data/map/basemaps.json 的只读索引。
//
// 数据本身由 schemas/basemap.schema.json、`pnpm contract:validate` 与
// `crates/sottr-core/src/basemap.rs` 共同守门；前端不重复校验，只做查找与投影。
//
// 图片是外置资源（D4 授权未决）：只有构建/开发时设置了 BASEMAP_IMAGE_ROOT，
// 才会注入图片基地址；否则 basemapImageUrl 返回 null，界面回退到散点。
import rawBasemaps from "../../../../data/map/basemaps.json";
import type { Basemap, BasemapDataset } from "../contract/basemap";

export type { Basemap } from "../contract/basemap";

const dataset = rawBasemaps as unknown as BasemapDataset;

/** 数据集版本，用于界面上的溯源文案。 */
export const basemapDatasetVersion = dataset.basemap_version;

const byUnit = new Map<number, Basemap>();
for (const entry of Object.values(dataset.basemaps)) {
  for (const unit of entry.unit_ids) {
    byUnit.set(unit, entry);
  }
}

/** 按子关卡 zone id 查基底层记录。 */
export function basemapForUnit(unit: number | null | undefined): Basemap | null {
  if (unit === null || unit === undefined) {
    return null;
  }
  return byUnit.get(unit) ?? null;
}

function minUnit(basemap: Basemap): number {
  return Math.min(...basemap.unit_ids);
}

/** 全部底图记录（按最小 unit 升序；同 unit 时按 basemap_id 稳定排序）。 */
const ordered = Object.values(dataset.basemaps).sort((left, right) => {
  const diff = minUnit(left) - minUnit(right);
  return diff !== 0 ? diff : left.basemap_id.localeCompare(right.basemap_id);
});

/** 数据集的全部底图记录（只读副本；地图查询视图的目录来源）。 */
export function allBasemaps(): Basemap[] {
  return [...ordered];
}

/** 某区域的底图记录（按最小 unit 升序）。 */
export function basemapsForRegion(regionSlug: string): Basemap[] {
  return ordered.filter((entry) => entry.region_id === regionSlug);
}

const imageBase = __BASEMAP_IMAGE_BASE__;

/** 图片基地址；未配置外置素材时为 null。 */
export function basemapImageBase(): string | null {
  return imageBase;
}

/**
 * 图片 URL；未配置外置素材时返回 null。
 *
 * 返回 null 是正常的：界面应回退到散点，而不是显示破图。
 */
export function basemapImageUrl(basemap: Basemap): string | null {
  if (!imageBase) {
    return null;
  }
  return `${imageBase.replace(/\/+$/, "")}/${basemap.image.replace(/^\/+/, "")}`;
}

/** 底图面板的显示判定结果。 */
export interface BasemapMode {
  /** `map` 画底图与点位，`scatter` 回退散点。 */
  mode: "map" | "scatter";
  /** 判定原因，供界面与测试使用。 */
  reason: "map" | "no-record" | "unverified" | "no-image" | "image-error";
}

/**
 * 决定「相对位置」段落画底图还是回退散点（P3-02）。
 *
 * 只有同时满足「有基底层记录」「状态不是 unverified」「图片地址可用」
 * 「图片没有加载失败」才画底图；任一条不满足就回退散点，
 * 回退时沿用原有「未校准」措辞，不暗示任何精度。
 */
export function resolveBasemapMode(
  basemap: Basemap | null,
  imageUrl: string | null,
  imageFailed: boolean,
): BasemapMode {
  if (!basemap) {
    return { mode: "scatter", reason: "no-record" };
  }
  if (basemap.status === "unverified") {
    return { mode: "scatter", reason: "unverified" };
  }
  if (!imageUrl) {
    return { mode: "scatter", reason: "no-image" };
  }
  if (imageFailed) {
    return { mode: "scatter", reason: "image-error" };
  }
  return { mode: "map", reason: "map" };
}

/** 把归一化坐标按变换矩阵映射到图片像素。 */
export function projectToImage(
  basemap: Basemap,
  x: number,
  y: number,
): { u: number; v: number } | null {
  const [a, b, c, d, e, f] = basemap.transform.matrix;
  if ([a, b, c, d, e, f].some((value) => !Number.isFinite(value))) {
    return null;
  }
  return { u: a * x + b * y + c, v: d * x + e * y + f };
}
