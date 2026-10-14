//! SX-008 组级定位门禁测试（验收矩阵 T19）：
//! 零/非零/未知/冲突/不完整/过期/参考不适用/组关联不确认场景。
//!
//! 门禁必须由核心随当前报告评估；前端不得从“0/8”文案自行推导。

use std::collections::BTreeSet;

use sottr_core::domain::assessment::ReferenceView;
use sottr_core::domain::context::{ContextAssessment, DeclaredContext, assess};
use sottr_core::domain::location::{
    GATE_COMPLETE_TABLE, GATE_CONTEXT_NOT_CAMPAIGN, GATE_GROUP_ASSOCIATION_UNCONFIRMED,
    GATE_GROUP_COUNT_CONFLICT, GATE_GROUP_COUNT_NONZERO, GATE_GROUP_COUNT_UNKNOWN,
    GATE_INVENTORY_INCOMPLETE, GATE_REFERENCE_MATCH, GATE_REFERENCE_NOT_APPLICABLE,
    GATE_SNAPSHOT_NOT_CURRENT, GATE_ZERO_GROUP_COUNT, GroupGateInputs, evaluate_group_gate,
};
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::report::{
    ContextSource, EvidenceLevel, Freshness, GroupActionKind, GroupActionability, InputDigest,
    InventoryObservation, Mode, RegionCounter, SupportStatus,
};

const REFERENCE_SET_ID: &str = "rs:stella-100pct-common";
const SAN_JUAN_CACHES: &str = "stella:san-juan-caches";
const SAN_JUAN_MONOLITH: &str = "stella:san-juan-monolith";
const SAN_JUAN_CRYPT: &str = "stella:san-juan-crypt";

fn real_bundle() -> MetadataBundle {
    let files = [
        (
            "collectibles.json",
            include_str!("../../../data/metadata/collectibles.json"),
        ),
        (
            "region_counters.json",
            include_str!("../../../data/metadata/region_counters.json"),
        ),
        (
            "reference_sets.json",
            include_str!("../../../data/metadata/reference_sets.json"),
        ),
    ];
    let inputs: Vec<MetadataFileInput<'_>> = files
        .iter()
        .map(|(file_id, text)| MetadataFileInput {
            file_id: (*file_id).to_string(),
            bytes: text.as_bytes(),
        })
        .collect();
    MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap()
}

fn campaign_context(platform: Option<&str>) -> ContextAssessment {
    assess(&DeclaredContext {
        platform: platform.map(str::to_string),
        mode: Mode::Campaign,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    })
}

fn digest() -> InputDigest {
    InputDigest {
        compressed_sha256: "a".repeat(64),
        expanded_sha256: Some("b".repeat(64)),
        compressed_bytes: 640_000,
        expanded_bytes: Some(4_194_304),
    }
}

fn inventory(complete: bool) -> InventoryObservation {
    InventoryObservation {
        status: if complete {
            SupportStatus::Supported
        } else {
            SupportStatus::Partial
        },
        inventory_complete: complete,
        completeness_scope: "test".to_string(),
        records: Vec::new(),
        record_count: 0,
        unique_id_count: 0,
        duplicate_record_count: 0,
    }
}

fn counter(key: &str, category: &str, value: Option<u32>, raw: u32) -> RegionCounter {
    RegionCounter {
        counter_key: key.to_string(),
        region_id: Some("mission_of_san_juan".to_string()),
        category_id: category.to_string(),
        value,
        unit: "count".to_string(),
        support_status: if value.is_some() {
            SupportStatus::Supported
        } else {
            SupportStatus::Partial
        },
        reason: value
            .is_none()
            .then(|| "原始读数超过来源目标，候选计数与剩余置 null，不裁剪到目标值".to_string()),
        evidence_level: if value.is_some() {
            EvidenceLevel::CommunityCorrelated
        } else {
            EvidenceLevel::Observed
        },
        evidence_refs: vec![format!("ev:{key}")],
        limitations: vec!["来源目标来自社区脚本".to_string()],
        source_target: None,
        within_source_target: value.is_some(),
        raw_value: raw,
        raw_value_hex: format!("0x{raw:08X}"),
    }
}

