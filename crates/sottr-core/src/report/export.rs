//! 导出（SX-011）：导出信封组装、严格 JSON、CSV 与分享脱敏。
//!
//! 规则（开发方案 SX-011 / 契约“信封”一节）：
//! - JSON 为规范接口：导出文件是 [`ExportDocument`] 信封，自带快照身份、
//!   `freshness` 与最近刷新错误码；普通核心 JSON（批量/报告 schema）与导出信封分开；
//! - CSV 每行保留 ID、全部候选名称、observation、assessment、证据级别、规则与
//!   定位精度，并以明确列保留同样的快照身份与刷新错误码；外部文本（含元数据名称）
//!   以 `=`、`+`、`-`、`@`、制表或回车等危险前缀开头时——含前导空白绕过——
//!   转义为安全文本（前缀 `'`），随后按 RFC 4180 引号规则包裹；
//! - 分享模式：`input_ref` 去目录、全文路径样片段（含相对分隔的 `.dat` 引用）
//!   替换为文件名，保持 schema、业务状态、证据与定位精度不变；分享导出必须通过
//!   [`scan_private_material`] 自检（无路径样命中）；
//! - 严格序列化：拒绝 `NaN`/`Infinity`（信封内浮点必须有限）；ID 保持
//!   `0x`+8 位大写十六进制字符串，u64 资源键只经字符串往返（`ResourceKey`）。
//!
//! 本模块不访问文件系统：文件读取、临时写入与提交由适配层（CLI）承担。

use std::borrow::Cow;

use serde::Serialize;
use serde_json::Value;

use super::enums::{Freshness, LocationPrecision};
use super::envelope::ExportDocument;
use super::ids::{ExportDocumentKind, SchemaVersion};
use super::model::{ContractError, Report};
use crate::domain::assessment::rule_descriptor;
use crate::metadata::MetadataBundle;

/// CSV 列定义（顺序稳定；契约文档固定）。
pub const CSV_COLUMNS: [&str; 15] = [
    "snapshot_id",
    "snapshot_time",
    "freshness",
    "last_refresh_error_code",
    "input_ref",
    "input_sha256",
    "id",
    "candidate_names",
    "observation",
    "assessment",
    "evidence_level",
    "rule_id",
    "rule_description",
    "location_id",
    "location_precision",
];

/// 导出信封的快照身份输入（由适配层提供；核心不取时钟）。
pub struct ExportSnapshot<'a> {
    /// 稳定输入引用（文件名，不含个人绝对路径）。
    pub input_ref: &'a str,
    /// 快照标识（非空）。
    pub snapshot_id: &'a str,
    /// 快照时间（ISO 8601 带时区，非空）。
    pub snapshot_time: &'a str,
    /// 时效状态（不能为 `none`）。
    pub freshness: Freshness,
    /// 最近刷新错误码；无错误为 `None`。
    pub last_refresh_error_code: Option<&'a str>,
}

/// 由报告组装导出信封。
///
/// `input_digest` 从报告复制，导出与报告不可能分叉；快照字段做契约级校验
/// （`freshness` 非 `none`、身份非空、错误码形如 `^[A-Z][A-Z0-9_]*$`）。
pub fn export_document(
    report: Report,
    snapshot: ExportSnapshot<'_>,
) -> Result<ExportDocument, ContractError> {
    if snapshot.freshness == Freshness::None {
        return Err(ContractError::new(
            "ExportDocument: 导出必须携带快照状态（freshness 不能为 none）",
        ));
    }
    if snapshot.input_ref.is_empty() {
        return Err(ContractError::new("ExportDocument: input_ref 不能为空"));
    }
    if snapshot.snapshot_id.is_empty() || snapshot.snapshot_time.is_empty() {
        return Err(ContractError::new("ExportDocument: 必须携带快照身份"));
    }
    if let Some(code) = snapshot.last_refresh_error_code
        && !is_reason_code(code)
    {
        return Err(ContractError::new(format!(
            "ExportDocument: 刷新错误码必须形如 ^[A-Z][A-Z0-9_]*$，收到 {code:?}"
        )));
    }
    let document = ExportDocument {
        schema_version: SchemaVersion,
        document_kind: ExportDocumentKind,
        input_ref: snapshot.input_ref.to_string(),
        snapshot_id: snapshot.snapshot_id.to_string(),
        snapshot_time: snapshot.snapshot_time.to_string(),
        freshness: snapshot.freshness,
        last_refresh_error_code: snapshot.last_refresh_error_code.map(str::to_string),
        input_digest: report.input_digest.clone(),
        report,
    };
    document.validate()?;
    Ok(document)
}

