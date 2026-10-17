//! 按区域收集计划（SX-022）：真实图鉴 + 合成观察集合的规则与保护性断言。
//!
//! 真实数据的区域×子类别计数与 `research/s022/scripts/validate_region_counters.py`
//! 对存档区域计数器的对账结果一致（43 行全部吻合）；任何数字变化都必须同时更新
//! `data/catalog/` 与 `research/s022/README.md`。

use std::collections::BTreeSet;

use sottr_core::atlas::{ItemStatus, RegionKind, plan};
use sottr_core::catalog::Catalog;
use sottr_core::metadata::MetadataLimits;

const CATALOG: &str = include_str!("../../../data/catalog/collectibles_catalog.json");

fn load() -> Catalog {
    Catalog::from_json(CATALOG.as_bytes(), &MetadataLimits::default()).expect("图鉴应可加载")
}

fn observed(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).to_string()).collect()
}

#[test]
fn real_catalog_regions_are_all_bound_and_named() {
    let catalog = load();
    let with_items: Vec<_> = catalog
        .regions
        .iter()
        .filter(|region| region.item_count > 0)
        .collect();
    assert_eq!(with_items.len(), 12, "有收集品的区域应为 12 个");
    for region in &with_items {
        assert!(region.slug.is_some(), "{} 缺 slug", region.region_hash);
        assert!(region.name_zh.is_some(), "{} 缺中文名", region.region_hash);
    }
    // 主区域必须有签名绑定证据；特殊区域（跨区域挑战古墓 / 全局区域 / 庄园）没有计数器，绑定为空。
    let special = ["_challenge_tombs", "_special", "croft_manor"];
    for region in &with_items {
        let slug = region.slug.as_deref().unwrap_or_default();
        if special.contains(&slug) {
            assert!(region.binding.is_none(), "特殊区域 {slug} 不应有签名绑定");
        } else {
            assert!(region.binding.is_some(), "主区域 {slug} 缺绑定证据");
        }
    }
    // 9 个主区域的签名距离必须为 0（唯一匹配），次优解距离 > 0。
    let zero_distance: Vec<_> = with_items
        .iter()
        .filter(|region| {
            region
                .binding
                .as_ref()
                .is_some_and(|binding| binding.signature_distance == 0)
        })
        .collect();
    assert_eq!(zero_distance.len(), 9, "9 个主区域应全部距离 0");
    for region in zero_distance {
        let runner_up = region
            .binding
            .as_ref()
            .and_then(|binding| binding.runner_up_distance);
        assert!(
            runner_up.is_some_and(|value| value > 0),
            "{} 次优解不唯一",
            region.region_hash
        );
    }
}

#[test]
fn real_catalog_story_subcategories_match_region_counters() {
    let catalog = load();
    let count = |slug: &str, subcategory: &str| {
        catalog
            .items
            .values()
            .filter(|item| {
                item.region_slug.as_deref() == Some(slug)
                    && item.subcategory.as_deref() == Some(subcategory)
                    && !item.is_dlc
            })
            .count()
    };
    // 与存档区域计数器的来源目标逐项一致（见 research/s022/README.md 的对账表）。
    assert_eq!(
        (
            count("paititi", "relics"),
            count("paititi", "documents"),
            count("paititi", "murals")
        ),
        (28, 42, 23)
    );
    assert_eq!(
        (
            count("mission_of_san_juan", "relics"),
            count("mission_of_san_juan", "documents"),
            count("mission_of_san_juan", "murals")
        ),
        (9, 27, 4)
    );
    assert_eq!(count("kuwaq_yaku", "documents"), 18);
    assert_eq!(count("peruvian_jungle", "relics"), 9);
}

