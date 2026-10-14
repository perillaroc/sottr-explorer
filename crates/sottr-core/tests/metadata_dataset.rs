//! SX-008 元数据集成测试：真实数据集加载、数量与交叉一致性、拒绝路径与无兜底。
//!
//! 真实数据文件通过 `include_str!` 嵌入（`data/metadata/`），内容哈希与 manifest
//! 对照；拒绝路径使用合成 JSON，不依赖外部样本。

use std::collections::BTreeSet;

use serde_json::json;

use sottr_core::hash::sha256_hex;
use sottr_core::metadata::{
    CODE_COUNTER_MAPPING_INVALID, CODE_DUPLICATE_KEY, CODE_ENTRY_CONFLICT, CODE_INVALID_URL,
    CODE_INVALID_VALUE, CODE_KEY_MISMATCH, CODE_LIMIT_EXCEEDED, CODE_NON_STANDARD_NUMBER,
    CODE_TOO_LARGE, CODE_UNRESOLVED_REFERENCE, CODE_VERSION_CONFLICT, LoadedMetadataFile,
    MetadataBundle, MetadataFileInput, MetadataLimits, load_dataset,
};
use sottr_core::report::{Context, ContextSource, InternalId, Mode, SchemaVersion};

const COLLECTIBLES: &str = include_str!("../../../data/metadata/collectibles.json");
const REGION_COUNTERS: &str = include_str!("../../../data/metadata/region_counters.json");
const REFERENCE_SETS: &str = include_str!("../../../data/metadata/reference_sets.json");
const MANIFEST: &str = include_str!("../../../data/metadata/manifest.json");

const REFERENCE_SET_ID: &str = "rs:stella-100pct-common";

fn real_bundle() -> MetadataBundle {
    let files = [
        ("collectibles.json", COLLECTIBLES),
        ("region_counters.json", REGION_COUNTERS),
        ("reference_sets.json", REFERENCE_SETS),
    ];
    let inputs: Vec<MetadataFileInput<'_>> = files
        .iter()
        .map(|(file_id, text)| MetadataFileInput {
            file_id: (*file_id).to_string(),
            bytes: text.as_bytes(),
        })
        .collect();
    MetadataBundle::from_files(&inputs, &MetadataLimits::default()).expect("真实元数据必须可加载")
}

fn minimal_document(version: &str) -> serde_json::Value {
    json!({
        "schema_version": "1.0.0",
        "metadata_version": version,
        "namespace": "sottr",
        "items": {},
        "categories": {},
        "regions": {},
        "guide_groups": {},
        "reference_sets": {},
        "region_counters": {},
        "sources": {}
    })
}

fn item_json(id: &str, alias: &str) -> serde_json::Value {
    json!({
        "internal_id": id,
        "aliases": [alias],
        "display_name": null,
        "display_language": null,
        "name_source": null,
        "name_kind": "unknown",
        "category_id": null,
        "region_id": null,
        "applicability": {
            "game": "sottr",
            "platform": null,
            "mode": null,
            "build_range": null,
            "dlc": null,
            "notes": []
        },
        "location_mappings": [],
        "source_refs": [],
        "review": null
    })
}

fn load_text(text: &str) -> Result<LoadedMetadataFile, sottr_core::metadata::MetadataError> {
    load_dataset(text.as_bytes(), &MetadataLimits::default())
}

fn code_of(text: &str) -> &'static str {
    load_text(text).expect_err("应当被拒绝").code()
}

fn bytes_of(value: &serde_json::Value) -> Vec<u8> {
    value.to_string().into_bytes()
}

