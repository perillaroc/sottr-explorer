//! SX-003 契约夹具测试：合法往返、非法拒绝与表示规则边界。
//!
//! 合法夹具必须能被 Rust 类型无损往返；非法夹具在反序列化时被拒绝；
//! 表示规则夹具验证 u64 键只以字符串往返。

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use sottr_core::metadata::MetadataDataset;
use sottr_core::report::{BatchReport, DesktopResult, ExportDocument, Report, ResourceKey};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/contract")
}

fn read_fixture(relative: &str) -> String {
    let path = fixtures_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("读取夹具失败 {path:?}: {error}"))
}

fn assert_round_trip<T: DeserializeOwned + Serialize>(relative: &str) {
    let text = read_fixture(relative);
    let original: serde_json::Value = serde_json::from_str(&text).unwrap();
    let typed: T = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{relative} 应可反序列化: {error}"));
    let reserialized = serde_json::to_value(&typed).unwrap();
    assert_eq!(original, reserialized, "{relative} 往返不一致");
}

#[test]
fn valid_report_fixtures_round_trip() {
    for name in [
        "report.valid.json",
        "report.degraded.json",
        "report.nonzero-group.json",
    ] {
        assert_round_trip::<Report>(&format!("valid/{name}"));
    }
}

#[test]
fn valid_envelope_and_metadata_fixtures_round_trip() {
    assert_round_trip::<BatchReport>("valid/batch.mixed.json");
    for name in [
        "desktop-result.current.json",
        "desktop-result.stale.json",
        "desktop-result.other-slot.json",
    ] {
        assert_round_trip::<DesktopResult>(&format!("valid/{name}"));
    }
    assert_round_trip::<ExportDocument>("valid/export.stale.json");
    assert_round_trip::<MetadataDataset>("valid/metadata.valid.json");
}

#[test]
fn invalid_report_fixtures_are_rejected() {
    for name in [
        "report.bad-observation.json",
        "report.null-value-without-reason.json",
        "report.verified-coordinates-null.json",
    ] {
        let text = read_fixture(&format!("invalid/{name}"));
        assert!(
            serde_json::from_str::<Report>(&text).is_err(),
            "{name} 应被拒绝"
        );
    }
}

#[test]
fn representation_boundary_fixture_round_trips_u64_as_strings() {
    let text = read_fixture("representation/u64-boundary.json");
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    let samples = document["samples"].as_array().unwrap();
    let expected = [u64::MAX, 9_007_199_254_740_993];
    for (sample, want) in samples.iter().zip(expected) {
        let key: ResourceKey = serde_json::from_value(sample["decimal"].clone()).unwrap();
        assert_eq!(key.raw(), want, "decimal 键应无损往返");
        let key: ResourceKey = serde_json::from_value(sample["hex"].clone()).unwrap();
        assert_eq!(key.raw(), want, "hex 键应无损往返");
    }
}

#[test]
fn representation_number_fixture_is_rejected() {
    let text = read_fixture("representation/u64-as-number.json");
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(document["decimal"].is_number());
    assert!(serde_json::from_value::<ResourceKey>(document["decimal"].clone()).is_err());
    assert!(serde_json::from_value::<ResourceKey>(document["hex"].clone()).is_err());
}
