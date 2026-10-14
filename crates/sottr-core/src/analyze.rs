//! 同核心分析管线（SX-009）：不可变字节 → 契约 [`Report`]。
//!
//! 边界与约束：
//! - 与 Python v0.2 参照的对照口径一致：容器/区段/记录/43 项计数/整体候选/
//!   参考差集全部来自同一份字节，不做全文件两两比较；
//! - 容器失败或资源超限返回输入级错误（`DECOMPRESSION_FAILED` / `RESOURCE_LIMIT`），
//!   不产生报告、不做语义解释；局部失败留在报告诊断与能力矩阵；
//! - 上下文不适用（模式未知/资料文件）时关闭收集语义：整体进度与区域计数
//!   以 `unsupported` + 原因表达，原始身份观测仍保留；
//! - 参考差集使用元数据提供的唯一参考集合；计数交叉检查按 `counts_toward`
//!   作用域进行（贮藏点计数覆盖普通与稀有类别）；
//! - 定位只输出元数据中的区域/攻略组线索，`group_actionability` 由领域门禁
//!   按当前快照评估；无精确坐标时不产生假点；
//! - 本模块不接收路径、不访问文件系统：磁盘读取、快照与目录发现由适配层承担。

use std::collections::{BTreeMap, BTreeSet};

use crate::container::{Decompression, DecompressionError, decompress};
use crate::domain::assessment::{
    AssessmentInputs, AssessmentOutcome, CounterScopeCheck, CrossCheckInputs, ReferenceView,
    assess as assess_reference,
};
use crate::domain::context::{ContextAssessment, DeclaredContext, assess as assess_context};
use crate::domain::evidence::{EvidenceSources, build_evidence};
use crate::domain::location::{GroupGateInputs, evaluate_group_gate};
use crate::domain::progress::{
    OverallReadout, RegionCounterReadout, RegionCounterSpec, overall_completion, region_counters,
};
use crate::domain::{CapabilityInputs, capability_matrix};
use crate::hash::sha256_hex;
use crate::inventory::{InventoryReason, InventoryRecovery, recover};
use crate::limits::Limits;
use crate::metadata::MetadataBundle;
use crate::report::{
    Capability, ContainerObservation, Coverage, Diagnostic, DiagnosticScope, EvidenceLevel,
    Freshness, InputDigest, InputError, InternalId, LocationHint, LocationPrecision, MappingKind,
    Observations, ProgressValue, Report, SchemaVersion, SectionsObservation, Severity,
    SupportStatus,
};
use crate::sections::{SectionError, SectionStream};

/// 解析器 profile 标识（契约字段；与 SX-003 夹具一致）。
pub const PARSER_PROFILE_ID: &str = "sottr-save-family-4mib-v1";

/// 工具版本（构建时取自 crate 版本）。
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 分析一份不可变输入字节。
///
/// 返回契约报告；容器无效或超出资源限制时返回输入级错误（不产生报告）。
pub fn analyze_bytes(
    bytes: &[u8],
    declared: &DeclaredContext,
    metadata: &MetadataBundle,
    limits: &Limits,
) -> Result<Report, InputError> {
    let decompression = decompress(bytes, limits);
    if !decompression.valid {
        return Err(input_error_for_decompression(&decompression, limits));
    }
    Ok(assemble(bytes, &decompression, declared, metadata, limits))
}

fn input_error_for_decompression(decompression: &Decompression, limits: &Limits) -> InputError {
    let error = decompression
        .error
        .as_ref()
        .expect("invalid decompression always carries an error");
    match error {
        DecompressionError::CompressedLimitExceeded => InputError::resource_limit(format!(
            "压缩输入 {} 字节超过上限 {}（compressed_size_limit_exceeded）",
            decompression.compressed_bytes, limits.max_compressed_bytes
        )),
        DecompressionError::ExpandedLimitExceeded => InputError::resource_limit(format!(
            "解压结果超过上限 {} 字节（expanded_size_limit_exceeded）",
            limits.max_expanded_bytes
        )),
        DecompressionError::InvalidStream { .. } | DecompressionError::Truncated => {
            InputError::decompression_failed(format!(
                "zlib 流无效或截断（{}）：不做语义解释",
                error.reason_code()
            ))
        }
    }
}

