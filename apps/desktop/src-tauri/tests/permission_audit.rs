//! SX-014 权限与 CSP 静态审查：命令清单、capability 权限、CSP 三方一致。
//!
//! 这些测试不启动应用，只读取仓库内的配置文件与源码，防止以下漂移：
//! 新增命令未登记权限（运行期被 ACL 拒绝）、capability 混入插件直调权限
//! （opener/dialog/shell/fs）、CSP 放行远程内容、handler 清单与审查清单不一致。

use std::collections::BTreeSet;
use std::path::PathBuf;

/// 期望开放的 IPC 命令（与 `src/lib.rs` 的 `generate_handler!` 一致）。
const EXPECTED_COMMANDS: &[&str] = &[
    "discover_saves",
    "list_candidates",
    "select_input",
    "analyze_file",
    "cancel_analysis",
    "export_report",
    "resolve_guide_link",
    "open_guide_link",
    "open_external_url",
    "open_detail_window",
    "read_detail_window_payload",
];

/// 只允许详情弹窗调用的命令（主窗口不授予读取 payload 权限）。
const DETAIL_WINDOW_ONLY_COMMANDS: &[&str] = &["read_detail_window_payload"];

/// 主窗口 capability 的核心权限（显式最小集）。
const MAIN_CORE_PERMISSIONS: &[&str] = &[
    "core:app:default",
    "core:event:default",
    "core:path:default",
    "core:window:default",
];

/// 详情弹窗 capability 的全部权限（只读 payload + 外链 + 关闭自身）。
const DETAIL_CAPABILITY_PERMISSIONS: &[&str] = &[
    "core:event:default",
    "core:window:allow-close",
    "allow-read-detail-window-payload",
    "allow-open-external-url",
];

fn permission_identifier(command: &str) -> String {
    format!("allow-{}", command.replace('_', "-"))
}

fn tauri_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(tauri_dir().join(relative))
        .unwrap_or_else(|error| panic!("必须能读取 {relative}：{error}"))
}

fn expected_permission_identifiers() -> BTreeSet<String> {
    EXPECTED_COMMANDS.iter().map(|command| permission_identifier(command)).collect()
}

/// 读取 capability 的窗口目标与权限列表。
fn capability(relative: &str) -> (Vec<String>, Vec<String>) {
    let text = read(relative);
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("{relative} 必须是合法 JSON：{error}"));
    let windows = value["windows"]
        .as_array()
        .unwrap_or_else(|| panic!("{relative} 必须有 windows 数组"))
        .iter()
        .map(|item| item.as_str().expect("窗口标签必须是字符串").to_string())
        .collect();
    let permissions = value["permissions"]
        .as_array()
        .unwrap_or_else(|| panic!("{relative} 必须有 permissions 数组"))
        .iter()
        .map(|item| item.as_str().expect("权限项必须是字符串").to_string())
        .collect();
    (windows, permissions)
}

#[test]
fn main_capability_grants_app_commands_except_detail_read() {
    let (windows, permissions) = capability("capabilities/default.json");
    assert_eq!(windows, vec!["main".to_string()], "主 capability 只允许主窗口使用");

    let expected: Vec<String> = MAIN_CORE_PERMISSIONS
        .iter()
        .map(|s| s.to_string())
        .chain(
            EXPECTED_COMMANDS
                .iter()
                .filter(|command| !DETAIL_WINDOW_ONLY_COMMANDS.contains(command))
                .map(|command| permission_identifier(command)),
        )
        .collect();
    assert_eq!(permissions, expected, "主窗口权限清单必须与审查清单逐项一致");
    assert!(
        !permissions.contains(&permission_identifier("read_detail_window_payload")),
        "主窗口不授予读取详情 payload 的权限（只有弹窗读取）"
    );

    // capability 的窗口目标必须与 tauri.conf.json 的窗口标签一致（只给主窗口权限）。
    let config_text = read("tauri.conf.json");
    let config: serde_json::Value =
        serde_json::from_str(&config_text).expect("tauri.conf.json 必须是合法 JSON");
    assert_eq!(
        config["app"]["windows"][0]["label"],
        serde_json::json!("main"),
        "窗口标签必须显式为 main，与 capability 的 windows 目标一致"
    );
}

