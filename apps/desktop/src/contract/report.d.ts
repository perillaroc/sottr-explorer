/* 由 apps/desktop/scripts/contract/generate-types.mjs 从 schemas/report.schema.json 生成，请勿手改。契约说明见 docs/contracts.md。 */

export type NonEmptyString = string;
export type Sha256 = string;
export type Mode = "campaign" | "profile" | "unknown" | "unsupported";
export type ContextSource = "user_declared" | "fixture_provenance" | "unknown";
/**
 * 字段在当前报告上下文中的可用性；unsupported 时 value 必为 null 且 reason 必填
 */
export type SupportStatus = "supported" | "partial" | "unsupported";
export type ReasonCode = string;
export type HexOffset = string;
/**
 * 未知值 value 为 null 且 reason 必填；合法零保留 0
 */
export type ProgressValue = {
  value: number | null;
  unit: NonEmptyString;
  support_status: SupportStatus;
  reason: string | null;
  evidence_level: EvidenceLevel;
  evidence_refs: NonEmptyString[];
  limitations: string[];
};
/**
 * 证据来源等级；禁止无依据概率分数
 */
export type EvidenceLevel = "observed" | "community_correlated" | "game_validated";
export type InventoryTable = "prefix" | "type28_overflow";
/**
 * u32 内部 ID，固定 0x 加 8 位大写十六进制
 */
export type InternalId = string;
/**
 * 计数超过来源目标时保留 raw_value，value 与剩余置 null，不裁剪到目标值
 */
export type RegionCounter = {
  counter_key: NonEmptyString;
  region_id: string | null;
  category_id: NonEmptyString;
  value: number | null;
  unit: NonEmptyString;
  support_status: SupportStatus;
  reason: string | null;
  evidence_level: EvidenceLevel;
  evidence_refs: NonEmptyString[];
  limitations: string[];
  source_target: number | null;
  within_source_target: boolean;
  raw_value: number;
  raw_value_hex: string;
};
/**
 * 身份不等于状态；原始 present 默认仅表示已观察
 */
export type AssessedItem = {
  internal_id: InternalId;
  observation: ObservationKind;
  assessment: AssessmentKind;
  rule_id: string | null;
  evidence_refs: NonEmptyString[];
  limitations: string[];
};
export type ObservationKind = "present" | "reference_present_current_absent" | "unknown";
/**
 * 首版无新游戏内证据时不产生 verified_*；likely_* 仅在对应规则全部满足时出现
 */
export type AssessmentKind =
  "likely_collected" | "likely_missing" | "unknown" | "verified_collected" | "verified_uncollected";
/**
 * 成员关系（member_ids）与精确条目映射（guide_item）分开；无已验证映射时 coordinates 为 null 且不伪造精度
 */
export type LocationHint = {
  location_id: NonEmptyString;
  precision: LocationPrecision;
  region_id: string | null;
  guide_group_id: string | null;
  guide_item: string | null;
  member_ids: InternalId[];
  target_count: number | null;
  mapping_kind: MappingKind;
  source_refs: NonEmptyString[];
  reference_set_version: string | null;
  applicable_mode: "campaign" | "profile" | "unknown" | "unsupported" | null;
  applicable_platform: string | null;
  coordinates: Coordinates | null;
  map_transform: MapTransform | null;
  group_actionability: GroupActionability | null;
  limitations: string[];
  reason: string | null;
};
export type LocationPrecision = "region" | "guide_group" | "guide_item" | "verified_coordinates";
export type MappingKind =
  "region_clue" | "guide_group_membership" | "exact_guide_item" | "verified_coordinates" | "none";
export type GroupActionKind = "whole_group_check" | "general_reference" | "unavailable";
export type Severity = "info" | "warning" | "error";
export type DiagnosticScope =
  "input" | "container" | "sections" | "inventory" | "counters" | "reference_difference" | "location" | "metadata";

/**
 * SOTTR Explorer 分析报告。sottr-explorer 契约 schema_version 1.0.0。业务状态（observation/assessment）、证据来源等级（evidence_level）、上下文（context）与定位精度（location precision）为相互独立字段，禁止合并为单个徽标字段。所有对象均要求显式列出全部字段；可空字段用 null 表达，未知必须附原因或诊断。
 */
export interface Report {
  schema_version: "1.0.0";
  tool_version: NonEmptyString;
  parser_profile_id: NonEmptyString;
  /**
   * 未知为 null；原因写入 diagnostics 的 METADATA_NOT_LOADED 等条目，不虚构版本号
   */
  metadata_version: string | null;
  /**
   * 未知为 null；原因写入 diagnostics，不虚构版本号
   */
  reference_set_version: string | null;
  input_digest: InputDigest;
  context: Context;
  capabilities: Capabilities;
  coverage: Coverage;
  observations: Observations;
  assessments: AssessedItem[];
  locations: LocationHint[];
  evidence: Evidence[];
  diagnostics: Diagnostic[];
}
/**
 * 个人绝对路径不属于契约字段
 */
