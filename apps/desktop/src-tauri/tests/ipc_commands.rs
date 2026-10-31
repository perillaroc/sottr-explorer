//! SX-014 IPC 集成测试：通过 Tauri MockRuntime 走真实命令通道（参数反序列化、
//! 会话状态、契约信封），覆盖非法 IPC 参数、取消、并发请求、晚到旧请求、
//! 跨目录导出与攻略链接校验。
//!
//! 这些测试不访问网络、不启动浏览器、不写回任何输入：合成夹具在临时目录，
//! 链接打开器替换为记录器，导出写入临时目录下的新目录。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewWindow};

use sottr_core::domain::context::DeclaredContext;
use sottr_core::limits::Limits;
use sottr_core::report::{
    ContextSource, DesktopResult, ExportDocument, Freshness, InputDigest, Mode,
};
use sottr_desktop_lib::commands::{
    CODE_INVALID_PARAMS, CODE_LINK_LOCATION_UNKNOWN, CODE_LINK_SOURCE_UNKNOWN,
    CODE_LINK_URL_UNSUPPORTED, CODE_SNAPSHOT_MISMATCH, CODE_SNAPSHOT_UNKNOWN,
    CODE_UNSUPPORTED_LOCATION,
};
use sottr_desktop_lib::session::{
    CommitOutcome, DesktopState, Session, StoredSnapshot, UrlOpener, embedded_metadata,
};

type MockRuntime = tauri::test::MockRuntime;
type MockWebview = WebviewWindow<MockRuntime>;

