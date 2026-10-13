//! 证据构造器（SX-007）：把各层观测装配为契约证据条目。
//!
//! 稳定证据 ID 约定（与观测层写入的 `evidence_refs` 一致）：
//! - `ev:sections:main`：主区段流步进；
//! - `ev:overall:0x28`：整体完成度 float32 读数；
//! - `ev:inventory:prefix`：前缀表恢复；
//! - `ev:inventory:type28`：type28 溢出表恢复；
//! - `ev:type28-mismatch:0x…`：长度异常的 type28 段；
//! - `ev:counter:…`：区域计数读数；
//! - `ev:refdiff:<reference_set_id>`：参考集合差集。
//!
//! 来源引用（`source_refs`）在 SX-008 元数据接线前为空数组，不虚构来源。

use std::collections::BTreeMap;

use crate::inventory::{InventoryReason, InventoryRecovery, PREFIX_OFFSET, TYPE28_TOTAL_BYTES};
use crate::report::{Evidence, SupportStatus};
use crate::sections::{MAIN_STREAM_START, SectionStream};

use super::assessment::ReferenceView;
use super::progress::{OverallReadout, RegionCounterReadout, RegionCounterSpec};

/// 主区段流证据 ID。
pub const EVIDENCE_SECTIONS: &str = "ev:sections:main";

/// 整体完成度证据 ID。
pub const EVIDENCE_OVERALL: &str = "ev:overall:0x28";

/// 前缀表证据 ID。
pub const EVIDENCE_PREFIX: &str = "ev:inventory:prefix";

/// type28 溢出表证据 ID。
pub const EVIDENCE_TYPE28: &str = "ev:inventory:type28";

