//! sottr-cli：`analyze` 与 `export` 命令（SX-009 / SX-011）。
//!
//! 用法（最终语法由任务固定，见 `--help`）：
//!
//! ```text
//! sottr-cli analyze --input <文件或目录> --context <campaign|unknown> \
//!     --output <新目录> --metadata <元数据目录>
//! sottr-cli export --input <文件> --context <campaign|unknown> \
//!     --output <新目录> --metadata <元数据目录> [--format json|csv|both] [--share] \
//!     [--snapshot-id <ID>] [--snapshot-time <RFC3339>] [--stale --refresh-error <CODE>]
//! ```
//!
//! 边界与行为：
//! - 目录读取、文件读取与输出写入由本适配层承担；核心只接收字节
//!   （`sottr_core::analyze::analyze_bytes`）；
//! - 输入/输出路径策略与桌面后端共用 `sottr_platform::path_policy`（SX-012）：
//!   只支持本地卷（UNC/设备命名空间明确拒绝），输出必须是不存在的新目录，
//!   与输入不得相等或互为父子（按规范路径逐组件比较，不用字符串前缀）；
//! - 输入以稳定快照读取（SX-013，`sottr_platform::read_snapshot`）：句柄约束下
//!   读取并做读取前后指纹/摘要复核，被占用返回 INPUT_UNREADABLE（可重试）、
//!   读取中变化返回 INPUT_CHANGED；输出用独占新目标与受控临时文件提交
//!   （`sottr_platform::output_commit`），失败/取消不留“完整报告”假象；
//! - 批量最多 128 个文件；一个坏文件保留自己的输入级错误，不影响其他文件；
//! - `profile.dat` 始终按资料文件处理（仅容器观察），不受 `--context` 影响；
//! - `--context campaign` 是用户声明，不是格式识别证明；
//! - 纯分析不做全文件两两比较；不写回、不修改任何输入文件；
//! - 输出目录必须不存在；分析成功与否都写出批量 JSON（除用法错误外）；
//! - export 写出带快照身份的导出信封（report.json）与 CSV（report.csv），
//!   分享模式去除绝对路径；产物先写临时文件再提交，失败/取消不留假象。
//!
//! 退出码：
//! - `0`：全部文件成功分析；或导出成功；
//! - `1`：分析完成但至少一个输入级错误（批量 JSON 仍含全部结果）；或导出的输入不可分析；
//! - `2`：用法或配置错误（参数、路径、输出已存在、重叠、批量超限、元数据加载失败）。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sottr_core::analyze::analyze_bytes;
use sottr_core::atlas;
use sottr_core::catalog::Catalog;
use sottr_core::domain::context::DeclaredContext;
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::missing::compute_missing;
use sottr_core::report::{
    BatchReport, BatchResult, ContextSource, ExportSnapshot, Freshness, InputError, Mode,
    SchemaVersion, export_document, is_reason_code, render_export_csv, render_export_json,
    scan_private_material, share_export_document,
};
use sottr_platform::{output_commit, path_policy, read_snapshot};

mod export_io;

/// 批量文件数量上限。
const BATCH_FILE_LIMIT: usize = 128;
/// analyze 输出文件名。
const OUTPUT_FILE_NAME: &str = "analysis.json";
/// export 输出文件名（JSON 信封）。
const EXPORT_JSON_NAME: &str = "report.json";
/// export 输出文件名（CSV）。
const EXPORT_CSV_NAME: &str = "report.csv";
/// 元数据目录中不参与加载的文件（清单不是数据集）。
const METADATA_MANIFEST: &str = "manifest.json";

const EXIT_OK: u8 = 0;
const EXIT_INPUT_ERROR: u8 = 1;
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("错误：{message}");
            eprintln!("运行 `sottr-cli --help` 查看用法。");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn run(args: &[String]) -> Result<u8, String> {
    if args.is_empty() {
        eprintln!("{USAGE}");
        return Ok(EXIT_USAGE);
    }
    if args[0] == "--help" || args[0] == "-h" {
        println!("{USAGE}");
        return Ok(EXIT_OK);
    }
    match args[0].as_str() {
        "analyze" => {
            if args[1..].iter().any(|arg| arg == "--help" || arg == "-h") {
                println!("{USAGE}");
                return Ok(EXIT_OK);
            }
            let parsed = parse_analyze_args(&args[1..])?;
            run_analyze(&parsed)
        }
        "export" => {
            if args[1..].iter().any(|arg| arg == "--help" || arg == "-h") {
                println!("{USAGE}");
                return Ok(EXIT_OK);
            }
            let parsed = parse_export_args(&args[1..])?;
            run_export(&parsed)
        }
        "missing" => {
            if args[1..].iter().any(|arg| arg == "--help" || arg == "-h") {
                println!("{USAGE}");
                return Ok(EXIT_OK);
            }
            let parsed = parse_missing_args(&args[1..])?;
            run_missing(&parsed)
        }
        "plan" => {
            if args[1..].iter().any(|arg| arg == "--help" || arg == "-h") {
                println!("{USAGE}");
                return Ok(EXIT_OK);
            }
            let parsed = parse_plan_args(&args[1..])?;
            run_plan(&parsed)
        }
        other => Err(format!(
            "未知命令 {other:?}（当前支持 analyze、export、missing 与 plan）"
        )),
    }
}

