//! 全收集缺失清单（SX-021 阶段 C）：规则与保护性断言（合成目录，无真实数据）。

use std::collections::BTreeSet;

use sottr_core::catalog::Catalog;
use sottr_core::metadata::MetadataLimits;
use sottr_core::missing::{REASON_OPTIONAL, REASON_UNRESOLVED, compute_missing};

const CATALOG: &str = r#"{
  "catalog_schema_version": "2.0.0",
  "catalog_version": "2.0.0",
  "generated_from": {},
  "totals": {"items": 5, "story": 0, "nonstory": 5, "required_for_100": 2, "dlc": 1, "with_position": 0, "with_region": 0},
  "acquisition_rules": {
    "monolith": {"text_zh": "解读石碑", "text_en": "Read the monolith", "source": "curated"},
    "explorer_backpack": {"text_zh": "拾取背包", "text_en": "Pick up the backpack", "source": "curated"},
    "challenge_tomb": {"text_zh": "完成古墓", "text_en": "Finish the tomb", "source": "curated"}
  },
  "categories": [
    {"id": "monoliths", "name": "石碑", "count": 3, "required_for_100": true, "required_count": 2, "acquisition_rule_key": "monolith"},
    {"id": "explorer_backpacks", "name": "背包", "count": 1, "required_for_100": false, "required_count": 0, "acquisition_rule_key": "explorer_backpack"},
    {"id": "challenge_tombs", "name": "挑战古墓", "count": 1, "required_for_100": null, "required_count": 0, "acquisition_rule_key": "challenge_tomb", "note": "未闭合"}
  ],
  "collections": [],
  "units": {},
  "regions": [],
  "items": {
    "0x00000001": {"kind": "nonstory", "name": "One", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": true, "region_hash": null, "region_slug": null, "category": "monoliths", "subcategory": null, "unit": null, "position": null, "acquisition_note": null},
    "0x00000002": {"kind": "nonstory", "name": "Two", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": true, "region_hash": null, "region_slug": null, "category": "monoliths", "subcategory": null, "unit": null, "position": null, "acquisition_note": null, "shared_id": true, "shared_names": ["Two", "TwoAlt"]},
    "0x00000003": {"kind": "nonstory", "name": "Three", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": false, "region_hash": null, "region_slug": null, "category": "explorer_backpacks", "subcategory": null, "unit": null, "position": null, "acquisition_note": null},
    "0x00000004": {"kind": "nonstory", "name": "Tomb", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": null, "region_hash": null, "region_slug": null, "category": "challenge_tombs", "subcategory": null, "unit": null, "position": null, "acquisition_note": null},
    "0x00000005": {"kind": "nonstory", "name": "DlcRelic", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": true, "required_for_100": false, "region_hash": null, "region_slug": null, "category": "monoliths", "subcategory": null, "unit": null, "position": null, "acquisition_note": null}
  }
}"#;

fn load() -> Catalog {
    Catalog::from_json(CATALOG.as_bytes(), &MetadataLimits::default()).expect("合成目录应可加载")
}

fn observed(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).to_string()).collect()
}

#[test]
fn required_category_lists_missing_with_shared_names() {
    let report = compute_missing(&load(), &observed(&["0x00000001", "0xDEADBEEF", "bad"]));
    assert_eq!(report.catalog_version, "2.0.0");
    assert_eq!(report.observed_items, 1);
    assert!(
        report
            .observed_not_in_catalog
            .contains(&"0xDEADBEEF".to_string())
    );
    assert!(report.observed_not_in_catalog.contains(&"BAD".to_string()));

    let monoliths = report
        .categories
        .iter()
        .find(|category| category.id == "monoliths")
        .expect("monoliths");
    assert_eq!(
        (monoliths.total, monoliths.observed, monoliths.missing),
        (2, 1, Some(1)),
        "必收类别计数与清单只含必收子集（DLC 条目不混入）"
    );
    let items = monoliths.items.as_ref().expect("必收类别应有清单");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "0x00000002");
    assert!(
        items.iter().all(|item| item.id != "0x00000005"),
        "DLC 条目不得出现在缺失清单"
    );
    assert_eq!(
        items[0].shared_names.as_ref().map(Vec::len),
        Some(2),
        "共用 ID 条目必须携带全部名称，不得拆分假点"
    );
}

#[test]
fn optional_category_counts_but_no_list() {
    let report = compute_missing(&load(), &observed(&["0x00000003"]));
    let backpacks = report
        .categories
        .iter()
        .find(|category| category.id == "explorer_backpacks")
        .expect("backpacks");
    assert_eq!(backpacks.required_for_100, Some(false));
    assert_eq!(
        (backpacks.total, backpacks.observed, backpacks.missing),
        (1, 1, Some(0))
    );
    assert!(backpacks.items.is_none());
    assert_eq!(backpacks.reason.as_deref(), Some(REASON_OPTIONAL));
}

#[test]
fn unresolved_category_has_no_missing_conclusion() {
    let report = compute_missing(&load(), &observed(&[]));
    let tombs = report
        .categories
        .iter()
        .find(|category| category.id == "challenge_tombs")
        .expect("challenge_tombs");
    assert_eq!(tombs.required_for_100, None);
    assert_eq!((tombs.total, tombs.observed), (1, 0));
    assert_eq!(tombs.missing, None, "未闭合类别不得给出缺失数");
    assert!(tombs.items.is_none());
    assert_eq!(tombs.reason.as_deref(), Some(REASON_UNRESOLVED));
}
