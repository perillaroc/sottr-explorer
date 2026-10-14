//! SX-009 CLI 集成测试：库与 CLI 业务报告一致、批量语义、退出码与输入保护。
//!
//! 使用合成夹具（不依赖私有样本）：CLI 二进制由 `CARGO_BIN_EXE_sottr-cli` 提供，
//! 元数据使用仓库内 `data/metadata/`。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sottr_core::analyze::analyze_bytes;
use sottr_core::domain::context::DeclaredContext;
use sottr_core::hash::sha256_hex;
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::report::{AssessmentKind, BatchReport, ContextSource, Mode, SupportStatus};
use sottr_core::synthetic::{SampleBuilder, compress};

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
    let dir = std::env::temp_dir().join(format!("sottr-cli-it-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// 构造一个“自洽”的 4 MiB 战役样本：前缀 ID 与区域计数一致。
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
        ("counter:mission_of_san_juan:survival_caches", 1u32),
        ("counter:mission_of_san_juan:monoliths", 1),
        ("counter:mission_of_san_juan:crypts", 1),
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

#[test]
fn analyze_matches_library_business_report_and_keeps_inputs() {
    let root = unique_dir("match");
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).unwrap();
    let bundle = load_metadata_bundle();
    let sample = consistent_sample(&bundle);
    fs::write(input.join("save1.dat"), &sample).unwrap();
    let before = sha256_hex(&sample);

    let result = run_cli(&[
        "analyze",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        output.to_str().unwrap(),
        "--metadata",
        metadata_dir().to_str().unwrap(),
    ]);
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&result)
    );

    let json = fs::read(output.join("analysis.json")).unwrap();
    let batch: BatchReport = serde_json::from_slice(&json).expect("批量 JSON 必须可解析");
    assert_eq!(batch.results.len(), 1);
    let report = batch.results[0].report.as_ref().expect("报告存在");

    // 与库调用逐字段一致（同一核心、同一输入、同一上下文）。
    let declared = DeclaredContext {
        platform: None,
        mode: Mode::Campaign,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    };
    let library = analyze_bytes(&sample, &declared, &bundle, &Limits::default()).unwrap();
    assert_eq!(report, &library, "库与 CLI 业务报告必须一致");

    // 业务断言：参考差集可用、present 为推定已收集、缺席为推定遗漏。
    assert_eq!(
        report.capabilities.reference_difference.status,
        SupportStatus::Supported
    );
    assert_eq!(report.metadata_version.as_deref(), Some("0.1.0"));
    assert_eq!(report.reference_set_version.as_deref(), Some("1.0.0"));
    let likely_collected = report
        .assessments
        .iter()
        .find(|item| item.internal_id.to_string() == "0x0F28B6EC")
        .unwrap();
    assert_eq!(likely_collected.assessment, AssessmentKind::LikelyCollected);
    let likely_missing = report
        .assessments
        .iter()
        .find(|item| item.internal_id.to_string() == "0x0F28B6ED")
        .unwrap();
    assert_eq!(likely_missing.assessment, AssessmentKind::LikelyMissing);

    // 组门禁随当前报告评估：非零计数撤销整组检查。
    let caches = report
        .locations
        .iter()
        .find(|location| location.guide_group_id.as_deref() == Some("stella:san-juan-caches"))
        .unwrap();
    let actionability = caches.group_actionability.as_ref().unwrap();
    assert_eq!(
        actionability.kind,
        sottr_core::report::GroupActionKind::GeneralReference
    );
    assert!(
        actionability
            .reason_codes
            .iter()
            .any(|code| code == "GROUP_COUNT_NONZERO")
    );
    assert!(
        report
            .locations
            .iter()
            .all(|location| location.coordinates.is_none())
    );

    // 输入未被修改，输出目录不写入输入目录。
    assert_eq!(
        sha256_hex(&fs::read(input.join("save1.dat")).unwrap()),
        before
    );
    let input_entries: Vec<String> = fs::read_dir(&input)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(input_entries, vec!["save1.dat"]);

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn usage_and_configuration_errors_exit_with_two() {
    let root = unique_dir("usage");
    let input = root.join("input");
    let output = root.join("output");
    let metadata = metadata_dir();
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("save1.dat"), b"x").unwrap();

    // 无参数：用法 + 退出码 2。
    let no_args = run_cli(&[]);
    assert_eq!(no_args.status.code(), Some(2));
    assert!(stderr_text(&no_args).contains("用法"));

    // --help：退出码 0。
    let help = run_cli(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(stdout_text(&help).contains("analyze"));

    let to_strings =
        |parts: &[&str]| -> Vec<String> { parts.iter().map(|part| (*part).to_string()).collect() };
    let input_str = input.to_str().unwrap();
    let output_str = output.to_str().unwrap();
    let metadata_str = metadata.to_str().unwrap();
    let missing_input = root.join("missing");
    let missing_metadata = root.join("missing-meta");
    let cases: Vec<Vec<String>> = vec![
        // 非法上下文。
        to_strings(&[
            "analyze",
            "--input",
            input_str,
            "--context",
            "profile",
            "--output",
            output_str,
            "--metadata",
            metadata_str,
        ]),
        // 缺少 --metadata。
        to_strings(&[
            "analyze",
            "--input",
            input_str,
            "--context",
            "campaign",
            "--output",
            output_str,
        ]),
        // 输入不存在。
        to_strings(&[
            "analyze",
            "--input",
            missing_input.to_str().unwrap(),
            "--context",
            "campaign",
            "--output",
            output_str,
            "--metadata",
            metadata_str,
        ]),
        // 元数据目录不存在。
        to_strings(&[
            "analyze",
            "--input",
            input_str,
            "--context",
            "campaign",
            "--output",
            output_str,
            "--metadata",
            missing_metadata.to_str().unwrap(),
        ]),
        // 未知命令。
        to_strings(&["unknown-command"]),
    ];
    for case in &cases {
        let args: Vec<&str> = case.iter().map(String::as_str).collect();
        let result = run_cli(&args);
        assert_eq!(result.status.code(), Some(2), "args={case:?}");
    }

    // 输出目录已存在：配置错误。
    fs::create_dir_all(&output).unwrap();
    let existing = run_cli(&[
        "analyze",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        output.to_str().unwrap(),
        "--metadata",
        metadata.to_str().unwrap(),
    ]);
    assert_eq!(existing.status.code(), Some(2));
    assert!(stderr_text(&existing).contains("输出目录已存在"));

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn batch_limit_is_enforced() {
    let root = unique_dir("limit");
    let input = root.join("input");
    fs::create_dir_all(&input).unwrap();
    for index in 0..=128 {
        fs::write(input.join(format!("save{index:03}.dat")), b"x").unwrap();
    }
    let result = run_cli(&[
        "analyze",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        root.join("output").to_str().unwrap(),
        "--metadata",
        metadata_dir().to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr_text(&result).contains("128"));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn mixed_batch_keeps_per_file_errors_and_profile_exclusion() {
    let root = unique_dir("mixed");
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).unwrap();
    let bundle = load_metadata_bundle();
    fs::write(input.join("save1.dat"), consistent_sample(&bundle)).unwrap();
    fs::write(input.join("broken.dat"), b"not a zlib stream").unwrap();
    fs::write(input.join("profile.dat"), compress(&vec![0u8; 25_600])).unwrap();

    let result = run_cli(&[
        "analyze",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        output.to_str().unwrap(),
        "--metadata",
        metadata_dir().to_str().unwrap(),
    ]);
    // 存在坏文件：退出码 1，但完整批量 JSON 仍写出。
    assert_eq!(
        result.status.code(),
        Some(1),
        "stderr: {}",
        stderr_text(&result)
    );

    let json = fs::read(output.join("analysis.json")).unwrap();
    let batch: BatchReport = serde_json::from_slice(&json).unwrap();
    assert_eq!(batch.results.len(), 3);

    let broken = batch
        .results
        .iter()
        .find(|entry| entry.input_ref == "broken.dat")
        .unwrap();
    let error = broken.error.as_ref().unwrap();
    assert_eq!(error.code, "DECOMPRESSION_FAILED");
    assert!(!error.retryable);
    assert!(broken.report.is_none());

    let profile = batch
        .results
        .iter()
        .find(|entry| entry.input_ref == "profile.dat")
        .unwrap();
    let profile_report = profile.report.as_ref().unwrap();
    assert_eq!(
        profile_report.context.mode,
        Mode::Profile,
        "批量 campaign 不得覆盖 profile 排除"
    );
    assert!(profile_report.observations.inventory.records.is_empty());
    assert_eq!(
        profile_report.capabilities.sections.status,
        SupportStatus::Unsupported
    );

    let good = batch
        .results
        .iter()
        .find(|entry| entry.input_ref == "save1.dat")
        .unwrap();
    assert!(good.report.is_some());
    assert!(good.error.is_none());

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn oversized_input_reports_resource_limit_per_file() {
    let root = unique_dir("oversize");
    let input = root.join("input");
    fs::create_dir_all(&input).unwrap();
    let big = input.join("big.dat");
    let file = fs::File::create(&big).unwrap();
    file.set_len(65 * 1024 * 1024).unwrap();
    drop(file);

    let result = run_cli(&[
        "analyze",
        "--input",
        input.to_str().unwrap(),
        "--context",
        "campaign",
        "--output",
        root.join("output").to_str().unwrap(),
        "--metadata",
        metadata_dir().to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(1));
    let batch: BatchReport =
        serde_json::from_slice(&fs::read(root.join("output/analysis.json")).unwrap()).unwrap();
    let error = batch.results[0].error.as_ref().unwrap();
    assert_eq!(error.code, "RESOURCE_LIMIT");
    assert!(!error.retryable);

    fs::remove_dir_all(&root).unwrap();
}
