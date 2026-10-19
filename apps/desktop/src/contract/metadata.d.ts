/* 由 apps/desktop/scripts/contract/generate-types.mjs 从 schemas/metadata.schema.json 生成，请勿手改。契约说明见 docs/contracts.md。 */

/**
 * 来源本地化名称 / 人工简名（不是正式游戏名称）/ 机器翻译 / 未知
 */
export type NameKind = "source_localized" | "curated_short" | "machine_translated" | "unknown";
export type ReviewState = "draft" | "reviewed" | "withdrawn";
export type Redistributable = "yes" | "no" | "pending";

/**
 * SOTTR Explorer 收集品元数据。独立版本化的本地 JSON 数据集：描述内部 ID、别名、类别、区域、显示名称、适用条件、来源、定位映射与稀疏区域计数映射。元数据不执行脚本；中文名未知则 null；不把内部名片段伪装游戏名称；坐标映射为空时不产生假点。
 */
export interface MetadataDataset {
  schema_version: "1.0.0";
  metadata_version: string;
  namespace: string;
  /**
   * 键为内部 ID（0x 加 8 位大写十六进制），与条目的 internal_id 必须一致（由加载器校验）
   */
  items: {
    [k: string]: MetadataItem;
  };
  categories: {
    [k: string]: Category;
  };
  regions: {
    [k: string]: Region;
  };
  guide_groups: {
    [k: string]: GuideGroup;
  };
  reference_sets: {
    [k: string]: ReferenceSet;
  };
  /**
   * 稀疏区域计数映射；缺项不是 0（由加载器校验偏移恒等式）
   */
  region_counters: {
    [k: string]: RegionCounterEntry;
  };
  sources: {
    [k: string]: Source;
  };
}
/**
 * 全部别名保留；display_name 为 null 时前端使用带标识的类别加 ID 回退文案，不自造正式名称
 */
export interface MetadataItem {
  /**
   * u32 内部 ID，固定 0x 加 8 位大写十六进制
   */
  internal_id: string;
  aliases: string[];
  display_name: string | null;
  /**
   * BCP-47，如 zh-Hans；未知为 null
   */
  display_language: string | null;
  /**
   * 名称来源引用或描述；未知为 null
   */
  name_source: string | null;
  name_kind: NameKind;
  category_id: string | null;
  region_id: string | null;
  applicability: Applicability;
  location_mappings: LocationMapping[];
  source_refs: string[];
  review: Review | null;
}
/**
 * 未知信息显式 null 并写入 notes，不猜测
 */
export interface Applicability {
  game: string;
  platform: string | null;
  mode: ("campaign" | "profile" | "unknown" | "unsupported") | null;
  build_range: string | null;
  dlc: string[] | null;
  notes: string[];
}
/**
 * 成员关系与精确条目映射分开；未确认时保留原始研究信息且不标为玩家可导航位置
 */
export interface LocationMapping {
  mapping_kind: "region_clue" | "guide_group_membership" | "exact_guide_item" | "verified_coordinates" | "none";
  region_id: string | null;
  guide_group_id: string | null;
  guide_item: string | null;
  target_count: number | null;
  coordinates: {
    x: number;
    y: number;
    z: number;
    coordinate_space: string;
    unit: string;
    level_id: string;
  } | null;
  map_transform: {
    basemap_version: string;
    transform_parameters: string;
    control_points_source: string;
    error_description: string;
  } | null;
  applicable_mode: ("campaign" | "profile" | "unknown" | "unsupported") | null;
  applicable_platform: string | null;
  source_refs: string[];
}
export interface Review {
  state: ReviewState;
  note: string | null;
  date: string | null;
}
/**
 * name_kind 标明来源本地化名称或人工简名；未知名称不伪装游戏名称
 */
export interface Category {
  category_id: string;
  display_name: string | null;
  name_kind: NameKind;
  source_refs: string[];
}
/**
 * name_kind 标明来源本地化名称或人工简名；中文名未知为 null
 */
export interface Region {
  region_id: string;
  display_name: string | null;
  name_kind: NameKind;
  source_refs: string[];
}
/**
 * 稳定组 ID 关联成员、攻略来源与整组判断计数证据；不生成 ID 到攻略编号的一对一对应；group_actionability 由领域层按当前报告评估
 */
export interface GuideGroup {
  guide_group_id: string;
  display_name: string | null;
  name_kind: NameKind;
  region_id: string | null;
  category_ids: string[];
  /**
   * Items: u32 内部 ID，固定 0x 加 8 位大写十六进制
   */
  member_ids: string[];
  target_count: number | null;
  mapping_kind: "region_clue" | "guide_group_membership" | "exact_guide_item" | "verified_coordinates" | "none";
  /**
   * 支撑本组的参考集合；未关联为 null
   */
  reference_set_id: string | null;
  counter_keys: string[];
  source_refs: string[];
}
export interface ReferenceSet {
  reference_set_id: string;
  version: string;
  category_ids: string[];
  /**
   * 逐类别参考 ID；键集合必须与 category_ids 一致（由加载器校验）
   */
  reference_ids: {
    /**
     * Items: u32 内部 ID，固定 0x 加 8 位大写十六进制
     */
    [k: string]: string[];
  };
  applicability: Applicability;
  source_save_hashes: string[];
  dedup_rule: string;
  source_refs: string[];
}
/**
 * 稀疏计数映射；偏移恒等式与家族尺寸由加载器校验；缺项不是 0
 */
export interface RegionCounterEntry {
  counter_key: string;
  region_id: string;
  category_id: string;
  serialized_offset: number;
  serialized_row_index: number;
  serialized_relative_offset: number;
  source_total: number;
  /**
   * 该计数对应的参考类别（合并口径）；无对应为空数组
   */
  counts_toward: string[];
  source_refs: string[];
}
export interface Source {
  /**
   * 仅允许 http/https；非法协议在加载器中被拒绝
   */
  url: string | null;
  fixed_revision: string | null;
  accessed: string | null;
  license: string | null;
  redistributable: Redistributable;
  method: string | null;
  review: string | null;
  /**
   * 链接载体；URL 非空时必须非空（跨字段规则由 Rust 加载器校验，JSON Schema 不强制）
   */
  medium?: "text" | "video" | null;
  /**
   * 来源用途；仅 role 为 guide 且 URL 非空的来源产生直链候选
   */
  role?: "guide" | "tool" | "save" | "research" | "other" | null;
}