#[test]
fn real_bundle_loads_and_records_content_hashes() {
    let bundle = real_bundle();
    assert_eq!(bundle.metadata_version(), Some("0.1.0"));
    assert_eq!(bundle.namespace(), Some("sottr"));
    assert_eq!(bundle.files().len(), 3);

    let manifest: serde_json::Value = serde_json::from_str(MANIFEST).expect("manifest 可解析");
    let manifest_files = manifest["files"].as_array().expect("manifest files");
    for record in bundle.files() {
        let expected = manifest_files
            .iter()
            .find(|entry| entry["file_id"] == record.file_id.as_str())
            .unwrap_or_else(|| panic!("manifest 缺少 {}", record.file_id));
        assert_eq!(record.sha256_hex, expected["sha256"].as_str().unwrap());
        assert_eq!(record.bytes, expected["bytes"].as_u64().unwrap());
    }

    // 嵌入内容与 manifest 哈希一致（漂移检测）。
    for (file_id, text) in [
        ("collectibles.json", COLLECTIBLES),
        ("region_counters.json", REGION_COUNTERS),
        ("reference_sets.json", REFERENCE_SETS),
    ] {
        let expected = manifest_files
            .iter()
            .find(|entry| entry["file_id"] == file_id)
            .unwrap();
        assert_eq!(
            sha256_hex(text.as_bytes()),
            expected["sha256"].as_str().unwrap()
        );
    }
}

#[test]
fn real_dataset_keeps_all_dictionary_names() {
    let bundle = real_bundle();
    let dataset = bundle.dataset().expect("数据集存在");
    assert_eq!(dataset.items.len(), 866, "866 个不同 ID");
    let alias_total: usize = dataset.items.values().map(|item| item.aliases.len()).sum();
    assert_eq!(alias_total, 871, "871 个名称");
    assert_eq!(
        dataset.items["0xC9D62DEE"].aliases.len(),
        5,
        "同 ID 多名称保留数组"
    );
    assert_eq!(dataset.items["0x98564B9D"].aliases.len(), 2);

    // 同名不同 ID（Yaaxil_01）保持两个独立身份，不因别名合并。
    let yaaxil: Vec<&str> = dataset
        .items
        .iter()
        .filter(|(_, item)| item.aliases.iter().any(|alias| alias == "Yaaxil_01"))
        .map(|(key, _)| key.as_str())
        .collect();
    assert_eq!(yaaxil, vec!["0x72A130AC", "0x94549BD7"]);

    // 名称字段不作为身份：未命名条目的显示名称一律 null。
    assert!(
        dataset
            .items
            .values()
            .all(|item| item.display_name.is_none() && item.name_source.is_none())
    );
}

#[test]
fn reference_set_matches_bounded_common_sets() {
    let bundle = real_bundle();
    let dataset = bundle.dataset().unwrap();
    let reference = dataset
        .reference_sets
        .get(REFERENCE_SET_ID)
        .expect("参考集合存在");
    let counts: Vec<(&str, usize)> = reference
        .category_ids
        .iter()
        .map(|category_id| {
            (
                category_id.as_str(),
                reference.reference_ids[category_id].len(),
            )
        })
        .collect();
    assert_eq!(
        counts,
        vec![
            ("crypt_treasures", 10),
            ("monoliths", 12),
            ("rare_resource_caches", 12),
            ("survival_caches", 65),
        ]
    );
    assert_eq!(
        reference.source_save_hashes,
        vec![
            "e57947b199677baecb312134aa23b01c6260e42cd8c38ae113ce1d49912cd813".to_string(),
            "6d9b468e51317025bdabb8ffeb91e789d4461547c2edb3786de5326d6c644ba2".to_string(),
        ]
    );
    // 全部参考 ID 在条目表中可查（用于名称回退与分类）。
    for ids in reference.reference_ids.values() {
        for id in ids {
            assert!(bundle.item(*id).is_some(), "参考 ID {id} 缺少条目观测");
        }
    }
    assert!(reference.reference_ids["survival_caches"].contains(&InternalId::new(0x0F28_B6EC)));
    assert!(reference.reference_ids["crypt_treasures"].contains(&InternalId::new(0xBA3B_0D06)));
}