/// analyze 命令参数。
#[derive(Debug, Clone, PartialEq)]
struct AnalyzeArgs {
    input: PathBuf,
    context: Mode,
    output: PathBuf,
    metadata: PathBuf,
}

fn parse_analyze_args(args: &[String]) -> Result<AnalyzeArgs, String> {
    let mut input: Option<PathBuf> = None;
    let mut context: Option<Mode> = None;
    let mut output: Option<PathBuf> = None;
    let mut metadata: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("参数 {flag} 缺少取值"))?;
        match flag {
            "--input" => set_once(&mut input, PathBuf::from(value), "--input")?,
            "--output" => set_once(&mut output, PathBuf::from(value), "--output")?,
            "--metadata" => set_once(&mut metadata, PathBuf::from(value), "--metadata")?,
            "--context" => {
                let mode = match value.as_str() {
                    "campaign" => Mode::Campaign,
                    "unknown" => Mode::Unknown,
                    other => {
                        return Err(format!(
                            "--context 仅支持 campaign 或 unknown，收到 {other:?}"
                        ));
                    }
                };
                set_once(&mut context, mode, "--context")?;
            }
            other => return Err(format!("未知参数 {other:?}")),
        }
        index += 2;
    }

    Ok(AnalyzeArgs {
        input: input.ok_or("缺少 --input")?,
        context: context.ok_or("缺少 --context")?,
        output: output.ok_or("缺少 --output")?,
        metadata: metadata.ok_or("缺少 --metadata")?,
    })
}

fn set_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("参数 {name} 重复"));
    }
    *slot = Some(value);
    Ok(())
}

fn run_analyze(args: &AnalyzeArgs) -> Result<u8, String> {
    // 路径策略与桌面后端共用（SX-012）：本地位置、存在性与输入/输出重叠校验。
    path_policy::validate_input(&args.input).map_err(|error| error.message)?;
    if !args.metadata.is_dir() {
        return Err(format!("元数据路径不是目录：{}", args.metadata.display()));
    }
    path_policy::validate_output(&args.output, &[args.input.as_path()])
        .map_err(|error| error.message)?;

    let metadata = load_metadata(&args.metadata)?;
    let files = collect_inputs(&args.input)?;
    let limits = Limits::default();

    let mut results: Vec<BatchResult> = Vec::new();
    let mut error_count = 0usize;
    for path in &files {
        let input_ref = file_name(path);
        match analyze_file(path, args, &metadata, &limits) {
            Ok(report) => results.push(BatchResult {
                input_ref,
                report: Some(report),
                error: None,
            }),
            Err(error) => {
                error_count += 1;
                results.push(BatchResult {
                    input_ref,
                    report: None,
                    error: Some(error),
                });
            }
        }
    }

    let batch = BatchReport {
        schema_version: SchemaVersion,
        results,
    };
    let mut json = serde_json::to_string_pretty(&batch)
        .map_err(|error| format!("序列化批量报告失败：{error}"))?;
    json.push('\n');
    // 输出用独占新目录与受控临时文件提交（SX-013）：失败或取消不留部分结果。
    let staging = output_commit::prepare(&args.output, &[(OUTPUT_FILE_NAME.to_string(), json)])
        .map_err(|error| error.message)?;
    let files = staging.commit().map_err(|error| error.message)?;
    let output_path = files
        .into_iter()
        .next()
        .unwrap_or_else(|| args.output.join(OUTPUT_FILE_NAME));

    let total = batch.results.len();
    println!("输入：{}", args.input.display());
    println!(
        "文件：{total}（成功 {}，输入错误 {error_count}）",
        total - error_count
    );
    for result in &batch.results {
        match (&result.report, &result.error) {
            (Some(report), None) => println!(
                "  {:<40} ok（记录 {}，整体候选 {}）",
                result.input_ref,
                report.observations.inventory.record_count,
                report
                    .observations
                    .overall_completion
                    .value
                    .map_or("不可用".to_string(), |value| format!("{value:.6}%"))
            ),
            (None, Some(error)) => println!(
                "  {:<40} error（{}：{}）",
                result.input_ref, error.code, error.message
            ),
            _ => unreachable!("BatchResult 恰有一个非空"),
        }
    }
    println!("输出：{}", output_path.display());

    Ok(if error_count > 0 {
        EXIT_INPUT_ERROR
    } else {
        EXIT_OK
    })
}

fn load_metadata(directory: &Path) -> Result<MetadataBundle, String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .map_err(|error| format!("读取元数据目录失败（{}）：{error}", directory.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.to_ascii_lowercase().ends_with(".json"))
        .filter(|name| !name.eq_ignore_ascii_case(METADATA_MANIFEST))
        .collect();
    names.sort();
    if names.is_empty() {
        return Err(format!(
            "元数据目录中没有 JSON 数据文件：{}",
            directory.display()
        ));
    }
    let mut contents: Vec<(String, Vec<u8>)> = Vec::new();
    for name in &names {
        let bytes = fs::read(directory.join(name))
            .map_err(|error| format!("读取元数据文件失败（{name}）：{error}"))?;
        contents.push((name.clone(), bytes));
    }
    let inputs: Vec<MetadataFileInput<'_>> = contents
        .iter()
        .map(|(file_id, bytes)| MetadataFileInput {
            file_id: file_id.clone(),
            bytes,
        })
        .collect();
    MetadataBundle::from_files(&inputs, &MetadataLimits::default())
        .map_err(|error| format!("元数据加载失败（{}）：{}", error.code(), error.message()))
}

