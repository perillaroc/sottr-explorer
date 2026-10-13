//! 版本化报告契约（schema_version 1.0.0）的类型与表示规则。
//!
//! 契约由仓库 `schemas/*.schema.json`、`fixtures/contract/` 与 `docs/contracts.md`
//! 共同冻结；本模块提供与之对应的 Rust 序列化类型。
//!
//! 边界与约束（SX-003）：
//! - 业务状态（observation/assessment）、证据来源等级（evidence_level）、
//!   上下文（context）与定位精度（precision）是相互独立的字段，不合并为单一徽标；
//! - 未知值用 `null` 表达且必附原因；合法零保留 0；
//! - u32 内部 ID 固定输出 `0x` 加 8 位大写十六进制；
//! - u64 资源键以字符串往返，禁止经 JavaScript Number；
//! - 非法状态、`null` 缺原因与未知字段在反序列化时被拒绝。
//!
//! 变更字段含义、枚举或单位必须升级契约版本（见 `docs/contracts.md`）。

mod enums;
mod envelope;
mod ids;
mod model;

pub use enums::{
    AssessmentKind, ContextSource, DiagnosticScope, EvidenceLevel, Freshness, GroupActionKind,
    InventoryTable, LocationPrecision, MappingKind, Mode, ObservationKind, Severity, SupportStatus,
};
pub use envelope::{BatchReport, BatchResult, DesktopResult, ExportDocument, InputError};
pub use ids::{ExportDocumentKind, InternalId, ResourceKey, SchemaVersion};
pub use model::{
    AssessedItem, Capabilities, Capability, ContainerObservation, Context, ContractError,
    Coordinates, Coverage, Diagnostic, Evidence, GroupActionability, InputDigest,
    InventoryObservation, InventoryRecord, LocationHint, MapTransform, Observations, ProgressValue,
    RegionCounter, Report, SectionsObservation,
};

/// 报告契约版本（字符串常量，见 `docs/contracts.md`）。
pub use ids::SCHEMA_VERSION;
