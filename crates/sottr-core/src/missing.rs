//! 全收集缺失清单（SX-021 阶段 C）：由全收集目录 + 存档观察集合计算。
//!
//! 规则（与 T22 验收一致）：
//! - 必收类别（`required_for_100 = Some(true)`）：给出逐项缺失清单；
//!   缺失 = 条目 ID 未出现在所选存档的观察集合中；计数与清单均按必收子集
//!   （DLC 与非必收条目不混入）。
//! - 可选类别（`Some(false)`）：给出计数与缺失数，不生成逐项清单。
//! - 未闭合类别（`None`，如挑战古墓）：不给出缺失数或清单，只给观察计数与原因。
//! - 共用 ID 条目：缺失条目携带 `shared_names`（一个 ID 对应多个名称），
//!   按 ID 粒度计数，不拆分假点。
//! - 观察集合中不属于目录的 ID 单独列出（未知），不混入任何类别的缺失。
//!
//! 纯函数：无文件系统、网络或时间依赖。

use std::collections::BTreeSet;

use serde::Serialize;

use crate::catalog::{Catalog, CatalogItem};

/// 未闭合类别不生成缺失清单。
pub const REASON_UNRESOLVED: &str = "未闭合类别：不生成缺失清单（T22）";
/// 可选类别不生成逐项缺失清单。
pub const REASON_OPTIONAL: &str = "可选类别：不在 100% 必需范围，不生成逐项清单";

/// 类别缺失进度。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CategoryMissing {
    /// 类别 ID。
    pub id: String,
    /// 展示名。
    pub name: String,
    /// `true` 必收 / `false` 可选 / `null` 未闭合。
    pub required_for_100: Option<bool>,
    /// 目录条目数（按观测 ID 粒度）。
    pub total: usize,
    /// 观察到的条目数。
    pub observed: usize,
    /// 缺失数；未闭合类别为 `None`。
    pub missing: Option<usize>,
    /// 不生成清单或缺失口径的原因。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 逐项缺失清单（仅必收类别给出）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<MissingItem>>,
}

/// 单条缺失。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingItem {
    /// 条目键（`0x` + 8 位大写十六进制）。
    pub id: String,
    /// 展示名称（可空）。
    pub name: Option<String>,
    /// 故事合集名（故事条目）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// 共用同一 ID 的全部名称（存在时不得按名称拆分判定）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_names: Option<Vec<String>>,
}

/// 全收集缺失报告。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingReport {
    /// 目录数据版本。
    pub catalog_version: String,
    /// 观察到的目录条目数。
    pub observed_items: usize,
    /// 观察集合中不属于目录的 ID（未知条目，保持原样）。
    pub observed_not_in_catalog: Vec<String>,
    /// 按目录顺序的类别进度。
    pub categories: Vec<CategoryMissing>,
}

/// 计算缺失报告。
pub fn compute_missing(catalog: &Catalog, observed: &BTreeSet<String>) -> MissingReport {
    let mut normalized: BTreeSet<String> = BTreeSet::new();
    let mut not_in_catalog: Vec<String> = Vec::new();
    for raw in observed {
        match normalize_observed(raw) {
            Some(id) => {
                if !catalog.items.contains_key(&id) {
                    not_in_catalog.push(id.clone());
                }
                normalized.insert(id);
            }
            None => not_in_catalog.push(raw.trim().to_ascii_uppercase()),
        }
    }
    not_in_catalog.sort();
    not_in_catalog.dedup();

    let mut categories = Vec::new();
    for category in &catalog.categories {
        let entries: Vec<(&String, &CatalogItem)> = catalog
            .items
            .iter()
            .filter(|(_, item)| item.category == category.id)
            .collect();
        let mut progress = CategoryMissing {
            id: category.id.clone(),
            name: category.name.clone(),
            required_for_100: category.required_for_100,
            total: 0,
            observed: 0,
            missing: None,
            reason: None,
            items: None,
        };
        match category.required_for_100 {
            Some(true) => {
                // 必收类别只统计必收子集：DLC 与非必收条目不混入进度与清单。
                let required_entries: Vec<(&String, &CatalogItem)> = entries
                    .iter()
                    .copied()
                    .filter(|(_, item)| item.required_for_100 == Some(true))
                    .collect();
                let observed_count = required_entries
                    .iter()
                    .filter(|(id, _)| normalized.contains(*id))
                    .count();
                let mut items: Vec<MissingItem> = required_entries
                    .iter()
                    .filter(|(id, _)| !normalized.contains(*id))
                    .map(|(id, item)| MissingItem {
                        id: (*id).clone(),
                        name: item.name.clone(),
                        collection: item.collection.clone(),
                        shared_names: item.shared_names.clone(),
                    })
                    .collect();
                items.sort_by(|left, right| left.id.cmp(&right.id));
                progress.total = required_entries.len();
                progress.observed = observed_count;
                progress.missing = Some(items.len());
                progress.items = Some(items);
            }
            Some(false) | None => {
                let observed_count = entries
                    .iter()
                    .filter(|(id, _)| normalized.contains(*id))
                    .count();
                progress.total = entries.len();
                progress.observed = observed_count;
                if category.required_for_100 == Some(false) {
                    progress.missing = Some(entries.len() - observed_count);
                    progress.reason = Some(REASON_OPTIONAL.to_string());
                } else {
                    progress.reason = Some(REASON_UNRESOLVED.to_string());
                }
            }
        }
        categories.push(progress);
    }

    MissingReport {
        catalog_version: catalog.version().to_string(),
        observed_items: normalized
            .iter()
            .filter(|id| catalog.items.contains_key(*id))
            .count(),
        observed_not_in_catalog: not_in_catalog,
        categories,
    }
}

/// 规范化为目录键形式（`0x` + 8 位大写十六进制）；非法返回 `None`。
pub fn normalize_observed(id: &str) -> Option<String> {
    let trimmed = id.trim();
    let body = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))?;
    if body.len() == 8 && body.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(format!("0x{}", body.to_ascii_uppercase()))
    } else {
        None
    }
}