#[test]
fn counters_cover_43_sparse_mappings_with_offset_identity() {
    let bundle = real_bundle();
    let dataset = bundle.dataset().unwrap();
    assert_eq!(dataset.region_counters.len(), 43);
    let specs = bundle.region_counter_specs();
    assert_eq!(specs.len(), 43);

    let san_juan = specs
        .iter()
        .find(|spec| spec.counter_key == "counter:mission_of_san_juan:survival_caches")
        .expect("圣胡安贮藏点计数");
    assert_eq!(san_juan.serialized_offset, 5336);
    assert_eq!(san_juan.source_total, 8);
    assert_eq!(san_juan.region_id.as_deref(), Some("mission_of_san_juan"));
    assert_eq!(san_juan.category_id, "survival_caches");

    let paititi = specs
        .iter()
        .find(|spec| spec.counter_key == "counter:paititi:survival_caches")
        .expect("帕伊提提贮藏点计数");
    assert_eq!(paititi.serialized_offset, 4608);
    assert_eq!(paititi.source_total, 34);

    // 偏移恒等式：0x10BC + 行号 * 104 + 相对偏移。
    for entry in dataset.region_counters.values() {
        let expected = 0x10BCu64
            + u64::from(entry.serialized_row_index) * 104
            + u64::from(entry.serialized_relative_offset);
        assert_eq!(entry.serialized_offset, expected, "{}", entry.counter_key);
    }
}

#[test]
fn counter_scope_totals_cross_support_reference_counts() {
    let bundle = real_bundle();
    let dataset = bundle.dataset().unwrap();
    let totals = bundle.counter_scope_totals(&[]);
    // 无读数时作用域合计仍给出来源目标；读数缺失记入 missing。
    let caches = totals
        .iter()
        .find(|total| total.category_ids == vec!["rare_resource_caches", "survival_caches"])
        .expect("贮藏点作用域");
    assert_eq!(caches.source_target_total, 77);
    assert_eq!(caches.observed_total, None);
    assert_eq!(caches.missing_counter_keys.len(), 7);
    let monoliths = totals
        .iter()
        .find(|total| total.category_ids == vec!["monoliths"])
        .unwrap();
    assert_eq!(monoliths.source_target_total, 12);
    let crypts = totals
        .iter()
        .find(|total| total.category_ids == vec!["crypt_treasures"])
        .unwrap();
    assert_eq!(crypts.source_target_total, 10);

    // 作用域来源目标合计与参考集合 ID 数一致（43 项计数与 65/12/12/10 的交叉）。
    for total in &totals {
        if total.category_ids.is_empty() {
            continue;
        }
        let reference_count =
            sottr_core::metadata::reference_id_count(dataset, &total.category_ids);
        assert_eq!(
            total.source_target_total, reference_count as u64,
            "{:?} 作用域与参考 ID 数不一致",
            total.category_ids
        );
    }
}

#[test]
fn classification_uses_internal_ids_only() {
    let bundle = real_bundle();
    let dataset = bundle.dataset().unwrap();
    let reference = dataset.reference_sets.get(REFERENCE_SET_ID).unwrap();
    let ids: Vec<InternalId> = reference
        .reference_ids
        .values()
        .flat_map(|ids| ids.iter().copied())
        .collect();
    let classified = bundle.classify_ids(&ids);
    assert_eq!(classified["survival_caches"].len(), 65);
    assert_eq!(classified["rare_resource_caches"].len(), 12);
    assert_eq!(classified["monoliths"].len(), 12);
    assert_eq!(classified["crypt_treasures"].len(), 10);

    // 未分类 ID 不进入类别集合；分类不因别名或显示名改变。
    assert!(
        bundle
            .classify_ids(&[InternalId::new(0xC9D6_2DEE)])
            .is_empty()
    );
    let yaaxil_a = bundle.item(InternalId::new(0x72A1_30AC)).unwrap();
    let yaaxil_b = bundle.item(InternalId::new(0x9454_9BD7)).unwrap();
    assert_ne!(yaaxil_a.internal_id, yaaxil_b.internal_id);
    assert!(yaaxil_a.aliases.contains(&"Yaaxil_01".to_string()));
    assert!(yaaxil_b.aliases.contains(&"Yaaxil_01".to_string()));
}

