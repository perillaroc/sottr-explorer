//! 版本化元数据：类型、加载校验、合并与领域视图转换（SX-008）。
//!
//! 边界与约束（开发方案 SX-008 与 SX-003 元数据契约）：
//! - 元数据是独立版本化的本地 JSON 数据集；本模块只处理不可变字节，不访问文件系统；
//! - 拒绝重复 JSON 键、非标准数（`NaN`/`Infinity`）、超大文件/数组/字符串、越界 ID、
//!   非法链接协议与冲突版本；缺目录或坏数据**不产生硬编码兜底推定**；
//! - 名称、状态和位置分开；中文名未知为 `null`，不把内部名片段伪装游戏名称；
//! - 定位只保存区域/攻略组线索；精确坐标映射为 0 时不产生假点；
//! - 组级行动门禁（`group_actionability`）由领域层结合当前报告评估
//!   （[`crate::domain::location::evaluate_group_gate`]），元数据只记录
//!   成员、计数键与目标数，不写死静态 true。
//!
//! 数据集文件布局（仓库 `data/metadata/`，三个文件作为一组加载）：
//! - `collectibles.json`：`items` 与 `categories`；
//! - `region_counters.json`：`regions` 与 43 项稀疏计数映射；
//! - `reference_sets.json`：`reference_sets` 与 `guide_groups`。
//!
//! 每个文件都包含相同的 `sources` 表；合并时相同条目去重，冲突条目报错
//! （[`CODE_ENTRY_CONFLICT`]）。映射键与条目自身 ID 必须一致，引用必须可解析，
//! 否则报 [`CODE_KEY_MISMATCH`] / [`CODE_UNRESOLVED_REFERENCE`]。
//!
//! 缺失的元数据（空目录/空输入）返回 [`MetadataBundle::empty`]，所有视图为空：
//! 计数规格为空 → 能力矩阵按 `COUNTER_MAPPING_UNAVAILABLE` 降级，参考视图不存在
//! → 不产生参考差额结论；这里没有任何内置字典或偏移兜底。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::domain::assessment::{ReferenceCategoryView, ReferenceView};
use crate::domain::progress::{REGION_COUNTER_BASE, REGION_COUNTER_STRIDE, RegionCounterSpec};
use crate::hash::sha256_hex;
use crate::report::SchemaVersion;
use crate::report::{
    Context, Coordinates, InternalId, MapTransform, MappingKind, Mode, RegionCounter,
};
use crate::sections::CAMPAIGN_FAMILY_EXPANDED_BYTES;

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

/// 适用范围；未知信息显式 `null` 并写入 notes。
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

/// 类别；`name_kind` 标明来源本地化名称或人工简名。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// 类别 ID。
    pub category_id: String,
    /// 显示名称；未知为 null。
    pub display_name: Option<String>,
    /// 名称种类。
    pub name_kind: NameKind,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 区域；`name_kind` 标明来源本地化名称或人工简名。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// 区域 ID。
    pub region_id: String,
    /// 显示名称；未知为 null。
    pub display_name: Option<String>,
    /// 名称种类。
    pub name_kind: NameKind,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 攻略组；稳定组 ID 关联成员、来源与整组判断证据。
///
/// `counter_keys` 指向区域计数映射；`target_count` 必须与这些计数来源目标之和一致
/// 才可能给出 `whole_group_check`（由领域层门禁评估，见 SX-008 界面交付）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuideGroup {
    /// 稳定组 ID。
    pub guide_group_id: String,
    /// 显示名称；未知为 null。
    pub display_name: Option<String>,
    /// 名称种类。
    pub name_kind: NameKind,
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
    /// 支撑本组的参考集合；未关联为 null。
    pub reference_set_id: Option<String>,
    /// 整组计数证据对应的区域计数键。
    pub counter_keys: Vec<String>,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 参考集合；含逐类别参考 ID 与适用范围。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSet {
    /// 参考集合 ID。
    pub reference_set_id: String,
    /// 版本。
    pub version: String,
    /// 类别范围。
    pub category_ids: Vec<String>,
    /// 逐类别参考 ID（仅使用两份参考槽位共同且相等的集合）。
    pub reference_ids: BTreeMap<String, Vec<InternalId>>,
    /// 适用范围。
    pub applicability: Applicability,
    /// 来源存档哈希（小写十六进制，压缩文件）。
    pub source_save_hashes: Vec<String>,
    /// 去重规则说明。
    pub dedup_rule: String,
    /// 来源引用。
    pub source_refs: Vec<String>,
}

/// 区域计数映射条目；由研究转录的稀疏映射，缺失即无映射（不是 0）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionCounterEntry {
    /// 稳定计数键（如 `counter:mission_of_san_juan:survival_caches`）。
    pub counter_key: String,
    /// 区域 ID。
    pub region_id: String,
    /// 计数类别 ID（社区计数口径，如 survival_caches）。
    pub category_id: String,
    /// 序列化偏移（小端 u32 读数位置）。
    pub serialized_offset: u64,
    /// 序列化行号。
    pub serialized_row_index: u32,
    /// 行内相对偏移。
    pub serialized_relative_offset: u32,
    /// 社区来源目标值。
    pub source_total: u32,
    /// 该计数对应的参考类别（合并口径）；无对应为空数组。
    pub counts_toward: Vec<String>,
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