fn assemble(
    bytes: &[u8],
    decompression: &Decompression,
    declared: &DeclaredContext,
    metadata: &MetadataBundle,
    limits: &Limits,
) -> Report {
    let data: &[u8] = &decompression.data;
    let context = assess_context(declared);
    let sections = SectionStream::walk(data, limits);
    let inventory = recover(data, &sections);
    let specs = metadata.region_counter_specs();

    let mut overall = overall_completion(data, &sections);
    let mut counters = region_counters(data, &sections, &specs);
    if !context.campaign_applicable {
        // 收集语义关闭：字段级 support_status 必须反映“不适用”，
        // 不能以社区候选值冒充当前展示值；原始身份观测仍保留。
        overall = closed_overall(overall);
        counters.counters = Vec::new();
    }

    let input_digest = InputDigest {
        compressed_sha256: sha256_hex(bytes),
        expanded_sha256: Some(sha256_hex(data)),
        compressed_bytes: decompression.compressed_bytes,
        expanded_bytes: Some(decompression.expanded_bytes),
    };

    let reference_view = metadata
        .primary_reference_set_id()
        .and_then(|reference_set_id| metadata.reference_view(reference_set_id, &context.context));
    let unavailable_reference = unavailable_reference_view();
    let assessment_reference = reference_view.as_ref().unwrap_or(&unavailable_reference);

    let unique_ids: Vec<InternalId> = {
        let set: BTreeSet<InternalId> = inventory
            .observation
            .records
            .iter()
            .map(|record| record.internal_id)
            .collect();
        set.into_iter().collect()
    };
    let observed_categories = metadata.classify_ids(&unique_ids);
    let cross_checks = build_cross_checks(metadata, &counters, assessment_reference);
    let assessment = assess_reference(&AssessmentInputs {
        context: &context,
        inventory: &inventory,
        reference: assessment_reference,
        observed_category_ids: &observed_categories,
        cross_checks: &cross_checks,
    });

    let evidence_sources = build_evidence_sources(metadata);
    let evidence = build_evidence(
        Some(&overall),
        &sections,
        &inventory,
        Some((&counters, &specs)),
        reference_view.as_ref(),
        &evidence_sources,
    );

    let locations = build_locations(
        metadata,
        &context,
        &inventory.observation,
        &counters,
        &specs,
        &input_digest,
        reference_view.as_ref(),
    );

    let location_capability = location_capability(&context, &locations);
    let capabilities = capability_matrix(&CapabilityInputs {
        decompression,
        sections: &sections,
        campaign_applicable: context.campaign_applicable,
        inventory: Some(&inventory),
        region_counters: Some(&counters),
        reference_difference: Some(&assessment),
        location: Some(location_capability),
    });

    let coverage = build_coverage(
        &capabilities,
        reference_view.as_ref(),
        &assessment,
        &counters,
    );

    let diagnostics = build_diagnostics(
        decompression,
        &context,
        &sections,
        &inventory,
        &counters,
        &assessment,
        metadata,
        reference_view.as_ref(),
        !locations.is_empty(),
    );

    Report {
        schema_version: SchemaVersion,
        tool_version: TOOL_VERSION.to_string(),
        parser_profile_id: PARSER_PROFILE_ID.to_string(),
        metadata_version: metadata.metadata_version().map(str::to_string),
        reference_set_version: reference_view.as_ref().map(|view| view.version.clone()),
        input_digest,
        context: context.context.clone(),
        capabilities,
        coverage,
        observations: Observations {
            container: ContainerObservation {
                status: SupportStatus::Supported,
                format: "zlib".to_string(),
                valid: true,
                eof: decompression.eof,
                compressed_bytes: decompression.compressed_bytes,
                expanded_bytes: Some(decompression.expanded_bytes),
                trailing_bytes: decompression.trailing_bytes,
                error: None,
            },
            sections: SectionsObservation {
                status: if sections.valid {
                    SupportStatus::Supported
                } else {
                    SupportStatus::Unsupported
                },
                valid: sections.valid,
                byte_length: u64::from(sections.byte_length),
                byte_length_hex: format!("0x{:X}", sections.byte_length),
                start_offset: sections.start_offset,
                start_offset_hex: format!("0x{:X}", sections.start_offset),
                section_count: sections.section_count() as u64,
                error: sections.error.map(|error| error.message().to_string()),
            },
            overall_completion: overall.progress,
            inventory: inventory.observation,
            region_counters: counters.counters,
        },
        assessments: assessment.items,
        locations,
        evidence,
        diagnostics,
    }
}

fn closed_overall(readout: OverallReadout) -> OverallReadout {
    OverallReadout {
        progress: ProgressValue {
            value: None,
            unit: "percent".to_string(),
            support_status: SupportStatus::Unsupported,
            reason: Some("普通战役上下文未确认：整体进度候选已关闭".to_string()),
            evidence_level: EvidenceLevel::Observed,
            evidence_refs: Vec::new(),
            limitations: vec!["模式未知或不适用时不展示收集语义候选".to_string()],
        },
        raw_bytes_hex: readout.raw_bytes_hex,
        raw_value: readout.raw_value,
    }
}

