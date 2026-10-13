//! zlib 容器处理与有界解压（SX-004）。
//!
//! 约束（开发方案 SX-004）：
//! - 从偏移 0 解压，验证 zlib 校验和与流 EOF；
//! - 尾随字节（首流结束后的其余输入，包括串接的第二个流）只记录、不解释、不串接；
//! - 输入与输出尺寸受 [`Limits`] 约束；压缩炸弹在超过输出上限时立即停止，
//!   不会为其分配完整解压内存；
//! - 任何失败都不产生“正常语义值”：失败时 [`Decompression::data`] 为空。
//!
//! 本模块不读磁盘、不接收路径：输入是调用方提供的不可变字节。

use std::fmt;

use flate2::{Decompress, FlushDecompress, Status};

use crate::limits::Limits;

/// 每次喂给解压器的输出缓冲上限（真实展开更大时循环继续）。
const OUTPUT_CHUNK_BYTES: usize = 64 * 1024;

/// 解压失败原因；原因码与研究包 Python v0.2 的字符串保持一致，便于差分对照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecompressionError {
    /// 压缩输入超过限制。
    CompressedLimitExceeded,
    /// zlib 流无效（含校验和不匹配）。
    InvalidStream {
        /// 解压器给出的细节（可能随实现变化，不进入契约枚举）。
        detail: String,
    },
    /// 流被截断或不完整（未到达 EOF）。
    Truncated,
    /// 解压结果超过限制。
    ExpandedLimitExceeded,
}

impl DecompressionError {
    /// 稳定的原因码（与 Python v0.2 的 `state["error"]` 取值一致）。
    pub fn reason_code(&self) -> &'static str {
        match self {
            DecompressionError::CompressedLimitExceeded => "compressed_size_limit_exceeded",
            DecompressionError::InvalidStream { .. } => "invalid_zlib_stream",
            DecompressionError::Truncated => "truncated_or_incomplete_zlib_stream",
            DecompressionError::ExpandedLimitExceeded => "expanded_size_limit_exceeded",
        }
    }

    /// 对应的契约诊断码（`DECOMPRESSION_FAILED` / `RESOURCE_LIMIT`）。
    pub fn contract_code(&self) -> &'static str {
        match self {
            DecompressionError::CompressedLimitExceeded
            | DecompressionError::ExpandedLimitExceeded => "RESOURCE_LIMIT",
            DecompressionError::InvalidStream { .. } | DecompressionError::Truncated => {
                "DECOMPRESSION_FAILED"
            }
        }
    }
}

impl fmt::Display for DecompressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecompressionError::CompressedLimitExceeded => {
                formatter.write_str("compressed input exceeds the configured limit")
            }
            DecompressionError::InvalidStream { detail } => {
                write!(formatter, "invalid zlib stream: {detail}")
            }
            DecompressionError::Truncated => {
                formatter.write_str("zlib stream is truncated or incomplete")
            }
            DecompressionError::ExpandedLimitExceeded => {
                formatter.write_str("expanded output exceeds the configured limit")
            }
        }
    }
}

/// 一次有界解压的完整观察结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decompression {
    /// 首流是否完整有效（到达 EOF 且校验和通过、未超限）。
    pub valid: bool,
    /// 是否到达 zlib 流 EOF。
    pub eof: bool,
    /// 失败原因；成功为 `None`。
    pub error: Option<DecompressionError>,
    /// 压缩输入字节数。
    pub compressed_bytes: u64,
    /// 解压字节数；失败为 0。
    pub expanded_bytes: u64,
    /// 首流结束后的尾随字节数（不解释）。
    pub trailing_bytes: u64,
    /// 尾随字节前 32 字节的十六进制（小写；无尾随为空串）。
    pub trailing_prefix_hex: String,
    /// 解压结果；失败为空。
    pub data: Vec<u8>,
}

impl Decompression {
    fn failure(compressed_bytes: u64, error: DecompressionError, eof: bool) -> Self {
        Self {
            valid: false,
            eof,
            error: Some(error),
            compressed_bytes,
            expanded_bytes: 0,
            trailing_bytes: 0,
            trailing_prefix_hex: String::new(),
            data: Vec::new(),
        }
    }
}

