//! SX-010 Python-Rust 差分回归（`cargo test -p sottr-core --test parity`）。
//!
//! Python v0.2（仓库内研究参照副本）作为正常已知输入的参照；其原始输出保留在
//! 输出目录，规范化适配器（`adapter.rs`）逐层对照并生成差异报告。
//!
//! 环境变量：
//! - `SOTTR_SAMPLE_DIR`：私有五文件目录（profile.dat、save1.dat、save202.dat、
//!   save204.dat、save206.dat）。未设置时跳过私有回归（记为 skip，不经过）。
//! - `SOTTR_PUBLIC_DIR`：按来源恢复的公开样本目录（三个归档的 .dat/.txt 成员；
//!   详见 `fixtures/public_samples.json` 与 `build_public_inventory.py`）。
//!   未设置时跳过公开覆盖并说明缺口（20 个战役样本未验证）。
//! - `SOTTR_PARITY_OUT_DIR`：差分报告与 Python 原始输出的基目录；未设置时写入
//!   `target/tmp/parity`。私有回归应指向仓库外目录（报告含私有样本哈希）。
//! - `SOTTR_PYTHON`：Python 解释器；默认探测 `py -3`、`python3`、`python`。
//!
//! 输出：`<基目录>/<集合>/parity_report.json`、`parity_summary.txt` 与
//! `<基目录>/<集合>/python-v02/`（Python 原始产物：report.json、CSV、日志）。

mod adapter;
mod fixtures;
mod python;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use sottr_core::container::decompress;
use sottr_core::hash::sha256_hex;
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};

use adapter::{
    Catalog, FixtureHashes, FixtureInput, FixtureReport, GroupReport, ScopeDoc, check_python_run,
    compare_duplicates, compare_fixture, scope_exclusions, skipped_fixture,
};
use fixtures::{Expectations, FixtureKind, PrivateFixture, PublicSample};

// ---------------------------------------------------------------------------
// 报告结构
// ---------------------------------------------------------------------------

/// 一次差分回归的完整报告。
#[derive(Serialize)]
struct ParityReport {
    /// 说明。
    note: &'static str,
    /// 集合名（private / public）。
    run: String,
    /// 输入目录（本地产物；不应外发）。
    source_dir: String,
    /// Python 参考运行信息。
    python: PythonDoc,
    /// 参照字典信息。
    catalog: CatalogDoc,
    /// 对照范围与排除说明。
    scope: ScopeDoc,
    /// Python 报告形状与运行自检。
    python_run: GroupReport,
    /// 逐夹具对照。
    fixtures: Vec<FixtureReport>,
    /// 跨文件重复组对照。
    cross_file: BTreeMap<String, GroupReport>,
    /// 覆盖说明（未验证文件、未提供样本、名称模式不匹配等）。
    coverage_notes: Vec<String>,
    /// 汇总。
    summary: Summary,
}

/// Python 参考运行信息。
#[derive(Serialize)]
struct PythonDoc {
    /// 解释器程序。
    program: String,
    /// 版本输出。
    version: String,
    /// 实际命令行。
    command_line: Vec<String>,
    /// 原始输出目录。
    output_dir: String,
}

/// 参照字典信息。
#[derive(Serialize)]
struct CatalogDoc {
    /// 字典路径。
    file: String,
    /// 字典 SHA-256。
    sha256: String,
}

/// 汇总计数。
#[derive(Serialize, Default)]
struct Summary {
    /// 清单条目数。
    fixtures_listed: usize,
    /// 实际对照数。
    compared: usize,
    /// 跳过数。
    skipped: usize,
    /// 战役夹具中已对照数。
    campaign_verified: usize,
    /// 战役夹具通过数（无未解释差异、无期望失败）。
    campaign_passed: usize,
    /// 战役夹具失败数。
    campaign_failed: usize,
    /// 资料夹具已对照数。
    profile_compared: usize,
    /// 总差异数。
    total_differences: usize,
    /// 未解释差异数。
    unexplained_differences: usize,
    /// 业务期望失败数。
    expectation_failures: usize,
}

