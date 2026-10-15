//! Windows Known Folder 的最小 FFI 封装（SX-012）。
//!
//! 只封装 `SHGetKnownFolderPath(FOLDERID_Documents)` 与 `CoTaskMemFree`：
//! `Documents` 是唯一需要的 Known Folder，OneDrive 重定向后的路径由该 API 直接返回，
//! 与 Python v0.2 研究参照（`research/python-v0.2/sottr_inspect.py::windows_documents`）
//! 使用同一 API 与同一 GUID（其 ctypes 版本作为交叉核对证据）。
//!
//! 不引入 `windows-sys`/`windows` 依赖：仅需要两个稳定导出与一个 GUID 常量，
//! 绑定正确性由本机测试与 `private/` 验证日志中的 PowerShell 独立核对约束。
//! 任何失败（HRESULT 非成功、路径为空）返回 `None`，由调用方回退到其他候选。

use std::path::PathBuf;

/// `SHGetKnownFolderPath` 的 REFKNOWNFOLDERID 结构（Windows GUID 布局）。
#[cfg(windows)]
#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

/// FOLDERID_Documents = {FDD39AD0-238F-46AF-ADB4-6C85480369C7}。
#[cfg(windows)]
const FOLDERID_DOCUMENTS: Guid = Guid {
    data1: 0xFDD3_9AD0,
    data2: 0x238F,
    data3: 0x46AF,
    data4: [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7],
};

/// 逐字长路径上限（UTF-16 单元）；用于在扫描终止符时设置上界。
#[cfg(windows)]
const MAX_WIDE_PATH: usize = 32_768;

#[cfg(windows)]
#[link(name = "shell32")]
unsafe extern "system" {
    /// <https://learn.microsoft.com/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath>
    fn SHGetKnownFolderPath(
        rfid: *const Guid,
        dwflags: u32,
        htoken: *mut core::ffi::c_void,
        ppszpath: *mut *mut u16,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "ole32")]
unsafe extern "system" {
    /// <https://learn.microsoft.com/windows/win32/api/combaseapi/nf-combaseapi-cotaskmemfree>
    fn CoTaskMemFree(pv: *mut core::ffi::c_void);
}

/// Windows Known Folder 的 Documents 路径（已包含 OneDrive 等重定向）。
///
/// 失败时返回 `None`（不 panic、不重试）：由调用方回退到 OneDrive 环境变量与
/// `%USERPROFILE%\Documents` 候选。非 Windows 目标恒为 `None`。
pub fn documents() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;

        let mut raw: *mut u16 = std::ptr::null_mut();
        // SAFETY: 按 SHGetKnownFolderPath 契约调用：rfid 指向 'static GUID 常量，
        // dwflags = 0（KF_FLAG_DEFAULT），htoken 为空（当前用户），ppszpath 指向
        // 已初始化为空的栈变量。成功时 API 分配以 NUL 结尾的 UTF-16 字符串并把
        // 所有权交给调用方；失败时契约要求把输出置为 NULL，故失败路径不读取指针。
        let hr =
            unsafe { SHGetKnownFolderPath(&FOLDERID_DOCUMENTS, 0, std::ptr::null_mut(), &mut raw) };
        if hr < 0 || raw.is_null() {
            return None;
        }
        // SAFETY: hr >= 0 且非空 => raw 指向以 NUL 结尾的 UTF-16 字符串。
        // 扫描在 MAX_WIDE_PATH 处设界，防御异常的未终止缓冲区；先复制为 OsString
        // 再调用 CoTaskMemFree 释放 API 分配的内存（恰好一次）。
        let mut len = 0usize;
        // SAFETY: 逐单元读取，len 始终 < MAX_WIDE_PATH 且有 NUL 终止契约兜底。
        unsafe {
            while len < MAX_WIDE_PATH && *raw.add(len) != 0 {
                len += 1;
            }
        }
        // SAFETY: raw 对所有 len 个单元有效（见上界扫描）。
        let wide = unsafe { std::slice::from_raw_parts(raw, len) };
        let path = PathBuf::from(std::ffi::OsString::from_wide(wide));
        // SAFETY: raw 来自本次成功的 SHGetKnownFolderPath，尚未释放。
        unsafe { CoTaskMemFree(raw.cast()) };
        if path.as_os_str().is_empty() {
            None
        } else {
            Some(path)
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_is_usable_when_present() {
        if let Some(documents) = documents() {
            assert!(
                documents.is_dir(),
                "Known Folder Documents 必须是已存在目录：{}",
                documents.display()
            );
        }
        // Windows 上没有 Documents 属异常环境：显式记录而不是静默跳过。
        #[cfg(windows)]
        assert!(
            documents().is_some(),
            "Windows 环境应提供 Known Folder Documents"
        );
    }
}
