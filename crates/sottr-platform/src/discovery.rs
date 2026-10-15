//! 受限目录发现（SX-012）。
//!
//! 默认候选顺序：
//!
//! 1. Windows Known Folder 的 Documents（`SHGetKnownFolderPath(FOLDERID_Documents)`，
//!    已包含 OneDrive 重定向与中文路径）；
//! 2. `%USERPROFILE%\Documents` 回退；
//! 3. OneDrive 环境变量（`OneDrive`、`OneDriveConsumer`、`OneDriveCommercial`）
//!    下的 `Documents`。
//!
//! 只在候选 Documents 目录下的 [`GAME_DIR_NAME`] 子目录发现候选，**不做全盘扫描**；
//! 枚举限制深度、文件数与单文件大小，跳过符号链接与重解析点（避免越出受限范围；
//! OneDrive 云占位文件不受影响），按名称稳定排序。发现逻辑接受显式候选列表，
//! 测试可注入根目录（[`scan_documents`]）。
//!
//! 只读：本模块只列目录与读元数据，不打开文件内容、不写回、不修改权限、不提权。
//! 修改时间以原始 UTC 毫秒返回，本地时区显示与“系统本地时间”文案由界面承担。

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::path_policy::{self, PathClass};

/// 游戏在 Documents 下的目录名。
pub const GAME_DIR_NAME: &str = "Shadow of the Tomb Raider";
/// 默认枚举深度：根为 0，最多下探 4 层。
pub const DEFAULT_MAX_DEPTH: usize = 4;
/// 默认候选文件数上限（与 CLI 批量上限一致）。
pub const DEFAULT_MAX_FILES: usize = 128;
/// 默认单文件压缩大小上限（与 `sottr-core` 的容器上限一致）。
pub const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// 枚举限制。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryLimits {
    /// 最大目录深度（根为 0；达到上限的目录不再展开但仍会列出其中文件）。
    pub max_depth: usize,
    /// 候选文件数量上限；超过时返回 [`DiscoveryError::TooManyFiles`]。
    pub max_files: usize,
    /// 单文件压缩大小上限；超过的文件仍列出并标记
    /// [`CandidateStatus::TooLarge`]（分析会以 `RESOURCE_LIMIT` 拒绝）。
    pub max_file_bytes: u64,
}

impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            max_files: DEFAULT_MAX_FILES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

/// Documents 候选目录的来源（证据标签，供界面与诊断说明）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentsSource {
    /// Windows Known Folder（已包含 OneDrive 重定向）。
    KnownFolder,
    /// `%USERPROFILE%\Documents` 回退候选。
    UserProfile,
    /// OneDrive 环境变量（变量名随附）。
    OneDriveEnv(String),
}

impl DocumentsSource {
    /// 供界面显示的中文来源标签。
    pub fn label(&self) -> String {
        match self {
            DocumentsSource::KnownFolder => {
                "Windows 文档位置（Known Folder，含 OneDrive 重定向）".to_string()
            }
            DocumentsSource::UserProfile => "%USERPROFILE%\\Documents（回退候选）".to_string(),
            DocumentsSource::OneDriveEnv(variable) => {
                format!("OneDrive 环境变量 {variable}（\\Documents）")
            }
        }
    }
}

/// 一个 Documents 候选目录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentsDir {
    /// 候选路径（原样保留，不解析链接；比较时另行规范）。
    pub path: PathBuf,
    /// 来源标签。
    pub source: DocumentsSource,
}

/// Windows Known Folder 的 Documents（含重定向）；不可用时为 `None`。
pub fn documents_known_folder() -> Option<PathBuf> {
    crate::windows_known_folder::documents()
}

/// 默认 Documents 候选（按优先级排序，未去重；去重在扫描时按规范路径进行）。
pub fn default_documents_dirs() -> Vec<DocumentsDir> {
    let mut candidates = Vec::new();
    if let Some(path) = documents_known_folder() {
        candidates.push(DocumentsDir {
            path,
            source: DocumentsSource::KnownFolder,
        });
    }
    if let Some(profile) = non_empty_env("USERPROFILE") {
        candidates.push(DocumentsDir {
            path: PathBuf::from(profile).join("Documents"),
            source: DocumentsSource::UserProfile,
        });
    }
    for variable in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(value) = non_empty_env(variable) {
            candidates.push(DocumentsDir {
                path: PathBuf::from(value).join("Documents"),
                source: DocumentsSource::OneDriveEnv(variable.to_string()),
            });
        }
    }
    candidates
}

