//! 路径策略（SX-012）：路径分类与输入/输出重叠校验。
//!
//! 规则（CLI 与桌面后端共用同一实现）：
//!
//! - 分类只做词法判定：本地卷（`C:\…`）、逐字本地路径（`\\?\C:\…`，
//!   `fs::canonicalize` 的输出形式）、UNC（`\\server\share\…`、
//!   `\\?\UNC\…`）与设备命名空间（`\\.\…`、`\\?\GLOBALROOT…` 等）；
//! - UNC 与设备命名空间**明确不支持**：返回带原因的稳定错误，不静默尝试，
//!   也不尝试提权；
//! - 输入与输出不得相等或互为父子关系；比较使用规范路径
//!   （`fs::canonicalize`，解析符号链接/重解析点并返回真实大小写），
//!   按路径组件比较（`Path::starts_with`），不使用字符串前缀；
//! - 输出必须是尚不存在的新目录；本模块只做只读检查，不创建目录、
//!   不修改权限、不提权（创建与提交由调用方负责，见 SX-011 的两阶段写盘）。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 路径分类（词法判定，不做文件系统访问）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathClass {
    /// 本地驱动器路径（`C:\…`）或相对路径。
    Local,
    /// `\\?\X:\…` 逐字本地路径（`fs::canonicalize` 的输出形式）。
    VerbatimLocal,
    /// UNC 网络路径（`\\server\share\…`、`\\?\UNC\server\share\…`）。
    Unc,
    /// 设备命名空间或其他逐字路径（`\\.\…`、`\\?\GLOBALROOT…` 等）。
    Device,
}

/// 路径分类。非 Windows 目标上一切路径按本地处理（产品只发布 Windows 版本）。
pub fn classify(path: &Path) -> PathClass {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(_) => PathClass::Local,
                Prefix::VerbatimDisk(_) => PathClass::VerbatimLocal,
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) => PathClass::Unc,
                Prefix::DeviceNS(_) | Prefix::Verbatim(_) => PathClass::Device,
            },
            _ => PathClass::Local,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        PathClass::Local
    }
}

/// 路径策略错误码（稳定标识；界面与测试不解析自然语言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPolicyCode {
    /// 路径不存在。
    PathMissing,
    /// 无法读取（权限等）；不尝试提权或修改权限。
    PathUnreadable,
    /// 无法解析（无效路径、符号链接环等）。
    PathUnresolvable,
    /// UNC 或设备命名空间位置；明确不支持。
    UnsupportedLocation,
    /// 输出目录已存在（必须使用新目录，不覆盖已有报告或存档）。
    OutputExists,
    /// 输出的已存在祖先不是目录。
    OutputParentNotDirectory,
    /// 输入与输出相等或互为父子关系。
    InputOutputOverlap,
}

impl PathPolicyCode {
    /// 稳定字符串（诊断与日志用）。
    pub fn as_str(self) -> &'static str {
        match self {
            PathPolicyCode::PathMissing => "PATH_MISSING",
            PathPolicyCode::PathUnreadable => "PATH_UNREADABLE",
            PathPolicyCode::PathUnresolvable => "PATH_UNRESOLVABLE",
            PathPolicyCode::UnsupportedLocation => "UNSUPPORTED_LOCATION",
            PathPolicyCode::OutputExists => "OUTPUT_EXISTS",
            PathPolicyCode::OutputParentNotDirectory => "OUTPUT_PARENT_NOT_DIRECTORY",
            PathPolicyCode::InputOutputOverlap => "INPUT_OUTPUT_OVERLAP",
        }
    }
}

/// 路径策略错误：稳定错误码 + 中文说明（含路径）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPolicyError {
    /// 稳定错误码。
    pub code: PathPolicyCode,
    /// 说明（可直接展示给用户）。
    pub message: String,
}

impl PathPolicyError {
    fn new(code: PathPolicyCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PathPolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}（{}）", self.message, self.code.as_str())
    }
}

impl std::error::Error for PathPolicyError {}

