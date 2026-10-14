//! sottr-cli：`analyze` 命令（SX-009）。
//!
//! 用法（最终语法在 SX-009 固定，见 `--help`）：
//!
//! ```text
//! sottr-cli analyze --input <文件或目录> --context <campaign|unknown> \
//!     --output <新目录> --metadata <元数据目录>
//! ```
//!
//! 边界与行为：
//! - 目录读取、文件读取与输出写入由本适配层承担；核心只接收字节
//!   （`sottr_core::analyze::analyze_bytes`）；
//! - 批量最多 128 个文件；一个坏文件保留自己的输入级错误，不影响其他文件；
//! - `profile.dat` 始终按资料文件处理（仅容器观察），不受 `--context` 影响；
//! - `--context campaign` 是用户声明，不是格式识别证明；
//! - 纯分析不做全文件两两比较；不写回、不修改任何输入文件；
//! - 输出目录必须不存在；成功与否都写出批量 JSON（除用法错误外）。
//!
//! 退出码：
//! - `0`：全部文件成功分析（报告可含局部诊断降级）；
//! - `1`：分析完成，但至少一个输入级错误（批量 JSON 仍包含全部结果）；
//! - `2`：用法或配置错误（参数、路径、输出已存在、批量超限、元数据加载失败）。

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sottr_core::analyze::analyze_bytes;
use sottr_core::domain::context::DeclaredContext;
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, MetadataFileInput, MetadataLimits};
use sottr_core::report::{
    BatchReport, BatchResult, ContextSource, InputError, Mode, SchemaVersion,
};

/// 批量文件数量上限。
const BATCH_FILE_LIMIT: usize = 128;
/// 输出文件名。
const OUTPUT_FILE_NAME: &str = "analysis.json";
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
        other => Err(format!("未知命令 {other:?}（当前仅支持 analyze）")),
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
    if !args.input.exists() {
        return Err(format!("输入路径不存在：{}", args.input.display()));
    }
    if !args.metadata.is_dir() {
        return Err(format!("元数据路径不是目录：{}", args.metadata.display()));
    }
    if args.output.exists() {
        return Err(format!(
            "输出目录已存在，必须使用新目录：{}",
            args.output.display()
        ));
    }

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

    fs::create_dir_all(&args.output)
        .map_err(|error| format!("创建输出目录失败（{}）：{error}", args.output.display()))?;
    let output_path = args.output.join(OUTPUT_FILE_NAME);
    let batch = BatchReport {
        schema_version: SchemaVersion,
        results,
    };
    let mut json = serde_json::to_vec_pretty(&batch)
        .map_err(|error| format!("序列化批量报告失败：{error}"))?;
    json.push(b'\n');
    let mut file = fs::File::create(&output_path)
        .map_err(|error| format!("创建输出文件失败（{}）：{error}", output_path.display()))?;
    file.write_all(&json)
        .map_err(|error| format!("写入输出文件失败（{}）：{error}", output_path.display()))?;

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
    // 适配层预检：明显超过压缩上限的输入不整份读入内存（核心仍会复核）。
    if let Ok(metadata_of_file) = fs::metadata(path)
        && metadata_of_file.len() > limits.max_compressed_bytes
    {
        return Err(InputError::resource_limit(format!(
            "文件 {name} 为 {} 字节，超过压缩上限 {}（compressed_size_limit_exceeded）",
            metadata_of_file.len(),
            limits.max_compressed_bytes
        )));
    }
    let bytes = fs::read(path)
        .map_err(|error| InputError::unreadable(format!("读取 {name} 失败：{error}")))?;
    // profile.dat 始终按资料文件处理：批量 --context 不得覆盖该排除。
    let mode = if name.eq_ignore_ascii_case("profile.dat") {
        Mode::Profile
    } else {
        args.context
    };
    let declared = DeclaredContext {
        platform: None,
        mode,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    };
    analyze_bytes(&bytes, &declared, metadata, limits)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

const USAGE: &str = "\
sottr-cli — SOTTR Explorer 命令行（只读分析）

用法：
  sottr-cli analyze --input <文件或目录> --context <campaign|unknown> \\
      --output <新目录> --metadata <元数据目录>
  sottr-cli --help

说明：
  - analyze 读取输入目录中的 *.dat（最多 128 个）或单个文件；
    输出写入必须尚不存在的目录，文件名为 analysis.json（批量 JSON）。
  - profile.dat 始终按资料文件处理（仅容器观察），不受 --context 影响。
  - --context campaign 是用户声明，不是格式识别证明；构建号与 DLC 未声明时
    保留适用限制，区域计数与参考差集按社区证据带限制展示。
  - 纯分析不做全文件两两比较；不写回、不修改任何输入文件。
  - 退出码：全部成功 0；至少一个输入级错误 1；用法或配置错误 2。

示例：
  sottr-cli analyze --input samples --context campaign --output analysis01 \\
      --metadata data/metadata";

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

    fn unique_temp_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sottr-cli-{tag}-{}", std::process::id()))
    }
}