#[test]
fn reference_view_applicability_is_context_driven() {
    let bundle = real_bundle();
    let campaign = Context {
        platform: Some("windows_steam".to_string()),
        mode: Mode::Campaign,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
        limitations: vec![],
    };
    let view = bundle.reference_view(REFERENCE_SET_ID, &campaign).unwrap();
    assert!(view.applicable, "未知构建不关闭参考适用性");
    let limits = view.applicability_limits.join(" ");
    assert!(limits.contains("构建号未知"), "{limits}");
    assert!(limits.contains("DLC 适用范围未逐项验证"), "{limits}");
    assert_eq!(view.categories.len(), 4);
    assert_eq!(view.version, "1.0.0");

    let wrong_platform = Context {
        platform: Some("ps4".to_string()),
        ..campaign.clone()
    };
    let view = bundle
        .reference_view(REFERENCE_SET_ID, &wrong_platform)
        .unwrap();
    assert!(!view.applicable);
    assert!(
        view.applicability_limits
            .join(" ")
            .contains("平台限定为 windows_steam")
    );

    let unknown_mode = Context {
        mode: Mode::Unknown,
        ..campaign
    };
    let view = bundle
        .reference_view(REFERENCE_SET_ID, &unknown_mode)
        .unwrap();
    assert!(!view.applicable);
}

#[test]
fn dataset_contains_no_fake_points_or_exact_mappings() {
    let bundle = real_bundle();
    let dataset = bundle.dataset().unwrap();
    let mut mappings = 0usize;
    for item in dataset.items.values() {
        for mapping in &item.location_mappings {
            mappings += 1;
            assert!(mapping.coordinates.is_none(), "不产生坐标");
            assert!(mapping.map_transform.is_none(), "不产生地图变换");
            assert!(mapping.guide_item.is_none(), "不产生精确条目映射");
            assert!(matches!(
                mapping.mapping_kind,
                sottr_core::report::MappingKind::GuideGroupMembership
                    | sottr_core::report::MappingKind::RegionClue
            ));
        }
    }
    assert_eq!(
        mappings, 12,
        "仅有 10 个圣胡安组成员 + 2 个帕伊提提区域线索"
    );
    for group in dataset.guide_groups.values() {
        assert_eq!(group.target_count, Some(group.member_ids.len() as u32));
    }
}

#[test]
fn unknown_or_empty_metadata_has_no_fallback() {
    let bundle = MetadataBundle::from_files(&[], &MetadataLimits::default()).unwrap();
    assert_eq!(bundle.metadata_version(), None);
    assert!(bundle.dataset().is_none());
    assert!(
        bundle.region_counter_specs().is_empty(),
        "没有硬编码计数映射"
    );
    assert!(bundle.item(InternalId::new(0x439B_19A6)).is_none());
    assert!(bundle.guide_group("stella:san-juan-caches").is_none());
    assert!(
        bundle
            .reference_view(
                REFERENCE_SET_ID,
                &Context {
                    platform: None,
                    mode: Mode::Campaign,
                    build: None,
                    dlc: None,
                    source: ContextSource::Unknown,
                    limitations: vec![],
                }
            )
            .is_none()
    );
    assert!(
        bundle
            .classify_ids(&[InternalId::new(0x439B_19A6)])
            .is_empty()
    );
    assert!(bundle.counter_scope_totals(&[]).is_empty());
}

#[test]
fn loader_rejects_duplicate_keys_and_non_standard_numbers() {
    let duplicate = r#"{
        "schema_version": "1.0.0",
        "metadata_version": "0.1.0",
        "metadata_version": "0.2.0",
        "namespace": "sottr",
        "items": {}, "categories": {}, "regions": {}, "guide_groups": {},
        "reference_sets": {}, "region_counters": {}, "sources": {}
    }"#;
    assert_eq!(code_of(duplicate), CODE_DUPLICATE_KEY);

    let nan = r#"{
        "schema_version": "1.0.0",
        "metadata_version": NaN,
        "namespace": "sottr",
        "items": {}, "categories": {}, "regions": {}, "guide_groups": {},
        "reference_sets": {}, "region_counters": {}, "sources": {}
    }"#;
    assert_eq!(code_of(nan), CODE_NON_STANDARD_NUMBER);

    let infinity = nan.replace("NaN", "Infinity");
    assert_eq!(code_of(&infinity), CODE_NON_STANDARD_NUMBER);
}