// ---------------------------------------------------------------------------
// 测试基础设施
// ---------------------------------------------------------------------------

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "sottr-desktop-s014-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("临时目录必须可创建");
        Self { path }
    }

    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    fn display(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct TestApp {
    #[allow(dead_code)]
    app: tauri::App<MockRuntime>,
    webview: MockWebview,
    opened_urls: Arc<Mutex<Vec<String>>>,
}

fn build_app() -> TestApp {
    let app = sottr_desktop_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("MockRuntime 应用应能构建");
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("mock 主窗口应能创建");
    let opened_urls = Arc::new(Mutex::new(Vec::new()));
    let recorder = opened_urls.clone();
    let opener: UrlOpener = Arc::new(move |url: &str| {
        recorder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(url.to_string());
        Ok(())
    });
    app.state::<DesktopState>().set_url_opener(opener);
    TestApp {
        app,
        webview,
        opened_urls,
    }
}

fn invoke(webview: &MockWebview, command: &str, args: Value) -> Result<Value, Value> {
    let request = InvokeRequest {
        cmd: command.to_string(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: if cfg!(any(windows, target_os = "android")) {
            "http://tauri.localhost".parse().expect("本地 IPC URL")
        } else {
            "tauri://localhost".parse().expect("本地 IPC URL")
        },
        body: InvokeBody::Json(args),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    get_ipc_response(webview, request)
        .map(|body| body.deserialize::<Value>().expect("命令响应必须是 JSON"))
}

fn desktop_result(value: &Value) -> DesktopResult {
    serde_json::from_value(value.clone()).expect("DesktopResult 必须符合 SX-003 契约")
}

fn error_code(value: &Value) -> String {
    value["code"]
        .as_str()
        .unwrap_or_else(|| panic!("命令错误必须携带 code：{value}"))
        .to_string()
}

/// 合成 4 MiB 战役样本（合法容器/区段/记录；不代表真实存档语义）。
fn write_campaign_fixture(path: &Path, prefix_ids: &[u32]) {
    use sottr_core::synthetic::{SampleBuilder, compress};
    let mut builder = SampleBuilder::new();
    for (index, id) in prefix_ids.iter().enumerate() {
        builder.write_prefix_slot(index, *id, 0);
    }
    builder.push_section(28, 4108);
    let raw = builder.finish();
    std::fs::write(path, compress(&raw)).expect("写合成夹具");
}

/// 较大的不可压缩输入（读取与摘要耗时可观测，用于取消/并发时序）。
fn write_slow_fixture(path: &Path, megabytes: usize) {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut bytes = Vec::with_capacity(megabytes * 1024 * 1024);
    while bytes.len() < megabytes * 1024 * 1024 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.extend_from_slice(&state.to_le_bytes());
    }
    std::fs::write(path, &bytes).expect("写慢速夹具");
}

fn analyze(webview: &MockWebview, request_id: &str, path: &Path, context: &str) -> DesktopResult {
    let value = invoke(
        webview,
        "analyze_file",
        json!({
            "requestId": request_id,
            "inputPath": path.to_string_lossy(),
            "context": context,
        }),
    )
    .expect("合法参数应返回结果信封");
    desktop_result(&value)
}

fn expected_direct_report(path: &Path, mode: Mode) -> sottr_core::report::Report {
    let bytes = std::fs::read(path).expect("读取夹具");
    let metadata = embedded_metadata().expect("内嵌元数据必须可加载");
    let declared = DeclaredContext {
        platform: None,
        mode,
        build: None,
        dlc: None,
        source: ContextSource::UserDeclared,
    };
    sottr_core::analyze::analyze_bytes(&bytes, &declared, &metadata, &Limits::default())
        .expect("核心管线应产生报告")
}

fn load_fixture_report() -> sottr_core::report::Report {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/contract/valid/report.valid.json");
    let text = std::fs::read_to_string(path).expect("必须存在 report.valid.json");
    serde_json::from_str(&text).expect("契约夹具必须可反序列化为 Report")
}

fn fake_snapshot(session: &Session, snapshot_id: &str, path: &Path, seq: u64) -> StoredSnapshot {
    StoredSnapshot {
        input_ref: "save1.dat".to_string(),
        input_path: path.to_path_buf(),
        snapshot_id: snapshot_id.to_string(),
        snapshot_time: "2026-10-16T00:00:00.000Z".to_string(),
        freshness: Freshness::Current,
        last_refresh_error: None,
        report: load_fixture_report(),
        metadata: session.metadata().expect("内嵌元数据"),
        seq,
    }
}

// ---------------------------------------------------------------------------
// 分析：同核心、信封身份、路径不泄漏
// ---------------------------------------------------------------------------

#[test]
fn analyze_matches_core_pipeline_and_keeps_private_paths_out() {
    let temp = TempDir::new("analyze");
    let path = temp.join("save1.dat");
    // 0x0F28B6EC 是参考集合内的圣胡安普通贮藏点（合成输入只用于管线一致性）。
    write_campaign_fixture(&path, &[0x0F28B6EC]);
    let app = build_app();

    let result = analyze(&app.webview, "req-1", &path, "campaign");
    assert_eq!(result.request_id, "req-1");
    assert_eq!(result.input_ref, "save1.dat", "信封只带文件名，不带目录");
    assert_eq!(result.freshness, Freshness::Current);
    let snapshot_id = result.snapshot_id.as_deref().expect("必须携带快照标识");
    assert!(snapshot_id.starts_with("snap-"), "{snapshot_id}");
    sottr_platform::time::validate_rfc3339(
        result.snapshot_time.as_deref().expect("必须携带快照时间"),
    )
    .expect("snapshot_time 必须是 RFC 3339");
    assert!(result.last_refresh_error.is_none());
    assert!(result.error.is_none());

    let report = result.report.expect("必须有报告");
    assert_eq!(report.context.mode, Mode::Campaign);
    assert_eq!(report.context.source, ContextSource::UserDeclared);
    assert_eq!(
        report,
        expected_direct_report(&path, Mode::Campaign),
        "桌面命令与核心管线必须产生同一业务报告"
    );
    assert!(
        report
            .locations
            .iter()
            .any(|location| location.guide_group_id.as_deref() == Some("stella:san-juan-caches")),
        "合成战役样本应带圣胡安攻略组线索"
    );

    // 信封不含私人绝对路径（只以文件名引用输入）。
    let text = serde_json::to_string(&json!({ "snapshot": snapshot_id })).unwrap();
    assert!(!text.contains(&temp.display()));
    let envelope_text = serde_json::to_string(&serde_json::to_value(&report).unwrap()).unwrap();
    assert!(
        !envelope_text.contains(&temp.display().replace('\\', "\\\\")),
        "报告序列化后不得包含输入目录"
    );
}

#[test]
fn analyze_rejects_invalid_ipc_params_and_maps_input_errors() {
    let temp = TempDir::new("params");
    let app = build_app();

    let error = invoke(
        &app.webview,
        "analyze_file",
        json!({"requestId": "", "inputPath": "x", "context": "campaign"}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS, "空 request_id");

    let error = invoke(
        &app.webview,
        "analyze_file",
        json!({"requestId": "req-x", "inputPath": "x", "context": "profile"}),
    )
    .unwrap_err();
    assert_eq!(
        error_code(&error),
        CODE_INVALID_PARAMS,
        "显式 profile 声明被拒绝"
    );

    let error = invoke(
        &app.webview,
        "analyze_file",
        json!({"requestId": "req-x", "inputPath": "   ", "context": "unknown"}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS, "空路径");

    let error = invoke(&app.webview, "not_a_command", json!({})).unwrap_err();
    let message = error
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string());
    assert!(
        message.contains("not_a_command"),
        "未知命令必须被拒绝：{message}"
    );

    let missing = temp.join("missing.dat");
    let result = analyze(&app.webview, "req-missing", &missing, "campaign");
    assert_eq!(result.freshness, Freshness::None);
    assert!(result.report.is_none());
    assert!(result.snapshot_id.is_none());
    let error = result.error.expect("缺失文件必须返回输入错误");
    assert_eq!(error.code, "INPUT_UNREADABLE");
    assert!(error.retryable);

    let directory = analyze(&app.webview, "req-dir", &temp.path, "campaign");
    let error = directory.error.expect("目录不是可分析输入");
    assert_eq!(error.code, "INPUT_UNREADABLE");

    #[cfg(windows)]
    {
        let error = invoke(
            &app.webview,
            "analyze_file",
            json!({
                "requestId": "req-unc",
                "inputPath": r"\\server\share\save1.dat",
                "context": "campaign",
            }),
        )
        .unwrap_err();
        assert_eq!(
            error_code(&error),
            CODE_UNSUPPORTED_LOCATION,
            "UNC 明确拒绝"
        );
    }
}

#[test]
fn profile_file_is_always_treated_as_profile_document() {
    let temp = TempDir::new("profile");
    let path = temp.join("profile.dat");
    write_campaign_fixture(&path, &[]);
    let app = build_app();

    let result = analyze(&app.webview, "req-profile", &path, "campaign");
    let report = result.report.expect("资料文件仍产生容器观察报告");
    assert_eq!(
        report.context.mode,
        Mode::Profile,
        "批量 campaign 声明不得覆盖 profile 排除"
    );
    assert_eq!(report, expected_direct_report(&path, Mode::Profile));
}

// ---------------------------------------------------------------------------
// 取消、并发与晚到旧请求
// ---------------------------------------------------------------------------

#[test]
fn cancel_inflight_request_returns_no_result_and_allows_retry() {
    let temp = TempDir::new("cancel");
    let path = temp.join("save1.dat");
    write_slow_fixture(&path, 24);
    let app = build_app();

    let webview = app.webview.clone();
    let path_text = path.to_string_lossy().into_owned();
    let analysis = thread::spawn(move || {
        invoke(
            &webview,
            "analyze_file",
            json!({"requestId": "req-cancel", "inputPath": path_text, "context": "campaign"}),
        )
    });

    // 等请求进入在飞状态后取消（轮询 IPC 取消命令，避免依赖固定休眠时长）。
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut cancelled = false;
    while Instant::now() < deadline {
        let value = invoke(
            &app.webview,
            "cancel_analysis",
            json!({"requestId": "req-cancel"}),
        )
        .expect("取消命令必须可用");
        if value["cancelled"] == json!(true) {
            cancelled = true;
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(cancelled, "分析应在时限内保持可取消状态");

    let value = analysis.join().expect("分析线程").expect("取消仍返回信封");
    let result = desktop_result(&value);
    assert_eq!(result.request_id, "req-cancel");
    assert_eq!(result.freshness, Freshness::None, "取消不返回完整成功");
    assert!(result.report.is_none());
    assert!(result.error.is_none(), "取消不是输入错误");
    assert!(result.snapshot_id.is_none());

    // 取消后同一输入的新请求正常（旧请求不残留状态）。
    let retry_value = invoke(
        &app.webview,
        "cancel_analysis",
        json!({"requestId": "req-cancel"}),
    )
    .expect("重复取消必须幂等");
    assert_eq!(
        retry_value["cancelled"],
        json!(false),
        "已完成请求不能再次取消"
    );

    let small = TempDir::new("cancel-retry");
    let small_path = small.join("save202.dat");
    write_campaign_fixture(&small_path, &[0x0F28B6EC]);
    let retry = analyze(&app.webview, "req-retry", &small_path, "campaign");
    assert_eq!(retry.freshness, Freshness::Current);
    assert!(retry.report.is_some());

    let error = invoke(
        &app.webview,
        "cancel_analysis",
        json!({"requestId": "bad id"}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS);
}

#[test]
fn concurrent_analyses_stay_isolated_by_request_id() {
    let temp = TempDir::new("concurrent");
    let first = temp.join("save1.dat");
    let second = temp.join("save202.dat");
    write_campaign_fixture(&first, &[0x0F28B6EC]);
    write_campaign_fixture(&second, &[0x0F28B6EC, 0x851B4915]);
    let app = build_app();

    let webview_a = app.webview.clone();
    let webview_b = app.webview.clone();
    let path_a = first.to_string_lossy().into_owned();
    let path_b = second.to_string_lossy().into_owned();
    let handle_a = thread::spawn(move || {
        invoke(
            &webview_a,
            "analyze_file",
            json!({"requestId": "req-a", "inputPath": path_a, "context": "campaign"}),
        )
    });
    let handle_b = thread::spawn(move || {
        invoke(
            &webview_b,
            "analyze_file",
            json!({"requestId": "req-b", "inputPath": path_b, "context": "campaign"}),
        )
    });

    let value_a = handle_a.join().expect("线程 A").expect("A 返回信封");
    let value_b = handle_b.join().expect("线程 B").expect("B 返回信封");
    let result_a = desktop_result(&value_a);
    let result_b = desktop_result(&value_b);
    assert_eq!(result_a.request_id, "req-a");
    assert_eq!(result_a.input_ref, "save1.dat");
    assert_eq!(result_b.request_id, "req-b");
    assert_eq!(result_b.input_ref, "save202.dat");
    assert_ne!(result_a.snapshot_id, result_b.snapshot_id);
    assert_eq!(
        result_a
            .report
            .as_ref()
            .expect("A 报告")
            .observations
            .inventory
            .unique_id_count,
        1
    );
    assert_eq!(
        result_b
            .report
            .as_ref()
            .expect("B 报告")
            .observations
            .inventory
            .unique_id_count,
        2
    );
    assert_eq!(
        result_a.report.expect("A 报告"),
        expected_direct_report(&first, Mode::Campaign)
    );
    assert_eq!(
        result_b.report.expect("B 报告"),
        expected_direct_report(&second, Mode::Campaign)
    );

    // 两个输入在会话存储中各占一个当前快照（互不覆盖）。
    let state = app.app.state::<DesktopState>();
    let (has_first, has_second) = state.with_session(|session| {
        (
            session.current_for_path(&first).is_some(),
            session.current_for_path(&second).is_some(),
        )
    });
    assert!(has_first);
    assert!(has_second);
}

#[test]
fn late_old_completion_cannot_replace_new_snapshot() {
    // 直接覆盖会话的顺序保证（确定性，不依赖线程调度）。
    let mut session = Session::new();
    let path = PathBuf::from(r"C:\saves\save1.dat");
    let (_, seq_a) = session.begin_request("req-a", &path).expect("登记 A");
    let (_, seq_b) = session.begin_request("req-b", &path).expect("登记 B");
    assert!(seq_b > seq_a, "同一输入后发起的请求序号更大");

    assert_eq!(
        session.complete_success("req-b", fake_snapshot(&session, "snap-b", &path, seq_b)),
        CommitOutcome::Committed
    );
    assert_eq!(
        session.complete_success("req-a", fake_snapshot(&session, "snap-a", &path, seq_a)),
        CommitOutcome::Superseded,
        "晚到的旧请求不写入存储"
    );
    assert_eq!(
        session
            .current_for_path(&path)
            .expect("当前快照")
            .snapshot_id,
        "snap-b"
    );

    // 重复 request_id 在飞时被拒绝。
    session.begin_request("req-c", &path).expect("登记 C");
    let error = session.begin_request("req-c", &path).unwrap_err();
    assert_eq!(error.code, CODE_INVALID_PARAMS);

    // 取消先于提交：不写入存储，也不返回成功。
    let (_, seq_d) = session.begin_request("req-d", &path).expect("登记 D");
    assert!(session.cancel_request("req-d"));
    assert_eq!(
        session.complete_success("req-d", fake_snapshot(&session, "snap-d", &path, seq_d)),
        CommitOutcome::Cancelled
    );
    assert_eq!(
        session
            .current_for_path(&path)
            .expect("当前快照")
            .snapshot_id,
        "snap-b"
    );

    // 失败刷新把当前快照标记为 stale 并记录错误码（旧结果导出必须携带）。
    let (_, seq_e) = session.begin_request("req-e", &path).expect("登记 E");
    let error = sottr_core::report::InputError::unreadable("测试：文件被占用");
    let outcome = session.complete_failure("req-e", &path, &error);
    match outcome {
        sottr_desktop_lib::session::FailureOutcome::Stale(snapshot) => {
            assert_eq!(snapshot.snapshot_id, "snap-b");
            assert_eq!(snapshot.freshness, Freshness::Stale);
            assert_eq!(
                snapshot.last_refresh_error_code().as_deref(),
                Some("INPUT_UNREADABLE")
            );
        }
        other => panic!("旧快照必须被标记为 stale：{other:?}"),
    }
    assert!(seq_a < seq_e);

    session.complete_cancelled("req-c");
    assert_eq!(session.pending_requests(), 0, "所有请求都已收尾");
}

// ---------------------------------------------------------------------------
// 导出：快照绑定、跨目录保护、过期标记
// ---------------------------------------------------------------------------

#[test]
fn export_binds_snapshot_and_rejects_cross_slot_or_overlapping_targets() {
    let temp = TempDir::new("export");
    // 输出必须与输入不重叠：输出用独立的目录树。
    let outputs = TempDir::new("export-out-root");
    let path = temp.join("save1.dat");
    write_campaign_fixture(&path, &[0x0F28B6EC]);
    let app = build_app();

    let result = analyze(&app.webview, "req-export", &path, "campaign");
    let report = result.report.clone().expect("报告");
    let snapshot_id = result.snapshot_id.clone().expect("快照标识");

    let output = outputs.join("export-out");
    let value = invoke(
        &app.webview,
        "export_report",
        json!({
            "snapshotId": snapshot_id,
            "inputRef": "save1.dat",
            "outputDir": output.to_string_lossy(),
        }),
    )
    .expect("导出应成功");
    assert_eq!(value["freshness"], json!("current"));
    assert_eq!(value["share"], json!(true), "默认分享脱敏");
    assert_eq!(value["artifacts"].as_array().expect("产物清单").len(), 2);

    let json_text = std::fs::read_to_string(output.join("report.json")).expect("report.json");
    let document: ExportDocument = serde_json::from_str(&json_text).expect("导出信封必须符合契约");
    assert_eq!(document.snapshot_id, snapshot_id);
    assert_eq!(document.freshness, Freshness::Current);
    assert_eq!(document.input_digest, report.input_digest);
    assert_eq!(document.report, report, "导出与屏幕报告必须同一快照");
    assert!(
        !json_text.contains(&temp.display()),
        "分享导出不得包含私人绝对路径"
    );
    let csv = std::fs::read_to_string(output.join("report.csv")).expect("report.csv");
    let header = csv.lines().next().expect("CSV 表头");
    assert!(header.contains("snapshot_id") && header.contains("freshness"));
    assert!(header.contains("last_refresh_error_code"));

    // 跨槽导出被拒绝（快照与输入引用必须一致）。
    let error = invoke(
        &app.webview,
        "export_report",
        json!({
            "snapshotId": snapshot_id,
            "inputRef": "save202.dat",
            "outputDir": outputs.join("cross-slot").to_string_lossy(),
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_SNAPSHOT_MISMATCH);

    // 未知快照被拒绝。
    let error = invoke(
        &app.webview,
        "export_report",
        json!({
            "snapshotId": "snap-unknown",
            "inputRef": "save1.dat",
            "outputDir": outputs.join("unknown").to_string_lossy(),
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_SNAPSHOT_UNKNOWN);

    // 输出位于输入所在目录内 → 重叠拒绝（不把报告与存档混放）。
    let error = invoke(
        &app.webview,
        "export_report",
        json!({
            "snapshotId": snapshot_id,
            "inputRef": "save1.dat",
            "outputDir": temp.join("nested").join("out").to_string_lossy(),
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), "INPUT_OUTPUT_OVERLAP");

    // 输出目录已存在 → 拒绝覆盖。
    let existing = outputs.join("existing");
    std::fs::create_dir_all(&existing).expect("创建已存在目录");
    let error = invoke(
        &app.webview,
        "export_report",
        json!({
            "snapshotId": snapshot_id,
            "inputRef": "save1.dat",
            "outputDir": existing.to_string_lossy(),
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), "OUTPUT_EXISTS");

    // 非法格式参数。
    let error = invoke(
        &app.webview,
        "export_report",
        json!({
            "snapshotId": snapshot_id,
            "inputRef": "save1.dat",
            "outputDir": outputs.join("format").to_string_lossy(),
            "format": "xlsx",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS);
}

#[test]
fn failed_refresh_keeps_stale_report_with_error_code() {
    let temp = TempDir::new("stale");
    // 输出必须与输入不重叠：输出用独立的目录树。
    let outputs = TempDir::new("stale-out-root");
    let path = temp.join("save1.dat");
    write_campaign_fixture(&path, &[0x0F28B6EC]);
    let app = build_app();

    let first = analyze(&app.webview, "req-1", &path, "campaign");
    let snapshot_id = first.snapshot_id.clone().expect("快照标识");
    let digest: InputDigest = first.report.as_ref().expect("报告").input_digest.clone();

    // 让输入不可读（模拟“游戏正在保存/文件被移走”），刷新失败。
    std::fs::remove_file(&path).expect("删除夹具模拟刷新失败");
    let second = analyze(&app.webview, "req-2", &path, "campaign");
    assert_eq!(second.freshness, Freshness::Stale);
    assert_eq!(
        second.snapshot_id.as_deref(),
        Some(snapshot_id.as_str()),
        "保留旧快照身份"
    );
    let error = second.error.expect("刷新失败必须携带错误");
    assert_eq!(error.code, "INPUT_UNREADABLE");
    assert_eq!(
        second.last_refresh_error.as_ref().expect("刷新错误").code,
        "INPUT_UNREADABLE"
    );
    let kept = second.report.expect("必须保留旧报告");
    assert_eq!(kept.input_digest, digest, "旧报告摘要与新错误并存");

    // 旧结果导出必须带 stale 标识与刷新错误码（不依赖文件名或截图）。
    let output = outputs.join("stale-export");
    let value = invoke(
        &app.webview,
        "export_report",
        json!({
            "snapshotId": snapshot_id,
            "inputRef": "save1.dat",
            "outputDir": output.to_string_lossy(),
        }),
    )
    .expect("旧结果可导出，但必须带过期标识");
    assert_eq!(value["freshness"], json!("stale"));
    assert_eq!(value["last_refresh_error_code"], json!("INPUT_UNREADABLE"));
    let json_text = std::fs::read_to_string(output.join("report.json")).expect("report.json");
    let document: ExportDocument = serde_json::from_str(&json_text).expect("导出信封");
    assert_eq!(document.freshness, Freshness::Stale);
    assert_eq!(
        document.last_refresh_error_code.as_deref(),
        Some("INPUT_UNREADABLE")
    );
    let csv = std::fs::read_to_string(output.join("report.csv")).expect("report.csv");
    assert!(csv.contains("stale") && csv.contains("INPUT_UNREADABLE"));
}

// ---------------------------------------------------------------------------
// 攻略链接与候选列表
// ---------------------------------------------------------------------------

#[test]
fn guide_links_are_metadata_verified_and_never_carry_private_paths() {
    let temp = TempDir::new("guide");
    let path = temp.join("save1.dat");
    write_campaign_fixture(&path, &[0x0F28B6EC]);
    let app = build_app();

    let result = analyze(&app.webview, "req-guide", &path, "campaign");
    let report = result.report.expect("报告");
    let snapshot_id = result.snapshot_id.expect("快照标识");
    let location_id = "loc:stella:san-juan-caches";
    let location = report
        .locations
        .iter()
        .find(|location| location.location_id == location_id)
        .expect("圣胡安攻略组线索");
    let expected_action = location
        .group_actionability
        .as_ref()
        .expect("攻略组必须带门禁结论")
        .kind;

    let value = invoke(
        &app.webview,
        "open_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": location_id,
            "sourceRef": "src:stella-san-juan",
        }),
    )
    .expect("元数据来源链接应可打开");
    assert_eq!(
        value["url"],
        json!(
            "https://tombraiders.net/stella/walks/TR11walk/collectibles/13-san-juan-murals-caches.html"
        )
    );
    assert_eq!(value["freshness"], json!("current"));
    assert_eq!(
        value["action_kind"],
        serde_json::to_value(expected_action).unwrap(),
        "动作结论来自当前报告的组门禁"
    );
    let opened = app
        .opened_urls
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert_eq!(opened.len(), 1);
    assert!(opened[0].starts_with("https://"), "只允许 http/https");
    assert!(
        !opened[0].contains(&temp.display()) && !opened[0].contains("save1.dat"),
        "链接不附加存档内容或私人路径"
    );

    // 自造目标：来源不属于该定位线索。
    let error = invoke(
        &app.webview,
        "open_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": location_id,
            "sourceRef": "src:trrebootmodtools-dict",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_LINK_SOURCE_UNKNOWN);

    // 未知定位。
    let error = invoke(
        &app.webview,
        "open_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": "loc:not-exist",
            "sourceRef": "src:stella-san-juan",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_LINK_LOCATION_UNKNOWN);

    // 来源在清单中但未提供 URL（只有书面记录）→ 协议/链接不可用。
    let error = invoke(
        &app.webview,
        "open_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": location_id,
            "sourceRef": "src:plan-doc",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_LINK_URL_UNSUPPORTED);

    // 未知快照与空参数。
    let error = invoke(
        &app.webview,
        "open_guide_link",
        json!({
            "snapshotId": "snap-unknown",
            "locationId": location_id,
            "sourceRef": "src:stella-san-juan",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_SNAPSHOT_UNKNOWN);
    let error = invoke(
        &app.webview,
        "open_guide_link",
        json!({"snapshotId": snapshot_id, "locationId": "", "sourceRef": ""}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS);

    // 失败路径不得触发任何打开动作。
    assert_eq!(
        app.opened_urls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        1
    );
}

#[test]
fn resolve_guide_link_verifies_without_opening() {
    let temp = TempDir::new("resolve-guide");
    let path = temp.join("save1.dat");
    write_campaign_fixture(&path, &[0x0F28B6EC]);
    let app = build_app();

    let result = analyze(&app.webview, "req-resolve", &path, "campaign");
    let snapshot_id = result.snapshot_id.expect("快照标识");
    let location_id = "loc:stella:san-juan-caches";

    // 成功解析：返回已校验 URL 与时效，但不触发系统浏览器。
    let value = invoke(
        &app.webview,
        "resolve_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": location_id,
            "sourceRef": "src:stella-san-juan",
        }),
    )
    .expect("元数据来源链接应可解析");
    let url = value["url"].as_str().expect("url 字段");
    assert!(url.starts_with("https://"), "只允许 http/https：{url}");
    assert_eq!(value["freshness"], json!("current"));
    assert!(!url.contains(&temp.display()), "链接不得携带私人路径");
    assert_eq!(
        app.opened_urls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        0,
        "仅解析不得打开系统浏览器"
    );

    // 错误语义与 open_guide_link 一致（未知定位/来源/无 URL/未知快照）。
    let error = invoke(
        &app.webview,
        "resolve_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": "loc:not-exist",
            "sourceRef": "src:stella-san-juan",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_LINK_LOCATION_UNKNOWN);
    let error = invoke(
        &app.webview,
        "resolve_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": location_id,
            "sourceRef": "src:trrebootmodtools-dict",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_LINK_SOURCE_UNKNOWN);
    let error = invoke(
        &app.webview,
        "resolve_guide_link",
        json!({
            "snapshotId": snapshot_id,
            "locationId": location_id,
            "sourceRef": "src:plan-doc",
        }),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_LINK_URL_UNSUPPORTED);
    let error = invoke(
        &app.webview,
        "resolve_guide_link",
        json!({"snapshotId": "snap-unknown", "locationId": location_id, "sourceRef": "src:stella-san-juan"}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_SNAPSHOT_UNKNOWN);
    let error = invoke(
        &app.webview,
        "resolve_guide_link",
        json!({"snapshotId": snapshot_id, "locationId": " ", "sourceRef": " "}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS);
    assert_eq!(
        app.opened_urls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        0,
        "解析失败也不得打开系统浏览器"
    );
}

#[test]
fn open_external_url_opens_synthesized_urls_and_rejects_invalid_ones() {
    let app = build_app();

    // 合成地址（不绑定快照）经白名单校验后交给系统浏览器。
    invoke(
        &app.webview,
        "open_external_url",
        json!({"url": "https://www.bing.com/search?q=%E5%8F%A4%E5%A2%93%E4%B8%BD%E5%BD%B1%20%E6%94%BB%E7%95%A5"}),
    )
    .expect("合成链接应可打开");
    let opened = app
        .opened_urls
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert_eq!(opened.len(), 1);
    assert!(opened[0].starts_with("https://"), "只允许 http/https");

    // 非法地址在产生任何副作用前被拒绝。
    for invalid in [
        "",
        "https://",
        "javascript:alert(1)",
        "file:///C:/secret.dat",
        "https://example.com/has space",
    ] {
        let error = invoke(&app.webview, "open_external_url", json!({"url": invalid})).unwrap_err();
        assert_eq!(error_code(&error), CODE_LINK_URL_UNSUPPORTED, "{invalid}");
    }
    assert_eq!(
        app.opened_urls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        1,
        "被拒绝的地址不得触发打开"
    );
}

#[test]
fn list_candidates_maps_files_and_rejects_bad_roots() {
    let temp = TempDir::new("list");
    std::fs::write(temp.join("save3.dat"), b"x").expect("save3");
    std::fs::write(temp.join("Profile.DAT"), b"x").expect("profile");
    std::fs::write(temp.join("notes.txt"), b"x").expect("txt 被过滤");
    let big = std::fs::File::create(temp.join("big.dat")).expect("big.dat");
    big.set_len(64 * 1024 * 1024 + 1).expect("稀疏超限文件");
    drop(big);
    let app = build_app();

    let value = invoke(
        &app.webview,
        "list_candidates",
        json!({"rootPath": temp.display()}),
    )
    .expect("列候选应成功");
    let files = value["files"].as_array().expect("files 数组");
    let names: Vec<&str> = files
        .iter()
        .map(|file| file["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["big.dat", "Profile.DAT", "save3.dat"]);
    assert_eq!(files[0]["status"], json!("too_large"));
    assert_eq!(files[1]["matches_observed_names"], json!(true));
    assert_eq!(files[2]["status"], json!("ready"));
    assert!(
        value["skipped"]
            .as_array()
            .expect("skipped 数组")
            .is_empty()
    );

    let error = invoke(
        &app.webview,
        "list_candidates",
        json!({"rootPath": temp.join("missing").to_string_lossy()}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), "DISCOVERY_ROOT_MISSING");

    let error = invoke(
        &app.webview,
        "list_candidates",
        json!({"rootPath": temp.join("save3.dat").to_string_lossy()}),
    )
    .unwrap_err();
    assert_eq!(error_code(&error), "DISCOVERY_ROOT_NOT_DIRECTORY");

    let error = invoke(&app.webview, "list_candidates", json!({"rootPath": "  "})).unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS);
}

#[test]
fn discover_saves_returns_stable_status_shape() {
    let app = build_app();
    let value = invoke(&app.webview, "discover_saves", json!({})).expect("发现命令应成功");
    let candidates = value["candidates"].as_array().expect("candidates 数组");
    let known = [
        "game_found",
        "no_game_dir",
        "link_not_followed",
        "missing",
        "not_directory",
        "unreadable",
        "unsupported_location",
    ];
    for candidate in candidates {
        let status = candidate["status"].as_str().expect("status 字符串");
        assert!(known.contains(&status), "未知状态 {status}");
        assert!(candidate["source_label"].as_str().unwrap().chars().count() > 0);
        assert!(
            candidate["source"].as_str().unwrap().contains(':')
                || !candidate["source"].as_str().unwrap().is_empty()
        );
    }
    for root in value["game_roots"].as_array().expect("game_roots 数组") {
        let root = root.as_str().expect("路径字符串");
        assert!(
            root.ends_with("Shadow of the Tomb Raider"),
            "游戏根目录应是 Documents 下的游戏目录：{root}"
        );
    }
}

#[test]
fn select_input_rejects_invalid_kind() {
    let app = build_app();
    let error = invoke(&app.webview, "select_input", json!({"kind": "everything"})).unwrap_err();
    assert_eq!(error_code(&error), CODE_INVALID_PARAMS);
}

// ---------------------------------------------------------------------------
// 真实夹具端到端（可选；供验证记录与 CLI 对照使用）
// ---------------------------------------------------------------------------

/// 设置 `SOTTR_DESKTOP_E2E_INPUT`（真实存档文件）与 `SOTTR_DESKTOP_E2E_OUT`
/// （结果 JSON 路径）时，经 IPC 分析该文件并把 `DesktopResult` 写入输出文件，
/// 供与 CLI 的 `analysis.json` 逐字段对照；未设置时安静跳过（不记为通过）。
///
/// 可选环境变量：
/// - `SOTTR_DESKTOP_E2E_CONTEXT`（默认 `campaign`）；
/// - `SOTTR_DESKTOP_E2E_EXPORT_DIR`（尚不存在的新目录）：在分析之后走
///   `export_report`（JSON+CSV、分享脱敏），并断言产物落盘且不含私人路径；
/// - `SOTTR_DESKTOP_E2E_RESOLVE_SOURCE`：在分析之后走 `resolve_guide_link`
///   校验来源链接（默认尝试 `src:stella-san-juan` 的圣胡安攻略组线索）。
#[test]
fn real_fixture_end_to_end_writes_contract_result() {
    let Ok(input) = std::env::var("SOTTR_DESKTOP_E2E_INPUT") else {
        eprintln!("未设置 SOTTR_DESKTOP_E2E_INPUT，跳过真实夹具端到端对照");
        return;
    };
    let out = std::env::var("SOTTR_DESKTOP_E2E_OUT").expect("必须同时设置 SOTTR_DESKTOP_E2E_OUT");
    let context =
        std::env::var("SOTTR_DESKTOP_E2E_CONTEXT").unwrap_or_else(|_| "campaign".to_string());
    let app = build_app();
    let value = invoke(
        &app.webview,
        "analyze_file",
        json!({"requestId": "req-e2e", "inputPath": input, "context": context}),
    )
    .expect("真实夹具分析必须返回信封");
    let result = desktop_result(&value);
    assert_eq!(result.request_id, "req-e2e");
    assert!(result.report.is_some() || result.error.is_some());
    let text = serde_json::to_string_pretty(&value).expect("序列化端到端结果");
    std::fs::write(&out, text).expect("写端到端结果");
    eprintln!("真实夹具端到端结果已写入：{out}");

    // 端到端导出（可选）：同一快照 → 新目录 → 产物落盘且分享脱敏。
    if let Ok(export_dir) = std::env::var("SOTTR_DESKTOP_E2E_EXPORT_DIR") {
        let snapshot_id = result.snapshot_id.clone().expect("成功结果必须有快照标识");
        let input_ref = result.input_ref.clone();
        let export = invoke(
            &app.webview,
            "export_report",
            json!({
                "snapshotId": snapshot_id,
                "inputRef": input_ref,
                "outputDir": export_dir,
                "format": "both",
                "share": true,
            }),
        )
        .expect("真实快照应可导出");
        assert_eq!(export["share"], json!(true));
        let artifacts = export["artifacts"].as_array().expect("artifacts 数组");
        assert_eq!(artifacts.len(), 2, "JSON+CSV 两个产物");
        for artifact in artifacts {
            let name = artifact["name"].as_str().expect("产物名");
            let path = PathBuf::from(artifact["path"].as_str().expect("产物路径"));
            assert!(path.is_file(), "产物必须落盘：{}", path.display());
            let contents = std::fs::read_to_string(&path).expect("读取产物");
            assert!(
                !contents.contains(&input),
                "{name} 分享导出不得包含输入的绝对路径"
            );
            if let Some(parent) = Path::new(&input).parent() {
                let parent_text = parent.to_string_lossy().into_owned();
                assert!(
                    !contents.contains(&parent_text),
                    "{name} 分享导出不得包含输入所在目录（{parent_text}）"
                );
            }
        }
        eprintln!("真实夹具端到端导出已写入：{export_dir}");
    }

    // 端到端链接解析（可选）：解析后 URL 已校验、打开器未被触发。
    let resolve_source = std::env::var("SOTTR_DESKTOP_E2E_RESOLVE_SOURCE")
        .unwrap_or_else(|_| "src:stella-san-juan".to_string());
    if let Some(report) = result.report.as_ref() {
        let location = report.locations.iter().find(|location| {
            location
                .source_refs
                .iter()
                .any(|source| source == &resolve_source)
        });
        if let Some(location) = location {
            let snapshot_id = result.snapshot_id.clone().expect("成功结果必须有快照标识");
            let opened_before = app
                .opened_urls
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len();
            let link = invoke(
                &app.webview,
                "resolve_guide_link",
                json!({
                    "snapshotId": snapshot_id,
                    "locationId": location.location_id,
                    "sourceRef": resolve_source,
                }),
            )
            .expect("真实报告的攻略来源应可解析");
            let url = link["url"].as_str().expect("url 字段");
            assert!(url.starts_with("https://"), "只允许 http/https：{url}");
            let opened_after = app
                .opened_urls
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len();
            assert_eq!(opened_after, opened_before, "仅解析不得打开系统浏览器");
            eprintln!("真实夹具链接解析：{} -> {url}", location.location_id);
        } else {
            eprintln!("真实报告中未找到含 {resolve_source} 的定位线索，跳过链接解析对照");
        }
    }
}

// ---------------------------------------------------------------------------
// 真实配置下的 ACL 门禁（权限配置生效的运行时证明）
// ---------------------------------------------------------------------------

/// 用真实 `tauri.conf.json`/capabilities/应用权限构建（`generate_context!`）：
/// 证明应用 ACL 门禁确实生效——已登记的应用命令可调用，未获得权限的插件命令
/// （如 `opener`）被拒绝。MockRuntime 不执行真实 WebView 逻辑。
#[test]
fn real_context_acl_allows_app_commands_and_rejects_plugin_commands() {
    let app = sottr_desktop_lib::configure(mock_builder())
        .build(tauri::generate_context!())
        .expect("真实配置应能构建 MockRuntime 应用");
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("mock 主窗口应能创建");

    // 已登记的应用命令：ACL 放行（这里用只读的发现命令）。
    invoke(&webview, "discover_saves", json!({})).expect("已登记命令必须被 ACL 放行");

    // 未获得权限的插件命令：被 ACL 拒绝（前端没有 opener 权限）。
    let error = invoke(
        &webview,
        "plugin:opener|open_url",
        json!({"url": "https://example.com/"}),
    )
    .expect_err("未授权插件命令必须被拒绝");
    let message = error
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string());
    assert!(
        message.contains("not allowed"),
        "拒绝原因必须是 ACL 门禁：{message}"
    );
}

// ---------------------------------------------------------------------------
// 详情弹窗 payload（阶段三）：只存内存、失败不建窗、可重复打开
// ---------------------------------------------------------------------------

#[test]
fn detail_window_payload_round_trips_and_rejects_invalid_payloads() {
    let app = build_app();
    let state = app.app.state::<DesktopState>();

    // 非对象 payload：拒绝且不写状态、不创建窗口。
    let error = invoke(
        &app.webview,
        "open_detail_window",
        json!({"payload": "[1,2,3]"}),
    )
    .expect_err("非对象 payload 必须被拒绝");
    assert_eq!(error["code"], CODE_INVALID_PARAMS);
    assert_eq!(state.detail_payload(), None, "非法 payload 不写内存");
    assert!(
        app.app.get_webview_window("detail").is_none(),
        "非法 payload 不创建窗口"
    );

    // 合法 payload：写入内存、创建 detail 窗口，可读回。
    let payload = json!({"version":1,"selectedId":"0xCAA74459","items":[]}).to_string();
    invoke(
        &app.webview,
        "open_detail_window",
        json!({"payload": payload}),
    )
    .expect("合法 payload 应被接受");
    assert!(app.app.get_webview_window("detail").is_some(), "应创建 detail 窗口");

    let read = invoke(&app.webview, "read_detail_window_payload", json!({}))
        .expect("详情弹窗应能读取 payload");
    assert_eq!(read, json!(payload));

    // 重复打开：复用同一窗口，不报错。
    invoke(
        &app.webview,
        "open_detail_window",
        json!({"payload": payload}),
    )
    .expect("重复打开应复用窗口");
}