/// 运行结果。
struct Outcome {
    /// 报告文件路径。
    report_path: PathBuf,
    /// 完整报告。
    report: ParityReport,
}

// ---------------------------------------------------------------------------
// 路径与环境
// ---------------------------------------------------------------------------

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tool_dir() -> PathBuf {
    repo_root().join("research/python-v0.2")
}

fn catalog_path() -> PathBuf {
    tool_dir().join("data/collectible_id_candidates.json")
}

fn metadata_dir() -> PathBuf {
    repo_root().join("data/metadata")
}

fn metadata_bundle() -> MetadataBundle {
    let dir = metadata_dir();
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("data/metadata 必须存在")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json") && name != "manifest.json")
        .collect();
    names.sort();
    let contents: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|name| {
            (
                name.clone(),
                fs::read(dir.join(name)).expect("元数据文件必须可读"),
            )
        })
        .collect();
    let inputs: Vec<MetadataFileInput<'_>> = contents
        .iter()
        .map(|(file_id, bytes)| MetadataFileInput {
            file_id: file_id.clone(),
            bytes,
        })
        .collect();
    MetadataBundle::from_files(&inputs, &MetadataLimits::default()).expect("元数据必须可加载")
}

fn out_base() -> PathBuf {
    let base = env_path("SOTTR_PARITY_OUT_DIR")
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("parity"));
    fs::create_dir_all(&base).expect("创建输出基目录");
    base
}

fn prepare_run_dir(base: &Path, set: &str) -> PathBuf {
    let dir = base.join(set);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("清理旧的回归输出目录");
    }
    fs::create_dir_all(&dir).expect("创建回归输出目录");
    dir
}

// ---------------------------------------------------------------------------
// 通用工具
// ---------------------------------------------------------------------------

fn find_python_record<'a>(
    report: &'a Value,
    name: &str,
    compressed_sha256: &str,
) -> Option<&'a Value> {
    report["known_observations"]["inventory"]
        .as_array()?
        .iter()
        .find(|record| {
            record["name"].as_str() == Some(name)
                && record["compressed_sha256"].as_str() == Some(compressed_sha256)
        })
}

fn fixture_hashes(relative_path: &str, source_path: &Path, bytes: &[u8]) -> FixtureHashes {
    let decompression = decompress(bytes, &Limits::default());
    FixtureHashes {
        relative_path: relative_path.to_string(),
        source_path: source_path.to_path_buf(),
        compressed_sha256: sha256_hex(bytes),
        expanded_sha256: if decompression.valid {
            Some(sha256_hex(&decompression.data))
        } else {
            None
        },
    }
}

fn summarize(
    fixtures: &[FixtureReport],
    python_run: &GroupReport,
    cross_file: &BTreeMap<String, GroupReport>,
) -> Summary {
    let mut summary = Summary {
        fixtures_listed: fixtures.len(),
        ..Summary::default()
    };
    for fixture in fixtures {
        if fixture.compared() {
            summary.compared += 1;
            match fixture.kind {
                "campaign" => {
                    summary.campaign_verified += 1;
                    if fixture.unexplained() == 0 && fixture.expectation_failures() == 0 {
                        summary.campaign_passed += 1;
                    } else {
                        summary.campaign_failed += 1;
                    }
                }
                "profile" => summary.profile_compared += 1,
                _ => {}
            }
        } else {
            summary.skipped += 1;
        }
        summary.total_differences += fixture.differences();
        summary.unexplained_differences += fixture.unexplained();
        summary.expectation_failures += fixture.expectation_failures();
    }
    summary.total_differences += python_run.differences.len();
    summary.unexplained_differences += python_run
        .differences
        .iter()
        .filter(|difference| difference.adjudication.is_none())
        .count();
    for group in cross_file.values() {
        summary.total_differences += group.differences.len();
        summary.unexplained_differences += group
            .differences
            .iter()
            .filter(|difference| difference.adjudication.is_none())
            .count();
    }
    summary
}

