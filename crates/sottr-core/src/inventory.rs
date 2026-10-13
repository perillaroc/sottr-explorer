//! 前缀与 type28 溢出记录恢复、完整性判定（SX-005）。
//!
//! 约束（开发方案 SX-005）：
//! - 前缀从 `0x44` 读取 512 个 8 字节槽；仅非零 ID 占用，可部分填满；
//! - 有效 type28 段总长恰为 4112 字节：跳过 16 字节头后读取 512 槽；
//! - 保留原始伴随 u32、最低两位、表类型、槽号与偏移；未知非零 ID 留存；
//! - 重复记录不折叠，另算唯一集合与重复数；
//! - 不扫旧镜像与声明流之外的残留；
//! - **异常 type28（长度不符）必须使 `inventory_complete = false` 并阻断缺席推断**
//!   ——这是比 Python v0.2 更强的新规则，列入差异白名单；
//! - 正常语料中其他未解释段（如镜像类型 17）不自动视为损坏。
//!
//! 恢复结果直接使用契约观测类型 [`InventoryObservation`]；结构化原因
//! （[`InventoryReason`]）供后续诊断组装使用。

use std::collections::BTreeSet;
use std::fmt;

use crate::report::{
    InternalId, InventoryObservation, InventoryRecord, InventoryTable, SupportStatus,
};
use crate::sections::{CAMPAIGN_FAMILY_EXPANDED_BYTES, SectionError, SectionStream};

/// 前缀表起始偏移。
pub const PREFIX_OFFSET: usize = 0x44;

/// 前缀槽数量。
pub const PREFIX_SLOTS: usize = 512;

/// 每个槽的字节数（u32 内部 ID + u32 伴随字）。
pub const SLOT_BYTES: usize = 8;

/// 有效 type28 段的含头总字节数。
pub const TYPE28_TOTAL_BYTES: u32 = 4112;

/// type28 段槽区之前跳过的字节数（4 字节段头 + 12 字节段内头）。
pub const TYPE28_HEADER_BYTES: usize = 16;

/// type28 段内的槽数量。
pub const TYPE28_SLOTS: usize = 512;

const COMPLETE_SCOPE: &str =
    "parser_profile 前缀与全部有效 type28 段恢复完整；不代表全游戏全集完备";
const PARTIAL_SCOPE: &str = "存在长度异常的 type28 段；物品表不完整，缺失推断必须关闭";
const UNSUPPORTED_SCOPE: &str = "区段流不可用；未执行记录恢复";

/// 记录恢复的结构化原因（映射到契约诊断码）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryReason {
    /// 区段流不可用（家族尺寸不符或流校验失败）。
    SectionStreamUnavailable {
        /// 区段层原因。
        why: SectionError,
    },
    /// type28 段存在但长度不符合 4112 字节的已知布局；该段不被读取。
    Type28LengthMismatch {
        /// 段头偏移。
        offset: u64,
        /// 观察到的含头字节数。
        bytes: u32,
    },
}

impl InventoryReason {
    /// 契约诊断码：家族尺寸不符按上下文处理，其余按布局处理。
    pub fn diagnostic_code(&self) -> &'static str {
        match self {
            InventoryReason::SectionStreamUnavailable {
                why: SectionError::UnrecognizedFamilySize,
            } => "UNSUPPORTED_CONTEXT",
            InventoryReason::SectionStreamUnavailable { .. } => "UNSUPPORTED_LAYOUT",
            InventoryReason::Type28LengthMismatch { .. } => "UNSUPPORTED_LAYOUT",
        }
    }

    /// 面向诊断消息的说明。
    pub fn message(&self) -> String {
        match self {
            InventoryReason::SectionStreamUnavailable { why } => {
                format!("区段流不可用（{}），未执行记录恢复", why.message())
            }
            InventoryReason::Type28LengthMismatch { offset, bytes } => format!(
                "type28 段位于 0x{offset:X}，长度 {bytes} 字节，不符合已知布局 {TYPE28_TOTAL_BYTES} 字节；该段未读取且表不完整"
            ),
        }
    }
}

impl fmt::Display for InventoryReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

