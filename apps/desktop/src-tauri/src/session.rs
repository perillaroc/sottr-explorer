//! 桌面会话状态（SX-014）：请求登记与取消、快照存储、元数据与攻略来源解析。
//!
//! 边界与不变量：
//!
//! - **请求隔离**：每次分析以调用方提供的 `request_id` 登记；`request_id` 在
//!   处理中不可重复。结果信封总是回显该 ID，前端只提交当前选定请求的结果；
//!   被取消的请求只返回“无结果信封”（`freshness = none`，`report`/`error` 均为
//!   `null`），不产生成功报告。
//! - **取消语义**：取消标记与结果提交在同一把会话锁内检查；取消先于提交时
//!   绝无成功结果，提交先于取消时 `cancel_request` 返回 `false`（请求已完成）。
//!   取消是检查点式的（读取前后、分析前后），不声称内核级中断。
//! - **晚到旧请求**：同一输入每次请求分配递增序号；只有“不旧于当前快照”的
//!   成功结果才会写入快照存储。后发起的请求先完成时，晚到的旧结果不能覆盖
//!   新快照（导出与攻略动作始终绑定正确快照，T13/T20）。
//! - **快照存储**：每个输入只保留最近一次成功快照（内存最多
//!   [`MAX_STORED_SNAPSHOTS`] 个，按插入顺序淘汰最旧输入）。失败刷新会把已存
//!   快照标记为 `stale` 并记录刷新错误码，供旧结果导出携带过期标识。
//! - **路径与隐私**：输入路径仅用于路径策略、去重与输出重叠校验；不进入 IPC
//!   信封（信封只带文件名 `input_ref`），也不进入攻略链接。
//! - **元数据**：默认内嵌仓库 `data/metadata/` 三个数据集（编译期）；加载失败
//!   时相关命令返回 `METADATA_UNAVAILABLE`，没有任何内置兜底字典。

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::report::{Freshness, GroupActionKind, InputError, Report};

use serde::Serialize;

use crate::commands::CommandError;

/// 内嵌元数据文件（`file_id` 与仓库 `data/metadata/` 同名；`manifest.json` 不是数据集）。
pub const EMBEDDED_METADATA_FILES: &[(&str, &str)] = &[
    (
        "collectibles.json",
        include_str!("../../../../data/metadata/collectibles.json"),
    ),
    (
        "region_counters.json",
        include_str!("../../../../data/metadata/region_counters.json"),
    ),
    (
        "reference_sets.json",
        include_str!("../../../../data/metadata/reference_sets.json"),
    ),
];

/// 内存中保留的分析快照上限（超出时淘汰最早写入的输入）。
pub const MAX_STORED_SNAPSHOTS: usize = 16;

/// 链接打开器：生产环境由 opener 插件注入（系统外部浏览器）；
/// 测试注入记录器，避免测试真的启动浏览器。
pub type UrlOpener = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// 文件/目录选择器：生产环境由 dialog 插件注入（原生选择器，前台不可直调）；
/// 测试注入固定返回值，避免弹出真实对话框。
///
/// 参数为稳定种类字符串（`file` / `directory`），返回所选路径（取消为 `None`）。
pub type FilePicker = Arc<dyn Fn(&str) -> Result<Option<PathBuf>, String> + Send + Sync>;

/// 加载内嵌元数据（错误文本可直接展示；不提供任何兜底字典）。
pub fn embedded_metadata() -> Result<MetadataBundle, String> {
    let inputs: Vec<MetadataFileInput<'_>> = EMBEDDED_METADATA_FILES
        .iter()
        .map(|(file_id, contents)| MetadataFileInput {
            file_id: (*file_id).to_string(),
            bytes: contents.as_bytes(),
        })
        .collect();
    MetadataBundle::from_files(&inputs, &MetadataLimits::default())
        .map_err(|error| format!("元数据加载失败（{}）：{}", error.code(), error.message()))
}

