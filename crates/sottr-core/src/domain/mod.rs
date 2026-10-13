//! 领域层：上下文适用性、进度观测与能力矩阵。
//!
//! 设计约束（开发方案 SX-006）：
//! - 结构识别与语义适用分开：文件名、4 MiB 尺寸与合法区段流都不足以识别普通战役，
//!   只有调用方显式声明模式后才开启战役语义；用户声明保留来源与适用限制；
//! - `0x28` float32 只接受有限且 0–100 的值；非有限值只保留原始字节；
//! - 区域只读取元数据提供的稀疏映射（43 项），不补齐为 9×7 全表，不生成区域百分比；
//! - 每项能力可独立降级；结构失败、模式未知或不适用时关闭相应收集推定。

pub mod context;
pub mod progress;

pub use context::{ContextAssessment, ContextFinding, DeclaredContext, assess};
pub use progress::{
    OverallReadout, RegionCounterReadout, RegionCounterSpec, RegionCounterSpecError,
    overall_completion, region_counters,
};

use crate::container::Decompression;
use crate::inventory::InventoryRecovery;
use crate::report::SupportStatus;
use crate::report::{Capabilities, Capability};
use crate::sections::{SectionError, SectionStream};

/// 能力矩阵输入（可独立提供；缺失的层按“未评估”处理）。
pub struct CapabilityInputs<'a> {
    /// 容器解压结果。
    pub decompression: &'a Decompression,
    /// 主区段流结果。
    pub sections: &'a SectionStream,
    /// 上下文是否允许战役语义（见 [`context::assess`]）。
    pub campaign_applicable: bool,
    /// 记录恢复结果（SX-005）。
    pub inventory: Option<&'a InventoryRecovery>,
    /// 区域计数读数；未评估为 `None`。
    pub region_counters: Option<&'a RegionCounterReadout>,
}

/// 组装能力矩阵；每项独立降级，参考差集与定位在 SX-007/SX-008 接线前标记未评估。
pub fn capability_matrix(inputs: &CapabilityInputs<'_>) -> Capabilities {
    let container_failure_code = inputs
        .decompression
        .error
        .as_ref()
        .map_or("DECOMPRESSION_FAILED", |error| error.contract_code());

    let container = if inputs.decompression.valid {
        supported()
    } else {
        unsupported(container_failure_code)
    };

    let sections = if !inputs.decompression.valid {
        // 容器失败：区段未执行，原因沿用容器级错误。
        unsupported(container_failure_code)
    } else if inputs.sections.valid {
        supported()
    } else {
        let code = match inputs.sections.error {
            Some(SectionError::UnrecognizedFamilySize) => "UNSUPPORTED_CONTEXT",
            _ => "UNSUPPORTED_LAYOUT",
        };
        unsupported(code)
    };

    let inventory = if !inputs.decompression.valid {
        unsupported(container_failure_code)
    } else if !inputs.campaign_applicable {
        unsupported("UNSUPPORTED_CONTEXT")
    } else if let Some(recovery) = inputs.inventory {
        let codes: Vec<&str> = recovery
            .reasons
            .iter()
            .map(|reason| reason.diagnostic_code())
            .collect();
        match recovery.observation.status {
            SupportStatus::Supported => supported(),
            SupportStatus::Partial => capability(SupportStatus::Partial, codes),
            SupportStatus::Unsupported => capability(SupportStatus::Unsupported, codes),
        }
    } else {
        unsupported("INVENTORY_NOT_EVALUATED")
    };

    let counters = if !inputs.decompression.valid {
        unsupported(container_failure_code)
    } else if !inputs.campaign_applicable {
        unsupported("UNSUPPORTED_CONTEXT")
    } else if let Some(readout) = inputs.region_counters {
        if !readout.guard_passed {
            unsupported("UNSUPPORTED_LAYOUT")
        } else if !readout.spec_errors.is_empty() {
            degraded(["COUNTER_MAPPING_INVALID"])
        } else if readout.counters.is_empty() {
            unsupported("COUNTER_MAPPING_UNAVAILABLE")
        } else if readout
            .counters
            .iter()
            .any(|counter| counter.support_status != SupportStatus::Supported)
        {
            degraded(["COUNTER_MISMATCH"])
        } else {
            supported()
        }
    } else {
        unsupported("COUNTERS_NOT_EVALUATED")
    };

    Capabilities {
        container,
        sections,
        inventory,
        counters,
        reference_difference: unsupported("NOT_EVALUATED"),
        location: unsupported("NOT_EVALUATED"),
    }
}

fn supported() -> Capability {
    capability(SupportStatus::Supported, std::iter::empty())
}