#[test]
fn plan_groups_by_region_and_marks_collected_status() {
    let catalog = load();
    // 帕依提提里挑三条真实故事条目：两条已观察、一条未观察。
    let paititi_relic = catalog
        .items
        .iter()
        .find(|(_, item)| {
            item.region_slug.as_deref() == Some("paititi")
                && item.subcategory.as_deref() == Some("relics")
                && !item.is_dlc
        })
        .map(|(id, _)| id.clone())
        .expect("帕依提提遗物");
    let missing_relic = catalog
        .items
        .iter()
        .find(|(id, item)| {
            item.region_slug.as_deref() == Some("paititi")
                && item.subcategory.as_deref() == Some("relics")
                && !item.is_dlc
                && **id != paititi_relic
        })
        .map(|(id, _)| id.clone())
        .expect("另一条帕依提提遗物");

    let result = plan(&catalog, &observed(&[paititi_relic.as_str()]));
    let paititi = result
        .regions
        .iter()
        .find(|region| region.slug == "paititi")
        .expect("帕依提提区域");
    assert_eq!(paititi.kind, RegionKind::Main);
    assert_eq!(paititi.name_zh.as_deref(), Some("帕依提提"));
    assert_eq!(paititi.total, 159);
    assert_eq!(paititi.collected, 1);

    let relics = paititi
        .groups
        .iter()
        .find(|group| group.subcategory.as_deref() == Some("relics"))
        .expect("遗物分组");
    assert_eq!((relics.collected, relics.total), (1, 28));

    let collected = relics
        .items
        .iter()
        .find(|item| item.id == paititi_relic)
        .expect("已收集条目");
    assert_eq!(collected.status, ItemStatus::Collected);
    assert!(collected.location_hint.contains("帕依提提"));
    assert!(collected.location_hint.contains("未校准"));

    let not_collected = relics
        .items
        .iter()
        .find(|item| item.id == missing_relic)
        .expect("未收集条目");
    assert_eq!(not_collected.status, ItemStatus::NotCollected);
    assert!(!not_collected.acquisition_text.is_empty());
}

#[test]
fn special_regions_are_separate_from_main_totals() {
    let catalog = load();
    let result = plan(&catalog, &observed(&[]));
    let main_total: usize = result
        .regions
        .iter()
        .filter(|region| region.kind == RegionKind::Main)
        .map(|region| region.total)
        .sum();
    let special_total: usize = result
        .regions
        .iter()
        .filter(|region| region.kind == RegionKind::Special)
        .map(|region| region.total)
        .sum();
    assert_eq!(result.main_total, main_total);
    assert_eq!(result.main_total + special_total, 470);

    for slug in ["_challenge_tombs", "_special", "croft_manor"] {
        let region = result
            .regions
            .iter()
            .find(|region| region.slug == slug)
            .unwrap_or_else(|| panic!("特殊区域 {slug}"));
        assert_eq!(region.kind, RegionKind::Special);
    }
}

#[test]
fn every_item_has_acquisition_text_and_item_note_wins() {
    let catalog = load();
    let mut with_note = catalog.clone();
    let target = with_note
        .items
        .keys()
        .next()
        .cloned()
        .expect("至少一条条目");
    if let Some(item) = with_note.items.get_mut(&target) {
        item.acquisition_note = Some("人工逐项注记".to_string());
    }
    let result = plan(&with_note, &observed(&[]));
    let all_items: Vec<_> = result
        .regions
        .iter()
        .flat_map(|region| region.groups.iter())
        .flat_map(|group| group.items.iter())
        .collect();
    assert_eq!(all_items.len(), catalog.items.len());
    assert!(
        all_items
            .iter()
            .all(|item| !item.acquisition_text.is_empty())
    );
    let noted = all_items
        .iter()
        .find(|item| item.id == target)
        .expect("被注记的条目");
    assert_eq!(noted.acquisition_source, "item_note");
    assert_eq!(noted.acquisition_text, "人工逐项注记");
}

#[test]
fn observed_not_in_catalog_is_listed_separately() {
    let catalog = load();
    let result = plan(&catalog, &observed(&["0xDEADBEEF", "0x00000001", "bad"]));
    assert_eq!(
        result.observed_not_in_catalog,
        vec![
            "0x00000001".to_string(),
            "0xDEADBEEF".to_string(),
            "BAD".to_string()
        ]
    );
}

#[test]
fn plan_is_deterministic() {
    let catalog = load();
    let ids = observed(&["0xF97006F6", "0x0F28B6EC"]);
    let first = plan(&catalog, &ids);
    let second = plan(&catalog, &ids);
    assert_eq!(first, second);
}
