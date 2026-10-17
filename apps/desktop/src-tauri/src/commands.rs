//! Tauri 最小权限命令层（SX-014）。
//!
//! 仅暴露界面完成“选择输入 → 列候选 → 分析 → 取消 → 导出 → 打开核验链接”
//! 所需命令；每条命令都复用 `sottr-core`（同核心分析）与 `sottr-platform`
//! （路径策略、稳定快照、受控输出提交），不因界面需要放宽核心或路径规则。
//!
//! IPC 约定：
//!
//! - 分析返回 SX-003 契约的 [`DesktopResult`]：`request_id` 与快照身份随信封
//!   返回；取消的请求返回“无结果信封”（`freshness = none`，`report`/`error`
//!   均为 `null`），失败刷新返回携带旧快照的 `stale` 信封；
//! - 其余命令返回结构化 JSON 或稳定错误对象 `{ code, message }`（界面不解析
//!   自然语言）；命令参数按 Tauri 约定使用 camelCase（如 `requestId`）；
//! - 攻略链接只接受稳定 ID（快照/定位/来源），URL 由后端从元数据解析并校验
//!   http/https 后交给系统外部浏览器；WebView 不加载外部页面（见 crate 根）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::State;

use sottr_core::analyze::analyze_bytes;
use sottr_core::domain::context::DeclaredContext;
use sottr_core::limits::Limits;
use sottr_core::metadata::MetadataBundle;
use sottr_core::report::{
    ContextSource, DesktopResult, ExportSnapshot, Freshness, InputError, Mode, SchemaVersion,
    export_document, render_export_csv, render_export_json, scan_private_material,
    share_export_document,
};
use sottr_platform::{discovery, output_commit, path_policy, read_snapshot, time};

use crate::session::{
    CommitOutcome, DesktopState, FailureOutcome, GuideLink, Session, StoredSnapshot,
    new_snapshot_id,
};

// ---------------------------------------------------------------------------
// 稳定错误码
// ---------------------------------------------------------------------------

/// 参数非法（缺失、越界、未知枚举）；命令在产生任何副作用前拒绝。
pub const CODE_INVALID_PARAMS: &str = "INVALID_PARAMS";
/// 内部错误（序列化、任务异常等；不应依赖重试）。
pub const CODE_INTERNAL: &str = "INTERNAL";
/// 元数据不可用（加载失败）；不做任何兜底字典。
pub const CODE_METADATA_UNAVAILABLE: &str = "METADATA_UNAVAILABLE";
/// 快照标识不存在或已被淘汰。
pub const CODE_SNAPSHOT_UNKNOWN: &str = "SNAPSHOT_UNKNOWN";
/// 快照与输入槽位不匹配（跨槽导出/动作被拒绝）。
pub const CODE_SNAPSHOT_MISMATCH: &str = "SNAPSHOT_MISMATCH";
/// 定位线索不属于该快照的报告。
pub const CODE_LINK_LOCATION_UNKNOWN: &str = "LINK_LOCATION_UNKNOWN";
/// 来源不属于该定位线索（自造目标被拒绝）。
pub const CODE_LINK_SOURCE_UNKNOWN: &str = "LINK_SOURCE_UNKNOWN";
/// 来源没有 http/https 链接或协议不受支持。
pub const CODE_LINK_URL_UNSUPPORTED: &str = "LINK_URL_UNSUPPORTED";
/// 交给系统浏览器打开失败（保留重试与复制链接入口）。
pub const CODE_LINK_OPEN_FAILED: &str = "LINK_OPEN_FAILED";
/// 网络（UNC）或设备命名空间位置：明确不支持。
pub const CODE_UNSUPPORTED_LOCATION: &str = "UNSUPPORTED_LOCATION";

/// IPC 命令错误：稳定错误码 + 可直接展示的说明。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandError {
    /// 稳定错误码（界面按码分支，不解析 message）。
    pub code: &'static str,
    /// 说明文本（中文，可直接展示）。
    pub message: String,
}

