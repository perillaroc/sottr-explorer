//! 参考差集与证据评估（SX-007）。
//!
//! 约束（开发方案 SX-007）：
//! - 差集只使用元数据提供的共同参考集合（当前为 65 普通贮藏点、12 稀有资源贮藏点、
//!   12 石碑、10 墓穴宝藏；具体内容由 SX-008 元数据校验），不使用字典全集；
//! - 先产生 observation，再按类别、上下文、表完整性、参考适用性与交叉检查生成
//!   assessment，并记录 `rule_id`；
//! - `likely_*` 只在对应规则全部条件满足时出现；表不完整或流无效时连缺席差集也禁用；
//!   结构完整但参考适用性不足时最多输出原始参考差，不给 `likely_missing`；
//! - 服装、剧情与测试对象只保留身份观测；差集为空不产生“全游戏 100%”结论。

use std::collections::{BTreeMap, BTreeSet};

use crate::inventory::InventoryRecovery;
use crate::report::{AssessedItem, Capability};
use crate::report::{AssessmentKind, EvidenceLevel, InternalId, ObservationKind, SupportStatus};

use super::context::ContextAssessment;

/// 规则：参考集合内、完整表、上下文与参考适用时的 `likely_collected`。
pub const RULE_LIKELY_COLLECTED: &str = "refdiff.v1.likely_collected";

/// 规则：参考存在而当前表未观察到、且全部条件满足时的 `likely_missing`。
pub const RULE_LIKELY_MISSING: &str = "refdiff.v1.likely_missing";

/// 规则登记表条目。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleDescriptor {
    /// 稳定规则 ID。
    pub rule_id: &'static str,
    /// 规则说明。
    pub description: &'static str,
    /// 规则产出的证据等级。
    pub evidence_level: EvidenceLevel,
}

/// 已登记规则（评估记录的 `rule_id` 必须来自此表）。
pub const RULE_REGISTRY: &[RuleDescriptor] = &[
    RuleDescriptor {
        rule_id: RULE_LIKELY_COLLECTED,
        description: "ID 位于共同参考集合、当前表完整、上下文为普通战役且参考适用",
        evidence_level: EvidenceLevel::CommunityCorrelated,
    },
    RuleDescriptor {
        rule_id: RULE_LIKELY_MISSING,
        description: "ID 位于共同参考集合且当前表未观察到，表完整、上下文与交叉检查全部通过",
        evidence_level: EvidenceLevel::CommunityCorrelated,
    },
];

/// 按 ID 查询规则登记表。
pub fn rule_descriptor(rule_id: &str) -> Option<&'static RuleDescriptor> {
    RULE_REGISTRY
        .iter()
        .find(|descriptor| descriptor.rule_id == rule_id)
}

/// 参考集合的分类视图（由元数据层提供）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCategoryView {
    /// 类别 ID（如 `survival_caches`）。
    pub category_id: String,
    /// 共同参考 ID 集合。
    pub reference_ids: BTreeSet<InternalId>,
}

/// 参考集合视图（由元数据层提供；本模块不硬编码具体类别内容）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceView {
    /// 参考集合 ID。
    pub reference_set_id: String,
    /// 参考集合版本。
    pub version: String,
    /// 适用范围判定（版本/DLC/上下文匹配；由元数据层计算）。
    pub applicable: bool,
    /// 适用性限制说明。
    pub applicability_limits: Vec<String>,
    /// 类别集合。
    pub categories: Vec<ReferenceCategoryView>,
}

/// 交叉检查输入（由组装层提供）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrossCheckInputs {
    /// 分类别计数汇总（来自区域计数读数；缺失类别表示不做该项检查）。
    pub category_counter_totals: BTreeMap<String, u32>,
    /// 汇总中是否包含不可用计数（超目标/未知）；为真时按 `COUNTER_MISMATCH` 降级。
    pub counter_totals_degraded: bool,
    /// 元数据冲突说明（SX-008 提供；非空即降级）。
    pub metadata_conflicts: Vec<String>,
}

/// 评估输入。
pub struct AssessmentInputs<'a> {
    /// 上下文评估（SX-006）。
    pub context: &'a ContextAssessment,
    /// 记录恢复（SX-005）。
    pub inventory: &'a InventoryRecovery,
    /// 参考集合视图。
    pub reference: &'a ReferenceView,
    /// 当前表按元数据类别的分类（类别 ID → ID 集合）。
    pub observed_category_ids: &'a BTreeMap<String, BTreeSet<InternalId>>,
    /// 交叉检查输入。
    pub cross_checks: &'a CrossCheckInputs,
}