fn collect_inputs(input: &Path) -> Result<Vec<PathBuf>, String> {
    if input.is_file() {
        return Ok(vec![input.to_path_buf()]);
    }
    if !input.is_dir() {
        return Err(format!("输入既不是文件也不是目录：{}", input.display()));
    }
    let mut files: Vec<PathBuf> = fs::read_dir(input)
        .map_err(|error| format!("读取输入目录失败（{}）：{error}", input.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dat"))
        })
        .collect();
    files.sort_by_key(|path| file_name(path).to_ascii_lowercase());
    if files.is_empty() {
        return Err(format!("输入目录中没有 .dat 文件：{}", input.display()));
    }
    if files.len() > BATCH_FILE_LIMIT {
        return Err(format!(
            "批量最多 {BATCH_FILE_LIMIT} 个文件，目录中发现 {} 个",
            files.len()
        ));
    }
    Ok(files)
}

fn analyze_file(
    path: &Path,
    args: &AnalyzeArgs,
    metadata: &MetadataBundle,
    limits: &Limits,
) -> Result<sottr_core::report::Report, InputError> {
    let name = file_name(path);
    // 稳定快照（SX-013）：句柄约束读取（Windows 读取期间拒绝写入/删除/改名）、
    // 读取前后指纹与内容摘要复核、路径身份复核；超限在被读取前拒绝。
    let snapshot = read_snapshot::read_stable(path, limits)?;
    let declared = declared_context(&name, args.context);
    analyze_bytes(&snapshot.bytes, &declared, metadata, limits)
}

/// 逐文件上下文：`profile.dat` 始终按资料文件处理（批量 --context 不得覆盖该排除）。
fn declared_context(name: &str, context: Mode) -> DeclaredContext {
    let mode = if name.eq_ignore_ascii_case("profile.dat") {
        Mode::Profile
    } else {
        context
    };
    DeclaredContext {
        platform: None,
        mode,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    }
}

// ---------------------------------------------------------------------------
// export 命令（SX-011）
// ---------------------------------------------------------------------------

/// 导出格式选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Json,
    Csv,
    Both,
}

impl ExportFormat {
    fn includes_json(self) -> bool {
        matches!(self, ExportFormat::Json | ExportFormat::Both)
    }

    fn includes_csv(self) -> bool {
        matches!(self, ExportFormat::Csv | ExportFormat::Both)
    }

    fn label(self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Both => "json+csv",
        }
    }
}

/// export 命令参数。
#[derive(Debug, Clone, PartialEq)]
struct ExportArgs {
    input: PathBuf,
    context: Mode,
    output: PathBuf,
    metadata: PathBuf,
    format: ExportFormat,
    share: bool,
    snapshot_id: Option<String>,
    snapshot_time: Option<String>,
    stale: bool,
    refresh_error: Option<String>,
}

fn parse_export_args(args: &[String]) -> Result<ExportArgs, String> {
    let mut input: Option<PathBuf> = None;
    let mut context: Option<Mode> = None;
    let mut output: Option<PathBuf> = None;
    let mut metadata: Option<PathBuf> = None;
    let mut format: Option<ExportFormat> = None;
    let mut share = false;
    let mut snapshot_id: Option<String> = None;
    let mut snapshot_time: Option<String> = None;
    let mut stale = false;
    let mut refresh_error: Option<String> = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--share" {
            if share {
                return Err("参数 --share 重复".to_string());
            }
            share = true;
            index += 1;
            continue;
        }
        if flag == "--stale" {
            if stale {
                return Err("参数 --stale 重复".to_string());
            }
            stale = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("参数 {flag} 缺少取值"))?;
        match flag {
            "--input" => set_once(&mut input, PathBuf::from(value), "--input")?,
            "--output" => set_once(&mut output, PathBuf::from(value), "--output")?,
            "--metadata" => set_once(&mut metadata, PathBuf::from(value), "--metadata")?,
            "--context" => {
                let mode = match value.as_str() {
                    "campaign" => Mode::Campaign,
                    "unknown" => Mode::Unknown,
                    other => {
                        return Err(format!(
                            "--context 仅支持 campaign 或 unknown，收到 {other:?}"
                        ));
                    }
                };
                set_once(&mut context, mode, "--context")?;
            }
            "--format" => {
                let parsed = match value.as_str() {
                    "json" => ExportFormat::Json,
                    "csv" => ExportFormat::Csv,
                    "both" => ExportFormat::Both,
                    other => {
                        return Err(format!("--format 仅支持 json、csv 或 both，收到 {other:?}"));
                    }
                };
                set_once(&mut format, parsed, "--format")?;
            }
            "--snapshot-id" => {
                if value.is_empty() {
                    return Err("--snapshot-id 不能为空".to_string());
                }
                set_once(&mut snapshot_id, value.to_string(), "--snapshot-id")?;
            }
            "--snapshot-time" => {
                export_io::validate_rfc3339(value)?;
                set_once(&mut snapshot_time, value.to_string(), "--snapshot-time")?;
            }
            "--refresh-error" => {
                if !is_reason_code(value) {
                    return Err(format!(
                        "--refresh-error 必须形如 ^[A-Z][A-Z0-9_]*$，收到 {value:?}"
                    ));
                }
                set_once(&mut refresh_error, value.to_string(), "--refresh-error")?;
            }
            other => return Err(format!("未知参数 {other:?}")),
        }
        index += 2;
    }

    if stale && refresh_error.is_none() {
        return Err(
            "--stale 必须与 --refresh-error <CODE> 一起使用（旧结果导出必须标记刷新失败原因）"
                .to_string(),
        );
    }
    if !stale && refresh_error.is_some() {
        return Err("--refresh-error 仅与 --stale 一起使用（当前导出为 current）".to_string());
    }

    Ok(ExportArgs {
        input: input.ok_or("缺少 --input")?,
        context: context.ok_or("缺少 --context")?,
        output: output.ok_or("缺少 --output")?,
        metadata: metadata.ok_or("缺少 --metadata")?,
        format: format.unwrap_or(ExportFormat::Both),
        share,
        snapshot_id,
        snapshot_time,
        stale,
        refresh_error,
    })
}