#[test]
fn loader_enforces_size_string_and_array_limits() {
    let minimal = minimal_document("0.1.0").to_string();
    let tiny_file = MetadataLimits {
        max_file_bytes: 16,
        ..MetadataLimits::default()
    };
    let error = load_dataset(minimal.as_bytes(), &tiny_file).unwrap_err();
    assert_eq!(error.code(), CODE_TOO_LARGE);

    let tiny_string = MetadataLimits {
        max_string_bytes: 4,
        ..MetadataLimits::default()
    };
    let error = load_dataset(minimal.as_bytes(), &tiny_string).unwrap_err();
    assert_eq!(error.code(), CODE_LIMIT_EXCEEDED);

    let mut document = minimal_document("0.1.0");
    document["items"]["0x11111111"] = item_json("0x11111111", "alias-a");
    document["items"]["0x11111111"]["aliases"] = serde_json::Value::Array(
        (0..3)
            .map(|index| json!(format!("alias-{index}")))
            .collect(),
    );
    let tiny_array = MetadataLimits {
        max_array_items: 2,
        ..MetadataLimits::default()
    };
    let error = load_dataset(&bytes_of(&document), &tiny_array).unwrap_err();
    assert_eq!(error.code(), CODE_LIMIT_EXCEEDED);
}

#[test]
fn loader_rejects_out_of_range_ids_and_key_mismatch() {
    let mut document = minimal_document("0.1.0");
    document["items"]["0x123456789"] = item_json("0x123456789", "alias");
    let error = load_dataset(&bytes_of(&document), &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_INVALID_VALUE, "9 位 ID 越界");

    let mut document = minimal_document("0.1.0");
    document["items"]["0x11111111"] = item_json("0x22222222", "alias");
    let error = load_dataset(&bytes_of(&document), &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_KEY_MISMATCH);

    let mut document = minimal_document("0.1.0");
    document["items"]["0x11111111"] = item_json("0x1111111A", "alias");
    let error = load_dataset(&bytes_of(&document), &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_KEY_MISMATCH, "键与值必须逐字节一致");
}

#[test]
fn loader_rejects_invalid_url_protocols() {
    for url in [
        "javascript:alert(1)",
        "ftp://example.invalid/file",
        "file:///C:/saves",
        "https://",
    ] {
        let mut document = minimal_document("0.1.0");
        document["sources"]["src:bad"] = json!({
            "url": url,
            "fixed_revision": null,
            "accessed": null,
            "license": null,
            "redistributable": "no",
            "method": null,
            "review": null
        });
        let error = load_dataset(&bytes_of(&document), &MetadataLimits::default()).unwrap_err();
        assert_eq!(error.code(), CODE_INVALID_URL, "{url}");
    }

    let mut document = minimal_document("0.1.0");
    document["sources"]["src:ok"] = json!({
        "url": "https://example.invalid/a",
        "fixed_revision": null,
        "accessed": null,
        "license": null,
        "redistributable": "pending",
        "method": null,
        "review": null
    });
    assert!(load_dataset(&bytes_of(&document), &MetadataLimits::default()).is_ok());
}

#[test]
fn loader_rejects_counter_mapping_mismatch() {
    let mut document = minimal_document("0.1.0");
    document["region_counters"]["counter:cozumel:relics"] = json!({
        "counter_key": "counter:cozumel:relics",
        "region_id": "cozumel",
        "category_id": "relics",
        "serialized_offset": 4913,
        "serialized_row_index": 6,
        "serialized_relative_offset": 4,
        "source_total": 2,
        "counts_toward": [],
        "source_refs": []
    });
    let error = load_dataset(&bytes_of(&document), &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_COUNTER_MAPPING_INVALID);
}