/// 刷新/原因码形态校验（与 schema `^[A-Z][A-Z0-9_]*$` 一致）。
pub fn is_reason_code(code: &str) -> bool {
    let mut bytes = code.bytes();
    match bytes.next() {
        Some(first) if first.is_ascii_uppercase() => {}
        _ => return false,
    }
    bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

/// 渲染严格 JSON：拒绝非有限浮点，缩进输出并追加换行。
pub fn render_export_json(document: &ExportDocument) -> Result<String, ContractError> {
    validate_finite(document)?;
    let mut text = serde_json::to_string_pretty(document)
        .map_err(|error| ContractError::new(format!("导出 JSON 序列化失败：{error}")))?;
    text.push('\n');
    Ok(text)
}

/// 信封内浮点必须有限（`NaN`/`Infinity` 会使 JSON 退化，必须拒绝）。
fn validate_finite(document: &ExportDocument) -> Result<(), ContractError> {
    let report = &document.report;
    if let Some(value) = report.observations.overall_completion.value
        && !value.is_finite()
    {
        return Err(ContractError::new(
            "ExportDocument: overall_completion.value 必须为有限浮点",
        ));
    }
    for location in &report.locations {
        if let Some(coordinates) = &location.coordinates
            && !(coordinates.x.is_finite()
                && coordinates.y.is_finite()
                && coordinates.z.is_finite())
        {
            return Err(ContractError::new("ExportDocument: 坐标分量必须为有限浮点"));
        }
        if let Some(actionability) = &location.group_actionability
            && actionability
                .evaluated_input_digest
                .compressed_sha256
                .is_empty()
        {
            return Err(ContractError::new("ExportDocument: 组门禁摘要不能为空"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

/// 渲染 CSV（RFC 4180：逗号分隔、`\r\n` 行尾、必要时双引号包裹并双写引号）。
///
/// `share` 为真时对候选名称等外部文本做分享脱敏（路径样片段替换为文件名）。
/// 恒不修改 JSON 数字或 ID 语义（ID 输出 `0x`+8 位大写十六进制）。
pub fn render_export_csv(
    document: &ExportDocument,
    metadata: &MetadataBundle,
    share: bool,
) -> String {
    let mut output = String::new();
    output.push_str(&CSV_COLUMNS.join(","));
    output.push_str("\r\n");
    for item in &document.report.assessments {
        let mut names: Vec<String> = metadata
            .item(item.internal_id)
            .map(|entry| entry.aliases.clone())
            .unwrap_or_default();
        if share {
            names = names
                .iter()
                .map(|name| match scrub_path_text(name) {
                    Cow::Borrowed(_) => name.clone(),
                    Cow::Owned(replaced) => replaced,
                })
                .collect();
        }
        let names_json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
        let (rule_id, rule_description, evidence_level) = match item.rule_id.as_deref() {
            Some(rule_id) => {
                let descriptor = rule_descriptor(rule_id);
                (
                    rule_id.to_string(),
                    descriptor
                        .map(|descriptor| descriptor.description.to_string())
                        .unwrap_or_default(),
                    descriptor
                        .map(|descriptor| enum_text(&descriptor.evidence_level))
                        .unwrap_or_default(),
                )
            }
            None => (String::new(), String::new(), String::new()),
        };
        let (location_id, location_precision) = item_location(&document.report, item.internal_id);
        let cells: [String; 15] = [
            document.snapshot_id.clone(),
            document.snapshot_time.clone(),
            enum_text(&document.freshness),
            document.last_refresh_error_code.clone().unwrap_or_default(),
            document.input_ref.clone(),
            document.input_digest.compressed_sha256.clone(),
            item.internal_id.to_string(),
            names_json,
            enum_text(&item.observation),
            enum_text(&item.assessment),
            evidence_level,
            rule_id,
            rule_description,
            location_id,
            location_precision,
        ];
        for (index, cell) in cells.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&csv_escape(cell));
        }
        output.push_str("\r\n");
    }
    output
}

/// 取最精确的定位提示（region < guide_group < guide_item < verified_coordinates）。
fn item_location(report: &Report, id: super::ids::InternalId) -> (String, String) {
    let mut best: Option<(&str, LocationPrecision)> = None;
    for location in &report.locations {
        if !location.member_ids.contains(&id) {
            continue;
        }
        let better = match best {
            None => true,
            Some((_, precision)) => precision_rank(location.precision) > precision_rank(precision),
        };
        if better {
            best = Some((location.location_id.as_str(), location.precision));
        }
    }
    match best {
        Some((location_id, precision)) => (location_id.to_string(), enum_text(&precision)),
        None => (String::new(), String::new()),
    }
}

fn precision_rank(precision: LocationPrecision) -> u8 {
    match precision {
        LocationPrecision::Region => 1,
        LocationPrecision::GuideGroup => 2,
        LocationPrecision::GuideItem => 3,
        LocationPrecision::VerifiedCoordinates => 4,
    }
}

fn enum_text<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(text)) => text,
        Ok(other) => other.to_string(),
        Err(_) => String::new(),
    }
}

/// CSV 单元格：先做危险前缀转义，再按 RFC 4180 引号规则包裹。
pub fn csv_escape(value: &str) -> String {
    let sanitized = sanitize_csv_text(value);
    if sanitized
        .chars()
        .any(|character| matches!(character, '"' | ',' | '\r' | '\n'))
    {
        let mut quoted = String::with_capacity(sanitized.len() + 2);
        quoted.push('"');
        for character in sanitized.chars() {
            if character == '"' {
                quoted.push('"');
            }
            quoted.push(character);
        }
        quoted.push('"');
        quoted
    } else {
        sanitized.into_owned()
    }
}

/// 危险前缀判定：忽略前导空白（空格/制表/回车/换行）后的首字符为
/// `=`、`+`、`-`、`@`，或原值本身以制表/回车/换行开头。
pub fn csv_text_is_unsafe(value: &str) -> bool {
    if matches!(value.chars().next(), Some('\t') | Some('\r') | Some('\n')) {
        return true;
    }
    matches!(
        value
            .trim_start_matches([' ', '\t', '\r', '\n'])
            .chars()
            .next(),
        Some('=') | Some('+') | Some('-') | Some('@')
    )
}

/// 危险前缀转义为安全文本：前缀 `'`（公式保护）。安全文本原样返回。
pub fn sanitize_csv_text(value: &str) -> Cow<'_, str> {
    if csv_text_is_unsafe(value) {
        Cow::Owned(format!("'{value}"))
    } else {
        Cow::Borrowed(value)
    }
}