/// 从偏移 0 解压单个 zlib 流，遵守输入/输出限制并记录尾随字节。
pub fn decompress(input: &[u8], limits: &Limits) -> Decompression {
    let compressed_bytes = input.len() as u64;
    if compressed_bytes > limits.max_compressed_bytes {
        return Decompression::failure(
            compressed_bytes,
            DecompressionError::CompressedLimitExceeded,
            false,
        );
    }

    let mut decompressor = Decompress::new(true);
    let mut output: Vec<u8> = Vec::new();
    let mut consumed: usize = 0;
    let limit = limits.max_expanded_bytes;

    loop {
        // 允许输出到 limit + 1：多出的 1 字节用于判定“超过上限”。
        let allowance = limit + 1 - decompressor.total_out();
        let chunk = OUTPUT_CHUNK_BYTES.min(allowance as usize).max(1);
        let mut buffer = vec![0u8; chunk];
        let before_in = decompressor.total_in();
        let before_out = decompressor.total_out();

        let status =
            match decompressor.decompress(&input[consumed..], &mut buffer, FlushDecompress::None) {
                Ok(status) => status,
                Err(error) => {
                    return Decompression::failure(
                        compressed_bytes,
                        DecompressionError::InvalidStream {
                            detail: error.to_string(),
                        },
                        false,
                    );
                }
            };

        let produced = (decompressor.total_out() - before_out) as usize;
        let taken = (decompressor.total_in() - before_in) as usize;
        output.extend_from_slice(&buffer[..produced]);
        consumed += taken;

        if status == Status::StreamEnd {
            break;
        }
        if decompressor.total_out() > limit {
            return Decompression::failure(
                compressed_bytes,
                DecompressionError::ExpandedLimitExceeded,
                false,
            );
        }
        if produced == 0 && taken == 0 {
            // 无任何进展：输入耗尽（截断）或解压器停滞，均按截断处理，避免死循环。
            return Decompression::failure(compressed_bytes, DecompressionError::Truncated, false);
        }
        if consumed >= input.len() && produced == 0 {
            return Decompression::failure(compressed_bytes, DecompressionError::Truncated, false);
        }
    }

    let trailing = &input[consumed..];
    let trailing_prefix_hex = trailing
        .iter()
        .take(32)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    Decompression {
        valid: true,
        eof: true,
        error: None,
        compressed_bytes,
        expanded_bytes: output.len() as u64,
        trailing_bytes: trailing.len() as u64,
        trailing_prefix_hex,
        data: output,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::compress;

    fn small_limits(max_expanded: u64) -> Limits {
        Limits::new(1024 * 1024, max_expanded, 50_000).expect("valid limits")
    }

    #[test]
    fn round_trip_small() {
        let raw = b"hello zlib".repeat(64);
        let result = decompress(&compress(&raw), &Limits::default());
        assert!(result.valid, "{:?}", result.error);
        assert!(result.eof);
        assert_eq!(result.error, None);
        assert_eq!(result.data, raw);
        assert_eq!(result.expanded_bytes, raw.len() as u64);
        assert_eq!(result.trailing_bytes, 0);
        assert_eq!(result.trailing_prefix_hex, "");
    }

    #[test]
    fn empty_stream_expands_to_nothing() {
        let result = decompress(&compress(&[]), &Limits::default());
        assert!(result.valid);
        assert!(result.eof);
        assert!(result.data.is_empty());
        assert_eq!(result.expanded_bytes, 0);
    }

    #[test]
    fn empty_input_is_truncated() {
        let result = decompress(&[], &Limits::default());
        assert!(!result.valid);
        assert!(!result.eof);
        assert_eq!(result.error, Some(DecompressionError::Truncated));
        assert!(result.data.is_empty());
    }

    #[test]
    fn garbage_input_is_invalid() {
        let result = decompress(b"not zlib", &Limits::default());
        assert!(!result.valid);
        assert!(matches!(
            result.error,
            Some(DecompressionError::InvalidStream { .. })
        ));
        assert!(result.data.is_empty());
    }

    #[test]
    fn checksum_corruption_is_invalid() {
        let mut blob = compress(b"payload payload payload");
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        let result = decompress(&blob, &Limits::default());
        assert!(!result.valid);
        assert!(matches!(
            result.error,
            Some(DecompressionError::InvalidStream { .. })
        ));
    }

    #[test]
    fn truncated_stream_is_reported() {
        let blob = compress(b"payload payload payload");
        let result = decompress(&blob[..blob.len() - 2], &Limits::default());
        assert!(!result.valid);
        assert!(!result.eof);
        assert_eq!(result.error, Some(DecompressionError::Truncated));
    }

    #[test]
    fn trailing_bytes_are_recorded_not_interpreted() {
        let mut blob = compress(b"payload");
        blob.extend_from_slice(b"tail");
        let result = decompress(&blob, &Limits::default());
        assert!(result.valid);
        assert_eq!(result.data, b"payload");
        assert_eq!(result.trailing_bytes, 4);
        assert_eq!(result.trailing_prefix_hex, "7461696c");
    }

    #[test]
    fn concatenated_second_stream_is_trailing_only() {
        let mut blob = compress(b"one");
        blob.extend_from_slice(&compress(b"two"));
        let result = decompress(&blob, &Limits::default());
        assert!(result.valid);
        assert_eq!(result.data, b"one");
        assert!(result.trailing_bytes > 0);
    }

    #[test]
    fn exact_expanded_limit_is_accepted() {
        let raw = vec![7u8; 1000];
        let result = decompress(&compress(&raw), &small_limits(1000));
        assert!(result.valid, "{:?}", result.error);
        assert_eq!(result.expanded_bytes, 1000);
    }

    #[test]
    fn expanded_limit_exceeded_is_bounded() {
        let raw = vec![7u8; 1024 * 1024];
        let result = decompress(&compress(&raw), &small_limits(1000));
        assert!(!result.valid);
        assert_eq!(
            result.error,
            Some(DecompressionError::ExpandedLimitExceeded)
        );
        assert_eq!(result.expanded_bytes, 0);
        assert!(result.data.is_empty());
    }

    #[test]
    fn compressed_limit_is_checked_before_work() {
        let blob = compress(&[1u8; 4096]);
        let limits = Limits::new(16, 1024 * 1024, 50_000).expect("valid limits");
        let result = decompress(&blob, &limits);
        assert!(!result.valid);
        assert_eq!(
            result.error,
            Some(DecompressionError::CompressedLimitExceeded)
        );
        assert_eq!(result.compressed_bytes, blob.len() as u64);
    }

    #[test]
    fn random_garbage_never_panics() {
        let mut state = 0x12345678u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for length in [0usize, 1, 2, 3, 5, 17, 64, 255, 1024] {
            for _ in 0..8 {
                let blob: Vec<u8> = (0..length).map(|_| (next() & 0xff) as u8).collect();
                let result = decompress(&blob, &Limits::default());
                if !result.valid {
                    assert!(result.error.is_some());
                    assert!(result.data.is_empty());
                }
            }
        }
    }

    #[test]
    fn reason_codes_match_python_tokens() {
        assert_eq!(
            DecompressionError::InvalidStream {
                detail: String::new()
            }
            .reason_code(),
            "invalid_zlib_stream"
        );
        assert_eq!(
            DecompressionError::Truncated.reason_code(),
            "truncated_or_incomplete_zlib_stream"
        );
        assert_eq!(
            DecompressionError::ExpandedLimitExceeded.reason_code(),
            "expanded_size_limit_exceeded"
        );
        assert_eq!(
            DecompressionError::CompressedLimitExceeded.reason_code(),
            "compressed_size_limit_exceeded"
        );
        assert_eq!(
            DecompressionError::Truncated.contract_code(),
            "DECOMPRESSION_FAILED"
        );
        assert_eq!(
            DecompressionError::ExpandedLimitExceeded.contract_code(),
            "RESOURCE_LIMIT"
        );
    }
}