/// 构造快照标识：`snap-<毫秒>-<内容摘要前 12 位>`。
pub fn new_snapshot_id(millis: u128, compressed_sha256: &str) -> String {
    let prefix = compressed_sha256.get(..12).unwrap_or(compressed_sha256);
    format!("snap-{millis}-{prefix}")
}

/// 一个已提交的分析快照（仅存于后端内存）。
#[derive(Debug, Clone)]
pub struct StoredSnapshot {
    /// 输入引用（文件名，不含目录；写入信封与导出）。
    pub input_ref: String,
    /// 规范输入路径（仅后端使用：路径策略、去重、输出重叠校验）。
    pub input_path: PathBuf,
    /// 快照标识。
    pub snapshot_id: String,
    /// 快照时间（RFC 3339 UTC）。
    pub snapshot_time: String,
    /// 时效状态（`current` 或失败刷新后的 `stale`）。
    pub freshness: Freshness,
    /// 最近刷新错误（`stale` 时必填）。
    pub last_refresh_error: Option<InputError>,
    /// 分析报告（与 CLI 同核心输出）。
    pub report: Report,
    /// 本次分析使用的元数据包（导出 CSV 与攻略来源必须绑定同一版本）。
    pub metadata: Arc<MetadataBundle>,
    /// 同输入请求的递增序号（用于拒绝晚到的旧结果）。
    pub seq: u64,
}

impl StoredSnapshot {
    /// 最近刷新错误码（导出信封要求显式携带；无错误为 `None`）。
    pub fn last_refresh_error_code(&self) -> Option<String> {
        self.last_refresh_error
            .as_ref()
            .map(|error| error.code.clone())
    }
}

/// 在飞请求登记项。
#[derive(Debug)]
struct RequestSlot {
    cancel: Arc<AtomicBool>,
    seq: u64,
}

/// 成功结果的提交结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    /// 已写入快照存储（成为该输入的当前快照）。
    Committed,
    /// 同一输入的更新请求已提交过更新的快照；本结果不写入存储（前端按
    /// `request_id` 丢弃晚到结果）。
    Superseded,
    /// 请求已被取消：不写入存储，也不返回成功信封。
    Cancelled,
}

/// 失败请求的收尾结论。
#[derive(Debug)]
pub enum FailureOutcome {
    /// 请求已被取消（返回无结果信封，不显示输入错误）。
    Cancelled,
    /// 该输入已有旧快照：已标记为 `stale` 并携带刷新错误，供前端保留旧摘要。
    Stale(Box<StoredSnapshot>),
    /// 没有可保留的旧快照（首次分析失败）。
    Failed,
}

/// 攻略链接解析结果（URL 来自元数据来源，前端不提交任意地址）。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuideLink {
    /// 快照标识。
    pub snapshot_id: String,
    /// 输入引用。
    pub input_ref: String,
    /// 定位 ID。
    pub location_id: String,
    /// 元数据来源 ID。
    pub source_ref: String,
    /// 已校验的 http/https 链接。
    pub url: String,
    /// 快照时效（`stale` 时界面须标注“不代表当前状态”）。
    pub freshness: Freshness,
    /// 当前报告的组级门禁结论（无组级线索为 `None`）。
    pub action_kind: Option<GroupActionKind>,
    /// 组级门禁原因码。
    pub action_reason_codes: Vec<String>,
}

/// 会话状态：请求、序号、快照存储共享同一把锁，保证取消/提交的顺序确定。
#[derive(Debug)]
pub struct Session {
    metadata: Result<Arc<MetadataBundle>, String>,
    requests: BTreeMap<String, RequestSlot>,
    attempts: BTreeMap<PathBuf, u64>,
    snapshots: BTreeMap<String, StoredSnapshot>,
    order: VecDeque<String>,
}

impl Session {
    /// 新会话：加载内嵌元数据（失败时保留错误文本，命令层报
    /// `METADATA_UNAVAILABLE`，不做兜底）。
    pub fn new() -> Self {
        let metadata = embedded_metadata().map(Arc::new);
        Self {
            metadata,
            requests: BTreeMap::new(),
            attempts: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            order: VecDeque::new(),
        }
    }

