//! 组级定位门禁（SX-008 界面交付 / 验收矩阵 T19）。
//!
//! `whole_group_check`（“本组可按攻略逐一检查”）只在以下条件**全部**满足时给出：
//! 当前快照（`freshness = current`）、普通战役上下文、物品表恢复完整、
//! 参考集合适用、组成员落于参考集合、计数键可解析且来源目标之和与组目标数一致、
//! 组计数当前读数为零且未降级。
//!
//! 任一条件不满足时按稳定原因码降级为 `general_reference` 或 `unavailable`：
//! - 非零计数：撤销“整组均待查”提示，仅作全组参考；
//! - 计数未知/冲突：不给整组遗漏行动结论；
//! - 表不完整、分析失败或快照过期：关闭行动摘要（`unavailable`）；
//! - 前端不得从“0/8”文案或计数自行推导本门禁，必须在每次刷新/切槽后
//!   由本模块重新评估（见开发方案 SX-008 界面交付与 SX-016）。
//!
//! 本模块不产生精确点位：同组多个 ID 保留原始身份，但只有一个组级线索。

use std::collections::BTreeSet;

use crate::domain::assessment::ReferenceView;
use crate::domain::context::ContextAssessment;
use crate::domain::progress::RegionCounterSpec;
use crate::metadata::GuideGroup;
use crate::report::{
    Freshness, GroupActionKind, GroupActionability, InputDigest, InternalId, InventoryObservation,
    RegionCounter, SupportStatus,
};

/// 表恢复完整（`whole_group_check` 条件之一）。
pub const GATE_COMPLETE_TABLE: &str = "COMPLETE_TABLE";
/// 组计数当前为零（`whole_group_check` 条件之一）。
pub const GATE_ZERO_GROUP_COUNT: &str = "ZERO_GROUP_COUNT";
/// 参考范围与组关联匹配（`whole_group_check` 条件之一）。
pub const GATE_REFERENCE_MATCH: &str = "REFERENCE_MATCH";
/// 组计数非零：撤销整组均待查提示。
pub const GATE_GROUP_COUNT_NONZERO: &str = "GROUP_COUNT_NONZERO";
/// 组计数未知（计数缺失或不可用）。
pub const GATE_GROUP_COUNT_UNKNOWN: &str = "GROUP_COUNT_UNKNOWN";
/// 组计数冲突（原始读数超过来源目标）。
pub const GATE_GROUP_COUNT_CONFLICT: &str = "GROUP_COUNT_CONFLICT";
/// 物品表不完整：关闭行动摘要。
pub const GATE_INVENTORY_INCOMPLETE: &str = "INVENTORY_INCOMPLETE";
/// 物品表不可用：关闭行动摘要。
pub const GATE_INVENTORY_UNAVAILABLE: &str = "INVENTORY_UNAVAILABLE";
/// 上下文不是普通战役：关闭行动摘要。
pub const GATE_CONTEXT_NOT_CAMPAIGN: &str = "CONTEXT_NOT_CAMPAIGN";
/// 参考集合适用性未确认：不给整组遗漏结论。
pub const GATE_REFERENCE_NOT_APPLICABLE: &str = "REFERENCE_NOT_APPLICABLE";
/// 组关联未确认（成员不在参考范围或计数目标不匹配）。
pub const GATE_GROUP_ASSOCIATION_UNCONFIRMED: &str = "GROUP_ASSOCIATION_UNCONFIRMED";
/// 快照不是当前快照（旧报告/刷新中/无快照）。
pub const GATE_SNAPSHOT_NOT_CURRENT: &str = "SNAPSHOT_NOT_CURRENT";

/// 门禁输入（由组装层提供；全部绑定同一报告与快照）。
pub struct GroupGateInputs<'a> {
    /// 元数据攻略组。
    pub group: &'a GuideGroup,
    /// 计数映射规格（来自元数据，与报告计数键一致）。
    pub counter_specs: &'a [RegionCounterSpec],
    /// 报告的组计数观测。
    pub counters: &'a [RegionCounter],
    /// 上下文评估。
    pub context: &'a ContextAssessment,
    /// 物品表观测（完整性）。
    pub inventory: &'a InventoryObservation,
    /// 参考集合视图（适用性）。
    pub reference: &'a ReferenceView,
    /// 当前快照时效；非 `current` 一律降级。
    pub freshness: Freshness,
    /// 评估所绑定的输入摘要（写入 `evaluated_input_digest`）。
    pub input_digest: &'a InputDigest,
}

/// 组计数状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupCountState {
    Zero,
    NonZero,
    Unknown,
    Conflict,
}