fn unavailable_reference_view() -> ReferenceView {
    ReferenceView {
        reference_set_id: "rs:unavailable".to_string(),
        version: String::new(),
        applicable: false,
        applicability_limits: vec![
            "未提供可用于本报告的参考集合：参考差集与 likely_* 不可用".to_string(),
        ],
        categories: Vec::new(),
    }
}

fn build_cross_checks(
    metadata: &MetadataBundle,
    counters: &RegionCounterReadout,
    reference: &ReferenceView,
) -> CrossCheckInputs {
    let mut inputs = CrossCheckInputs {
        counter_totals_degraded: !counters.spec_errors.is_empty(),
        ..CrossCheckInputs::default()
    };
    let view_categories: BTreeSet<&str> = reference
        .categories
        .iter()
        .map(|category| category.category_id.as_str())
        .collect();
    for scope in metadata.counter_scope_totals(&counters.counters) {
        if scope.category_ids.is_empty() {
            continue;
        }
        if !scope
            .category_ids
            .iter()
            .all(|category| view_categories.contains(category.as_str()))
        {
            // 作用域类别不在当前参考集合内：不做交叉检查。
            continue;
        }
        if scope.category_ids.len() == 1 {
            match scope.observed_total {
                Some(value) => {
                    inputs
                        .category_counter_totals
                        .insert(scope.category_ids[0].clone(), value as u32);
                }
                None => inputs.counter_totals_degraded = true,
            }
        } else {
            inputs.counter_scope_checks.push(CounterScopeCheck {
                counter_total: scope.observed_total.unwrap_or(0) as u32,
                degraded: scope.degraded
                    || scope.observed_total.is_none()
                    || !scope.missing_counter_keys.is_empty(),
                category_ids: scope.category_ids.clone(),
            });
        }
    }
    inputs
}

fn build_evidence_sources(metadata: &MetadataBundle) -> EvidenceSources {
    let counter_sources = metadata
        .dataset()
        .map(|dataset| {
            dataset
                .region_counters
                .iter()
                .map(|(counter_key, entry)| (counter_key.clone(), entry.source_refs.clone()))
                .collect()
        })
        .unwrap_or_default();
    let reference_sources = metadata
        .primary_reference_set_id()
        .and_then(|reference_set_id| metadata.reference_set(reference_set_id))
        .map(|reference_set| reference_set.source_refs.clone())
        .unwrap_or_default();
    EvidenceSources {
        counter_sources,
        reference_sources,
    }
}