impl CommandError {
    /// 构造命令错误。
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 参数非法。
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(CODE_INVALID_PARAMS, message)
    }

    /// 内部错误。
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(CODE_INTERNAL, message)
    }

    /// 元数据不可用。
    pub fn metadata_unavailable(message: impl Into<String>) -> Self {
        Self::new(CODE_METADATA_UNAVAILABLE, message)
    }

    /// 未知快照。
    pub fn snapshot_unknown(snapshot_id: &str) -> Self {
        Self::new(
            CODE_SNAPSHOT_UNKNOWN,
            format!("当前没有快照 {snapshot_id:?}（未分析、已被淘汰或不属于本次运行）"),
        )
    }

    /// 快照与槽位不匹配。
    pub fn snapshot_mismatch(expected: &str, actual: &str) -> Self {
        Self::new(
            CODE_SNAPSHOT_MISMATCH,
            format!("快照属于 {expected:?}，与请求的 {actual:?} 不一致；不能跨槽导出或执行动作"),
        )
    }

    /// 定位线索未知。
    pub fn link_location_unknown(message: impl Into<String>) -> Self {
        Self::new(CODE_LINK_LOCATION_UNKNOWN, message)
    }

    /// 来源未知。
    pub fn link_source_unknown(message: impl Into<String>) -> Self {
        Self::new(CODE_LINK_SOURCE_UNKNOWN, message)
    }

    /// 链接协议不受支持。
    pub fn link_url_unsupported(message: impl Into<String>) -> Self {
        Self::new(CODE_LINK_URL_UNSUPPORTED, message)
    }

    /// 链接打开失败。
    pub fn link_open_failed(message: impl Into<String>) -> Self {
        Self::new(CODE_LINK_OPEN_FAILED, message)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}（{}）", self.message, self.code)
    }
}

impl std::error::Error for CommandError {}

// ---------------------------------------------------------------------------
// 参数校验（纯函数；单元测试覆盖）
// ---------------------------------------------------------------------------

/// `request_id` 长度上限（前端生成，如 `req-<uuid>`）。
pub const REQUEST_ID_MAX_LEN: usize = 64;

/// 校验 `request_id`：1–64 个 `[A-Za-z0-9_-]` 字符（避免空白与不可见字符混入信封）。
pub fn validate_request_id(request_id: &str) -> Result<(), CommandError> {
    let valid = !request_id.is_empty()
        && request_id.len() <= REQUEST_ID_MAX_LEN
        && request_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        });
    if valid {
        Ok(())
    } else {
        Err(CommandError::invalid_params(format!(
            "request_id 必须是 1–{REQUEST_ID_MAX_LEN} 个 [A-Za-z0-9_-] 字符"
        )))
    }
}

/// 解析界面声明的分析用途（`campaign`/`unknown`；`profile` 由文件名判定，不接受显式声明）。
pub fn parse_context(context: &str) -> Result<Mode, CommandError> {
    match context {
        "campaign" => Ok(Mode::Campaign),
        "unknown" => Ok(Mode::Unknown),
        other => Err(CommandError::invalid_params(format!(
            "context 仅支持 campaign 或 unknown，收到 {other:?}"
        ))),
    }
}

/// 导出格式选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// 仅 JSON 信封。
    Json,
    /// 仅 CSV。
    Csv,
    /// JSON + CSV。
    Both,
}

impl ExportFormat {
    /// 是否写出 JSON。
    pub fn includes_json(self) -> bool {
        matches!(self, ExportFormat::Json | ExportFormat::Both)
    }

    /// 是否写出 CSV。
    pub fn includes_csv(self) -> bool {
        matches!(self, ExportFormat::Csv | ExportFormat::Both)
    }
}

/// 解析导出格式；缺省为 `both`。
pub fn parse_export_format(format: Option<&str>) -> Result<ExportFormat, CommandError> {
    match format.unwrap_or("both") {
        "json" => Ok(ExportFormat::Json),
        "csv" => Ok(ExportFormat::Csv),
        "both" => Ok(ExportFormat::Both),
        other => Err(CommandError::invalid_params(format!(
            "format 仅支持 json、csv 或 both，收到 {other:?}"
        ))),
    }
}