/// 元数据数据集（单文件解析形态）。
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
    /// 区域计数映射表。
    pub region_counters: BTreeMap<String, RegionCounterEntry>,
    /// 来源表。
    pub sources: BTreeMap<String, Source>,
}

// ---------------------------------------------------------------------------
// 错误与限制
// ---------------------------------------------------------------------------

/// 输入过大（超过 `MetadataLimits::max_file_bytes`）。
pub const CODE_TOO_LARGE: &str = "METADATA_TOO_LARGE";
/// JSON 语法错误（无法解析）。
pub const CODE_INVALID_JSON: &str = "METADATA_INVALID_JSON";
/// 非标准 JSON 数字（`NaN` / `Infinity` / `-Infinity`）。
pub const CODE_NON_STANDARD_NUMBER: &str = "METADATA_NON_STANDARD_NUMBER";
/// 对象中出现重复键。
pub const CODE_DUPLICATE_KEY: &str = "METADATA_DUPLICATE_KEY";
/// 超过字符串/数组/对象条目/嵌套深度限制。
pub const CODE_LIMIT_EXCEEDED: &str = "METADATA_LIMIT_EXCEEDED";
/// 字段值不满足类型或契约规则（含越界 ID）。
pub const CODE_INVALID_VALUE: &str = "METADATA_INVALID_VALUE";
/// 来源链接不是 http/https。
pub const CODE_INVALID_URL: &str = "METADATA_INVALID_URL";
/// 映射键与条目自身 ID 不一致。
pub const CODE_KEY_MISMATCH: &str = "METADATA_KEY_MISMATCH";
/// 计数映射偏移与行号/相对偏移/家族尺寸不一致。
pub const CODE_COUNTER_MAPPING_INVALID: &str = "METADATA_COUNTER_MAPPING_INVALID";
/// 引用的来源、类别、区域、组、参考集合或计数键不存在。
pub const CODE_UNRESOLVED_REFERENCE: &str = "METADATA_UNRESOLVED_REFERENCE";
/// 多文件之间 schema/metadata 版本或命名空间冲突。
pub const CODE_VERSION_CONFLICT: &str = "METADATA_VERSION_CONFLICT";
/// 多文件之间同名条目内容冲突。
pub const CODE_ENTRY_CONFLICT: &str = "METADATA_ENTRY_CONFLICT";

/// 元数据加载/校验错误；`code` 为稳定错误码（见 `docs/contracts.md`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataError {
    code: &'static str,
    message: String,
}

impl MetadataError {
    /// 构造错误。
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 稳定错误码。
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// 错误说明（含字段或键名，用于诊断；UI 不解析文案做业务判断）。
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for MetadataError {}

/// 元数据加载限制（默认值按当前数据集体量设定；可按调用方评审调整）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetadataLimits {
    /// 单文件最大字节数。
    pub max_file_bytes: usize,
    /// 单个字符串（含键）最大字节数。
    pub max_string_bytes: usize,
    /// 单个数组最大元素数。
    pub max_array_items: usize,
    /// 单个对象最大条目数。
    pub max_object_entries: usize,
    /// 最大嵌套深度。
    pub max_nesting_depth: usize,
}

impl Default for MetadataLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 4 * 1024 * 1024,
            max_string_bytes: 4096,
            max_array_items: 8192,
            max_object_entries: 8192,
            max_nesting_depth: 64,
        }
    }
}

// ---------------------------------------------------------------------------
// 严格 JSON 解析（重复键、非标准数、限制）
// ---------------------------------------------------------------------------

enum StrictNode {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<StrictNode>),
    Object(Vec<(String, StrictNode)>),
}

impl<'de> Deserialize<'de> for StrictNode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NodeVisitor;

        impl<'de> Visitor<'de> for NodeVisitor {
            type Value = StrictNode;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictNode::Bool(value))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictNode::Number(value.into()))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictNode::Number(value.into()))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                if !value.is_finite() {
                    return Err(E::custom("non-standard number"));
                }
                serde_json::Number::from_f64(value)
                    .map(StrictNode::Number)
                    .ok_or_else(|| E::custom("non-standard number"))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictNode::String(value.to_string()))
            }

            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(StrictNode::String(value))
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictNode::Null)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictNode::Null)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<StrictNode>()? {
                    items.push(item);
                }
                Ok(StrictNode::Array(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries: Vec<(String, StrictNode)> = Vec::new();
                let mut seen: BTreeSet<String> = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(de::Error::custom(format!("duplicate key:{key}")));
                    }
                    let value = map.next_value::<StrictNode>()?;
                    entries.push((key, value));
                }
                Ok(StrictNode::Object(entries))
            }
        }

        deserializer.deserialize_any(NodeVisitor)
    }
}