fn non_empty_env(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

/// 单个候选 Documents 目录的扫描状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentsScanStatus {
    /// 发现游戏目录（路径为 `Documents\Shadow of the Tomb Raider`）。
    GameFound(PathBuf),
    /// Documents 目录存在，但没有游戏目录（未安装或未使用该位置）。
    NoGameDir,
    /// Documents 目录本身就是符号链接/重解析点：发现层不跟随。
    LinkNotFollowed,
    /// 目录不存在。
    Missing,
    /// 路径存在但不是目录（异常布局）。
    NotDirectory,
    /// 读取失败（权限等）；不尝试提权或修改权限。
    Unreadable(String),
    /// UNC 或设备命名空间位置：明确不支持。
    UnsupportedLocation,
}

/// 一个候选目录的扫描结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentsScan {
    /// 候选目录（含来源）。
    pub dir: DocumentsDir,
    /// 扫描状态。
    pub status: DocumentsScanStatus,
}

/// 扫描候选 Documents 目录（可注入测试根目录）；同一路径按首个来源去重。
pub fn scan_documents(candidates: &[DocumentsDir]) -> Vec<DocumentsScan> {
    let mut seen: Vec<String> = Vec::new();
    let mut scans = Vec::new();
    for candidate in candidates {
        let key = path_key(&candidate.path);
        if seen.iter().any(|existing| existing == &key) {
            continue;
        }
        seen.push(key);
        scans.push(DocumentsScan {
            dir: candidate.clone(),
            status: scan_one(&candidate.path),
        });
    }
    scans
}

/// 使用默认候选扫描（Known Folder → `%USERPROFILE%\Documents` → OneDrive 环境变量）。
pub fn scan_default_documents() -> Vec<DocumentsScan> {
    scan_documents(&default_documents_dirs())
}

/// 从扫描结果取出可用游戏目录（保持候选顺序；多目录时由用户选择）。
pub fn game_roots(scans: &[DocumentsScan]) -> Vec<PathBuf> {
    scans
        .iter()
        .filter_map(|scan| match &scan.status {
            DocumentsScanStatus::GameFound(path) => Some(path.clone()),
            _ => None,
        })
        .collect()
}

fn scan_one(documents: &Path) -> DocumentsScanStatus {
    match path_policy::classify(documents) {
        PathClass::Unc | PathClass::Device => return DocumentsScanStatus::UnsupportedLocation,
        PathClass::Local | PathClass::VerbatimLocal => {}
    }
    match fs::metadata(documents) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return DocumentsScanStatus::Missing;
        }
        Err(error) => return DocumentsScanStatus::Unreadable(error.to_string()),
        Ok(metadata) if !metadata.is_dir() => return DocumentsScanStatus::NotDirectory,
        Ok(_) => {}
    }
    let game = documents.join(GAME_DIR_NAME);
    match fs::symlink_metadata(&game) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => DocumentsScanStatus::NoGameDir,
        Err(error) => DocumentsScanStatus::Unreadable(error.to_string()),
        Ok(link_metadata) if link_metadata.file_type().is_symlink() => {
            DocumentsScanStatus::LinkNotFollowed
        }
        Ok(_) => match fs::metadata(&game) {
            Ok(metadata) if metadata.is_dir() => match fs::read_dir(&game) {
                Ok(_) => DocumentsScanStatus::GameFound(game),
                Err(error) => DocumentsScanStatus::Unreadable(error.to_string()),
            },
            Ok(_) => DocumentsScanStatus::NoGameDir,
            Err(error) if error.kind() == io::ErrorKind::NotFound => DocumentsScanStatus::NoGameDir,
            Err(error) => DocumentsScanStatus::Unreadable(error.to_string()),
        },
    }
}

/// 候选文件的读取状态（仅元数据层面，不代表内容已验证）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateStatus {
    /// 可进入分析（读取与稳定快照仍按 SX-013 校验）。
    Ready,
    /// 超过单文件压缩上限：仍列出；分析会以 `RESOURCE_LIMIT` 拒绝该文件。
    TooLarge,
    /// 元数据读取失败（权限等）；不尝试提权。
    Unreadable(String),
}