fn san_juan_counters(value: Option<u32>, raw: u32) -> Vec<RegionCounter> {
    vec![
        counter(
            "counter:mission_of_san_juan:survival_caches",
            "survival_caches",
            value,
            raw,
        ),
        counter(
            "counter:mission_of_san_juan:monoliths",
            "monoliths",
            value.map(|_| 0),
            raw,
        ),
        counter(
            "counter:mission_of_san_juan:crypts",
            "crypts",
            value.map(|_| 0),
            raw,
        ),
    ]
}

fn evaluate(
    bundle: &MetadataBundle,
    group_id: &str,
    counters: &[RegionCounter],
    freshness: Freshness,
    context: &ContextAssessment,
    reference: &ReferenceView,
    inventory: &InventoryObservation,
) -> GroupActionability {
    let group = bundle.guide_group(group_id).unwrap();
    let specs = bundle.region_counter_specs();
    let digest = digest();
    evaluate_group_gate(&GroupGateInputs {
        group,
        counter_specs: &specs,
        counters,
        context,
        inventory,
        reference,
        freshness,
        input_digest: &digest,
    })
}

#[test]
fn zero_counts_allow_whole_group_check_for_all_three_san_juan_groups() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    let counters = san_juan_counters(Some(0), 0);
    let inventory = inventory(true);

    for group_id in [SAN_JUAN_CACHES, SAN_JUAN_MONOLITH, SAN_JUAN_CRYPT] {
        let action = evaluate(
            &bundle,
            group_id,
            &counters,
            Freshness::Current,
            &context,
            &reference,
            &inventory,
        );
        assert_eq!(action.kind, GroupActionKind::WholeGroupCheck, "{group_id}");
        assert_eq!(
            action.reason_codes,
            vec![
                GATE_COMPLETE_TABLE,
                GATE_ZERO_GROUP_COUNT,
                GATE_REFERENCE_MATCH
            ],
            "{group_id}"
        );
        assert!(!action.evidence_refs.is_empty(), "{group_id}");
        assert_eq!(action.evaluated_input_digest, digest(), "{group_id}");
    }
}

#[test]
fn nonzero_group_count_revokes_whole_group_check() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &san_juan_counters(Some(3), 3),
        Freshness::Current,
        &context,
        &reference,
        &inventory(true),
    );
    assert_eq!(action.kind, GroupActionKind::GeneralReference);
    assert_eq!(action.reason_codes, vec![GATE_GROUP_COUNT_NONZERO]);
}

#[test]
fn unknown_group_count_disallows_whole_group_claims() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    // 计数观测缺失。
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &[],
        Freshness::Current,
        &context,
        &reference,
        &inventory(true),
    );
    assert_eq!(action.kind, GroupActionKind::GeneralReference);
    assert_eq!(action.reason_codes, vec![GATE_GROUP_COUNT_UNKNOWN]);
    assert_eq!(
        action.evidence_refs,
        vec![format!("ev:refdiff:{REFERENCE_SET_ID}")]
    );
}

#[test]
fn over_target_counter_is_conflict_not_zero() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &san_juan_counters(None, 99),
        Freshness::Current,
        &context,
        &reference,
        &inventory(true),
    );
    assert_eq!(action.kind, GroupActionKind::GeneralReference);
    assert_eq!(action.reason_codes, vec![GATE_GROUP_COUNT_CONFLICT]);
}

#[test]
fn incomplete_table_closes_action_summary() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &san_juan_counters(Some(0), 0),
        Freshness::Current,
        &context,
        &reference,
        &inventory(false),
    );
    assert_eq!(action.kind, GroupActionKind::Unavailable);
    assert_eq!(action.reason_codes, vec![GATE_INVENTORY_INCOMPLETE]);

    // 表不完整且计数未知：两个原因都保留（与夹具 report.degraded.json 一致）。
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &[],
        Freshness::Current,
        &context,
        &reference,
        &inventory(false),
    );
    assert_eq!(action.kind, GroupActionKind::Unavailable);
    assert_eq!(
        action.reason_codes,
        vec![GATE_INVENTORY_INCOMPLETE, GATE_GROUP_COUNT_UNKNOWN]
    );
}

