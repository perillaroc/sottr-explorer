//! SX-004 公共接口端到端测试：合成样本 → zlib 容器 → 主流区段。
//!
//! 验证从公开 API（`container`、`sections`、`synthetic`）完成整条路径，
//! 并固定“坏流不产生正常语义值”的边界（下游任务据此关闭语义解释）。

use sottr_core::container::decompress;
use sottr_core::limits::Limits;
use sottr_core::sections::{
    CAMPAIGN_FAMILY_EXPANDED_BYTES, MAIN_STREAM_START, SectionError, SectionStream,
};
use sottr_core::synthetic::{SampleBuilder, compress};

#[test]
fn synthetic_campaign_sample_flows_through_public_api() {
    let mut builder = SampleBuilder::new();
    let first = builder.push_section(1, 8);
    builder.data_mut()[first.payload_offset..first.payload_offset + 8].copy_from_slice(&[0xAA; 8]);
    let overflow = builder.push_section(28, 4108);
    builder.data_mut()[overflow.payload_offset..overflow.payload_offset + 4]
        .copy_from_slice(&0x12345678u32.to_le_bytes());
    let raw = builder.finish();

    let decompressed = decompress(&compress(&raw), &Limits::default());
    assert!(decompressed.valid, "{:?}", decompressed.error);
    assert!(decompressed.eof);
    assert_eq!(
        decompressed.expanded_bytes,
        CAMPAIGN_FAMILY_EXPANDED_BYTES as u64
    );
    assert_eq!(decompressed.trailing_bytes, 0);

    let stream = SectionStream::walk(&decompressed.data, &Limits::default());
    assert!(stream.valid, "{:?}", stream.error);
    assert_eq!(stream.section_count(), 2);
    assert_eq!(stream.sections[0].offset, MAIN_STREAM_START as u64);
    assert_eq!(stream.sections[0].section_type, 1);
    assert_eq!(stream.sections[1].section_type, 28);
    assert_eq!(stream.sections[1].bytes, 4112);

    let payload_offset = stream.sections[0].offset as usize + 4;
    assert_eq!(
        &decompressed.data[payload_offset..payload_offset + 8],
        &[0xAA; 8]
    );
}

#[test]
fn profile_like_small_file_reports_container_only() {
    // profile（25,600 字节）不是 4 MiB 样本家族：容器可观察，区段步进明确不支持，
    // 不据此进入战役语义（结构识别与语义适用分开）。
    let raw = vec![0u8; 25_600];
    let decompressed = decompress(&compress(&raw), &Limits::default());
    assert!(decompressed.valid);
    assert_eq!(decompressed.data.len(), 25_600);

    let stream = SectionStream::walk(&decompressed.data, &Limits::default());
    assert!(!stream.valid);
    assert_eq!(stream.error, Some(SectionError::UnrecognizedFamilySize));
    assert_eq!(stream.section_count(), 0);
}

#[test]
fn broken_container_does_not_yield_a_stream() {
    let mut blob = compress(&vec![0u8; CAMPAIGN_FAMILY_EXPANDED_BYTES]);
    let last = blob.len() - 1;
    blob[last] ^= 0x01;
    let decompressed = decompress(&blob, &Limits::default());
    assert!(!decompressed.valid);
    assert!(decompressed.data.is_empty());

    // 失败容器不提供任何可步进数据：下游不得产生正常语义值。
    let stream = SectionStream::walk(&decompressed.data, &Limits::default());
    assert!(!stream.valid);
}
