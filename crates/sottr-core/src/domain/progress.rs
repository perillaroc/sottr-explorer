//! 进度观测：整体候选与 43 项稀疏区域计数。
//!
//! 约束（开发方案 SX-006）：
//! - `0x28` float32 只接受有限且 0–100 的值；非有限浮点只保留 `raw_hex`/字节表示，
//!   绝不进入 JSON 数值；越界有限值同样降级并保留原始值；
//! - 区域计数只读元数据提供的稀疏映射（基址 `0x10BC`、步长 104），不补齐为 9×7 全表；
//! - 原始读数超过来源目标（或来源目标缺失）时保留 `raw_value`，候选 `value` 置 `null`；
//! - 缺映射不是零：没有映射时返回空集合（不是 43 个 0），不生成区域百分比。

use crate::report::{EvidenceLevel, ProgressValue, RegionCounter, SupportStatus};
use crate::sections::{CAMPAIGN_FAMILY_EXPANDED_BYTES, SectionStream};

/// 整体完成度候选的 float32 偏移。
pub const OVERALL_OFFSET: usize = 0x28;

/// 区域计数行基址。
pub const REGION_COUNTER_BASE: usize = 0x10BC;

/// 区域计数行步长。
pub const REGION_COUNTER_STRIDE: usize = 104;

const OVERALL_EVIDENCE_REF: &str = "ev:overall:0x28";
const OVERALL_LIMITATION: &str = "社区校准候选，未经游戏内独立核实";
const COUNTER_LIMITATION: &str = "来源目标来自社区脚本，未证明适用所有版本/DLC";

/// 整体完成度读数。
#[derive(Debug, Clone, PartialEq)]
pub struct OverallReadout {
    /// 契约进度值；不可用时 `value` 为 `null` 且 `reason` 必填。
    pub progress: ProgressValue,
    /// 原始 4 字节的十六进制（小写）；不可读取时为 `None`。
    pub raw_bytes_hex: Option<String>,
    /// 有限的原始浮点值（含越界值）；非有限值或不可读取时为 `None`。
    pub raw_value: Option<f32>,
}

/// 区域计数映射规格（由元数据层提供；本模块不硬编码映射）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionCounterSpec {
    /// 稳定计数键（如 `counter:mission_of_san_juan:survival_caches`）。
    pub counter_key: String,
    /// 区域 ID；未知为 `None`。
    pub region_id: Option<String>,
    /// 类别 ID。
    pub category_id: String,
    /// 序列化偏移（小端 u32 读数位置）。
    pub serialized_offset: u64,
    /// 社区来源目标值。
    pub source_total: u32,
}

/// 无法读取的映射规格（元数据缺陷；不产生假读数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionCounterSpecError {
    /// 计数键。
    pub counter_key: String,
    /// 规格声明的偏移。
    pub serialized_offset: u64,
    /// 稳定原因。
    pub reason: String,
}

/// 区域计数读数。
#[derive(Debug, Clone, PartialEq)]
pub struct RegionCounterReadout {
    /// 结构守卫是否通过（4 MiB 家族尺寸且区段流有效）。
    pub guard_passed: bool,
    /// 计数观测（稀疏；可能为空）。
    pub counters: Vec<RegionCounter>,
    /// 无法读取的规格。
    pub spec_errors: Vec<RegionCounterSpecError>,
}

/// 读取整体完成度候选；结构守卫失败时关闭且保留原始字节。
pub fn overall_completion(data: &[u8], sections: &SectionStream) -> OverallReadout {
    let raw_bytes = data.get(OVERALL_OFFSET..OVERALL_OFFSET + 4);
    let raw_bytes_hex = raw_bytes.map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect());

    if data.len() != CAMPAIGN_FAMILY_EXPANDED_BYTES || !sections.valid {
        return OverallReadout {
            progress: closed_value("区段流或家族尺寸校验未通过，整体进度候选已关闭"),
            raw_bytes_hex,
            raw_value: None,
        };
    }

    let Some(bytes) = raw_bytes else {
        return OverallReadout {
            progress: closed_value("0x28 读取越界，整体进度候选已关闭"),
            raw_bytes_hex: None,
            raw_value: None,
        };
    };
    let value = f32::from_le_bytes(bytes.try_into().expect("four bytes"));
    if value.is_finite() && (0.0..=100.0).contains(&value) {
        OverallReadout {
            progress: ProgressValue {
                value: Some(f64::from(value)),
                unit: "percent".to_string(),
                support_status: SupportStatus::Supported,
                reason: None,
                evidence_level: EvidenceLevel::CommunityCorrelated,
                evidence_refs: vec![OVERALL_EVIDENCE_REF.to_string()],
                limitations: vec![OVERALL_LIMITATION.to_string()],
            },
            raw_bytes_hex,
            raw_value: Some(value),
        }
    } else {
        OverallReadout {
            progress: closed_value("0x28 浮点值非有限或超出 0–100，仅保留原始字节表示"),
            raw_bytes_hex,
            raw_value: value.is_finite().then_some(value),
        }
    }
}