fn run_export(args: &ExportArgs) -> Result<u8, String> {
    // 路径策略与桌面后端共用（SX-012）：输入本地位置校验 + 输出重叠校验。
    let canonical_input =
        path_policy::validate_input(&args.input).map_err(|error| error.message)?;
    if !canonical_input.is_file() {
        return Err(format!(
            "export 只接受单个存档文件：{}",
            args.input.display()
        ));
    }
    if !args.metadata.is_dir() {
        return Err(format!("元数据路径不是目录：{}", args.metadata.display()));
    }
    path_policy::validate_output(&args.output, &[args.input.as_path()])
        .map_err(|error| error.message)?;

    let metadata = load_metadata(&args.metadata)?;
    let limits = Limits::default();
    let name = file_name(&args.input);
    // 稳定快照（SX-013）：被占用返回 INPUT_UNREADABLE（可重试）、读取中变化返回
    // INPUT_CHANGED、超限返回 RESOURCE_LIMIT；不产生部分结果。
    let snapshot = match read_snapshot::read_stable(&args.input, &limits) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("错误：{name} 无法读取：{}（{}）", error.message, error.code);
            return Ok(EXIT_INPUT_ERROR);
        }
    };
    let declared = declared_context(&name, args.context);
    let report = match analyze_bytes(&snapshot.bytes, &declared, &metadata, &limits) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("错误：{name} 无法分析：{}（{}）", error.message, error.code);
            return Ok(EXIT_INPUT_ERROR);
        }
    };

    let millis = export_io::unix_millis_now();
    let snapshot_id = match &args.snapshot_id {
        Some(id) => id.clone(),
        None => format!("snap-{millis}"),
    };
    let snapshot_time = match &args.snapshot_time {
        Some(time) => time.clone(),
        None => export_io::format_rfc3339_utc_millis(millis),
    };
    let freshness = if args.stale {
        Freshness::Stale
    } else {
        Freshness::Current
    };
    let document = export_document(
        report,
        ExportSnapshot {
            input_ref: &name,
            snapshot_id: &snapshot_id,
            snapshot_time: &snapshot_time,
            freshness,
            last_refresh_error_code: args.refresh_error.as_deref(),
        },
    )
    .map_err(|error| format!("导出信封组装失败：{error}"))?;
    let document = if args.share {
        share_export_document(&document).map_err(|error| format!("分享脱敏失败：{error}"))?
    } else {
        document
    };

    println!(
        "导出计划：格式 {}；脱敏 {}；目标 {}",
        args.format.label(),
        if args.share {
            "分享模式（去除绝对路径与路径样片段）"
        } else {
            "完整（本地用途）"
        },
        args.output.display()
    );
    println!(
        "快照：{}（{}，{}）",
        document.snapshot_id,
        document.snapshot_time,
        match document.freshness {
            Freshness::Current => "current",
            Freshness::Stale => "stale",
            _ => "其他",
        }
    );

    let mut artifacts: Vec<(String, String)> = Vec::new();
    if args.format.includes_json() {
        let json = render_export_json(&document)
            .map_err(|error| format!("导出 JSON 生成失败：{error}"))?;
        artifacts.push((EXPORT_JSON_NAME.to_string(), json));
    }
    if args.format.includes_csv() {
        let csv = render_export_csv(&document, &metadata, args.share);
        artifacts.push((EXPORT_CSV_NAME.to_string(), csv));
    }
    if args.share {
        for (artifact_name, contents) in &artifacts {
            let findings = scan_private_material(contents);
            if !findings.is_empty() {
                return Err(format!(
                    "分享导出自检失败（{artifact_name} 仍含路径样片段）：{}",
                    findings.join("；")
                ));
            }
        }
    }

    let staging =
        output_commit::prepare(&args.output, &artifacts).map_err(|error| error.message)?;
    let files = staging.commit().map_err(|error| error.message)?;
    for path in &files {
        let size = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        println!("输出：{}（{size} 字节）", path.display());
    }
    println!(
        "输入摘要：{}…；元数据 {}；参考集合 {}",
        short_hash(&document.input_digest.compressed_sha256),
        document
            .report
            .metadata_version
            .as_deref()
            .unwrap_or("未知"),
        document
            .report
            .reference_set_version
            .as_deref()
            .unwrap_or("未知"),
    );
    Ok(EXIT_OK)
}

