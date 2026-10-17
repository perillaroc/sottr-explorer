//! 按区域的收集计划（SX-022）：图鉴 + 存档观察集合 → 逐区域 / 逐类别的收集状态与查找线索。
//!
//! 与 `missing` 的分工：`missing` 回答「100% 必收还差哪些」（按类别），
//! 本模块回答「这个区域里有什么、收了没有、去哪找、怎么拿」（按区域）。
//!
//! 规则（纯函数，无文件系统/网络/时间依赖）：
//! - 状态只有两态：`collected`（条目 ID 在观察集合内）/ `not_collected`。
//!   观察集合就是存档里已记录的收集品集合（SX-022 已用存档区域计数器逐行验证）。
//! - **未闭合口径不进入状态结论**：`required_for_100 = null` 的类别（挑战古墓）
//!   与 DLC / 非必收条目照常给出状态，但计划里显式携带 `required_for_100` 与 `is_dlc`，
//!   由展示层区分，不混入 100% 进度。
//! - 区域进度分母是**该区域图鉴条目数**，不是游戏内地图百分比（后者口径未闭合，见 SX-020）。
//! - 特殊区域（跨区域挑战古墓、全局/特殊区域、克劳馥庄园）单独成组，`kind = special`。
//! - 位置线索来自图鉴的关卡内归一化坐标，**未校准**；文案必须显式标注。
//! - 不在图鉴内的观察 ID 单列，不混入任何区域或类别。

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::catalog::{Catalog, CatalogItem, CatalogPosition};
use crate::missing::normalize_observed;

/// 条目收集状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    /// 存档观察集合中出现（已收集）。
    Collected,
    /// 存档观察集合中未出现（未收集）。
    NotCollected,
}

/// 区域分组性质。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind {
    /// 有存档区域计数器的主区域（9 个）。
    Main,
    /// 特殊区域（跨区域挑战古墓、全局区域、克劳馥庄园）：单独成组，进度不与主区域混算。
    Special,
}

/// 子关卡（单位）命名。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitPlan {
    /// 单位 ID。
    pub unit: u32,
    /// 游戏 zone 名（如 `pa_hub_paititi`）。
    pub zone: Option<String>,
    /// 中文显示名。
    pub display_zh: Option<String>,
    /// 英文显示名。
    pub display_en: Option<String>,
    /// 名称来源（`game_locale` / `curated` / `zone_name`）。
    pub name_source: String,
}

/// 逐项计划条目。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemPlan {
    /// 条目键（`0x` + 8 位大写十六进制）。
    pub id: String,
    /// 展示名（游戏中文名 → 英文名 → 类别占位）。
    pub display_name: String,
    /// 游戏英文名（故事条目）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    /// 游戏中文名（故事条目）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_zh: Option<String>,
    /// 名称来源；为空表示没有游戏正式名（非故事收集品在游戏内本就没有独立名）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_source: Option<String>,
    /// 内部候选名（MIT 字典），供搜索。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_name: Option<String>,
    /// 故事合集名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// 收集状态。
    pub status: ItemStatus,
    /// 是否 DLC 条目。
    pub is_dlc: bool,
    /// `true` 必收 / `false` 可选 / `null` 未闭合（不进入 100% 进度结论）。
    pub required_for_100: Option<bool>,
    /// 所属子关卡。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<UnitPlan>,
    /// 关卡内相对位置（未校准）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<CatalogPosition>,
    /// 获取方式正文（逐项注记优先，否则为类别通用规则）。
    pub acquisition_text: String,
    /// 获取方式来源：`item_note`（人工逐项注记）/ `category_rule`（类别通用规则）。
    pub acquisition_source: &'static str,
    /// 位置线索（区域 · 子关卡 · 相对位置 · 同子关卡未收集数）。
    pub location_hint: String,
    /// 同一子关卡内**除本项以外**仍未收集的条目数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_unit_remaining: Option<usize>,
    /// 共用同一 ID 的全部名称（存在时不得按名称拆分判定）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_names: Option<Vec<String>>,
}

/// 类别（或故事子类别）分组。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryPlan {
    /// 类别 ID。
    pub category: String,
    /// 子类别 ID（故事条目 `relics` / `documents` / `murals`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    /// 分组展示名。
    pub name: String,
    /// `true` 必收 / `false` 可选 / `null` 未闭合。
    pub required_for_100: Option<bool>,
    /// 已收集数。
    pub collected: usize,
    /// 总数。
    pub total: usize,
    /// 获取方式规则键。
    pub acquisition_rule_key: String,
    /// 逐项计划（按子关卡、位置、ID 稳定排序）。
    pub items: Vec<ItemPlan>,
}

/// 区域计划。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegionPlan {
    /// 区域 slug。
    pub slug: String,
    /// 区域哈希。
    pub region_hash: String,
    /// 中文显示名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_zh: Option<String>,
    /// 英文显示名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    /// 名称来源。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_source: Option<String>,
    /// 分组性质。
    pub kind: RegionKind,
    /// 归属口径说明（特殊区域）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// 已收集数。
    pub collected: usize,
    /// 总数。
    pub total: usize,
    /// 分组清单（按类别，故事类别再按子类别）。
    pub groups: Vec<CategoryPlan>,
}