fn closed_value(reason: &str) -> ProgressValue {
    ProgressValue {
        value: None,
        unit: "percent".to_string(),
        support_status: SupportStatus::Unsupported,
        reason: Some(reason.to_string()),
        evidence_level: EvidenceLevel::Observed,
        evidence_refs: Vec::new(),
        limitations: vec![OVERALL_LIMITATION.to_string()],
    }
}

/// 按稀疏映射读取区域计数；缺映射返回空集合且在能力矩阵中降级，不用 0 填充。
pub fn region_counters(
    data: &[u8],
    sections: &SectionStream,
    specs: &[RegionCounterSpec],
) -> RegionCounterReadout {
    if data.len() != CAMPAIGN_FAMILY_EXPANDED_BYTES || !sections.valid {
        return RegionCounterReadout {
            guard_passed: false,
            counters: Vec::new(),
            spec_errors: Vec::new(),
        };
    }

    let mut counters = Vec::new();
    let mut spec_errors = Vec::new();
    for spec in specs {
        let Some(offset) = usize::try_from(spec.serialized_offset).ok() else {
            spec_errors.push(RegionCounterSpecError {
                counter_key: spec.counter_key.clone(),
                serialized_offset: spec.serialized_offset,
                reason: "SERIALIZED_OFFSET_UNREPRESENTABLE".to_string(),
            });
            continue;
        };
        let Some(bytes) = data.get(offset..offset + 4) else {
            spec_errors.push(RegionCounterSpecError {
                counter_key: spec.counter_key.clone(),
                serialized_offset: spec.serialized_offset,
                reason: "SERIALIZED_OFFSET_OUT_OF_BOUNDS".to_string(),
            });
            continue;
        };
        let raw = u32::from_le_bytes(bytes.try_into().expect("four bytes"));
        let within = raw <= spec.source_total;

        let mut limitations = vec![COUNTER_LIMITATION.to_string()];
        let (value, support_status, reason, evidence_level) = if within {
            (
                Some(raw),
                SupportStatus::Supported,
                None,
                EvidenceLevel::CommunityCorrelated,
            )
        } else {
            limitations.push("原始读数超过来源目标：候选计数与剩余置 null，不裁剪".to_string());
            (
                None,
                SupportStatus::Partial,
                Some("原始读数超过来源目标，候选计数与剩余置 null，不裁剪到目标值".to_string()),
                EvidenceLevel::Observed,
            )
        };

        counters.push(RegionCounter {
            counter_key: spec.counter_key.clone(),
            region_id: spec.region_id.clone(),
            category_id: spec.category_id.clone(),
            value,
            unit: "count".to_string(),
            support_status,
            reason,
            evidence_level,
            evidence_refs: vec![format!("ev:{}", spec.counter_key)],
            limitations,
            source_target: Some(spec.source_total),
            within_source_target: within,
            raw_value: raw,
            raw_value_hex: format!("0x{raw:08X}"),
        });
    }

    RegionCounterReadout {
        guard_passed: true,
        counters,
        spec_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;
    use crate::synthetic::SampleBuilder;

    fn sample_bytes() -> Vec<u8> {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.finish()
    }

    fn walk(raw: &[u8]) -> SectionStream {
        SectionStream::walk(raw, &Limits::default())
    }

    fn spec(key: &str, offset: u64, total: u32) -> RegionCounterSpec {
        RegionCounterSpec {
            counter_key: key.to_string(),
            region_id: Some("mission_of_san_juan".to_string()),
            category_id: "survival_caches".to_string(),
            serialized_offset: offset,
            source_total: total,
        }
    }

    #[test]
    fn overall_valid_percent_is_supported_community_value() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_f32_le(OVERALL_OFFSET, 66.204_82_f32);
        let raw = builder.finish();

        let readout = overall_completion(&raw, &walk(&raw));
        assert_eq!(readout.progress.support_status, SupportStatus::Supported);
        assert_eq!(readout.progress.reason, None);
        assert_eq!(
            readout.progress.evidence_level,
            EvidenceLevel::CommunityCorrelated
        );
        assert_eq!(readout.progress.value, Some(f64::from(66.204_82_f32)));
        let bytes = 66.204_82_f32.to_le_bytes();
        assert_eq!(
            readout.raw_bytes_hex.as_deref(),
            Some(
                bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .as_str()
            )
        );
    }

    #[test]
    fn legal_zero_and_hundred_are_kept() {
        for value in [0.0_f32, 100.0_f32] {
            let mut builder = SampleBuilder::new();
            builder.push_section(1, 0);
            builder.write_f32_le(OVERALL_OFFSET, value);
            let raw = builder.finish();
            let readout = overall_completion(&raw, &walk(&raw));
            assert_eq!(readout.progress.value, Some(f64::from(value)));
            assert_eq!(readout.progress.support_status, SupportStatus::Supported);
        }
    }

    #[test]
    fn non_finite_and_out_of_range_values_are_closed_with_raw_bytes() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 120.0, -1.0] {
            let mut builder = SampleBuilder::new();
            builder.push_section(1, 0);
            builder.write_f32_le(OVERALL_OFFSET, value);
            let raw = builder.finish();
            let readout = overall_completion(&raw, &walk(&raw));

            assert_eq!(readout.progress.value, None, "{value}");
            assert_eq!(readout.progress.support_status, SupportStatus::Unsupported);
            assert!(readout.progress.reason.is_some());
            assert!(readout.raw_bytes_hex.is_some(), "原始字节必须保留");
            if value.is_finite() {
                assert_eq!(readout.raw_value, Some(value), "有限越界值保留原始值");
            } else {
                assert_eq!(readout.raw_value, None, "非有限浮点不得进入数值表示");
            }
        }
    }

    #[test]
    fn invalid_stream_closes_overall_candidate() {
        let mut builder = SampleBuilder::new();
        builder.write_f32_le(OVERALL_OFFSET, 66.2);
        let raw = builder.finish_with_declared_length(0);
        let readout = overall_completion(&raw, &walk(&raw));
        assert_eq!(readout.progress.value, None);
        assert_eq!(readout.progress.support_status, SupportStatus::Unsupported);
        assert!(readout.raw_bytes_hex.is_some());
    }

    #[test]
    fn counters_read_sparse_specs_without_percentages() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_u32_le(REGION_COUNTER_BASE, 0);
        builder.write_u32_le(REGION_COUNTER_BASE + REGION_COUNTER_STRIDE, 31);
        let raw = builder.finish();
        let specs = [
            spec(
                "counter:mission_of_san_juan:survival_caches",
                REGION_COUNTER_BASE as u64,
                8,
            ),
            spec(
                "counter:paititi:survival_caches",
                (REGION_COUNTER_BASE + REGION_COUNTER_STRIDE) as u64,
                34,
            ),
        ];

        let readout = region_counters(&raw, &walk(&raw), &specs);
        assert!(readout.guard_passed);
        assert!(readout.spec_errors.is_empty());
        assert_eq!(readout.counters.len(), 2);
        let first = &readout.counters[0];
        assert_eq!(first.value, Some(0));
        assert_eq!(first.raw_value, 0);
        assert_eq!(first.raw_value_hex, "0x00000000");
        assert_eq!(first.unit, "count");
        assert!(first.within_source_target);
        assert_eq!(
            first.evidence_refs,
            vec!["ev:counter:mission_of_san_juan:survival_caches"]
        );
        let second = &readout.counters[1];
        assert_eq!(second.value, Some(31));
        assert_eq!(second.source_target, Some(34));
    }

    #[test]
    fn counter_over_target_degrades_and_keeps_raw_value() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_u32_le(REGION_COUNTER_BASE, 99);
        let raw = builder.finish();
        let specs = [spec(
            "counter:mission_of_san_juan:survival_caches",
            REGION_COUNTER_BASE as u64,
            8,
        )];

        let readout = region_counters(&raw, &walk(&raw), &specs);
        let counter = &readout.counters[0];
        assert_eq!(counter.value, None);
        assert_eq!(counter.support_status, SupportStatus::Partial);
        assert!(counter.reason.is_some());
        assert_eq!(counter.evidence_level, EvidenceLevel::Observed);
        assert_eq!(counter.raw_value, 99, "原始读数保留");
        assert_eq!(counter.raw_value_hex, "0x00000063");
        assert!(!counter.within_source_target);
        assert_eq!(counter.source_target, Some(8));
    }

    #[test]
    fn missing_mapping_is_not_zero_filled() {
        let raw = sample_bytes();
        let readout = region_counters(&raw, &walk(&raw), &[]);
        assert!(readout.guard_passed);
        assert!(readout.counters.is_empty(), "缺映射不是 43 个 0");
        assert!(readout.spec_errors.is_empty());
    }

    #[test]
    fn out_of_bounds_spec_is_reported_without_fake_reading() {
        let raw = sample_bytes();
        let specs = [spec(
            "counter:oob",
            (CAMPAIGN_FAMILY_EXPANDED_BYTES - 2) as u64,
            8,
        )];
        let readout = region_counters(&raw, &walk(&raw), &specs);
        assert!(readout.counters.is_empty());
        assert_eq!(readout.spec_errors.len(), 1);
        assert_eq!(
            readout.spec_errors[0].reason,
            "SERIALIZED_OFFSET_OUT_OF_BOUNDS"
        );
    }

    #[test]
    fn structural_guard_failure_returns_no_counters() {
        let raw = vec![0u8; 1024 * 1024];
        let specs = [spec("counter:any", REGION_COUNTER_BASE as u64, 8)];
        let readout = region_counters(&raw, &walk(&raw), &specs);
        assert!(!readout.guard_passed);
        assert!(readout.counters.is_empty());
        assert!(readout.spec_errors.is_empty());
    }
}
