//! 主区段流步进与边界检查（SX-004）。
//!
//! 已知布局（仅对 4 MiB 样本家族启用；结构校验不是语义适用性证明）：
//! - `0xBFD0`：u32 小端，主流字节长度；
//! - `0xBFD4`：主流起点；
//! - 每个区段头：低 8 位为观察类型，高 24 位为含头的 DWORD 数量（乘 4 得字节数）；
//! - 主流必须精确走到声明终点；段数量受 [`Limits::max_sections`] 约束；
//! - 通用段最小可步进长度为 4 字节，不把未知段强制为 16 字节头。
//!
//! 所有偏移运算都做溢出与范围检查；失败时保留已步进的部分区段与稳定原因，
//! 但不产生 `valid = true` 的流。

use std::fmt;

use crate::limits::Limits;

/// 4 MiB 样本家族的展开尺寸；
/// `profile.dat` 等其他尺寸不进入战役语义（结构识别与语义适用分开）。
pub const CAMPAIGN_FAMILY_EXPANDED_BYTES: usize = 4 * 1024 * 1024;

/// 主流长度字段偏移。
pub const MAIN_STREAM_LENGTH_OFFSET: usize = 0xBFD0;

/// 主流起点。
pub const MAIN_STREAM_START: usize = 0xBFD4;

/// 通用段最小可步进字节数（研究包基线：4 字节）。
pub const MIN_SECTION_BYTES: u32 = 4;

/// 区段流失败原因；措辞与研究包 Python v0.2 的 `packed_sections` 保持一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionError {
    /// 展开尺寸不属于 4 MiB 样本家族（如 profile 或其他版本）。
    UnrecognizedFamilySize,
    /// 长度字段为零、非 4 字节对齐或终点越界。
    InvalidStreamLength,
    /// 达到段数量上限，或剩余空间不足以容纳段头。
    SectionLimitOrTruncatedHeader,
    /// 段长度为零/小于最小步进，或越过声明流终点。
    InvalidOrOutOfBoundsSection,
}

impl SectionError {
    /// 稳定原因措辞（Python v0.2 对照用）。
    pub fn message(self) -> &'static str {
        match self {
            SectionError::UnrecognizedFamilySize => "Unrecognized sample-family size",
            SectionError::InvalidStreamLength => "Invalid stream length",
            SectionError::SectionLimitOrTruncatedHeader => "Section limit or truncated header",
            SectionError::InvalidOrOutOfBoundsSection => "Invalid or out-of-bounds section size",
        }
    }
}

impl fmt::Display for SectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

/// 一个已步进的区段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    /// 区段头的字节偏移。
    pub offset: u64,
    /// 观察类型（段头低 8 位）。
    pub section_type: u8,
    /// 含头的 DWORD 数量（段头高 24 位）。
    pub dwords: u32,
    /// 含头的字节数。
    pub bytes: u32,
}

/// 主流步进结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionStream {
    /// 长度字段偏移（固定 `0xBFD0`）。
    pub length_offset: u64,
    /// 流起点偏移（固定 `0xBFD4`）。
    pub start_offset: u64,
    /// 声明的字节长度。
    pub byte_length: u32,
    /// 声明终点（起点 + 长度；无效时为 0）。
    pub end_exclusive: u64,
    /// 流是否精确走到声明终点。
    pub valid: bool,
    /// 失败原因；成功为 `None`。
    pub error: Option<SectionError>,
    /// 已步进区段（失败时保留失败前的部分）。
    pub sections: Vec<Section>,
}

