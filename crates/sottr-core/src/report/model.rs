//! 报告模型：观测、评估、定位、证据与诊断。
//!
//! 所有对象都显式列出全部字段（可空字段用 `Option` 并用 `null` 往返）；
//! `deny_unknown_fields` 保证契约外字段在反序列化时被拒绝；
//! `ProgressValue`、`RegionCounter`、`AssessedItem`、`LocationHint` 的跨字段规则
//! 在反序列化时校验（`try_from`），构造代码可调用各自的 `validate`。

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::enums::{
    AssessmentKind, ContextSource, DiagnosticScope, EvidenceLevel, GroupActionKind, InventoryTable,
    LocationPrecision, MappingKind, Mode, ObservationKind, Severity, SupportStatus,
};
use super::ids::{InternalId, SchemaVersion};

/// 契约校验错误（跨字段规则）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError {
    message: String,
}

impl ContractError {
    /// 带说明的校验错误。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 错误说明。
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ContractError {}

/// 顶层分析报告。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// 契约版本，固定 1.0.0。
    pub schema_version: SchemaVersion,
    /// 工具版本。
    pub tool_version: String,
    /// 解析器 profile 标识。
    pub parser_profile_id: String,
    /// 元数据版本；未知为 null（原因见 diagnostics）。
    pub metadata_version: Option<String>,
    /// 参考集合版本；未知为 null（原因见 diagnostics）。
    pub reference_set_version: Option<String>,
    /// 输入摘要（压缩/解压哈希与尺寸）；不含个人绝对路径。
    pub input_digest: InputDigest,
    /// 上下文（平台、模式、构建、DLC、来源与限制）。
    pub context: Context,
    /// 能力矩阵。
    pub capabilities: Capabilities,
    /// 覆盖范围（支持与参考类别、计数键、不可用原因）。
    pub coverage: Coverage,
    /// 观测层。
    pub observations: Observations,
    /// 评估层（逐项）。
    pub assessments: Vec<AssessedItem>,
    /// 定位提示。
    pub locations: Vec<LocationHint>,
    /// 字段级证据。
    pub evidence: Vec<Evidence>,
    /// 诊断。
    pub diagnostics: Vec<Diagnostic>,
}

/// 输入摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputDigest {
    /// 压缩文件 SHA-256（小写十六进制）。
    pub compressed_sha256: String,
    /// 解压结果 SHA-256；未解压为 null。
    pub expanded_sha256: Option<String>,
    /// 压缩字节数。
    pub compressed_bytes: u64,
    /// 解压字节数；未解压为 null。
    pub expanded_bytes: Option<u64>,
}

/// 上下文；用户声明是上下文证据，不是格式识别证明。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    /// 平台标识（如 windows_steam）；未知为 null。
    pub platform: Option<String>,
    /// 用途模式。
    pub mode: Mode,
    /// 构建号；未知为 null，不虚构。
    pub build: Option<String>,
    /// 已知 DLC 列表；未知为 null。
    pub dlc: Option<Vec<String>>,
    /// 上下文来源。
    pub source: ContextSource,
    /// 适用限制。
    pub limitations: Vec<String>,
}

/// 单项能力。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    /// 支持状态。
    pub status: SupportStatus,
    /// 原因码列表。
    pub reason_codes: Vec<String>,
}

/// 能力矩阵。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// zlib 容器。
    pub container: Capability,
    /// 主区段流。
    pub sections: Capability,
    /// 物品表。
    pub inventory: Capability,
    /// 区域计数。
    pub counters: Capability,
    /// 参考差集。
    pub reference_difference: Capability,
    /// 定位。
    pub location: Capability,
}

/// 覆盖范围。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// 已支持类别 ID。
    pub supported_category_ids: Vec<String>,
    /// 参考集合类别 ID。
    pub reference_category_ids: Vec<String>,
    /// 已支持区域计数键。
    pub supported_region_counter_keys: Vec<String>,
    /// 不可用原因码。
    pub unavailable_reason_codes: Vec<String>,
    /// 覆盖声明对应的证据引用。
    pub evidence_refs: Vec<String>,
}

/// zlib 容器观测。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerObservation {
    /// 支持状态。
    pub status: SupportStatus,
    /// 容器格式。
    pub format: String,
    /// 首流是否完整有效。
    pub valid: bool,
    /// 是否到达流 EOF。
    pub eof: bool,
    /// 压缩字节数。
    pub compressed_bytes: u64,
    /// 解压字节数；未解压为 null。
    pub expanded_bytes: Option<u64>,
    /// 尾随字节数（不解释）。
    pub trailing_bytes: u64,
    /// 错误码或错误说明；无错误为 null。
    pub error: Option<String>,
}

