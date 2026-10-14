//! SX-009 分析管线测试：字节 → 契约报告（合成夹具 + 可选私有样本）。
//!
//! 覆盖：完整战役报告、上下文关闭、资料文件、输入级错误、尾随字节、
//! 计数不一致降级、空元数据无兜底，以及私有样本的产品级期望（环境变量可选）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sottr_core::analyze::{PARSER_PROFILE_ID, analyze_bytes};
use sottr_core::domain::context::DeclaredContext;
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::report::{
    AssessmentKind, ContextSource, GroupActionKind, InternalId, Mode, ObservationKind,
    SupportStatus,
};
use sottr_core::synthetic::{SampleBuilder, compress};

fn metadata_bundle() -> MetadataBundle {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/metadata");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json") && name != "manifest.json")
        .collect();
    names.sort();
    let contents: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|name| (name.clone(), std::fs::read(dir.join(name)).unwrap()))
        .collect();
    let inputs: Vec<MetadataFileInput<'_>> = contents
        .iter()
        .map(|(file_id, bytes)| MetadataFileInput {
            file_id: file_id.clone(),
            bytes,
        })
        .collect();
    MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap()
}

fn campaign() -> DeclaredContext {
    DeclaredContext {
        platform: Some("windows_steam".to_string()),
        mode: Mode::Campaign,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    }
}

/// 构造 4 MiB 战役样本：整体进度 + 指定前缀 ID + 指定计数读数。
fn sample(prefix_ids: &[u32], counter_values: &[(&str, u32)]) -> Vec<u8> {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    builder.write_f32_le(0x28, 66.204_82_f32);
    for (index, id) in prefix_ids.iter().enumerate() {
        builder.write_prefix_slot(index, *id, 1);
    }
    let mut raw = builder.finish();
    if !counter_values.is_empty() {
        let bundle = metadata_bundle();
        let specs = bundle.region_counter_specs();
        for (counter_key, value) in counter_values {
            let spec = specs
                .iter()
                .find(|spec| &spec.counter_key == counter_key)
                .unwrap_or_else(|| panic!("缺少计数规格 {counter_key}"));
            let offset = spec.serialized_offset as usize;
            raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    compress(&raw)
}

const SAN_JUAN_CACHES: &str = "counter:mission_of_san_juan:survival_caches";
const SAN_JUAN_MONOLITHS: &str = "counter:mission_of_san_juan:monoliths";
const SAN_JUAN_CRYPTS: &str = "counter:mission_of_san_juan:crypts";

#[test]
fn campaign_pipeline_produces_full_report_with_whole_group_check() {
    let bundle = metadata_bundle();
    let bytes = sample(&[], &[]);
    let report = analyze_bytes(&bytes, &campaign(), &bundle, &Limits::default()).unwrap();

    assert_eq!(report.parser_profile_id, PARSER_PROFILE_ID);
    assert_eq!(report.tool_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(report.metadata_version.as_deref(), Some("0.1.0"));
    assert_eq!(report.reference_set_version.as_deref(), Some("1.0.0"));
    assert_eq!(report.input_digest.compressed_sha256.len(), 64);
    assert_eq!(report.input_digest.expanded_bytes, Some(4_194_304));

    for capability in [
        &report.capabilities.container,
        &report.capabilities.sections,
        &report.capabilities.inventory,
        &report.capabilities.counters,
        &report.capabilities.reference_difference,
    ] {
        assert_eq!(
            capability.status,
            SupportStatus::Supported,
            "{capability:?}"
        );
    }
    assert_eq!(report.capabilities.location.status, SupportStatus::Partial);
    assert_eq!(
        report.capabilities.location.reason_codes,
        vec!["LOCATION_UNVERIFIED"]
    );

    assert_eq!(
        report.observations.overall_completion.value,
        Some(f64::from(66.204_82_f32))
    );
    assert_eq!(report.observations.region_counters.len(), 43);

    // 全部参考 ID 缺席：99 项推定遗漏；参考集合内不存在观察项。
    let likely_missing: Vec<&InternalId> = report
        .assessments
        .iter()
        .filter(|item| item.assessment == AssessmentKind::LikelyMissing)
        .map(|item| &item.internal_id)
        .collect();
    assert_eq!(likely_missing.len(), 99);
    assert!(
        report
            .assessments
            .iter()
            .all(|item| { item.assessment != AssessmentKind::LikelyCollected })
    );

    // 零计数的三个圣胡安组给出整组检查；定位不产生坐标。
    let expected_groups = [
        "stella:san-juan-caches",
        "stella:san-juan-monolith",
        "stella:san-juan-crypt",
    ];
    for group_id in expected_groups {
        let location = report
            .locations
            .iter()
            .find(|location| location.guide_group_id.as_deref() == Some(group_id))
            .unwrap_or_else(|| panic!("缺少定位 {group_id}"));
        assert_eq!(
            location.group_actionability.as_ref().unwrap().kind,
            GroupActionKind::WholeGroupCheck,
            "{group_id}"
        );
    }
    assert!(
        report
            .locations
            .iter()
            .all(|location| { location.coordinates.is_none() && location.guide_item.is_none() })
    );
    // 2 个帕伊提提 ID 只有区域线索（一条区域提示）。
    let region_hint = report
        .locations
        .iter()
        .find(|location| location.location_id == "loc:region:paititi")
        .unwrap();
    assert_eq!(region_hint.member_ids.len(), 2);

    // 覆盖范围：参考类别与 43 项计数键；定位已按线索级别标注。
    assert_eq!(report.coverage.supported_category_ids.len(), 4);
    assert_eq!(report.coverage.reference_category_ids.len(), 4);
    assert_eq!(report.coverage.supported_region_counter_keys.len(), 43);
    assert!(report.coverage.unavailable_reason_codes.is_empty());

    // 证据：参考与计数来源已接线；评估引用可解析。
    let refdiff = report
        .evidence
        .iter()
        .find(|evidence| evidence.evidence_id.starts_with("ev:refdiff:"))
        .unwrap();
    assert_eq!(
        refdiff.source_refs,
        vec![
            "src:stella-saves".to_string(),
            "src:research-evidence".to_string()
        ]
    );
    let counter = report
        .evidence
        .iter()
        .find(|evidence| evidence.evidence_id == format!("ev:{SAN_JUAN_CACHES}"))
        .unwrap();
    assert_eq!(counter.source_refs, vec!["src:asl-sottr".to_string()]);
    assert!(sottr_core::domain::references_resolve(
        &report.assessments,
        &report.evidence
    ));

    // 诊断：声明限制与定位未验证可查。
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "MODE_USER_DECLARED_CAMPAIGN" })
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "LOCATION_UNVERIFIED"
            && diagnostic
                .evidence_refs
                .iter()
                .any(|r| r.starts_with("ev:refdiff:"))
    }));
}