fn summary_text(report: &ParityReport) -> String {
    let mut text = String::new();
    text.push_str(&format!("SX-010 Python v0.2 差分回归（{}）\n", report.run));
    text.push_str(&format!(
        "Python：{}（{}）\n",
        report.python.program, report.python.version
    ));
    text.push_str(&format!("输入：{}\n", report.source_dir));
    let summary = &report.summary;
    text.push_str(&format!(
        "清单 {} 条：对照 {}、跳过 {}；战役通过 {}/{}、资料对照 {}；\
         差异 {}（未解释 {}）、期望失败 {}\n",
        summary.fixtures_listed,
        summary.compared,
        summary.skipped,
        summary.campaign_passed,
        summary.campaign_verified,
        summary.profile_compared,
        summary.total_differences,
        summary.unexplained_differences,
        summary.expectation_failures,
    ));
    for fixture in &report.fixtures {
        let status = if fixture.compared() {
            format!("对照（差异 {}）", fixture.differences())
        } else {
            format!(
                "跳过（{}）",
                fixture.skip_reason.as_deref().unwrap_or("原因未记录")
            )
        };
        text.push_str(&format!(
            "  {:<28} {:<9} {}\n",
            fixture.name, fixture.kind, status
        ));
        for note in &fixture.notes {
            text.push_str(&format!("    · {note}\n"));
        }
    }
    for (name, group) in &report.cross_file {
        text.push_str(&format!(
            "跨文件 {name}：差异 {}\n",
            group.differences.len()
        ));
    }
    text.push_str(&format!(
        "Python 运行自检差异：{}\n",
        report.python_run.differences.len()
    ));
    for note in &report.coverage_notes {
        text.push_str(&format!("覆盖说明：{note}\n"));
    }
    text
}

fn write_report(run_dir: &Path, report: &ParityReport) -> PathBuf {
    let path = run_dir.join("parity_report.json");
    let mut json = serde_json::to_vec_pretty(report).expect("差分报告必须可序列化");
    json.push(b'\n');
    fs::write(&path, json).expect("写出差分报告");
    fs::write(run_dir.join("parity_summary.txt"), summary_text(report)).expect("写出摘要");
    path
}

// ---------------------------------------------------------------------------
// 私有回归
// ---------------------------------------------------------------------------

fn run_private(samples_dir: &Path, specs: &[PrivateFixture]) -> Outcome {
    let bundle = metadata_bundle();
    let catalog = Catalog::load(&catalog_path()).expect("字典必须可加载");
    let base = out_base();
    let run_dir = prepare_run_dir(&base, "private");
    let python_out = run_dir.join("python-v02");

    let run = python::run_v02(&tool_dir(), samples_dir, &python_out, &catalog_path())
        .unwrap_or_else(|error| panic!("{error}"));
    let python_run = check_python_run(&run.report);

    let mut fixtures: Vec<FixtureReport> = Vec::new();
    let mut hashes: Vec<FixtureHashes> = Vec::new();
    for spec in specs {
        let path = samples_dir.join(&spec.name);
        let bytes = fs::read(&path).expect("已检查私有夹具存在");
        let sha = sha256_hex(&bytes);
        match find_python_record(&run.report, &spec.name, &sha) {
            Some(record) => {
                fixtures.push(compare_fixture(
                    &FixtureInput {
                        name: &spec.name,
                        relative_path: &spec.name,
                        kind: spec.kind,
                        bytes: &bytes,
                        expect: &spec.expect,
                    },
                    record,
                    &bundle,
                    &catalog,
                ));
                hashes.push(fixture_hashes(&spec.name, &path, &bytes));
            }
            None => fixtures.push(skipped_fixture(
                &spec.name,
                &spec.name,
                spec.kind,
                "Python v0.2 输出中无匹配记录（名称 + 压缩哈希）",
            )),
        }
    }

    let duplicates = compare_duplicates(&hashes, &run.report);
    let report = ParityReport {
        note: "SX-010 私有差分回归：Python v0.2（研究参照副本）为参照；私有样本不随仓库分发，本报告含私有样本哈希，禁止外发。",
        run: "private".to_string(),
        source_dir: samples_dir.display().to_string(),
        python: PythonDoc {
            program: run.interpreter.program.clone(),
            version: run.interpreter.version.clone(),
            command_line: run.command_line.clone(),
            output_dir: run.output_dir.display().to_string(),
        },
        catalog: CatalogDoc {
            file: catalog_path().display().to_string(),
            sha256: catalog.sha256.clone(),
        },
        scope: scope_exclusions(),
        python_run: python_run.clone(),
        cross_file: duplicates.groups.clone(),
        coverage_notes: duplicates.notes.clone(),
        summary: summarize(&fixtures, &python_run, &duplicates.groups),
        fixtures,
    };
    let report_path = write_report(&run_dir, &report);
    Outcome {
        report_path,
        report,
    }
}

