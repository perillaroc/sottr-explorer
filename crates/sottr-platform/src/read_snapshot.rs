//! 稳定快照（SX-013）：句柄约束读取与读取前后校验。
//!
//! 读取策略（Windows 为目标平台）：
//!
//! 1. 以 `FILE_SHARE_READ` 打开文件：**读取期间系统拒绝其他句柄的写入、
//!    截断、删除与改名**（共享冲突），读取到的字节因此是一致视图；
//! 2. 从**句柄**读取身份（卷序列号 + 文件索引）、大小与最后写入时间，然后
//!    读取内容；读取前后复核指纹，不一致即 `INPUT_CHANGED`；
//! 3. 读取前后内容摘要复核：同一句柄从头重读一遍并比较 SHA-256；
//! 4. 路径复核：同一路径仍解析到同一文件身份，防止目录/链接在检查后替换；
//! 5. 大小上限来自调用方 [`Limits`]（默认 64 MiB），超限在读取前拒绝。
//!
//! 结果是应用控制的不可变内存快照（[`FileSnapshot`]）：后续分析只读字节，
//! 不再接触原文件；句柄在读取完成后立即释放（不长期占用，也不阻止游戏保存）。
//!
//! 契约与限制：
//!
//! - 失败以输入级错误返回：`INPUT_UNREADABLE`（不存在/权限/被占用，可重试）、
//!   `INPUT_CHANGED`（读取前后不一致）、`RESOURCE_LIMIT`（超限）；
//! - 保留旧报告时必须标记 stale 并显示旧 `input_digest`（桌面接线在 SX-014）；
//!   本模块提供快照摘要（[`FileSnapshot::compressed_sha256`]）供其使用；
//! - 默认**手动刷新**，不监视文件；未来加入文件监视必须复用本快照门禁；
//! - 其他平台只做指纹与摘要复核，不声称同等共享约束；
//! - 残余边界：不阻止其他进程在我们打开**之前**替换文件（读到的即打开时的
//!   内容，由指纹与路径复核保证自洽）；本模块不把路径检查称为绝对隔离。

use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use sottr_core::hash::sha256_hex;
use sottr_core::limits::Limits;
use sottr_core::report::InputError;

/// 文件身份：所在卷与文件系统内索引（Windows：卷序列号 + 文件索引）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileIdentity {
    /// 卷标识（Windows：卷序列号；Unix：设备号）。
    pub volume: u64,
    /// 文件索引（Windows：`nFileIndex`；Unix：inode）。
    pub file_index: u64,
}

/// 读取快照时记录的文件指纹（身份、大小、最后写入时间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFingerprint {
    /// 文件大小（字节）。
    pub size: u64,
    /// 最后写入时间原始值（Windows：FILETIME，100ns；其他平台：Unix 毫秒）。
    pub last_write_time: u64,
    /// 文件身份；平台不可用时为 `None`（此时只比较大小与时间）。
    pub identity: Option<FileIdentity>,
}

/// 稳定读取得到的不可变内存快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    /// 读取到的字节（应用控制的内存副本；后续分析只读此缓冲区）。
    pub bytes: Vec<u8>,
    /// 压缩内容 SHA-256（小写十六进制）；用于 `input_digest` 与旧快照标识。
    pub compressed_sha256: String,
    /// 读取时记录的文件指纹。
    pub fingerprint: FileFingerprint,
    /// 读取期间是否由平台约束拒绝外部写入/删除/改名（Windows 为 `true`）。
    pub write_denied_during_read: bool,
}

