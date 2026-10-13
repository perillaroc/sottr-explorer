//! SX-007 公共接口端到端：合成样本 → 各层观测 → 参考差集评估 → 证据 → 能力矩阵。

use std::collections::{BTreeMap, BTreeSet};

use sottr_core::container::decompress;
use sottr_core::domain::assessment::{
    AssessmentInputs, CrossCheckInputs, ReferenceCategoryView, ReferenceView,
    assess as assess_reference,
};
use sottr_core::domain::context::{DeclaredContext, assess as assess_context};
use sottr_core::domain::evidence::{build_evidence, references_resolve};
use sottr_core::domain::progress::{
    OVERALL_OFFSET, REGION_COUNTER_BASE, RegionCounterSpec, overall_completion, region_counters,
};
use sottr_core::domain::{CapabilityInputs, capability_matrix};
use sottr_core::inventory::recover;
use sottr_core::limits::Limits;
use sottr_core::report::{
    AssessmentKind, ContextSource, InternalId, Mode, ObservationKind, SupportStatus,
};
use sottr_core::sections::SectionStream;
use sottr_core::synthetic::{SampleBuilder, compress};

fn id(value: u32) -> InternalId {
    InternalId::new(value)
}

#[test]
fn full_pipeline_yields_likely_states_evidence_and_supported_matrix() {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    builder.write_f32_le(OVERALL_OFFSET, 66.204_82_f32);
    builder.write_prefix_slot(0, 0x0F28_B6EC, 1);
    builder.write_prefix_slot(1, 0x0F28_B6ED, 3);
    let overflow = builder.push_section(28, 4108);
    builder.write_u32_le(overflow.offset + 16, 0x439B_19A6);
    builder.write_u32_le(REGION_COUNTER_BASE, 2);
    let raw = builder.finish();

    let decompression = decompress(&compress(&raw), &Limits::default());
    assert!(decompression.valid, "{:?}", decompression.error);
    let sections = SectionStream::walk(&decompression.data, &Limits::default());
    assert!(sections.valid, "{:?}", sections.error);
    let inventory = recover(&decompression.data, &sections);
    assert!(inventory.observation.inventory_complete);

    let specs = [RegionCounterSpec {
        counter_key: "counter:mission_of_san_juan:survival_caches".to_string(),
        region_id: Some("mission_of_san_juan".to_string()),
        category_id: "survival_caches".to_string(),
        serialized_offset: REGION_COUNTER_BASE as u64,
        source_total: 8,
    }];
    let counters = region_counters(&decompression.data, &sections, &specs);
    let overall = overall_completion(&decompression.data, &sections);

    let reference = ReferenceView {
        reference_set_id: "rs:stella-100pct-common".to_string(),
        version: "1.0.0".to_string(),
        applicable: true,
        applicability_limits: Vec::new(),
        categories: vec![
            ReferenceCategoryView {
                category_id: "survival_caches".to_string(),
                reference_ids: [0x0F28_B6EC, 0x0F28_B6ED, 0x0F28_B6EE, 0x0F28_B6EF]
                    .into_iter()
                    .map(id)
                    .collect(),
            },
            ReferenceCategoryView {
                category_id: "monoliths".to_string(),
                reference_ids: [0x439B_19A6].into_iter().map(id).collect(),
            },
        ],
    };
    let observed: BTreeMap<String, BTreeSet<InternalId>> = [
        ("survival_caches", vec![0x0F28_B6EC, 0x0F28_B6ED]),
        ("monoliths", vec![0x439B_19A6]),
    ]
    .into_iter()
    .map(|(category, ids)| (category.to_string(), ids.into_iter().map(id).collect()))
    .collect();
    let cross = CrossCheckInputs {
        category_counter_totals: [
            ("survival_caches".to_string(), 2),
            ("monoliths".to_string(), 1),
        ]
        .into_iter()
        .collect(),
        counter_totals_degraded: false,
        metadata_conflicts: Vec::new(),
    };

    let context = assess_context(&DeclaredContext {
        platform: Some("windows_steam".to_string()),
        mode: Mode::Campaign,
        source: ContextSource::UserDeclared,
        ..DeclaredContext::default()
    });
    let outcome = assess_reference(&AssessmentInputs {
        context: &context,
        inventory: &inventory,
        reference: &reference,
        observed_category_ids: &observed,
        cross_checks: &cross,
    });

    assert_eq!(outcome.capability.status, SupportStatus::Supported);
    let find = |value: u32| {
        outcome
            .items
            .iter()
            .find(|item| item.internal_id == id(value))
            .unwrap_or_else(|| panic!("missing {value:#X}"))
    };
    assert_eq!(
        find(0x0F28_B6EC).assessment,
        AssessmentKind::LikelyCollected
    );
    assert_eq!(
        find(0x439B_19A6).assessment,
        AssessmentKind::LikelyCollected
    );
    assert_eq!(find(0x0F28_B6EE).assessment, AssessmentKind::LikelyMissing);
    assert_eq!(
        find(0x0F28_B6EE).observation,
        ObservationKind::ReferencePresentCurrentAbsent
    );
    assert_eq!(find(0x0F28_B6EF).assessment, AssessmentKind::LikelyMissing);
    let survival = outcome
        .diff
        .iter()
        .find(|diff| diff.category_id == "survival_caches")
        .unwrap();
    assert_eq!(survival.absent_ids, vec![id(0x0F28_B6EE), id(0x0F28_B6EF)]);

    let evidence = build_evidence(
        Some(&overall),
        &sections,
        &inventory,
        Some((&counters, &specs)),
        Some(&reference),
    );
    assert!(references_resolve(&outcome.items, &evidence));

    let matrix = capability_matrix(&CapabilityInputs {
        decompression: &decompression,
        sections: &sections,
        campaign_applicable: context.campaign_applicable,
        inventory: Some(&inventory),
        region_counters: Some(&counters),
        reference_difference: Some(&outcome),
    });
    assert_eq!(matrix.reference_difference.status, SupportStatus::Supported);
    assert!(matrix.reference_difference.reason_codes.is_empty());
    assert_eq!(matrix.container.status, SupportStatus::Supported);
    assert_eq!(matrix.counters.status, SupportStatus::Supported);
}