    /// 当前元数据包（错误为稳定命令错误）。
    pub fn metadata(&self) -> Result<Arc<MetadataBundle>, CommandError> {
        self.metadata
            .clone()
            .map_err(CommandError::metadata_unavailable)
    }

    /// 登记一个分析请求；重复 `request_id` 直接拒绝（避免结果身份混淆）。
    ///
    /// 返回取消标记与本次请求的递增序号。
    pub fn begin_request(
        &mut self,
        request_id: &str,
        input_path: &Path,
    ) -> Result<(Arc<AtomicBool>, u64), CommandError> {
        if self.requests.contains_key(request_id) {
            return Err(CommandError::invalid_params(format!(
                "request_id {request_id:?} 正在处理中，不能重复使用"
            )));
        }
        let seq = self.attempts.get(input_path).copied().unwrap_or(0) + 1;
        self.attempts.insert(input_path.to_path_buf(), seq);
        let cancel = Arc::new(AtomicBool::new(false));
        self.requests.insert(
            request_id.to_string(),
            RequestSlot {
                cancel: cancel.clone(),
                seq,
            },
        );
        Ok((cancel, seq))
    }

    /// 在飞请求数（诊断与测试用）。
    pub fn pending_requests(&self) -> usize {
        self.requests.len()
    }