#[test]
fn detail_capability_is_minimal_and_targets_detail_window_only() {
    let text = read("capabilities/detail.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("detail capability 必须是合法 JSON");
    assert_eq!(
        value["windows"],
        serde_json::json!(["detail"]),
        "详情 capability 只允许 detail 窗口"
    );
    let (windows, permissions) = capability("capabilities/detail.json");
    assert_eq!(windows, vec!["detail".to_string()]);
    assert_eq!(
        permissions,
        DETAIL_CAPABILITY_PERMISSIONS.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "详情弹窗只授予读取 payload、外链与关闭自身窗口所需权限"
    );
    assert!(
        text.contains("allow-read-detail-window-payload"),
        "详情窗口必须能读取 Rust 内存中的 payload"
    );
    assert!(
        !text.contains("allow-open-detail-window"),
        "详情窗口不得再次打开详情窗口（单例由主窗口控制）"
    );
}

#[test]
fn neither_capability_grants_plugin_permissions_or_remote_access() {
    for relative in ["capabilities/default.json", "capabilities/detail.json"] {
        let text = read(relative);
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("capability 必须是合法 JSON");
        for forbidden in [
            "opener:",
            "dialog:",
            "shell:",
            "fs:",
            "http:",
            "process:",
            "core:image:",
            "core:menu:",
            "core:tray:",
            "core:resources:",
            "core:webview:",
        ] {
            assert!(
                !text.contains(forbidden),
                "{relative} 不得获得 {forbidden} 权限（外部链接、原生选择器与文件读取由 Rust 命令内部校验）"
            );
        }
        assert!(value.get("remote").is_none(), "{relative} 不得为远程来源开放 capability");
    }
}

#[test]
fn every_command_permission_appears_in_at_least_one_capability() {
    let (_, main_permissions) = capability("capabilities/default.json");
    let (_, detail_permissions) = capability("capabilities/detail.json");
    let granted: BTreeSet<String> = main_permissions
        .iter()
        .chain(detail_permissions.iter())
        .cloned()
        .collect();
    for command in EXPECTED_COMMANDS {
        let identifier = permission_identifier(command);
        assert!(
            granted.contains(&identifier),
            "命令权限 {identifier} 必须出现在 default 或 detail capability 中"
        );
    }
    // 弹窗不能调用「打开详情窗口」，主窗口不能读取（读取只属于弹窗）。
    let main_set: BTreeSet<String> = main_permissions.into_iter().collect();
    let detail_set: BTreeSet<String> = detail_permissions.into_iter().collect();
    assert!(!main_set.contains(&permission_identifier("read_detail_window_payload")));
    assert!(!detail_set.contains(&permission_identifier("open_detail_window")));
}

#[test]
fn permission_file_lists_every_command_exactly_once() {
    let text = read("permissions/sottr-commands.toml");
    let identifiers: BTreeSet<String> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("identifier = "))
        .map(|value| value.trim().trim_matches('"').to_string())
        .collect();
    assert_eq!(identifiers, expected_permission_identifiers());

    let commands: Vec<String> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("commands.allow = ["))
        .flat_map(|rest| rest.trim_end_matches(']').split(','))
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect();
    let expected: BTreeSet<String> = EXPECTED_COMMANDS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        commands.iter().cloned().collect::<BTreeSet<String>>(),
        expected,
        "权限文件的 commands.allow 必须覆盖全部命令且不包含未知命令"
    );
    assert_eq!(commands.len(), expected.len(), "每个命令只登记一次");
}

#[test]
fn handler_list_matches_audited_commands() {
    let text = read("src/lib.rs");
    let start = text
        .find("generate_handler![")
        .expect("lib.rs 必须包含 generate_handler!");
    let end = text[start..]
        .find(']')
        .expect("generate_handler! 必须有结束括号")
        + start;
    let block = &text[start..end];
    let found: BTreeSet<String> = block
        .lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches(',');
            line.strip_prefix("commands::").map(|name| name.to_string())
        })
        .collect();
    assert_eq!(
        found,
        EXPECTED_COMMANDS.iter().map(|s| s.to_string()).collect(),
        "handler 清单与审查清单必须一致；新增/删除命令必须同步本测试与权限文件"
    );
}

#[test]
fn csp_blocks_remote_content_and_stays_local_in_development() {
    let text = read("tauri.conf.json");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("tauri.conf.json 必须是合法 JSON");
    let security = &value["app"]["security"];
    let csp = security["csp"]
        .as_str()
        .expect("必须显式配置 CSP（当前为 null 即代表未设置）");
    for required in [
        "default-src 'self'",
        "script-src 'self'",
        "connect-src 'self' ipc: http://ipc.localhost",
        "object-src 'none'",
        "base-uri 'self'",
        "frame-ancestors 'none'",
        "form-action 'none'",
    ] {
        assert!(
            csp.contains(required),
            "CSP 缺少：{required}（当前：{csp}）"
        );
    }
    assert!(
        !csp.contains('*') && !csp.contains("http: http"),
        "CSP 不得放行任意远程来源：{csp}"
    );
    assert!(
        security.get("dangerousRemoteDomainIpcAccess").is_none(),
        "不得允许远程域访问 IPC"
    );

    let dev_csp = security["devCsp"]
        .as_str()
        .expect("必须显式配置开发 CSP（Vite HMR 需要 WebSocket 例外）");
    assert!(dev_csp.contains("default-src 'self'"));
    assert!(dev_csp.contains("ws://localhost:1420"));
    assert!(
        !dev_csp.contains("https://") && !dev_csp.contains('*'),
        "开发 CSP 也不得放行远程来源：{dev_csp}"
    );
}

#[test]
fn embedded_metadata_matches_repository_manifest() {
    // 桌面端内嵌 data/metadata 三个文件；此测试防止内嵌副本与仓库数据漂移。
    use sottr_desktop_lib::session::EMBEDDED_METADATA_FILES;

    let manifest_path = tauri_dir().join("../../../data/metadata/manifest.json");
    let manifest_text =
        std::fs::read_to_string(&manifest_path).expect("必须存在 data/metadata/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest 必须是合法 JSON");
    let files = manifest["files"]
        .as_array()
        .expect("manifest.files 必须是数组");
    assert_eq!(files.len(), EMBEDDED_METADATA_FILES.len());
    for (file_id, contents) in EMBEDDED_METADATA_FILES {
        let entry = files
            .iter()
            .find(|entry| entry["file_id"] == *file_id)
            .unwrap_or_else(|| panic!("manifest 缺少 {file_id}"));
        let expected_hash = entry["sha256"]
            .as_str()
            .expect("manifest 条目必须有 sha256");
        let actual_hash = sottr_core::hash::sha256_hex(contents.as_bytes());
        assert_eq!(
            actual_hash, expected_hash,
            "内嵌元数据 {file_id} 与 manifest 哈希不一致"
        );
        let expected_bytes = entry["bytes"].as_u64().expect("manifest 条目必须有 bytes");
        assert_eq!(contents.len() as u64, expected_bytes);
    }
}
