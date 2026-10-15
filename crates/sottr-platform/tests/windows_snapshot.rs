//! SX-013 Windows 平台测试（真实文件系统：稳定快照与路径替换）。
//!
//! 覆盖：稳定快照摘要与输入不变、输出目录被重解析点替换时拒绝提交、
//! 输出路径本身是既存链接时拒绝。受限分支（无法创建 junction）明确跳过并说明。
//!
//! 运行：`cargo test -p sottr-platform`（Windows 实机）。

#![cfg(windows)]

use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sottr_core::hash::sha256_hex;
use sottr_core::limits::Limits;
use sottr_platform::output_commit::{self, OutputErrorCode};
use sottr_platform::read_snapshot;

fn scratch(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sottr-platform-stage-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 创建目录 junction（重解析点）；不可用时返回 false 由调用方跳过。
fn create_junction(link: &Path, target: &Path) -> bool {
    Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            &link.to_string_lossy(),
            &target.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn entries(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

#[test]
fn stable_read_matches_disk_digest_and_leaves_input_unchanged() {
    let root = scratch("read");
    let path = root.join("save1.dat");
    let content = vec![7u8; 4096];
    fs::write(&path, &content).unwrap();
    let before = fs::metadata(&path).unwrap().last_write_time();

    let snapshot = read_snapshot::read_stable(&path, &Limits::default()).unwrap();
    assert_eq!(snapshot.bytes, content);
    assert_eq!(snapshot.compressed_sha256, sha256_hex(&content));
    assert!(
        snapshot.write_denied_during_read,
        "Windows 必须使用共享约束"
    );

    let after = fs::metadata(&path).unwrap().last_write_time();
    assert_eq!(after, before, "读取不得改变输入时间戳");
    assert_eq!(fs::read(&path).unwrap(), content, "读取不得改变输入内容");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn output_directory_replaced_by_junction_is_refused() {
    let root = scratch("replace");
    let victim = scratch("replace-victim");
    let dir = root.join("out");
    let artifacts = vec![("report.json".to_string(), "{\"x\":1}\n".to_string())];

    let staging = output_commit::prepare(&dir, &artifacts).unwrap();
    // 攻击模拟：把已准备好的输出目录整体改名，并在原路径放一个指向受害目录的
    // junction（重解析点）——提交必须失败，且不得通过链接写入任何文件。
    let moved = root.join("moved");
    fs::rename(&dir, &moved).unwrap();
    if !create_junction(&dir, &victim) {
        eprintln!("junction 创建不可用，跳过路径替换分支");
        let _ = fs::remove_dir_all(&moved);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&victim);
        return;
    }

    let error = staging.commit().unwrap_err();
    assert_eq!(
        error.code,
        OutputErrorCode::LinkReplaced,
        "{}",
        error.message
    );
    assert!(!error.retryable());
    assert!(
        entries(&victim).is_empty(),
        "不得通过替换后的链接写入任何文件"
    );

    let _ = fs::remove_dir(&dir); // junction 本身（Drop 清理通常已移除）
    let _ = fs::remove_dir_all(&moved); // 被改名走目录中的临时文件（残余边界）
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&victim);
}

#[test]
fn output_path_that_is_a_junction_is_refused() {
    let root = scratch("existing-link");
    let victim = scratch("existing-link-victim");
    let dir = root.join("out");
    if !create_junction(&dir, &victim) {
        eprintln!("junction 创建不可用，跳过既存链接分支");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&victim);
        return;
    }

    let error =
        output_commit::prepare(&dir, &[("report.json".to_string(), "{}".to_string())]).unwrap_err();
    assert_eq!(
        error.code,
        OutputErrorCode::OutputExists,
        "{}",
        error.message
    );
    assert!(entries(&victim).is_empty(), "受害目录不得被写入任何文件");

    let _ = fs::remove_dir(&dir);
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&victim);
}