#[test]
fn stale_snapshot_closes_action_even_with_zero_counts() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    for freshness in [Freshness::Stale, Freshness::Refreshing, Freshness::None] {
        let action = evaluate(
            &bundle,
            SAN_JUAN_CACHES,
            &san_juan_counters(Some(0), 0),
            freshness,
            &context,
            &reference,
            &inventory(true),
        );
        assert_eq!(action.kind, GroupActionKind::Unavailable, "{freshness:?}");
        assert_eq!(action.reason_codes, vec![GATE_SNAPSHOT_NOT_CURRENT]);
    }
}

#[test]
fn non_campaign_context_closes_action() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    let unknown = assess(&DeclaredContext::default());
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &san_juan_counters(Some(0), 0),
        Freshness::Current,
        &unknown,
        &reference,
        &inventory(true),
    );
    assert_eq!(action.kind, GroupActionKind::Unavailable);
    assert_eq!(action.reason_codes, vec![GATE_CONTEXT_NOT_CAMPAIGN]);
}

#[test]
fn inapplicable_reference_degrades_to_general_reference() {
    let bundle = real_bundle();
    let context = campaign_context(Some("ps4"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    assert!(!reference.applicable);
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &san_juan_counters(Some(0), 0),
        Freshness::Current,
        &context,
        &reference,
        &inventory(true),
    );
    assert_eq!(action.kind, GroupActionKind::GeneralReference);
    assert_eq!(action.reason_codes, vec![GATE_REFERENCE_NOT_APPLICABLE]);
}

#[test]
fn group_association_mismatch_degrades() {
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    let mut group = bundle.guide_group(SAN_JUAN_CACHES).unwrap().clone();
    group.target_count = Some(9);
    let specs = bundle.region_counter_specs();
    let action = evaluate_group_gate(&GroupGateInputs {
        group: &group,
        counter_specs: &specs,
        counters: &san_juan_counters(Some(0), 0),
        context: &context,
        inventory: &inventory(true),
        reference: &reference,
        freshness: Freshness::Current,
        input_digest: &digest(),
    });
    assert_eq!(action.kind, GroupActionKind::GeneralReference);
    assert_eq!(
        action.reason_codes,
        vec![GATE_GROUP_ASSOCIATION_UNCONFIRMED]
    );
}

#[test]
fn group_keeps_one_group_level_hint_for_eight_ids() {
    let bundle = real_bundle();
    let group = bundle.guide_group(SAN_JUAN_CACHES).unwrap();
    assert_eq!(group.member_ids.len(), 8);
    assert_eq!(group.target_count, Some(8));
    // 同组 8 个 ID 保留原始身份，但只有一个组级线索，不复制为 8 个条目映射。
    let mut seen_group_ids = BTreeSet::new();
    for id in &group.member_ids {
        let item = bundle.item(*id).expect("组成员有身份观测");
        let membership: Vec<&str> = item
            .location_mappings
            .iter()
            .filter_map(|mapping| mapping.guide_group_id.as_deref())
            .collect();
        assert_eq!(membership, vec![SAN_JUAN_CACHES], "{id}");
        assert!(
            item.location_mappings
                .iter()
                .all(|mapping| mapping.guide_item.is_none())
        );
        seen_group_ids.extend(membership.iter().map(|value| value.to_string()));
    }
    assert_eq!(seen_group_ids.len(), 1);
}

#[test]
fn gate_output_serializes_into_contract_shape() {
    // 复用“圣胡安 0/8”输入形态，确认核心输出可直接作为契约 group_actionability 消费。
    let bundle = real_bundle();
    let context = campaign_context(Some("windows_steam"));
    let reference = bundle
        .reference_view(REFERENCE_SET_ID, &context.context)
        .unwrap();
    let action = evaluate(
        &bundle,
        SAN_JUAN_CACHES,
        &san_juan_counters(Some(0), 0),
        Freshness::Current,
        &context,
        &reference,
        &inventory(true),
    );
    let json = serde_json::to_value(&action).unwrap();
    assert_eq!(json["kind"], "whole_group_check");
    assert_eq!(json["reason_codes"][0], "COMPLETE_TABLE");
    assert_eq!(
        json["evaluated_input_digest"]["compressed_sha256"],
        "a".repeat(64)
    );
}
