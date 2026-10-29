//! 真实地图基底层（P2-02）：真实数据加载与非法输入拒绝。
//!
//! 真实数据的断言与 `doc/2026-10-19-真实地图/真实地图方案.md` §5.4 一致；
//! 任何数字变化都必须同时更新两处，否则测试失败。

use std::collections::BTreeSet;

use sottr_core::basemap::{
    BasemapDataset, BasemapStatus, CODE_BASEMAP_ID_INVALID, CODE_BASEMAP_IMAGE_INVALID,
    CODE_BASEMAP_JSON_INVALID, CODE_BASEMAP_LIMIT, CODE_BASEMAP_PRECISION_INSUFFICIENT,
    CODE_BASEMAP_SOURCE_MISSING, CODE_BASEMAP_STATUS_INVALID, CODE_BASEMAP_TRANSFORM_INVALID,
    CODE_BASEMAP_UNIT_DUPLICATE, CODE_BASEMAP_VERSION_INVALID, CoordinateYAxis, ImageRootKind,
    TransformBasis, TransformKind,
};
use sottr_core::metadata::MetadataLimits;

const BASEMAPS: &str = include_str!("../../../data/map/basemaps.json");
const FIXTURE_VALID: &str = include_str!("../../../fixtures/contract/valid/basemap.valid.json");
const FIXTURE_MINIMAL: &str = include_str!("../../../fixtures/contract/valid/basemap.minimal.json");

fn load() -> BasemapDataset {
    BasemapDataset::from_json(BASEMAPS.as_bytes(), &MetadataLimits::default())
        .expect("基底层应可加载")
}

#[test]
fn real_dataset_shape() {
    let dataset = load();
    assert_eq!(dataset.schema_version, "1.0.0");
    assert_eq!(dataset.version(), "0.1.0");
    assert_eq!(dataset.basemaps.len(), 42);
    assert_eq!(dataset.sources.len(), 3);
    assert!(!dataset.images_bundled());
    assert_eq!(dataset.image_root.kind, ImageRootKind::External);

    let regions: BTreeSet<&str> = dataset
        .basemaps
        .values()
        .map(|basemap| basemap.region_id.as_str())
        .collect();
    assert_eq!(regions.len(), 10);
    assert!(regions.contains("unnamed"));
}

#[test]
fn every_real_transform_is_identity_scaled() {
    let dataset = load();
    for (key, basemap) in &dataset.basemaps {
        assert_eq!(basemap.transform.kind, TransformKind::Affine, "{key}");
        assert_eq!(basemap.transform_basis, TransformBasis::Analytic, "{key}");
        assert_eq!(
            basemap.transform.coordinate_y_axis,
            CoordinateYAxis::Down,
            "{key}"
        );
        let width = f64::from(basemap.image_width);
        let height = f64::from(basemap.image_height);
        assert_eq!(
            basemap.transform.matrix,
            vec![width, 0.0, 0.0, 0.0, height, 0.0],
            "{key}"
        );
        assert_eq!(
            basemap.to_pixel(0.0, 0.0),
            Some((0.0, 0.0)),
            "{key} 西北角"
        );
        assert_eq!(
            basemap.to_pixel(1.0, 1.0),
            Some((width, height)),
            "{key} 东南角"
        );
    }
}

#[test]
fn only_measured_units_are_calibrated() {
    let dataset = load();
    let mut calibrated: Vec<u32> = dataset
        .basemaps
        .values()
        .filter(|basemap| basemap.status == BasemapStatus::Calibrated)
        .flat_map(|basemap| basemap.unit_ids.clone())
        .collect();
    calibrated.sort_unstable();
    assert_eq!(calibrated, vec![318, 2287]);

    let village = dataset.for_unit(318).expect("San Juan 村庄");
    assert_eq!(village.image, "maps/sm_spanish_mission_village.webp");
    assert_eq!((village.image_width, village.image_height), (2048, 2048));
    assert_eq!(village.median_error_px, Some(3.54));
    assert_eq!(village.validation_samples, Some(38));

    assert!(dataset.for_unit(424242).is_none());
}

#[test]
fn contract_fixtures_load_in_rust_loader() {
    // schema 与加载器是两层独立校验；合法夹具必须同时通过两层。
    for (name, json) in [("basemap.valid", FIXTURE_VALID), ("basemap.minimal", FIXTURE_MINIMAL)] {
        BasemapDataset::from_json(json.as_bytes(), &MetadataLimits::default())
            .unwrap_or_else(|error| panic!("{name} 应通过 Rust 加载器：{error}"));
    }
}

// --- 非法输入 -------------------------------------------------------------

/// 一份最小合法数据集；测试用字符串替换制造各类坏数据。
const SYNTHETIC: &str = r#"{
  "schema_version": "1.0.0",
  "basemap_version": "0.1.0",
  "image_root": { "kind": "external", "path": "resource/map-processed" },
  "sources": {
    "src:test": { "license": "test", "redistributable": "pending", "method": "test" }
  },
  "basemaps": {
    "bm:test_region/1": {
      "basemap_id": "bm:test_region/1",
      "version": "0.1.0",
      "region_id": "test_region",
      "unit_ids": [1],
      "image": "maps/test.webp",
      "image_width": 1024,
      "image_height": 1024,
      "transform": { "kind": "affine", "matrix": [1024.0, 0.0, 0.0, 0.0, 1024.0, 0.0], "coordinate_y_axis": "down", "image_origin": "top_left" },
      "transform_basis": "analytic",
      "status": "unverified",
      "median_error_px": null,
      "p95_error_px": null,
      "max_error_px": null,
      "error_source": null,
      "validation_samples": 0,
      "source_refs": ["src:test"],
      "review": { "state": "draft" }
    }
  }
}"#;