impl SectionStream {
    /// 步进主区段流。
    ///
    /// 输入必须是 zlib 解压后的字节；非 4 MiB 展开尺寸返回
    /// [`SectionError::UnrecognizedFamilySize`]，调用方不得据此推断其他格式。
    pub fn walk(data: &[u8], limits: &Limits) -> Self {
        let mut stream = SectionStream {
            length_offset: MAIN_STREAM_LENGTH_OFFSET as u64,
            start_offset: MAIN_STREAM_START as u64,
            byte_length: 0,
            end_exclusive: 0,
            valid: false,
            error: None,
            sections: Vec::new(),
        };

        if data.len() != CAMPAIGN_FAMILY_EXPANDED_BYTES {
            stream.error = Some(SectionError::UnrecognizedFamilySize);
            return stream;
        }

        let length = u32::from_le_bytes(
            data[MAIN_STREAM_LENGTH_OFFSET..MAIN_STREAM_START]
                .try_into()
                .expect("checked family size guarantees the length field"),
        );
        stream.byte_length = length;

        if length == 0 || !length.is_multiple_of(4) {
            stream.error = Some(SectionError::InvalidStreamLength);
            return stream;
        }

        let end = match (MAIN_STREAM_START as u64).checked_add(length as u64) {
            Some(end) if end <= data.len() as u64 => end,
            _ => {
                stream.error = Some(SectionError::InvalidStreamLength);
                return stream;
            }
        };
        stream.end_exclusive = end;

        let mut cursor = MAIN_STREAM_START as u64;
        while cursor < end {
            let header_end = match cursor.checked_add(MIN_SECTION_BYTES as u64) {
                Some(header_end) => header_end,
                None => {
                    stream.error = Some(SectionError::SectionLimitOrTruncatedHeader);
                    return stream;
                }
            };
            if stream.sections.len() as u32 >= limits.max_sections || header_end > end {
                stream.error = Some(SectionError::SectionLimitOrTruncatedHeader);
                return stream;
            }

            let header = u32::from_le_bytes(
                data[cursor as usize..cursor as usize + MIN_SECTION_BYTES as usize]
                    .try_into()
                    .expect("checked bounds guarantee a full header"),
            );
            let dwords = header >> 8;
            let bytes = match dwords.checked_mul(4) {
                Some(bytes) => bytes,
                None => {
                    stream.error = Some(SectionError::InvalidOrOutOfBoundsSection);
                    return stream;
                }
            };
            let section_end = match cursor.checked_add(bytes as u64) {
                Some(section_end) => section_end,
                None => {
                    stream.error = Some(SectionError::InvalidOrOutOfBoundsSection);
                    return stream;
                }
            };
            if bytes < MIN_SECTION_BYTES || section_end > end {
                stream.error = Some(SectionError::InvalidOrOutOfBoundsSection);
                return stream;
            }

            stream.sections.push(Section {
                offset: cursor,
                section_type: (header & 0xFF) as u8,
                dwords,
                bytes,
            });
            cursor = section_end;
        }

        stream.valid = cursor == end;
        if !stream.valid {
            stream.error = Some(SectionError::InvalidOrOutOfBoundsSection);
        }
        stream
    }

    /// 已步进区段数量。
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::SampleBuilder;

    fn walk(data: &[u8]) -> SectionStream {
        SectionStream::walk(data, &Limits::default())
    }

