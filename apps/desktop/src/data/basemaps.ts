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