// ---------------------------------------------------------------------------
// 分享脱敏与自检
// ---------------------------------------------------------------------------

/// 分享脱敏：`input_ref` 去目录；所有字符串字段中的路径样片段
/// （盘符、UNC、`file://`、含分隔符的 `.dat` 引用）替换为路径文件名；
/// 证书/业务值/定位精度保持不变。
pub fn share_export_document(document: &ExportDocument) -> Result<ExportDocument, ContractError> {
    let mut value = serde_json::to_value(document)
        .map_err(|error| ContractError::new(format!("分享文档序列化失败：{error}")))?;
    scrub_value(&mut value);
    if let Some(input_ref) = value.get_mut("input_ref")
        && let Some(text) = input_ref.as_str()
    {
        *input_ref = Value::String(file_basename(text));
    }
    let shared: ExportDocument = serde_json::from_value(value)
        .map_err(|error| ContractError::new(format!("分享文档必须保持契约有效：{error}")))?;
    shared.validate()?;
    Ok(shared)
}

fn scrub_value(value: &mut Value) {
    match value {
        Value::String(text) => {
            if let Cow::Owned(replaced) = scrub_path_text(text) {
                *text = replaced;
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub_value(item);
            }
        }
        Value::Object(entries) => {
            for item in entries.values_mut() {
                scrub_value(item);
            }
        }
        _ => {}
    }
}

