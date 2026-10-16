//! WebView 导航门禁（SX-014）：WebView 只允许加载应用自身页面。
//!
//! 外部攻略页面一律不进入 WebView：攻略链接由后端命令 `open_guide_link`
//! 校验后交给系统外部浏览器打开。即使前端出现意外外链被点击，导航也会在
//! 此处被取消（`false` 表示取消导航），不获得 WebView 浏览权限。

use tauri::Url;

/// 允许的导航目标：应用自身来源（生产自定义协议）与本地开发服务器。
///
/// 生产构建在 Windows 上使用 `http://tauri.localhost`，其他平台为
/// `tauri://localhost`；开发模式使用 `http://localhost:<port>`。除这些本地
/// 来源与 `about:blank` 外，其余（http/https 外部站点、`javascript:`、`file:`
/// 等）一律拒绝。
pub fn is_allowed_navigation(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => matches!(
            url.host_str(),
            Some("tauri.localhost") | Some("localhost") | Some("127.0.0.1")
        ),
        "about" => url.path() == "blank",
        _ => false,
    }
}

/// 导航门禁插件：注册到 Tauri Builder 后对所有 WebView 生效。
pub fn navigation_guard<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("sottr-navigation-guard")
        .on_navigation(|_webview, url| is_allowed_navigation(url))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).expect("测试 URL 必须可解析")
    }

    #[test]
    fn allows_application_origins_only() {
        for allowed in [
            "tauri://localhost/index.html",
            "http://tauri.localhost/",
            "http://localhost:1420/",
            "http://127.0.0.1:1420/index.html",
            "about:blank",
        ] {
            assert!(is_allowed_navigation(&url(allowed)), "{allowed}");
        }
        for denied in [
            "https://tombraiders.net/stella/walks/TR11walk/collectibles/13-san-juan-murals-caches.html",
            "http://example.com/",
            "https://github.com/arcusmaximus/TrRebootModTools",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "ws://localhost:1420/socket",
        ] {
            assert!(!is_allowed_navigation(&url(denied)), "{denied}");
        }
    }
}