/// 校验分析输入路径（文件或目录）：必须存在、位于本地卷且可解析，返回规范路径。
///
/// 符号链接与重解析点：显式选择（手动选文件/文件夹）允许，解析为规范路径后返回；
/// 目录发现不会自动给出链接条目（见 [`crate::discovery`]）。不读取文件内容。
pub fn validate_input(path: &Path) -> Result<PathBuf, PathPolicyError> {
    ensure_local(path)?;
    let canonical = fs::canonicalize(path).map_err(|error| map_input_error(path, &error))?;
    let metadata = fs::metadata(&canonical).map_err(|error| map_input_error(&canonical, &error))?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(PathPolicyError::new(
            PathPolicyCode::UnsupportedLocation,
            format!("输入必须是文件或目录：{}", path.display()),
        ));
    }
    Ok(canonical)
}

/// 校验输出目标：必须是尚不存在的新目录，且与每个输入不相等、不互为父子。
///
/// 比较使用规范路径（解析链接/重解析点与真实大小写）并按路径组件判断；
/// 输出自身不需要存在，但它的最深已存在祖先必须存在且是目录。只读检查：
/// 不创建目录、不写文件、不改权限。
pub fn validate_output(output: &Path, inputs: &[&Path]) -> Result<(), PathPolicyError> {
    if output.as_os_str().is_empty() {
        return Err(PathPolicyError::new(
            PathPolicyCode::PathUnresolvable,
            "输出路径不能为空".to_string(),
        ));
    }
    ensure_local(output)?;
    if output.exists() {
        return Err(PathPolicyError::new(
            PathPolicyCode::OutputExists,
            format!("输出目录已存在，必须使用新目录：{}", output.display()),
        ));
    }
    let existing = deepest_existing_ancestor(output).ok_or_else(|| {
        PathPolicyError::new(
            PathPolicyCode::PathUnresolvable,
            format!("输出路径没有可用的父目录：{}", output.display()),
        )
    })?;
    let existing_canonical = fs::canonicalize(&existing).map_err(|error| {
        PathPolicyError::new(
            PathPolicyCode::PathUnresolvable,
            format!("输出父路径无法解析（{}）：{error}", existing.display()),
        )
    })?;
    // 先判重叠（更具体的安全错误，如“输出位于输入文件之下”），再判父路径类型：
    // 两者都成立时优先报告重叠。
    for input in inputs {
        ensure_disjoint(&existing_canonical, output, input)?;
    }
    let ancestor_is_dir = fs::metadata(&existing_canonical)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false);
    if !ancestor_is_dir {
        return Err(PathPolicyError::new(
            PathPolicyCode::OutputParentNotDirectory,
            format!(
                "输出父路径不是目录：{}；请选择新的目录路径",
                existing.display()
            ),
        ));
    }
    Ok(())
}

fn ensure_local(path: &Path) -> Result<(), PathPolicyError> {
    match classify(path) {
        PathClass::Local | PathClass::VerbatimLocal => Ok(()),
        PathClass::Unc => Err(PathPolicyError::new(
            PathPolicyCode::UnsupportedLocation,
            format!(
                "暂不支持网络（UNC）位置：{}；请选择本地磁盘上的存档路径",
                path.display()
            ),
        )),
        PathClass::Device => Err(PathPolicyError::new(
            PathPolicyCode::UnsupportedLocation,
            format!(
                "暂不支持设备命名空间路径：{}；请选择本地磁盘上的存档路径",
                path.display()
            ),
        )),
    }
}

