//! 导出适配（SX-011）：目标校验、快照时钟与两阶段提交。
//!
//! 核心报告不接触文件系统；本模块承担 CLI 侧的写盘边界：
//! - 目标必须不存在；输入与输出不得重叠（输出不得位于输入所在的目录树内，
//!   也不得包含输入文件），比较使用规范路径而非字符串前缀；
//! - 所有产物先写 `<名称>.tmp`，全部成功后再逐一改名提交；任何失败或取消
//!   （未提交即丢弃）都会清理临时文件与本次创建的目录，不留下“完整报告”假象；
//! - 快照时间由本模块生成（RFC 3339 UTC，毫秒精度）；核心只接收字符串。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 当前 Unix 毫秒（时钟异常时回退 0）。
pub fn unix_millis_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

/// Unix 毫秒 → RFC 3339 UTC（`YYYY-MM-DDTHH:MM:SS.mmmZ`）。
pub fn format_rfc3339_utc_millis(millis: u128) -> String {
    let seconds = (millis / 1000) as i64;
    let millis_part = (millis % 1000) as u32;
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis_part:03}Z")
}

/// Howard Hinnant 的 civil_from_days（1970-01-01 为 0）。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// 轻量 RFC 3339 校验：`YYYY-MM-DDTHH:MM:SS` + 可选小数秒 + `Z` 或 `±HH:MM`。
pub fn validate_rfc3339(text: &str) -> Result<(), String> {
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
    }
    let digit = |index: usize| -> Result<u32, String> {
        let byte = bytes[index];
        if !byte.is_ascii_digit() {
            return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
        }
        Ok(u32::from(byte - b'0'))
    };
    let number = |start: usize, length: usize| -> Result<u32, String> {
        let mut value = 0;
        for index in start..start + length {
            value = value * 10 + digit(index)?;
        }
        Ok(value)
    };
    let year = number(0, 4)?;
    let month = number(5, 2)?;
    let day = number(8, 2)?;
    if bytes[4] != b'-' || bytes[7] != b'-' || year < 1 || !(1..=12).contains(&month) {
        return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
    }
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
    };
    if day < 1 || day > days_in_month {
        return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
    }
    if !matches!(bytes[10], b'T' | b't') {
        return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
    }
    let hour = number(11, 2)?;
    let minute = number(14, 2)?;
    let second = number(17, 2)?;
    if bytes[13] != b':' || bytes[16] != b':' || hour > 23 || minute > 59 || second > 60 {
        return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
    }
    let mut index = 19;
    if bytes[index] == b'.' {
        index += 1;
        let fraction_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() && index - fraction_start < 9 {
            index += 1;
        }
        if index == fraction_start {
            return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
        }
    }
    match bytes.get(index) {
        Some(b'Z') | Some(b'z') => {
            if index + 1 != bytes.len() {
                return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
            }
            Ok(())
        }
        Some(b'+') | Some(b'-') => {
            if index + 6 != bytes.len() {
                return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
            }
            let offset_hour = number(index + 1, 2)?;
            let offset_minute = number(index + 4, 2)?;
            if bytes[index + 3] != b':' || offset_hour > 23 || offset_minute > 59 {
                return Err(format!("时间必须为 RFC 3339（收到 {text:?}）"));
            }
            Ok(())
        }
        _ => Err(format!("时间必须为 RFC 3339（收到 {text:?}）")),
    }
}

