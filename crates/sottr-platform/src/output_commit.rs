//! 输出提交（SX-013）：独占新目标与受控临时文件。
//!
//! 规则（CLI 与桌面后端共用）：
//!
//! - 输出目录必须不存在：目录及缺失的父目录逐级**独占创建**（`create_dir`），
//!   创建后立即校验不是符号链接/重解析点；目标目录已存在（含并发创建）直接拒绝；
//! - 产物先写 `<名称>.tmp`（`create_new` 独占创建，绝不跟随既有链接或覆盖）；
//! - 提交前逐项复核：输出目录仍不是重解析点、临时文件是普通文件、目标名尚且
//!   不存在；任一复核失败即整体失败并回滚，**绝不回退为“直接写目标名”的不受控
//!   写入**（Windows 上 `fs::rename` 替换的是目录项本身，不写穿符号链接）；
//! - 失败或取消清理临时文件与本次创建的目录（只删除空目录，绝不递归删除，
//!   也绝不删除任何既有文件）。
//!
//! 残余边界：其他进程若在我们创建目录后把该目录整体改名并替换为链接，提交会
//! 失败且不会写入替换后的位置；但留在被改名走目录中的临时文件无法按路径清理。
//! 本模块不声称绝对隔离（普通路径检查不等于内核级隔离）。

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// 输出提交错误码（稳定标识；界面与测试不解析自然语言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputErrorCode {
    /// 目标无效（空路径、非法产物名称）。
    InvalidTarget,
    /// 输出目录、临时文件或目标文件已存在（拒绝覆盖）。
    OutputExists,
    /// 已存在祖先不是目录，或无法创建目录（权限等）。
    ParentNotDirectory,
    /// 输出路径或临时文件是/被替换为符号链接或重解析点。
    LinkReplaced,
    /// 写临时文件失败。
    WriteFailed,
    /// 提交（改名）失败。
    CommitFailed,
}

impl OutputErrorCode {
    /// 稳定字符串（诊断与日志用）。
    pub fn as_str(self) -> &'static str {
        match self {
            OutputErrorCode::InvalidTarget => "OUTPUT_INVALID_TARGET",
            OutputErrorCode::OutputExists => "OUTPUT_EXISTS",
            OutputErrorCode::ParentNotDirectory => "OUTPUT_PARENT_NOT_DIRECTORY",
            OutputErrorCode::LinkReplaced => "OUTPUT_LINK_REPLACED",
            OutputErrorCode::WriteFailed => "OUTPUT_WRITE_FAILED",
            OutputErrorCode::CommitFailed => "OUTPUT_COMMIT_FAILED",
        }
    }
}

/// 输出提交错误：稳定错误码 + 中文说明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputError {
    /// 稳定错误码。
    pub code: OutputErrorCode,
    /// 说明（可直接展示给用户）。
    pub message: String,
}

impl OutputError {
    fn new(code: OutputErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 是否可原样重试（写盘/提交失败可重试；路径与链接问题需用户处理后重试）。
    pub fn retryable(&self) -> bool {
        matches!(
            self.code,
            OutputErrorCode::WriteFailed | OutputErrorCode::CommitFailed
        )
    }
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}（{}）", self.message, self.code.as_str())
    }
}

impl std::error::Error for OutputError {}

/// 已准备但尚未提交的输出：持有临时文件与本次创建的目录记录。
#[derive(Debug)]
pub struct OutputStaging {
    dir: PathBuf,
    /// （临时文件, 目标文件）按产物顺序。
    temps: Vec<(PathBuf, PathBuf)>,
    /// 本次创建的目录（创建顺序：祖先在前）。
    created_dirs: Vec<PathBuf>,
    committed: bool,
}