/// 单个候选文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFile {
    /// 实际文件名（原样显示，不臆定 save 数字的自动/手动语义）。
    pub name: String,
    /// 完整路径（原样拼接，不解析链接）。
    pub path: PathBuf,
    /// 压缩文件大小；元数据不可读时为 `None`。
    pub size_bytes: Option<u64>,
    /// 修改时间（Unix 毫秒，原始 UTC）；不可用或早于 1970 时为 `None`。
    pub modified_millis: Option<u128>,
    /// 是否匹配 `profile.dat` / `saveN.dat` 观察模式。
    ///
    /// 这只是发现层的弱上下文提示，**不是模式识别证明**：挑战古墓等文件被改名后
    /// 仍可能匹配，模式适用性以显式声明与结构观察为准。
    pub matches_observed_names: bool,
    /// 读取状态。
    pub status: CandidateStatus,
}

/// 被跳过的目录项原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkippedReason {
    /// 符号链接或重解析点：发现层不跟随（避免越出受限范围）。
    LinkOrReparsePoint,
    /// 目录不可读（权限等）；不尝试提权。
    Unreadable(String),
}

/// 被跳过的目录项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedPath {
    /// 被跳过的路径。
    pub path: PathBuf,
    /// 跳过原因。
    pub reason: SkippedReason,
}

/// 候选文件列表结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateListing {
    /// 枚举根目录（原样回显）。
    pub root: PathBuf,
    /// 候选文件（按名称稳定排序）。
    pub files: Vec<CandidateFile>,
    /// 被跳过的目录项（符号链接/重解析点、不可读条目）。
    pub skipped: Vec<SkippedPath>,
    /// 达到深度上限未展开的目录。
    pub depth_limited_dirs: Vec<PathBuf>,
}