/// 校验导出目标：目录必须不存在，且与输入不存在重叠（按规范路径比较）。
///
/// 规则：输出不得位于输入文件的目录树内（含输入目录本身），输出路径的
/// 已存在祖先必须是目录且不与输入文件重叠；比较使用规范路径，不用字符串前缀。
pub fn validate_export_target(output: &Path, input: &Path) -> Result<(), String> {
    if output.exists() {
        return Err(format!(
            "输出目录已存在，必须使用新目录：{}",
            output.display()
        ));
    }
    let input_canonical = fs::canonicalize(input)
        .map_err(|error| format!("输入路径无法解析（{}）：{error}", input.display()))?;
    let input_dir = input_canonical
        .parent()
        .ok_or_else(|| format!("输入路径没有父目录：{}", input.display()))?
        .to_path_buf();
    let existing = deepest_existing_ancestor(output)
        .ok_or_else(|| format!("输出路径没有可用的父目录：{}", output.display()))?;
    let existing_canonical = fs::canonicalize(&existing)
        .map_err(|error| format!("输出父路径无法解析（{}）：{error}", existing.display()))?;
    if existing_canonical == input_canonical || existing_canonical.starts_with(&input_canonical) {
        return Err(format!(
            "输出不能位于输入文件之内（重叠）：{}",
            output.display()
        ));
    }
    if fs::metadata(&existing_canonical)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        return Err(format!(
            "输出父路径不是目录：{}；请选择新的目录路径",
            existing.display()
        ));
    }
    if existing_canonical == input_dir || existing_canonical.starts_with(&input_dir) {
        return Err(format!(
            "输出不能位于输入所在目录内（重叠）：{}；请选择输入存档目录之外的新目录",
            output.display()
        ));
    }
    Ok(())
}

fn deepest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => current = parent.to_path_buf(),
            _ => return None,
        }
    }
}

/// 两阶段导出：先写全部临时文件，再提交；未提交即丢弃会清理现场。
#[derive(Debug)]
pub struct PendingExport {
    dir: PathBuf,
    temps: Vec<(PathBuf, PathBuf)>,
    committed: bool,
}

impl PendingExport {
    /// 准备导出：目录必须不存在；所有产物写到 `<名称>.tmp`。
    pub fn prepare(dir: &Path, artifacts: &[(String, String)]) -> Result<Self, String> {
        if dir.exists() {
            return Err(format!("输出目录已存在，必须使用新目录：{}", dir.display()));
        }
        fs::create_dir_all(dir)
            .map_err(|error| format!("创建输出目录失败（{}）：{error}", dir.display()))?;
        let mut pending = PendingExport {
            dir: dir.to_path_buf(),
            temps: Vec::new(),
            committed: false,
        };
        for (name, contents) in artifacts {
            let temp = dir.join(format!("{name}.tmp"));
            if let Err(error) = fs::write(&temp, contents.as_bytes()) {
                return Err(format!("写入临时文件失败（{}）：{error}", temp.display()));
            }
            pending.temps.push((temp, dir.join(name)));
        }
        Ok(pending)
    }

    /// 提交：逐一改名到目标名称；改名失败回滚已提交文件并清理，不留假象。
    pub fn commit(mut self) -> Result<Vec<PathBuf>, String> {
        let mut committed_files: Vec<PathBuf> = Vec::new();
        for (temp, target) in &self.temps {
            if let Err(error) = fs::rename(temp, target) {
                for committed in &committed_files {
                    let _ = fs::remove_file(committed);
                }
                return Err(format!("提交导出失败（{}）：{error}", target.display()));
            }
            committed_files.push(target.clone());
        }
        self.committed = true;
        Ok(committed_files)
    }

    /// 取消（等价于丢弃）：清理临时文件与本次创建的目录（测试显式调用；
    /// 生产路径由 Drop 承担同一语义）。
    #[cfg(test)]
    pub fn cancel(self) {
        drop(self);
    }
}