/// 校验分析输入：非空、非 UNC/设备命名空间；返回**原样路径**。
///
/// 存在性、可读性、占用与读取中变化都由稳定快照读取判定，并作为 SX-003 信封的
/// 输入级错误（`INPUT_UNREADABLE`/`INPUT_CHANGED`/`RESOURCE_LIMIT`）返回，
/// 避免与契约的错误词汇分叉；命令错误只用于参数与位置类别问题。
pub fn validate_analyze_input(raw: &str) -> Result<PathBuf, CommandError> {
    if raw.trim().is_empty() {
        return Err(CommandError::invalid_params("input_path 不能为空"));
    }
    let path = PathBuf::from(raw);
    match path_policy::classify(&path) {
        path_policy::PathClass::Unc | path_policy::PathClass::Device => {
            return Err(CommandError::new(
                CODE_UNSUPPORTED_LOCATION,
                format!(
                    "暂不支持网络（UNC）或设备命名空间位置：{}；请选择本地磁盘上的存档文件",
                    path.display()
                ),
            ));
        }
        path_policy::PathClass::Local | path_policy::PathClass::VerbatimLocal => {}
    }
    Ok(path)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// 输出路径错误 → 路径策略稳定码（`OUTPUT_EXISTS`、`INPUT_OUTPUT_OVERLAP` 等）。
fn map_output_path_error(error: path_policy::PathPolicyError) -> CommandError {
    CommandError::new(error.code.as_str(), error.message)
}

fn map_discovery_error(error: discovery::DiscoveryError) -> CommandError {
    let code = match error {
        discovery::DiscoveryError::RootMissing { .. } => "DISCOVERY_ROOT_MISSING",
        discovery::DiscoveryError::RootNotDirectory { .. } => "DISCOVERY_ROOT_NOT_DIRECTORY",
        discovery::DiscoveryError::RootUnreadable { .. } => "DISCOVERY_ROOT_UNREADABLE",
        discovery::DiscoveryError::UnsupportedLocation { .. } => CODE_UNSUPPORTED_LOCATION,
        discovery::DiscoveryError::TooManyFiles { .. } => "DISCOVERY_TOO_MANY_FILES",
    };
    CommandError::new(code, error.to_string())
}

/// 逐文件分析上下文：`profile.dat` 始终按资料文件处理（与 CLI 相同规则）。
fn declared_context(name: &str, mode: Mode) -> DeclaredContext {
    let mode = if name.eq_ignore_ascii_case("profile.dat") {
        Mode::Profile
    } else {
        mode
    };
    DeclaredContext {
        platform: None,
        mode,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    }
}

// ---------------------------------------------------------------------------
// 命令：发现与候选
// ---------------------------------------------------------------------------

/// 一个 Documents 候选目录的扫描结果（`status` 为稳定字符串）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentsCandidate {
    /// 候选目录路径（原样显示）。
    pub path: String,
    /// 来源稳定标识（`known_folder` / `user_profile`）。
    pub source: String,
    /// 来源中文标签。
    pub source_label: String,
    /// 扫描状态：`game_found` / `no_game_dir` / `link_not_followed` /
    /// `missing` / `not_directory` / `unreadable` / `unsupported_location`。
    pub status: String,
    /// 失败原因（`unreadable` 时）。
    pub detail: Option<String>,
}

/// 目录发现结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoverOutcome {
    /// 候选 Documents 目录（按优先级，已按规范路径去重）。
    pub candidates: Vec<DocumentsCandidate>,
    /// 发现到的游戏目录（`Documents\Shadow of the Tomb Raider`）。
    pub game_roots: Vec<String>,
}

/// 把扫描结果映射为 IPC DTO（纯函数，可注入测试候选）。
pub fn discover_outcome(scans: &[discovery::DocumentsScan]) -> DiscoverOutcome {
    let candidates = scans
        .iter()
        .map(|scan| {
            let (status, detail) = match &scan.status {
                discovery::DocumentsScanStatus::GameFound(_) => ("game_found", None),
                discovery::DocumentsScanStatus::NoGameDir => ("no_game_dir", None),
                discovery::DocumentsScanStatus::LinkNotFollowed => ("link_not_followed", None),
                discovery::DocumentsScanStatus::Missing => ("missing", None),
                discovery::DocumentsScanStatus::NotDirectory => ("not_directory", None),
                discovery::DocumentsScanStatus::Unreadable(message) => {
                    ("unreadable", Some(message.clone()))
                }
                discovery::DocumentsScanStatus::UnsupportedLocation => {
                    ("unsupported_location", None)
                }
            };
            DocumentsCandidate {
                path: scan.dir.path.to_string_lossy().into_owned(),
                source: match &scan.dir.source {
                    discovery::DocumentsSource::KnownFolder => "known_folder".to_string(),
                    discovery::DocumentsSource::UserProfile => "user_profile".to_string(),
                },
                source_label: scan.dir.source.label(),
                status: status.to_string(),
                detail,
            }
        })
        .collect();
    DiscoverOutcome {
        candidates,
        game_roots: discovery::game_roots(scans)
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    }
}