/// 目录发现错误（输入层；调用方不得在错误时伪造空结果）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// 根目录不存在。
    RootMissing {
        /// 根路径。
        path: PathBuf,
    },
    /// 根路径不是目录。
    RootNotDirectory {
        /// 根路径。
        path: PathBuf,
    },
    /// 根目录无法读取（权限等）；不尝试提权。
    RootUnreadable {
        /// 根路径。
        path: PathBuf,
        /// 底层错误说明。
        message: String,
    },
    /// UNC 或设备命名空间位置：明确不支持。
    UnsupportedLocation {
        /// 根路径。
        path: PathBuf,
    },
    /// 候选文件数量超过上限（`found` 为触发时的计数，至少 `limit + 1`）。
    TooManyFiles {
        /// 上限。
        limit: usize,
        /// 触发计数（至少 `limit + 1`）。
        found: usize,
    },
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::RootMissing { path } => {
                write!(formatter, "候选目录不存在：{}", path.display())
            }
            DiscoveryError::RootNotDirectory { path } => {
                write!(formatter, "候选路径不是目录：{}", path.display())
            }
            DiscoveryError::RootUnreadable { path, message } => {
                write!(
                    formatter,
                    "候选目录无法读取（权限等）：{}（{message}）；本工具不会提权",
                    path.display()
                )
            }
            DiscoveryError::UnsupportedLocation { path } => {
                write!(
                    formatter,
                    "暂不支持网络（UNC）或设备命名空间位置：{}；请选择本地磁盘上的存档目录",
                    path.display()
                )
            }
            DiscoveryError::TooManyFiles { limit, found } => {
                write!(
                    formatter,
                    "候选文件超过上限 {limit}（至少 {found} 个）；请选择更具体的目录"
                )
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// 枚举候选文件：受深度、数量与大小限制；跳过符号链接与重解析点。
///
/// 只列出 `*.dat`（扩展名大小写不敏感）；文件按名称（大小写不敏感、再按原样）
/// 稳定排序，不依赖目录顺序，也不臆定 save 数字的槽位语义。
pub fn list_candidates(
    root: &Path,
    limits: &DiscoveryLimits,
) -> Result<CandidateListing, DiscoveryError> {
    match path_policy::classify(root) {
        PathClass::Unc | PathClass::Device => {
            return Err(DiscoveryError::UnsupportedLocation {
                path: root.to_path_buf(),
            });
        }
        PathClass::Local | PathClass::VerbatimLocal => {}
    }
    let metadata = fs::metadata(root).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => DiscoveryError::RootMissing {
            path: root.to_path_buf(),
        },
        _ => DiscoveryError::RootUnreadable {
            path: root.to_path_buf(),
            message: error.to_string(),
        },
    })?;
    if !metadata.is_dir() {
        return Err(DiscoveryError::RootNotDirectory {
            path: root.to_path_buf(),
        });
    }

    let mut listing = CandidateListing {
        root: root.to_path_buf(),
        files: Vec::new(),
        skipped: Vec::new(),
        depth_limited_dirs: Vec::new(),
    };
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0));
    while let Some((dir, depth)) = queue.pop_front() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if depth == 0 => {
                return Err(DiscoveryError::RootUnreadable {
                    path: dir,
                    message: error.to_string(),
                });
            }
            Err(error) => {
                listing.skipped.push(SkippedPath {
                    path: dir,
                    reason: SkippedReason::Unreadable(error.to_string()),
                });
                continue;
            }
        };
        let mut children: Vec<(PathBuf, fs::FileType)> = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    listing.skipped.push(SkippedPath {
                        path: dir.clone(),
                        reason: SkippedReason::Unreadable(error.to_string()),
                    });
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    listing.skipped.push(SkippedPath {
                        path,
                        reason: SkippedReason::Unreadable(error.to_string()),
                    });
                    continue;
                }
            };
            if file_type.is_symlink() {
                listing.skipped.push(SkippedPath {
                    path,
                    reason: SkippedReason::LinkOrReparsePoint,
                });
                continue;
            }
            children.push((path, file_type));
        }
        children.sort_by_key(|entry| sort_key(&entry.0));
        for (path, file_type) in children {
            if file_type.is_dir() {
                if depth < limits.max_depth {
                    queue.push_back((path, depth + 1));
                } else {
                    listing.depth_limited_dirs.push(path);
                }
            } else if file_type.is_file() && is_dat_file(&path) {
                if listing.files.len() >= limits.max_files {
                    return Err(DiscoveryError::TooManyFiles {
                        limit: limits.max_files,
                        found: listing.files.len() + 1,
                    });
                }
                listing.files.push(candidate_file(path, limits));
            }
        }
    }
    listing.files.sort_by_key(|file| {
        (
            file.name.to_lowercase(),
            file.name.clone(),
            sort_key(&file.path),
        )
    });
    listing
        .skipped
        .sort_by_key(|skipped| sort_key(&skipped.path));
    listing
        .depth_limited_dirs
        .sort_by_key(|path| sort_key(path));
    Ok(listing)
}

fn candidate_file(path: PathBuf, limits: &DiscoveryLimits) -> CandidateFile {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    match fs::metadata(&path) {
        Ok(metadata) => {
            let size = metadata.len();
            CandidateFile {
                matches_observed_names: matches_observed_name(&name),
                name,
                path,
                size_bytes: Some(size),
                modified_millis: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis()),
                status: if size > limits.max_file_bytes {
                    CandidateStatus::TooLarge
                } else {
                    CandidateStatus::Ready
                },
            }
        }
        Err(error) => CandidateFile {
            matches_observed_names: matches_observed_name(&name),
            name,
            path,
            size_bytes: None,
            modified_millis: None,
            status: CandidateStatus::Unreadable(error.to_string()),
        },
    }
}

fn is_dat_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dat"))
}

/// 观察到的存档文件名模式：`profile.dat` 或 `save<数字>.dat`（大小写不敏感）。
///
/// 与 Python v0.2 的 `SAVE_NAME` 正则同义；仅用于发现层提示，不是模式识别。
fn matches_observed_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower == "profile.dat" {
        return true;
    }
    match lower
        .strip_prefix("save")
        .and_then(|rest| rest.strip_suffix(".dat"))
    {
        Some(number) => !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()),
        None => false,
    }
}

fn sort_key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

