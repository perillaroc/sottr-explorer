//! SX-005 公共接口端到端测试：合成样本 → 容器 → 区段 → 记录恢复。

use sottr_core::container::decompress;
use sottr_core::inventory::{InventoryReason, TYPE28_HEADER_BYTES, recover};
use sottr_core::limits::Limits;
use sottr_core::report::{InternalId, InventoryTable, SupportStatus};
use sottr_core::sections::SectionStream;
use sottr_core::synthetic::{SampleBuilder, compress};

#[test]
fn synthetic_sample_recovers_prefix_and_overflow_records() {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    builder.write_prefix_slot(0, 0x0F28_B6EC, 1);
    builder.write_prefix_slot(1, 0xFFFF_FFFF, 0xFFFF_FFFD);
    let overflow = builder.push_section(28, 4108);
    let slot = overflow.offset + TYPE28_HEADER_BYTES;
    builder.write_u32_le(slot, 0x439B_19A6);
    builder.write_u32_le(slot + 4, 3);

    let decompressed = decompress(&compress(&builder.finish()), &Limits::default());
    assert!(decompressed.valid, "{:?}", decompressed.error);
    let stream = SectionStream::walk(&decompressed.data, &Limits::default());
    assert!(stream.valid, "{:?}", stream.error);

    let result = recover(&decompressed.data, &stream);
    let observation = &result.observation;
    assert_eq!(observation.status, SupportStatus::Supported);
    assert!(observation.inventory_complete);
    assert_eq!(observation.record_count, 3);
    assert_eq!(observation.unique_id_count, 3);
    assert_eq!(observation.duplicate_record_count, 0);
    assert!(result.reasons.is_empty());

    assert_eq!(
        observation.records[0].internal_id,
        InternalId::new(0x0F28_B6EC)
    );
    assert_eq!(observation.records[0].table, InventoryTable::Prefix);
    assert_eq!(observation.records[1].raw_low_two_bits, 1);
    assert_eq!(observation.records[2].table, InventoryTable::Type28Overflow);
    assert_eq!(
        observation.records[2].internal_id,
        InternalId::new(0x439B_19A6)
    );
    assert_eq!(observation.records[2].raw_low_two_bits, 3);
}

#[test]
fn malformed_type28_is_reported_as_incomplete_not_silently_skipped() {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    let bad = builder.push_section(28, 4100); // 总长 4104，不符合已知布局
    builder.write_u32_le(bad.offset + TYPE28_HEADER_BYTES, 0x1234_5678);

    let decompressed = decompress(&compress(&builder.finish()), &Limits::default());
    let stream = SectionStream::walk(&decompressed.data, &Limits::default());
    let result = recover(&decompressed.data, &stream);

    assert!(!result.observation.inventory_complete);
    assert_eq!(result.observation.status, SupportStatus::Partial);
    assert!(matches!(
        result.reasons.as_slice(),
        [InventoryReason::Type28LengthMismatch { .. }]
    ));
    assert!(result.observation.records.is_empty());
}

#[test]
fn profile_like_input_recovers_nothing_and_stays_unsupported() {
    let raw = vec![0u8; 25_600];
    let decompressed = decompress(&compress(&raw), &Limits::default());
    assert!(decompressed.valid);

    let stream = SectionStream::walk(&decompressed.data, &Limits::default());
    let result = recover(&decompressed.data, &stream);
    assert_eq!(result.observation.status, SupportStatus::Unsupported);
    assert!(!result.observation.inventory_complete);
    assert!(result.observation.records.is_empty());
    assert_eq!(result.reasons[0].diagnostic_code(), "UNSUPPORTED_CONTEXT");
}
