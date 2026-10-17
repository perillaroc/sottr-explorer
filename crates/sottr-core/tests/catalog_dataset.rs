//! 全收集目录（SX-021，阶段 B）：真实数据加载与非法输入拒绝。
//!
//! 真实数据断言与 `research/s020/README.md` 的对账结论一致；任何数字变化都必须
//! 同时更新两处，否则测试失败。

use sottr_core::catalog::{
    CODE_CATALOG_ID_INVALID, CODE_CATALOG_REQUIRED_CONFLICT, CODE_CATALOG_SHARED_ID_INVALID,
    CODE_CATALOG_TOTALS_MISMATCH, Catalog,
};
use sottr_core::metadata::MetadataLimits;

const CATALOG: &str = include_str!("../../../data/catalog/collectibles_catalog.json");

fn load() -> Catalog {
    Catalog::from_json(CATALOG.as_bytes(), &MetadataLimits::default()).expect("目录应可加载")
}

#[test]
fn real_catalog_totals_match_reconciliation() {
    let catalog = load();
    let totals = catalog.totals;
    assert_eq!(catalog.version(), "1.0.0");
    assert_eq!(
        (
            totals.items,
            totals.story,
            totals.nonstory,
            totals.required_for_100,
            totals.dlc
        ),
        (470, 302, 168, 352, 42)
    );
    assert_eq!(catalog.required_item_count(), 352);
    assert_eq!(catalog.categories.len(), 12);
}

#[test]
fn story_required_is_253_and_prologue_is_extra() {
    let catalog = load();
    let story_required = catalog
        .items
        .values()
        .filter(|item| {
            item.kind == sottr_core::catalog::ItemKind::Story && item.required_for_100 == Some(true)
        })
        .count();
    assert_eq!(story_required, 253);

    let prologue = catalog
        .category("croft_manor_extra")
        .expect("Prologue 类别");
    assert_eq!(prologue.required_for_100, Some(false));
    assert_eq!(prologue.count, 7);

    let sample = catalog.items.get("0xF97006F6").expect("故事条目样例");
    assert_eq!(sample.collection.as_deref(), Some("Path_of_the_Stars"));
    assert_eq!(sample.required_for_100, Some(true));
}

#[test]
fn unresolved_and_optional_categories_are_not_required() {
    let catalog = load();
    let tombs = catalog.category("challenge_tombs").expect("挑战古墓类别");
    assert_eq!(tombs.required_for_100, None);
    assert_eq!(tombs.required_count, 0);

    let shared = catalog.items.get("0xC9D62DEE").expect("共用 ID 条目");
    assert_eq!(shared.shared_id, Some(true));
    assert_eq!(shared.shared_names.as_ref().map(Vec::len), Some(5));
    assert_ne!(shared.required_for_100, Some(true));

    for id in [
        "explorer_backpacks",
        "archivist_maps",
        "strongboxes",
        "crypt_entrances",
        "gym_training",
        "test_items",
    ] {
        let category = catalog.category(id).expect(id);
        assert_eq!(category.required_for_100, Some(false), "{id}");
        assert_eq!(category.required_count, 0, "{id}");
    }
}

const SYNTHETIC: &str = r#"{
  "catalog_schema_version": "1.0.0",
  "catalog_version": "1.0.0",
  "generated_from": {},
  "totals": {"items": 1, "story": 0, "nonstory": 1, "required_for_100": 1, "dlc": 0},
  "categories": [
    {"id": "monoliths", "name": "M", "count": 1, "required_for_100": true, "required_count": 1}
  ],
  "collections": [],
  "regions": [],
  "items": {
    "0x00000001": {
      "kind": "nonstory", "name": "X", "is_dlc": false,
      "required_for_100": true, "category": "monoliths"
    }
  }
}"#;

fn expect_code(json: &str, code: &str) {
    let error =
        Catalog::from_json(json.as_bytes(), &MetadataLimits::default()).expect_err("应被拒绝");
    assert_eq!(error.code(), code);
}

#[test]
fn rejects_totals_mismatch() {
    expect_code(
        &SYNTHETIC.replace("\"items\": 1", "\"items\": 2"),
        CODE_CATALOG_TOTALS_MISMATCH,
    );
}

#[test]
fn rejects_bad_item_key() {
    expect_code(
        &SYNTHETIC.replace("\"0x00000001\"", "\"0x0000001\""),
        CODE_CATALOG_ID_INVALID,
    );
}

#[test]
fn rejects_required_item_in_unresolved_category() {
    expect_code(
        &SYNTHETIC.replace(
            "\"required_for_100\": true, \"required_count\": 1",
            "\"required_for_100\": null, \"required_count\": 1",
        ),
        CODE_CATALOG_REQUIRED_CONFLICT,
    );
}

#[test]
fn rejects_shared_id_without_names() {
    expect_code(
        &SYNTHETIC.replace(
            "\"category\": \"monoliths\"",
            "\"category\": \"monoliths\", \"shared_id\": true",
        ),
        CODE_CATALOG_SHARED_ID_INVALID,
    );
}