fn short_hash(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// missing 命令参数。
#[derive(Debug, Clone, PartialEq)]
struct MissingArgs {
    catalog: PathBuf,
    analysis: PathBuf,
    input: Option<String>,
    output: PathBuf,
}

/// analysis.json 读取上限（批量报告可能较大，独立于元数据上限）。
const ANALYSIS_FILE_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
/// missing 输出文件名。
const MISSING_FILE_NAME: &str = "missing.json";
/// plan 输出文件名。
const PLAN_FILE_NAME: &str = "plan.json";

/// 读取并校验图鉴与分析输入，返回（图鉴、观察集合、`input_ref`）。
///
/// `missing` 与 `plan` 共用同一份输入校验，保证两个命令对同一输入集合的解释一致。
fn load_catalog_and_observed(
    catalog_path: &Path,
    analysis_path: &Path,
    input: Option<&str>,
) -> Result<(Catalog, BTreeSet<String>, String), String> {
    path_policy::validate_input(catalog_path).map_err(|error| error.message)?;
    path_policy::validate_input(analysis_path).map_err(|error| error.message)?;
    if !catalog_path.is_file() {
        return Err(format!("目录 JSON 不是文件：{}", catalog_path.display()));
    }
    if !analysis_path.is_file() {
        return Err(format!(
            "analysis.json 不是文件：{}",
            analysis_path.display()
        ));
    }

    let catalog_bytes =
        fs::read(catalog_path).map_err(|error| format!("读取目录 JSON 失败：{error}"))?;
    let catalog = Catalog::from_json(&catalog_bytes, &MetadataLimits::default())
        .map_err(|error| format!("目录加载失败（{}）：{}", error.code(), error.message()))?;

    let analysis_size = fs::metadata(analysis_path)
        .map_err(|error| format!("读取 analysis.json 信息失败：{error}"))?
        .len();
    if analysis_size > ANALYSIS_FILE_LIMIT_BYTES {
        return Err(format!(
            "analysis.json 为 {analysis_size} 字节，超过上限 {ANALYSIS_FILE_LIMIT_BYTES}"
        ));
    }
    let analysis_bytes =
        fs::read(analysis_path).map_err(|error| format!("读取 analysis.json 失败：{error}"))?;
    let analysis: serde_json::Value = serde_json::from_slice(&analysis_bytes)
        .map_err(|error| format!("解析 analysis.json 失败：{error}"))?;
    let results = analysis
        .get("results")
        .and_then(|value| value.as_array())
        .ok_or("analysis.json 缺少 results 数组（需为 analyze 输出的批量报告）")?;
    let with_report: Vec<&serde_json::Value> = results
        .iter()
        .filter(|result| result.get("report").is_some_and(|report| !report.is_null()))
        .collect();

    let chosen = match input {
        Some(name) => with_report
            .iter()
            .copied()
            .find(|result| result.get("input_ref").and_then(|value| value.as_str()) == Some(name))
            .ok_or_else(|| format!("在 analysis.json 中找不到带报告的输入 {name:?}"))?,
        None => match with_report.as_slice() {
            [only] => only,
            [] => return Err("analysis.json 中没有可用报告".to_string()),
            _ => return Err("analysis.json 含多个输入报告，请用 --input 指定".to_string()),
        },
    };
    let input_ref = chosen
        .get("input_ref")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let records = chosen
        .pointer("/report/observations/inventory/records")
        .and_then(|value| value.as_array())
        .ok_or("报告缺少 observations.inventory.records")?;
    let mut observed: BTreeSet<String> = BTreeSet::new();
    for record in records {
        let id = record
            .get("internal_id")
            .and_then(|value| value.as_str())
            .ok_or("观察记录缺少 internal_id")?;
        observed.insert(id.to_string());
    }
    Ok((catalog, observed, input_ref))
}

/// plan 命令参数。
#[derive(Debug, Clone, PartialEq)]
struct PlanArgs {
    catalog: PathBuf,
    analysis: PathBuf,
    input: Option<String>,
    output: PathBuf,
}

fn parse_plan_args(args: &[String]) -> Result<PlanArgs, String> {
    let mut catalog: Option<PathBuf> = None;
    let mut analysis: Option<PathBuf> = None;
    let mut input: Option<String> = None;
    let mut output: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("参数 {flag} 缺少取值"))?;
        match flag {
            "--catalog" => set_once(&mut catalog, PathBuf::from(value), "--catalog")?,
            "--analysis" => set_once(&mut analysis, PathBuf::from(value), "--analysis")?,
            "--input" => set_once(&mut input, value.to_string(), "--input")?,
            "--output" => set_once(&mut output, PathBuf::from(value), "--output")?,
            other => return Err(format!("未知参数 {other:?}")),
        }
        index += 2;
    }

    Ok(PlanArgs {
        catalog: catalog.ok_or("缺少 --catalog")?,
        analysis: analysis.ok_or("缺少 --analysis")?,
        input,
        output: output.ok_or("缺少 --output")?,
    })
}