/// 记录恢复结果。
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryRecovery {
    /// 契约观测（status、完整性、记录与计数）。
    pub observation: InventoryObservation,
    /// 结构化原因；为空表示按已知布局完整恢复。
    pub reasons: Vec<InventoryReason>,
}

/// 从展开字节与已步进的区段流恢复物品记录。
///
/// `stream` 必须是 [`SectionStream::walk`] 对同一份 `data` 的结果；
/// 流无效或尺寸不符时返回 `status = unsupported` 且不产生记录。
pub fn recover(data: &[u8], stream: &SectionStream) -> InventoryRecovery {
    if !stream.valid || data.len() != CAMPAIGN_FAMILY_EXPANDED_BYTES {
        let why = stream.error.unwrap_or(SectionError::UnrecognizedFamilySize);
        return InventoryRecovery {
            observation: InventoryObservation {
                status: SupportStatus::Unsupported,
                inventory_complete: false,
                completeness_scope: UNSUPPORTED_SCOPE.to_string(),
                records: Vec::new(),
                record_count: 0,
                unique_id_count: 0,
                duplicate_record_count: 0,
            },
            reasons: vec![InventoryReason::SectionStreamUnavailable { why }],
        };
    }

    let prefix_end = PREFIX_OFFSET + PREFIX_SLOTS * SLOT_BYTES;
    let Some(prefix) = data.get(PREFIX_OFFSET..prefix_end) else {
        return unsupported_with(InventoryReason::SectionStreamUnavailable {
            why: SectionError::InvalidOrOutOfBoundsSection,
        });
    };

    let mut records: Vec<InventoryRecord> = Vec::new();
    for index in 0..PREFIX_SLOTS {
        let (internal_id, companion) = read_slot(prefix, index);
        if internal_id != 0 {
            records.push(build_record(
                InventoryTable::Prefix,
                index as u32,
                internal_id,
                companion,
                PREFIX_OFFSET + index * SLOT_BYTES,
            ));
        }
    }

    let mut reasons: Vec<InventoryReason> = Vec::new();
    for section in &stream.sections {
        if section.section_type != 28 {
            continue;
        }
        if section.bytes != TYPE28_TOTAL_BYTES {
            reasons.push(InventoryReason::Type28LengthMismatch {
                offset: section.offset,
                bytes: section.bytes,
            });
            continue;
        }
        let slots_start = section.offset as usize + TYPE28_HEADER_BYTES;
        let slots_end = slots_start + TYPE28_SLOTS * SLOT_BYTES;
        let Some(slots) = data.get(slots_start..slots_end) else {
            reasons.push(InventoryReason::SectionStreamUnavailable {
                why: SectionError::InvalidOrOutOfBoundsSection,
            });
            continue;
        };
        for index in 0..TYPE28_SLOTS {
            let (internal_id, companion) = read_slot(slots, index);
            if internal_id != 0 {
                records.push(build_record(
                    InventoryTable::Type28Overflow,
                    index as u32,
                    internal_id,
                    companion,
                    slots_start + index * SLOT_BYTES,
                ));
            }
        }
    }

    let record_count = records.len() as u64;
    let unique_id_count = records
        .iter()
        .map(|record| record.internal_id)
        .collect::<BTreeSet<InternalId>>()
        .len() as u64;
    let inventory_complete = reasons.is_empty();
    let status = if inventory_complete {
        SupportStatus::Supported
    } else {
        SupportStatus::Partial
    };
    let completeness_scope = if inventory_complete {
        COMPLETE_SCOPE
    } else {
        PARTIAL_SCOPE
    }
    .to_string();

    InventoryRecovery {
        observation: InventoryObservation {
            status,
            inventory_complete,
            completeness_scope,
            record_count,
            unique_id_count,
            duplicate_record_count: record_count - unique_id_count,
            records,
        },
        reasons,
    }
}

fn unsupported_with(reason: InventoryReason) -> InventoryRecovery {
    InventoryRecovery {
        observation: InventoryObservation {
            status: SupportStatus::Unsupported,
            inventory_complete: false,
            completeness_scope: UNSUPPORTED_SCOPE.to_string(),
            records: Vec::new(),
            record_count: 0,
            unique_id_count: 0,
            duplicate_record_count: 0,
        },
        reasons: vec![reason],
    }
}