/// 全收集计划。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CollectionPlan {
    /// 图鉴数据版本。
    pub catalog_version: String,
    /// 主区域合计已收集数（不含特殊区域）。
    pub main_collected: usize,
    /// 主区域合计总数。
    pub main_total: usize,
    /// 区域计划（主区域按图鉴顺序，特殊区域在后）。
    pub regions: Vec<RegionPlan>,
    /// 观察集合中不属于图鉴的 ID（未知条目，保持原样，单列不混入）。
    pub observed_not_in_catalog: Vec<String>,
}

/// 分组键：类别 + 可选的故事子类别。
type GroupKey = (String, Option<String>);
/// 分组下的条目引用（ID + 图鉴条目）。
type ItemRefs<'a> = Vec<(&'a String, &'a CatalogItem)>;

fn region_kind(slug: &str) -> RegionKind {
    if slug.starts_with('_') || slug == "croft_manor" {
        RegionKind::Special
    } else {
        RegionKind::Main
    }
}

fn unit_plan(catalog: &Catalog, unit: u32) -> Option<UnitPlan> {
    if unit == 0 {
        return None;
    }
    let entry = catalog.units.get(&unit.to_string())?;
    Some(UnitPlan {
        unit,
        zone: entry.zone.clone(),
        display_zh: entry.display_zh.clone(),
        display_en: entry.display_en.clone(),
        name_source: entry.name_source.clone(),
    })
}

/// 展示名：游戏中文名 → 游戏英文名 → 内部候选名 → 短 ID。
fn display_name(item: &CatalogItem, id: &str) -> String {
    if let Some(name) = item.name_zh.as_deref().filter(|value| !value.is_empty()) {
        return name.to_string();
    }
    if let Some(name) = item.name_en.as_deref().filter(|value| !value.is_empty()) {
        return name.to_string();
    }
    if let Some(name) = item.name.as_deref().filter(|value| !value.is_empty()) {
        return name.to_string();
    }
    id.to_string()
}

/// 位置线索文案：区域 · 子关卡 · 相对位置（未校准）· 同子关卡其他未收集数。
fn location_hint(
    region_name: &str,
    unit: Option<&UnitPlan>,
    position: Option<&CatalogPosition>,
    others_uncollected: Option<usize>,
) -> String {
    let mut parts = vec![region_name.to_string()];
    let unit_name = unit.and_then(|entry| {
        entry
            .display_zh
            .clone()
            .or_else(|| entry.display_en.clone())
            .or_else(|| entry.zone.clone())
    });
    if let Some(name) = unit_name
        && !parts.iter().any(|part| part == &name)
    {
        parts.push(name);
    }
    let mut text = parts.join(" · ");
    if let Some(position) = position {
        text.push_str(&format!(
            "；关卡内相对位置约 ({:.2}, {:.2})（未校准，仅供同关卡内相对参照）",
            position.x, position.y
        ));
    }
    if let Some(others) = others_uncollected.filter(|value| *value > 0) {
        text.push_str(&format!("；该子关卡还有 {others} 项未收集"));
    }
    text
}