fn run_plan(args: &PlanArgs) -> Result<u8, String> {
    path_policy::validate_output(
        &args.output,
        &[args.catalog.as_path(), args.analysis.as_path()],
    )
    .map_err(|error| error.message)?;

    let (catalog, observed, input_ref) =
        load_catalog_and_observed(&args.catalog, &args.analysis, args.input.as_deref())?;

    let plan = atlas::plan(&catalog, &observed);
    let mut document =
        serde_json::to_value(&plan).map_err(|error| format!("序列化收集计划失败：{error}"))?;
    document["input_ref"] = serde_json::Value::String(input_ref.clone());
    let mut json = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("序列化收集计划失败：{error}"))?;
    json.push('\n');
    let staging = output_commit::prepare(&args.output, &[(PLAN_FILE_NAME.to_string(), json)])
        .map_err(|error| error.message)?;
    let files = staging.commit().map_err(|error| error.message)?;
    let output_path = files
        .into_iter()
        .next()
        .unwrap_or_else(|| args.output.join(PLAN_FILE_NAME));

    println!(
        "输入：{input_ref}（图鉴 {}；主区域 {}/{}；目录外观察 ID {} 个）",
        plan.catalog_version,
        plan.main_collected,
        plan.main_total,
        plan.observed_not_in_catalog.len()
    );
    for region in &plan.regions {
        let label = region
            .name_zh
            .clone()
            .or_else(|| region.name_en.clone())
            .unwrap_or_else(|| region.slug.clone());
        let kind = match region.kind {
            atlas::RegionKind::Main => "",
            atlas::RegionKind::Special => "（特殊区域，不计入主区域合计）",
        };
        println!(
            "  {:<26} {:>4}/{:<4} 已收集{kind}",
            label, region.collected, region.total
        );
        for group in &region.groups {
            if group.collected == group.total {
                continue;
            }
            println!(
                "      {:<28} {:>4}/{:<4} 已收集",
                group.name, group.collected, group.total
            );
        }
    }
    println!("输出：{}", output_path.display());

    Ok(EXIT_OK)
}

fn parse_missing_args(args: &[String]) -> Result<MissingArgs, String> {
    let mut catalog: Option<PathBuf> = None;
    let mut analysis: Option<PathBuf> = None;
    let mut input: Option<String> = None;
    let mut output: Option<PathBuf> = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("参数 {flag} 缺少取值"))?;
        match flag {
            "--catalog" => set_once(&mut catalog, PathBuf::from(value), "--catalog")?,
            "--analysis" => set_once(&mut analysis, PathBuf::from(value), "--analysis")?,
            "--input" => set_once(&mut input, value.to_string(), "--input")?,
            "--output" => set_once(&mut output, PathBuf::from(value), "--output")?,
            other => return Err(format!("未知参数 {other:?}")),
        }
        index += 2;
    }

    Ok(MissingArgs {
        catalog: catalog.ok_or("缺少 --catalog")?,
        analysis: analysis.ok_or("缺少 --analysis")?,
        input,
        output: output.ok_or("缺少 --output")?,
    })
}

fn run_missing(args: &MissingArgs) -> Result<u8, String> {
    path_policy::validate_output(
        &args.output,
        &[args.catalog.as_path(), args.analysis.as_path()],
    )
    .map_err(|error| error.message)?;

    let (catalog, observed, input_ref) =
        load_catalog_and_observed(&args.catalog, &args.analysis, args.input.as_deref())?;

    let missing = compute_missing(&catalog, &observed);
    let mut document =
        serde_json::to_value(&missing).map_err(|error| format!("序列化缺失报告失败：{error}"))?;
    document["input_ref"] = serde_json::Value::String(input_ref.clone());
    let mut json = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("序列化缺失报告失败：{error}"))?;
    json.push('\n');
    let staging = output_commit::prepare(&args.output, &[(MISSING_FILE_NAME.to_string(), json)])
        .map_err(|error| error.message)?;
    let files = staging.commit().map_err(|error| error.message)?;
    let output_path = files
        .into_iter()
        .next()
        .unwrap_or_else(|| args.output.join(MISSING_FILE_NAME));

    println!(
        "输入：{input_ref}（观察到目录条目 {} 个，未知 {} 个）",
        missing.observed_items,
        missing.observed_not_in_catalog.len()
    );
    for category in &missing.categories {
        match category.required_for_100 {
            Some(true) => println!(
                "  {:<22} {}/{} 已观察，缺失 {}",
                category.id,
                category.observed,
                category.total,
                category.missing.unwrap_or(0)
            ),
            Some(false) => println!(
                "  {:<22} {}/{} 已观察（可选，无清单）",
                category.id, category.observed, category.total
            ),
            None => println!(
                "  {:<22} {}/{} 已观察（未闭合，无缺失结论）",
                category.id, category.observed, category.total
            ),
        }
    }
    println!("输出：{}", output_path.display());

    Ok(EXIT_OK)
}