/// 发现候选目录：只在受限位置（Known Folder Documents、`%USERPROFILE%\Documents`）
/// 查找游戏目录，不做全盘扫描；不读取文件内容。
#[tauri::command]
pub async fn discover_saves() -> Result<DiscoverOutcome, CommandError> {
    let scans = discovery::scan_default_documents();
    Ok(discover_outcome(&scans))
}

/// 候选文件读取状态（仅元数据层面）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateFileDto {
    /// 实际文件名（不臆定 save 数字语义）。
    pub name: String,
    /// 完整路径（供用户选择后交给后端再校验）。
    pub path: String,
    /// 压缩大小；元数据不可读为 `null`。
    pub size_bytes: Option<u64>,
    /// 修改时间（Unix 毫秒，原始 UTC）；不可用为 `null`。
    pub modified_millis: Option<u64>,
    /// 是否匹配观察到的文件名模式（弱上下文提示，不是模式识别证明）。
    pub matches_observed_names: bool,
    /// `ready` / `too_large` / `unreadable`。
    pub status: String,
    /// 读取失败原因（`unreadable` 时）。
    pub detail: Option<String>,
}

/// 被跳过的目录项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkippedPathDto {
    /// 路径。
    pub path: String,
    /// `link_or_reparse_point` / `unreadable`。
    pub reason: String,
    /// 失败原因（`unreadable` 时）。
    pub detail: Option<String>,
}

/// 候选文件列表（有界枚举结果）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidatesOutcome {
    /// 枚举根目录（原样回显）。
    pub root: String,
    /// 候选文件（按名称稳定排序）。
    pub files: Vec<CandidateFileDto>,
    /// 被跳过的目录项（符号链接/重解析点、不可读条目）。
    pub skipped: Vec<SkippedPathDto>,
    /// 达到深度上限未展开的目录。
    pub depth_limited_dirs: Vec<String>,
}

/// 把有界枚举结果映射为 IPC DTO（纯函数）。
pub fn candidates_outcome(listing: discovery::CandidateListing) -> CandidatesOutcome {
    let files = listing
        .files
        .iter()
        .map(|file| {
            let (status, detail) = match &file.status {
                discovery::CandidateStatus::Ready => ("ready", None),
                discovery::CandidateStatus::TooLarge => ("too_large", None),
                discovery::CandidateStatus::Unreadable(message) => {
                    ("unreadable", Some(message.clone()))
                }
            };
            CandidateFileDto {
                name: file.name.clone(),
                path: file.path.to_string_lossy().into_owned(),
                size_bytes: file.size_bytes,
                modified_millis: file.modified_millis.map(|millis| millis as u64),
                matches_observed_names: file.matches_observed_names,
                status: status.to_string(),
                detail,
            }
        })
        .collect();
    let skipped = listing
        .skipped
        .iter()
        .map(|skipped| {
            let (reason, detail) = match &skipped.reason {
                discovery::SkippedReason::LinkOrReparsePoint => ("link_or_reparse_point", None),
                discovery::SkippedReason::Unreadable(message) => {
                    ("unreadable", Some(message.clone()))
                }
            };
            SkippedPathDto {
                path: skipped.path.to_string_lossy().into_owned(),
                reason: reason.to_string(),
                detail,
            }
        })
        .collect();
    CandidatesOutcome {
        root: listing.root.to_string_lossy().into_owned(),
        files,
        skipped,
        depth_limited_dirs: listing
            .depth_limited_dirs
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    }
}

/// 列出目录内的候选存档：本地卷、有界枚举（深度/数量/大小）、跳过符号链接与
/// 重解析点；不读取文件内容。
#[tauri::command]
pub async fn list_candidates(root_path: String) -> Result<CandidatesOutcome, CommandError> {
    if root_path.trim().is_empty() {
        return Err(CommandError::invalid_params("root_path 不能为空"));
    }
    let root = PathBuf::from(&root_path);
    let listing = discovery::list_candidates(&root, &discovery::DiscoveryLimits::default())
        .map_err(map_discovery_error)?;
    Ok(candidates_outcome(listing))
}

