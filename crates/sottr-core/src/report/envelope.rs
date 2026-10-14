//! 信封类型：批量外层、桌面 IPC 结果与导出文档。
//!
//! 信封与核心 [`Report`] 分离：请求/快照身份与 freshness 不进入报告业务值；
//! 取消和晚到结果由 `request_id` 与当前选定请求比对识别。

use serde::{Deserialize, Serialize};

use super::enums::Freshness;
use super::ids::{ExportDocumentKind, SchemaVersion};
use super::model::{ContractError, InputDigest, Report};

/// 输入级错误；局部失败留在 `Report.diagnostics`，不占用此结构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputError {
    /// 错误码（如 INPUT_UNREADABLE）。
    pub code: String,
    /// 说明。
    pub message: String,
    /// 是否可重试。
    pub retryable: bool,
}

/// 输入不可读（路径、权限或文件类型问题；不尝试提权）。
pub const INPUT_UNREADABLE: &str = "INPUT_UNREADABLE";
/// 读取前后不一致（文件可能在保存）；要求稳定副本或重试。
pub const INPUT_CHANGED: &str = "INPUT_CHANGED";
/// 解压失败（无效 zlib 或不完整流）；禁止语义解释。
pub const DECOMPRESSION_FAILED: &str = "DECOMPRESSION_FAILED";
/// 尺寸或数量超过限制；停止并说明限制。
pub const RESOURCE_LIMIT: &str = "RESOURCE_LIMIT";

impl InputError {
    /// 构造输入级错误。
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }

    /// 路径、权限或文件类型问题（可重试：用户可另选文件或修复权限）。
    pub fn unreadable(message: impl Into<String>) -> Self {
        Self::new(INPUT_UNREADABLE, message, true)
    }

    /// 读取前后不一致（可重试：等待保存完成或选稳定副本）。
    pub fn changed(message: impl Into<String>) -> Self {
        Self::new(INPUT_CHANGED, message, true)
    }

    /// 解压失败（不可重试：同一字节再试不会成功）。
    pub fn decompression_failed(message: impl Into<String>) -> Self {
        Self::new(DECOMPRESSION_FAILED, message, false)
    }

    /// 资源超限（不可重试：同一输入不会变小）。
    pub fn resource_limit(message: impl Into<String>) -> Self {
        Self::new(RESOURCE_LIMIT, message, false)
    }
}

/// 批量分析外层。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchReport {
    /// 契约版本。
    pub schema_version: SchemaVersion,
    /// 每文件结果。
    pub results: Vec<BatchResult>,
}

/// 单文件结果；report 与 error 恰有一个非空。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "BatchResultRaw")]
pub struct BatchResult {
    /// 稳定输入引用（通常为文件名，不含个人绝对路径）。
    pub input_ref: String,
    /// 报告；输入级错误时为 null。
    pub report: Option<Report>,
    /// 输入级错误；成功时为 null。
    pub error: Option<InputError>,
}