/// 计算按区域的收集计划。
pub fn plan(catalog: &Catalog, observed: &BTreeSet<String>) -> CollectionPlan {
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

    // 区域名查询 + 每个 slug 对应的条目
    let mut by_region: BTreeMap<&str, Vec<(&String, &CatalogItem)>> = BTreeMap::new();
    for (id, item) in &catalog.items {
        if let Some(slug) = item.region_slug.as_deref() {
            by_region.entry(slug).or_default().push((id, item));
        }
    }

    // 同一子关卡内未收集数（跨类别统计：同一关卡里还有多少东西要找）
    let mut unit_remaining: BTreeMap<(&str, u32), usize> = BTreeMap::new();
    for (slug, entries) in &by_region {
        for (id, item) in entries {
            if normalized.contains(*id) {
                continue;
            }
            if let Some(unit) = item.unit.filter(|value| *value != 0) {
                *unit_remaining.entry((slug, unit)).or_insert(0) += 1;
            }
        }
    }

    let mut regions: Vec<RegionPlan> = Vec::new();
    let mut main_collected = 0usize;
    let mut main_total = 0usize;

    // 主区域按图鉴 regions 顺序（游戏区域表顺序），特殊区域在后
    let mut ordered: Vec<(RegionKind, String, String)> = Vec::new();
    for region in &catalog.regions {
        let Some(slug) = region.slug.as_deref() else {
            continue;
        };
        if !by_region.contains_key(slug) {
            continue;
        }
        ordered.push((
            region_kind(slug),
            slug.to_string(),
            region.region_hash.clone(),
        ));
    }
    ordered.sort_by_key(|(kind, _, _)| if *kind == RegionKind::Main { 0 } else { 1 });

    for (kind, slug, region_hash) in ordered {
        let entries = &by_region[slug.as_str()];
        let record = catalog
            .regions
            .iter()
            .find(|region| region.region_hash == region_hash);
        let name_zh = record.and_then(|region| region.name_zh.clone());
        let name_en = record.and_then(|region| region.name_en.clone());
        let region_name = name_zh
            .clone()
            .or_else(|| name_en.clone())
            .unwrap_or_else(|| slug.clone());

        // 类别 → 子类别 → 条目
        let mut buckets: BTreeMap<GroupKey, ItemRefs> = BTreeMap::new();
        for (id, item) in entries {
            buckets
                .entry((item.category.clone(), item.subcategory.clone()))
                .or_default()
                .push((*id, *item));
        }

        let mut groups: Vec<CategoryPlan> = Vec::new();
        let mut region_collected = 0usize;
        for ((category_id, subcategory), mut bucket) in buckets {
            let category = catalog.category(&category_id);
            let rule_key = match &subcategory {
                Some(sub) => category
                    .and_then(|entry| entry.subcategories.as_ref())
                    .and_then(|list| list.iter().find(|entry| &entry.id == sub))
                    .map(|entry| entry.acquisition_rule_key.clone())
                    .unwrap_or_else(|| {
                        category
                            .map(|entry| entry.acquisition_rule_key.clone())
                            .unwrap_or_default()
                    }),
                None => category
                    .map(|entry| entry.acquisition_rule_key.clone())
                    .unwrap_or_default(),
            };
            let rule_text = catalog
                .acquisition_rules
                .get(&rule_key)
                .map(|rule| rule.text_zh.clone())
                .unwrap_or_default();
            let group_name = match &subcategory {
                Some(sub) => category
                    .and_then(|entry| entry.subcategories.as_ref())
                    .and_then(|list| list.iter().find(|entry| &entry.id == sub))
                    .map(|entry| entry.name.clone())
                    .unwrap_or_else(|| sub.clone()),
                None => category
                    .map(|entry| entry.name.clone())
                    .unwrap_or(category_id.clone()),
            };

            // 稳定排序：子关卡 → 位置（y 再 x）→ ID
            bucket.sort_by(|left, right| {
                let key = |item: &CatalogItem| {
                    (
                        item.unit.unwrap_or(0),
                        item.position.as_ref().map(|p| p.y).unwrap_or(-1.0),
                        item.position.as_ref().map(|p| p.x).unwrap_or(-1.0),
                    )
                };
                key(left.1)
                    .partial_cmp(&key(right.1))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.0.cmp(right.0))
            });

            let mut items = Vec::new();
            let mut collected = 0usize;
            for (id, item) in bucket {
                let status = if normalized.contains(id) {
                    collected += 1;
                    ItemStatus::Collected
                } else {
                    ItemStatus::NotCollected
                };
                let unit = item.unit.and_then(|value| unit_plan(catalog, value));
                let others_uncollected = item
                    .unit
                    .filter(|value| *value != 0)
                    .and_then(|value| unit_remaining.get(&(slug.as_str(), value)).copied())
                    .map(|total| {
                        if status == ItemStatus::NotCollected {
                            total.saturating_sub(1)
                        } else {
                            total
                        }
                    });
                let note = item
                    .acquisition_note
                    .as_deref()
                    .filter(|value| !value.is_empty());
                items.push(ItemPlan {
                    id: id.clone(),
                    display_name: display_name(item, id),
                    name_en: item.name_en.clone(),
                    name_zh: item.name_zh.clone(),
                    name_source: item.name_source.clone(),
                    internal_name: item.name.clone(),
                    collection: item.collection.clone(),
                    status,
                    is_dlc: item.is_dlc,
                    required_for_100: item.required_for_100,
                    unit: unit.clone(),
                    position: item.position.clone(),
                    acquisition_text: note.unwrap_or(&rule_text).to_string(),
                    acquisition_source: if note.is_some() {
                        "item_note"
                    } else {
                        "category_rule"
                    },
                    location_hint: location_hint(
                        &region_name,
                        unit.as_ref(),
                        item.position.as_ref(),
                        others_uncollected,
                    ),
                    same_unit_remaining: others_uncollected,
                    shared_names: item.shared_names.clone(),
                });
            }
            region_collected += collected;
            groups.push(CategoryPlan {
                category: category_id,
                subcategory,
                name: group_name,
                required_for_100: category.and_then(|entry| entry.required_for_100),
                collected,
                total: items.len(),
                acquisition_rule_key: rule_key,
                items,
            });
        }

        let region_total = region_collected
            + groups
                .iter()
                .map(|group| group.items.len() - group.collected)
                .sum::<usize>();
        if kind == RegionKind::Main {
            main_collected += region_collected;
            main_total += region_total;
        }
        regions.push(RegionPlan {
            slug,
            region_hash,
            name_zh,
            name_en,
            name_source: record.and_then(|region| region.name_source.clone()),
            kind,
            note: record.and_then(|region| region.note.clone()),
            collected: region_collected,
            total: region_total,
            groups,
        });
    }

    CollectionPlan {
        catalog_version: catalog.version().to_string(),
        main_collected,
        main_total,
        regions,
        observed_not_in_catalog: not_in_catalog,
    }
}