#[test]
fn present_item_and_counter_produce_likely_collected_states() {
    let bundle = metadata_bundle();
    let bytes = sample(
        &[0x0F28_B6EC, 0x439B_19A6, 0xBA3B_0D06],
        &[
            (SAN_JUAN_CACHES, 1),
            (SAN_JUAN_MONOLITHS, 1),
            (SAN_JUAN_CRYPTS, 1),
        ],
    );
    let report = analyze_bytes(&bytes, &campaign(), &bundle, &Limits::default()).unwrap();
    assert_eq!(
        report.capabilities.reference_difference.status,
        SupportStatus::Supported
    );

    let find = |id: u32| {
        report
            .assessments
            .iter()
            .find(|item| item.internal_id == InternalId::new(id))
            .unwrap()
    };
    assert_eq!(
        find(0x0F28_B6EC).assessment,
        AssessmentKind::LikelyCollected
    );
    assert_eq!(
        find(0x0F28_B6ED).assessment,
        AssessmentKind::LikelyMissing,
        "同组缺席 ID 仍为推定遗漏"
    );

    // 非零计数：撤销整组检查提示。
    let caches = report
        .locations
        .iter()
        .find(|location| location.guide_group_id.as_deref() == Some("stella:san-juan-caches"))
        .unwrap();
    let actionability = caches.group_actionability.as_ref().unwrap();
    assert_eq!(actionability.kind, GroupActionKind::GeneralReference);
    assert!(
        actionability
            .reason_codes
            .iter()
            .any(|code| code == "GROUP_COUNT_NONZERO")
    );
}