/// 稳定读取一个输入文件（见模块文档；上限取自 `limits.max_compressed_bytes`）。
///
/// 稳定输入前后哈希不变；被占用、读取中变化或路径被替换时返回相应的输入级错误，
/// 不产生部分结果，也不写入输入文件。
pub fn read_stable(path: &Path, limits: &Limits) -> Result<FileSnapshot, InputError> {
    let mut file = open_read_constrained(path).map_err(|error| map_open_error(path, &error))?;
    let before = fingerprint_of_file(&file).map_err(|error| map_read_error(path, &error))?;
    if before.size > limits.max_compressed_bytes {
        return Err(InputError::resource_limit(format!(
            "文件 {} 为 {} 字节，超过压缩上限 {}（compressed_size_limit_exceeded）",
            display_name(path),
            before.size,
            limits.max_compressed_bytes
        )));
    }

    let first =
        read_exact_size(&mut file, before.size).map_err(|error| map_snapshot_error(path, error))?;
    let after = fingerprint_of_file(&file).map_err(|error| map_read_error(path, &error))?;
    if before != after {
        return Err(InputError::changed(format!(
            "读取期间文件已变化（身份/大小/时间不一致）：{}",
            path.display()
        )));
    }

    file.seek(SeekFrom::Start(0))
        .map_err(|error| map_read_error(path, &error))?;
    let second =
        read_exact_size(&mut file, before.size).map_err(|error| map_snapshot_error(path, error))?;
    let digest = sha256_hex(&first);
    if digest != sha256_hex(&second) {
        return Err(InputError::changed(format!(
            "读取前后内容摘要不一致：{}",
            path.display()
        )));
    }

    verify_path_identity(path, &before)?;
    Ok(FileSnapshot {
        bytes: first,
        compressed_sha256: digest,
        fingerprint: before,
        write_denied_during_read: cfg!(windows),
    })
}

/// 校验路径仍解析到 `expected` 指纹所指的文件（目录/链接被替换、文件被删除时失败）。
///
/// 只做只读打开与指纹比较；不读取内容。返回 `INPUT_CHANGED` 表示路径不再可信。
pub fn verify_path_identity(path: &Path, expected: &FileFingerprint) -> Result<(), InputError> {
    let file = match open_read_constrained(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(InputError::changed(format!(
                "路径已不存在或不再可解析：{}",
                path.display()
            )));
        }
        Err(error) => return Err(map_open_error(path, &error)),
    };
    let current = fingerprint_of_file(&file).map_err(|error| map_read_error(path, &error))?;
    if let (Some(expected_id), Some(current_id)) = (expected.identity, current.identity) {
        if expected_id != current_id {
            return Err(InputError::changed(format!(
                "路径已不再指向读取的文件：{}",
                path.display()
            )));
        }
        return Ok(());
    }
    if current.size != expected.size || current.last_write_time != expected.last_write_time {
        return Err(InputError::changed(format!(
            "路径指向的文件大小或时间已变化：{}",
            path.display()
        )));
    }
    Ok(())
}

/// 以受限共享模式打开文件：Windows 上只允许后续的只读共享（`FILE_SHARE_READ`），
/// 读取期间拒绝外部写入/删除/改名；其他平台为普通只读打开。
pub(crate) fn open_read_constrained(path: &Path) -> io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_SHARE_READ：允许其他只读打开；拒绝写入/删除/改名（共享冲突）。
        const FILE_SHARE_READ: u32 = 0x0000_0001;
        options.share_mode(FILE_SHARE_READ);
    }
    options.open(path)
}

fn fingerprint_of_file(file: &fs::File) -> io::Result<FileFingerprint> {
    let metadata = file.metadata()?;
    Ok(FileFingerprint {
        size: metadata.len(),
        last_write_time: raw_last_write_time(&metadata),
        identity: crate::windows_file::identity(file).or_else(|| unix_identity(&metadata)),
    })
}

#[cfg(windows)]
fn raw_last_write_time(metadata: &fs::Metadata) -> u64 {
    std::os::windows::fs::MetadataExt::last_write_time(metadata)
}

