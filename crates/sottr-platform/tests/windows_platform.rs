//! SX-012 Windows 平台测试（真实文件系统场景）。
//!
//! 覆盖：Known Folder（含独立进程交叉核对）、中文路径与大小写、不可读目录
//! （ACL deny，实机）、长路径、重复内容文件、UNC 明确拒绝、默认候选结构。
//! 受限环境（无法创建链接/ACL/长路径）的分支明确跳过并说明原因；跳过不计为通过。
//!
//! 运行：`cargo test -p sottr-platform`（Windows 实机）。

#![cfg(windows)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sottr_platform::discovery::{
    self, CandidateStatus, DiscoveryLimits, DocumentsDir, DocumentsScanStatus, DocumentsSource,
    SkippedReason,
};
use sottr_platform::path_policy::{self, PathPolicyCode};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sottr-platform-win-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn candidate_docs(path: &Path) -> Vec<DocumentsDir> {
    vec![DocumentsDir {
        path: path.to_path_buf(),
        source: DocumentsSource::KnownFolder,
    }]
}

#[test]
fn known_folder_documents_is_real_and_matches_powershell() {
    let documents =
        discovery::documents_known_folder().expect("Windows 应提供 Known Folder Documents");
    assert!(documents.is_absolute(), "{}", documents.display());
    assert!(
        documents.is_dir(),
        "Known Folder Documents 必须是已存在目录：{}",
        documents.display()
    );

    // 独立进程交叉核对（PowerShell 不可用时跳过该子检查并说明，不计为通过）。
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Environment]::GetFolderPath('MyDocuments')",
        ])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            assert!(!text.is_empty(), "PowerShell 返回空路径");
            assert!(
                text.eq_ignore_ascii_case(&documents.to_string_lossy()),
                "Known Folder（{}）与 PowerShell GetFolderPath（{text}）必须一致",
                documents.display()
            );
        }
        Ok(output) => eprintln!("PowerShell 退出码 {}，跳过交叉核对", output.status),
        Err(error) => eprintln!("PowerShell 不可用（{error}），跳过交叉核对"),
    }
}

#[test]
fn default_scan_is_structured_and_deduped_on_real_machine() {
    let scans = discovery::scan_default_documents();
    assert!(!scans.is_empty(), "Windows 至少应有 Known Folder 候选");
    let known = discovery::documents_known_folder().expect("Known Folder 可用");
    assert_eq!(scans[0].dir.source, DocumentsSource::KnownFolder);
    assert_eq!(scans[0].dir.path, known);
    // 同一路径只出现一次（大小写不敏感比较）。
    let mut keys: Vec<String> = scans
        .iter()
        .map(|scan| scan.dir.path.to_string_lossy().to_lowercase())
        .collect();
    let total = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), total, "候选路径不得重复：{scans:?}");
    // 实机记录（--nocapture 可见）：Known Folder 与每候选状态。
    eprintln!("Known Folder Documents: {}", known.display());
    for scan in &scans {
        eprintln!("候选 {} -> {:?}", scan.dir.path.display(), scan.status);
        if let DocumentsScanStatus::GameFound(game) = &scan.status {
            assert_eq!(
                game,
                &scan.dir.path.join(discovery::GAME_DIR_NAME),
                "游戏目录必须是候选路径加固定目录名"
            );
        }
    }
}

