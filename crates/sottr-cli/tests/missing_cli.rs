//! SX-021 阶段 C：`missing` 子命令端到端（合成目录 + 最小批量报告夹具）。
//!
//! 使用 `CARGO_BIN_EXE_sottr-cli` 运行真实二进制；本测试不读取游戏数据。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_sottr-cli")
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("sottr-cli-missing-{tag}-{nanos}"))
}

const CATALOG: &str = r#"{
  "catalog_schema_version": "2.0.0",
  "catalog_version": "2.0.0",
  "generated_from": {},
  "totals": {"items": 3, "story": 0, "nonstory": 3, "required_for_100": 2, "dlc": 0, "with_position": 0, "with_region": 0},
  "acquisition_rules": {
    "monolith": {"text_zh": "解读石碑", "text_en": "Read the monolith", "source": "curated"},
    "explorer_backpack": {"text_zh": "拾取背包", "text_en": "Pick up the backpack", "source": "curated"}
  },
  "categories": [
    {"id": "monoliths", "name": "石碑", "count": 2, "required_for_100": true, "required_count": 2, "acquisition_rule_key": "monolith"},
    {"id": "explorer_backpacks", "name": "背包", "count": 1, "required_for_100": false, "required_count": 0, "acquisition_rule_key": "explorer_backpack"}
  ],
  "collections": [],
  "units": {},
  "regions": [],
  "items": {
    "0x00000001": {"kind": "nonstory", "name": "One", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": true, "region_hash": null, "region_slug": null, "category": "monoliths", "subcategory": null, "unit": null, "position": null, "acquisition_note": null},
    "0x00000002": {"kind": "nonstory", "name": "Two", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": true, "region_hash": null, "region_slug": null, "category": "monoliths", "subcategory": null, "unit": null, "position": null, "acquisition_note": null, "shared_id": true, "shared_names": ["Two", "TwoAlt"]},
    "0x00000003": {"kind": "nonstory", "name": "Three", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": false, "region_hash": null, "region_slug": null, "category": "explorer_backpacks", "subcategory": null, "unit": null, "position": null, "acquisition_note": null}
  }
}"#;

const ANALYSIS: &str = r#"{
  "schema_version": "1.0.0",
  "results": [
    {
      "input_ref": "save1.dat",
      "report": {
        "observations": {
          "inventory": {
            "records": [
              {"internal_id": "0x00000001"},
              {"internal_id": "0x9A2BBEFC"}
            ]
          }
        }
      },
      "error": null
    }
  ]
}"#;

fn run_missing(dir: &Path, output: &Path, extra: &[&str]) -> Output {
    let mut command = Command::new(cli_binary());
    command
        .arg("missing")
        .arg("--catalog")
        .arg(dir.join("catalog.json"))
        .arg("--analysis")
        .arg(dir.join("analysis.json"))
        .arg("--output")
        .arg(output);
    for arg in extra {
        command.arg(arg);
    }
    command.output().unwrap()
}

#[test]
fn missing_command_writes_category_progress_and_protected_list() {
    let dir = unique_dir("ok");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("catalog.json"), CATALOG).unwrap();
    fs::write(dir.join("analysis.json"), ANALYSIS).unwrap();

    let output = unique_dir("ok-out");
    let result = run_missing(&dir, &output, &[]);
    assert_eq!(result.status.code(), Some(0), "{result:?}");
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("缺失 1"), "{stdout}");

    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("missing.json")).unwrap()).unwrap();
    assert_eq!(document["input_ref"], "save1.dat");
    assert_eq!(document["catalog_version"], "2.0.0");
    assert_eq!(document["observed_items"], 1);
    assert_eq!(document["observed_not_in_catalog"][0], "0x9A2BBEFC");

    let monoliths = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["id"] == "monoliths")
        .unwrap();
    assert_eq!(monoliths["missing"], 1);
    let item = &monoliths["items"][0];
    assert_eq!(item["id"], "0x00000002");
    assert_eq!(item["shared_names"].as_array().unwrap().len(), 2);

    let backpacks = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["id"] == "explorer_backpacks")
        .unwrap();
    assert!(backpacks.get("items").is_none(), "可选类别不得给出清单");

    fs::remove_dir_all(&output).unwrap();
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_command_rejects_existing_output_and_unknown_input() {
    let dir = unique_dir("reject");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("catalog.json"), CATALOG).unwrap();
    fs::write(dir.join("analysis.json"), ANALYSIS).unwrap();

    let output = unique_dir("reject-out");
    let first = run_missing(&dir, &output, &[]);
    assert_eq!(first.status.code(), Some(0), "{first:?}");

    let reuse = run_missing(&dir, &output, &[]);
    assert_eq!(reuse.status.code(), Some(2), "输出目录已存在应拒绝");

    let unknown = run_missing(
        &dir,
        &unique_dir("reject-out2"),
        &["--input", "missing.dat"],
    );
    assert_eq!(unknown.status.code(), Some(2), "找不到指定输入应拒绝");

    fs::remove_dir_all(&output).unwrap();
    fs::remove_dir_all(&dir).unwrap();
}