// ---------------------------------------------------------------------------
// 公开回归
// ---------------------------------------------------------------------------

fn discover_dat_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, files: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        for path in paths {
            let Ok(file_type) = fs::symlink_metadata(&path) else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if depth < 4 {
                    walk(&path, depth + 1, files);
                }
            } else if file_type.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("dat"))
            {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(root, 0, &mut files);
    files.sort();
    files
}

fn expectations_from_public(sample: &PublicSample) -> Expectations {
    Expectations {
        overall_f32: sample.completion_candidate,
        expanded_bytes: sample.raw_length,
        prefix_nonzero: sample.prefix_nonzero,
        prefix_known: sample.prefix_known,
        raw_sha256: sample.raw_sha256.clone(),
        ..Expectations::default()
    }
}

fn run_public(public_dir: &Path) -> Outcome {
    let inventory = fixtures::public_inventory();
    let bundle = metadata_bundle();
    let catalog = Catalog::load(&catalog_path()).expect("字典必须可加载");
    let base = out_base();
    let run_dir = prepare_run_dir(&base, "public");
    let python_out = run_dir.join("python-v02");

    // 公开清单来源自检（URL / ZIP 哈希 / 成员）：不一致即记录覆盖说明。
    let mut coverage_notes: Vec<String> = Vec::new();
    let mut provenance_ok = 0usize;
    for archive in &inventory.archives {
        if archive.url.is_empty() || archive.sha256.len() != 64 || archive.bytes == 0 {
            coverage_notes.push(format!("归档来源字段不完整：{}", archive.name));
        }
    }
    for sample in &inventory.samples {
        match inventory
            .archives
            .iter()
            .find(|archive| archive.name == sample.archive)
        {
            Some(archive) if archive.members.iter().any(|member| member == &sample.name) => {
                provenance_ok += 1;
            }
            Some(archive) => coverage_notes.push(format!(
                "清单不一致：{} 不在归档 {} 的成员清单",
                sample.name, archive.name
            )),
            None => {
                coverage_notes.push(format!("清单不一致：归档 {} 未在来源清单", sample.archive))
            }
        }
    }
    coverage_notes.push(format!(
        "公开清单自检：{} 条样本对应来源归档与成员；{} 个归档、ZIP 合计 {} 字节；\
         ZIP 哈希与逐成员核对见恢复清单（仓库外 private/s010_parity/public_restore_manifest.json）",
        provenance_ok,
        inventory.archives.len(),
        inventory
            .archives
            .iter()
            .map(|archive| archive.bytes)
            .sum::<u64>()
    ));

    // 发现并按（名称 + 压缩哈希）核对清单；未验证文件不参与对照。
    let discovered = discover_dat_files(public_dir);
    let mut claimed = vec![false; inventory.samples.len()];
    let mut verified: Vec<Option<(PathBuf, Vec<u8>, String)>> = vec![None; inventory.samples.len()];
    for path in &discovered {
        let Ok(bytes) = fs::read(path) else {
            coverage_notes.push(format!("读取失败（不计通过）：{}", path.display()));
            continue;
        };
        let sha = sha256_hex(&bytes);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let claimed_index = inventory
            .samples
            .iter()
            .enumerate()
            .position(|(index, sample)| {
                !claimed[index]
                    && sample.name == name
                    && match &sample.compressed_sha256 {
                        Some(expected) => expected == &sha,
                        None => sample.kind == FixtureKind::Profile,
                    }
            });
        match claimed_index {
            Some(index) => {
                claimed[index] = true;
                verified[index] = Some((path.clone(), bytes, sha));
            }
            None => coverage_notes.push(format!(
                "未验证文件（不在清单或已被认领，不计通过）：{}",
                path.strip_prefix(public_dir).unwrap_or(path).display()
            )),
        }
    }

    let run = python::run_v02(&tool_dir(), public_dir, &python_out, &catalog_path())
        .unwrap_or_else(|error| panic!("{error}"));
    let python_run = check_python_run(&run.report);

    let mut fixtures: Vec<FixtureReport> = Vec::new();
    let mut hashes: Vec<FixtureHashes> = Vec::new();
    for (index, sample) in inventory.samples.iter().enumerate() {
        let Some((path, bytes, sha)) = &verified[index] else {
            let reason = match sample.kind {
                FixtureKind::ChallengeTomb => {
                    "Python v0.2 名称模式不匹配（profile/save<number>）；不在本轮对照"
                }
                FixtureKind::Profile => "未在目录中发现（资料文件按名称匹配）",
                FixtureKind::Campaign => "未在目录中发现或哈希不符（不计通过）",
            };
            fixtures.push(skipped_fixture(
                &sample.name,
                &sample.relative_path,
                sample.kind,
                reason,
            ));
            continue;
        };
        if sample.kind == FixtureKind::ChallengeTomb {
            fixtures.push(skipped_fixture(
                &sample.name,
                &sample.relative_path,
                sample.kind,
                "Python v0.2 名称模式不匹配（profile/save<number>）；不在本轮对照",
            ));
            continue;
        }
        match find_python_record(&run.report, &sample.name, sha) {
            Some(record) => {
                let expect = expectations_from_public(sample);
                fixtures.push(compare_fixture(
                    &FixtureInput {
                        name: &sample.name,
                        relative_path: &sample.relative_path,
                        kind: sample.kind,
                        bytes,
                        expect: &expect,
                    },
                    record,
                    &bundle,
                    &catalog,
                ));
                hashes.push(fixture_hashes(&sample.relative_path, path, bytes));
            }
            None => fixtures.push(skipped_fixture(
                &sample.name,
                &sample.relative_path,
                sample.kind,
                "Python v0.2 输出中无匹配记录（名称 + 压缩哈希）",
            )),
        }
    }

    let duplicates = compare_duplicates(&hashes, &run.report);
    coverage_notes.extend(duplicates.notes.clone());
    if let Some(note) = inventory
        .samples
        .iter()
        .find(|sample| sample.kind == FixtureKind::ChallengeTomb)
        .and_then(|sample| sample.note.clone())
    {
        coverage_notes.push(note);
    }

    let report = ParityReport {
        note: "SX-010 公开差分回归：20 个公开战役样本为按来源恢复的公开下载（哈希核对）；样本字节不随仓库分发；未提供样本按覆盖缺口记录。",
        run: "public".to_string(),
        source_dir: public_dir.display().to_string(),
        python: PythonDoc {
            program: run.interpreter.program.clone(),
            version: run.interpreter.version.clone(),
            command_line: run.command_line.clone(),
            output_dir: run.output_dir.display().to_string(),
        },
        catalog: CatalogDoc {
            file: catalog_path().display().to_string(),
            sha256: catalog.sha256.clone(),
        },
        scope: scope_exclusions(),
        python_run: python_run.clone(),
        cross_file: duplicates.groups.clone(),
        coverage_notes,
        summary: summarize(&fixtures, &python_run, &duplicates.groups),
        fixtures,
    };
    let report_path = write_report(&run_dir, &report);
    Outcome {
        report_path,
        report,
    }
}