fn read_slot(slots: &[u8], index: usize) -> (u32, u32) {
    let offset = index * SLOT_BYTES;
    let chunk = &slots[offset..offset + SLOT_BYTES];
    (
        u32::from_le_bytes(chunk[0..4].try_into().expect("slot id bytes")),
        u32::from_le_bytes(chunk[4..8].try_into().expect("slot companion bytes")),
    )
}

fn build_record(
    table: InventoryTable,
    index: u32,
    internal_id: u32,
    companion: u32,
    offset: usize,
) -> InventoryRecord {
    InventoryRecord {
        table,
        index,
        internal_id: InternalId::new(internal_id),
        raw_companion_u32: companion,
        raw_companion_hex: format!("0x{companion:08X}"),
        raw_low_two_bits: (companion & 0x3) as u8,
        id_offset: offset as u64,
        id_offset_hex: format!("0x{offset:X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;
    use crate::sections::MAIN_STREAM_START;
    use crate::synthetic::{SampleBuilder, SectionHandle};

    fn recovery(raw: &[u8]) -> InventoryRecovery {
        let stream = SectionStream::walk(raw, &Limits::default());
        recover(raw, &stream)
    }

    fn write_overflow_slot(
        builder: &mut SampleBuilder,
        handle: SectionHandle,
        slot: usize,
        internal_id: u32,
        companion: u32,
    ) {
        let base = handle.offset + TYPE28_HEADER_BYTES + slot * SLOT_BYTES;
        builder.write_u32_le(base, internal_id);
        builder.write_u32_le(base + 4, companion);
    }

    fn assert_counts(recovery: &InventoryRecovery) {
        let observation = &recovery.observation;
        assert_eq!(
            observation.record_count,
            observation.unique_id_count + observation.duplicate_record_count,
            "记录数必须等于唯一数加重复数"
        );
        assert_eq!(observation.record_count as usize, observation.records.len());
    }

    #[test]
    fn full_prefix_and_overflow_recovery() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        for index in 0..PREFIX_SLOTS {
            builder.write_prefix_slot(index, 0x1000_0000 + index as u32, 1);
        }
        let overflow = builder.push_section(28, 4108);
        for index in 0..TYPE28_SLOTS {
            write_overflow_slot(&mut builder, overflow, index, 0x2000_0000 + index as u32, 1);
        }
        let raw = builder.finish();

        let recovery = recovery(&raw);
        let observation = &recovery.observation;
        assert_counts(&recovery);
        assert_eq!(observation.record_count, 1024);
        assert_eq!(observation.unique_id_count, 1024);
        assert_eq!(observation.duplicate_record_count, 0);
        assert!(observation.inventory_complete);
        assert_eq!(observation.status, SupportStatus::Supported);
        assert!(recovery.reasons.is_empty());

        let first = &observation.records[0];
        assert_eq!(first.table, InventoryTable::Prefix);
        assert_eq!(first.index, 0);
        assert_eq!(first.internal_id, InternalId::new(0x1000_0000));
        assert_eq!(first.id_offset, PREFIX_OFFSET as u64);
        assert_eq!(first.id_offset_hex, "0x44");
        assert_eq!(first.raw_low_two_bits, 1);

        let first_overflow = &observation.records[PREFIX_SLOTS];
        assert_eq!(first_overflow.table, InventoryTable::Type28Overflow);
        assert_eq!(first_overflow.index, 0);
        assert_eq!(
            first_overflow.id_offset,
            (overflow.offset + TYPE28_HEADER_BYTES) as u64
        );
    }

    #[test]
    fn partial_prefix_is_complete() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_prefix_slot(0, 0xAAAA_0001, 1);
        builder.write_prefix_slot(1, 0xAAAA_0002, 3);
        builder.write_prefix_slot(2, 0xAAAA_0003, 1);
        let raw = builder.finish();

        let recovery = recovery(&raw);
        assert_counts(&recovery);
        assert_eq!(recovery.observation.record_count, 3);
        assert!(recovery.observation.inventory_complete);
    }

    #[test]
    fn zero_id_slots_with_dirty_companions_are_not_records() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_prefix_slot(0, 0, 0xFFFF_FFFF);
        builder.write_prefix_slot(1, 0xABCD_1234, 0);
        builder.write_prefix_slot(2, 0, 0xDEAD_BEEF);
        let raw = builder.finish();

        let recovery = recovery(&raw);
        assert_counts(&recovery);
        assert_eq!(recovery.observation.record_count, 1);
        assert_eq!(
            recovery.observation.records[0].internal_id,
            InternalId::new(0xABCD_1234)
        );
    }

    #[test]
    fn companion_high_bits_are_preserved_not_interpreted() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_prefix_slot(0, 0xAABB_CCDD, 0xFFFF_FFFD);
        let raw = builder.finish();

        let record = &recovery(&raw).observation.records[0];
        assert_eq!(record.raw_companion_u32, 0xFFFF_FFFD);
        assert_eq!(record.raw_companion_hex, "0xFFFFFFFD");
        assert_eq!(record.raw_low_two_bits, 1);
    }

    #[test]
    fn no_type28_with_unknown_sections_stays_complete() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.push_section(17, 1024); // 镜像类段：不扫描、不视为损坏
        builder.push_section(99, 8); // 未解释段：不自动视为损坏
        builder.write_prefix_slot(0, 0x0102_0304, 1);
        let raw = builder.finish();

        let recovery = recovery(&raw);
        assert_counts(&recovery);
        assert_eq!(recovery.observation.record_count, 1);
        assert!(recovery.observation.inventory_complete);
        assert!(recovery.reasons.is_empty());
    }

    #[test]
    fn type28_last_slot_is_recovered() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        let overflow = builder.push_section(28, 4108);
        write_overflow_slot(&mut builder, overflow, 511, 0xFEED_FACE, 3);
        let raw = builder.finish();

        let recovery = recovery(&raw);
        assert_counts(&recovery);
        assert_eq!(recovery.observation.record_count, 1);
        let record = &recovery.observation.records[0];
        assert_eq!(record.index, 511);
        assert_eq!(record.table, InventoryTable::Type28Overflow);
        assert_eq!(
            record.id_offset,
            (overflow.offset + TYPE28_HEADER_BYTES + 511 * SLOT_BYTES) as u64
        );
        assert_eq!(record.raw_low_two_bits, 3);
    }

    #[test]
    fn multiple_type28_sections_are_read_in_stream_order() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        let first = builder.push_section(28, 4108);
        write_overflow_slot(&mut builder, first, 0, 0x0000_0001, 1);
        let second = builder.push_section(28, 4108);
        write_overflow_slot(&mut builder, second, 0, 0x0000_0002, 1);
        let raw = builder.finish();

        let recovery = recovery(&raw);
        assert_counts(&recovery);
        assert_eq!(recovery.observation.record_count, 2);
        assert_eq!(
            recovery.observation.records[0].internal_id,
            InternalId::new(1)
        );
        assert_eq!(
            recovery.observation.records[1].internal_id,
            InternalId::new(2)
        );
        assert!(recovery.observation.inventory_complete);
    }

    #[test]
    fn duplicate_ids_keep_all_records_and_count_separately() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_prefix_slot(0, 0x7777_7777, 1);
        let overflow = builder.push_section(28, 4108);
        write_overflow_slot(&mut builder, overflow, 0, 0x7777_7777, 3);
        write_overflow_slot(&mut builder, overflow, 1, 0x8888_8888, 1);
        let raw = builder.finish();

        let recovery = recovery(&raw);
        assert_counts(&recovery);
        assert_eq!(recovery.observation.record_count, 3);
        assert_eq!(recovery.observation.unique_id_count, 2);
        assert_eq!(recovery.observation.duplicate_record_count, 1);
        assert_eq!(
            recovery.observation.records[0].table,
            InventoryTable::Prefix
        );
        assert_eq!(
            recovery.observation.records[1].table,
            InventoryTable::Type28Overflow
        );
        assert_eq!(
            recovery.observation.records[0].internal_id,
            recovery.observation.records[1].internal_id
        );
    }

    #[test]
    fn type28_length_mismatch_degrades_completeness_and_skips_section() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        let good = builder.push_section(28, 4108);
        write_overflow_slot(&mut builder, good, 0, 0x1111_1111, 1);
        let bad = builder.push_section(28, 4104); // 总长 4108，不符合 4112
        write_overflow_slot(&mut builder, bad, 0, 0x2222_2222, 1);
        builder.write_prefix_slot(0, 0x3333_3333, 1);
        let raw = builder.finish();

        let recovery = recovery(&raw);
        assert_counts(&recovery);
        assert!(!recovery.observation.inventory_complete);
        assert_eq!(recovery.observation.status, SupportStatus::Partial);
        assert_eq!(recovery.reasons.len(), 1);
        assert_eq!(
            recovery.reasons[0],
            InventoryReason::Type28LengthMismatch {
                offset: bad.offset as u64,
                bytes: 4108
            }
        );
        assert_eq!(recovery.reasons[0].diagnostic_code(), "UNSUPPORTED_LAYOUT");
        // 合法段与前缀仍被读取；异常段整体不读取。
        let ids: Vec<u32> = recovery
            .observation
            .records
            .iter()
            .map(|record| record.internal_id.raw())
            .collect();
        assert!(ids.contains(&0x1111_1111));
        assert!(ids.contains(&0x3333_3333));
        assert!(!ids.contains(&0x2222_2222));
    }

    #[test]
    fn invalid_stream_is_unsupported_with_no_records() {
        let mut builder = SampleBuilder::new();
        builder.write_prefix_slot(0, 0x1234_5678, 1);
        let raw = builder.finish_with_declared_length(0);

        let recovery = recovery(&raw);
        assert_eq!(recovery.observation.status, SupportStatus::Unsupported);
        assert!(!recovery.observation.inventory_complete);
        assert!(recovery.observation.records.is_empty());
        assert_eq!(
            recovery.reasons,
            vec![InventoryReason::SectionStreamUnavailable {
                why: SectionError::InvalidStreamLength
            }]
        );
        assert_eq!(recovery.reasons[0].diagnostic_code(), "UNSUPPORTED_LAYOUT");
    }

    #[test]
    fn wrong_family_size_is_unsupported_context() {
        let raw = vec![0u8; 1024 * 1024];
        let recovery = recovery(&raw);
        assert_eq!(recovery.observation.status, SupportStatus::Unsupported);
        assert!(recovery.observation.records.is_empty());
        assert_eq!(recovery.reasons[0].diagnostic_code(), "UNSUPPORTED_CONTEXT");
    }

    #[test]
    fn stale_tail_and_mirror_like_data_are_ignored() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        let overflow = builder.push_section(28, 4108);
        write_overflow_slot(&mut builder, overflow, 0, 0x5555_5555, 1);
        let mut raw = builder.finish();

        let stream = SectionStream::walk(&raw, &Limits::default());
        assert!(stream.valid);
        let end = stream.end_exclusive as usize;

        // 声明流之外的伪 type28 段（旧残留）：不得被扫描。
        let stale_header = ((1028u32 << 8) | 28).to_le_bytes();
        raw[end + 32..end + 36].copy_from_slice(&stale_header);
        raw[end + 48..end + 52].copy_from_slice(&0xBAD0_0001u32.to_le_bytes());
        // 另一处的镜像式前缀（旧副本）：同样不得被扫描。
        raw[0x20_0000..0x20_0004].copy_from_slice(&0xBAD0_0002u32.to_le_bytes());

        let recovery = recover(&raw, &stream);
        assert_counts(&recovery);
        assert_eq!(recovery.observation.record_count, 1);
        assert_eq!(
            recovery.observation.records[0].internal_id,
            InternalId::new(0x5555_5555)
        );
        assert!(recovery.observation.inventory_complete);
    }

    #[test]
    fn stream_end_and_prefix_offsets_are_consistent() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        let raw = builder.finish();
        let stream = SectionStream::walk(&raw, &Limits::default());
        assert_eq!(stream.start_offset, MAIN_STREAM_START as u64);
        let recovery = recover(&raw, &stream);
        assert!(recovery.observation.inventory_complete);
        assert_eq!(recovery.observation.record_count, 0);
    }
}