#[cfg(not(windows))]
fn raw_last_write_time(metadata: &fs::Metadata) -> u64 {
    use std::time::UNIX_EPOCH;
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(unix)]
fn unix_identity(metadata: &fs::Metadata) -> Option<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    Some(FileIdentity {
        volume: metadata.dev(),
        file_index: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn unix_identity(_metadata: &fs::Metadata) -> Option<FileIdentity> {
    None
}

/// 读取期间大小变化的两种方向（都映射为 `INPUT_CHANGED`）。
enum SnapshotReadError {
    /// 底层读取失败。
    Io(io::Error),
    /// 文件比指纹记录得更短（读取中截断）。
    Shrank,
    /// 文件比指纹记录得更长（读取中追加）。
    Grew,
}

fn read_exact_size(file: &mut fs::File, size: u64) -> Result<Vec<u8>, SnapshotReadError> {
    let mut bytes = vec![0u8; size as usize];
    file.read_exact(&mut bytes).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            SnapshotReadError::Shrank
        } else {
            SnapshotReadError::Io(error)
        }
    })?;
    let mut extra = [0u8; 1];
    if file.read(&mut extra).map_err(SnapshotReadError::Io)? != 0 {
        return Err(SnapshotReadError::Grew);
    }
    Ok(bytes)
}

fn map_snapshot_error(path: &Path, error: SnapshotReadError) -> InputError {
    match error {
        SnapshotReadError::Io(error) => map_read_error(path, &error),
        SnapshotReadError::Shrank => InputError::changed(format!(
            "读取期间文件变短（可能被截断）：{}",
            path.display()
        )),
        SnapshotReadError::Grew => InputError::changed(format!(
            "读取期间文件变长（可能被追加）：{}",
            path.display()
        )),
    }
}

fn map_open_error(path: &Path, error: &io::Error) -> InputError {
    // ERROR_SHARING_VIOLATION(32) / ERROR_LOCK_VIOLATION(33)：文件被占用或正在保存。
    if matches!(error.raw_os_error(), Some(32) | Some(33)) {
        return InputError::unreadable(format!(
            "文件可能正在保存或被其他程序占用：{}；请稍后重试（本工具不会提权或强制读取）",
            display_name(path)
        ));
    }
    match error.kind() {
        io::ErrorKind::NotFound => {
            InputError::unreadable(format!("输入路径不存在：{}", path.display()))
        }
        io::ErrorKind::PermissionDenied => InputError::unreadable(format!(
            "输入路径无法读取（权限不足或不是普通文件）：{}；本工具不会提权",
            path.display()
        )),
        _ => InputError::unreadable(format!("读取输入失败：{}（{error}）", path.display())),
    }
}