/// 打开系统文件/目录选择器（用户主动选择；取消返回 `null`，不算错误）。
///
/// 选择器由生产 setup 注入 dialog 插件实现；前端没有 `dialog:*` 权限，
/// 只能通过本命令发起用户选择。
#[tauri::command]
pub async fn select_input(
    state: State<'_, DesktopState>,
    kind: String,
) -> Result<Option<String>, CommandError> {
    if kind != "file" && kind != "directory" {
        return Err(CommandError::invalid_params(format!(
            "kind 仅支持 file 或 directory，收到 {kind:?}"
        )));
    }
    let picker = state.file_picker()?;
    // 原生对话框是阻塞调用（不能在主线程或异步工作线程上等待）。
    let kind_for_task = kind.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || picker(&kind_for_task))
        .await
        .map_err(|error| CommandError::internal(format!("选择器异常结束：{error}")))?
        .map_err(CommandError::internal)?;
    Ok(picked.map(|path| path.to_string_lossy().into_owned()))
}

// ---------------------------------------------------------------------------
// 命令：分析
// ---------------------------------------------------------------------------

/// 分析一个存档文件，返回 SX-003 契约的桌面结果信封。
///
/// 参数与位置类别问题（空参数、非法 context、UNC/设备命名空间）返回命令错误；
/// 文件级问题（不存在、目录、被占用、读取中变化、解压失败、超限）在信封的
/// `error` 字段返回。被取消的请求返回无结果信封；失败刷新返回携带旧快照的
/// `stale` 信封。
#[tauri::command]
pub async fn analyze_file(
    state: State<'_, DesktopState>,
    request_id: String,
    input_path: String,
    context: String,
) -> Result<DesktopResult, CommandError> {
    validate_request_id(&request_id)?;
    let mode = parse_context(&context)?;
    let input = validate_analyze_input(&input_path)?;
    let owned = state.inner().clone();
    let input_ref = file_name(&input);

    let (metadata, cancel, seq) = {
        let session = owned.session();
        let mut session = crate::session::lock(&session);
        let metadata = session.metadata()?;
        let (cancel, seq) = session.begin_request(&request_id, &input)?;
        (metadata, cancel, seq)
    };

    let session = owned.session();
    let cleanup_request_id = request_id.clone();
    let task = AnalysisTask {
        request_id,
        input,
        input_ref,
        mode,
        metadata,
        cancel,
        seq,
    };
    let handle = tauri::async_runtime::spawn_blocking(move || run_analysis(&session, task));
    handle.await.map_err(|error| {
        // 任务异常结束（不应发生）：移除登记，避免请求永久占用 request_id。
        owned.with_session(|session| session.complete_cancelled(&cleanup_request_id));
        CommandError::internal(format!("分析任务异常结束：{error}"))
    })
}

/// 一次分析请求的运行时参数（登记信息 + 只读输入 + 取消标记）。
struct AnalysisTask {
    request_id: String,
    input: PathBuf,
    input_ref: String,
    mode: Mode,
    metadata: Arc<MetadataBundle>,
    cancel: Arc<AtomicBool>,
    seq: u64,
}

/// 阻塞线程内的分析流程（读取在分析前后各检查一次取消标记）。
fn run_analysis(session: &Arc<Mutex<Session>>, task: AnalysisTask) -> DesktopResult {
    let AnalysisTask {
        request_id,
        input,
        input_ref,
        mode,
        metadata,
        cancel,
        seq,
    } = task;
    let request_id = request_id.as_str();
    let input_ref = input_ref.as_str();
    let input = input.as_path();
    let limits = Limits::default();
    if cancel.load(Ordering::SeqCst) {
        return finish_cancelled(session, request_id, input_ref);
    }
    let snapshot = match read_snapshot::read_stable(input, &limits) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            if cancel.load(Ordering::SeqCst) {
                return finish_cancelled(session, request_id, input_ref);
            }
            return finish_failure(session, request_id, input, input_ref, &error);
        }
    };
    if cancel.load(Ordering::SeqCst) {
        return finish_cancelled(session, request_id, input_ref);
    }
    let declared = declared_context(input_ref, mode);
    let report = match analyze_bytes(&snapshot.bytes, &declared, &metadata, &limits) {
        Ok(report) => report,
        Err(error) => {
            if cancel.load(Ordering::SeqCst) {
                return finish_cancelled(session, request_id, input_ref);
            }
            return finish_failure(session, request_id, input, input_ref, &error);
        }
    };

    let millis = time::unix_millis_now();
    let snapshot_id = new_snapshot_id(millis, &snapshot.compressed_sha256);
    let snapshot_time = time::format_rfc3339_utc_millis(millis);
    let report_for_envelope = report.clone();
    let stored = StoredSnapshot {
        input_ref: input_ref.to_string(),
        input_path: input.to_path_buf(),
        snapshot_id: snapshot_id.clone(),
        snapshot_time: snapshot_time.clone(),
        freshness: Freshness::Current,
        last_refresh_error: None,
        report,
        metadata,
        seq,
    };
    let outcome = {
        let mut session = crate::session::lock(session);
        session.complete_success(request_id, stored)
    };
    match outcome {
        CommitOutcome::Cancelled => cancelled_result(request_id, input_ref),
        CommitOutcome::Committed | CommitOutcome::Superseded => DesktopResult {
            schema_version: SchemaVersion,
            request_id: request_id.to_string(),
            input_ref: input_ref.to_string(),
            snapshot_id: Some(snapshot_id),
            snapshot_time: Some(snapshot_time),
            freshness: Freshness::Current,
            last_refresh_error: None,
            report: Some(report_for_envelope),
            error: None,
        },
    }
}