/// 单个类别的集合差（原始观察）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryDiff {
    /// 类别 ID。
    pub category_id: String,
    /// 参考 ID 总数。
    pub reference_count: usize,
    /// 参考与当前表都存在的 ID。
    pub present_reference_ids: Vec<InternalId>,
    /// 参考存在而当前表未观察到的 ID。
    pub absent_ids: Vec<InternalId>,
    /// 当前表分类为同类但不在参考集合中的 ID。
    pub outside_reference_ids: Vec<InternalId>,
}

/// 评估原因（映射到契约诊断码）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssessmentReason {
    /// 普通战役上下文未确认。
    ContextNotCampaign,
    /// 记录恢复不可用（区段流/家族尺寸失败）。
    InventoryUnsupported,
    /// 物品表不完整（异常 type28 等）；缺席差集禁用。
    InventoryIncomplete,
    /// 参考集合适用范围不足（未知构建/DLC 等）；最多输出原始参考差。
    ReferenceNotApplicable {
        /// 适用性限制说明。
        limits: Vec<String>,
    },
    /// 参考集合外出现同类 ID。
    ExtraCategoryIdsOutsideReference {
        /// 类别 ID。
        category_id: String,
        /// 参考外的同类 ID。
        ids: Vec<InternalId>,
    },
    /// 分类别计数与观察集合不一致。
    CounterMismatch {
        /// 类别 ID。
        category_id: String,
        /// 观察到的类别 ID 数。
        observed: u32,
        /// 计数器汇总。
        counter_total: u32,
    },
    /// 计数汇总包含不可用计数。
    CounterTotalsDegraded,
    /// 元数据冲突。
    MetadataConflict {
        /// 冲突说明。
        detail: String,
    },
}

impl AssessmentReason {
    /// 契约诊断码。
    pub fn diagnostic_code(&self) -> &'static str {
        match self {
            AssessmentReason::ContextNotCampaign => "UNSUPPORTED_CONTEXT",
            AssessmentReason::InventoryUnsupported | AssessmentReason::InventoryIncomplete => {
                "UNSUPPORTED_LAYOUT"
            }
            AssessmentReason::ReferenceNotApplicable { .. } => "METADATA_CONFLICT",
            AssessmentReason::ExtraCategoryIdsOutsideReference { .. }
            | AssessmentReason::CounterMismatch { .. }
            | AssessmentReason::CounterTotalsDegraded => "COUNTER_MISMATCH",
            AssessmentReason::MetadataConflict { .. } => "METADATA_CONFLICT",
        }
    }

    /// 面向诊断消息的说明。
    pub fn message(&self) -> String {
        match self {
            AssessmentReason::ContextNotCampaign => {
                "普通战役上下文未确认：收集推定已关闭".to_string()
            }
            AssessmentReason::InventoryUnsupported => "记录恢复不可用：收集推定已关闭".to_string(),
            AssessmentReason::InventoryIncomplete => {
                "物品表不完整：缺席差集与 likely_* 已禁用".to_string()
            }
            AssessmentReason::ReferenceNotApplicable { limits } => {
                if limits.is_empty() {
                    "参考集合适用范围未确认：最多输出原始参考差".to_string()
                } else {
                    format!(
                        "参考集合适用范围未确认（{}）：最多输出原始参考差",
                        limits.join("；")
                    )
                }
            }
            AssessmentReason::ExtraCategoryIdsOutsideReference { category_id, ids } => {
                format!(
                    "类别 {category_id} 出现参考集合外的同类 ID（{} 个）：差集只作原始观察，不给 likely_missing",
                    ids.len()
                )
            }
            AssessmentReason::CounterMismatch {
                category_id,
                observed,
                counter_total,
            } => {
                format!(
                    "类别 {category_id} 计数不一致（观察 {observed}，计数汇总 {counter_total}）：差集降级为原始观察"
                )
            }
            AssessmentReason::CounterTotalsDegraded => {
                "计数汇总包含不可用读数：差集降级为原始观察".to_string()
            }
            AssessmentReason::MetadataConflict { detail } => {
                format!("元数据冲突（{detail}）：受影响结果降级")
            }
        }
    }
}