/// 去重键：优先规范路径（解析链接与真实大小写），失败时退回字面路径。
fn path_key(path: &Path) -> String {
    let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    resolved.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sottr-platform-discovery-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn observed_names_match_python_reference_pattern() {
        for name in [
            "profile.dat",
            "Profile.DAT",
            "save1.dat",
            "SAVE202.DAT",
            "save0.dat",
            "save999.dat",
        ] {
            assert!(matches_observed_name(name), "{name}");
        }
        for name in [
            "challengetomb.dat",
            "save.dat",
            "save1.txt",
            "saves1.dat",
            "save1x.dat",
            "",
        ] {
            assert!(!matches_observed_name(name), "{name}");
        }
    }

    #[test]
    fn scan_documents_reports_distinct_statuses() {
        let root = temp_dir("scan");
        let found = root.join("docs-found");
        fs::create_dir_all(found.join(GAME_DIR_NAME)).unwrap();
        let empty = root.join("docs-empty");
        fs::create_dir_all(&empty).unwrap();
        let missing = root.join("docs-missing");
        let file_instead = root.join("docs-file");
        fs::write(&file_instead, b"x").unwrap();

        let candidates = vec![
            DocumentsDir {
                path: found.clone(),
                source: DocumentsSource::KnownFolder,
            },
            DocumentsDir {
                path: empty.clone(),
                source: DocumentsSource::UserProfile,
            },
            DocumentsDir {
                path: missing.clone(),
                source: DocumentsSource::OneDriveEnv("OneDrive".to_string()),
            },
            DocumentsDir {
                path: file_instead.clone(),
                source: DocumentsSource::UserProfile,
            },
        ];
        let scans = scan_documents(&candidates);
        assert_eq!(scans.len(), 4);
        assert_eq!(
            scans[0].status,
            DocumentsScanStatus::GameFound(found.join(GAME_DIR_NAME))
        );
        assert_eq!(scans[1].status, DocumentsScanStatus::NoGameDir);
        assert_eq!(scans[2].status, DocumentsScanStatus::Missing);
        assert_eq!(scans[3].status, DocumentsScanStatus::NotDirectory);
        assert_eq!(game_roots(&scans), vec![found.join(GAME_DIR_NAME)]);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn scan_documents_dedupes_same_path_and_case_variants() {
        let root = temp_dir("dedupe");
        let docs = root.join("Docs");
        fs::create_dir_all(&docs).unwrap();
        let candidates = vec![
            DocumentsDir {
                path: docs.clone(),
                source: DocumentsSource::KnownFolder,
            },
            DocumentsDir {
                path: docs.clone(),
                source: DocumentsSource::UserProfile,
            },
            DocumentsDir {
                path: root.join("docs"),
                source: DocumentsSource::OneDriveEnv("OneDrive".to_string()),
            },
        ];
        let scans = scan_documents(&candidates);
        assert_eq!(scans.len(), 1, "同一路径（含大小写变体）只扫描一次");
        assert_eq!(scans[0].dir.source, DocumentsSource::KnownFolder);

        // 都不存在时按字面路径去重（canonicalize 失败回退）。
        let candidates = vec![
            DocumentsDir {
                path: root.join("Ghost").join("Documents"),
                source: DocumentsSource::KnownFolder,
            },
            DocumentsDir {
                path: root.join("ghost").join("documents"),
                source: DocumentsSource::UserProfile,
            },
        ];
        assert_eq!(scan_documents(&candidates).len(), 1);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn list_candidates_sorts_flags_and_filters() {
        let root = temp_dir("list");
        write(&root.join("save10.dat"), b"x");
        write(&root.join("SAVE9.DAT"), b"x");
        write(&root.join("Profile.DAT"), b"x");
        write(&root.join("notes.txt"), b"x");
        write(&root.join("challengetomb.dat"), b"x");
        write(&root.join("sub").join("save3.dat"), b"x");

        let listing = list_candidates(&root, &DiscoveryLimits::default()).unwrap();
        let names: Vec<&str> = listing
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "challengetomb.dat",
                "Profile.DAT",
                "save10.dat",
                "save3.dat",
                "SAVE9.DAT"
            ]
        );
        let flags: Vec<bool> = listing
            .files
            .iter()
            .map(|file| file.matches_observed_names)
            .collect();
        assert_eq!(flags, vec![false, true, true, true, true]);
        assert!(listing.files.iter().all(|file| file.size_bytes == Some(1)));
        assert!(
            listing
                .files
                .iter()
                .all(|file| file.modified_millis.is_some())
        );
        assert!(listing.skipped.is_empty());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn list_candidates_enforces_depth_count_and_size() {
        let root = temp_dir("limits");
        // 深度：根为 0；depth=4 目录中的文件可见，depth=5 不可见。
        write(&root.join("a/b/c/d/save4.dat"), b"x");
        write(&root.join("a/b/c/d/e/save5.dat"), b"x");
        let listing = list_candidates(&root, &DiscoveryLimits::default()).unwrap();
        let names: Vec<&str> = listing
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect();
        assert_eq!(names, vec!["save4.dat"]);
        assert_eq!(listing.depth_limited_dirs.len(), 1);
        assert!(listing.depth_limited_dirs[0].ends_with("e"));
        fs::remove_dir_all(&root).unwrap();

        // 数量：超过上限报错，不返回部分结果。
        let root = temp_dir("count");
        for index in 0..3 {
            write(&root.join(format!("save{index}.dat")), b"x");
        }
        let limits = DiscoveryLimits {
            max_files: 2,
            ..DiscoveryLimits::default()
        };
        assert_eq!(
            list_candidates(&root, &limits).unwrap_err(),
            DiscoveryError::TooManyFiles { limit: 2, found: 3 }
        );
        fs::remove_dir_all(&root).unwrap();

        // 大小：超限文件仍列出并标记，不读取内容。
        let root = temp_dir("size");
        let big = root.join("big.dat");
        let file = fs::File::create(&big).unwrap();
        file.set_len(100).unwrap();
        drop(file);
        write(&root.join("small.dat"), b"x");
        let limits = DiscoveryLimits {
            max_file_bytes: 50,
            ..DiscoveryLimits::default()
        };
        let listing = list_candidates(&root, &limits).unwrap();
        assert_eq!(listing.files.len(), 2);
        assert_eq!(listing.files[0].name, "big.dat");
        assert_eq!(listing.files[0].size_bytes, Some(100));
        assert_eq!(listing.files[0].status, CandidateStatus::TooLarge);
        assert_eq!(listing.files[1].status, CandidateStatus::Ready);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn list_candidates_reports_explicit_errors() {
        let root = temp_dir("errors");
        assert_eq!(
            list_candidates(&root.join("missing"), &DiscoveryLimits::default()).unwrap_err(),
            DiscoveryError::RootMissing {
                path: root.join("missing")
            }
        );
        let file = root.join("save1.dat");
        write(&file, b"x");
        assert_eq!(
            list_candidates(&file, &DiscoveryLimits::default()).unwrap_err(),
            DiscoveryError::RootNotDirectory { path: file }
        );
        #[cfg(windows)]
        {
            let unc = Path::new(r"\\server\share\saves");
            assert_eq!(
                list_candidates(unc, &DiscoveryLimits::default()).unwrap_err(),
                DiscoveryError::UnsupportedLocation {
                    path: unc.to_path_buf()
                }
            );
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn list_candidates_skips_links_and_reparse_points() {
        let root = temp_dir("links");
        let outside = temp_dir("links-target");
        write(&outside.join("inside.dat"), b"x");
        write(&root.join("save1.dat"), b"x");

        let mut created = Vec::new();
        if std::os::windows::fs::symlink_dir(&outside, root.join("symdir")).is_ok() {
            created.push("symdir");
        }
        let junction = root.join("junction");
        let status = std::process::Command::new("cmd")
            .args([
                "/c",
                "mklink",
                "/J",
                &junction.to_string_lossy(),
                &outside.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if status.is_ok_and(|status| status.success()) {
            created.push("junction");
        }
        if std::os::windows::fs::symlink_file(root.join("save1.dat"), root.join("alias.dat"))
            .is_ok()
        {
            created.push("alias.dat");
        }
        if created.is_empty() {
            eprintln!("链接创建不可用，跳过链接分支");
            fs::remove_dir_all(&root).unwrap();
            fs::remove_dir_all(&outside).unwrap();
            return;
        }

        let listing = list_candidates(&root, &DiscoveryLimits::default()).unwrap();
        let names: Vec<&str> = listing
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect();
        assert_eq!(names, vec!["save1.dat"], "链接与链接内文件不得成为候选");
        for name in &created {
            let path = root.join(name);
            assert!(
                listing.skipped.iter().any(|skipped| skipped.path == path
                    && skipped.reason == SkippedReason::LinkOrReparsePoint),
                "应记录跳过：{name}"
            );
        }

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }
}
