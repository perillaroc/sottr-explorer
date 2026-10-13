//! 版本化元数据模型（契约 schema_version 1.0.0）。
//!
//! 元数据是独立版本化的本地 JSON 数据集：描述内部 ID、别名、类别、区域、
//! 显示名称、适用条件、来源与定位映射。加载与校验（大小、重复键、链接协议、
//! 键与 internal_id 一致性等）在 SX-008 实现；本模块只冻结类型。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::report::{Coordinates, InternalId, MapTransform, MappingKind, Mode, SchemaVersion};

/// 名称种类；人工简名不是正式游戏名称。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameKind {
    /// 来源本地化名称。
    SourceLocalized,
    /// 人工简名。
    CuratedShort,
    /// 机器翻译。
    MachineTranslated,
    /// 未知。
    Unknown,
}

/// 条目审核状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    /// 草稿。
    Draft,
    /// 已审核。
    Reviewed,
    /// 已撤回。
    Withdrawn,
}

/// 再分发许可状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Redistributable {
    /// 可再分发。
    Yes,
    /// 不可再分发。
    No,
    /// 待处理。
    Pending,
}

/// 适用范围；未知信息显式 null 并写入 notes。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applicability {
    /// 游戏标识（如 sottr）。
    pub game: String,
    /// 平台；未知为 null。
    pub platform: Option<String>,
    /// 模式；未限定为 null。
    pub mode: Option<Mode>,
    /// 构建范围；未知为 null。
    pub build_range: Option<String>,
    /// DLC 列表；未知为 null。
    pub dlc: Option<Vec<String>>,
    /// 备注。
    pub notes: Vec<String>,
}

/// 定位映射；成员关系与精确条目映射分开。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationMapping {
    /// 映射种类。
    pub mapping_kind: MappingKind,
    /// 区域 ID。
    pub region_id: Option<String>,
    /// 攻略组 ID。
    pub guide_group_id: Option<String>,
    /// 精确攻略条目。
    pub guide_item: Option<String>,
    /// 组目标数；未知为 null。
    pub target_count: Option<u32>,
    /// 坐标；无验证映射为 null。
    pub coordinates: Option<Coordinates>,
    /// 地图变换；无验证变换为 null。
    pub map_transform: Option<MapTransform>,
    /// 适用模式。
    pub applicable_mode: Option<Mode>,
    /// 适用平台。
    pub applicable_platform: Option<String>,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 审核记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// 状态。
    pub state: ReviewState,
    /// 备注。
    pub note: Option<String>,
    /// 日期（YYYY-MM-DD）。
    pub date: Option<String>,
}

/// 收集品元数据条目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataItem {
    /// 内部 ID。
    pub internal_id: InternalId,
    /// 全部别名保留。
    pub aliases: Vec<String>,
    /// 显示名称；未知为 null。
    pub display_name: Option<String>,
    /// 显示语言（BCP-47）；未知为 null。
    pub display_language: Option<String>,
    /// 名称来源引用或描述；未知为 null。
    pub name_source: Option<String>,
    /// 名称种类。
    pub name_kind: NameKind,
    /// 类别 ID；未知为 null。
    pub category_id: Option<String>,
    /// 区域 ID；未知为 null。
    pub region_id: Option<String>,
    /// 适用范围。
    pub applicability: Applicability,
    /// 定位映射。
    pub location_mappings: Vec<LocationMapping>,
    /// 来源引用。
    pub source_refs: Vec<String>,
    /// 审核记录；无记录为 null。
    pub review: Option<Review>,
}

/// 类别。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// 类别 ID。
    pub category_id: String,
    /// 显示名称；未知为 null。
    pub display_name: Option<String>,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 区域。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// 区域 ID。
    pub region_id: String,
    /// 显示名称；未知为 null。
    pub display_name: Option<String>,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 攻略组；稳定组 ID 关联成员与来源，不生成 ID 到攻略编号的一对一对应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuideGroup {
    /// 稳定组 ID。
    pub guide_group_id: String,
    /// 显示名称；未知为 null。
    pub display_name: Option<String>,
    /// 区域 ID。
    pub region_id: Option<String>,
    /// 类别 ID 列表。
    pub category_ids: Vec<String>,
    /// 成员内部 ID。
    pub member_ids: Vec<InternalId>,
    /// 组目标数；未知为 null。
    pub target_count: Option<u32>,
    /// 映射种类。
    pub mapping_kind: MappingKind,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 参考集合。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSet {
    /// 参考集合 ID。
    pub reference_set_id: String,
    /// 版本。
    pub version: String,
    /// 类别范围。
    pub category_ids: Vec<String>,
    /// 来源存档哈希（小写十六进制）。
    pub source_save_hashes: Vec<String>,
    /// 去重规则说明。
    pub dedup_rule: String,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 证据来源。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// 来源 URL；仅允许 http/https，无链接为 null。
    pub url: Option<String>,
    /// 固定提交或文档版本。
    pub fixed_revision: Option<String>,
    /// 访问日期（YYYY-MM-DD）。
    pub accessed: Option<String>,
    /// 许可说明。
    pub license: Option<String>,
    /// 再分发状态。
    pub redistributable: Redistributable,
    /// 推导方法。
    pub method: Option<String>,
    /// 审核记录。
    pub review: Option<String>,
}

/// 元数据数据集。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataDataset {
    /// 契约版本。
    pub schema_version: SchemaVersion,
    /// 元数据版本。
    pub metadata_version: String,
    /// 命名空间。
    pub namespace: String,
    /// 条目（键为 0x 加 8 位大写十六进制；与 internal_id 一致由加载器校验）。
    pub items: BTreeMap<String, MetadataItem>,
    /// 类别表。
    pub categories: BTreeMap<String, Category>,
    /// 区域表。
    pub regions: BTreeMap<String, Region>,
    /// 攻略组表。
    pub guide_groups: BTreeMap<String, GuideGroup>,
    /// 参考集合表。
    pub reference_sets: BTreeMap<String, ReferenceSet>,
    /// 来源表。
    pub sources: BTreeMap<String, Source>,
}
