//! SX-022：`plan` 子命令端到端（合成图鉴 + 最小批量报告夹具）。
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
    std::env::temp_dir().join(format!("sottr-cli-plan-{tag}-{nanos}"))
}

const CATALOG: &str = r#"{
  "catalog_schema_version": "2.0.0",
  "catalog_version": "2.0.0",
  "generated_from": {},
  "totals": {"items": 4, "story": 3, "nonstory": 1, "required_for_100": 4, "dlc": 0, "with_position": 4, "with_region": 4},
  "acquisition_rules": {
    "relic": {"text_zh": "拾取遗物", "text_en": "Pick up the relic", "source": "curated"},
    "document": {"text_zh": "拾取文献", "text_en": "Pick up the document", "source": "curated"},
    "survival_cache": {"text_zh": "打开箱子", "text_en": "Open the crate", "source": "curated"}
  },
  "categories": [
    {"id": "artifacts", "name": "故事收藏", "count": 3, "required_for_100": true, "required_count": 3, "acquisition_rule_key": "relic",
     "subcategories": [
        {"id": "relics", "name": "Relic（遗物）", "count": 2, "acquisition_rule_key": "relic"},
        {"id": "documents", "name": "Document（文献）", "count": 1, "acquisition_rule_key": "document"},
        {"id": "murals", "name": "Mural（壁画）", "count": 0, "acquisition_rule_key": "relic"}
     ]},
    {"id": "survival_caches", "name": "生存贮藏点", "count": 1, "required_for_100": true, "required_count": 1, "acquisition_rule_key": "survival_cache"}
  ],
  "collections": [],
  "units": {
    "243": {"zone": "pa_hub_paititi", "display_en": "PAITITI HUB", "display_zh": "帕依提提主城", "name_source": "curated", "region_slug": "paititi"},
    "2287": {"zone": "hv_hollow_village_v2_01", "display_en": "KUWAQ YAKU", "display_zh": "库阿克雅库", "name_source": "curated", "region_slug": "kuwaq_yaku"}
  },
  "regions": [
    {"region_hash": "C5B30CDB", "slug": "paititi", "name_en": "PAITITI", "name_zh": "帕依提提", "name_source": "game_locale", "is_dlc": false, "counts_toward_percent": true, "item_count": 3, "binding": null, "challenge_tombs": [], "campsites": [], "challenges": [], "missions": []},
    {"region_hash": "838BAE8C", "slug": "kuwaq_yaku", "name_en": "KUWAQ YAKU", "name_zh": "库阿克雅库", "name_source": "game_locale", "is_dlc": false, "counts_toward_percent": true, "item_count": 1, "binding": null, "challenge_tombs": [], "campsites": [], "challenges": [], "missions": []}
  ],
  "items": {
    "0x00000001": {"kind": "story", "name": "R1", "name_en": "RELIC ONE", "name_zh": "遗物一", "name_source": "game_locale", "collection": "C1", "is_dlc": false, "required_for_100": true, "region_hash": "C5B30CDB", "region_slug": "paititi", "category": "artifacts", "subcategory": "relics", "unit": 243, "position": {"x": 0.4, "y": 0.5, "space": "unit_normalized_unverified"}, "acquisition_note": null},
    "0x00000002": {"kind": "story", "name": "D1", "name_en": "DOC ONE", "name_zh": "文献一", "name_source": "game_locale", "collection": "C1", "is_dlc": false, "required_for_100": true, "region_hash": "C5B30CDB", "region_slug": "paititi", "category": "artifacts", "subcategory": "documents", "unit": 243, "position": {"x": 0.6, "y": 0.2, "space": "unit_normalized_unverified"}, "acquisition_note": null},
    "0x00000003": {"kind": "nonstory", "name": "Cache1", "name_en": null, "name_zh": null, "name_source": null, "is_dlc": false, "required_for_100": true, "region_hash": "C5B30CDB", "region_slug": "paititi", "category": "survival_caches", "subcategory": null, "unit": 243, "position": {"x": 0.7, "y": 0.3, "space": "unit_normalized_unverified"}, "acquisition_note": null},
    "0x00000004": {"kind": "story", "name": "R2", "name_en": "RELIC TWO", "name_zh": "遗物二", "name_source": "game_locale", "collection": "C2", "is_dlc": false, "required_for_100": true, "region_hash": "838BAE8C", "region_slug": "kuwaq_yaku", "category": "artifacts", "subcategory": "relics", "unit": 2287, "position": {"x": 0.2, "y": 0.8, "space": "unit_normalized_unverified"}, "acquisition_note": null}
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

fn run_plan(dir: &Path, output: &Path, extra: &[&str]) -> Output {
    let mut command = Command::new(cli_binary());
    command
        .arg("plan")
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
fn plan_command_writes_region_progress_and_item_hints() {
    let dir = unique_dir("ok");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("catalog.json"), CATALOG).unwrap();
    fs::write(dir.join("analysis.json"), ANALYSIS).unwrap();

    let output = unique_dir("ok-out");
    let result = run_plan(&dir, &output, &[]);
    assert_eq!(result.status.code(), Some(0), "{result:?}");
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("帕依提提"), "{stdout}");

    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("plan.json")).unwrap()).unwrap();
    assert_eq!(document["input_ref"], "save1.dat");
    assert_eq!(document["catalog_version"], "2.0.0");
    assert_eq!(document["main_collected"], 1);
    assert_eq!(document["main_total"], 4);
    assert_eq!(document["observed_not_in_catalog"][0], "0x9A2BBEFC");

    let regions = document["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 2);
    let paititi = regions
        .iter()
        .find(|region| region["slug"] == "paititi")
        .unwrap();
    assert_eq!(paititi["name_zh"], "帕依提提");
    assert_eq!(paititi["kind"], "main");
    assert_eq!(paititi["collected"], 1);
    assert_eq!(paititi["total"], 3);

    let groups = paititi["groups"].as_array().unwrap();
    let relics = groups
        .iter()
        .find(|group| group["subcategory"] == "relics")
        .unwrap();
    assert_eq!(relics["collected"], 1);
    assert_eq!(relics["total"], 1);
    let item = relics["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "0x00000001")
        .unwrap();
    assert_eq!(item["status"], "collected");
    assert_eq!(item["display_name"], "遗物一");
    assert!(item["location_hint"].as_str().unwrap().contains("未校准"));
    assert!(
        item["location_hint"]
            .as_str()
            .unwrap()
            .contains("帕依提提主城")
    );
    assert_eq!(item["acquisition_source"], "category_rule");
    assert_eq!(item["acquisition_text"], "拾取遗物");

    let documents = groups
        .iter()
        .find(|group| group["subcategory"] == "documents")
        .unwrap();
    let doc_item = &documents["items"][0];
    assert_eq!(doc_item["status"], "not_collected");
    assert_eq!(doc_item["acquisition_text"], "拾取文献");

    fs::remove_dir_all(&output).unwrap();
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn plan_command_rejects_existing_output_and_unknown_input() {
    let dir = unique_dir("reject");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("catalog.json"), CATALOG).unwrap();
    fs::write(dir.join("analysis.json"), ANALYSIS).unwrap();

    let output = unique_dir("reject-out");
    let first = run_plan(&dir, &output, &[]);
    assert_eq!(first.status.code(), Some(0), "{first:?}");

    let reuse = run_plan(&dir, &output, &[]);
    assert_eq!(reuse.status.code(), Some(2), "输出目录已存在应拒绝");

    let unknown = run_plan(
        &dir,
        &unique_dir("reject-out2"),
        &["--input", "missing.dat"],
    );
    assert_eq!(unknown.status.code(), Some(2), "找不到指定输入应拒绝");

    fs::remove_dir_all(&output).unwrap();
    fs::remove_dir_all(&dir).unwrap();
}