/// 输入与输出不得相等或互为父子；输出位于输入文件所在目录内也不允许
/// （不把报告与存档混放），与 SX-011 已验收的导出规则一致。
fn ensure_disjoint(
    existing_canonical: &Path,
    output: &Path,
    input: &Path,
) -> Result<(), PathPolicyError> {
    let input_canonical =
        fs::canonicalize(input).map_err(|error| map_input_error(input, &error))?;
    let input_metadata = fs::metadata(&input_canonical)
        .map_err(|error| map_input_error(&input_canonical, &error))?;
    if existing_canonical == input_canonical || existing_canonical.starts_with(&input_canonical) {
        return Err(PathPolicyError::new(
            PathPolicyCode::InputOutputOverlap,
            format!(
                "输出不能位于输入之内或与输入相等（重叠）：{}",
                output.display()
            ),
        ));
    }
    if input_metadata.is_file()
        && let Some(parent) = input_canonical.parent()
        && (existing_canonical == parent || existing_canonical.starts_with(parent))
    {
        return Err(PathPolicyError::new(
            PathPolicyCode::InputOutputOverlap,
            format!(
                "输出不能位于输入所在目录内（重叠）：{}；请选择输入存档目录之外的新目录",
                output.display()
            ),
        ));
    }
    // 输出包含输入的情形无需显式分支：输出必须不存在，而输入的祖先必然存在，
    // 因此“输出是输入祖先”必然触发前面的 OutputExists。
    Ok(())
}

/// 输入路径错误的映射（NotFound 与权限失败分开说明；不尝试提权）。
fn map_input_error(path: &Path, error: &io::Error) -> PathPolicyError {
    match error.kind() {
        io::ErrorKind::NotFound => PathPolicyError::new(
            PathPolicyCode::PathMissing,
            format!("输入路径不存在：{}", path.display()),
        ),
        io::ErrorKind::PermissionDenied => PathPolicyError::new(
            PathPolicyCode::PathUnreadable,
            format!(
                "输入路径无法读取（权限不足）：{}；本工具不会提权或修改权限",
                path.display()
            ),
        ),
        _ => PathPolicyError::new(
            PathPolicyCode::PathUnresolvable,
            format!("输入路径无法解析：{}（{error}）", path.display()),
        ),
    }
}