const USAGE: &str = "\
sottr-cli — SOTTR Explorer 命令行（只读分析 / 导出）

用法：
  sottr-cli analyze --input <文件或目录> --context <campaign|unknown> \\
      --output <新目录> --metadata <元数据目录>
  sottr-cli export --input <文件> --context <campaign|unknown> \\
      --output <新目录> --metadata <元数据目录> \\
      [--format json|csv|both] [--share] [--snapshot-id <ID>] \\
      [--snapshot-time <RFC3339>] [--stale --refresh-error <CODE>]
  sottr-cli missing --catalog <全收集图鉴 JSON> --analysis <analysis.json> \\
      --output <新目录> [--input <input_ref>]
  sottr-cli plan --catalog <全收集图鉴 JSON> --analysis <analysis.json> \\
      --output <新目录> [--input <input_ref>]
  sottr-cli --help

说明：
  - analyze 读取输入目录中的 *.dat（最多 128 个）或单个文件；
    输出写入必须尚不存在的目录，文件名为 analysis.json（批量 JSON）。
  - 输入与输出只支持本地卷；网络（UNC）与设备命名空间路径明确拒绝；
    输出不得与输入相等、包含输入或位于输入（或输入文件所在目录）之内，
    比较按规范路径（解析链接与真实大小写），不使用字符串前缀。
  - 输入以稳定快照读取：文件被占用或正在保存时拒绝并提示稍后重试
    （INPUT_UNREADABLE，可重试）；读取期间文件变化或路径被替换时返回
    INPUT_CHANGED，不产生部分结果。输出用独占新目录与受控临时文件提交，
    失败或取消不留“完整报告”假象。
  - profile.dat 始终按资料文件处理（仅容器观察），不受 --context 影响。
  - --context campaign 是用户声明，不是格式识别证明；构建号与 DLC 未声明时
    保留适用限制，区域计数与参考差集按社区证据带限制展示。
  - 纯分析不做全文件两两比较；不写回、不修改任何输入文件。
  - export 输出 report.json（导出信封：快照身份、freshness、刷新错误码、
    input_digest 与报告）与 report.csv（逐项评估；快照身份与刷新错误码为明确列）。
  - --share 分享模式：去除绝对路径与路径样片段（input_ref 去目录、文本路径替换为
    文件名），并执行分享自检；默认（完整）面向本地查看。CSV 对以 =、+、-、@、
    制表或回车等危险前缀开头的外部文本统一转义为安全文本。
  - 旧结果导出用 --stale --refresh-error <CODE> 明确标记过期原因；
    产物先写临时文件再提交，失败或取消不留下“完整报告”假象；输出目录必须新目录。
  - missing 依据全收集目录（data/catalog）与 analyze 批量报告计算按类别进度与逐项缺失：
    必收类别给出清单（缺失=未在所选存档观察集合中出现）、可选类别只给计数、
    未闭合类别（如挑战古墓）不出缺失结论；共用同一 ID 的条目按 ID 计数并附全部名称；
    观察集合中不在目录的 ID 单列，不混入缺失；报告多于一个时用 --input 指定。
  - plan 依据同一图鉴与观察集合计算**按区域的收集计划**：每个区域的收集品清单、
    已收集/未收集状态、所属子关卡、关卡内相对位置（未校准）与获取方式，
    以及区域与类别的进度；特殊区域（跨区域挑战古墓、全局区域、克劳馥庄园）
    单独成组，不计入主区域合计；区域进度分母是该区域图鉴条目数，
    不是游戏内地图百分比（后者口径未闭合，见 research/s020）。
  - 退出码：成功 0；输入级错误（不可读/无法分析）1；用法或配置错误 2。

