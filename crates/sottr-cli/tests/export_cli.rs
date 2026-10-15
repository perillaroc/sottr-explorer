//! SX-011 CLI 导出集成测试：信封与 CSV 输出、分享模式、目标安全与格式选择。
//!
//! 使用合成夹具与仓库元数据；断言导出与库业务报告一致、原件不变、
//! 失败与取消不留下“完整报告”假象。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sottr_core::analyze::analyze_bytes;
use sottr_core::domain::context::DeclaredContext;
use sottr_core::hash::sha256_hex;
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::report::{
    ContextSource, ExportDocument, Freshness, Mode, SupportStatus, csv_text_is_unsafe,
    scan_private_material,
};
use sottr_core::synthetic::{SampleBuilder, compress};

const SAN_JUAN_CACHES: &str = "counter:mission_of_san_juan:survival_caches";
const SAN_JUAN_MONOLITHS: &str = "counter:mission_of_san_juan:monoliths";
const SAN_JUAN_CRYPTS: &str = "counter:mission_of_san_juan:crypts";

fn cli_binary() -> &'static str {
    env!("CARGO_BIN_EXE_sottr-cli")
}

fn metadata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/metadata")
}

fn load_metadata_bundle() -> MetadataBundle {
    let dir = metadata_dir();
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json") && name != "manifest.json")
        .collect();
    names.sort();
    let contents: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|name| (name.clone(), fs::read(dir.join(name)).unwrap()))
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

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sottr-export-it-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn consistent_sample(bundle: &MetadataBundle) -> Vec<u8> {
    let mut builder = SampleBuilder::new();
    builder.push_section(1, 0);
    builder.write_f32_le(0x28, 66.204_82_f32);
    builder.write_prefix_slot(0, 0x0F28_B6EC, 1);
    builder.write_prefix_slot(1, 0x439B_19A6, 1);
    builder.write_prefix_slot(2, 0xBA3B_0D06, 1);
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

fn run_cli(args: &[&str]) -> Output {
    Command::new(cli_binary())
        .args(args)
        .output()
        .expect("CLI 可执行文件应可运行")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// 最小 RFC 4180 解析器（引号包裹、双写引号、CRLF/LF）。
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

fn setup_samples(tag: &str) -> (PathBuf, PathBuf, Vec<u8>) {
    let root = unique_dir(tag);
    let samples = root.join("samples");
    fs::create_dir_all(&samples).unwrap();
    let bundle = load_metadata_bundle();
    let sample = consistent_sample(&bundle);
    fs::write(samples.join("save1.dat"), &sample).unwrap();
    (root, samples, sample)
}

#[test]
fn export_writes_envelope_and_csv_with_snapshot_identity() {
    let (root, samples, sample) = setup_samples("basic");
    let input = samples.join("save1.dat");
    let before = sha256_hex(&sample);
    let out = root.join("exports/out01");

    let result = run_cli(&[
        "export",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        out.to_str().unwrap(),
        "--metadata",
        metadata_dir().to_str().unwrap(),
        "--snapshot-id",
        "snap-it-0001",
        "--snapshot-time",
        "2026-10-15T12:00:00+08:00",
    ]);
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&result)
    );
    let stdout = stdout_text(&result);
    assert!(stdout.contains("导出计划"), "{stdout}");
    assert!(!stdout.contains("分享模式"), "默认非分享：{stdout}");
    assert!(stdout.contains("report.json"), "{stdout}");

    // 导出信封：快照身份、freshness、input_digest 与报告。
    let json = fs::read(out.join("report.json")).unwrap();
    let document: ExportDocument = serde_json::from_slice(&json).expect("信封必须严格可解析");
    assert_eq!(document.input_ref, "save1.dat");
    assert_eq!(document.snapshot_id, "snap-it-0001");
    assert_eq!(document.snapshot_time, "2026-10-15T12:00:00+08:00");
    assert_eq!(document.freshness, Freshness::Current);
    assert_eq!(document.last_refresh_error_code, None);
    assert_eq!(document.input_digest, document.report.input_digest);
    assert_eq!(
        document.report.capabilities.reference_difference.status,
        SupportStatus::Supported
    );

    // 导出内报告与库业务报告逐字段一致（同一输入摘要、元数据版本与证据）。
    let bundle = load_metadata_bundle();
    let declared = DeclaredContext {
        platform: None,
        mode: Mode::Campaign,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    };
    let library = analyze_bytes(&sample, &declared, &bundle, &Limits::default()).unwrap();
    assert_eq!(document.report, library);

    // CSV：表头、行数与快照列；全表无危险前缀单元格。
    let csv = fs::read_to_string(out.join("report.csv")).unwrap();
    let rows = parse_csv(&csv);
    assert_eq!(rows[0].len(), 15);
    assert_eq!(rows[0][0], "snapshot_id");
    assert_eq!(rows.len(), 1 + document.report.assessments.len());
    for row in &rows[1..] {
        assert_eq!(row.len(), 15);
        assert_eq!(row[0], "snap-it-0001");
        assert_eq!(row[1], "2026-10-15T12:00:00+08:00");
        assert_eq!(row[2], "current");
        assert_eq!(row[3], "");
        assert_eq!(row[5], document.input_digest.compressed_sha256);
        for cell in row {
            assert!(!csv_text_is_unsafe(cell), "{cell:?}");
        }
    }

    // 原件未改变；输入目录未写入任何输出。
    assert_eq!(sha256_hex(&fs::read(&input).unwrap()), before);
    let entries: Vec<String> = fs::read_dir(&samples)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries, vec!["save1.dat"]);

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn stale_export_marks_refresh_error_in_envelope_and_csv() {
    let (root, samples, _) = setup_samples("stale");
    let out = root.join("exports/out-stale");
    let result = run_cli(&[
        "export",
        "--input",
        samples.join("save1.dat").to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        out.to_str().unwrap(),
        "--metadata",
        metadata_dir().to_str().unwrap(),
        "--stale",
        "--refresh-error",
        "INPUT_CHANGED",
        "--snapshot-id",
        "snap-old-0001",
        "--snapshot-time",
        "2026-09-01T08:00:00Z",
    ]);
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&result)
    );
    let document: ExportDocument =
        serde_json::from_slice(&fs::read(out.join("report.json")).unwrap()).unwrap();
    assert_eq!(document.freshness, Freshness::Stale);
    assert_eq!(
        document.last_refresh_error_code.as_deref(),
        Some("INPUT_CHANGED")
    );
    let rows = parse_csv(&fs::read_to_string(out.join("report.csv")).unwrap());
    for row in &rows[1..] {
        assert_eq!(row[2], "stale");
        assert_eq!(row[3], "INPUT_CHANGED");
    }
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn share_export_scans_clean_and_keeps_business_values() {
    let (root, samples, _) = setup_samples("share");
    let input = samples.join("save1.dat");
    let out_full = root.join("exports/full");
    let out_share = root.join("exports/share");
    let metadata = metadata_dir();
    let common = [
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--metadata",
        metadata.to_str().unwrap(),
        "--snapshot-id",
        "snap-share-0001",
        "--snapshot-time",
        "2026-10-15T12:00:00Z",
    ];
    let mut full_args = vec!["export", "--output", out_full.to_str().unwrap()];
    full_args.extend(common);
    let result = run_cli(&full_args);
    assert_eq!(result.status.code(), Some(0), "{}", stderr_text(&result));

    let mut share_args = vec!["export", "--output", out_share.to_str().unwrap(), "--share"];
    share_args.extend(common);
    let result = run_cli(&share_args);
    assert_eq!(result.status.code(), Some(0), "{}", stderr_text(&result));
    assert!(stdout_text(&result).contains("分享模式"));

    let full_document: ExportDocument =
        serde_json::from_slice(&fs::read(out_full.join("report.json")).unwrap()).unwrap();
    let share_document: ExportDocument =
        serde_json::from_slice(&fs::read(out_share.join("report.json")).unwrap()).unwrap();
    // 正常数据没有路径样内容：分享与完整导出业务值一致（分享为防御层）。
    assert_eq!(share_document, full_document);
    let share_json = fs::read_to_string(out_share.join("report.json")).unwrap();
    let share_csv = fs::read_to_string(out_share.join("report.csv")).unwrap();
    assert!(scan_private_material(&share_json).is_empty());
    assert!(scan_private_material(&share_csv).is_empty());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn export_rejects_bad_targets_and_leaves_no_artifacts() {
    let (root, samples, sample) = setup_samples("reject");
    let input = samples.join("save1.dat");
    let before = sha256_hex(&sample);
    let metadata = metadata_dir();
    let exports = root.join("exports");
    fs::create_dir_all(&exports).unwrap();
    let file_blocker = exports.join("blocker");
    fs::write(&file_blocker, b"x").unwrap();

    let cases: Vec<(&str, Vec<String>, &str)> = vec![
        (
            "输出已存在",
            vec![
                "export",
                "--input",
                input.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                exports.to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "已存在",
        ),
        (
            "输出在输入目录内",
            vec![
                "export",
                "--input",
                input.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                samples.join("out").to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "重叠",
        ),
        (
            "输出在输入文件之下",
            vec![
                "export",
                "--input",
                input.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                samples.join("save1.dat/child").to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "重叠",
        ),
        (
            "输出父路径是文件",
            vec![
                "export",
                "--input",
                input.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                file_blocker.join("out").to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "不是目录",
        ),
        (
            "输入是目录",
            vec![
                "export",
                "--input",
                samples.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                exports.join("out-a").to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "只接受单个存档文件",
        ),
        (
            "--stale 缺少 --refresh-error",
            vec![
                "export",
                "--input",
                input.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                exports.join("out-b").to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
                "--stale",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "--refresh-error",
        ),
        (
            "--refresh-error 未配合 --stale",
            vec![
                "export",
                "--input",
                input.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                exports.join("out-c").to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
                "--refresh-error",
                "INPUT_CHANGED",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "--stale",
        ),
        (
            "非法快照时间",
            vec![
                "export",
                "--input",
                input.to_str().unwrap(),
                "--context",
                "campaign",
                "--output",
                exports.join("out-d").to_str().unwrap(),
                "--metadata",
                metadata.to_str().unwrap(),
                "--snapshot-time",
                "2026-10-15",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            "RFC 3339",
        ),
    ];
    for (label, case, expect_message) in &cases {
        let args: Vec<&str> = case.iter().map(String::as_str).collect();
        let result = run_cli(&args);
        assert_eq!(
            result.status.code(),
            Some(2),
            "{label}: {}",
            stderr_text(&result)
        );
        assert!(
            stderr_text(&result).contains(expect_message),
            "{label}: {}",
            stderr_text(&result)
        );
    }

    // 失败不留下任何导出产物；原件不变。
    let entries: Vec<String> = fs::read_dir(&exports)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries, vec!["blocker"], "失败后不得留下输出目录或临时文件");
    assert_eq!(sha256_hex(&fs::read(&input).unwrap()), before);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn export_format_selection_and_input_errors() {
    let (root, samples, _) = setup_samples("format");
    let input = samples.join("save1.dat");
    let metadata = metadata_dir();

    let out_json = root.join("exports/json-only");
    let result = run_cli(&[
        "export",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        out_json.to_str().unwrap(),
        "--metadata",
        metadata.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(result.status.code(), Some(0), "{}", stderr_text(&result));
    let entries: Vec<String> = fs::read_dir(&out_json)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries, vec!["report.json"]);

    let out_csv = root.join("exports/csv-only");
    let result = run_cli(&[
        "export",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        out_csv.to_str().unwrap(),
        "--metadata",
        metadata.to_str().unwrap(),
        "--format",
        "csv",
    ]);
    assert_eq!(result.status.code(), Some(0), "{}", stderr_text(&result));
    let entries: Vec<String> = fs::read_dir(&out_csv)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries, vec!["report.csv"]);

    // 输入级错误：坏文件退出码 1，且不创建输出目录。
    let broken = samples.join("broken.dat");
    fs::write(&broken, b"not a zlib stream").unwrap();
    let out_broken = root.join("exports/broken");
    let result = run_cli(&[
        "export",
        "--input",
        broken.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        out_broken.to_str().unwrap(),
        "--metadata",
        metadata.to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(1), "{}", stderr_text(&result));
    assert!(!out_broken.exists(), "输入错误不得留下输出目录");

    fs::remove_dir_all(&root).unwrap();
}