fn build_locations(
    metadata: &MetadataBundle,
    context: &ContextAssessment,
    inventory: &crate::report::InventoryObservation,
    counters: &RegionCounterReadout,
    specs: &[RegionCounterSpec],
    input_digest: &InputDigest,
    primary_reference: Option<&ReferenceView>,
) -> Vec<LocationHint> {
    if !context.campaign_applicable {
        return Vec::new();
    }
    let Some(dataset) = metadata.dataset() else {
        return Vec::new();
    };
    let fallback = unavailable_reference_view();
    let mut hints = Vec::new();

    for group in dataset.guide_groups.values() {
        let group_view = group
            .reference_set_id
            .as_deref()
            .and_then(|reference_set_id| {
                metadata.reference_view(reference_set_id, &context.context)
            });
        let group_reference = group_view
            .as_ref()
            .or(primary_reference)
            .unwrap_or(&fallback);
        let (applicable_mode, applicable_platform) = group
            .reference_set_id
            .as_deref()
            .and_then(|reference_set_id| metadata.reference_set(reference_set_id))
            .map(|reference_set| {
                (
                    reference_set.applicability.mode,
                    reference_set.applicability.platform.clone(),
                )
            })
            .unwrap_or((None, None));
        let actionability = evaluate_group_gate(&GroupGateInputs {
            group,
            counter_specs: specs,
            counters: &counters.counters,
            context,
            inventory,
            reference: group_reference,
            freshness: Freshness::Current,
            input_digest,
        });
        let reference_set_version = group
            .reference_set_id
            .as_deref()
            .and_then(|reference_set_id| metadata.reference_set(reference_set_id))
            .map(|reference_set| reference_set.version.clone())
            .or_else(|| primary_reference.map(|view| view.version.clone()));
        hints.push(LocationHint {
            location_id: format!("loc:{}", group.guide_group_id),
            precision: LocationPrecision::GuideGroup,
            region_id: group.region_id.clone(),
            guide_group_id: Some(group.guide_group_id.clone()),
            guide_item: None,
            member_ids: group.member_ids.clone(),
            target_count: group.target_count,
            mapping_kind: group.mapping_kind,
            source_refs: group.source_refs.clone(),
            reference_set_version,
            applicable_mode,
            applicable_platform,
            coordinates: None,
            map_transform: None,
            group_actionability: Some(actionability),
            limitations: vec!["未建立内部 ID 与攻略编号或坐标的一对一映射".to_string()],
            reason: None,
        });
    }

    // 区域线索：按区域合并为一条组级提示（不逐项复制为精确点位）。
    struct RegionClue {
        members: BTreeSet<InternalId>,
        source_refs: Vec<String>,
        applicable_mode: Option<crate::report::Mode>,
        applicable_platform: Option<String>,
    }
    let mut clues: BTreeMap<String, RegionClue> = BTreeMap::new();
    for item in dataset.items.values() {
        for mapping in &item.location_mappings {
            if mapping.mapping_kind != MappingKind::RegionClue {
                continue;
            }
            let Some(region_id) = mapping.region_id.clone() else {
                continue;
            };
            let clue = clues.entry(region_id).or_insert_with(|| RegionClue {
                members: BTreeSet::new(),
                source_refs: Vec::new(),
                applicable_mode: mapping.applicable_mode,
                applicable_platform: mapping.applicable_platform.clone(),
            });
            clue.members.insert(item.internal_id);
            for source_ref in &mapping.source_refs {
                if !clue.source_refs.contains(source_ref) {
                    clue.source_refs.push(source_ref.clone());
                }
            }
        }
    }
    for (region_id, clue) in clues {
        hints.push(LocationHint {
            location_id: format!("loc:region:{region_id}"),
            precision: LocationPrecision::Region,
            region_id: Some(region_id),
            guide_group_id: None,
            guide_item: None,
            member_ids: clue.members.into_iter().collect(),
            target_count: None,
            mapping_kind: MappingKind::RegionClue,
            source_refs: clue.source_refs,
            reference_set_version: None,
            applicable_mode: clue.applicable_mode,
            applicable_platform: clue.applicable_platform,
            coordinates: None,
            map_transform: None,
            group_actionability: None,
            limitations: vec!["仅有区域线索，无攻略组或精确条目对应".to_string()],
            reason: None,
        });
    }

    hints
}

fn location_capability(context: &ContextAssessment, locations: &[LocationHint]) -> Capability {
    if !context.campaign_applicable {
        return capability(SupportStatus::Unsupported, &["UNSUPPORTED_CONTEXT"]);
    }
    if locations.is_empty() {
        return capability(SupportStatus::Unsupported, &["LOCATION_UNVERIFIED"]);
    }
    capability(SupportStatus::Partial, &["LOCATION_UNVERIFIED"])
}

fn capability(status: SupportStatus, codes: &[&str]) -> Capability {
    Capability {
        status,
        reason_codes: codes.iter().map(|code| (*code).to_string()).collect(),
    }
}