示例：
  sottr-cli analyze --input samples --context campaign --output analysis01 \\
      --metadata data/metadata
  sottr-cli export --input samples/save1.dat --context campaign --output export01 \\
      --metadata data/metadata --format both --share";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_all_flags() {
        let args = vec!["--input".to_string(), "a".to_string()];
        let error = parse_analyze_args(&args).unwrap_err();
        assert!(error.contains("--context"), "{error}");
    }

    #[test]
    fn parse_rejects_unknown_context_and_duplicates() {
        let base = || {
            vec![
                "--input".to_string(),
                "in".to_string(),
                "--context".to_string(),
                "campaign".to_string(),
                "--output".to_string(),
                "out".to_string(),
                "--metadata".to_string(),
                "meta".to_string(),
            ]
        };
        let parsed = parse_analyze_args(&base()).unwrap();
        assert_eq!(parsed.context, Mode::Campaign);
        assert_eq!(parsed.input, PathBuf::from("in"));

        let mut bad_context = base();
        bad_context[3] = "profile".to_string();
        assert!(parse_analyze_args(&bad_context).is_err());

        let mut duplicate = base();
        duplicate.extend(["--input".to_string(), "other".to_string()]);
        assert!(parse_analyze_args(&duplicate).is_err());
    }

    #[test]
    fn missing_command_returns_usage_exit_code() {
        assert_eq!(run(&[]).unwrap(), EXIT_USAGE);
        assert_eq!(run(&["--help".to_string()]).unwrap(), EXIT_OK);
        assert!(run(&["analyze".to_string()]).is_err());
    }

    #[test]
    fn collect_single_explicit_file_ignores_extension() {
        let dir = unique_temp_dir("cli-collect-single");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save1.dat");
        fs::write(&path, b"x").unwrap();
        let collected = collect_inputs(&path).unwrap();
        assert_eq!(collected, vec![path.clone()]);
        fs::remove_dir_all(&dir).unwrap();

        let missing = dir.join("save1.dat");
        let error = collect_inputs(&missing).unwrap_err();
        assert!(error.contains("既不是文件也不是目录"), "{error}");
    }

    #[test]
    fn collect_directory_sorts_and_filters_dat_files() {
        let dir = unique_temp_dir("cli-collect-dir");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("saveB.dat"), b"x").unwrap();
        fs::write(dir.join("savea.dat"), b"x").unwrap();
        fs::write(dir.join("notes.txt"), b"x").unwrap();
        let collected = collect_inputs(&dir).unwrap();
        let names: Vec<String> = collected.iter().map(|path| file_name(path)).collect();
        assert_eq!(names, vec!["savea.dat", "saveB.dat"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unreadable_entry_maps_to_input_unreadable() {
        // 目录路径无法作为文件读取：映射为 INPUT_UNREADABLE（真实 ACL 场景属 SX-012/013）。
        let dir = unique_temp_dir("cli-unreadable");
        fs::create_dir_all(&dir).unwrap();
        let args = AnalyzeArgs {
            input: dir.clone(),
            context: Mode::Campaign,
            output: dir.join("out"),
            metadata: dir.clone(),
        };
        let error =
            analyze_file(&dir, &args, &MetadataBundle::empty(), &Limits::default()).unwrap_err();
        assert_eq!(error.code, "INPUT_UNREADABLE");
        assert!(error.retryable);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn profile_name_detection_is_case_insensitive() {
        assert!("profile.dat".eq_ignore_ascii_case("Profile.DAT"));
        assert!("PROFILE.DAT".eq_ignore_ascii_case("profile.dat"));
    }

    #[test]
    fn export_parse_defaults_and_full_flags() {
        let base = || {
            vec![
                "--input".to_string(),
                "save1.dat".to_string(),
                "--context".to_string(),
                "campaign".to_string(),
                "--output".to_string(),
                "out".to_string(),
                "--metadata".to_string(),
                "meta".to_string(),
            ]
        };
        let parsed = parse_export_args(&base()).unwrap();
        assert_eq!(parsed.format, ExportFormat::Both);
        assert!(!parsed.share);
        assert!(!parsed.stale);
        assert_eq!(parsed.snapshot_id, None);

        let mut full = base();
        full.extend([
            "--format".to_string(),
            "csv".to_string(),
            "--share".to_string(),
            "--stale".to_string(),
            "--refresh-error".to_string(),
            "INPUT_CHANGED".to_string(),
            "--snapshot-id".to_string(),
            "snap-test-1".to_string(),
            "--snapshot-time".to_string(),
            "2026-10-15T12:34:56.789Z".to_string(),
        ]);
        let parsed = parse_export_args(&full).unwrap();
        assert_eq!(parsed.format, ExportFormat::Csv);
        assert!(parsed.share);
        assert!(parsed.stale);
        assert_eq!(parsed.refresh_error.as_deref(), Some("INPUT_CHANGED"));
        assert_eq!(parsed.snapshot_id.as_deref(), Some("snap-test-1"));
    }

    #[test]
    fn export_parse_rejects_conflicts_and_bad_values() {
        let base = || {
            vec![
                "--input".to_string(),
                "save1.dat".to_string(),
                "--context".to_string(),
                "campaign".to_string(),
                "--output".to_string(),
                "out".to_string(),
                "--metadata".to_string(),
                "meta".to_string(),
            ]
        };
        let cases: Vec<Vec<String>> = vec![
            // --stale 缺少刷新错误码。
            {
                let mut args = base();
                args.push("--stale".to_string());
                args
            },
            // --refresh-error 未配合 --stale。
            {
                let mut args = base();
                args.extend(["--refresh-error".to_string(), "INPUT_CHANGED".to_string()]);
                args
            },
            // 非法错误码。
            {
                let mut args = base();
                args.extend([
                    "--stale".to_string(),
                    "--refresh-error".to_string(),
                    "input_changed".to_string(),
                ]);
                args
            },
            // 非法时间。
            {
                let mut args = base();
                args.extend([
                    "--snapshot-time".to_string(),
                    "2026-13-40T99:00:00Z".to_string(),
                ]);
                args
            },
            // 非法格式。
            {
                let mut args = base();
                args.extend(["--format".to_string(), "pdf".to_string()]);
                args
            },
            // --share 重复。
            {
                let mut args = base();
                args.extend(["--share".to_string(), "--share".to_string()]);
                args
            },
        ];
        for case in cases {
            assert!(parse_export_args(&case).is_err(), "{case:?}");
        }
    }

    fn unique_temp_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sottr-cli-{tag}-{}", std::process::id()))
    }
}