    #[test]
    fn valid_stream_with_sections_reaches_exact_end() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 12);
        builder.push_section(28, 4108);
        builder.push_section(3, 0);
        let data = builder.finish();

        let stream = walk(&data);
        assert!(stream.valid, "{:?}", stream.error);
        assert_eq!(stream.error, None);
        assert_eq!(stream.section_count(), 3);
        assert_eq!(stream.byte_length as usize, 16 + 4112 + 4);
        assert_eq!(
            stream.end_exclusive,
            (MAIN_STREAM_START + stream.byte_length as usize) as u64
        );
        assert_eq!(stream.sections[0].section_type, 1);
        assert_eq!(stream.sections[0].bytes, 16);
        assert_eq!(stream.sections[1].section_type, 28);
        assert_eq!(stream.sections[1].bytes, 4112);
        assert_eq!(stream.sections[1].dwords, 1028);
        assert_eq!(stream.sections[2].bytes, 4);
    }

    #[test]
    fn four_byte_minimal_section_is_steppable() {
        let mut builder = SampleBuilder::new();
        builder.push_section(9, 0);
        let stream = walk(&builder.finish());
        assert!(stream.valid);
        assert_eq!(stream.sections[0].bytes, MIN_SECTION_BYTES);
    }

    #[test]
    fn family_size_guard_rejects_other_sizes() {
        for size in [
            0usize,
            25_600,
            1024 * 1024,
            CAMPAIGN_FAMILY_EXPANDED_BYTES + 4,
        ] {
            let data = vec![0u8; size];
            let stream = walk(&data);
            assert!(!stream.valid);
            assert_eq!(stream.error, Some(SectionError::UnrecognizedFamilySize));
        }
    }

    #[test]
    fn zero_declared_length_is_invalid() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 4);
        let stream = walk(&builder.finish_with_declared_length(0));
        assert!(!stream.valid);
        assert_eq!(stream.error, Some(SectionError::InvalidStreamLength));
    }

    #[test]
    fn unaligned_declared_length_is_invalid() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 4);
        let stream = walk(&builder.finish_with_declared_length(6));
        assert!(!stream.valid);
        assert_eq!(stream.error, Some(SectionError::InvalidStreamLength));
    }

    #[test]
    fn declared_length_beyond_data_is_invalid() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 4);
        let stream = walk(&builder.finish_with_declared_length(0xFFFFFFFC));
        assert!(!stream.valid);
        assert_eq!(stream.error, Some(SectionError::InvalidStreamLength));
    }

    #[test]
    fn overflow_length_field_is_invalid_without_panic() {
        let data = vec![0u8; CAMPAIGN_FAMILY_EXPANDED_BYTES];
        let mut data = data;
        data[MAIN_STREAM_LENGTH_OFFSET..MAIN_STREAM_START]
            .copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        let stream = walk(&data);
        assert!(!stream.valid);
        assert_eq!(stream.error, Some(SectionError::InvalidStreamLength));
    }

    #[test]
    fn tiny_declared_length_is_invalid() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 4);
        let stream = walk(&builder.finish_with_declared_length(2));
        assert!(!stream.valid);
        assert_eq!(stream.error, Some(SectionError::InvalidStreamLength));
    }

    #[test]
    fn zero_sized_section_header_is_invalid() {
        let mut builder = SampleBuilder::new();
        builder.push_raw_header(28);
        let stream = walk(&builder.finish());
        assert!(!stream.valid);
        assert_eq!(
            stream.error,
            Some(SectionError::InvalidOrOutOfBoundsSection)
        );
        assert!(stream.sections.is_empty());
    }

    #[test]
    fn section_overshoot_is_invalid() {
        let mut builder = SampleBuilder::new();
        builder.push_raw_header((1029 << 8) | 28);
        let stream = walk(&builder.finish());
        assert!(!stream.valid);
        assert_eq!(
            stream.error,
            Some(SectionError::InvalidOrOutOfBoundsSection)
        );
    }

    #[test]
    fn endpoint_gap_is_invalid() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        // 声明终点比已写入多一个 DWORD（为零头），该零头是零长度段 → 无效。
        let stream = walk(&builder.finish_with_declared_length(8));
        assert!(!stream.valid);
        assert_eq!(
            stream.error,
            Some(SectionError::InvalidOrOutOfBoundsSection)
        );
        assert_eq!(stream.section_count(), 1, "失败前已步进的部分保留");
    }

    #[test]
    fn section_limit_boundary() {
        let mut ok = SampleBuilder::new();
        for _ in 0..50_000 {
            ok.push_section(2, 0);
        }
        let stream = walk(&ok.finish());
        assert!(stream.valid, "{:?}", stream.error);
        assert_eq!(stream.section_count(), 50_000);

        let mut too_many = SampleBuilder::new();
        for _ in 0..50_001 {
            too_many.push_section(2, 0);
        }
        let stream = walk(&too_many.finish());
        assert!(!stream.valid);
        assert_eq!(
            stream.error,
            Some(SectionError::SectionLimitOrTruncatedHeader)
        );

        let limits = Limits::new(1024 * 1024, 1024 * 1024, 3).expect("valid limits");
        let mut custom = SampleBuilder::new();
        for _ in 0..4 {
            custom.push_section(2, 0);
        }
        let stream = SectionStream::walk(&custom.finish(), &limits);
        assert!(!stream.valid);
        assert_eq!(
            stream.error,
            Some(SectionError::SectionLimitOrTruncatedHeader)
        );
    }

    #[test]
    fn random_headers_never_panic_and_keep_invariants() {
        let mut state = 0x9E3779B9u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for _ in 0..16 {
            let mut data = vec![0u8; CAMPAIGN_FAMILY_EXPANDED_BYTES];
            for index in 0..256 {
                let value = next();
                let offset = MAIN_STREAM_START + index * 4;
                data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            let declared = (next() % 4096) * 4;
            data[MAIN_STREAM_LENGTH_OFFSET..MAIN_STREAM_START]
                .copy_from_slice(&declared.to_le_bytes());

            let stream = walk(&data);
            assert_eq!(
                stream.valid,
                stream.error.is_none(),
                "valid 与 error 必须互为补集"
            );
            if stream.valid {
                let walked: u64 = stream.sections.iter().map(|s| s.bytes as u64).sum();
                assert_eq!(walked, stream.byte_length as u64);
            } else {
                assert!(stream.error.is_some());
            }
        }
    }
}
