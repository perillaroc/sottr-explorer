//! SX-011 导出集成测试：导出信封、严格序列化、CSV 与分享脱敏。
//!
//! 使用合成夹具与真实元数据；恶意元数据名称使用合成数据集构造，
//! 不依赖外部样本。

use std::collections::BTreeMap;

use serde_json::{Value, json};

use sottr_core::analyze::analyze_bytes;
use sottr_core::domain::context::DeclaredContext;
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::report::{
    CSV_COLUMNS, ContextSource, Diagnostic, DiagnosticScope, ExportDocument, ExportSnapshot,
    Freshness, Mode, Report, ResourceKey, Severity, csv_escape, csv_text_is_unsafe,
    export_document, render_export_csv, render_export_json, sanitize_csv_text,
    scan_private_material, share_export_document,
};
use sottr_core::synthetic::{SampleBuilder, compress};

const SAN_JUAN_CACHES: &str = "counter:mission_of_san_juan:survival_caches";
const SAN_JUAN_MONOLITHS: &str = "counter:mission_of_san_juan:monoliths";
const SAN_JUAN_CRYPTS: &str = "counter:mission_of_san_juan:crypts";

fn metadata_bundle() -> MetadataBundle {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/metadata");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json") && name != "manifest.json")
        .collect();
    names.sort();
    let contents: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|name| (name.clone(), std::fs::read(dir.join(name)).unwrap()))
        .collect();
    let inputs: Vec<MetadataFileInput<'_>> = contents
        .iter()
        .map(|(file_id, bytes)| MetadataFileInput {
            file_id: file_id.clone(),
            bytes,
        })
        .collect();
    MetadataBundle::from_files(&inputs, &MetadataLimits::default()).unwrap()
}

fn campaign() -> DeclaredContext {
    DeclaredContext {
        platform: Some("windows_steam".to_string()),
        mode: Mode::Campaign,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    }
}

/// 合成自洽样本：前缀 ID 与计数读数一致。
fn consistent_sample(bundle: &MetadataBundle) -> Vec<u8> {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    builder.write_f32_le(0x28, 66.204_82_f32);
    builder.write_prefix_slot(0, 0x0F28_B6EC, 1); // 圣胡安贮藏点（普通）
    builder.write_prefix_slot(1, 0x439B_19A6, 1); // 圣胡安石碑
    builder.write_prefix_slot(2, 0xBA3B_0D06, 1); // 圣胡安墓穴宝藏
    let mut raw = builder.finish();
    let specs = bundle.region_counter_specs();
    for (counter_key, value) in [
        (SAN_JUAN_CACHES, 1u32),
        (SAN_JUAN_MONOLITHS, 1),
        (SAN_JUAN_CRYPTS, 1),
    ] {
        let spec = specs
            .iter()
            .find(|spec| spec.counter_key == counter_key)
            .unwrap();
        let offset = spec.serialized_offset as usize;
        raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    compress(&raw)
}

fn sample_report() -> Report {
    let bundle = metadata_bundle();
    let bytes = consistent_sample(&bundle);
    analyze_bytes(&bytes, &campaign(), &bundle, &Limits::default()).unwrap()
}

fn snapshot<'a>(
    input_ref: &'a str,
    snapshot_id: &'a str,
    snapshot_time: &'a str,
) -> ExportSnapshot<'a> {
    ExportSnapshot {
        input_ref,
        snapshot_id,
        snapshot_time,
        freshness: Freshness::Current,
        last_refresh_error_code: None,
    }
}

fn export_current(report: Report) -> ExportDocument {
    export_document(
        report,
        snapshot("save1.dat", "snap-test-0001", "2026-10-15T12:00:00Z"),
    )
    .unwrap()
}

