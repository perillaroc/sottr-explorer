//! 契约枚举：业务状态、证据等级、上下文与定位精度相互独立。

use serde::{Deserialize, Serialize};

/// 字段在当前报告上下文中的可用性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    /// 可作为当前展示值。
    Supported,
    /// 可用但适用范围受限，必须带 limitations。
    Partial,
    /// 不适用或无法解释；value 必为 null 且 reason 必填。
    Unsupported,
}

/// 证据来源等级；禁止无依据概率分数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    /// 原始观察（字节、记录、哈希）。
    Observed,
    /// 社区来源与样本交叉支持。
    CommunityCorrelated,
    /// 游戏内或权威格式验证。
    GameValidated,
}

/// 文件用途模式；用户声明是上下文证据，不是格式识别证明。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// 普通战役。
    Campaign,
    /// 资料文件（profile）。
    Profile,
    /// 未知。
    Unknown,
    /// 结构或语义不适用。
    Unsupported,
}

/// 上下文来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    /// 用户声明。
    UserDeclared,
    /// 夹具来源记录。
    FixtureProvenance,
    /// 未知。
    Unknown,
}

/// 单项观测状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    /// 在当前表中观察到。
    Present,
    /// 在参考集合中存在而当前表未观察到。
    ReferencePresentCurrentAbsent,
    /// 未知。
    Unknown,
}

/// 单项评估状态；身份不等于状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentKind {
    /// 推定已收集（需要全部规则条件满足）。
    LikelyCollected,
    /// 推定遗漏（需要全部规则条件满足）。
    LikelyMissing,
    /// 未知。
    Unknown,
    /// 游戏内验证已收集（首版不产生）。
    VerifiedCollected,
    /// 游戏内验证未收集（首版不产生）。
    VerifiedUncollected,
}

/// 诊断严重级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// 信息。
    Info,
    /// 警告。
    Warning,
    /// 错误。
    Error,
}

/// 诊断作用域；UI 不依赖解析自然语言决定是否允许推定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticScope {
    /// 输入。
    Input,
    /// zlib 容器。
    Container,
    /// 主区段流。
    Sections,
    /// 物品表。
    Inventory,
    /// 区域计数。
    Counters,
    /// 参考差集。
    ReferenceDifference,
    /// 定位。
    Location,
    /// 元数据。
    Metadata,
}

/// 物品表来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryTable {
    /// 固定前缀。
    Prefix,
    /// type28 溢出表。
    Type28Overflow,
}

/// 定位精度等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationPrecision {
    /// 区域线索。
    Region,
    /// 攻略物品组。
    GuideGroup,
    /// 精确攻略条目。
    GuideItem,
    /// 已验证坐标。
    VerifiedCoordinates,
}

/// 定位映射种类；成员关系与精确条目映射分开。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingKind {
    /// 区域线索。
    RegionClue,
    /// 攻略组成员关系。
    GuideGroupMembership,
    /// 精确攻略条目映射。
    ExactGuideItem,
    /// 已验证坐标映射。
    VerifiedCoordinates,
    /// 无映射。
    None,
}

/// 组级行动门禁结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupActionKind {
    /// 可按攻略逐项检查整组（条件全部匹配时）。
    WholeGroupCheck,
    /// 仅作全组参考。
    GeneralReference,
    /// 不可用。
    Unavailable,
}

/// 本应用最近分析/刷新状态；不声称与游戏实时同步。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// 当前快照。
    Current,
    /// 正在刷新，展示的是上次结果。
    Refreshing,
    /// 旧快照（刷新失败或输入变化）。
    Stale,
    /// 尚无快照。
    None,
}
