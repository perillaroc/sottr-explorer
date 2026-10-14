//! SX-006 公共接口端到端：上下文声明 → 进度观测 → 区域计数 → 能力矩阵。

use sottr_core::container::decompress;
use sottr_core::domain::context::{DeclaredContext, assess};
use sottr_core::domain::progress::{
    OVERALL_OFFSET, REGION_COUNTER_BASE, RegionCounterSpec, overall_completion, region_counters,
};
use sottr_core::domain::{CapabilityInputs, capability_matrix};
use sottr_core::inventory::recover;
use sottr_core::limits::Limits;
use sottr_core::report::{ContextSource, Mode, SupportStatus};
use sottr_core::sections::SectionStream;
use sottr_core::synthetic::{SampleBuilder, compress};

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
fn campaign_flow_keeps_declaration_limits_and_community_candidates() {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    builder.write_f32_le(OVERALL_OFFSET, 66.204_82_f32);
    builder.write_prefix_slot(0, 0x0F28_B6EC, 1);
    builder.write_u32_le(REGION_COUNTER_BASE, 0);
    let raw = builder.finish();

    let decompression = decompress(&compress(&raw), &Limits::default());
    assert!(decompression.valid, "{:?}", decompression.error);
    let sections = SectionStream::walk(&decompression.data, &Limits::default());
    assert!(sections.valid, "{:?}", sections.error);
    let inventory = recover(&decompression.data, &sections);
    let specs = [counter_spec()];
    let counters = region_counters(&decompression.data, &sections, &specs);
    let overall = overall_completion(&decompression.data, &sections);

    let assessment = assess(&DeclaredContext {
        platform: Some("windows_steam".to_string()),
        mode: Mode::Campaign,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    });
    assert!(assessment.campaign_applicable);

    let matrix = capability_matrix(&CapabilityInputs {
        decompression: &decompression,
        sections: &sections,
        campaign_applicable: assessment.campaign_applicable,
        inventory: Some(&inventory),
        region_counters: Some(&counters),
        reference_difference: None,
        location: None,
    });
    assert_eq!(matrix.container.status, SupportStatus::Supported);
    assert_eq!(matrix.sections.status, SupportStatus::Supported);
    assert_eq!(matrix.inventory.status, SupportStatus::Supported);
    assert_eq!(matrix.counters.status, SupportStatus::Supported);

    assert_eq!(overall.progress.value, Some(f64::from(66.204_82_f32)));
    assert_eq!(counters.counters[0].value, Some(0));
    assert_eq!(counters.counters[0].source_target, Some(8));

    let limitations = assessment.context.limitations.join(" ");
    assert!(limitations.contains("不是格式识别证明"), "{limitations}");
    assert!(limitations.contains("构建号未知"), "{limitations}");
}

#[test]
fn challenge_tomb_like_input_without_declaration_closes_collection_semantics() {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    builder.write_f32_le(OVERALL_OFFSET, 0.0);
    let raw = builder.finish();

    let decompression = decompress(&compress(&raw), &Limits::default());
    let sections = SectionStream::walk(&decompression.data, &Limits::default());
    assert!(sections.valid, "结构校验可以通过");

    let inventory = recover(&decompression.data, &sections);
    let specs = [counter_spec()];
    let counters = region_counters(&decompression.data, &sections, &specs);

    // 未提供可靠上下文：不得仅凭结构开启收集语义。
    let assessment = assess(&DeclaredContext::default());
    assert!(!assessment.campaign_applicable);
    assert_eq!(assessment.context.source, ContextSource::Unknown);

    let matrix = capability_matrix(&CapabilityInputs {
        decompression: &decompression,
        sections: &sections,
        campaign_applicable: assessment.campaign_applicable,
        inventory: Some(&inventory),
        region_counters: Some(&counters),
        reference_difference: None,
        location: None,
    });
    assert_eq!(matrix.container.status, SupportStatus::Supported);
    assert_eq!(matrix.sections.status, SupportStatus::Supported);
    assert_eq!(matrix.inventory.status, SupportStatus::Unsupported);
    assert_eq!(matrix.inventory.reason_codes, vec!["UNSUPPORTED_CONTEXT"]);
    assert_eq!(matrix.counters.status, SupportStatus::Unsupported);
    assert_eq!(matrix.counters.reason_codes, vec!["UNSUPPORTED_CONTEXT"]);
}