/// 构造证据列表（按证据 ID 排序；不虚构来源）。
pub fn build_evidence(
    overall: Option<&OverallReadout>,
    sections: &SectionStream,
    inventory: &InventoryRecovery,
    counters: Option<(&RegionCounterReadout, &[RegionCounterSpec])>,
    reference: Option<&ReferenceView>,
) -> Vec<Evidence> {
    let mut evidence: Vec<Evidence> = Vec::new();

    if sections.valid {
        evidence.push(Evidence {
            evidence_id: EVIDENCE_SECTIONS.to_string(),
            method: "packed section stream walk".to_string(),
            source_refs: Vec::new(),
            sample_scope: "campaign_family_4mib".to_string(),
            limitations: Vec::new(),
            offset: Some(MAIN_STREAM_START as u64),
            offset_hex: Some(format!("0x{MAIN_STREAM_START:X}")),
            record_index: None,
            reference_set_id: None,
            cross_checks: vec![format!("section_count={}", sections.section_count())],
            source_versions: BTreeMap::new(),
        });
    }

    if let Some(readout) = overall {
        evidence.push(Evidence {
            evidence_id: EVIDENCE_OVERALL.to_string(),
            method: "float32_le read at fixed offset".to_string(),
            source_refs: Vec::new(),
            sample_scope: "campaign_family_4mib".to_string(),
            limitations: readout.progress.limitations.clone(),
            offset: Some(super::progress::OVERALL_OFFSET as u64),
            offset_hex: Some("0x28".to_string()),
            record_index: None,
            reference_set_id: None,
            cross_checks: vec!["公开 100% 存档在 0x28 处为 100.0".to_string()],
            source_versions: BTreeMap::new(),
        });
    }

    let observation = &inventory.observation;
    if observation.status != SupportStatus::Unsupported {
        evidence.push(Evidence {
            evidence_id: EVIDENCE_PREFIX.to_string(),
            method: "prefix u32 pair read".to_string(),
            source_refs: Vec::new(),
            sample_scope: "campaign_family_4mib".to_string(),
            limitations: Vec::new(),
            offset: Some(PREFIX_OFFSET as u64),
            offset_hex: Some(format!("0x{PREFIX_OFFSET:X}")),
            record_index: None,
            reference_set_id: None,
            cross_checks: vec![format!(
                "record_count={} = unique_id_count={} + duplicate_record_count={}",
                observation.record_count,
                observation.unique_id_count,
                observation.duplicate_record_count
            )],
            source_versions: BTreeMap::new(),
        });

        let type28_sections: Vec<&crate::sections::Section> = sections
            .sections
            .iter()
            .filter(|section| section.section_type == 28)
            .collect();
        let first_offset = type28_sections.first().map(|section| section.offset);
        evidence.push(Evidence {
            evidence_id: EVIDENCE_TYPE28.to_string(),
            method: "type28 overflow table read".to_string(),
            source_refs: Vec::new(),
            sample_scope: "campaign_family_4mib".to_string(),
            limitations: vec![format!(
                "仅读取总长为 {TYPE28_TOTAL_BYTES} 字节的 type28 段"
            )],
            offset: first_offset,
            offset_hex: first_offset.map(|offset| format!("0x{offset:X}")),
            record_index: None,
            reference_set_id: None,
            cross_checks: vec![format!("type28_section_count={}", type28_sections.len())],
            source_versions: BTreeMap::new(),
        });
    }

    for reason in &inventory.reasons {
        if let InventoryReason::Type28LengthMismatch { offset, bytes } = reason {
            evidence.push(Evidence {
                evidence_id: format!("ev:type28-mismatch:0x{offset:X}"),
                method: "type28 length check".to_string(),
                source_refs: Vec::new(),
                sample_scope: "current_input_only".to_string(),
                limitations: vec![reason.message()],
                offset: Some(*offset),
                offset_hex: Some(format!("0x{offset:X}")),
                record_index: None,
                reference_set_id: None,
                cross_checks: vec![format!("observed_bytes={bytes}")],
                source_versions: BTreeMap::new(),
            });
        }
    }

    if let Some((readout, specs)) = counters {
        for counter in &readout.counters {
            let offset = specs
                .iter()
                .find(|spec| spec.counter_key == counter.counter_key)
                .map(|spec| spec.serialized_offset);
            evidence.push(Evidence {
                evidence_id: format!("ev:{}", counter.counter_key),
                method: "u32_le read at mapped offset".to_string(),
                source_refs: Vec::new(),
                sample_scope: "campaign_family_4mib".to_string(),
                limitations: counter.limitations.clone(),
                offset,
                offset_hex: offset.map(|value| format!("0x{value:X}")),
                record_index: None,
                reference_set_id: None,
                cross_checks: vec!["分类别唯一 ID 数与计数汇总交叉检查".to_string()],
                source_versions: BTreeMap::new(),
            });
        }
    }

    if let Some(reference) = reference {
        let cross_checks: Vec<String> = reference
            .categories
            .iter()
            .map(|category| {
                format!(
                    "{}: reference_count={}",
                    category.category_id,
                    category.reference_ids.len()
                )
            })
            .collect();
        let mut source_versions = BTreeMap::new();
        source_versions.insert(
            reference.reference_set_id.clone(),
            reference.version.clone(),
        );
        evidence.push(Evidence {
            evidence_id: format!("ev:refdiff:{}", reference.reference_set_id),
            method: "set difference against bounded reference set".to_string(),
            source_refs: Vec::new(),
            sample_scope: "two_author_labeled_100pct_slots".to_string(),
            limitations: vec!["仅限共同参考类别；差集为空不等于全游戏收集完成".to_string()],
            offset: None,
            offset_hex: None,
            record_index: None,
            reference_set_id: Some(reference.reference_set_id.clone()),
            cross_checks,
            source_versions,
        });
    }

    evidence.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
    evidence
}