/// 最深已存在祖先；相对路径在耗尽父组件后落到当前目录（`.`）。
fn deepest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    };
    loop {
        if current.exists() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => current = parent.to_path_buf(),
            _ if current.is_relative() => current = PathBuf::from("."),
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sottr-platform-policy-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(windows)]
    #[test]
    fn classify_distinguishes_local_verbatim_unc_and_device() {
        assert_eq!(classify(Path::new(r"C:\saves\save1.dat")), PathClass::Local);
        assert_eq!(classify(Path::new(r"relative\save1.dat")), PathClass::Local);
        assert_eq!(
            classify(Path::new(r"\\?\C:\saves\save1.dat")),
            PathClass::VerbatimLocal
        );
        assert_eq!(classify(Path::new(r"\\server\share\saves")), PathClass::Unc);
        assert_eq!(
            classify(Path::new(r"\\?\UNC\server\share\saves")),
            PathClass::Unc
        );
        assert_eq!(
            classify(Path::new(r"\\.\PhysicalDrive0")),
            PathClass::Device
        );
        assert_eq!(
            classify(Path::new(r"\\?\GLOBALROOT\Device\HarddiskVolume1")),
            PathClass::Device
        );
    }

    #[test]
    fn validate_input_maps_missing_unreadable_and_accepts_file_dir_and_link() {
        let root = temp_dir("input");
        let file = root.join("save1.dat");
        fs::write(&file, b"x").unwrap();
        let dir = root.join("folder");
        fs::create_dir_all(&dir).unwrap();

        let canonical = validate_input(&file).unwrap();
        assert!(canonical.is_absolute());
        assert!(validate_input(&dir).unwrap().is_absolute());

        let missing = validate_input(&root.join("missing.dat")).unwrap_err();
        assert_eq!(missing.code, PathPolicyCode::PathMissing);
        assert!(missing.message.contains("不存在"), "{}", missing.message);

        // 符号链接：显式选择允许，解析为规范路径（目标存在）。
        #[cfg(windows)]
        {
            match std::os::windows::fs::symlink_file(&file, root.join("alias.dat")) {
                Ok(()) => {
                    let link = root.join("alias.dat");
                    assert_eq!(validate_input(&link).unwrap(), canonical);
                }
                Err(_) => eprintln!("符号链接不可用，跳过链接分支"),
            }
        }

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_output_rejects_existing_parent_file_and_overlaps() {
        let root = temp_dir("output");
        let input_dir = root.join("saves");
        fs::create_dir_all(&input_dir).unwrap();
        let input_file = input_dir.join("save1.dat");
        fs::write(&input_file, b"x").unwrap();

        // 输出已存在。
        let existing = root.join("existing");
        fs::create_dir_all(&existing).unwrap();
        let error = validate_output(&existing, &[&input_file]).unwrap_err();
        assert_eq!(error.code, PathPolicyCode::OutputExists);
        assert!(error.message.contains("已存在"), "{}", error.message);

        // 输出在输入文件所在目录内：重叠。
        let inside = input_dir.join("out");
        let error = validate_output(&inside, &[&input_file]).unwrap_err();
        assert_eq!(error.code, PathPolicyCode::InputOutputOverlap);

        // 输出在输入目录内：重叠。
        let inside_dir = validate_output(&input_dir.join("out"), &[&input_dir]).unwrap_err();
        assert_eq!(inside_dir.code, PathPolicyCode::InputOutputOverlap);
        // 输出与输入目录相等（已存在）。
        assert_eq!(
            validate_output(&input_dir, &[&input_dir]).unwrap_err().code,
            PathPolicyCode::OutputExists
        );

        // 输出在输入文件之下：优先报告重叠（更具体的安全错误）。
        let error = validate_output(&input_file.join("out"), &[&input_file]).unwrap_err();
        assert_eq!(error.code, PathPolicyCode::InputOutputOverlap);

        // 输出父路径是文件（与输入无关）：拒绝。
        let blocker = root.join("blocker");
        fs::write(&blocker, b"x").unwrap();
        let error = validate_output(&blocker.join("out"), &[&input_file]).unwrap_err();
        assert_eq!(error.code, PathPolicyCode::OutputParentNotDirectory);
        assert!(error.message.contains("不是目录"), "{}", error.message);

        // 输入之外的新目录树：允许（同一磁盘上的兄弟目录）。
        let outside = root.join("export");
        validate_output(&outside, &[&input_file]).unwrap();
        validate_output(&root.join("export-dir"), &[&input_dir]).unwrap();

        // 相对输出路径可用（最深已存在祖先落到当前目录）。
        let relative = Path::new("sottr-platform-relative-out");
        assert!(!relative.exists());
        validate_output(relative, &[&input_file]).unwrap();

        fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn validate_output_uses_canonical_paths_not_string_prefixes() {
        let root = temp_dir("canonical");
        let saves = root.join("Saves");
        fs::create_dir_all(&saves).unwrap();
        let input = saves.join("save1.dat");
        fs::write(&input, b"x").unwrap();

        // 大小写不同的同一路径必须视为重叠（规范路径比较）。
        let case_variant = root.join("saves").join("Out");
        let error = validate_output(&case_variant, &[&input]).unwrap_err();
        assert_eq!(error.code, PathPolicyCode::InputOutputOverlap);

        // 字符串前缀相同但不是路径祖先：`Saves2` 不是 `Saves` 的子路径，允许。
        let sibling_prefix = root.join("Saves2");
        validate_output(&sibling_prefix, &[&input]).unwrap();

        fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn validate_input_and_output_reject_unc_and_device_locations() {
        let unc = Path::new(r"\\server\share\save1.dat");
        let error = validate_input(unc).unwrap_err();
        assert_eq!(error.code, PathPolicyCode::UnsupportedLocation);
        assert!(error.message.contains("网络"), "{}", error.message);

        let device = Path::new(r"\\.\PhysicalDrive0");
        assert_eq!(
            validate_input(device).unwrap_err().code,
            PathPolicyCode::UnsupportedLocation
        );

        let root = temp_dir("unc-out");
        let input = root.join("save1.dat");
        fs::write(&input, b"x").unwrap();
        let error = validate_output(Path::new(r"\\server\share\out"), &[&input]).unwrap_err();
        assert_eq!(error.code, PathPolicyCode::UnsupportedLocation);
        fs::remove_dir_all(&root).unwrap();
    }
}