fn build_coverage(
    capabilities: &crate::report::Capabilities,
    reference_view: Option<&ReferenceView>,
    assessment: &AssessmentOutcome,
    counters: &RegionCounterReadout,
) -> Coverage {
    let supported_category_ids: Vec<String> = reference_view
        .map(|view| {
            view.categories
                .iter()
                .map(|category| category.category_id.clone())
                .collect()
        })
        .unwrap_or_default();
    let reference_category_ids = if assessment.capability.status == SupportStatus::Unsupported {
        Vec::new()
    } else {
        supported_category_ids.clone()
    };
    let supported_region_counter_keys: Vec<String> = counters
        .counters
        .iter()
        .filter(|counter| counter.support_status != SupportStatus::Unsupported)
        .map(|counter| counter.counter_key.clone())
        .collect();
    let mut unavailable_reason_codes: Vec<String> = Vec::new();
    for capability in [
        &capabilities.container,
        &capabilities.sections,
        &capabilities.inventory,
        &capabilities.counters,
        &capabilities.reference_difference,
        &capabilities.location,
    ] {
        if capability.status != SupportStatus::Unsupported {
            continue;
        }
        for code in &capability.reason_codes {
            if !unavailable_reason_codes.contains(code) {
                unavailable_reason_codes.push(code.clone());
            }
        }
    }
    let evidence_refs = reference_view
        .map(|view| vec![format!("ev:refdiff:{}", view.reference_set_id)])
        .unwrap_or_default();
    Coverage {
        supported_category_ids,
        reference_category_ids,
        supported_region_counter_keys,
        unavailable_reason_codes,
        evidence_refs,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_diagnostics(
    decompression: &Decompression,
    context: &ContextAssessment,
    sections: &SectionStream,
    inventory: &InventoryRecovery,
    counters: &RegionCounterReadout,
    assessment: &AssessmentOutcome,
    metadata: &MetadataBundle,
    reference_view: Option<&ReferenceView>,
    has_locations: bool,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for finding in &context.findings {
        push_diagnostic(
            &mut diagnostics,
            finding.code(),
            Severity::Info,
            DiagnosticScope::Input,
            finding.limitation(),
            Vec::new(),
        );
    }

    if decompression.trailing_bytes > 0 {
        push_diagnostic(
            &mut diagnostics,
            "CONTAINER_TRAILING_BYTES",
            Severity::Info,
            DiagnosticScope::Container,
            format!(
                "首流结束后存在 {} 个尾随字节（不解释、不参与当前表）",
                decompression.trailing_bytes
            ),
            Vec::new(),
        );
    }

    if let Some(error) = sections.error {
        let code = match error {
            SectionError::UnrecognizedFamilySize => "UNSUPPORTED_CONTEXT",
            _ => "UNSUPPORTED_LAYOUT",
        };
        push_diagnostic(
            &mut diagnostics,
            code,
            Severity::Warning,
            DiagnosticScope::Sections,
            format!("主区段流校验失败（{}）", error.message()),
            Vec::new(),
        );
    }

    for reason in &inventory.reasons {
        let evidence_refs = match reason {
            InventoryReason::Type28LengthMismatch { offset, .. } => {
                vec![format!("ev:type28-mismatch:0x{offset:X}")]
            }
            InventoryReason::SectionStreamUnavailable { .. } => Vec::new(),
        };
        push_diagnostic(
            &mut diagnostics,
            reason.diagnostic_code(),
            Severity::Warning,
            DiagnosticScope::Inventory,
            reason.message(),
            evidence_refs,
        );
    }

    for error in &counters.spec_errors {
        push_diagnostic(
            &mut diagnostics,
            "COUNTER_MAPPING_INVALID",
            Severity::Warning,
            DiagnosticScope::Counters,
            format!(
                "计数映射 {} 无法读取（{}）",
                error.counter_key, error.reason
            ),
            Vec::new(),
        );
    }
    for counter in &counters.counters {
        if !counter.within_source_target {
            push_diagnostic(
                &mut diagnostics,
                "COUNTER_MISMATCH",
                Severity::Warning,
                DiagnosticScope::Counters,
                "区域计数原始读数超过来源目标：保留原始值并降级候选",
                counter.evidence_refs.clone(),
            );
        }
    }

    for reason in &assessment.reasons {
        push_diagnostic(
            &mut diagnostics,
            reason.diagnostic_code(),
            Severity::Warning,
            DiagnosticScope::ReferenceDifference,
            reason.message(),
            Vec::new(),
        );
    }

    if metadata.metadata_version().is_none() {
        push_diagnostic(
            &mut diagnostics,
            "METADATA_UNAVAILABLE",
            Severity::Warning,
            DiagnosticScope::Metadata,
            "未加载元数据：参考集合、计数映射与定位不可用",
            Vec::new(),
        );
    } else if metadata
        .dataset()
        .is_some_and(|dataset| dataset.reference_sets.len() > 1)
    {
        push_diagnostic(
            &mut diagnostics,
            "METADATA_CONFLICT",
            Severity::Warning,
            DiagnosticScope::Metadata,
            "存在多个参考集合，未配置选择规则：本次不产生参考差集",
            Vec::new(),
        );
    }

    if has_locations {
        let evidence_refs = reference_view
            .map(|view| vec![format!("ev:refdiff:{}", view.reference_set_id)])
            .unwrap_or_default();
        push_diagnostic(
            &mut diagnostics,
            "LOCATION_UNVERIFIED",
            Severity::Info,
            DiagnosticScope::Location,
            "已验证地图坐标 0 项；界面不得绘制精确点位",
            evidence_refs,
        );
    }

    diagnostics
}

fn push_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    code: &str,
    severity: Severity,
    scope: DiagnosticScope,
    message: impl Into<String>,
    evidence_refs: Vec<String>,
) {
    let message = message.into();
    if diagnostics.iter().any(|existing| {
        existing.code == code && existing.scope == scope && existing.message == message
    }) {
        return;
    }
    diagnostics.push(Diagnostic {
        code: code.to_string(),
        severity,
        scope,
        message,
        evidence_refs,
    });
}