impl Drop for PendingExport {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        for (temp, _) in &self.temps {
            let _ = fs::remove_file(temp);
        }
        // 仅当目录空时移除；best-effort，不掩盖调用方错误。
        let _ = fs::remove_dir(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sottr-export-io-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn utc_formatting_matches_known_instants() {
        assert_eq!(format_rfc3339_utc_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format_rfc3339_utc_millis(951_782_400_000),
            "2000-02-29T00:00:00.000Z"
        );
        assert_eq!(
            format_rfc3339_utc_millis(1_709_164_800_500),
            "2024-02-29T00:00:00.500Z"
        );
        assert_eq!(
            format_rfc3339_utc_millis(1_761_825_600_000),
            "2025-10-30T12:00:00.000Z"
        );
        assert_eq!(
            format_rfc3339_utc_millis(1_792_067_696_789),
            "2026-10-15T12:34:56.789Z"
        );
        assert_eq!(
            format_rfc3339_utc_millis(1_801_872_000_000),
            "2027-02-06T00:00:00.000Z"
        );
    }

    #[test]
    fn rfc3339_validation_accepts_and_rejects() {
        for valid in [
            "2026-10-15T12:34:56Z",
            "2026-10-15T12:34:56.789Z",
            "2026-10-15T12:34:56+08:00",
            "2028-02-29T00:00:00Z",
        ] {
            assert!(validate_rfc3339(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "",
            "2026-10-15",
            "2026-13-15T00:00:00Z",
            "2026-02-29T00:00:00Z",
            "2025-02-29T00:00:00Z",
            "2026-10-15T24:00:00Z",
            "2026-10-15T12:34:56",
            "2026-10-15T12:34:56.Z",
            "2026-10-15T12:34:56+0800",
        ] {
            assert!(validate_rfc3339(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn target_validation_rejects_overlap_and_existing() {
        let root = temp_dir("target");
        fs::create_dir_all(&root).unwrap();
        let input = root.join("save1.dat");
        fs::write(&input, b"x").unwrap();

        // 输出在输入目录内：重叠。
        let inside = root.join("out");
        assert!(validate_export_target(&inside, &input).is_err());
        // 输出目录已存在。
        let existing = root.join("existing");
        fs::create_dir_all(&existing).unwrap();
        assert!(
            validate_export_target(&existing, &input)
                .unwrap_err()
                .contains("已存在")
        );
        // 输入文件之下：重叠。
        let under_input = input.join("child");
        assert!(validate_export_target(&under_input, &input).is_err());
        // 输出父路径是文件：拒绝。
        let under_file = std::env::current_dir()
            .unwrap()
            .join(format!("sottr-export-io-file-{}", std::process::id()));
        fs::write(&under_file, b"x").unwrap();
        assert!(
            validate_export_target(&under_file.join("out"), &input)
                .unwrap_err()
                .contains("不是目录")
        );
        let _ = fs::remove_file(&under_file);
        // 输入目录之外的新目录树：允许。
        let outside = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("sottr-export-io-outside-{}", std::process::id()));
        assert!(validate_export_target(&outside, &input).is_ok());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pending_export_commits_and_cancels_without_leftovers() {
        let dir = temp_dir("commit");
        let artifacts = vec![
            ("report.json".to_string(), "{\"a\":1}\n".to_string()),
            ("report.csv".to_string(), "h\r\n1\r\n".to_string()),
        ];
        let pending = PendingExport::prepare(&dir, &artifacts).unwrap();
        let files = pending.commit().unwrap();
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
        let pending = PendingExport::prepare(&dir, &artifacts).unwrap();
        pending.cancel();
        assert!(!dir.exists(), "取消后不应留下任何报告文件或目录");
    }

    #[test]
    fn pending_export_write_failure_leaves_no_scene() {
        let dir = temp_dir("fail");
        let artifacts = vec![
            ("report.json".to_string(), "{}".to_string()),
            ("sub/evil".to_string(), "x".to_string()),
        ];
        let error = PendingExport::prepare(&dir, &artifacts).unwrap_err();
        assert!(error.contains("写入临时文件失败"), "{error}");
        assert!(!dir.exists(), "写盘失败后不应留下输出目录");
    }

    #[test]
    fn pending_export_refuses_existing_directory() {
        let dir = temp_dir("exists");
        fs::create_dir_all(&dir).unwrap();
        let error =
            PendingExport::prepare(&dir, &[("a".to_string(), "b".to_string())]).unwrap_err();
        assert!(error.contains("已存在"), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }
}
