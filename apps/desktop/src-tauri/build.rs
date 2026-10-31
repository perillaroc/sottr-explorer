//! 构建脚本（SOTTR Explorer 桌面后端，SX-014）。
//!
//! `tauri_build::build()` 解析 `tauri.conf.json`、capabilities 与应用命令
//! permissions（启用应用 ACL 门禁），并生成 Windows 资源（图标、版本信息与
//! comctl32 v6 应用清单）。
//!
//! 附加步骤（仅 Windows）：把同一份 Windows 资源也链接进测试目标。
//! tauri-build 默认只把资源链接到二进制目标（`cargo:rustc-link-arg-bins`），
//! 而集成测试（`tests/*.rs`）同样链接 Tauri 桌面依赖，会导入 comctl32 v6 才有
//! 的 `TaskDialogIndirect` 等符号；测试二进制缺少清单时系统加载 comctl32 v5，
//! 进程会以 `STATUS_ENTRYPOINT_NOT_FOUND`(0xc0000139) 直接退出，IPC 集成测试
//! 无法运行。

fn main() {
    // tauri-build 只监听 tauri.conf.json、capabilities 与 permissions；
    // 图标变更不会触发 Windows 资源重新生成，这里显式监听，避免 exe 内嵌旧图标。
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build();
    #[cfg(windows)]
    link_resource_into_tests();
}

/// 把 build 脚本 OUT_DIR 中已生成的 `resource.lib` 追加链接到测试目标。
#[cfg(windows)]
fn link_resource_into_tests() {
    let out_dir = std::env::var("OUT_DIR").expect("cargo 必须提供 OUT_DIR");
    let resource = std::path::Path::new(&out_dir).join("resource.lib");
    if resource.exists() {
        println!("cargo:rustc-link-arg-tests={}", resource.display());
    } else {
        println!(
            "cargo:warning=未找到 {}，测试目标可能缺少 Windows 应用清单",
            resource.display()
        );
    }
}