/// 取消/无需继续：移除请求登记并返回“无结果信封”（不产生成功报告）。
fn finish_cancelled(
    session: &Arc<Mutex<Session>>,
    request_id: &str,
    input_ref: &str,
) -> DesktopResult {
    {
        let mut session = crate::session::lock(session);
        session.complete_cancelled(request_id);
    }
    cancelled_result(request_id, input_ref)
}

/// 失败收尾：无旧快照返回错误信封；有旧快照时标记 stale 并随信返回旧报告。
fn finish_failure(
    session: &Arc<Mutex<Session>>,
    request_id: &str,
    input: &Path,
    input_ref: &str,
    error: &InputError,
) -> DesktopResult {
    let outcome = {
        let mut session = crate::session::lock(session);
        session.complete_failure(request_id, input, error)
    };
    match outcome {
        FailureOutcome::Cancelled => cancelled_result(request_id, input_ref),
        FailureOutcome::Stale(snapshot) => DesktopResult {
            schema_version: SchemaVersion,
            request_id: request_id.to_string(),
            input_ref: input_ref.to_string(),
            snapshot_id: Some(snapshot.snapshot_id.clone()),
            snapshot_time: Some(snapshot.snapshot_time.clone()),
            freshness: Freshness::Stale,
            last_refresh_error: Some(error.clone()),
            report: Some(snapshot.report.clone()),
            error: Some(error.clone()),
        },
        FailureOutcome::Failed => DesktopResult {
            schema_version: SchemaVersion,
            request_id: request_id.to_string(),
            input_ref: input_ref.to_string(),
            snapshot_id: None,
            snapshot_time: None,
            freshness: Freshness::None,
            last_refresh_error: None,
            report: None,
            error: Some(error.clone()),
        },
    }
}

/// 取消的请求：不返回报告或错误（前端保留原视图，显示“已取消”）。
fn cancelled_result(request_id: &str, input_ref: &str) -> DesktopResult {
    DesktopResult {
        schema_version: SchemaVersion,
        request_id: request_id.to_string(),
        input_ref: input_ref.to_string(),
        snapshot_id: None,
        snapshot_time: None,
        freshness: Freshness::None,
        last_refresh_error: None,
        report: None,
        error: None,
    }
}

/// 取消结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CancelOutcome {
    /// 请求标识。
    pub request_id: String,
    /// 请求仍在进行且已被标记取消；请求已完成或未知为 `false`。
    pub cancelled: bool,
}

/// 取消一个在飞分析请求（幂等：已完成或未知返回 `cancelled = false`）。
#[tauri::command]
pub fn cancel_analysis(
    state: State<'_, DesktopState>,
    request_id: String,
) -> Result<CancelOutcome, CommandError> {
    validate_request_id(&request_id)?;
    let cancelled = state.with_session(|session| session.cancel_request(&request_id));
    Ok(CancelOutcome {
        request_id,
        cancelled,
    })
}

// ---------------------------------------------------------------------------
// 命令：导出
// ---------------------------------------------------------------------------

/// 导出产物。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportArtifact {
    /// 文件名（`report.json` / `report.csv`）。
    pub name: String,
    /// 实际写出路径。
    pub path: String,
}

/// 导出结果（实际目标路径与快照身份；无报告、跨槽或旧结果均有明确字段）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportOutcome {
    /// 快照标识。
    pub snapshot_id: String,
    /// 输入引用。
    pub input_ref: String,
    /// 快照时效（`stale` 时产物内同时携带过期标识与刷新错误码）。
    pub freshness: Freshness,
    /// 最近刷新错误码。
    pub last_refresh_error_code: Option<String>,
    /// 是否分享脱敏。
    pub share: bool,
    /// 产物清单。
    pub artifacts: Vec<ExportArtifact>,
}