impl BatchResult {
    /// 契约规则：report 与 error 恰有一个非空。
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.report.is_some() == self.error.is_some() {
            return Err(ContractError::new(
                "BatchResult: report 与 error 恰有一个非空",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchResultRaw {
    input_ref: String,
    report: Option<Report>,
    error: Option<InputError>,
}

impl TryFrom<BatchResultRaw> for BatchResult {
    type Error = ContractError;

    fn try_from(raw: BatchResultRaw) -> Result<Self, Self::Error> {
        let value = Self {
            input_ref: raw.input_ref,
            report: raw.report,
            error: raw.error,
        };
        value.validate()?;
        Ok(value)
    }
}

/// 桌面 IPC 结果信封；freshness 只说明本应用最近分析/刷新状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DesktopResultRaw")]
pub struct DesktopResult {
    /// 契约版本。
    pub schema_version: SchemaVersion,
    /// 请求标识；晚到结果据此丢弃。
    pub request_id: String,
    /// 输入引用。
    pub input_ref: String,
    /// 快照标识；尚无快照为 null。
    pub snapshot_id: Option<String>,
    /// 快照时间（ISO 8601 带时区）；尚无快照为 null。
    pub snapshot_time: Option<String>,
    /// 时效状态。
    pub freshness: Freshness,
    /// 最近刷新错误；无错误为 null。
    pub last_refresh_error: Option<InputError>,
    /// 报告；无快照或输入错误时为 null。
    pub report: Option<Report>,
    /// 输入级错误；无错误为 null。
    pub error: Option<InputError>,
}

impl DesktopResult {
    /// 契约规则：freshness 非 none 时必须携带快照身份。
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.freshness != Freshness::None
            && (self
                .snapshot_id
                .as_deref()
                .map(str::is_empty)
                .unwrap_or(true)
                || self
                    .snapshot_time
                    .as_deref()
                    .map(str::is_empty)
                    .unwrap_or(true))
        {
            return Err(ContractError::new(
                "DesktopResult: freshness 非 none 时必须携带 snapshot_id/snapshot_time",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DesktopResultRaw {
    schema_version: SchemaVersion,
    request_id: String,
    input_ref: String,
    snapshot_id: Option<String>,
    snapshot_time: Option<String>,
    freshness: Freshness,
    last_refresh_error: Option<InputError>,
    report: Option<Report>,
    error: Option<InputError>,
}

impl TryFrom<DesktopResultRaw> for DesktopResult {
    type Error = ContractError;

    fn try_from(raw: DesktopResultRaw) -> Result<Self, Self::Error> {
        let value = Self {
            schema_version: raw.schema_version,
            request_id: raw.request_id,
            input_ref: raw.input_ref,
            snapshot_id: raw.snapshot_id,
            snapshot_time: raw.snapshot_time,
            freshness: raw.freshness,
            last_refresh_error: raw.last_refresh_error,
            report: raw.report,
            error: raw.error,
        };
        value.validate()?;
        Ok(value)
    }
}

/// 导出文档信封；旧报告导出必须带 stale 标识与最近刷新错误码。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ExportDocumentRaw")]
pub struct ExportDocument {
    /// 契约版本。
    pub schema_version: SchemaVersion,
    /// 固定为 report_export。
    pub document_kind: ExportDocumentKind,
    /// 输入引用。
    pub input_ref: String,
    /// 快照标识。
    pub snapshot_id: String,
    /// 快照时间（ISO 8601 带时区）。
    pub snapshot_time: String,
    /// 时效状态（current/refreshing/stale；无快照不导出）。
    pub freshness: Freshness,
    /// 最近刷新错误码；无错误为 null。
    pub last_refresh_error_code: Option<String>,
    /// 输入摘要（与屏幕报告一致）。
    pub input_digest: InputDigest,
    /// 报告。
    pub report: Report,
}

impl ExportDocument {
    /// 契约规则：导出必须携带快照身份且 freshness 不是 none。
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.freshness == Freshness::None {
            return Err(ContractError::new("ExportDocument: freshness 不能为 none"));
        }
        if self.snapshot_id.is_empty() || self.snapshot_time.is_empty() {
            return Err(ContractError::new("ExportDocument: 必须携带快照身份"));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportDocumentRaw {
    schema_version: SchemaVersion,
    document_kind: ExportDocumentKind,
    input_ref: String,
    snapshot_id: String,
    snapshot_time: String,
    freshness: Freshness,
    last_refresh_error_code: Option<String>,
    input_digest: InputDigest,
    report: Report,
}

impl TryFrom<ExportDocumentRaw> for ExportDocument {
    type Error = ContractError;

    fn try_from(raw: ExportDocumentRaw) -> Result<Self, Self::Error> {
        let value = Self {
            schema_version: raw.schema_version,
            document_kind: raw.document_kind,
            input_ref: raw.input_ref,
            snapshot_id: raw.snapshot_id,
            snapshot_time: raw.snapshot_time,
            freshness: raw.freshness,
            last_refresh_error_code: raw.last_refresh_error_code,
            input_digest: raw.input_digest,
            report: raw.report,
        };
        value.validate()?;
        Ok(value)
    }
}
