//! `GetFileInformationByHandle` 的最小 FFI 封装（SX-013）。
//!
//! 为什么不用 std：`std::os::windows::fs::MetadataExt::file_index` 与
//! `volume_serial_number` 仍是不稳定接口（feature `windows_by_handle`），
//! stable 工具链不可用；本模块直接读取 `BY_HANDLE_FILE_INFORMATION` 的同名字段，
//! 只做只读查询、没有副作用。绑定正确性由单元测试与
//! `private/s013_verification.log` 中的 `fsutil file queryfileid` 独立核对约束。

use crate::read_snapshot::FileIdentity;

#[cfg(windows)]
#[repr(C)]
struct ByHandleFileInformation {
    file_attributes: u32,
    creation_time: [u32; 2],
    last_access_time: [u32; 2],
    last_write_time: [u32; 2],
    volume_serial_number: u32,
    file_size_high: u32,
    file_size_low: u32,
    number_of_links: u32,
    file_index_high: u32,
    file_index_low: u32,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    /// <https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle>
    fn GetFileInformationByHandle(
        handle: *mut core::ffi::c_void,
        info: *mut ByHandleFileInformation,
    ) -> i32;
}

/// 读取文件句柄的身份（卷序列号 + 文件索引）；失败返回 `None`。
///
/// 句柄必须已经打开（本调用不改变文件位置、不读取内容）。非 Windows 返回 `None`。
#[cfg(windows)]
pub fn identity(file: &std::fs::File) -> Option<FileIdentity> {
    use std::os::windows::io::AsRawHandle;

    let mut info = std::mem::MaybeUninit::<ByHandleFileInformation>::uninit();
    // SAFETY: 句柄来自存活 `File` 借期的 as_raw_handle（调用期间有效）；
    // info 指向可写栈缓冲；契约：成功时写满结构体并返回非零，失败时不读取。
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), info.as_mut_ptr()) };
    if ok == 0 {
        return None;
    }
    // SAFETY: 仅在上面的调用成功（非零）后读取已初始化的结构体。
    let info = unsafe { info.assume_init() };
    Some(FileIdentity {
        volume: u64::from(info.volume_serial_number),
        file_index: (u64::from(info.file_index_high) << 32) | u64::from(info.file_index_low),
    })
}

/// 非 Windows：不提供句柄身份。
#[cfg(not(windows))]
pub fn identity(_file: &std::fs::File) -> Option<FileIdentity> {
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn identity_is_stable_for_same_file_and_differs_for_others() {
        let root =
            std::env::temp_dir().join(format!("sottr-platform-identity-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("a.dat");
        let second = root.join("b.dat");
        std::fs::write(&first, b"a").unwrap();
        std::fs::write(&second, b"b").unwrap();

        let handle = std::fs::File::open(&first).unwrap();
        let first_identity = identity(&handle).expect("Windows 应提供文件身份");
        assert_eq!(
            identity(&handle),
            Some(first_identity),
            "同一句柄身份必须稳定"
        );
        let reopened = std::fs::File::open(&first).unwrap();
        assert_eq!(
            identity(&reopened),
            Some(first_identity),
            "重新打开同一文件身份必须一致"
        );

        let other = std::fs::File::open(&second).unwrap();
        assert_ne!(
            identity(&other),
            Some(first_identity),
            "不同文件身份必须不同"
        );

        // 替换路径上的文件：新文件身份必须不同（旧句柄仍指向旧文件）。
        let other_identity = identity(&other).unwrap();
        let replacement = root.join("c.dat");
        std::fs::write(&replacement, b"b").unwrap();
        std::fs::rename(&replacement, &second).unwrap();
        let replaced = std::fs::File::open(&second).unwrap();
        assert_ne!(
            identity(&replaced).map(|new| new.file_index),
            Some(other_identity.file_index),
            "改名替换后索引应变化"
        );

        drop(handle);
        drop(reopened);
        drop(other);
        drop(replaced);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
