/* 由 apps/desktop/scripts/contract/generate-types.mjs 从 schemas/basemap.schema.json 生成，请勿手改。契约说明见 docs/contracts.md。 */

export type Basemap = {
  /**
   * 必须等于 basemaps 的键（由加载器校验）
   */
  basemap_id: string;
  version: string;
  /**
   * 干净的区域 slug；无对应区域时为 unnamed
   */
  region_id: string;
  /**
   * 子关卡 zone id；每个 unit 全局最多出现一次（由加载器校验）
   *
   * @minItems 1
   */
  unit_ids: [number, ...number[]];
  map_path?: string;
  level_drm?: string;
  source_sheet?: string;
  /**
   * 相对 image_root.path 的图片路径
   */
  image: string;
  image_width: number;
  image_height: number;
  transform: Transform;
  /**
   * analytic = 由游戏数据解析出的确定性映射；measured = 由控制点拟合
   */
  transform_basis: "analytic" | "measured";
  /**
   * calibrated 要求中位误差 ≤ 0.5% 对角线且 P95 ≤ 1%；approximate 要求中位 ≤ 2%
   */
  status: "unverified" | "approximate" | "calibrated";
  median_error_px?: number | null;
  p95_error_px?: number | null;
  max_error_px?: number | null;
  /**
   * measured = 逐单位抽样验证；global_estimate = 使用其它单位的汇总估计
   */
  error_source?: "measured" | "global_estimate";
  validation_samples?: number;
  /**
   * 必须解析到顶层 sources（由加载器校验）
   *
   * @minItems 1
   */
  source_refs: [string, ...string[]];
  image_sha256?: string;
  image_bytes?: number;
  review: Review;
};

/**
 * SOTTR Explorer 真实地图基底层。独立版本化的本地 JSON 数据集：记录每个子关卡（unit）使用的底图、把图鉴归一化坐标映射到图片像素的仿射变换，以及变换的依据、状态与误差。坐标空间是每单位独立的 [0,1] 归一化空间，原点在取景框西北角、y 向下增大；变换首版恒为比例映射。
 */
export interface BasemapDataset {
  schema_version: "1.0.0";
  basemap_version: string;
  /**
   * 生成说明（工具与日期），仅供追溯，不参与逻辑
   */
  generated?: string;
  image_root: ImageRoot;
  /**
   * 面向维护者的人类可读说明
   */
  notes?: string[];
  sources: {
    [k: string]: Source;
  };
  /**
   * 键必须等于条目的 basemap_id（由加载器校验）；同一 unit 只能出现在一条记录里
   */
  basemaps: {
    [k: string]: Basemap;
  };
}
export interface ImageRoot {
  /**
   * external = 图片在仓库外、由用户指向本地目录；bundled = 图片随应用分发
   */
  kind: "external" | "bundled";
  /**
   * image 字段的相对根目录；kind=bundled 时相对仓库 data/map
   */
  path: string;
  note?: string;
}
export interface Source {
  /**
   * 原始来源页；不可考时为 null
   */
  url?: string | null;
  fixed_revision?: string | null;
  accessed?: string | null;
  license: string;
  /**
   * 分发授权状态；非 allowed 的图片不得进入发布包
   */
  redistributable: "pending" | "allowed" | "denied";
  method: string;
  review?: string;
}
export interface Transform {
  kind: "affine";
  /**
   * u = a*x + b*y + c, v = d*x + e*y + f，顺序为 [a,b,c,d,e,f]。首版恒为 [W,0,0,0,H,0]（由加载器校验与 image_width/height 一致）
   *
   * @minItems 6
   * @maxItems 6
   */
  matrix: [number, number, number, number, number, number];
  /**
   * 图鉴坐标的 y 方向；实测为 down（原点在西北角，向南增大）
   */
  coordinate_y_axis: "up" | "down";
  image_origin: "top_left";
}
export interface Review {
  state: "draft" | "reviewed";
  note?: string | null;
  date?: string | null;
}
