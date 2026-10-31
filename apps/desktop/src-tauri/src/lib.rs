//! SOTTR Explorer 桌面后端（Tauri 最小权限命令层，SX-014）。
//!
//! 当前只实现最小 Builder、命令层、权限配置（`capabilities/default.json` +
//! `permissions/sottr-commands.toml`）、CSP（`tauri.conf.json`）与 IPC 集成测试；
//! 界面（槽位选择、概览、区域清单、详情）由 SX-015/SX-016 实现。
//!
//! 边界（开发方案 SX-014 与「只读安全与隐私设计」）：
//!
//! - 仅开放选择输入、列候选、分析、取消、导出、解析/打开核验链接所需命令；
//!   不开放任意文件读写、shell 执行或外部页面加载；
//! - 分析调用 `sottr_core::analyze::analyze_bytes`（与 CLI 同一核心），返回
//!   SX-003 契约的 [`sottr_core::report::DesktopResult`] 信封；
//! - 目录发现、路径策略、稳定快照与输出提交复用 `sottr-platform`（与 CLI 共用），
//!   不因界面需要放宽路径规则；
//! - WebView 不加载外部攻略页面（导航门禁 + CSP + 无 opener/dialog 前端权限）；
//!   攻略链接由 `commands::open_guide_link` 校验后交给系统外部浏览器，界面合成的
//!   搜索链接走 `commands::open_external_url`（同一 http/https 白名单）；
//! - 默认不携带 Python 运行时或 sidecar；默认无网络、无遥测、无后台更新；
//! - 任何路径都不写回、修复或覆盖原存档。
//!
//! 前端权限清单可审查：`capabilities/default.json` 只给主窗口显式最小核心权限
//! （`core:app/event/path/window`）与本应用命令权限；新增命令必须同时加入
//! `permissions/sottr-commands.toml` 与 capabilities，否则会被 ACL 拒绝
//! （`tests/permission_audit.rs` 交叉核对命令清单、权限文件与 capability）。

pub mod commands;
pub mod navigation;
pub mod session;

use std::sync::Arc;

use tauri::Manager;

/// 窗口销毁时的动作：只有主窗口销毁才取消在飞分析并退出应用。
///
/// 详情弹窗（`detail`）关闭不得取消主窗口在飞的分析——窗口事件必须按标签收窄。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDestroyAction {
    /// 主窗口：取消全部在飞分析、关闭详情窗口并退出应用。
    ShutDown,
    /// 其他窗口（详情弹窗等）：不做任何事。
    Ignore,
}

/// 按窗口标签决定销毁动作（可测的纯函数）。
pub fn window_destroy_action(label: &str) -> WindowDestroyAction {
    if label == commands::MAIN_WINDOW_LABEL {
        WindowDestroyAction::ShutDown
    } else {
        WindowDestroyAction::Ignore
    }
}

/// 组装桌面后端（生产与测试共用）：
///
/// - opener 插件用于在系统外部浏览器打开攻略链接（Rust 端白名单调用，
///   前端没有 `opener:*` 权限）；
/// - dialog 插件仅用于 Rust 端“选择文件/目录”（前端没有 `dialog:*` 权限）；
/// - 导航门禁拒绝 WebView 加载外部页面；
/// - 窗口销毁按标签收窄：只有主窗口销毁才取消在飞分析，并一并关闭详情弹窗。
pub fn configure<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(navigation::navigation_guard())
        .manage(session::DesktopState::new())
        .invoke_handler(tauri::generate_handler![
            commands::discover_saves,
            commands::list_candidates,
            commands::select_input,
            commands::analyze_file,
            commands::cancel_analysis,
            commands::export_report,
            commands::resolve_guide_link,
            commands::open_guide_link,
            commands::open_external_url,
            commands::open_detail_window,
            commands::read_detail_window_payload
        ])
        .setup(|app| {
            // 生产：攻略链接交给系统外部浏览器；WebView 不加载外部页面。
            // （setup 在 `App::run` 时执行；测试直接注入记录器代替。）
            let state = app.state::<session::DesktopState>();
            let opener_handle = app.handle().clone();
            state.set_url_opener(Arc::new(move |url: &str| {
                use tauri_plugin_opener::OpenerExt;
                opener_handle
                    .opener()
                    .open_url(url.to_string(), None::<&str>)
                    .map_err(|error| error.to_string())
            }));
            // 生产：原生文件/目录选择器（前端无 dialog 权限，只能经命令请求）。
            let dialog_handle = app.handle().clone();
            state.set_file_picker(Arc::new(move |kind: &str| {
                use tauri_plugin_dialog::DialogExt;
                let dialog = dialog_handle
                    .dialog()
                    .file()
                    .set_title("选择 SOTTR 存档文件或目录");
                let picked = if kind == "file" {
                    dialog.blocking_pick_file()
                } else {
                    dialog.blocking_pick_folder()
                };
                Ok(picked.and_then(|path| path.into_path().ok()))
            }));
            Ok(())
        })
        .on_window_event(|window, event| {
            if !matches!(event, tauri::WindowEvent::Destroyed) {
                return;
            }
            if window_destroy_action(window.label()) != WindowDestroyAction::ShutDown {
                return;
            }
            // 主窗口关闭：取消全部在飞分析，避免旧回调恢复成功状态。
            window
                .state::<session::DesktopState>()
                .with_session(|session| session.cancel_all());
            // 一并关闭详情弹窗，避免残留窗口阻止应用退出。
            if let Some(detail) = window
                .app_handle()
                .get_webview_window(commands::DETAIL_WINDOW_LABEL)
            {
                let _ = detail.close();
            }
            window.app_handle().exit(0);
        })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    configure(tauri::Builder::default())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_main_window_destroy_shuts_down() {
        // 关闭详情弹窗不得取消主窗口在飞的分析。
        assert_eq!(
            window_destroy_action(commands::MAIN_WINDOW_LABEL),
            WindowDestroyAction::ShutDown
        );
        assert_eq!(
            window_destroy_action(commands::DETAIL_WINDOW_LABEL),
            WindowDestroyAction::Ignore
        );
        assert_eq!(window_destroy_action("other"), WindowDestroyAction::Ignore);
    }
}