/// 把当前快照导出为 JSON 信封与/或 CSV；引用同一 `input_digest` 与元数据版本。
///
/// 目标必须是尚不存在的新目录，且与输入不重叠（不覆盖存档或既有报告）；
/// 分享模式默认开启，导出前通过路径扫描自检。
#[tauri::command]
pub async fn export_report(
    state: State<'_, DesktopState>,
    snapshot_id: String,
    input_ref: String,
    output_dir: String,
    format: Option<String>,
    share: Option<bool>,
) -> Result<ExportOutcome, CommandError> {
    if snapshot_id.trim().is_empty() {
        return Err(CommandError::invalid_params("snapshot_id 不能为空"));
    }
    let format = parse_export_format(format.as_deref())?;
    let share = share.unwrap_or(true);
    let stored = state.with_session(|session| session.snapshot(&snapshot_id))?;
    if stored.input_ref != input_ref {
        return Err(CommandError::snapshot_mismatch(
            &stored.input_ref,
            &input_ref,
        ));
    }
    let output = PathBuf::from(&output_dir);
    // 输入与输出不得重叠：输入仍可解析时用文件本身（解析链接与真实大小写）；
    // 输入已被移走或锁定（旧结果导出场景）时退回其父目录，规则等价（输出仍不得
    // 位于存档目录内），且不需要打开输入。
    let overlap_input = if stored.input_path.is_file() {
        stored.input_path.clone()
    } else {
        stored
            .input_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| stored.input_path.clone())
    };
    path_policy::validate_output(&output, &[overlap_input.as_path()])
        .map_err(map_output_path_error)?;

    let refresh_error_code = stored.last_refresh_error_code();
    let document = export_document(
        stored.report.clone(),
        ExportSnapshot {
            input_ref: &stored.input_ref,
            snapshot_id: &stored.snapshot_id,
            snapshot_time: &stored.snapshot_time,
            freshness: stored.freshness,
            last_refresh_error_code: refresh_error_code.as_deref(),
        },
    )
    .map_err(|error| CommandError::internal(format!("导出信封组装失败：{error}")))?;
    let document = if share {
        share_export_document(&document)
            .map_err(|error| CommandError::internal(format!("分享脱敏失败：{error}")))?
    } else {
        document
    };

    let mut artifacts: Vec<(String, String)> = Vec::new();
    if format.includes_json() {
        let json = render_export_json(&document)
            .map_err(|error| CommandError::internal(format!("导出 JSON 生成失败：{error}")))?;
        artifacts.push(("report.json".to_string(), json));
    }
    if format.includes_csv() {
        let csv = render_export_csv(&document, &stored.metadata, share);
        artifacts.push(("report.csv".to_string(), csv));
    }
    if share {
        for (name, contents) in &artifacts {
            let findings = scan_private_material(contents);
            if !findings.is_empty() {
                return Err(CommandError::internal(format!(
                    "分享导出自检失败（{name} 仍含路径样片段）：{}",
                    findings.join("；")
                )));
            }
        }
    }

    let artifact_names: Vec<String> = artifacts.iter().map(|(name, _)| name.clone()).collect();
    let paths = tauri::async_runtime::spawn_blocking(move || {
        output_commit::prepare(&output, &artifacts).and_then(|staging| staging.commit())
    })
    .await
    .map_err(|error| CommandError::internal(format!("导出任务异常结束：{error}")))?
    .map_err(|error| CommandError::new(error.code.as_str(), error.message))?;

    Ok(ExportOutcome {
        snapshot_id: stored.snapshot_id,
        input_ref: stored.input_ref,
        freshness: stored.freshness,
        last_refresh_error_code: refresh_error_code,
        share,
        artifacts: artifact_names
            .into_iter()
            .zip(paths)
            .map(|(name, path)| ExportArtifact {
                name,
                path: path.to_string_lossy().into_owned(),
            })
            .collect(),
    })
}

// ---------------------------------------------------------------------------
// 命令：攻略链接
// ---------------------------------------------------------------------------