fn expect_code(json: &str, code: &str) {
    let error = BasemapDataset::from_json(json.as_bytes(), &MetadataLimits::default())
        .expect_err("坏数据应被拒绝");
    assert_eq!(error.code(), code, "实际错误：{error}");
}

#[test]
fn accepts_synthetic() {
    let dataset = BasemapDataset::from_json(SYNTHETIC.as_bytes(), &MetadataLimits::default())
        .expect("最小数据集应可加载");
    assert_eq!(dataset.basemaps.len(), 1);
}

#[test]
fn rejects_key_id_mismatch() {
    expect_code(
        &SYNTHETIC.replace(
            r#""basemap_id": "bm:test_region/1","#,
            r#""basemap_id": "bm:test_region/2","#,
        ),
        CODE_BASEMAP_ID_INVALID,
    );
}

#[test]
fn rejects_duplicate_unit() {
    expect_code(
        &SYNTHETIC.replace(r#""unit_ids": [1],"#, r#""unit_ids": [1, 1],"#),
        CODE_BASEMAP_UNIT_DUPLICATE,
    );
}

#[test]
fn rejects_matrix_size_mismatch() {
    expect_code(
        &SYNTHETIC.replace(
            "[1024.0, 0.0, 0.0, 0.0, 1024.0, 0.0]",
            "[1.0, 0.0, 0.0, 0.0, 1.0, 0.0]",
        ),
        CODE_BASEMAP_TRANSFORM_INVALID,
    );
}

#[test]
fn rejects_analytic_with_y_up() {
    expect_code(
        &SYNTHETIC.replace(r#""coordinate_y_axis": "down""#, r#""coordinate_y_axis": "up""#),
        CODE_BASEMAP_TRANSFORM_INVALID,
    );
}

#[test]
fn rejects_missing_source() {
    expect_code(
        &SYNTHETIC.replace(r#"["src:test"]"#, r#"["src:absent"]"#),
        CODE_BASEMAP_SOURCE_MISSING,
    );
}

#[test]
fn rejects_unverified_with_error_fields() {
    expect_code(
        &SYNTHETIC.replace(r#""median_error_px": null,"#, r#""median_error_px": 1.0,"#),
        CODE_BASEMAP_STATUS_INVALID,
    );
}

#[test]
fn rejects_calibrated_without_errors() {
    expect_code(
        &SYNTHETIC.replace(r#""status": "unverified""#, r#""status": "calibrated""#),
        CODE_BASEMAP_STATUS_INVALID,
    );
}

#[test]
fn rejects_measured_without_samples() {
    let json = SYNTHETIC
        .replace(r#""status": "unverified""#, r#""status": "approximate""#)
        .replace(r#""median_error_px": null,"#, r#""median_error_px": 1.0,"#)
        .replace(r#""p95_error_px": null,"#, r#""p95_error_px": 2.0,"#)
        .replace(r#""max_error_px": null,"#, r#""max_error_px": 3.0,"#)
        .replace(r#""error_source": null,"#, r#""error_source": "measured","#);
    expect_code(&json, CODE_BASEMAP_STATUS_INVALID);
}

#[test]
fn rejects_calibrated_below_threshold() {
    // 1024² 对角线 ≈ 1448px，0.5% ≈ 7.24px；中位 100px 显然不够。
    let json = SYNTHETIC
        .replace(r#""status": "unverified""#, r#""status": "calibrated""#)
        .replace(r#""median_error_px": null,"#, r#""median_error_px": 100.0,"#)
        .replace(r#""p95_error_px": null,"#, r#""p95_error_px": 101.0,"#)
        .replace(r#""max_error_px": null,"#, r#""max_error_px": 102.0,"#);
    expect_code(&json, CODE_BASEMAP_PRECISION_INSUFFICIENT);
}

#[test]
fn accepts_calibrated_within_threshold() {
    let json = SYNTHETIC
        .replace(r#""status": "unverified""#, r#""status": "calibrated""#)
        .replace(r#""median_error_px": null,"#, r#""median_error_px": 1.0,"#)
        .replace(r#""p95_error_px": null,"#, r#""p95_error_px": 2.0,"#)
        .replace(r#""max_error_px": null,"#, r#""max_error_px": 3.0,"#);
    BasemapDataset::from_json(json.as_bytes(), &MetadataLimits::default())
        .expect("阈值内的 calibrated 应可加载");
}

#[test]
fn rejects_parent_traversal_in_image() {
    expect_code(
        &SYNTHETIC.replace(r#""maps/test.webp""#, r#""maps/../test.webp""#),
        CODE_BASEMAP_IMAGE_INVALID,
    );
}

#[test]
fn rejects_bundled_without_sha256() {
    expect_code(
        &SYNTHETIC.replace(r#""kind": "external""#, r#""kind": "bundled""#),
        CODE_BASEMAP_IMAGE_INVALID,
    );
}

#[test]
fn rejects_unknown_field() {
    expect_code(
        &SYNTHETIC.replace(r#""review": { "state": "draft" }"#, r#""review": { "state": "draft" }, "extra": 1"#),
        CODE_BASEMAP_JSON_INVALID,
    );
}

#[test]
fn rejects_bad_schema_version() {
    expect_code(
        &SYNTHETIC.replace(r#""schema_version": "1.0.0""#, r#""schema_version": "2.0.0""#),
        CODE_BASEMAP_VERSION_INVALID,
    );
}

#[test]
fn rejects_oversized_input() {
    let limits = MetadataLimits {
        max_file_bytes: 16,
        ..MetadataLimits::default()
    };
    let error = BasemapDataset::from_json(SYNTHETIC.as_bytes(), &limits)
        .expect_err("超限应被拒绝");
    assert_eq!(error.code(), CODE_BASEMAP_LIMIT);
}