/// 扫描文本中的私人路径样片段（分享导出自检；返回命中片段，空表示干净）。
pub fn scan_private_material(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut findings = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if boundary_before(&characters, index) && path_like_at(&characters, index) {
            let mut end = index;
            while end < characters.len() && !is_hard_terminator(characters[end]) {
                end += 1;
            }
            let snippet: String = characters[index..end.min(index + 96)].iter().collect();
            findings.push(snippet);
            index = end;
        } else {
            index += 1;
        }
    }
    findings
}

/// 路径样片段替换：命中片段替换为其中最后一个路径分量的文件名字段；
/// 无文件名时替换为 `<redacted-path>`。
pub fn scrub_path_text(value: &str) -> Cow<'_, str> {
    let characters: Vec<char> = value.chars().collect();
    if !(0..characters.len())
        .any(|index| boundary_before(&characters, index) && path_like_at(&characters, index))
    {
        return Cow::Borrowed(value);
    }
    let mut output = String::with_capacity(value.len());
    let mut index = 0;
    while index < characters.len() {
        if boundary_before(&characters, index) && path_like_at(&characters, index) {
            let mut end = index;
            while end < characters.len() && !is_hard_terminator(characters[end]) {
                end += 1;
            }
            let segment: String = characters[index..end].iter().collect();
            output.push_str(&redacted_replacement(&segment));
            index = end;
        } else {
            output.push(characters[index]);
            index += 1;
        }
    }
    Cow::Owned(output)
}

fn redacted_replacement(segment: &str) -> String {
    let basename = file_basename(segment);
    // 文件名字段：不含空白的前导可打印 ASCII 段（"save1.dat 失败" → "save1.dat"）；
    // 其后紧邻的尾部（消息文本）原样保留，路径目录部分被移除。
    let filename: String = basename
        .chars()
        .take_while(|character| {
            character.is_ascii_graphic() && !matches!(character, '"' | '\'' | '<' | '>' | '|')
        })
        .collect();
    if filename.is_empty() {
        if basename.is_empty() {
            "<redacted-path>".to_string()
        } else {
            basename
        }
    } else {
        let mut replacement = filename;
        replacement.push_str(&basename[replacement.len()..]);
        replacement
    }
}

fn file_basename(text: &str) -> String {
    text.rsplit(['\\', '/'])
        .find(|part| !part.is_empty())
        .unwrap_or_default()
        .to_string()
}

/// 路径样起始判定：盘符（`C:\`/`C:/`）、UNC/网络（`\\`、`//`）、`file://`，
/// 或从边界起含有以分隔符引出的 `.dat` 引用（如 `..\save1.dat`）。
fn path_like_at(characters: &[char], index: usize) -> bool {
    let remaining = &characters[index..];
    if remaining.len() >= 3
        && remaining[0].is_ascii_alphabetic()
        && remaining[1] == ':'
        && matches!(remaining[2], '\\' | '/')
    {
        return true;
    }
    if remaining.len() >= 3
        && matches!(remaining[0], '\\')
        && remaining[1] == '\\'
        && remaining[2] != '\\'
    {
        return true;
    }
    if remaining.len() >= 3 && remaining[0] == '/' && remaining[1] == '/' && remaining[2] != '/' {
        return true;
    }
    if remaining.len() >= 7
        && remaining[..7]
            .iter()
            .collect::<String>()
            .eq_ignore_ascii_case("file://")
    {
        return true;
    }
    // 相对分隔的 `.dat` 引用（如 `..\save1.dat`）：起始到首个分隔符之间不能含
    // 空白（避免把散文吞进替换范围）；`://` URL 不受此规则影响（避免误伤来源链接）。
    let mut cursor = index;
    let mut saw_separator = false;
    while cursor < characters.len() && !is_hard_terminator(characters[cursor]) {
        if characters[cursor] == ':'
            && cursor + 2 < characters.len()
            && characters[cursor + 1] == '/'
            && characters[cursor + 2] == '/'
        {
            return false;
        }
        if matches!(characters[cursor], '\\' | '/') && !saw_separator {
            if characters[index..cursor]
                .iter()
                .any(|character| character.is_whitespace())
            {
                return false;
            }
            saw_separator = true;
        }
        if saw_separator
            && characters[cursor] == '.'
            && cursor + 4 <= characters.len()
            && characters[cursor + 1].eq_ignore_ascii_case(&'d')
            && characters[cursor + 2].eq_ignore_ascii_case(&'a')
            && characters[cursor + 3].eq_ignore_ascii_case(&'t')
            && (cursor + 4 == characters.len() || !characters[cursor + 4].is_ascii_alphanumeric())
        {
            return true;
        }
        cursor += 1;
    }
    false
}