export interface InputDigest {
  compressed_sha256: Sha256;
  expanded_sha256: string | null;
  compressed_bytes: number;
  expanded_bytes: number | null;
}
/**
 * 用户声明是上下文证据，不是格式识别证明
 */
export interface Context {
  /**
   * 如 windows_steam；未知为 null
   */
  platform: string | null;
  mode: Mode;
  /**
   * 未知为 null，不虚构构建号
   */
  build: string | null;
  dlc: string[] | null;
  source: ContextSource;
  limitations: string[];
}
export interface Capabilities {
  container: Capability;
  sections: Capability;
  inventory: Capability;
  counters: Capability;
  reference_difference: Capability;
  location: Capability;
}
export interface Capability {
  status: SupportStatus;
  reason_codes: ReasonCode[];
}
/**
 * 43 项稀疏计数不通过数组填充变成完整矩阵；汇总计数必须携带 scope 与支持状态
 */
export interface Coverage {
  supported_category_ids: NonEmptyString[];
  reference_category_ids: NonEmptyString[];
  supported_region_counter_keys: NonEmptyString[];
  unavailable_reason_codes: ReasonCode[];
  evidence_refs: NonEmptyString[];
}
export interface Observations {
  container: ContainerObservation;
  sections: SectionsObservation;
  overall_completion: ProgressValue;
  inventory: InventoryObservation;
  region_counters: RegionCounter[];
}
export interface ContainerObservation {
  status: SupportStatus;
  format: NonEmptyString;
  valid: boolean;
  eof: boolean;
  compressed_bytes: number;
  expanded_bytes: number | null;
  trailing_bytes: number;
  error: string | null;
}
export interface SectionsObservation {
  status: SupportStatus;
  valid: boolean;
  byte_length: number;
  byte_length_hex: HexOffset;
  start_offset: number;
  start_offset_hex: HexOffset;
  section_count: number;
  error: string | null;
}
export interface InventoryObservation {
  status: SupportStatus;
  /**
   * 仅指当前 parser profile 下表恢复完整，不代表全游戏全集完备
   */
  inventory_complete: boolean;
  completeness_scope: NonEmptyString;
  records: InventoryRecord[];
  record_count: number;
  unique_id_count: number;
  duplicate_record_count: number;
}
/**
 * 重复 ID 保留全部原始记录；未知非零 ID 不丢弃；伴随字含义未确立，不赋状态语义
 */
export interface InventoryRecord {
  table: InventoryTable;
  index: number;
  internal_id: InternalId;
  raw_companion_u32: number;
  raw_companion_hex: string;
  raw_low_two_bits: number;
  id_offset: number;
  id_offset_hex: HexOffset;
}
export interface Coordinates {
  x: number;
  y: number;
  z: number;
  coordinate_space: NonEmptyString;
  unit: NonEmptyString;
  level_id: NonEmptyString;
}
/**
 * 底图版本、变换参数、控制点来源与误差描述；无验证变换时整体为 null
 */
export interface MapTransform {
  basemap_version: NonEmptyString;
  /**
   * JSON 字符串，内容是 schemas/basemap.schema.json 的 Transform 对象：kind/matrix/coordinate_y_axis/image_origin。定义见 docs/contracts.md「BasemapDataset」一节
   */
  transform_parameters: string;
  control_points_source: NonEmptyString;
  error_description: string;
}
/**
 * whole_group_check 只在完整性、兼容计数、零当前计数、参考范围和组关联全部匹配时给出；非零、未知、冲突或变化时不得延用
 */
export interface GroupActionability {
  kind: GroupActionKind;
  reason_codes: ReasonCode[];
  evidence_refs: NonEmptyString[];
  evaluated_input_digest: InputDigest;
}
export interface Evidence {
  evidence_id: NonEmptyString;
  method: NonEmptyString;
  source_refs: NonEmptyString[];
  sample_scope: NonEmptyString;
  limitations: string[];
  offset: number | null;
  offset_hex: string | null;
  record_index: number | null;
  reference_set_id: string | null;
  cross_checks: string[];
  /**
   * 来源版本表：来源标识到固定提交/文档版本；无则为空对象
   */
  source_versions: {
    [k: string]: string;
  };
}
/**
 * UI 不依赖解析自然语言决定是否允许推定，只看 code/severity/scope
 */
export interface Diagnostic {
  code: ReasonCode;
  severity: Severity;
  scope: DiagnosticScope;
  message: NonEmptyString;
  evidence_refs: NonEmptyString[];
}
