//! 导出适配（SX-011 / SX-013）：快照时钟。
//!
//! 核心报告不接触文件系统；本模块只保留 CLI 侧的快照时钟（RFC 3339 UTC）。
//! 目标与输入的重叠校验在 `sottr_platform::path_policy`（SX-012），独占新目标
//! 与受控临时文件提交在 `sottr_platform::output_commit`（SX-013），二者与桌面
//! 后端共用；稳定快照读取在 `sottr_platform::read_snapshot`（SX-013）。
//! 本模块不再包含写盘逻辑。

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
