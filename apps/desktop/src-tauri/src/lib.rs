//! SOTTR Explorer 桌面端 Tauri 后端（占位）。
//!
//! 当前仅包含最小 Builder；文件选择、分析调度、取消、导出与打开核验链接等
//! 最小权限命令在 SX-014 按 SX-003 契约实现。WebView 不加载外部页面；
//! capabilities 保持最小集合（见 capabilities/default.json）。
//!
//! 注意：任何阶段都不写回、修复或覆盖原存档。

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