#[test]
fn counter_mismatch_blocks_likely_states() {
    let bundle = metadata_bundle();
    // 前缀含 1 个圣胡安贮藏点 ID，但计数读数保持 0：交叉检查必须降级。
    let bytes = sample(&[0x0F28_B6EC], &[]);
    let report = analyze_bytes(&bytes, &campaign(), &bundle, &Limits::default()).unwrap();
    assert_eq!(
        report.capabilities.reference_difference.status,
        SupportStatus::Partial
    );
    assert_eq!(
        report.capabilities.reference_difference.reason_codes,
        vec!["COUNTER_MISMATCH"]
    );
    assert!(
        report
            .assessments
            .iter()
            .all(|item| item.assessment != AssessmentKind::LikelyMissing),
        "计数不一致时不给 likely_* 结论"
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "COUNTER_MISMATCH"
            && diagnostic.scope == sottr_core::report::DiagnosticScope::ReferenceDifference
    }));
}

#[test]
fn unknown_context_closes_collection_semantics_but_keeps_identity() {
    let bundle = metadata_bundle();
    let bytes = sample(&[0x0F28_B6EC], &[]);
    let declared = DeclaredContext {
        platform: None,
        mode: Mode::Unknown,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    };
    let report = analyze_bytes(&bytes, &declared, &bundle, &Limits::default()).unwrap();

    assert_eq!(report.context.mode, Mode::Unknown);
    let overall = &report.observations.overall_completion;
    assert_eq!(overall.value, None);
    assert_eq!(overall.support_status, SupportStatus::Unsupported);
    assert!(overall.reason.is_some(), "null 值必须附原因");
    assert!(report.observations.region_counters.is_empty());
    assert!(report.locations.is_empty());
    assert_eq!(
        report.capabilities.inventory.status,
        SupportStatus::Unsupported
    );
    assert_eq!(
        report.capabilities.inventory.reason_codes,
        vec!["UNSUPPORTED_CONTEXT"]
    );
    assert_eq!(
        report.capabilities.reference_difference.status,
        SupportStatus::Unsupported
    );
    assert_eq!(
        report.capabilities.location.reason_codes,
        vec!["UNSUPPORTED_CONTEXT"]
    );

    // 原始身份观测仍保留：present + unknown，不产生 likely_*。
    let item = report
        .assessments
        .iter()
        .find(|item| item.internal_id == InternalId::new(0x0F28_B6EC))
        .unwrap();
    assert_eq!(item.observation, ObservationKind::Present);
    assert_eq!(item.assessment, AssessmentKind::Unknown);
    assert!(item.rule_id.is_none());
}

#[test]
fn profile_input_reports_container_only() {
    let bundle = metadata_bundle();
    let bytes = compress(&vec![0u8; 25_600]);
    let declared = DeclaredContext {
        mode: Mode::Profile,
        source: ContextSource::UserDeclared,
        ..DeclaredContext::default()
    };
    let report = analyze_bytes(&bytes, &declared, &bundle, &Limits::default()).unwrap();
    assert!(report.observations.container.valid);
    assert_eq!(report.observations.container.expanded_bytes, Some(25_600));
    assert_eq!(
        report.capabilities.sections.status,
        SupportStatus::Unsupported
    );
    assert_eq!(
        report.capabilities.sections.reason_codes,
        vec!["UNSUPPORTED_CONTEXT"]
    );
    assert!(report.observations.inventory.records.is_empty());
    assert!(report.locations.is_empty());
    assert_eq!(report.context.mode, Mode::Profile);
}

#[test]
fn bad_containers_and_oversize_are_input_errors() {
    let bundle = metadata_bundle();
    let context = campaign();

    let garbage = analyze_bytes(b"not a save", &context, &bundle, &Limits::default()).unwrap_err();
    assert_eq!(garbage.code, "DECOMPRESSION_FAILED");
    assert!(!garbage.retryable);

    let truncated = compress(&vec![7u8; 4096]);
    let error = analyze_bytes(
        &truncated[..truncated.len() - 3],
        &context,
        &bundle,
        &Limits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "DECOMPRESSION_FAILED");

    let bytes = sample(&[], &[]);
    let tiny = Limits::new(16, 1024, 100).unwrap();
    let error = analyze_bytes(&bytes, &context, &bundle, &tiny).unwrap_err();
    assert_eq!(error.code, "RESOURCE_LIMIT");
    assert!(!error.retryable);
}

