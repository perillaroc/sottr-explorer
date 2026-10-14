//! Python v0.2 参照运行器（SX-010）：调用仓库内研究参照副本，保留原始输出。

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// 解释器信息。
#[derive(Debug, Clone, Serialize)]
pub struct InterpreterInfo {
    /// 程序名（py / python3 / python）。
    pub program: String,
    /// `--version` 输出。
    pub version: String,
}

/// 一次 Python v0.2 运行结果。
#[derive(Debug)]
pub struct PythonRun {
    /// `report.json` 原始解析值。
    pub report: serde_json::Value,
    /// 输出目录（保留全部原始产物）。
    pub output_dir: PathBuf,
    /// 实际命令行（含解释器参数）。
    pub command_line: Vec<String>,
    /// 解释器信息。
    pub interpreter: InterpreterInfo,
}

/// 运行错误。
#[derive(Debug)]
pub enum PythonRunError {
    /// 找不到可用解释器或脚本。
    NotFound(String),
    /// 执行失败。
    Failed(String),
}

impl fmt::Display for PythonRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PythonRunError::NotFound(detail) => write!(formatter, "Python 不可用：{detail}"),
            PythonRunError::Failed(detail) => write!(formatter, "Python v0.2 运行失败：{detail}"),
        }
    }
}

/// 探测解释器；优先 `SOTTR_PYTHON`，否则依次尝试 `py -3`、`python3`、`python`。
pub fn find_interpreter() -> Result<(InterpreterInfo, Vec<String>), PythonRunError> {
    if let Ok(explicit) = std::env::var("SOTTR_PYTHON") {
        if let Some(version) = probe(&explicit, &[]) {
            return Ok((
                InterpreterInfo {
                    program: explicit,
                    version,
                },
                Vec::new(),
            ));
        }
        return Err(PythonRunError::NotFound(format!(
            "SOTTR_PYTHON={explicit} 不可用"
        )));
    }
    for (program, args) in [
        ("py", vec!["-3"]),
        ("python3", Vec::new()),
        ("python", Vec::new()),
    ] {
        let prefix: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        if let Some(version) = probe(program, &prefix) {
            return Ok((
                InterpreterInfo {
                    program: program.to_string(),
                    version,
                },
                prefix,
            ));
        }
    }
    Err(PythonRunError::NotFound(
        "未找到 py/python3/python 解释器（可用 SOTTR_PYTHON 指定）".to_string(),
    ))
}

fn probe(program: &str, prefix_args: &[String]) -> Option<String> {
    let output = Command::new(program)
        .args(prefix_args)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let text = if stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).trim().to_string()
    } else {
        stdout
    };
    if text.is_empty() { None } else { Some(text) }
}

/// 运行 Python v0.2（`--max-pairs 0`：两两比较不在本回归范围）并读取原始报告。
pub fn run_v02(
    tool_dir: &Path,
    input: &Path,
    output: &Path,
    catalog: &Path,
) -> Result<PythonRun, PythonRunError> {
    let (interpreter, prefix_args) = find_interpreter()?;
    let script = tool_dir.join("sottr_inspect.py");
    if !script.is_file() {
        return Err(PythonRunError::NotFound(format!(
            "未找到研究参照程序：{}",
            script.display()
        )));
    }
    if !catalog.is_file() {
        return Err(PythonRunError::NotFound(format!(
            "未找到字典副本：{}",
            catalog.display()
        )));
    }
    if output.exists() {
        fs::remove_dir_all(output).map_err(|error| {
            PythonRunError::Failed(format!("清理输出目录失败（{}）：{error}", output.display()))
        })?;
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            PythonRunError::Failed(format!(
                "创建输出父目录失败（{}）：{error}",
                parent.display()
            ))
        })?;
    }

    let mut command = Command::new(&interpreter.program);
    command
        .args(&prefix_args)
        .arg(&script)
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--max-pairs")
        .arg("0")
        .arg("--catalog")
        .arg(catalog)
        .env("PYTHONDONTWRITEBYTECODE", "1");
    let output_result = command.output().map_err(|error| {
        PythonRunError::Failed(format!("无法执行 {}：{error}", interpreter.program))
    })?;
    if !output_result.status.success() {
        return Err(PythonRunError::Failed(format!(
            "退出码 {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output_result.status.code(),
            String::from_utf8_lossy(&output_result.stdout),
            String::from_utf8_lossy(&output_result.stderr)
        )));
    }

    let report_path = output.join("report.json");
    let bytes = fs::read(&report_path).map_err(|error| {
        PythonRunError::Failed(format!("读取 {} 失败：{error}", report_path.display()))
    })?;
    let report: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        PythonRunError::Failed(format!("解析 {} 失败：{error}", report_path.display()))
    })?;

    let mut command_line = vec![interpreter.program.clone()];
    command_line.extend(prefix_args);
    command_line.extend([
        script.display().to_string(),
        "--input".to_string(),
        input.display().to_string(),
        "--output".to_string(),
        output.display().to_string(),
        "--max-pairs".to_string(),
        "0".to_string(),
        "--catalog".to_string(),
        catalog.display().to_string(),
    ]);

    Ok(PythonRun {
        report,
        output_dir: output.to_path_buf(),
        command_line,
        interpreter,
    })
}