/// 评估组级行动门禁；不读取缓存，不依赖文案。
pub fn evaluate_group_gate(inputs: &GroupGateInputs<'_>) -> GroupActionability {
    let digest = inputs.input_digest.clone();
    let evidence = group_evidence_refs(inputs);

    if inputs.freshness != Freshness::Current {
        return action(
            GroupActionKind::Unavailable,
            &[GATE_SNAPSHOT_NOT_CURRENT],
            evidence,
            digest,
        );
    }
    if !inputs.context.campaign_applicable {
        return action(
            GroupActionKind::Unavailable,
            &[GATE_CONTEXT_NOT_CAMPAIGN],
            evidence,
            digest,
        );
    }
    if inputs.inventory.status == SupportStatus::Unsupported {
        return action(
            GroupActionKind::Unavailable,
            &[GATE_INVENTORY_UNAVAILABLE],
            evidence,
            digest,
        );
    }

    let mut codes: Vec<&str> = Vec::new();
    let inventory_incomplete = !inputs.inventory.inventory_complete;
    if inventory_incomplete {
        codes.push(GATE_INVENTORY_INCOMPLETE);
    }

    let count_state = group_count_state(inputs.group, inputs.counters);
    match count_state {
        GroupCountState::Zero => {}
        GroupCountState::NonZero => codes.push(GATE_GROUP_COUNT_NONZERO),
        GroupCountState::Unknown => codes.push(GATE_GROUP_COUNT_UNKNOWN),
        GroupCountState::Conflict => codes.push(GATE_GROUP_COUNT_CONFLICT),
    }

    let reference_applicable = inputs.reference.applicable;
    let association_ok = group_association_ok(inputs.group, inputs.reference, inputs.counter_specs);
    if count_state == GroupCountState::Zero {
        if !reference_applicable {
            codes.push(GATE_REFERENCE_NOT_APPLICABLE);
        } else if !association_ok {
            codes.push(GATE_GROUP_ASSOCIATION_UNCONFIRMED);
        }
    }

    if inventory_incomplete {
        return action(GroupActionKind::Unavailable, &codes, evidence, digest);
    }

    let whole_group_ready = codes.is_empty()
        && count_state == GroupCountState::Zero
        && reference_applicable
        && association_ok;
    if whole_group_ready {
        return action(
            GroupActionKind::WholeGroupCheck,
            &[
                GATE_COMPLETE_TABLE,
                GATE_ZERO_GROUP_COUNT,
                GATE_REFERENCE_MATCH,
            ],
            evidence,
            digest,
        );
    }
    if codes.is_empty() {
        codes.push(GATE_GROUP_ASSOCIATION_UNCONFIRMED);
    }
    action(GroupActionKind::GeneralReference, &codes, evidence, digest)
}

fn action(
    kind: GroupActionKind,
    codes: &[&str],
    evidence_refs: Vec<String>,
    evaluated_input_digest: InputDigest,
) -> GroupActionability {
    let mut reason_codes = Vec::new();
    for code in codes {
        if !reason_codes.iter().any(|existing| existing == code) {
            reason_codes.push((*code).to_string());
        }
    }
    GroupActionability {
        kind,
        reason_codes,
        evidence_refs,
        evaluated_input_digest,
    }
}

fn group_count_state(group: &GuideGroup, counters: &[RegionCounter]) -> GroupCountState {
    if group.counter_keys.is_empty() {
        return GroupCountState::Unknown;
    }
    let mut sum = 0u64;
    let mut unknown = false;
    let mut conflict = false;
    for counter_key in &group.counter_keys {
        match counters
            .iter()
            .find(|counter| &counter.counter_key == counter_key)
        {
            Some(counter) => {
                if !counter.within_source_target {
                    conflict = true;
                }
                match counter.value {
                    Some(value) => sum += u64::from(value),
                    None => unknown = true,
                }
            }
            None => unknown = true,
        }
    }
    if conflict {
        GroupCountState::Conflict
    } else if unknown {
        GroupCountState::Unknown
    } else if sum == 0 {
        GroupCountState::Zero
    } else {
        GroupCountState::NonZero
    }
}

fn group_association_ok(
    group: &GuideGroup,
    reference: &ReferenceView,
    counter_specs: &[RegionCounterSpec],
) -> bool {
    if group.counter_keys.is_empty() {
        return false;
    }
    let mut member_union: BTreeSet<InternalId> = BTreeSet::new();
    for category_id in &group.category_ids {
        match reference
            .categories
            .iter()
            .find(|category| &category.category_id == category_id)
        {
            Some(category) => member_union.extend(category.reference_ids.iter().copied()),
            None => return false,
        }
    }
    if !group.member_ids.iter().all(|id| member_union.contains(id)) {
        return false;
    }
    let mut target_sum = 0u32;
    for counter_key in &group.counter_keys {
        match counter_specs
            .iter()
            .find(|spec| &spec.counter_key == counter_key)
        {
            Some(spec) => target_sum = target_sum.saturating_add(spec.source_total),
            None => return false,
        }
    }
    group.target_count == Some(target_sum)
}

fn group_evidence_refs(inputs: &GroupGateInputs<'_>) -> Vec<String> {
    let mut refs = Vec::new();
    for counter_key in &inputs.group.counter_keys {
        if let Some(counter) = inputs
            .counters
            .iter()
            .find(|counter| &counter.counter_key == counter_key)
        {
            for reference in &counter.evidence_refs {
                if !refs.contains(reference) {
                    refs.push(reference.clone());
                }
            }
        }
    }
    if refs.is_empty()
        && let Some(reference_set_id) = &inputs.group.reference_set_id
    {
        refs.push(format!("ev:refdiff:{reference_set_id}"));
    }
    refs
}