fn map_read_error(path: &Path, error: &io::Error) -> InputError {
    match error.kind() {
        io::ErrorKind::NotFound => {
            InputError::changed(format!("读取期间输入已不可用：{}", path.display()))
        }
        _ => InputError::unreadable(format!("读取输入失败：{}（{error}）", path.display())),
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sottr-platform-snapshot-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn read_stable_returns_digest_and_leaves_file_unchanged() {
        let root = temp_dir("stable");
        let path = root.join("save1.dat");
        let content = b"stable snapshot bytes";
        write(&path, content);

        let snapshot = read_stable(&path, &Limits::default()).unwrap();
        assert_eq!(snapshot.bytes, content);
        assert_eq!(snapshot.compressed_sha256, sha256_hex(content));
        assert_eq!(snapshot.fingerprint.size, content.len() as u64);
        assert_eq!(snapshot.write_denied_during_read, cfg!(windows));

        // 分析器自身未写入：内容、身份、大小与时间都不变。
        assert_eq!(fs::read(&path).unwrap(), content);
        let after = fingerprint_of_file(&fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(after, snapshot.fingerprint, "读取不得改变文件指纹");

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn read_stable_handles_empty_file() {
        let root = temp_dir("empty");
        let path = root.join("empty.dat");
        write(&path, b"");
        let snapshot = read_stable(&path, &Limits::default()).unwrap();
        assert!(snapshot.bytes.is_empty());
        assert_eq!(
            snapshot.compressed_sha256,
            sha256_hex(b""),
            "空文件摘要必须是合法 SHA-256"
        );
        assert_eq!(snapshot.fingerprint.size, 0);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn read_stable_reports_missing_directory_and_oversize() {
        let root = temp_dir("limits");
        let missing = root.join("missing.dat");
        let error = read_stable(&missing, &Limits::default()).unwrap_err();
        assert_eq!(error.code, "INPUT_UNREADABLE");
        assert!(error.retryable);
        assert!(error.message.contains("不存在"), "{}", error.message);

        // 目录不是普通输入文件。
        let error = read_stable(&root, &Limits::default()).unwrap_err();
        assert_eq!(error.code, "INPUT_UNREADABLE");

        // 超限：从句柄大小判定，不读取内容、不产生正常结果。
        let big = root.join("big.dat");
        let file = fs::File::create(&big).unwrap();
        file.set_len(64 * 1024 * 1024 + 1).unwrap();
        drop(file);
        let error = read_stable(&big, &Limits::default()).unwrap_err();
        assert_eq!(error.code, "RESOURCE_LIMIT");
        assert!(!error.retryable);
        assert!(error.message.contains("压缩上限"), "{}", error.message);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn verify_path_identity_detects_replaced_and_missing_paths() {
        let root = temp_dir("identity");
        let path = root.join("save1.dat");
        write(&path, b"first");
        let fingerprint = fingerprint_of_file(&fs::File::open(&path).unwrap()).unwrap();
        verify_path_identity(&path, &fingerprint).unwrap();

        // 替换：用另一个预先创建的文件改名覆盖（身份与时间必然变化）。
        let replacement = root.join("other.dat");
        write(&replacement, b"replacement");
        fs::rename(&replacement, &path).unwrap();
        let error = verify_path_identity(&path, &fingerprint).unwrap_err();
        assert_eq!(error.code, "INPUT_CHANGED", "{}", error.message);
        assert!(error.retryable);

        // 删除：路径不再存在。
        fs::remove_file(&path).unwrap();
        let error = verify_path_identity(&path, &fingerprint).unwrap_err();
        assert_eq!(error.code, "INPUT_CHANGED");

        fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn constrained_reader_blocks_writers_renames_and_deletes() {
        let root = temp_dir("constrain");
        let path = root.join("save1.dat");
        write(&path, b"protected");
        let mut reader = open_read_constrained(&path).unwrap();

        let attempts: Vec<(&str, io::Result<()>)> = vec![
            (
                "写入/截断",
                fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&path)
                    .map(|_| ()),
            ),
            (
                "追加",
                fs::OpenOptions::new().append(true).open(&path).map(|_| ()),
            ),
            ("删除", fs::remove_file(&path)),
            ("改名", fs::rename(&path, root.join("moved.dat"))),
        ];
        for (label, result) in attempts {
            let error = result.expect_err(&format!("受限读取期间{label}必须被拒绝"));
            assert_eq!(error.raw_os_error(), Some(32), "{label}: {error}");
        }

        // 同行读者可共存，内容不受影响。
        let second = open_read_constrained(&path).unwrap();
        drop(second);
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"protected");
        drop(reader);
        assert_eq!(fs::read(&path).unwrap(), b"protected");

        fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn writer_holding_file_makes_read_report_busy_until_release() {
        let root = temp_dir("busy");
        let path = root.join("save1.dat");
        write(&path, b"content");
        let holder = fs::OpenOptions::new().write(true).open(&path).unwrap();

        let error = read_stable(&path, &Limits::default()).unwrap_err();
        assert_eq!(error.code, "INPUT_UNREADABLE");
        assert!(error.retryable, "被占用必须给出可重试提示");
        assert!(
            error.message.contains("占用") || error.message.contains("保存"),
            "{}",
            error.message
        );

        drop(holder);
        let snapshot = read_stable(&path, &Limits::default()).unwrap();
        assert_eq!(snapshot.bytes, b"content");

        fs::remove_dir_all(&root).unwrap();
    }
}