/// 片段边界：字符串开头或前一个字符不是 ASCII 字母数字/下划线。
fn boundary_before(characters: &[char], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    let previous = characters[index - 1];
    !(previous.is_ascii_alphanumeric() || previous == '_')
}

fn is_hard_terminator(character: char) -> bool {
    matches!(
        character,
        '"' | '\'' | '<' | '>' | '|' | '?' | '*' | '\r' | '\n' | '\t'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_quotes_and_escapes_rfc4180() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("中文字段"), "中文字段");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(csv_escape("line1\r\nline2"), "\"line1\r\nline2\"");
    }

    #[test]
    fn csv_escape_prefixes_dangerous_formulas_including_whitespace_bypass() {
        for dangerous in [
            "=SUM(A1:A2)",
            "+cmd",
            "-2+3",
            "@import",
            " =1+1",
            "  =1+1",
            "\t=1+1",
            "\tcmd",
            "\ncmd",
            "\r=cmd",
            "-",
        ] {
            let escaped = csv_escape(dangerous);
            let sanitized = sanitize_csv_text(dangerous);
            assert!(
                sanitized.starts_with('\''),
                "{dangerous:?} 必须以转义标记开头"
            );
            assert!(
                escaped.starts_with('\'') || escaped.starts_with("\"'"),
                "{dangerous:?} 转义结果 {escaped:?}"
            );
        }
        // 安全文本不被改写：ID 与数字语义保持。
        assert_eq!(csv_escape("0x0F28B6EC"), "0x0F28B6EC");
        assert_eq!(csv_escape("18446744073709551615"), "18446744073709551615");
        assert_eq!(csv_escape("66.204819"), "66.204819");
    }

    #[test]
    fn scrub_replaces_absolute_paths_with_basename() {
        assert_eq!(
            scrub_path_text("读取 D:\\Secret\\save1.dat 失败"),
            "读取 save1.dat 失败"
        );
        assert_eq!(
            scrub_path_text("\\\\server\\share\\x.dat 已复制"),
            "x.dat 已复制"
        );
        assert_eq!(
            scrub_path_text("文件 file:///C:/Users/me/save202.dat 存在"),
            "文件 save202.dat 存在"
        );
        assert_eq!(
            scrub_path_text("相对 ..\\..\\saves\\save204.dat 引用"),
            "相对 save204.dat 引用"
        );
        assert_eq!(scrub_path_text("没有路径"), "没有路径");
        // 连字符与普通文本不受影响。
        assert_eq!(
            scrub_path_text("likely-missing 不是路径"),
            "likely-missing 不是路径"
        );
    }

    #[test]
    fn scan_flags_private_material_only_when_present() {
        assert!(scan_private_material("读取 save1.dat 失败").is_empty());
        assert!(!scan_private_material("D:\\Secret\\save1.dat").is_empty());
        assert!(!scan_private_material("\\\\server\\share\\x.dat").is_empty());
        assert!(!scan_private_material("..\\saves\\save1.dat").is_empty());
        assert!(scan_private_material("<redacted-path> 之后").is_empty());
    }

    #[test]
    fn reason_code_shape_matches_schema_pattern() {
        assert!(is_reason_code("INPUT_CHANGED"));
        assert!(is_reason_code("A1_B2"));
        assert!(!is_reason_code("input_changed"));
        assert!(!is_reason_code("1INPUT"));
        assert!(!is_reason_code(""));
        assert!(!is_reason_code("INPUT-CHANGED"));
    }
}