fn unsupported(code: &str) -> Capability {
    capability(SupportStatus::Unsupported, [code])
}

fn degraded<'a>(codes: impl IntoIterator<Item = &'a str>) -> Capability {
    capability(SupportStatus::Partial, codes)
}

fn capability<'a>(status: SupportStatus, codes: impl IntoIterator<Item = &'a str>) -> Capability {
    let mut reason_codes: Vec<String> = Vec::new();
    for code in codes {
        if !reason_codes.iter().any(|existing| existing == code) {
            reason_codes.push(code.to_string());
        }
    }
    Capability {
        status,
        reason_codes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Decompression;
    use crate::domain::context::{DeclaredContext, assess};
    use crate::domain::progress::{
        OVERALL_OFFSET, REGION_COUNTER_BASE, RegionCounterSpec, overall_completion, region_counters,
    };
    use crate::inventory::InventoryRecovery;
    use crate::limits::Limits;
    use crate::report::{ContextSource, EvidenceLevel, Mode};
    use crate::synthetic::{SampleBuilder, compress};

    struct Evaluated {
        decompression: Decompression,
        sections: SectionStream,
        inventory: InventoryRecovery,
        counters: RegionCounterReadout,
    }

    fn evaluate(raw: &[u8], specs: &[RegionCounterSpec]) -> Evaluated {
        let decompression = crate::container::decompress(&compress(raw), &Limits::default());
        let sections = SectionStream::walk(&decompression.data, &Limits::default());
        let inventory = crate::inventory::recover(&decompression.data, &sections);
        let counters = region_counters(&decompression.data, &sections, specs);
        Evaluated {
            decompression,
            sections,
            inventory,
            counters,
        }
    }

    fn campaign_context() -> ContextAssessment {
        assess(&DeclaredContext {
            platform: Some("windows_steam".to_string()),
            mode: Mode::Campaign,
            source: ContextSource::UserDeclared,
            ..DeclaredContext::default()
        })
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

    fn good_sample() -> Vec<u8> {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_f32_le(OVERALL_OFFSET, 66.204_82_f32);
        builder.write_prefix_slot(0, 0x0F28_B6EC, 1);
        builder.write_u32_le(REGION_COUNTER_BASE, 0);
        builder.finish()
    }

    fn matrix_for(evaluated: &Evaluated, assessment: &ContextAssessment) -> Capabilities {
        capability_matrix(&CapabilityInputs {
            decompression: &evaluated.decompression,
            sections: &evaluated.sections,
            campaign_applicable: assessment.campaign_applicable,
            inventory: Some(&evaluated.inventory),
            region_counters: Some(&evaluated.counters),
        })
    }

    fn codes(capability: &Capability) -> Vec<&str> {
        capability.reason_codes.iter().map(String::as_str).collect()
    }

    #[test]
    fn campaign_with_all_layers_supported() {
        let counter_spec = spec(
            "counter:mission_of_san_juan:survival_caches",
            REGION_COUNTER_BASE as u64,
            8,
        );
        let evaluated = evaluate(&good_sample(), std::slice::from_ref(&counter_spec));
        let assessment = campaign_context();
        let matrix = matrix_for(&evaluated, &assessment);

        assert_eq!(matrix.container.status, SupportStatus::Supported);
        assert_eq!(matrix.sections.status, SupportStatus::Supported);
        assert_eq!(matrix.inventory.status, SupportStatus::Supported);
        assert_eq!(matrix.counters.status, SupportStatus::Supported);
        assert_eq!(
            matrix.reference_difference.status,
            SupportStatus::Unsupported
        );
        assert_eq!(codes(&matrix.reference_difference), vec!["NOT_EVALUATED"]);

        // 未知构建不阻断有限观测与带限制的社区候选。
        assert!(
            assessment
                .context
                .limitations
                .join(" ")
                .contains("构建号未知")
        );
        let overall = overall_completion(&evaluated.decompression.data, &evaluated.sections);
        assert_eq!(overall.progress.support_status, SupportStatus::Supported);
        assert_eq!(
            overall.progress.evidence_level,
            EvidenceLevel::CommunityCorrelated
        );
    }

    #[test]
    fn profile_context_degrades_inventory_and_counters_independently() {
        let raw = vec![0u8; 25_600];
        let counter_spec = spec("counter:any", REGION_COUNTER_BASE as u64, 8);
        let evaluated = evaluate(&raw, std::slice::from_ref(&counter_spec));
        let assessment = assess(&DeclaredContext {
            mode: Mode::Profile,
            ..DeclaredContext::default()
        });
        let matrix = matrix_for(&evaluated, &assessment);

        assert_eq!(
            matrix.container.status,
            SupportStatus::Supported,
            "容器观察仍可用"
        );
        assert_eq!(matrix.sections.status, SupportStatus::Unsupported);
        assert_eq!(codes(&matrix.sections), vec!["UNSUPPORTED_CONTEXT"]);
        assert_eq!(matrix.inventory.status, SupportStatus::Unsupported);
        assert_eq!(codes(&matrix.inventory), vec!["UNSUPPORTED_CONTEXT"]);
        assert_eq!(matrix.counters.status, SupportStatus::Unsupported);
        assert_eq!(codes(&matrix.counters), vec!["UNSUPPORTED_CONTEXT"]);
    }

    #[test]
    fn unknown_mode_closes_collection_semantics_without_touching_structure() {
        let counter_spec = spec(
            "counter:mission_of_san_juan:survival_caches",
            REGION_COUNTER_BASE as u64,
            8,
        );
        let evaluated = evaluate(&good_sample(), std::slice::from_ref(&counter_spec));
        let assessment = assess(&DeclaredContext::default());
        assert!(!assessment.campaign_applicable);
        let matrix = matrix_for(&evaluated, &assessment);

        assert_eq!(matrix.container.status, SupportStatus::Supported);
        assert_eq!(
            matrix.sections.status,
            SupportStatus::Supported,
            "结构识别不受模式声明影响"
        );
        assert_eq!(codes(&matrix.inventory), vec!["UNSUPPORTED_CONTEXT"]);
        assert_eq!(codes(&matrix.counters), vec!["UNSUPPORTED_CONTEXT"]);
    }

    #[test]
    fn corrupt_container_closes_dependents_with_container_reason() {
        let mut blob = compress(&good_sample());
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        let decompression = crate::container::decompress(&blob, &Limits::default());
        let sections = SectionStream::walk(&decompression.data, &Limits::default());
        let inventory = crate::inventory::recover(&decompression.data, &sections);
        let counters = region_counters(&decompression.data, &sections, &[]);
        let evaluated = Evaluated {
            decompression,
            sections,
            inventory,
            counters,
        };
        let matrix = matrix_for(&evaluated, &campaign_context());

        assert_eq!(matrix.container.status, SupportStatus::Unsupported);
        assert_eq!(codes(&matrix.container), vec!["DECOMPRESSION_FAILED"]);
        assert_eq!(codes(&matrix.sections), vec!["DECOMPRESSION_FAILED"]);
        assert_eq!(codes(&matrix.inventory), vec!["DECOMPRESSION_FAILED"]);
        assert_eq!(codes(&matrix.counters), vec!["DECOMPRESSION_FAILED"]);
    }

    #[test]
    fn inventory_partial_degrades_with_layout_reason_only() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.push_section(28, 4104); // 总长 4108：不符合 4112
        builder.write_prefix_slot(0, 0x1111_1111, 1);
        let counter_spec = spec(
            "counter:mission_of_san_juan:survival_caches",
            REGION_COUNTER_BASE as u64,
            8,
        );
        let evaluated = evaluate(&builder.finish(), std::slice::from_ref(&counter_spec));
        let matrix = matrix_for(&evaluated, &campaign_context());

        assert_eq!(matrix.inventory.status, SupportStatus::Partial);
        assert_eq!(codes(&matrix.inventory), vec!["UNSUPPORTED_LAYOUT"]);
        assert_eq!(matrix.counters.status, SupportStatus::Supported);
    }

    #[test]
    fn counter_over_target_degrades_counters_only() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_u32_le(REGION_COUNTER_BASE, 99);
        let counter_spec = spec(
            "counter:mission_of_san_juan:survival_caches",
            REGION_COUNTER_BASE as u64,
            8,
        );
        let evaluated = evaluate(&builder.finish(), std::slice::from_ref(&counter_spec));
        let matrix = matrix_for(&evaluated, &campaign_context());

        assert_eq!(matrix.counters.status, SupportStatus::Partial);
        assert_eq!(codes(&matrix.counters), vec!["COUNTER_MISMATCH"]);
        assert_eq!(matrix.inventory.status, SupportStatus::Supported);
        assert_eq!(matrix.container.status, SupportStatus::Supported);
    }

    #[test]
    fn missing_counter_mapping_is_unsupported_not_zero() {
        let evaluated = evaluate(&good_sample(), &[]);
        assert!(evaluated.counters.counters.is_empty());
        let matrix = matrix_for(&evaluated, &campaign_context());

        assert_eq!(matrix.counters.status, SupportStatus::Unsupported);
        assert_eq!(codes(&matrix.counters), vec!["COUNTER_MAPPING_UNAVAILABLE"]);
        assert_eq!(matrix.inventory.status, SupportStatus::Supported);
    }
}
