//! 资源限制：有界解压、循环与列表。
//!
//! 沿用研究包的安全上限作为默认值（压缩/解压各 64 MiB、主流区段最多 50,000），
//! 并保留可评审调整的空间；但任何取值都必须为正，不能取消有界性。
//! 硬上限（[`HARD_MAX_*`]）用于拦截明显失控的配置，评审后可调整。

use std::fmt;

/// 默认单文件压缩字节上限：64 MiB。
pub const DEFAULT_MAX_COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
/// 默认单文件解压字节上限：64 MiB。
pub const DEFAULT_MAX_EXPANDED_BYTES: u64 = 64 * 1024 * 1024;
/// 默认主流区段数量上限。
pub const DEFAULT_MAX_SECTIONS: u32 = 50_000;

/// 硬上限：压缩字节。
pub const HARD_MAX_COMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;
/// 硬上限：解压字节。
pub const HARD_MAX_EXPANDED_BYTES: u64 = 1024 * 1024 * 1024;
/// 硬上限：区段数量。
pub const HARD_MAX_SECTIONS: u32 = 1_000_000;

/// 资源限制配置错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitsError {
    /// 压缩字节上限必须为正。
    ZeroCompressedLimit,
    /// 解压字节上限必须为正。
    ZeroExpandedLimit,
    /// 区段上限必须为正。
    ZeroSectionLimit,
    /// 压缩字节上限超过硬上限。
    CompressedLimitAboveCeiling,
    /// 解压字节上限超过硬上限。
    ExpandedLimitAboveCeiling,
    /// 区段上限超过硬上限。
    SectionLimitAboveCeiling,
}

impl fmt::Display for LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            LimitsError::ZeroCompressedLimit => "max_compressed_bytes 必须为正",
            LimitsError::ZeroExpandedLimit => "max_expanded_bytes 必须为正",
            LimitsError::ZeroSectionLimit => "max_sections 必须为正",
            LimitsError::CompressedLimitAboveCeiling => "max_compressed_bytes 超过硬上限",
            LimitsError::ExpandedLimitAboveCeiling => "max_expanded_bytes 超过硬上限",
            LimitsError::SectionLimitAboveCeiling => "max_sections 超过硬上限",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LimitsError {}

/// 一次分析使用的资源限制。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// 单文件压缩字节上限。
    pub max_compressed_bytes: u64,
    /// 单文件解压字节上限。
    pub max_expanded_bytes: u64,
    /// 主流区段数量上限。
    pub max_sections: u32,
}

impl Limits {
    /// 校验并构造限制；为零或超过硬上限都会返回错误。
    pub fn new(
        max_compressed_bytes: u64,
        max_expanded_bytes: u64,
        max_sections: u32,
    ) -> Result<Self, LimitsError> {
        if max_compressed_bytes == 0 {
            return Err(LimitsError::ZeroCompressedLimit);
        }
        if max_expanded_bytes == 0 {
            return Err(LimitsError::ZeroExpandedLimit);
        }
        if max_sections == 0 {
            return Err(LimitsError::ZeroSectionLimit);
        }
        if max_compressed_bytes > HARD_MAX_COMPRESSED_BYTES {
            return Err(LimitsError::CompressedLimitAboveCeiling);
        }
        if max_expanded_bytes > HARD_MAX_EXPANDED_BYTES {
            return Err(LimitsError::ExpandedLimitAboveCeiling);
        }
        if max_sections > HARD_MAX_SECTIONS {
            return Err(LimitsError::SectionLimitAboveCeiling);
        }
        Ok(Self {
            max_compressed_bytes,
            max_expanded_bytes,
            max_sections,
        })
    }
}

impl Default for Limits {
    /// 方案默认值：压缩/解压各 64 MiB、区段最多 50,000。
    fn default() -> Self {
        Self {
            max_compressed_bytes: DEFAULT_MAX_COMPRESSED_BYTES,
            max_expanded_bytes: DEFAULT_MAX_EXPANDED_BYTES,
            max_sections: DEFAULT_MAX_SECTIONS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_plan() {
        let limits = Limits::default();
        assert_eq!(limits.max_compressed_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.max_expanded_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.max_sections, 50_000);
        assert!(
            Limits::new(
                limits.max_compressed_bytes,
                limits.max_expanded_bytes,
                limits.max_sections
            )
            .is_ok()
        );
    }

    #[test]
    fn zero_limits_are_rejected() {
        assert_eq!(Limits::new(0, 1, 1), Err(LimitsError::ZeroCompressedLimit));
        assert_eq!(Limits::new(1, 0, 1), Err(LimitsError::ZeroExpandedLimit));
        assert_eq!(Limits::new(1, 1, 0), Err(LimitsError::ZeroSectionLimit));
    }

    #[test]
    fn above_hard_ceiling_is_rejected() {
        assert_eq!(
            Limits::new(HARD_MAX_COMPRESSED_BYTES + 1, 1, 1),
            Err(LimitsError::CompressedLimitAboveCeiling)
        );
        assert_eq!(
            Limits::new(1, HARD_MAX_EXPANDED_BYTES + 1, 1),
            Err(LimitsError::ExpandedLimitAboveCeiling)
        );
        assert_eq!(
            Limits::new(1, 1, HARD_MAX_SECTIONS + 1),
            Err(LimitsError::SectionLimitAboveCeiling)
        );
    }
}