/// 评估结果。
#[derive(Debug, Clone, PartialEq)]
pub struct AssessmentOutcome {
    /// 逐项评估（按内部 ID 稳定排序）。
    pub items: Vec<AssessedItem>,
    /// 分类别原始集合差（禁用时为空）。
    pub diff: Vec<CategoryDiff>,
    /// 参考差集能力（供能力矩阵使用）。
    pub capability: Capability,
    /// 结构化原因。
    pub reasons: Vec<AssessmentReason>,
}

impl AssessmentOutcome {
    /// 评估结果中使用的全部原因码（去重）。
    pub fn reason_codes(&self) -> Vec<&'static str> {
        let mut codes = Vec::new();
        for reason in &self.reasons {
            let code = reason.diagnostic_code();
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
        codes
    }
}

/// 执行评估：生成 observation、assessment、rule_id 与原始差集。
pub fn assess(inputs: &AssessmentInputs<'_>) -> AssessmentOutcome {
    let inventory = &inputs.inventory.observation;
    let inventory_usable = inventory.status != SupportStatus::Unsupported;
    let inventory_complete = inventory.inventory_complete;

    let mut reasons = Vec::new();
    if !inputs.context.campaign_applicable {
        reasons.push(AssessmentReason::ContextNotCampaign);
    }
    if !inventory_usable {
        reasons.push(AssessmentReason::InventoryUnsupported);
    } else if !inventory_complete {
        reasons.push(AssessmentReason::InventoryIncomplete);
    }
    let diff_allowed = inputs.context.campaign_applicable && inventory_usable && inventory_complete;
    let mut likely_blocked = !inputs.reference.applicable;
    if diff_allowed && !inputs.reference.applicable {
        reasons.push(AssessmentReason::ReferenceNotApplicable {
            limits: inputs.reference.applicability_limits.clone(),
        });
    }

    // 交叉检查（仅在差集允许时评估）。
    let mut outside_by_category: BTreeMap<String, Vec<InternalId>> = BTreeMap::new();
    if diff_allowed {
        if inputs.cross_checks.counter_totals_degraded {
            reasons.push(AssessmentReason::CounterTotalsDegraded);
            likely_blocked = true;
        }
        for detail in &inputs.cross_checks.metadata_conflicts {
            reasons.push(AssessmentReason::MetadataConflict {
                detail: detail.clone(),
            });
            likely_blocked = true;
        }
        for category in &inputs.reference.categories {
            let empty = BTreeSet::new();
            let observed = inputs
                .observed_category_ids
                .get(&category.category_id)
                .unwrap_or(&empty);
            let outside: Vec<InternalId> = observed
                .iter()
                .filter(|id| !category.reference_ids.contains(id))
                .copied()
                .collect();
            if !outside.is_empty() {
                reasons.push(AssessmentReason::ExtraCategoryIdsOutsideReference {
                    category_id: category.category_id.clone(),
                    ids: outside.clone(),
                });
                outside_by_category.insert(category.category_id.clone(), outside);
                likely_blocked = true;
            }
            if let Some(counter_total) = inputs
                .cross_checks
                .category_counter_totals
                .get(&category.category_id)
            {
                let observed_count = observed.len() as u32;
                if *counter_total != observed_count {
                    reasons.push(AssessmentReason::CounterMismatch {
                        category_id: category.category_id.clone(),
                        observed: observed_count,
                        counter_total: *counter_total,
                    });
                    likely_blocked = true;
                }
            }
        }
    }

    let unique_present: BTreeSet<InternalId> = inventory
        .records
        .iter()
        .map(|record| record.internal_id)
        .collect();
    let reference_category_ids: BTreeSet<&str> = inputs
        .reference
        .categories
        .iter()
        .map(|category| category.category_id.as_str())
        .collect();
    let mut id_category: BTreeMap<InternalId, &str> = BTreeMap::new();
    for (category_id, ids) in inputs.observed_category_ids {
        for id in ids {
            id_category.entry(*id).or_insert(category_id.as_str());
        }
    }

    let limitation_present = "参考集合内且当前表完整；ID 出现为社区相关证据，未经游戏内直接验证";
    let mut items: Vec<AssessedItem> = Vec::new();

    for id in &unique_present {
        let category = id_category.get(id).copied();
        match category {
            Some(category_id) if reference_category_ids.contains(category_id) => {
                let reference_ids = inputs
                    .reference
                    .categories
                    .iter()
                    .find(|view| view.category_id == category_id)
                    .map(|view| &view.reference_ids);
                let in_reference = reference_ids.is_some_and(|ids| ids.contains(id));
                if in_reference {
                    if inputs.context.campaign_applicable
                        && inventory_complete
                        && inputs.reference.applicable
                    {
                        items.push(AssessedItem {
                            internal_id: *id,
                            observation: ObservationKind::Present,
                            assessment: AssessmentKind::LikelyCollected,
                            rule_id: Some(RULE_LIKELY_COLLECTED.to_string()),
                            evidence_refs: record_evidence_refs(*id, inventory),
                            limitations: vec![limitation_present.to_string()],
                        });
                    } else {
                        items.push(AssessedItem {
                            internal_id: *id,
                            observation: ObservationKind::Present,
                            assessment: AssessmentKind::Unknown,
                            rule_id: None,
                            evidence_refs: record_evidence_refs(*id, inventory),
                            limitations: reason_messages(&reasons),
                        });
                    }
                } else {
                    items.push(AssessedItem {
                        internal_id: *id,
                        observation: ObservationKind::Present,
                        assessment: AssessmentKind::Unknown,
                        rule_id: None,
                        evidence_refs: record_evidence_refs(*id, inventory),
                        limitations: vec![
                            "参考集合外同类 ID：保留原始观察，不提升为收集状态".to_string(),
                        ],
                    });
                }
            }
            Some(_) => {
                items.push(AssessedItem {
                    internal_id: *id,
                    observation: ObservationKind::Present,
                    assessment: AssessmentKind::Unknown,
                    rule_id: None,
                    evidence_refs: record_evidence_refs(*id, inventory),
                    limitations: vec!["非受支持参考类别：仅保留身份观测".to_string()],
                });
            }
            None => {
                items.push(AssessedItem {
                    internal_id: *id,
                    observation: ObservationKind::Present,
                    assessment: AssessmentKind::Unknown,
                    rule_id: None,
                    evidence_refs: record_evidence_refs(*id, inventory),
                    limitations: vec!["未分类记录：仅保留身份观测".to_string()],
                });
            }
        }
    }

    let mut diff = Vec::new();
    if diff_allowed {
        for category in &inputs.reference.categories {
            let reference_ids = &category.reference_ids;
            let present_reference_ids: Vec<InternalId> = reference_ids
                .iter()
                .filter(|id| unique_present.contains(id))
                .copied()
                .collect();
            let absent_ids: Vec<InternalId> = reference_ids
                .iter()
                .filter(|id| !unique_present.contains(id))
                .copied()
                .collect();
            let outside_reference_ids = outside_by_category
                .get(&category.category_id)
                .cloned()
                .unwrap_or_default();
            diff.push(CategoryDiff {
                category_id: category.category_id.clone(),
                reference_count: reference_ids.len(),
                present_reference_ids,
                absent_ids: absent_ids.clone(),
                outside_reference_ids,
            });
            for id in absent_ids {
                let assessable = !likely_blocked && inputs.reference.applicable;
                let messages = reason_messages(&reasons);
                items.push(AssessedItem {
                    internal_id: id,
                    observation: ObservationKind::ReferencePresentCurrentAbsent,
                    assessment: if assessable {
                        AssessmentKind::LikelyMissing
                    } else {
                        AssessmentKind::Unknown
                    },
                    rule_id: assessable.then(|| RULE_LIKELY_MISSING.to_string()),
                    evidence_refs: vec![format!(
                        "ev:refdiff:{}",
                        inputs.reference.reference_set_id
                    )],
                    limitations: if assessable {
                        vec!["仅限共同参考类别；不等于全游戏遗漏清单".to_string()]
                    } else if messages.is_empty() {
                        vec!["参考适用性或交叉检查不足：仅原始参考差".to_string()]
                    } else {
                        messages
                    },
                });
            }
        }
    }

    items.sort_by_key(|item| item.internal_id);

    let capability = assessment_capability(
        inputs.context.campaign_applicable,
        inventory_usable,
        inventory_complete,
        inputs.reference.applicable,
        likely_blocked,
        &reasons,
    );

    AssessmentOutcome {
        items,
        diff,
        capability,
        reasons,
    }
}