/// 准备输出：独占创建目录并写入全部 `<名称>.tmp`；失败或未提交即丢弃。
///
/// 产物名必须是简单文件名（不得含路径分隔符或 `..`），防止写到目标目录之外。
pub fn prepare(dir: &Path, artifacts: &[(String, String)]) -> Result<OutputStaging, OutputError> {
    if dir.as_os_str().is_empty() {
        return Err(OutputError::new(
            OutputErrorCode::InvalidTarget,
            "输出路径不能为空",
        ));
    }
    if dir.exists() {
        return Err(OutputError::new(
            OutputErrorCode::OutputExists,
            format!("输出目录已存在，必须使用新目录：{}", dir.display()),
        ));
    }
    for (name, _) in artifacts {
        validate_artifact_name(name)?;
    }

    let created_dirs = create_output_dirs(dir)?;
    let mut staging = OutputStaging {
        dir: dir.to_path_buf(),
        temps: Vec::new(),
        created_dirs,
        committed: false,
    };
    // 任一失败即返回：局部 staging 被丢弃，由 Drop 清理已写临时文件与本次创建的目录。
    staging.write_temps(artifacts)?;
    staging.verify_dir()?;
    Ok(staging)
}

impl OutputStaging {
    /// 提交：逐项复核并改名到目标名；失败回滚已提交项，不留下“完整报告”假象。
    pub fn commit(mut self) -> Result<Vec<PathBuf>, OutputError> {
        let mut committed: Vec<PathBuf> = Vec::new();
        if let Err(error) = self.commit_all(&mut committed) {
            for path in &committed {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        self.committed = true;
        Ok(committed)
    }

    /// 取消（等价于丢弃）：清理临时文件与本次创建的目录（测试显式调用；
    /// 生产路径由 Drop 承担同一语义）。
    #[cfg(test)]
    pub fn cancel(self) {
        drop(self);
    }

    fn write_temps(&mut self, artifacts: &[(String, String)]) -> Result<(), OutputError> {
        for (name, content) in artifacts {
            let temp = self.dir.join(format!("{name}.tmp"));
            let target = self.dir.join(name);
            write_exclusive(&temp, content.as_bytes())?;
            self.temps.push((temp, target));
        }
        Ok(())
    }

    fn commit_all(&mut self, committed: &mut Vec<PathBuf>) -> Result<(), OutputError> {
        self.verify_dir()?;
        for (temp, target) in &self.temps {
            // 每次提交前复核输出目录（防中途被替换为重解析点）。
            self.verify_dir()?;
            if target.symlink_metadata().is_ok() {
                return Err(OutputError::new(
                    OutputErrorCode::OutputExists,
                    format!("输出目标已存在，拒绝覆盖：{}", target.display()),
                ));
            }
            let metadata = temp.symlink_metadata().map_err(|error| {
                OutputError::new(
                    OutputErrorCode::CommitFailed,
                    format!("临时文件不可用（{}）：{error}", temp.display()),
                )
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(OutputError::new(
                    OutputErrorCode::LinkReplaced,
                    format!(
                        "临时文件被替换为链接或非常规文件，拒绝提交：{}",
                        temp.display()
                    ),
                ));
            }
            // Windows 上 fs::rename 替换的是目录项本身，不写穿符号链接。
            fs::rename(temp, target).map_err(|error| {
                OutputError::new(
                    OutputErrorCode::CommitFailed,
                    format!("提交输出失败（{}）：{error}", target.display()),
                )
            })?;
            committed.push(target.clone());
        }
        Ok(())
    }

    /// 复核输出目录仍是普通目录（不是符号链接/重解析点）。
    fn verify_dir(&self) -> Result<(), OutputError> {
        let metadata = fs::symlink_metadata(&self.dir).map_err(|error| {
            OutputError::new(
                OutputErrorCode::CommitFailed,
                format!("输出目录无法读取（{}）：{error}", self.dir.display()),
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(OutputError::new(
                OutputErrorCode::LinkReplaced,
                format!(
                    "输出目录是（或已被替换为）符号链接/重解析点，拒绝写入：{}",
                    self.dir.display()
                ),
            ));
        }
        if !metadata.is_dir() {
            return Err(OutputError::new(
                OutputErrorCode::CommitFailed,
                format!("输出目录已被替换或删除：{}", self.dir.display()),
            ));
        }
        Ok(())
    }
}

impl Drop for OutputStaging {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        for (temp, _) in &self.temps {
            let _ = fs::remove_file(temp);
        }
        // 仅删除本次创建的空目录（逆序，最深的先删）；绝不递归删除、绝不删除既有文件。
        for dir in self.created_dirs.iter().rev() {
            let _ = fs::remove_dir(dir);
        }
    }
}

fn validate_artifact_name(name: &str) -> Result<(), OutputError> {
    let simple = !name.is_empty()
        && !name.contains(['/', '\\'])
        && name != "."
        && name != ".."
        && Path::new(name).file_name() == Some(std::ffi::OsStr::new(name));
    if !simple {
        return Err(OutputError::new(
            OutputErrorCode::InvalidTarget,
            format!("产物名称必须是简单文件名（不得包含路径分隔符或上级引用）：{name:?}"),
        ));
    }
    Ok(())
}

/// 独占创建输出目录及其缺失父目录；返回本次创建的目录（祖先在前）。
fn create_output_dirs(dir: &Path) -> Result<Vec<PathBuf>, OutputError> {
    let missing = missing_ancestors(dir);
    let mut created: Vec<PathBuf> = Vec::new();
    for level in missing.iter().rev() {
        if let Err(error) = create_level(level, dir, &mut created) {
            for path in created.iter().rev() {
                let _ = fs::remove_dir(path);
            }
            return Err(error);
        }
    }
    Ok(created)
}

/// 输出路径及其不存在的祖先（`dir` 在最前，最深缺失祖先在最后）。
fn missing_ancestors(dir: &Path) -> Vec<PathBuf> {
    let mut missing: Vec<PathBuf> = Vec::new();
    let mut current = dir.to_path_buf();
    while !current.exists() {
        missing.push(current.clone());
        match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => current = parent.to_path_buf(),
            _ => break,
        }
    }
    missing
}

fn create_level(
    level: &Path,
    target: &Path,
    created: &mut Vec<PathBuf>,
) -> Result<(), OutputError> {
    match fs::create_dir(level) {
        Ok(()) => created.push(level.to_path_buf()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if level == target {
                return Err(OutputError::new(
                    OutputErrorCode::OutputExists,
                    format!("输出目录已存在，必须使用新目录：{}", target.display()),
                ));
            }
            // 并发创建的父目录：继续，由下面的复核确认它不是链接。
        }
        Err(error) => {
            return Err(OutputError::new(
                OutputErrorCode::ParentNotDirectory,
                format!("无法创建输出目录（{}）：{error}", level.display()),
            ));
        }
    }
    let metadata = fs::symlink_metadata(level).map_err(|error| {
        OutputError::new(
            OutputErrorCode::ParentNotDirectory,
            format!("输出路径无法读取（{}）：{error}", level.display()),
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(OutputError::new(
            OutputErrorCode::LinkReplaced,
            format!(
                "输出路径是符号链接或重解析点，拒绝写入：{}",
                level.display()
            ),
        ));
    }
    if !metadata.is_dir() {
        return Err(OutputError::new(
            OutputErrorCode::OutputExists,
            format!("输出路径已存在且不是目录：{}", level.display()),
        ));
    }
    Ok(())
}

/// `create_new` 独占写入：目标名已存在（含链接）时拒绝，绝不跟随或覆盖。
///
/// 文件一旦由本次调用创建，任何后续失败都会先删除它再返回，避免留下部分产物。
fn write_exclusive(path: &Path, bytes: &[u8]) -> Result<(), OutputError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => OutputError::new(
                OutputErrorCode::OutputExists,
                format!(
                    "输出中已存在同名文件（拒绝覆盖或跟随链接）：{}",
                    path.display()
                ),
            ),
            _ => OutputError::new(
                OutputErrorCode::WriteFailed,
                format!("创建临时文件失败（{}）：{error}", path.display()),
            ),
        })?;
    if let Err(error) = write_and_verify(path, &mut file, bytes) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

fn write_and_verify(path: &Path, file: &mut fs::File, bytes: &[u8]) -> Result<(), OutputError> {
    file.write_all(bytes).map_err(|error| {
        OutputError::new(
            OutputErrorCode::WriteFailed,
            format!("写入临时文件失败（{}）：{error}", path.display()),
        )
    })?;
    file.flush().map_err(|error| {
        OutputError::new(
            OutputErrorCode::WriteFailed,
            format!("刷新临时文件失败（{}）：{error}", path.display()),
        )
    })?;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        OutputError::new(
            OutputErrorCode::WriteFailed,
            format!("临时文件无法复核（{}）：{error}", path.display()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(OutputError::new(
            OutputErrorCode::LinkReplaced,
            format!("临时文件是链接或非常规文件，拒绝使用：{}", path.display()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sottr-platform-output-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn artifacts() -> Vec<(String, String)> {
        vec![
            ("report.json".to_string(), "{\"a\":1}\n".to_string()),
            ("report.csv".to_string(), "h\r\n1\r\n".to_string()),
        ]
    }

    #[test]
    fn prepare_commits_and_cancels_without_leftovers() {
        let dir = temp_dir("commit");
        let staging = prepare(&dir, &artifacts()).unwrap();
        let files = staging.commit().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(
            fs::read_to_string(dir.join("report.json")).unwrap(),
            "{\"a\":1}\n"
        );
        assert!(dir.join("report.csv").is_file());
        assert!(!dir.join("report.json.tmp").exists());
        fs::remove_dir_all(&dir).unwrap();

        // 取消（丢弃）：目录与临时文件都被清理。
        let dir = temp_dir("cancel");
        let staging = prepare(&dir, &artifacts()).unwrap();
        staging.cancel();
        assert!(!dir.exists(), "取消后不应留下任何报告文件或目录");
    }

    #[test]
    fn prepare_refuses_existing_directory_and_invalid_names() {
        let dir = temp_dir("exists");
        fs::create_dir_all(&dir).unwrap();
        let error = prepare(&dir, &[("a".to_string(), "b".to_string())]).unwrap_err();
        assert_eq!(error.code, OutputErrorCode::OutputExists);
        assert!(error.message.contains("已存在"), "{}", error.message);
        fs::remove_dir_all(&dir).unwrap();

        for name in ["", ".", "..", "sub/evil", "sub\\evil", "C:\\evil", "a:b"] {
            let dir = temp_dir("names");
            let error = prepare(&dir, &[(name.to_string(), "x".to_string())]).unwrap_err();
            assert_eq!(error.code, OutputErrorCode::InvalidTarget, "name={name:?}");
            assert!(!dir.exists(), "非法名称不得创建目录：{name:?}");
        }
    }

    #[test]
    fn commit_refuses_target_that_appeared_and_rolls_back() {
        let dir = temp_dir("target-appeared");
        let staging = prepare(&dir, &artifacts()).unwrap();
        // 提交前目标名被占用（任何类型都必须拒绝）。
        fs::write(dir.join("report.csv"), b"occupied").unwrap();
        let error = staging.commit().unwrap_err();
        assert_eq!(error.code, OutputErrorCode::OutputExists);
        assert!(!error.retryable());
        // 回滚：已提交的第一项被移除，第二项保留占位文件内容，临时文件清理。
        assert!(!dir.join("report.json").exists(), "部分提交必须回滚");
        assert_eq!(
            fs::read_to_string(dir.join("report.csv")).unwrap(),
            "occupied",
            "不得覆盖既有文件"
        );
        let entries: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(entries, vec!["report.csv"], "不得留下临时文件");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn commit_creates_and_cleans_parent_chain() {
        let root = temp_dir("chain");
        let dir = root.join("a").join("b").join("out");
        let staging = prepare(&dir, &artifacts()).unwrap();
        staging.cancel();
        assert!(!root.join("a").exists(), "本次创建的父目录应被清理");
        let _ = fs::remove_dir_all(&root);

        let staging = prepare(&dir, &artifacts()).unwrap();
        let files = staging.commit().unwrap();
        assert_eq!(files.len(), 2);
        assert!(dir.join("report.json").is_file());
        fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn commit_refuses_symlink_at_target_name_without_touching_target() {
        let dir = temp_dir("target-link");
        let victim = temp_dir("victim");
        fs::create_dir_all(&victim).unwrap();
        let victim_file = victim.join("original.dat");
        fs::write(&victim_file, b"do-not-touch").unwrap();

        let staging = prepare(&dir, &artifacts()).unwrap();
        match std::os::windows::fs::symlink_file(&victim_file, dir.join("report.json")) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("符号链接不可用，跳过链接分支");
                drop(staging);
                let _ = fs::remove_dir_all(&dir);
                let _ = fs::remove_dir_all(&victim);
                return;
            }
        }
        let error = staging.commit().unwrap_err();
        assert_eq!(
            error.code,
            OutputErrorCode::OutputExists,
            "{}",
            error.message
        );
        assert_eq!(
            fs::read(&victim_file).unwrap(),
            b"do-not-touch",
            "符号链接目标不得被写入"
        );
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&victim);
    }
}