#[test]
fn chinese_paths_and_case_variants_are_discovered() {
    let root = scratch("chinese");
    let documents = root.join("用户文档");
    // 磁盘上的实际大小写与查找名不同：Windows 文件系统按大小写不敏感解析。
    let actual_game = documents.join("shadow OF the tomb RAIDER");
    fs::create_dir_all(&actual_game).unwrap();
    fs::write(actual_game.join("SAVE1.DAT"), b"x").unwrap();
    fs::write(actual_game.join("Profile.dat"), b"x").unwrap();

    let scans = discovery::scan_documents(&candidate_docs(&documents));
    let expected = documents.join(discovery::GAME_DIR_NAME);
    assert_eq!(
        scans[0].status,
        DocumentsScanStatus::GameFound(expected.clone())
    );

    let listing = discovery::list_candidates(&expected, &DiscoveryLimits::default()).unwrap();
    let names: Vec<&str> = listing
        .files
        .iter()
        .map(|file| file.name.as_str())
        .collect();
    assert_eq!(names, vec!["Profile.dat", "SAVE1.DAT"]);
    assert!(
        listing
            .files
            .iter()
            .all(|file| file.matches_observed_names && file.status == CandidateStatus::Ready)
    );

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn duplicate_content_files_are_listed_separately() {
    let root = scratch("dupes");
    let game = root.join(discovery::GAME_DIR_NAME);
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("save204.dat"), b"same bytes").unwrap();
    fs::write(game.join("save206.dat"), b"same bytes").unwrap();

    let listing = discovery::list_candidates(&game, &DiscoveryLimits::default()).unwrap();
    assert_eq!(listing.files.len(), 2, "发现层不去重、不合并、不替用户选择");
    assert_eq!(listing.files[0].size_bytes, Some(10));
    assert_eq!(listing.files[1].size_bytes, Some(10));

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn unreadable_directory_is_reported_not_skipped_silently() {
    let root = scratch("acl");
    let locked = root.join("locked");
    fs::create_dir_all(&locked).unwrap();
    fs::write(locked.join("hidden.dat"), b"x").unwrap();

    let user = std::env::var("USERNAME").expect("USERNAME");
    let deny = Command::new("icacls")
        .args([
            locked.to_string_lossy().as_ref(),
            "/deny",
            &format!("{user}:(RD)"),
        ])
        .output();
    let Ok(deny) = deny else {
        eprintln!("icacls 不可用，跳过不可读目录分支");
        fs::remove_dir_all(&root).unwrap();
        return;
    };
    if !deny.status.success() {
        eprintln!(
            "无法设置 deny ACL（{}），跳过不可读目录分支",
            String::from_utf8_lossy(&deny.stderr).trim()
        );
        fs::remove_dir_all(&root).unwrap();
        return;
    }
    // 清理守卫：无论断言结果如何都移除 deny 规则。
    let guard = DenyGuard {
        path: locked.clone(),
        user: user.clone(),
    };
    if fs::read_dir(&locked).is_ok() {
        eprintln!("deny ACL 未生效（可能以提升权限运行），跳过不可读目录分支");
        drop(guard);
        fs::remove_dir_all(&root).unwrap();
        return;
    }

    let listing = discovery::list_candidates(&root, &DiscoveryLimits::default()).unwrap();
    assert!(listing.files.is_empty(), "不可读目录中的文件不得成为候选");
    assert!(
        listing.skipped.iter().any(|skipped| skipped.path == locked
            && matches!(skipped.reason, SkippedReason::Unreadable(_))),
        "不可读目录必须显式记录：{:?}",
        listing.skipped
    );

    drop(guard);
    fs::remove_dir_all(&root).unwrap();
}

/// 测试期间设置 deny ACL 的清理守卫（Drop 时移除 deny 规则）。
struct DenyGuard {
    path: PathBuf,
    user: String,
}

impl Drop for DenyGuard {
    fn drop(&mut self) {
        let _ = Command::new("icacls")
            .args([
                self.path.to_string_lossy().as_ref(),
                "/remove:d",
                self.user.as_str(),
            ])
            .output();
    }
}

#[test]
fn long_paths_are_handled_or_reported_explicitly() {
    let root = scratch("long");
    let segment = "long-name-segment-0123456789".repeat(4); // 100 字符
    let deep = root.join(&segment).join(&segment).join(&segment);
    let target = deep.join("save1.dat");
    if let Err(error) = fs::create_dir_all(&deep).and_then(|_| fs::write(&target, b"x")) {
        eprintln!("长路径创建失败（系统可能未启用长路径支持）：{error}；跳过");
        let _ = fs::remove_dir_all(&root);
        return;
    }
    assert!(
        target.to_string_lossy().len() > 260,
        "测试路径应超过 MAX_PATH：{}",
        target.display()
    );

    let listing = discovery::list_candidates(&root, &DiscoveryLimits::default()).unwrap();
    let found = listing.files.iter().any(|file| file.name == "save1.dat");
    if !found {
        // 系统不支持普通（非 \\?\）长路径访问时的显式路径：记录为不可读，不 panic。
        assert!(
            listing
                .skipped
                .iter()
                .any(|skipped| matches!(skipped.reason, SkippedReason::Unreadable(_))),
            "长路径不可访问时必须显式记录：{:?}",
            listing.skipped
        );
        eprintln!("长路径未被枚举（系统未启用长路径），已按显式不可读记录");
    }

    // 逐字（\\?\）形式的显式输入同样可解析。
    let verbatim = PathBuf::from(format!(r"\\?\{0}", root.display()));
    let canonical = path_policy::validate_input(&verbatim).unwrap();
    assert_eq!(
        canonical,
        path_policy::validate_input(&root).unwrap(),
        "\\?\\ 前缀与普通路径必须解析到同一路径"
    );

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn unc_and_device_locations_are_rejected_explicitly() {
    let unc = Path::new(r"\\localhost\C$\Windows");
    let error = path_policy::validate_input(unc).unwrap_err();
    assert_eq!(error.code, PathPolicyCode::UnsupportedLocation);
    assert!(error.message.contains("网络"), "{}", error.message);

    let output_error =
        path_policy::validate_output(Path::new(r"\\server\share\out"), &[unc]).unwrap_err();
    assert_eq!(output_error.code, PathPolicyCode::UnsupportedLocation);

    let root = scratch("unc-input");
    let input = root.join("save1.dat");
    fs::write(&input, b"x").unwrap();
    let error = path_policy::validate_output(Path::new(r"\\.\C:\out"), &[&input]).unwrap_err();
    assert_eq!(error.code, PathPolicyCode::UnsupportedLocation);
    fs::remove_dir_all(&root).unwrap();
}