/// 评估条目引用的证据 ID 是否都在给定证据列表中存在。
pub fn references_resolve(items: &[crate::report::AssessedItem], evidence: &[Evidence]) -> bool {
    items.iter().all(|item| {
        item.evidence_refs
            .iter()
            .all(|reference| evidence.iter().any(|entry| &entry.evidence_id == reference))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::container::decompress;
    use crate::domain::assessment::{
        AssessmentInputs, CrossCheckInputs, ReferenceCategoryView, assess as assess_reference,
    };
    use crate::domain::context::{DeclaredContext, assess as assess_context};
    use crate::domain::progress::{
        OVERALL_OFFSET, REGION_COUNTER_BASE, RegionCounterSpec, overall_completion, region_counters,
    };
    use crate::limits::Limits;
    use crate::report::{ContextSource, InternalId, Mode};
    use crate::synthetic::{SampleBuilder, compress};

    fn counter_spec() -> RegionCounterSpec {
        RegionCounterSpec {
            counter_key: "counter:mission_of_san_juan:survival_caches".to_string(),
            region_id: Some("mission_of_san_juan".to_string()),
            category_id: "survival_caches".to_string(),
            serialized_offset: REGION_COUNTER_BASE as u64,
            source_total: 8,
        }
    }

    #[test]
    fn builds_stable_and_resolvable_evidence() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        builder.write_f32_le(OVERALL_OFFSET, 66.204_82_f32);
        builder.write_prefix_slot(0, 0x0F28_B6EC, 1);
        let overflow = builder.push_section(28, 4108);
        builder.write_u32_le(overflow.offset + 16, 0x439B_19A6);
        let raw = builder.finish();

        let decompression = decompress(&compress(&raw), &Limits::default());
        let sections = SectionStream::walk(&decompression.data, &Limits::default());
        let inventory = crate::inventory::recover(&decompression.data, &sections);
        let specs = [counter_spec()];
        let counters = region_counters(&decompression.data, &sections, &specs);
        let overall = overall_completion(&decompression.data, &sections);
        let reference = ReferenceView {
            reference_set_id: "rs:test".to_string(),
            version: "1.0.0".to_string(),
            applicable: true,
            applicability_limits: Vec::new(),
            categories: vec![
                ReferenceCategoryView {
                    category_id: "survival_caches".to_string(),
                    reference_ids: [0x0F28_B6EC].into_iter().map(InternalId::new).collect(),
                },
                ReferenceCategoryView {
                    category_id: "monoliths".to_string(),
                    reference_ids: [0x439B_19A6, 0x1111_1111]
                        .into_iter()
                        .map(InternalId::new)
                        .collect(),
                },
            ],
        };

        let evidence = build_evidence(
            Some(&overall),
            &sections,
            &inventory,
            Some((&counters, &specs)),
            Some(&reference),
        );
        let ids: Vec<&str> = evidence
            .iter()
            .map(|entry| entry.evidence_id.as_str())
            .collect();
        for expected in [
            EVIDENCE_SECTIONS,
            EVIDENCE_OVERALL,
            EVIDENCE_PREFIX,
            EVIDENCE_TYPE28,
            "ev:refdiff:rs:test",
            "ev:counter:mission_of_san_juan:survival_caches",
        ] {
            assert!(ids.contains(&expected), "缺少 {expected}: {ids:?}");
        }
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "证据 ID 必须唯一");

        let prefix = evidence
            .iter()
            .find(|entry| entry.evidence_id == EVIDENCE_PREFIX)
            .unwrap();
        assert_eq!(prefix.offset, Some(68));
        assert_eq!(prefix.offset_hex.as_deref(), Some("0x44"));
        let refdiff = evidence
            .iter()
            .find(|entry| entry.evidence_id == "ev:refdiff:rs:test")
            .unwrap();
        assert_eq!(refdiff.reference_set_id.as_deref(), Some("rs:test"));
        assert_eq!(
            refdiff.source_versions.get("rs:test").map(String::as_str),
            Some("1.0.0")
        );

        let context = assess_context(&DeclaredContext {
            mode: Mode::Campaign,
            source: ContextSource::UserDeclared,
            ..DeclaredContext::default()
        });
        let mut observed = BTreeMap::new();
        observed.insert(
            "survival_caches".to_string(),
            [0x0F28_B6EC].into_iter().map(InternalId::new).collect(),
        );
        observed.insert(
            "monoliths".to_string(),
            [0x439B_19A6].into_iter().map(InternalId::new).collect(),
        );
        let assessed = assess_reference(&AssessmentInputs {
            context: &context,
            inventory: &inventory,
            reference: &reference,
            observed_category_ids: &observed,
            cross_checks: &CrossCheckInputs::default(),
        });
        assert!(
            assessed
                .items
                .iter()
                .any(|entry| entry.internal_id == InternalId::new(0x1111_1111)),
            "参考中存在而当前表缺失的 ID 应进入评估"
        );
        assert!(
            references_resolve(&assessed.items, &evidence),
            "评估引用必须能解析到证据"
        );
    }

    #[test]
    fn mismatch_evidence_carries_offset_and_limitation() {
        let mut builder = SampleBuilder::new();
        builder.push_section(1, 0);
        let bad = builder.push_section(28, 4104);
        let raw = builder.finish();

        let decompression = decompress(&compress(&raw), &Limits::default());
        let sections = SectionStream::walk(&decompression.data, &Limits::default());
        let inventory = crate::inventory::recover(&decompression.data, &sections);
        let evidence = build_evidence(None, &sections, &inventory, None, None);

        let mismatch = evidence
            .iter()
            .find(|entry| entry.evidence_id.starts_with("ev:type28-mismatch:"))
            .expect("长度异常应有对应证据");
        assert_eq!(mismatch.offset, Some(bad.offset as u64));
        assert!(
            mismatch
                .limitations
                .iter()
                .any(|text| text.contains("不符合已知布局"))
        );
    }
}