#[test]
fn bundle_rejects_version_and_entry_conflicts() {
    let first = minimal_document("0.1.0").to_string();
    let second = minimal_document("0.2.0").to_string();
    let inputs = [
        MetadataFileInput {
            file_id: "a.json".to_string(),
            bytes: first.as_bytes(),
        },
        MetadataFileInput {
            file_id: "b.json".to_string(),
            bytes: second.as_bytes(),
        },
    ];
    let error = MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_VERSION_CONFLICT);

    let mut document_a = minimal_document("0.1.0");
    document_a["items"]["0x11111111"] = item_json("0x11111111", "alias-a");
    let mut document_b = minimal_document("0.1.0");
    document_b["items"]["0x11111111"] = item_json("0x11111111", "alias-b");
    let text_a = document_a.to_string();
    let text_b = document_b.to_string();
    let inputs = [
        MetadataFileInput {
            file_id: "a.json".to_string(),
            bytes: text_a.as_bytes(),
        },
        MetadataFileInput {
            file_id: "b.json".to_string(),
            bytes: text_b.as_bytes(),
        },
    ];
    let error = MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_ENTRY_CONFLICT);

    // 相同条目跨文件去重（真实数据三个文件共享同一份 sources 表）。
    let inputs = [
        MetadataFileInput {
            file_id: "a.json".to_string(),
            bytes: text_a.as_bytes(),
        },
        MetadataFileInput {
            file_id: "b.json".to_string(),
            bytes: text_a.as_bytes(),
        },
    ];
    let bundle = MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap();
    assert_eq!(bundle.dataset().unwrap().items.len(), 1);
}

#[test]
fn bundle_rejects_unresolved_references() {
    let mut document = minimal_document("0.1.0");
    document["items"]["0x11111111"] = item_json("0x11111111", "alias");
    document["items"]["0x11111111"]["category_id"] = json!("ghost_category");
    let text = document.to_string();
    let inputs = [MetadataFileInput {
        file_id: "a.json".to_string(),
        bytes: text.as_bytes(),
    }];
    let error = MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_UNRESOLVED_REFERENCE);

    let mut document = minimal_document("0.1.0");
    document["items"]["0x11111111"] = item_json("0x11111111", "alias");
    document["items"]["0x11111111"]["source_refs"] = json!(["src:missing"]);
    let text = document.to_string();
    let inputs = [MetadataFileInput {
        file_id: "a.json".to_string(),
        bytes: text.as_bytes(),
    }];
    let error = MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_UNRESOLVED_REFERENCE);
}

#[test]
fn schema_version_marker_is_fixed() {
    // 契约版本不匹配（"2"）在反序列化时被拒绝。
    let mut document = minimal_document("0.1.0");
    document["schema_version"] = json!("2");
    let error = load_dataset(&bytes_of(&document), &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_INVALID_VALUE);
    assert_eq!(SchemaVersion::VALUE, "1.0.0");

    // 空版本与空命名空间不是有效元数据。
    let empty_version = minimal_document("");
    let error = load_dataset(&bytes_of(&empty_version), &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_INVALID_VALUE);
    let mut empty_namespace = minimal_document("0.1.0");
    empty_namespace["namespace"] = json!("");
    let error = load_dataset(&bytes_of(&empty_namespace), &MetadataLimits::default()).unwrap_err();
    assert_eq!(error.code(), CODE_INVALID_VALUE);
}

#[test]
fn reference_id_set_identity_is_stable_across_serialization() {
    let bundle = real_bundle();
    let dataset = bundle.dataset().unwrap();
    let text = serde_json::to_string(dataset).unwrap();
    let reparsed: sottr_core::metadata::MetadataDataset = serde_json::from_str(&text).unwrap();
    assert_eq!(&reparsed, dataset);
    let unique: BTreeSet<InternalId> = dataset
        .reference_sets
        .get(REFERENCE_SET_ID)
        .unwrap()
        .reference_ids
        .values()
        .flat_map(|ids| ids.iter().copied())
        .collect();
    assert_eq!(unique.len(), 99);
}