/// 最小解析器（RFC 4180：引号包裹、双写引号、CRLF/LF 行尾）。
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut in_quotes = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if in_quotes {
            if character == '"' {
                if characters.peek() == Some(&'"') {
                    cell.push('"');
                    characters.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cell.push(character);
            }
        } else {
            match character {
                '"' => in_quotes = true,
                ',' => row.push(std::mem::take(&mut cell)),
                '\r' => {
                    if characters.peek() == Some(&'\n') {
                        characters.next();
                    }
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                '\n' => {
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                other => cell.push(other),
            }
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

#[test]
fn export_document_links_digest_metadata_and_evidence() {
    let report = sample_report();
    let expected_digest = report.input_digest.clone();
    let expected_metadata = report.metadata_version.clone();
    let expected_assessments = report.assessments.clone();
    let document = export_current(report);

    assert_eq!(
        sottr_core::report::ExportDocumentKind::VALUE,
        "report_export"
    );
    assert_eq!(sottr_core::report::SchemaVersion::VALUE, "1.0.0");
    assert_eq!(document.input_ref, "save1.dat");
    assert_eq!(document.snapshot_id, "snap-test-0001");
    assert_eq!(document.snapshot_time, "2026-10-15T12:00:00Z");
    assert_eq!(document.freshness, Freshness::Current);
    assert_eq!(document.last_refresh_error_code, None);
    // 导出与屏幕报告指向同一输入摘要、元数据版本与评估证据。
    assert_eq!(document.input_digest, expected_digest);
    assert_eq!(document.report.input_digest, expected_digest);
    assert_eq!(document.report.metadata_version, expected_metadata);
    assert_eq!(document.report.assessments, expected_assessments);
    assert!(
        document
            .report
            .evidence
            .iter()
            .any(|evidence| evidence.evidence_id.starts_with("ev:refdiff:"))
    );
}

#[test]
fn export_json_round_trips_strictly_and_stays_js_safe() {
    let document = export_current(sample_report());
    let text = render_export_json(&document).unwrap();
    let parsed: ExportDocument = serde_json::from_str(&text).expect("严格反序列化必须通过");
    assert_eq!(parsed, document);

    // ID 与摘要为字符串；数值无 JS 精度风险且浮点有限。
    let value: Value = serde_json::to_value(&document).unwrap();
    assert!(value["report"]["assessments"][0]["internal_id"].is_string());
    assert!(value["report"]["input_digest"]["compressed_sha256"].is_string());
    check_js_safe(&value);

    // u64 资源键字符串往返（禁止经 JavaScript Number）。
    assert_eq!(
        serde_json::to_string(&ResourceKey::new(u64::MAX)).unwrap(),
        "\"18446744073709551615\""
    );

    // NaN/Infinity 必须被拒绝而不是退化为 null。
    let mut broken = document.clone();
    broken.report.observations.overall_completion.value = Some(f64::NAN);
    assert!(render_export_json(&broken).is_err());
    let mut broken = document;
    broken.report.locations[0]
        .coordinates
        .replace(sottr_core::report::Coordinates {
            x: f64::INFINITY,
            y: 0.0,
            z: 0.0,
            coordinate_space: "world".to_string(),
            unit: "meter".to_string(),
            level_id: "test".to_string(),
        });
    assert!(render_export_json(&broken).is_err());
}

fn check_js_safe(value: &Value) {
    match value {
        Value::Number(number) => {
            if let Some(unsigned) = number.as_u64() {
                assert!(unsigned <= (1u64 << 53), "整数超过 JS 安全范围：{number}");
            } else if let Some(signed) = number.as_i64() {
                assert!(
                    signed.unsigned_abs() <= (1u64 << 53),
                    "整数超过 JS 安全范围：{number}"
                );
            } else if let Some(float) = number.as_f64() {
                assert!(float.is_finite(), "非有限浮点：{number}");
            } else {
                panic!("非法 JSON 数值：{number}");
            }
        }
        Value::Array(items) => items.iter().for_each(check_js_safe),
        Value::Object(entries) => entries.values().for_each(check_js_safe),
        _ => {}
    }
}

#[test]
fn export_csv_keeps_identity_and_item_semantics() {
    let bundle = metadata_bundle();
    let document = export_current(sample_report());
    let csv = render_export_csv(&document, &bundle, false);
    let rows = parse_csv(&csv);

    assert_eq!(rows[0], CSV_COLUMNS.to_vec());
    assert_eq!(rows.len(), 1 + document.report.assessments.len());
    for row in &rows[1..] {
        assert_eq!(row.len(), CSV_COLUMNS.len(), "列数必须与表头一致：{row:?}");
        assert_eq!(row[0], "snap-test-0001");
        assert_eq!(row[1], "2026-10-15T12:00:00Z");
        assert_eq!(row[2], "current");
        assert_eq!(row[3], "");
        assert_eq!(row[4], "save1.dat");
        assert_eq!(
            row[5], document.input_digest.compressed_sha256,
            "CSV 必须引用同一 input_digest"
        );
    }

    // 逐项语义：present 贮藏点 → likely_collected；组定位为 guide_group。
    let cache_row = rows[1..]
        .iter()
        .find(|row| row[6] == "0x0F28B6EC")
        .expect("缓存条目必须在 CSV 中");
    assert_eq!(cache_row[8], "present");
    assert_eq!(cache_row[9], "likely_collected");
    assert_eq!(cache_row[10], "community_correlated");
    assert_eq!(cache_row[11], "refdiff.v1.likely_collected");
    assert!(cache_row[12].contains("参考"), "规则描述：{cache_row:?}");
    assert_eq!(
        cache_row[13],
        "loc:guide:mission_of_san_juan:survival_caches"
    );
    assert_eq!(cache_row[14], "guide_group");
    let names: Vec<String> = serde_json::from_str(&cache_row[7]).unwrap();
    assert!(
        names
            .iter()
            .any(|name| name == "CollectibleTypeSurvivalCache_SpanishMission_03"),
        "{names:?}"
    );

    // 缺席参考项 → likely_missing。
    let missing_row = rows[1..]
        .iter()
        .find(|row| row[6] == "0x0F28B6ED")
        .expect("缺席缓存必须在 CSV 中");
    assert_eq!(missing_row[9], "likely_missing");
    assert_eq!(missing_row[11], "refdiff.v1.likely_missing");

    // 帕伊提提线索 ID 现在属于区域×类别攻略组，导出取更精确的 guide_group 定位。
    let region_row = rows[1..]
        .iter()
        .find(|row| row[6] == "0x851B4915")
        .expect("帕伊提提线索条目必须在 CSV 中");
    assert_eq!(region_row[13], "loc:guide:paititi:survival_caches");
    assert_eq!(region_row[14], "guide_group");

    // 全表无危险前缀单元格（公式保护在整表范围成立）。
    for row in &rows[1..] {
        for cell in row {
            assert!(!csv_text_is_unsafe(cell), "危险单元格未被转义：{cell:?}");
        }
    }
}

/// 恶意元数据名称：公式前缀、前导空白绕过、引号、逗号、换行与路径。
fn malicious_bundle() -> MetadataBundle {
    let aliases = [
        "=cmd|' /C calc'!A0",
        "  =前导空白绕过",
        "+加号开头",
        "-减号开头",
        "@at 开头",
        "中文字段",
        "包含,逗号",
        "包含\"引号\"",
        "包含\n换行",
        "D:\\Secret\\save1.dat",
        "..\\saves\\save2.dat",
    ];
    let item = json!({
        "internal_id": "0x0F28B6EC",
        "aliases": aliases,
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
    });
    let document = json!({
        "schema_version": "1.0.0",
        "metadata_version": "0.1.0-test",
        "namespace": "sottr",
        "items": { "0x0F28B6EC": item },
        "categories": {},
        "regions": {},
        "guide_groups": {},
        "reference_sets": {},
        "region_counters": {},
        "sources": {}
    });
    let text = document.to_string();
    MetadataBundle::from_files(
        &[MetadataFileInput {
            file_id: "malicious.json".to_string(),
            bytes: text.as_bytes(),
        }],
        &MetadataLimits::default(),
    )
    .unwrap()
}

#[test]
fn export_csv_escapes_malicious_metadata_names() {
    let bundle = malicious_bundle();
    let document = export_current(sample_report());
    let csv = render_export_csv(&document, &bundle, false);
    let rows = parse_csv(&csv);
    let row = rows[1..]
        .iter()
        .find(|row| row[6] == "0x0F28B6EC")
        .expect("恶意名称条目必须在 CSV 中");

    // JSON 数组编码逐字保留全部名称（含逗号、引号与换行转义），
    // 单元格首字符为 `[`，本身不构成公式前缀。
    let names: Vec<String> = serde_json::from_str(&row[7]).unwrap();
    assert_eq!(names.len(), 11);
    assert_eq!(names[0], "=cmd|' /C calc'!A0");
    assert_eq!(names[1], "  =前导空白绕过");
    assert!(names.iter().any(|name| name == "包含,逗号"));
    assert!(names.iter().any(|name| name == "包含\"引号\""));
    assert!(names.iter().any(|name| name == "包含\n换行"));
    assert!(!row[7].contains('\n'), "JSON 数组不得含裸换行");

    // 危险前缀转义函数本身：每个恶意名称都必须被加转义标记。
    for dangerous in [
        "=cmd|' /C calc'!A0",
        "  =前导空白绕过",
        "+加号开头",
        "-减号开头",
        "@at 开头",
        "\t制表开头",
    ] {
        assert!(
            sanitize_csv_text(dangerous).starts_with('\''),
            "{dangerous}"
        );
        assert!(
            csv_escape(dangerous).starts_with('\'') || csv_escape(dangerous).starts_with("\"'")
        );
    }
    // 安全文本不被改写。
    assert_eq!(csv_escape("0x0F28B6EC"), "0x0F28B6EC");
    assert_eq!(csv_escape("66.204819"), "66.204819");

    // 全表无危险前缀单元格。
    for row in &rows[1..] {
        for cell in row {
            assert!(!csv_text_is_unsafe(cell), "{cell:?}");
        }
    }

    // 分享模式的 CSV：路径样名称替换为文件名，其余名称保持不变。
    let shared_csv = render_export_csv(&document, &bundle, true);
    let shared_rows = parse_csv(&shared_csv);
    let shared_row = shared_rows[1..]
        .iter()
        .find(|row| row[6] == "0x0F28B6EC")
        .unwrap();
    let shared_names: Vec<String> = serde_json::from_str(&shared_row[7]).unwrap();
    assert!(shared_names.iter().any(|name| name == "save1.dat"));
    assert!(shared_names.iter().any(|name| name == "save2.dat"));
    assert!(!shared_names.iter().any(|name| name.contains("Secret")));
    assert!(scan_private_material(&shared_csv).is_empty());
}

#[test]
fn share_export_scrubs_paths_and_keeps_business_values() {
    let mut document = export_current(sample_report());
    document.input_ref = "C:\\Users\\me\\Shadow of the Tomb Raider\\save1.dat".to_string();
    document.report.diagnostics.push(Diagnostic {
        code: "TEST_DIAGNOSTIC".to_string(),
        severity: Severity::Info,
        scope: DiagnosticScope::Input,
        message: "读取 D:\\Secret\\save1.dat 失败".to_string(),
        evidence_refs: Vec::new(),
    });
    document
        .report
        .context
        .limitations
        .push("来源目录 \\\\server\\share\\x.dat".to_string());
    document.report.locations[0]
        .limitations
        .push("相对 ..\\saves\\save2.dat 引用".to_string());
    if let Some(evidence) = document.report.evidence.first_mut() {
        evidence.method = "read C:/Users/me/save1.dat".to_string();
    }
    let assessments_before = document.report.assessments.clone();
    let digest_before = document.report.input_digest.clone();

    let shared = share_export_document(&document).unwrap();
    assert_eq!(shared.input_ref, "save1.dat");
    let diagnostic = shared
        .report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "TEST_DIAGNOSTIC")
        .unwrap();
    assert_eq!(diagnostic.message, "读取 save1.dat 失败");
    assert!(
        shared
            .report
            .context
            .limitations
            .iter()
            .any(|limitation| limitation == "来源目录 x.dat")
    );
    assert!(
        shared.report.locations[0]
            .limitations
            .iter()
            .any(|limitation| limitation == "相对 save2.dat 引用")
    );
    if let Some(evidence) = shared.report.evidence.first() {
        assert_eq!(evidence.method, "read save1.dat");
    }

    // 业务值、摘要与快照不变；分享文档仍然契约有效且自检干净。
    assert_eq!(shared.report.assessments, assessments_before);
    assert_eq!(shared.report.input_digest, digest_before);
    assert_eq!(shared.snapshot_id, document.snapshot_id);
    assert_eq!(shared.freshness, document.freshness);
    let text = render_export_json(&shared).unwrap();
    let reparsed: ExportDocument = serde_json::from_str(&text).unwrap();
    assert_eq!(reparsed, shared);
    assert!(scan_private_material(&text).is_empty(), "{text}");
}

#[test]
fn export_document_rejects_invalid_snapshot_inputs() {
    // freshness=none 不可导出。
    let error = export_document(
        sample_report(),
        ExportSnapshot {
            input_ref: "save1.dat",
            snapshot_id: "snap-1",
            snapshot_time: "2026-10-15T12:00:00Z",
            freshness: Freshness::None,
            last_refresh_error_code: None,
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("freshness"), "{error}");

    // 空快照身份不可导出。
    assert!(
        export_document(
            sample_report(),
            ExportSnapshot {
                input_ref: "save1.dat",
                snapshot_id: "",
                snapshot_time: "2026-10-15T12:00:00Z",
                freshness: Freshness::Current,
                last_refresh_error_code: None,
            },
        )
        .is_err()
    );

    // 非法刷新错误码不可导出。
    assert!(
        export_document(
            sample_report(),
            ExportSnapshot {
                input_ref: "save1.dat",
                snapshot_id: "snap-1",
                snapshot_time: "2026-10-15T12:00:00Z",
                freshness: Freshness::Stale,
                last_refresh_error_code: Some("input_changed"),
            },
        )
        .is_err()
    );

    // 过期导出必须携带刷新错误码（与 CLI 层规则一致）。
    let document = export_document(
        sample_report(),
        ExportSnapshot {
            input_ref: "save1.dat",
            snapshot_id: "snap-1",
            snapshot_time: "2026-10-15T12:00:00Z",
            freshness: Freshness::Stale,
            last_refresh_error_code: Some("INPUT_CHANGED"),
        },
    )
    .unwrap();
    assert_eq!(document.freshness, Freshness::Stale);
    assert_eq!(
        document.last_refresh_error_code.as_deref(),
        Some("INPUT_CHANGED")
    );
    let csv = render_export_csv(&document, &MetadataBundle::empty(), false);
    let rows = parse_csv(&csv);
    assert_eq!(rows[1][2], "stale");
    assert_eq!(rows[1][3], "INPUT_CHANGED");
}

#[test]
fn export_csv_with_empty_assessments_keeps_header_only() {
    // 空元数据 + 非战役上下文：评估为空，CSV 仍应输出表头。
    let bundle = MetadataBundle::empty();
    let bytes = compress(&vec![0u8; 25_600]);
    let declared = DeclaredContext {
        mode: Mode::Profile,
        source: ContextSource::UserDeclared,
        ..DeclaredContext::default()
    };
    let report = analyze_bytes(&bytes, &declared, &bundle, &Limits::default()).unwrap();
    assert!(report.assessments.is_empty());
    let document = export_current(report);
    let csv = render_export_csv(&document, &bundle, false);
    let rows = parse_csv(&csv);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], CSV_COLUMNS.to_vec());
    let json = render_export_json(&document).unwrap();
    assert!(serde_json::from_str::<ExportDocument>(&json).is_ok());
}

#[test]
fn csv_columns_and_helpers_are_stable() {
    assert_eq!(CSV_COLUMNS.len(), 15);
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for column in CSV_COLUMNS {
        *seen.entry(column).or_insert(0) += 1;
    }
    assert!(seen.values().all(|count| *count == 1), "列名不得重复");
}