// ---------------------------------------------------------------------------
// 测试入口
// ---------------------------------------------------------------------------

#[test]
fn python_v02_parity_private_samples() {
    let Some(samples_dir) = env_path("SOTTR_SAMPLE_DIR") else {
        eprintln!("SOTTR_SAMPLE_DIR 未设置：跳过 Python v0.2 私有差分回归（记为 skip，不经过）");
        return;
    };
    let specs = fixtures::private_fixtures();
    let missing: Vec<&str> = specs
        .iter()
        .filter(|spec| !samples_dir.join(&spec.name).is_file())
        .map(|spec| spec.name.as_str())
        .collect();
    assert!(
        missing.is_empty(),
        "私有差分回归缺少必需夹具（必须提供 profile.dat 与四个战役文件）：{}",
        missing.join("、")
    );

    let outcome = run_private(&samples_dir, &specs);
    println!("{}", summary_text(&outcome.report));
    println!("报告：{}", outcome.report_path.display());
    let summary = &outcome.report.summary;
    assert_eq!(
        summary.campaign_passed,
        4,
        "四个私人战役文件必须全部通过：{}",
        outcome.report_path.display()
    );
    assert_eq!(
        summary.unexplained_differences,
        0,
        "存在未解释差异：{}",
        outcome.report_path.display()
    );
    assert_eq!(
        summary.expectation_failures,
        0,
        "业务期望失败：{}",
        outcome.report_path.display()
    );
    assert_eq!(summary.campaign_failed, 0);
    assert!(
        outcome
            .report
            .fixtures
            .iter()
            .any(|fixture| fixture.kind == "profile" && fixture.compared()),
        "profile 必须按容器/哈希/原始观察对照"
    );
}