/// 主区段流观测。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionsObservation {
    /// 支持状态。
    pub status: SupportStatus,
    /// 流是否精确走到声明终点。
    pub valid: bool,
    /// 声明字节长度。
    pub byte_length: u64,
    /// 声明长度的十六进制显示。
    pub byte_length_hex: String,
    /// 流起始偏移。
    pub start_offset: u64,
    /// 流起始偏移的十六进制显示。
    pub start_offset_hex: String,
    /// 区段数量。
    pub section_count: u64,
    /// 错误说明；无错误为 null。
    pub error: Option<String>,
}

/// 进度类值对象；未知值 value 为 null 且 reason 必填，合法零保留 0。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProgressValueRaw")]
pub struct ProgressValue {
    /// 数值；未知为 null。
    pub value: Option<f64>,
    /// 单位（如 percent）。
    pub unit: String,
    /// 支持状态。
    pub support_status: SupportStatus,
    /// 未知/降级原因；成功时为 null。
    pub reason: Option<String>,
    /// 证据来源等级。
    pub evidence_level: EvidenceLevel,
    /// 证据引用。
    pub evidence_refs: Vec<String>,
    /// 适用限制。
    pub limitations: Vec<String>,
}

impl ProgressValue {
    /// 契约规则：value 为 null 时 reason 必填且非空。
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.value.is_none() && self.reason.as_deref().map(str::is_empty).unwrap_or(true) {
            return Err(ContractError::new(
                "ProgressValue: value 为 null 时 reason 必填",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgressValueRaw {
    value: Option<f64>,
    unit: String,
    support_status: SupportStatus,
    reason: Option<String>,
    evidence_level: EvidenceLevel,
    evidence_refs: Vec<String>,
    limitations: Vec<String>,
}

impl TryFrom<ProgressValueRaw> for ProgressValue {
    type Error = ContractError;

    fn try_from(raw: ProgressValueRaw) -> Result<Self, Self::Error> {
        let value = Self {
            value: raw.value,
            unit: raw.unit,
            support_status: raw.support_status,
            reason: raw.reason,
            evidence_level: raw.evidence_level,
            evidence_refs: raw.evidence_refs,
            limitations: raw.limitations,
        };
        value.validate()?;
        Ok(value)
    }
}

/// 物品表记录；重复 ID 保留全部原始记录，未知非零 ID 不丢弃。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryRecord {
    /// 记录来源表。
    pub table: InventoryTable,
    /// 槽序号。
    pub index: u32,
    /// 内部 ID。
    pub internal_id: InternalId,
    /// 原始伴随 u32。
    pub raw_companion_u32: u32,
    /// 原始伴随字的十六进制显示。
    pub raw_companion_hex: String,
    /// 伴随字最低两位观察值（不赋状态语义）。
    pub raw_low_two_bits: u8,
    /// 记录偏移。
    pub id_offset: u64,
    /// 记录偏移的十六进制显示。
    pub id_offset_hex: String,
}

/// 物品表观测。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryObservation {
    /// 支持状态。
    pub status: SupportStatus,
    /// 仅指当前 parser profile 下表恢复完整，不代表全游戏全集完备。
    pub inventory_complete: bool,
    /// 完整性判定范围说明。
    pub completeness_scope: String,
    /// 全部原始记录。
    pub records: Vec<InventoryRecord>,
    /// 记录总数。
    pub record_count: u64,
    /// 唯一 ID 数。
    pub unique_id_count: u64,
    /// 重复记录数。
    pub duplicate_record_count: u64,
}

/// 区域计数观测；超过来源目标时保留 raw_value，value 置 null 并附原因。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RegionCounterRaw")]
pub struct RegionCounter {
    /// 稳定计数键。
    pub counter_key: String,
    /// 区域 ID；未知为 null。
    pub region_id: Option<String>,
    /// 类别 ID。
    pub category_id: String,
    /// 候选计数；不可用为 null。
    pub value: Option<u32>,
    /// 单位（如 count）。
    pub unit: String,
    /// 支持状态。
    pub support_status: SupportStatus,
    /// 降级原因；正常为 null。
    pub reason: Option<String>,
    /// 证据来源等级。
    pub evidence_level: EvidenceLevel,
    /// 证据引用。
    pub evidence_refs: Vec<String>,
    /// 适用限制。
    pub limitations: Vec<String>,
    /// 来源目标；未知为 null。
    pub source_target: Option<u32>,
    /// 原始读数是否在来源目标以内。
    pub within_source_target: bool,
    /// 原始 u32 读数。
    pub raw_value: u32,
    /// 原始读数的十六进制显示。
    pub raw_value_hex: String,
}

impl RegionCounter {
    /// 契约规则：value 为 null 时 reason 必填且非空。
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.value.is_none() && self.reason.as_deref().map(str::is_empty).unwrap_or(true) {
            return Err(ContractError::new(
                "RegionCounter: value 为 null 时 reason 必填",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegionCounterRaw {
    counter_key: String,
    region_id: Option<String>,
    category_id: String,
    value: Option<u32>,
    unit: String,
    support_status: SupportStatus,
    reason: Option<String>,
    evidence_level: EvidenceLevel,
    evidence_refs: Vec<String>,
    limitations: Vec<String>,
    source_target: Option<u32>,
    within_source_target: bool,
    raw_value: u32,
    raw_value_hex: String,
}

impl TryFrom<RegionCounterRaw> for RegionCounter {
    type Error = ContractError;

    fn try_from(raw: RegionCounterRaw) -> Result<Self, Self::Error> {
        let value = Self {
            counter_key: raw.counter_key,
            region_id: raw.region_id,
            category_id: raw.category_id,
            value: raw.value,
            unit: raw.unit,
            support_status: raw.support_status,
            reason: raw.reason,
            evidence_level: raw.evidence_level,
            evidence_refs: raw.evidence_refs,
            limitations: raw.limitations,
            source_target: raw.source_target,
            within_source_target: raw.within_source_target,
            raw_value: raw.raw_value,
            raw_value_hex: raw.raw_value_hex,
        };
        value.validate()?;
        Ok(value)
    }
}

/// 观测层。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observations {
    /// zlib 容器。
    pub container: ContainerObservation,
    /// 主区段流。
    pub sections: SectionsObservation,
    /// 整体进度候选。
    pub overall_completion: ProgressValue,
    /// 物品表。
    pub inventory: InventoryObservation,
    /// 区域计数（稀疏，不补齐为完整矩阵）。
    pub region_counters: Vec<RegionCounter>,
}

/// 逐项评估；likely_*/verified_* 必须附 rule_id。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AssessedItemRaw")]
pub struct AssessedItem {
    /// 内部 ID。
    pub internal_id: InternalId,
    /// 观测状态。
    pub observation: ObservationKind,
    /// 评估状态。
    pub assessment: AssessmentKind,
    /// 规则 ID；assessment 为 unknown 时可 null。
    pub rule_id: Option<String>,
    /// 证据引用。
    pub evidence_refs: Vec<String>,
    /// 适用限制。
    pub limitations: Vec<String>,
}

impl AssessedItem {
    /// 契约规则：likely_*/verified_* 必须附非空 rule_id。
    pub fn validate(&self) -> Result<(), ContractError> {
        let needs_rule = matches!(
            self.assessment,
            AssessmentKind::LikelyCollected
                | AssessmentKind::LikelyMissing
                | AssessmentKind::VerifiedCollected
                | AssessmentKind::VerifiedUncollected
        );
        if needs_rule && self.rule_id.as_deref().map(str::is_empty).unwrap_or(true) {
            return Err(ContractError::new(
                "AssessedItem: likely_*/verified_* 必须附 rule_id",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssessedItemRaw {
    internal_id: InternalId,
    observation: ObservationKind,
    assessment: AssessmentKind,
    rule_id: Option<String>,
    evidence_refs: Vec<String>,
    limitations: Vec<String>,
}

impl TryFrom<AssessedItemRaw> for AssessedItem {
    type Error = ContractError;

    fn try_from(raw: AssessedItemRaw) -> Result<Self, Self::Error> {
        let value = Self {
            internal_id: raw.internal_id,
            observation: raw.observation,
            assessment: raw.assessment,
            rule_id: raw.rule_id,
            evidence_refs: raw.evidence_refs,
            limitations: raw.limitations,
        };
        value.validate()?;
        Ok(value)
    }
}

/// 坐标（验证后才可进入契约；无验证映射时为 null）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coordinates {
    /// X 分量。
    pub x: f64,
    /// Y 分量。
    pub y: f64,
    /// Z 分量。
    pub z: f64,
    /// 坐标系。
    pub coordinate_space: String,
    /// 单位。
    pub unit: String,
    /// 关卡标识。
    pub level_id: String,
}

/// 地图变换（底图版本、变换参数、控制点来源与误差描述）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapTransform {
    /// 底图版本。
    pub basemap_version: String,
    /// 变换参数（序列化形式待 SX-020 定义）。
    pub transform_parameters: String,
    /// 控制点来源。
    pub control_points_source: String,
    /// 误差描述。
    pub error_description: String,
}

/// 组级行动门禁；条件不再满足时必须重新降级。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupActionability {
    /// 门禁结果。
    pub kind: GroupActionKind,
    /// 原因码。
    pub reason_codes: Vec<String>,
    /// 证据引用。
    pub evidence_refs: Vec<String>,
    /// 评估所绑定的输入摘要（与当前快照一致）。
    pub evaluated_input_digest: InputDigest,
}

/// 定位提示；成员关系与精确条目映射分开，无验证坐标时为 null。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LocationHintRaw")]
pub struct LocationHint {
    /// 稳定定位 ID。
    pub location_id: String,
    /// 精度等级。
    pub precision: LocationPrecision,
    /// 区域 ID；未知为 null。
    pub region_id: Option<String>,
    /// 攻略组 ID；不属于组时为 null。
    pub guide_group_id: Option<String>,
    /// 精确攻略条目；未建立时为 null。
    pub guide_item: Option<String>,
    /// 组成员内部 ID。
    pub member_ids: Vec<InternalId>,
    /// 组目标数；未知为 null。
    pub target_count: Option<u32>,
    /// 映射种类。
    pub mapping_kind: MappingKind,
    /// 来源引用。
    pub source_refs: Vec<String>,
    /// 适用参考集合版本；未知为 null。
    pub reference_set_version: Option<String>,
    /// 适用模式；未限定为 null。
    pub applicable_mode: Option<Mode>,
    /// 适用平台；未限定为 null。
    pub applicable_platform: Option<String>,
    /// 已验证坐标；无验证映射时为 null。
    pub coordinates: Option<Coordinates>,
    /// 地图变换；无验证变换时为 null。
    pub map_transform: Option<MapTransform>,
    /// 组级行动门禁；非组精度为 null。
    pub group_actionability: Option<GroupActionability>,
    /// 适用限制。
    pub limitations: Vec<String>,
    /// 未知定位的说明；正常为 null。
    pub reason: Option<String>,
}

impl LocationHint {
    /// 契约规则：precision 为 verified_coordinates 时必须附 coordinates。
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.precision == LocationPrecision::VerifiedCoordinates && self.coordinates.is_none() {
            return Err(ContractError::new(
                "LocationHint: verified_coordinates 精度必须附 coordinates",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationHintRaw {
    location_id: String,
    precision: LocationPrecision,
    region_id: Option<String>,
    guide_group_id: Option<String>,
    guide_item: Option<String>,
    member_ids: Vec<InternalId>,
    target_count: Option<u32>,
    mapping_kind: MappingKind,
    source_refs: Vec<String>,
    reference_set_version: Option<String>,
    applicable_mode: Option<Mode>,
    applicable_platform: Option<String>,
    coordinates: Option<Coordinates>,
    map_transform: Option<MapTransform>,
    group_actionability: Option<GroupActionability>,
    limitations: Vec<String>,
    reason: Option<String>,
}

impl TryFrom<LocationHintRaw> for LocationHint {
    type Error = ContractError;

    fn try_from(raw: LocationHintRaw) -> Result<Self, Self::Error> {
        let value = Self {
            location_id: raw.location_id,
            precision: raw.precision,
            region_id: raw.region_id,
            guide_group_id: raw.guide_group_id,
            guide_item: raw.guide_item,
            member_ids: raw.member_ids,
            target_count: raw.target_count,
            mapping_kind: raw.mapping_kind,
            source_refs: raw.source_refs,
            reference_set_version: raw.reference_set_version,
            applicable_mode: raw.applicable_mode,
            applicable_platform: raw.applicable_platform,
            coordinates: raw.coordinates,
            map_transform: raw.map_transform,
            group_actionability: raw.group_actionability,
            limitations: raw.limitations,
            reason: raw.reason,
        };
        value.validate()?;
        Ok(value)
    }
}

/// 字段级证据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// 稳定证据 ID。
    pub evidence_id: String,
    /// 方法。
    pub method: String,
    /// 来源引用。
    pub source_refs: Vec<String>,
    /// 样本范围。
    pub sample_scope: String,
    /// 适用限制。
    pub limitations: Vec<String>,
    /// 字节偏移；无偏移为 null。
    pub offset: Option<u64>,
    /// 偏移的十六进制显示；无偏移为 null。
    pub offset_hex: Option<String>,
    /// 记录序号；无记录为 null。
    pub record_index: Option<u32>,
    /// 参考集合 ID；非参考差集证据为 null。
    pub reference_set_id: Option<String>,
    /// 交叉检查说明。
    pub cross_checks: Vec<String>,
    /// 来源版本表（来源标识 → 固定提交/文档版本）。
    pub source_versions: BTreeMap<String, String>,
}

/// 诊断。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// 原因/错误码。
    pub code: String,
    /// 严重级别。
    pub severity: Severity,
    /// 作用域。
    pub scope: DiagnosticScope,
    /// 说明。
    pub message: String,
    /// 证据引用。
    pub evidence_refs: Vec<String>,
}