/// 解析当前报告的攻略来源链接（只校验与解析，不打开系统浏览器）。
///
/// 供“复制已核验链接”等无副作用路径使用：返回已校验的 http/https 地址与
/// 快照时效、组门禁结论；打开失败与解析失败因此在界面上可分别处理。
#[tauri::command]
pub fn resolve_guide_link(
    state: State<'_, DesktopState>,
    snapshot_id: String,
    location_id: String,
    source_ref: String,
) -> Result<GuideLink, CommandError> {
    resolve_link(&state, &snapshot_id, &location_id, &source_ref)
}

/// 打开当前报告的攻略来源链接（只允许元数据来源中的 http/https 地址）。
///
/// 前端提交 `snapshot_id`/`location_id`/`source_ref` 稳定 ID；后端从该快照的
/// 元数据解析 URL 并交给系统外部浏览器。链接不附加存档内容、私人路径或账户
/// 信息；读取失败与打开失败分别返回稳定错误码，界面保留重试与复制入口。
#[tauri::command]
pub fn open_guide_link(
    state: State<'_, DesktopState>,
    snapshot_id: String,
    location_id: String,
    source_ref: String,
) -> Result<GuideLink, CommandError> {
    let link = resolve_link(&state, &snapshot_id, &location_id, &source_ref)?;
    state.open_url(&link.url)?;
    Ok(link)
}

/// 攻略链接的公共校验与解析（打开与仅解析命令共用）。
fn resolve_link(
    state: &DesktopState,
    snapshot_id: &str,
    location_id: &str,
    source_ref: &str,
) -> Result<GuideLink, CommandError> {
    if snapshot_id.trim().is_empty()
        || location_id.trim().is_empty()
        || source_ref.trim().is_empty()
    {
        return Err(CommandError::invalid_params(
            "snapshot_id、location_id 与 source_ref 都不能为空",
        ));
    }
    state.with_session(|session| session.resolve_guide_link(snapshot_id, location_id, source_ref))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids_are_restricted_to_safe_characters() {
        for valid in [
            "req-1",
            "a",
            "0f5c1e2a-9b1c-4a1f",
            &"x".repeat(REQUEST_ID_MAX_LEN),
        ] {
            assert!(validate_request_id(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "",
            " ",
            "req 1",
            "req\n1",
            "请求",
            &"x".repeat(REQUEST_ID_MAX_LEN + 1),
        ] {
            let error = validate_request_id(invalid).unwrap_err();
            assert_eq!(error.code, CODE_INVALID_PARAMS);
        }
    }

    #[test]
    fn context_and_format_parsing_rejects_unknown_values() {
        assert_eq!(parse_context("campaign").unwrap(), Mode::Campaign);
        assert_eq!(parse_context("unknown").unwrap(), Mode::Unknown);
        assert_eq!(
            parse_context("profile").unwrap_err().code,
            CODE_INVALID_PARAMS,
            "profile 由文件名判定，不接受显式声明"
        );
        assert_eq!(parse_export_format(None).unwrap(), ExportFormat::Both);
        assert_eq!(
            parse_export_format(Some("json")).unwrap(),
            ExportFormat::Json
        );
        assert_eq!(
            parse_export_format(Some("xlsx")).unwrap_err().code,
            CODE_INVALID_PARAMS
        );
    }

    #[test]
    fn discover_outcome_maps_stable_statuses() {
        use discovery::{DocumentsDir, DocumentsScan, DocumentsScanStatus, DocumentsSource};
        let scans = vec![
            DocumentsScan {
                dir: DocumentsDir {
                    path: PathBuf::from(r"C:\Users\demo\Documents"),
                    source: DocumentsSource::KnownFolder,
                },
                status: DocumentsScanStatus::GameFound(PathBuf::from(
                    r"C:\Users\demo\Documents\Shadow of the Tomb Raider",
                )),
            },
            DocumentsScan {
                dir: DocumentsDir {
                    path: PathBuf::from(r"C:\Users\demo\Documents"),
                    source: DocumentsSource::UserProfile,
                },
                status: DocumentsScanStatus::Unreadable("拒绝访问".to_string()),
            },
        ];
        let outcome = discover_outcome(&scans);
        assert_eq!(outcome.candidates.len(), 2);
        assert_eq!(outcome.candidates[0].status, "game_found");
        assert_eq!(outcome.candidates[0].source, "known_folder");
        assert_eq!(outcome.candidates[1].status, "unreadable");
        assert_eq!(outcome.candidates[1].detail.as_deref(), Some("拒绝访问"));
        assert_eq!(outcome.game_roots.len(), 1);
        assert!(outcome.game_roots[0].ends_with("Shadow of the Tomb Raider"));
    }
}