#[test]
fn python_v02_parity_public_samples() {
    let Some(public_dir) = env_path("SOTTR_PUBLIC_DIR") else {
        eprintln!(
            "SOTTR_PUBLIC_DIR 未设置：公开 20 样本未按来源恢复，跳过并记录覆盖缺口\
             （覆盖缺口：20 个公开战役样本未验证；恢复方法见 fixtures/public_samples.json）"
        );
        return;
    };
    assert!(
        public_dir.is_dir(),
        "SOTTR_PUBLIC_DIR 不是目录：{}",
        public_dir.display()
    );

    let outcome = run_public(&public_dir);
    println!("{}", summary_text(&outcome.report));
    println!("报告：{}", outcome.report_path.display());
    let summary = &outcome.report.summary;
    if summary.campaign_verified == 0 {
        eprintln!(
            "目录中未找到清单内公开战役样本（20 个）：按 skip 处理并记录覆盖缺口\
             （若确无公开夹具，请勿设置 SOTTR_PUBLIC_DIR）"
        );
        return;
    }
    assert_eq!(
        summary.campaign_failed,
        0,
        "存在未通过的公开样本：{}",
        outcome.report_path.display()
    );
    assert_eq!(
        summary.unexplained_differences,
        0,
        "存在未解释差异：{}",
        outcome.report_path.display()
    );
    assert_eq!(
        summary.expectation_failures,
        0,
        "业务期望失败：{}",
        outcome.report_path.display()
    );
    eprintln!(
        "公开样本复现：{}/20 战役通过（未提供或未验证的样本按覆盖缺口记录）",
        summary.campaign_passed
    );
}