fn assessment_capability(
    campaign_applicable: bool,
    inventory_usable: bool,
    inventory_complete: bool,
    reference_applicable: bool,
    likely_blocked: bool,
    reasons: &[AssessmentReason],
) -> Capability {
    if !campaign_applicable {
        return unsupported("UNSUPPORTED_CONTEXT");
    }
    if !inventory_usable || !inventory_complete {
        return unsupported("UNSUPPORTED_LAYOUT");
    }
    if !reference_applicable {
        return partial(&["METADATA_CONFLICT"]);
    }
    if likely_blocked {
        let codes: Vec<&str> = reasons
            .iter()
            .map(|reason| reason.diagnostic_code())
            .filter(|code| *code != "UNSUPPORTED_CONTEXT" && *code != "UNSUPPORTED_LAYOUT")
            .collect();
        let mut unique: Vec<&str> = Vec::new();
        for code in codes {
            if !unique.contains(&code) {
                unique.push(code);
            }
        }
        if unique.is_empty() {
            unique.push("COUNTER_MISMATCH");
        }
        return partial(&unique);
    }
    Capability {
        status: SupportStatus::Supported,
        reason_codes: Vec::new(),
    }
}

fn partial(codes: &[&str]) -> Capability {
    Capability {
        status: SupportStatus::Partial,
        reason_codes: codes.iter().map(|code| (*code).to_string()).collect(),
    }
}