    /// 取消在飞请求：存在且尚未完成时置位并返回 `true`；已完成或未知返回 `false`。
    pub fn cancel_request(&mut self, request_id: &str) -> bool {
        match self.requests.get(request_id) {
            Some(slot) => {
                slot.cancel.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    /// 取消全部在飞请求（窗口关闭等场景）；返回取消数量。
    pub fn cancel_all(&mut self) -> usize {
        for slot in self.requests.values() {
            slot.cancel.store(true, Ordering::SeqCst);
        }
        self.requests.len()
    }

    /// 收尾一个已观测到取消（或无需继续）的请求：移除登记，不写入任何结果。
    ///
    /// 返回 `true` 表示登记项存在且取消标记已置位；无论如何都不会产生快照。
    pub fn complete_cancelled(&mut self, request_id: &str) -> bool {
        match self.requests.remove(request_id) {
            Some(slot) => slot.cancel.load(Ordering::SeqCst),
            None => false,
        }
    }

    /// 提交成功结果：只有在同一把锁内确认请求未被取消时才写入快照存储。
    pub fn complete_success(
        &mut self,
        request_id: &str,
        snapshot: StoredSnapshot,
    ) -> CommitOutcome {
        let Some(slot) = self.requests.remove(request_id) else {
            return CommitOutcome::Cancelled;
        };
        if slot.cancel.load(Ordering::SeqCst) {
            return CommitOutcome::Cancelled;
        }
        if let Some(existing) = self.current_for_path(&snapshot.input_path)
            && existing.seq > snapshot.seq
        {
            return CommitOutcome::Superseded;
        }
        let snapshot_id = snapshot.snapshot_id.clone();
        let input_path = snapshot.input_path.clone();
        self.drop_snapshots_for_path(&input_path);
        self.snapshots.insert(snapshot_id.clone(), snapshot);
        self.order.retain(|existing| existing != &snapshot_id);
        self.order.push_back(snapshot_id);
        self.evict_over_limit();
        CommitOutcome::Committed
    }

    /// 提交失败/取消：取消优先；否则把同一输入的旧快照标记为 `stale`
    /// 并携带刷新错误（有旧快照时）。
    pub fn complete_failure(
        &mut self,
        request_id: &str,
        path: &Path,
        error: &InputError,
    ) -> FailureOutcome {
        let Some(slot) = self.requests.remove(request_id) else {
            return FailureOutcome::Failed;
        };
        if slot.cancel.load(Ordering::SeqCst) {
            return FailureOutcome::Cancelled;
        }
        let Some(snapshot_id) = self.snapshot_id_for_path(path) else {
            return FailureOutcome::Failed;
        };
        let Some(entry) = self.snapshots.get_mut(&snapshot_id) else {
            return FailureOutcome::Failed;
        };
        if entry.seq > slot.seq {
            // 更新请求已提交更新的快照：本次失败不得把新快照降级。
            return FailureOutcome::Failed;
        }
        entry.freshness = Freshness::Stale;
        entry.last_refresh_error = Some(error.clone());
        FailureOutcome::Stale(Box::new(entry.clone()))
    }

    /// 按快照标识取快照（导出与攻略动作使用；未知返回 `SNAPSHOT_UNKNOWN`）。
    pub fn snapshot(&self, snapshot_id: &str) -> Result<StoredSnapshot, CommandError> {
        self.snapshots
            .get(snapshot_id)
            .cloned()
            .ok_or_else(|| CommandError::snapshot_unknown(snapshot_id))
    }

    /// 某输入的当前（最近成功）快照。
    pub fn current_for_path(&self, path: &Path) -> Option<StoredSnapshot> {
        self.snapshots
            .values()
            .find(|snapshot| snapshot.input_path == path)
            .cloned()
    }

    /// 解析攻略链接：定位 → 来源 → 元数据 URL，全部在本会话内校验。
    ///
    /// 前端只提交稳定 ID（`snapshot_id`/`location_id`/`source_ref`），不能提交
    /// 任意地址；URL 必须来自该报告的元数据来源且为 http/https。快照为 `stale`
    /// 时仍允许打开（作为一般来源阅读），由信封携带 `freshness` 供界面标注。
    pub fn resolve_guide_link(
        &self,
        snapshot_id: &str,
        location_id: &str,
        source_ref: &str,
    ) -> Result<GuideLink, CommandError> {
        let snapshot = self.snapshot(snapshot_id)?;
        let location = snapshot
            .report
            .locations
            .iter()
            .find(|location| location.location_id == location_id)
            .ok_or_else(|| {
                CommandError::link_location_unknown(format!(
                    "当前快照没有定位线索 {location_id:?}（须来自同一报告的 locations）"
                ))
            })?;
        if !location.source_refs.iter().any(|known| known == source_ref) {
            return Err(CommandError::link_source_unknown(format!(
                "来源 {source_ref:?} 不属于定位线索 {location_id:?}（不能提交自造目标）"
            )));
        }
        let url = snapshot
            .metadata
            .dataset()
            .and_then(|dataset| dataset.sources.get(source_ref))
            .and_then(|source| source.url.clone())
            .ok_or_else(|| {
                CommandError::link_url_unsupported(format!(
                    "来源 {source_ref:?} 没有可打开链接（未提供 URL 或来源未知）"
                ))
            })?;
        if !is_supported_guide_url(&url) {
            return Err(CommandError::link_url_unsupported(format!(
                "来源链接协议不受支持（仅允许 http/https）：{url}"
            )));
        }
        let (action_kind, action_reason_codes) = location
            .group_actionability
            .as_ref()
            .map(|action| (Some(action.kind), action.reason_codes.clone()))
            .unwrap_or((None, Vec::new()));
        Ok(GuideLink {
            snapshot_id: snapshot.snapshot_id.clone(),
            input_ref: snapshot.input_ref.clone(),
            location_id: location.location_id.clone(),
            source_ref: source_ref.to_string(),
            url,
            freshness: snapshot.freshness,
            action_kind,
            action_reason_codes,
        })
    }

    fn snapshot_id_for_path(&self, path: &Path) -> Option<String> {
        self.snapshots
            .values()
            .find(|snapshot| snapshot.input_path == path)
            .map(|snapshot| snapshot.snapshot_id.clone())
    }

    fn drop_snapshots_for_path(&mut self, path: &Path) {
        let removed: Vec<String> = self
            .snapshots
            .iter()
            .filter(|(_, snapshot)| snapshot.input_path == path)
            .map(|(snapshot_id, _)| snapshot_id.clone())
            .collect();
        for snapshot_id in removed {
            self.snapshots.remove(&snapshot_id);
            self.order.retain(|existing| existing != &snapshot_id);
        }
    }

    fn evict_over_limit(&mut self) {
        while self.snapshots.len() > MAX_STORED_SNAPSHOTS {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.snapshots.remove(&oldest);
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// 桌面状态：会话 + 链接打开器 + 文件选择器（生产由 `run()` 的 setup 注入，
/// 测试注入记录器/固定值）。
#[derive(Clone)]
pub struct DesktopState {
    session: Arc<Mutex<Session>>,
    url_opener: Arc<Mutex<Option<UrlOpener>>>,
    file_picker: Arc<Mutex<Option<FilePicker>>>,
}

impl DesktopState {
    /// 新状态（加载内嵌元数据；不访问网络）。
    pub fn new() -> Self {
        Self {
            session: Arc::new(Mutex::new(Session::new())),
            url_opener: Arc::new(Mutex::new(None)),
            file_picker: Arc::new(Mutex::new(None)),
        }
    }

    /// 共享会话句柄（命令在阻塞线程内使用）。
    pub fn session(&self) -> Arc<Mutex<Session>> {
        self.session.clone()
    }

    /// 注入链接打开器（生产：opener 插件；测试：记录器）。
    pub fn set_url_opener(&self, opener: UrlOpener) {
        *lock(&self.url_opener) = Some(opener);
    }

    /// 注入文件/目录选择器（生产：dialog 插件；测试：固定值）。
    pub fn set_file_picker(&self, picker: FilePicker) {
        *lock(&self.file_picker) = Some(picker);
    }

    /// 取文件选择器；未注入时返回 `INTERNAL`（生产不出现）。
    pub fn file_picker(&self) -> Result<FilePicker, CommandError> {
        lock(&self.file_picker)
            .clone()
            .ok_or_else(|| CommandError::internal("文件选择器未初始化"))
    }

    /// 打开已校验的外链（失败映射 `LINK_OPEN_FAILED`，不重试、不缓存页面）。
    pub fn open_url(&self, url: &str) -> Result<(), CommandError> {
        let opener = lock(&self.url_opener)
            .clone()
            .ok_or_else(|| CommandError::internal("链接打开器未初始化"))?;
        opener(url).map_err(CommandError::link_open_failed)
    }

    /// 借用会话执行一段操作（命令层与窗口关闭清理共用）。
    pub fn with_session<T>(&self, action: impl FnOnce(&mut Session) -> T) -> T {
        action(&mut lock(&self.session))
    }
}

impl Default for DesktopState {
    fn default() -> Self {
        Self::new()
    }
}

/// 读取互斥锁；持锁线程 panic 被毒化时恢复内部值（会话数据不因一次 panic 报废）。
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// 攻略链接白名单：仅 http/https 且主机非空、不含空白（与元数据加载器同口径，
/// 此处作为链接出口的独立防御层）。
pub fn is_supported_guide_url(url: &str) -> bool {
    let scheme_ok = url
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || url
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
    if !scheme_ok {
        return false;
    }
    let remainder = &url[url.find("://").expect("scheme checked above") + 3..];
    let host = remainder.split(['/', '?', '#']).next().unwrap_or_default();
    !host.is_empty() && !url.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_ids_carry_time_and_digest_prefix() {
        assert_eq!(
            new_snapshot_id(
                1_792_067_696_789,
                "ea2fa5b6c0ffee0000000000000000000000000000000000000000000000"
            ),
            "snap-1792067696789-ea2fa5b6c0ff"
        );
    }

    #[test]
    fn guide_url_whitelist_accepts_only_http_https_with_host() {
        for allowed in [
            "https://tombraiders.net/stella/walks/TR11walk/collectibles/13-san-juan-murals-caches.html",
            "http://example.com/",
        ] {
            assert!(is_supported_guide_url(allowed), "{allowed}");
        }
        for denied in [
            "",
            "https://",
            "http://",
            "javascript:alert(1)",
            "file:///C:/secret.dat",
            "data:text/html,<script>alert(1)</script>",
            "https://example.com/has space",
            "ftp://example.com/x",
        ] {
            assert!(!is_supported_guide_url(denied), "{denied}");
        }
    }
}