fn to_json_value(node: StrictNode) -> serde_json::Value {
    match node {
        StrictNode::Null => serde_json::Value::Null,
        StrictNode::Bool(value) => serde_json::Value::Bool(value),
        StrictNode::Number(value) => serde_json::Value::Number(value),
        StrictNode::String(value) => serde_json::Value::String(value),
        StrictNode::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(to_json_value).collect())
        }
        StrictNode::Object(entries) => serde_json::Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, to_json_value(value)))
                .collect(),
        ),
    }
}

fn validate_limits(
    node: &StrictNode,
    limits: &MetadataLimits,
    depth: usize,
) -> Result<(), MetadataError> {
    if depth > limits.max_nesting_depth {
        return Err(MetadataError::new(
            CODE_LIMIT_EXCEEDED,
            format!("嵌套深度超过 {}", limits.max_nesting_depth),
        ));
    }
    match node {
        StrictNode::String(value) if value.len() > limits.max_string_bytes => {
            Err(MetadataError::new(
                CODE_LIMIT_EXCEEDED,
                format!(
                    "字符串长度 {} 超过 {}",
                    value.len(),
                    limits.max_string_bytes
                ),
            ))
        }
        StrictNode::Array(items) => {
            if items.len() > limits.max_array_items {
                return Err(MetadataError::new(
                    CODE_LIMIT_EXCEEDED,
                    format!("数组元素数 {} 超过 {}", items.len(), limits.max_array_items),
                ));
            }
            for item in items {
                validate_limits(item, limits, depth + 1)?;
            }
            Ok(())
        }
        StrictNode::Object(entries) => {
            if entries.len() > limits.max_object_entries {
                return Err(MetadataError::new(
                    CODE_LIMIT_EXCEEDED,
                    format!(
                        "对象条目数 {} 超过 {}",
                        entries.len(),
                        limits.max_object_entries
                    ),
                ));
            }
            for (key, value) in entries {
                if key.len() > limits.max_string_bytes {
                    return Err(MetadataError::new(
                        CODE_LIMIT_EXCEEDED,
                        format!("键长度 {} 超过 {}", key.len(), limits.max_string_bytes),
                    ));
                }
                validate_limits(value, limits, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn classify_parse_error(error: &serde_json::Error, bytes: &[u8]) -> &'static str {
    let text = error.to_string();
    if text.contains("duplicate key:") {
        return CODE_DUPLICATE_KEY;
    }
    if text.contains("non-standard number") {
        return CODE_NON_STANDARD_NUMBER;
    }
    // serde_json 对 NaN/Infinity 报语法错误；用原始字节做定位提示
    // （仅在解析已失败时分类，不代表这些片段一定在字符串外）。
    if bytes.windows(3).any(|window| window == b"NaN")
        || bytes.windows(8).any(|window| window == b"Infinity")
    {
        return CODE_NON_STANDARD_NUMBER;
    }
    CODE_INVALID_JSON
}

/// 解析单个元数据文件（严格 JSON：无重复键、无非标准数、受限制约束）。
pub fn load_dataset(
    bytes: &[u8],
    limits: &MetadataLimits,
) -> Result<LoadedMetadataFile, MetadataError> {
    if bytes.len() > limits.max_file_bytes {
        return Err(MetadataError::new(
            CODE_TOO_LARGE,
            format!("文件 {} 字节超过 {}", bytes.len(), limits.max_file_bytes),
        ));
    }
    let node: StrictNode = serde_json::from_slice(bytes).map_err(|error| {
        MetadataError::new(classify_parse_error(&error, bytes), error.to_string())
    })?;
    validate_limits(&node, limits, 0)?;
    let dataset: MetadataDataset = serde_json::from_value(to_json_value(node))
        .map_err(|error| MetadataError::new(CODE_INVALID_VALUE, error.to_string()))?;
    validate_dataset_shape(&dataset)?;
    Ok(LoadedMetadataFile {
        sha256_hex: sha256_hex(bytes),
        dataset,
    })
}

/// 单文件解析结果。
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedMetadataFile {
    /// 文件内容 SHA-256（小写十六进制）。
    pub sha256_hex: String,
    /// 解析后的数据集。
    pub dataset: MetadataDataset,
}

fn validate_dataset_shape(dataset: &MetadataDataset) -> Result<(), MetadataError> {
    if dataset.metadata_version.is_empty() {
        return Err(MetadataError::new(
            CODE_INVALID_VALUE,
            "metadata_version 不能为空",
        ));
    }
    if dataset.namespace.is_empty() {
        return Err(MetadataError::new(CODE_INVALID_VALUE, "namespace 不能为空"));
    }
    for (key, item) in &dataset.items {
        if key != &item.internal_id.to_string() {
            return Err(MetadataError::new(
                CODE_KEY_MISMATCH,
                format!("items 键 {key} 与 internal_id {} 不一致", item.internal_id),
            ));
        }
    }
    for (key, category) in &dataset.categories {
        if key != &category.category_id {
            return Err(MetadataError::new(
                CODE_KEY_MISMATCH,
                format!("categories 键 {key} 与 category_id 不一致"),
            ));
        }
    }
    for (key, region) in &dataset.regions {
        if key != &region.region_id {
            return Err(MetadataError::new(
                CODE_KEY_MISMATCH,
                format!("regions 键 {key} 与 region_id 不一致"),
            ));
        }
    }
    for (key, group) in &dataset.guide_groups {
        if key != &group.guide_group_id {
            return Err(MetadataError::new(
                CODE_KEY_MISMATCH,
                format!("guide_groups 键 {key} 与 guide_group_id 不一致"),
            ));
        }
    }
    for (key, reference_set) in &dataset.reference_sets {
        if key != &reference_set.reference_set_id {
            return Err(MetadataError::new(
                CODE_KEY_MISMATCH,
                format!("reference_sets 键 {key} 与 reference_set_id 不一致"),
            ));
        }
    }
    for (key, entry) in &dataset.region_counters {
        if key != &entry.counter_key {
            return Err(MetadataError::new(
                CODE_KEY_MISMATCH,
                format!("region_counters 键 {key} 与 counter_key 不一致"),
            ));
        }
    }
    for source in dataset.sources.values() {
        validate_source_url(source.url.as_deref())?;
    }
    for entry in dataset.region_counters.values() {
        if entry.source_total == 0 {
            return Err(MetadataError::new(
                CODE_COUNTER_MAPPING_INVALID,
                format!("{} 的来源目标为 0", entry.counter_key),
            ));
        }
        let expected = REGION_COUNTER_BASE as u64
            + u64::from(entry.serialized_row_index) * REGION_COUNTER_STRIDE as u64
            + u64::from(entry.serialized_relative_offset);
        if expected != entry.serialized_offset {
            return Err(MetadataError::new(
                CODE_COUNTER_MAPPING_INVALID,
                format!(
                    "{} 的偏移 {} 与行号/相对偏移推导值 {expected} 不一致",
                    entry.counter_key, entry.serialized_offset
                ),
            ));
        }
        if entry.serialized_offset + 4 > CAMPAIGN_FAMILY_EXPANDED_BYTES as u64 {
            return Err(MetadataError::new(
                CODE_COUNTER_MAPPING_INVALID,
                format!(
                    "{} 的偏移 {} 超出战役家族尺寸",
                    entry.counter_key, entry.serialized_offset
                ),
            ));
        }
    }
    Ok(())
}

fn validate_source_url(url: Option<&str>) -> Result<(), MetadataError> {
    let Some(url) = url else {
        return Ok(());
    };
    let scheme_ok = url
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || url
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
    if !scheme_ok {
        return Err(MetadataError::new(
            CODE_INVALID_URL,
            format!("来源链接仅允许 http/https：{url}"),
        ));
    }
    let remainder = &url[url
        .find("://")
        .expect("scheme checked above")
        .saturating_add(3)..];
    let host = remainder.split(['/', '?', '#']).next().unwrap_or_default();
    if host.is_empty() {
        return Err(MetadataError::new(
            CODE_INVALID_URL,
            format!("来源链接缺少主机名：{url}"),
        ));
    }
    if url.len() > 2048 || url.chars().any(|character| character.is_whitespace()) {
        return Err(MetadataError::new(
            CODE_INVALID_URL,
            format!("来源链接含空白或超长：{url}"),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 多文件合并（MetadataBundle）
// ---------------------------------------------------------------------------

/// 加载输入：文件标识（文件名或逻辑名）与字节。
pub struct MetadataFileInput<'a> {
    /// 文件标识（用于诊断与内容哈希清单；可为文件名）。
    pub file_id: String,
    /// 文件字节。
    pub bytes: &'a [u8],
}

/// 已加载文件记录（内容哈希与版本）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataFileRecord {
    /// 文件标识。
    pub file_id: String,
    /// 内容 SHA-256（小写十六进制）。
    pub sha256_hex: String,
    /// 字节数。
    pub bytes: u64,
    /// 元数据版本。
    pub metadata_version: String,
}

/// 区域计数作用域合计（供参考差额交叉检查的组装层使用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterScopeTotal {
    /// 参考类别集合（合并口径，已排序去重）。
    pub category_ids: Vec<String>,
    /// 作用域内来源目标之和。
    pub source_target_total: u64,
    /// 作用域内读出值之和；任一计数缺失或不可用为 None。
    pub observed_total: Option<u64>,
    /// 是否出现超出来源目标的原始读数。
    pub degraded: bool,
    /// 缺少读数的计数键。
    pub missing_counter_keys: Vec<String>,
}

/// 合并后的元数据包（三个数据文件作为一组）。
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataBundle {
    metadata_version: Option<String>,
    namespace: Option<String>,
    files: Vec<MetadataFileRecord>,
    dataset: Option<MetadataDataset>,
}

impl MetadataBundle {
    /// 空元数据包：所有视图为空，不产生任何兜底推定。
    pub fn empty() -> Self {
        Self {
            metadata_version: None,
            namespace: None,
            files: Vec::new(),
            dataset: None,
        }
    }

    /// 加载并合并多个元数据文件；空输入等价于 [`MetadataBundle::empty`]。
    pub fn from_files(
        inputs: &[MetadataFileInput<'_>],
        limits: &MetadataLimits,
    ) -> Result<Self, MetadataError> {
        if inputs.is_empty() {
            return Ok(Self::empty());
        }

        let mut metadata_version: Option<String> = None;
        let mut namespace: Option<String> = None;
        let mut files = Vec::new();
        let mut merged = MetadataDataset {
            schema_version: SchemaVersion,
            metadata_version: String::new(),
            namespace: String::new(),
            items: BTreeMap::new(),
            categories: BTreeMap::new(),
            regions: BTreeMap::new(),
            guide_groups: BTreeMap::new(),
            reference_sets: BTreeMap::new(),
            region_counters: BTreeMap::new(),
            sources: BTreeMap::new(),
        };

        for input in inputs {
            let loaded = load_dataset(input.bytes, limits)?;
            match &metadata_version {
                None => metadata_version = Some(loaded.dataset.metadata_version.clone()),
                Some(expected) if expected != &loaded.dataset.metadata_version => {
                    return Err(MetadataError::new(
                        CODE_VERSION_CONFLICT,
                        format!(
                            "文件 {} 的 metadata_version {} 与已有 {} 冲突",
                            input.file_id, loaded.dataset.metadata_version, expected
                        ),
                    ));
                }
                _ => {}
            }
            match &namespace {
                None => namespace = Some(loaded.dataset.namespace.clone()),
                Some(expected) if expected != &loaded.dataset.namespace => {
                    return Err(MetadataError::new(
                        CODE_VERSION_CONFLICT,
                        format!(
                            "文件 {} 的 namespace {} 与已有 {} 冲突",
                            input.file_id, loaded.dataset.namespace, expected
                        ),
                    ));
                }
                _ => {}
            }

            merge_map(
                &input.file_id,
                "items",
                &mut merged.items,
                loaded.dataset.items,
            )?;
            merge_map(
                &input.file_id,
                "categories",
                &mut merged.categories,
                loaded.dataset.categories,
            )?;
            merge_map(
                &input.file_id,
                "regions",
                &mut merged.regions,
                loaded.dataset.regions,
            )?;
            merge_map(
                &input.file_id,
                "guide_groups",
                &mut merged.guide_groups,
                loaded.dataset.guide_groups,
            )?;
            merge_map(
                &input.file_id,
                "reference_sets",
                &mut merged.reference_sets,
                loaded.dataset.reference_sets,
            )?;
            merge_map(
                &input.file_id,
                "region_counters",
                &mut merged.region_counters,
                loaded.dataset.region_counters,
            )?;
            merge_map(
                &input.file_id,
                "sources",
                &mut merged.sources,
                loaded.dataset.sources,
            )?;

            files.push(MetadataFileRecord {
                file_id: input.file_id.clone(),
                sha256_hex: loaded.sha256_hex,
                bytes: input.bytes.len() as u64,
                metadata_version: loaded.dataset.metadata_version,
            });
        }

        merged.metadata_version = metadata_version.clone().unwrap_or_default();
        merged.namespace = namespace.clone().unwrap_or_default();
        validate_bundle(&merged)?;

        Ok(Self {
            metadata_version,
            namespace,
            files,
            dataset: Some(merged),
        })
    }

    /// 元数据版本；无文件为 `None`。
    pub fn metadata_version(&self) -> Option<&str> {
        self.metadata_version.as_deref()
    }

    /// 命名空间；无文件为 `None`。
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// 文件记录（含内容哈希），按加载顺序。
    pub fn files(&self) -> &[MetadataFileRecord] {
        &self.files
    }

    /// 合并数据集；空包为 `None`。
    pub fn dataset(&self) -> Option<&MetadataDataset> {
        self.dataset.as_ref()
    }

    /// 按内部 ID 查询条目。
    pub fn item(&self, id: InternalId) -> Option<&MetadataItem> {
        self.dataset.as_ref()?.items.get(&id.to_string())
    }

    /// 按组 ID 查询攻略组。
    pub fn guide_group(&self, guide_group_id: &str) -> Option<&GuideGroup> {
        self.dataset.as_ref()?.guide_groups.get(guide_group_id)
    }

    /// 按 ID 查询参考集合。
    pub fn reference_set(&self, reference_set_id: &str) -> Option<&ReferenceSet> {
        self.dataset.as_ref()?.reference_sets.get(reference_set_id)
    }

    /// 区域计数映射规格（按偏移、键排序）；空包为空列表。
    pub fn region_counter_specs(&self) -> Vec<RegionCounterSpec> {
        let Some(dataset) = &self.dataset else {
            return Vec::new();
        };
        let mut specs: Vec<RegionCounterSpec> = dataset
            .region_counters
            .values()
            .map(|entry| RegionCounterSpec {
                counter_key: entry.counter_key.clone(),
                region_id: Some(entry.region_id.clone()),
                category_id: entry.category_id.clone(),
                serialized_offset: entry.serialized_offset,
                source_total: entry.source_total,
            })
            .collect();
        specs.sort_by(|left, right| {
            left.serialized_offset
                .cmp(&right.serialized_offset)
                .then_with(|| left.counter_key.cmp(&right.counter_key))
        });
        specs
    }

    /// 构造参考集合视图（适用范围按当前上下文计算）；无该集合为 `None`。
    pub fn reference_view(
        &self,
        reference_set_id: &str,
        context: &Context,
    ) -> Option<ReferenceView> {
        let reference_set = self
            .dataset
            .as_ref()?
            .reference_sets
            .get(reference_set_id)?;
        let (applicable, applicability_limits) =
            evaluate_reference_applicability(&reference_set.applicability, context);
        let categories = reference_set
            .category_ids
            .iter()
            .map(|category_id| ReferenceCategoryView {
                category_id: category_id.clone(),
                reference_ids: reference_set
                    .reference_ids
                    .get(category_id)
                    .map(|ids| ids.iter().copied().collect())
                    .unwrap_or_default(),
            })
            .collect();
        Some(ReferenceView {
            reference_set_id: reference_set.reference_set_id.clone(),
            version: reference_set.version.clone(),
            applicable,
            applicability_limits,
            categories,
        })
    }

    /// 按元数据类别分类观察 ID（未分类 ID 不入结果）；身份只由内部 ID 决定。
    pub fn classify_ids(&self, ids: &[InternalId]) -> BTreeMap<String, BTreeSet<InternalId>> {
        let mut classified: BTreeMap<String, BTreeSet<InternalId>> = BTreeMap::new();
        let Some(dataset) = &self.dataset else {
            return classified;
        };
        for id in ids {
            let Some(item) = dataset.items.get(&id.to_string()) else {
                continue;
            };
            let Some(category_id) = &item.category_id else {
                continue;
            };
            classified
                .entry(category_id.clone())
                .or_default()
                .insert(*id);
        }
        classified
    }

    /// 按参考类别作用域汇总区域计数（供组装层做交叉检查；不替代领域评估）。
    pub fn counter_scope_totals(&self, counters: &[RegionCounter]) -> Vec<CounterScopeTotal> {
        let Some(dataset) = &self.dataset else {
            return Vec::new();
        };
        let mut grouped: BTreeMap<Vec<String>, Vec<&RegionCounterEntry>> = BTreeMap::new();
        for entry in dataset.region_counters.values() {
            let mut scope = entry.counts_toward.clone();
            scope.sort();
            scope.dedup();
            grouped.entry(scope).or_default().push(entry);
        }
        let observations: BTreeMap<&str, &RegionCounter> = counters
            .iter()
            .map(|counter| (counter.counter_key.as_str(), counter))
            .collect();

        let mut totals = Vec::new();
        for (category_ids, entries) in grouped {
            let mut source_target_total = 0u64;
            let mut observed_sum = 0u64;
            let mut observed_available = true;
            let mut degraded = false;
            let mut missing_counter_keys = Vec::new();
            for entry in entries {
                source_target_total += u64::from(entry.source_total);
                match observations.get(entry.counter_key.as_str()) {
                    Some(counter) => {
                        if !counter.within_source_target {
                            degraded = true;
                        }
                        match counter.value {
                            Some(value) => observed_sum += u64::from(value),
                            None => observed_available = false,
                        }
                    }
                    None => {
                        missing_counter_keys.push(entry.counter_key.clone());
                        observed_available = false;
                    }
                }
            }
            missing_counter_keys.sort();
            totals.push(CounterScopeTotal {
                category_ids,
                source_target_total,
                observed_total: observed_available.then_some(observed_sum),
                degraded,
                missing_counter_keys,
            });
        }
        totals
    }
}

fn merge_map<T: PartialEq>(
    file_id: &str,
    section: &str,
    target: &mut BTreeMap<String, T>,
    incoming: BTreeMap<String, T>,
) -> Result<(), MetadataError> {
    for (key, value) in incoming {
        match target.get(&key) {
            None => {
                target.insert(key, value);
            }
            Some(existing) if existing == &value => {}
            Some(_) => {
                return Err(MetadataError::new(
                    CODE_ENTRY_CONFLICT,
                    format!("文件 {file_id} 的 {section}.{key} 与已有定义冲突"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_bundle(dataset: &MetadataDataset) -> Result<(), MetadataError> {
    let categories: BTreeSet<&str> = dataset.categories.keys().map(String::as_str).collect();
    let regions: BTreeSet<&str> = dataset.regions.keys().map(String::as_str).collect();
    let sources: BTreeSet<&str> = dataset.sources.keys().map(String::as_str).collect();
    let counter_keys: BTreeSet<&str> = dataset.region_counters.keys().map(String::as_str).collect();
    let reference_categories: BTreeSet<&str> = dataset
        .reference_sets
        .values()
        .flat_map(|reference_set| reference_set.category_ids.iter().map(String::as_str))
        .collect();

    let require_source = |source_ref: &str, owner: &str| -> Result<(), MetadataError> {
        if sources.contains(source_ref) {
            Ok(())
        } else {
            Err(MetadataError::new(
                CODE_UNRESOLVED_REFERENCE,
                format!("{owner} 引用的来源 {source_ref} 不存在"),
            ))
        }
    };
    let require_sources = |refs: &[String], owner: &str| -> Result<(), MetadataError> {
        for source_ref in refs {
            require_source(source_ref, owner)?;
        }
        Ok(())
    };

    for (key, item) in &dataset.items {
        let owner = format!("items.{key}");
        if let Some(category_id) = &item.category_id
            && !categories.contains(category_id.as_str())
        {
            return Err(MetadataError::new(
                CODE_UNRESOLVED_REFERENCE,
                format!("{owner} 的类别 {category_id} 不存在"),
            ));
        }
        if let Some(region_id) = &item.region_id
            && !regions.contains(region_id.as_str())
        {
            return Err(MetadataError::new(
                CODE_UNRESOLVED_REFERENCE,
                format!("{owner} 的区域 {region_id} 不存在"),
            ));
        }
        require_sources(&item.source_refs, &owner)?;
        if let Some(name_source) = &item.name_source
            && name_source.starts_with("src:")
        {
            require_source(name_source, &owner)?;
        }
        for mapping in &item.location_mappings {
            if let Some(region_id) = &mapping.region_id
                && !regions.contains(region_id.as_str())
            {
                return Err(MetadataError::new(
                    CODE_UNRESOLVED_REFERENCE,
                    format!("{owner} 的定位区域 {region_id} 不存在"),
                ));
            }
            if let Some(group_id) = &mapping.guide_group_id
                && !dataset.guide_groups.contains_key(group_id)
            {
                return Err(MetadataError::new(
                    CODE_UNRESOLVED_REFERENCE,
                    format!("{owner} 的攻略组 {group_id} 不存在"),
                ));
            }
            if mapping.mapping_kind == MappingKind::GuideGroupMembership
                && mapping.guide_group_id.is_none()
            {
                return Err(MetadataError::new(
                    CODE_INVALID_VALUE,
                    format!("{owner} 的攻略组成员映射缺少 guide_group_id"),
                ));
            }
            if mapping.mapping_kind == MappingKind::VerifiedCoordinates
                && mapping.coordinates.is_none()
            {
                return Err(MetadataError::new(
                    CODE_INVALID_VALUE,
                    format!("{owner} 的已验证坐标映射缺少 coordinates"),
                ));
            }
            require_sources(&mapping.source_refs, &owner)?;
        }
    }

    for (key, category) in &dataset.categories {
        require_sources(&category.source_refs, &format!("categories.{key}"))?;
    }
    for (key, region) in &dataset.regions {
        require_sources(&region.source_refs, &format!("regions.{key}"))?;
    }

    for (key, reference_set) in &dataset.reference_sets {
        let owner = format!("reference_sets.{key}");
        require_sources(&reference_set.source_refs, &owner)?;
        let declared: BTreeSet<&str> = reference_set
            .category_ids
            .iter()
            .map(String::as_str)
            .collect();
        let provided: BTreeSet<&str> = reference_set
            .reference_ids
            .keys()
            .map(String::as_str)
            .collect();
        if declared != provided {
            return Err(MetadataError::new(
                CODE_INVALID_VALUE,
                format!("{owner} 的 category_ids 与 reference_ids 键不一致"),
            ));
        }
        for category_id in &reference_set.category_ids {
            if !categories.contains(category_id.as_str()) {
                return Err(MetadataError::new(
                    CODE_UNRESOLVED_REFERENCE,
                    format!("{owner} 的类别 {category_id} 不存在"),
                ));
            }
        }
        for (category_id, ids) in &reference_set.reference_ids {
            let mut unique = BTreeSet::new();
            for id in ids {
                if !unique.insert(*id) {
                    return Err(MetadataError::new(
                        CODE_INVALID_VALUE,
                        format!("{owner}.{category_id} 存在重复参考 ID {id}"),
                    ));
                }
            }
        }
        for hash in &reference_set.source_save_hashes {
            if hash.len() != 64
                || !hash
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(MetadataError::new(
                    CODE_INVALID_VALUE,
                    format!("{owner} 的来源存档哈希不是小写十六进制：{hash}"),
                ));
            }
        }
    }

    for (key, group) in &dataset.guide_groups {
        let owner = format!("guide_groups.{key}");
        require_sources(&group.source_refs, &owner)?;
        if let Some(region_id) = &group.region_id
            && !regions.contains(region_id.as_str())
        {
            return Err(MetadataError::new(
                CODE_UNRESOLVED_REFERENCE,
                format!("{owner} 的区域 {region_id} 不存在"),
            ));
        }
        if group.category_ids.is_empty() || group.member_ids.is_empty() {
            return Err(MetadataError::new(
                CODE_INVALID_VALUE,
                format!("{owner} 缺少类别或成员"),
            ));
        }
        let mut unique_members = BTreeSet::new();
        for id in &group.member_ids {
            if !unique_members.insert(*id) {
                return Err(MetadataError::new(
                    CODE_INVALID_VALUE,
                    format!("{owner} 存在重复成员 {id}"),
                ));
            }
        }
        for category_id in &group.category_ids {
            if !categories.contains(category_id.as_str()) {
                return Err(MetadataError::new(
                    CODE_UNRESOLVED_REFERENCE,
                    format!("{owner} 的类别 {category_id} 不存在"),
                ));
            }
        }
        for counter_key in &group.counter_keys {
            if !counter_keys.contains(counter_key.as_str()) {
                return Err(MetadataError::new(
                    CODE_UNRESOLVED_REFERENCE,
                    format!("{owner} 的计数键 {counter_key} 不存在"),
                ));
            }
        }
        if let Some(reference_set_id) = &group.reference_set_id
            && !dataset.reference_sets.contains_key(reference_set_id)
        {
            return Err(MetadataError::new(
                CODE_UNRESOLVED_REFERENCE,
                format!("{owner} 的参考集合 {reference_set_id} 不存在"),
            ));
        }
    }

    for (key, entry) in &dataset.region_counters {
        let owner = format!("region_counters.{key}");
        require_sources(&entry.source_refs, &owner)?;
        if !regions.contains(entry.region_id.as_str()) {
            return Err(MetadataError::new(
                CODE_UNRESOLVED_REFERENCE,
                format!("{owner} 的区域 {} 不存在", entry.region_id),
            ));
        }
        if !categories.contains(entry.category_id.as_str()) {
            return Err(MetadataError::new(
                CODE_UNRESOLVED_REFERENCE,
                format!("{owner} 的类别 {} 不存在", entry.category_id),
            ));
        }
        for category_id in &entry.counts_toward {
            if !reference_categories.contains(category_id.as_str()) {
                return Err(MetadataError::new(
                    CODE_INVALID_VALUE,
                    format!("{owner} 的 counts_toward 类别 {category_id} 不是参考类别"),
                ));
            }
        }
    }

    Ok(())
}

fn evaluate_reference_applicability(
    applicability: &Applicability,
    context: &Context,
) -> (bool, Vec<String>) {
    let mut applicable = true;
    let mut limits = Vec::new();

    if let Some(mode) = applicability.mode
        && mode != context.mode
    {
        applicable = false;
        limits.push(format!(
            "参考集合模式限定为 {mode:?}，当前模式 {:?} 不匹配",
            context.mode
        ));
    }
    match (&applicability.platform, &context.platform) {
        (Some(expected), Some(actual)) if expected != actual => {
            applicable = false;
            limits.push(format!(
                "参考集合平台限定为 {expected}，当前 {actual} 不匹配"
            ));
        }
        (Some(_), None) => {
            limits.push("平台未知：未按平台核对参考集合适用性".to_string());
        }
        _ => {}
    }
    match (&applicability.build_range, &context.build) {
        (Some(range), Some(build)) if range != build => {
            applicable = false;
            limits.push(format!(
                "参考集合构建范围 {range} 与当前构建 {build} 不一致"
            ));
        }
        (Some(range), None) => {
            applicable = false;
            limits.push(format!("参考集合限定构建范围 {range}，当前构建号未知"));
        }
        (None, _) => {
            limits.push("参考集合未限定构建范围；构建号未知或变化时不保证逐版适用".to_string());
        }
        _ => {}
    }
    if applicability.dlc.is_none() {
        limits.push("DLC 适用范围未逐项验证".to_string());
    }
    limits.extend(applicability.notes.iter().cloned());
    (applicable, limits)
}

/// 参考类别作用域与参考 ID 数（数据自检用：43 项计数与参考集合的对应关系）。
pub fn reference_id_count(dataset: &MetadataDataset, category_ids: &[String]) -> usize {
    let scope: BTreeSet<&str> = category_ids.iter().map(String::as_str).collect();
    let mut unique: BTreeSet<InternalId> = BTreeSet::new();
    for reference_set in dataset.reference_sets.values() {
        for (category_id, ids) in &reference_set.reference_ids {
            if scope.contains(category_id.as_str()) {
                unique.extend(ids.iter().copied());
            }
        }
    }
    unique.len()
}