fn unsupported(code: &str) -> Capability {
    Capability {
        status: SupportStatus::Unsupported,
        reason_codes: vec![code.to_string()],
    }
}

fn reason_messages(reasons: &[AssessmentReason]) -> Vec<String> {
    reasons.iter().map(|reason| reason.message()).collect()
}

fn record_evidence_refs(
    id: InternalId,
    inventory: &crate::report::InventoryObservation,
) -> Vec<String> {
    let mut refs = Vec::new();
    for record in &inventory.records {
        if record.internal_id != id {
            continue;
        }
        let reference = match record.table {
            crate::report::InventoryTable::Prefix => "ev:inventory:prefix",
            crate::report::InventoryTable::Type28Overflow => "ev:inventory:type28",
        };
        if !refs.iter().any(|existing| existing == reference) {
            refs.push(reference.to_string());
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::context::{DeclaredContext, assess as assess_context};
    use crate::inventory::InventoryReason;
    use crate::report::{
        ContextSource, InventoryObservation, InventoryRecord, InventoryTable, Mode,
    };

    fn id(value: u32) -> InternalId {
        InternalId::new(value)
    }

    fn inventory(ids: &[u32], complete: bool) -> InventoryRecovery {
        let records: Vec<InventoryRecord> = ids
            .iter()
            .enumerate()
            .map(|(index, value)| InventoryRecord {
                table: InventoryTable::Prefix,
                index: index as u32,
                internal_id: id(*value),
                raw_companion_u32: 1,
                raw_companion_hex: "0x00000001".to_string(),
                raw_low_two_bits: 1,
                id_offset: 0x44 + index as u64 * 8,
                id_offset_hex: format!("0x{:X}", 0x44 + index * 8),
            })
            .collect();
        let unique: BTreeSet<InternalId> =
            records.iter().map(|record| record.internal_id).collect();
        InventoryRecovery {
            observation: InventoryObservation {
                status: if complete {
                    SupportStatus::Supported
                } else {
                    SupportStatus::Partial
                },
                inventory_complete: complete,
                completeness_scope: "test scope".to_string(),
                record_count: records.len() as u64,
                unique_id_count: unique.len() as u64,
                duplicate_record_count: records.len() as u64 - unique.len() as u64,
                records,
            },
            reasons: if complete {
                Vec::new()
            } else {
                vec![InventoryReason::Type28LengthMismatch {
                    offset: 0xBFD4,
                    bytes: 4108,
                }]
            },
        }
    }

    fn unavailable_inventory() -> InventoryRecovery {
        InventoryRecovery {
            observation: InventoryObservation {
                status: SupportStatus::Unsupported,
                inventory_complete: false,
                completeness_scope: "unavailable".to_string(),
                records: Vec::new(),
                record_count: 0,
                unique_id_count: 0,
                duplicate_record_count: 0,
            },
            reasons: vec![InventoryReason::SectionStreamUnavailable {
                why: crate::sections::SectionError::InvalidStreamLength,
            }],
        }
    }

    fn reference(applicable: bool) -> ReferenceView {
        ReferenceView {
            reference_set_id: "rs:test".to_string(),
            version: "1.0.0".to_string(),
            applicable,
            applicability_limits: if applicable {
                Vec::new()
            } else {
                vec!["构建号未知".to_string()]
            },
            categories: vec![
                ReferenceCategoryView {
                    category_id: "survival_caches".to_string(),
                    reference_ids: [0xAA01, 0xAA02, 0xAA03].into_iter().map(id).collect(),
                },
                ReferenceCategoryView {
                    category_id: "monoliths".to_string(),
                    reference_ids: [0xBB01].into_iter().map(id).collect(),
                },
            ],
        }
    }

    fn classification(pairs: &[(&str, &[u32])]) -> BTreeMap<String, BTreeSet<InternalId>> {
        pairs
            .iter()
            .map(|(category, ids)| {
                (
                    (*category).to_string(),
                    ids.iter().map(|value| id(*value)).collect(),
                )
            })
            .collect()
    }

    fn campaign() -> ContextAssessment {
        assess_context(&DeclaredContext {
            mode: Mode::Campaign,
            source: ContextSource::UserDeclared,
            ..DeclaredContext::default()
        })
    }

    fn unknown_context() -> ContextAssessment {
        assess_context(&DeclaredContext::default())
    }

    fn evaluate<'a>(
        context: &'a ContextAssessment,
        inventory: &'a InventoryRecovery,
        reference: &'a ReferenceView,
        observed: &'a BTreeMap<String, BTreeSet<InternalId>>,
        cross: &'a CrossCheckInputs,
    ) -> AssessmentOutcome {
        assess(&AssessmentInputs {
            context,
            inventory,
            reference,
            observed_category_ids: observed,
            cross_checks: cross,
        })
    }

    fn item(outcome: &AssessmentOutcome, id: u32) -> &AssessedItem {
        outcome
            .items
            .iter()
            .find(|entry| entry.internal_id == InternalId::new(id))
            .unwrap_or_else(|| panic!("missing assessment for {id:#X}"))
    }

    #[test]
    fn complete_reference_yields_likely_states_with_rules() {
        let inventory = inventory(&[0xAA01, 0xAA02, 0xBB01], true);
        let reference = reference(true);
        let observed = classification(&[
            ("survival_caches", &[0xAA01, 0xAA02]),
            ("monoliths", &[0xBB01]),
        ]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert_eq!(outcome.capability.status, SupportStatus::Supported);
        assert_eq!(
            item(&outcome, 0xAA01).assessment,
            AssessmentKind::LikelyCollected
        );
        assert_eq!(
            item(&outcome, 0xAA01).rule_id.as_deref(),
            Some(RULE_LIKELY_COLLECTED)
        );
        assert_eq!(
            item(&outcome, 0xBB01).assessment,
            AssessmentKind::LikelyCollected
        );
        assert_eq!(
            item(&outcome, 0xAA03).assessment,
            AssessmentKind::LikelyMissing
        );
        assert_eq!(
            item(&outcome, 0xAA03).rule_id.as_deref(),
            Some(RULE_LIKELY_MISSING)
        );
        assert_eq!(
            item(&outcome, 0xAA03).observation,
            ObservationKind::ReferencePresentCurrentAbsent
        );
        assert!(rule_descriptor(RULE_LIKELY_COLLECTED).is_some());
        assert!(rule_descriptor(RULE_LIKELY_MISSING).is_some());

        let survival = outcome
            .diff
            .iter()
            .find(|diff| diff.category_id == "survival_caches")
            .unwrap();
        assert_eq!(survival.reference_count, 3);
        assert_eq!(survival.absent_ids, vec![id(0xAA03)]);
        let monoliths = outcome
            .diff
            .iter()
            .find(|diff| diff.category_id == "monoliths")
            .unwrap();
        assert!(monoliths.absent_ids.is_empty());
    }

    #[test]
    fn empty_diff_is_not_a_completion_claim() {
        let inventory = inventory(&[0xAA01, 0xAA02, 0xAA03, 0xBB01], true);
        let reference = reference(true);
        let observed = classification(&[
            ("survival_caches", &[0xAA01, 0xAA02, 0xAA03]),
            ("monoliths", &[0xBB01]),
        ]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert!(outcome.diff.iter().all(|diff| diff.absent_ids.is_empty()));
        assert!(
            outcome
                .items
                .iter()
                .all(|entry| entry.assessment != AssessmentKind::LikelyMissing),
            "差集为空不产生遗漏结论，也不产生全游戏完成声明"
        );
        assert_eq!(outcome.capability.status, SupportStatus::Supported);
    }

    #[test]
    fn type28_mismatch_disables_diff_and_likely_states() {
        let inventory = inventory(&[0xAA01], false);
        let reference = reference(true);
        let observed = classification(&[("survival_caches", &[0xAA01])]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert!(outcome.diff.is_empty(), "表不完整时连缺席差集也禁用");
        assert_eq!(item(&outcome, 0xAA01).assessment, AssessmentKind::Unknown);
        assert_eq!(item(&outcome, 0xAA01).observation, ObservationKind::Present);
        assert!(
            outcome
                .reasons
                .contains(&AssessmentReason::InventoryIncomplete)
        );
        assert_eq!(outcome.capability.status, SupportStatus::Unsupported);
        assert_eq!(outcome.capability.reason_codes, vec!["UNSUPPORTED_LAYOUT"]);
        assert!(!outcome.items.iter().any(|entry| entry.rule_id.is_some()));
    }

    #[test]
    fn bad_sections_give_no_assessments() {
        let inventory = unavailable_inventory();
        let reference = reference(true);
        let observed = classification(&[]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert!(outcome.items.is_empty());
        assert!(outcome.diff.is_empty());
        assert!(
            outcome
                .reasons
                .contains(&AssessmentReason::InventoryUnsupported)
        );
        assert_eq!(outcome.capability.status, SupportStatus::Unsupported);
    }

    #[test]
    fn unknown_mode_closes_collection_inference_but_keeps_identity() {
        let inventory = inventory(&[0xAA01], true);
        let reference = reference(true);
        let observed = classification(&[("survival_caches", &[0xAA01])]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(
            &unknown_context(),
            &inventory,
            &reference,
            &observed,
            &cross,
        );

        assert_eq!(item(&outcome, 0xAA01).observation, ObservationKind::Present);
        assert_eq!(item(&outcome, 0xAA01).assessment, AssessmentKind::Unknown);
        assert!(item(&outcome, 0xAA01).rule_id.is_none());
        assert_eq!(outcome.capability.status, SupportStatus::Unsupported);
        assert_eq!(outcome.capability.reason_codes, vec!["UNSUPPORTED_CONTEXT"]);
        assert!(
            !outcome
                .items
                .iter()
                .any(|entry| entry.internal_id == id(0xAA03))
        );
    }

    #[test]
    fn extra_category_ids_outside_reference_keep_raw_diff() {
        let inventory = inventory(&[0xAA01, 0xAA09], true);
        let reference = reference(true);
        let observed = classification(&[("survival_caches", &[0xAA01, 0xAA09])]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        let extra = item(&outcome, 0xAA09);
        assert_eq!(extra.assessment, AssessmentKind::Unknown);
        assert!(
            extra
                .limitations
                .iter()
                .any(|text| text.contains("参考集合外同类 ID"))
        );
        assert!(outcome.reasons.iter().any(|reason| matches!(
            reason,
            AssessmentReason::ExtraCategoryIdsOutsideReference { .. }
        )));
        let absent = item(&outcome, 0xAA03);
        assert_eq!(
            absent.observation,
            ObservationKind::ReferencePresentCurrentAbsent
        );
        assert_eq!(
            absent.assessment,
            AssessmentKind::Unknown,
            "不给 likely_missing"
        );
        assert_eq!(outcome.capability.status, SupportStatus::Partial);
        assert_eq!(outcome.capability.reason_codes, vec!["COUNTER_MISMATCH"]);
        assert!(
            outcome
                .diff
                .iter()
                .any(|diff| diff.outside_reference_ids == vec![id(0xAA09)])
        );
    }

    #[test]
    fn counter_mismatch_keeps_raw_diff() {
        let inventory = inventory(&[0xAA01], true);
        let reference = reference(true);
        let observed = classification(&[("survival_caches", &[0xAA01])]);
        let cross = CrossCheckInputs {
            category_counter_totals: [("survival_caches".to_string(), 5)].into_iter().collect(),
            counter_totals_degraded: false,
            metadata_conflicts: Vec::new(),
        };
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert!(outcome.reasons.iter().any(|reason| matches!(
            reason,
            AssessmentReason::CounterMismatch {
                observed: 1,
                counter_total: 5,
                ..
            }
        )));
        assert_eq!(item(&outcome, 0xAA02).assessment, AssessmentKind::Unknown);
        assert_eq!(
            item(&outcome, 0xAA01).assessment,
            AssessmentKind::LikelyCollected
        );
        assert_eq!(outcome.capability.status, SupportStatus::Partial);
    }

    #[test]
    fn counter_totals_degraded_blocks_likely_missing() {
        let inventory = inventory(&[0xAA01], true);
        let reference = reference(true);
        let observed = classification(&[("survival_caches", &[0xAA01])]);
        let cross = CrossCheckInputs {
            category_counter_totals: BTreeMap::new(),
            counter_totals_degraded: true,
            metadata_conflicts: Vec::new(),
        };
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert!(
            outcome
                .reasons
                .contains(&AssessmentReason::CounterTotalsDegraded)
        );
        assert_eq!(item(&outcome, 0xAA02).assessment, AssessmentKind::Unknown);
        assert_eq!(outcome.capability.status, SupportStatus::Partial);
        assert_eq!(outcome.capability.reason_codes, vec!["COUNTER_MISMATCH"]);
    }

    #[test]
    fn metadata_conflict_degrades_with_metadata_code() {
        let inventory = inventory(&[0xAA01], true);
        let reference = reference(true);
        let observed = classification(&[("survival_caches", &[0xAA01])]);
        let cross = CrossCheckInputs {
            category_counter_totals: BTreeMap::new(),
            counter_totals_degraded: false,
            metadata_conflicts: vec!["别名与类别冲突".to_string()],
        };
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert!(
            outcome
                .reasons
                .iter()
                .any(|reason| matches!(reason, AssessmentReason::MetadataConflict { .. }))
        );
        assert_eq!(item(&outcome, 0xAA02).assessment, AssessmentKind::Unknown);
        assert_eq!(outcome.capability.status, SupportStatus::Partial);
        assert_eq!(outcome.capability.reason_codes, vec!["METADATA_CONFLICT"]);
    }

    #[test]
    fn reference_not_applicable_keeps_raw_diff_only() {
        let inventory = inventory(&[0xAA01], true);
        let reference = reference(false);
        let observed = classification(&[("survival_caches", &[0xAA01])]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        assert_eq!(
            item(&outcome, 0xAA01).assessment,
            AssessmentKind::Unknown,
            "不发 likely_collected"
        );
        let absent = item(&outcome, 0xAA02);
        assert_eq!(
            absent.observation,
            ObservationKind::ReferencePresentCurrentAbsent
        );
        assert_eq!(absent.assessment, AssessmentKind::Unknown);
        assert!(
            outcome
                .reasons
                .iter()
                .any(|reason| matches!(reason, AssessmentReason::ReferenceNotApplicable { .. }))
        );
        assert_eq!(outcome.capability.status, SupportStatus::Partial);
        assert_eq!(outcome.capability.reason_codes, vec!["METADATA_CONFLICT"]);
        let entered = item(&outcome, 0xAA02);
        assert!(
            entered
                .limitations
                .iter()
                .any(|text| text.contains("构建号未知"))
        );
    }

    #[test]
    fn non_reference_categories_keep_identity_only() {
        let inventory = inventory(&[0xCC01], true);
        let reference = reference(true);
        let observed = classification(&[("outfits", &[0xCC01])]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        let entry = item(&outcome, 0xCC01);
        assert_eq!(entry.observation, ObservationKind::Present);
        assert_eq!(entry.assessment, AssessmentKind::Unknown);
        assert!(entry.rule_id.is_none());
        assert!(
            entry
                .limitations
                .iter()
                .any(|text| text.contains("非受支持参考类别"))
        );
    }

    #[test]
    fn unclassified_ids_keep_identity_only() {
        let inventory = inventory(&[0xDD01], true);
        let reference = reference(true);
        let observed = classification(&[]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        let entry = item(&outcome, 0xDD01);
        assert_eq!(entry.assessment, AssessmentKind::Unknown);
        assert!(
            entry
                .limitations
                .iter()
                .any(|text| text.contains("未分类记录"))
        );
    }

    #[test]
    fn duplicate_records_deduplicate_assessments() {
        let inventory = inventory(&[0xAA01, 0xAA01], true);
        assert_eq!(inventory.observation.duplicate_record_count, 1);
        let reference = reference(true);
        let observed = classification(&[("survival_caches", &[0xAA01])]);
        let cross = CrossCheckInputs::default();
        let outcome = evaluate(&campaign(), &inventory, &reference, &observed, &cross);

        let matches: Vec<&AssessedItem> = outcome
            .items
            .iter()
            .filter(|entry| entry.internal_id == id(0xAA01))
            .collect();
        assert_eq!(matches.len(), 1, "逐项评估按唯一 ID 聚合");
    }

    #[test]
    fn rule_registry_is_consistent() {
        for descriptor in RULE_REGISTRY {
            assert!(!descriptor.rule_id.is_empty());
            assert!(!descriptor.description.is_empty());
            assert_eq!(
                rule_descriptor(descriptor.rule_id).map(|found| found.rule_id),
                Some(descriptor.rule_id)
            );
        }
        assert!(rule_descriptor("unknown.rule").is_none());
    }
}