#[test]
fn trailing_bytes_are_diagnosed_not_interpreted() {
    let bundle = metadata_bundle();
    let mut bytes = sample(&[0x0F28_B6EC], &[(SAN_JUAN_CACHES, 1)]);
    bytes.extend_from_slice(b"stale tail");
    let report = analyze_bytes(&bytes, &campaign(), &bundle, &Limits::default()).unwrap();
    assert!(report.observations.container.trailing_bytes > 0);
    assert_eq!(report.observations.inventory.unique_id_count, 1);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CONTAINER_TRAILING_BYTES")
    );
}

#[test]
fn missing_metadata_degrades_without_hardcoded_fallback() {
    let bytes = sample(&[0x0F28_B6EC], &[]);
    let report = analyze_bytes(
        &bytes,
        &campaign(),
        &MetadataBundle::empty(),
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(report.metadata_version, None);
    assert_eq!(report.reference_set_version, None);
    assert_eq!(
        report.capabilities.counters.status,
        SupportStatus::Unsupported
    );
    assert_eq!(
        report.capabilities.counters.reason_codes,
        vec!["COUNTER_MAPPING_UNAVAILABLE"]
    );
    assert!(report.observations.region_counters.is_empty(), "无兜底计数");
    assert!(report.locations.is_empty(), "无兜底定位");
    assert!(
        report
            .assessments
            .iter()
            .all(|item| item.assessment == AssessmentKind::Unknown)
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "METADATA_UNAVAILABLE"
            && diagnostic.scope == sottr_core::report::DiagnosticScope::Metadata
    }));
}

/// 私有样本的产品级期望（`SOTTR_SAMPLE_DIR` 可选；未设置时安静跳过）。
#[test]
fn private_samples_full_pipeline_matches_product_expectations() {
    let Some(dir) = std::env::var_os("SOTTR_SAMPLE_DIR").map(PathBuf::from) else {
        eprintln!("SOTTR_SAMPLE_DIR 未设置：跳过私有样本管线验证（记为 skip，不经过）");
        return;
    };
    let bundle = metadata_bundle();

    for (name, unique_ids, missing_count, extra_missing) in [
        ("save1.dat", 517u64, 12usize, None),
        ("save204.dat", 516, 13, Some(0xDD26_A1BCu32)),
    ] {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        let report = analyze_bytes(&bytes, &campaign(), &bundle, &Limits::default()).unwrap();
        assert_eq!(
            report.observations.inventory.unique_id_count, unique_ids,
            "{name}"
        );
        assert_eq!(
            report.capabilities.reference_difference.status,
            SupportStatus::Supported,
            "{name}"
        );
        let likely_missing: BTreeSet<u32> = report
            .assessments
            .iter()
            .filter(|item| item.assessment == AssessmentKind::LikelyMissing)
            .map(|item| item.internal_id.raw())
            .collect();
        assert_eq!(likely_missing.len(), missing_count, "{name}");
        for expected in [
            0x439B_19A6,
            0xD8CD_8257,
            0x0F28_B6EC,
            0x0F28_B6ED,
            0x0F28_B6EE,
            0x0F28_B6EF,
            0x851B_4915,
            0xAE1A_2E7A,
            0xB419_E6FE,
            0xFF9E_AC6E,
            0xFF9E_AC79,
            0xBA3B_0D06,
        ] {
            assert!(
                likely_missing.contains(&expected),
                "{name} 缺少 {expected:#X}"
            );
        }
        if let Some(extra) = extra_missing {
            assert!(likely_missing.contains(&extra), "{name} 缺少 {extra:#X}");
        }
        // 圣胡安 0/8：三组均可按攻略整组检查。
        for group_id in [
            "stella:san-juan-caches",
            "stella:san-juan-monolith",
            "stella:san-juan-crypt",
        ] {
            let location = report
                .locations
                .iter()
                .find(|location| location.guide_group_id.as_deref() == Some(group_id))
                .unwrap();
            assert_eq!(
                location.group_actionability.as_ref().unwrap().kind,
                GroupActionKind::WholeGroupCheck,
                "{name} {group_id}"
            );
        }
        assert_eq!(report.coverage.supported_region_counter_keys.len(), 43);
        assert!(
            report
                .locations
                .iter()
                .all(|location| location.coordinates.is_none()),
            "{name}: 不得产生坐标"
        );
    }
}
