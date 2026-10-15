//! SOTTR Explorer 平台适配层（SX-012 / SX-013）。
//!
//! `sottr-core` 保持纯库：不接收路径、不访问文件系统。Windows 侧的受限目录
//! 发现、路径策略、稳定快照与输出提交由本 crate 承担，供 CLI（`sottr-cli`）与
//! 桌面后端（SX-014 起）**共用同一实现**，避免出现“桌面安全、CLI 绕过”的差异；
//! 操作系统差异止于本层。
//!
//! - [`discovery`]：Windows Known Folder 的 Documents（支持 OneDrive 重定向与
//!   中文路径）、OneDrive 环境变量与 `%USERPROFILE%\Documents` 回退；只在
//!   `Documents\Shadow of the Tomb Raider` 下发现候选，枚举受深度/数量/大小限制，
//!   跳过符号链接与重解析点，不做全盘扫描；候选列表可注入测试根目录。
//! - [`path_policy`]：路径分类（本地 / 逐字本地 / UNC / 设备命名空间）与
//!   输入输出重叠校验（规范路径逐组件比较，不使用字符串前缀）。
//! - [`read_snapshot`]：句柄约束读取（Windows 读取期间拒绝写入/删除/改名）、
//!   读取前后指纹与摘要复核、路径身份复核；输出不可变内存快照。
//! - [`output_commit`]：独占新目标与受控临时文件、提交前链接/覆盖复核、
//!   失败或取消清理，绝不回退为不受控写入。
//!
//! 本层只读输入、只写调用方指定的新输出目录；不写回、不修改权限、不提权。
//! 默认手动刷新，不监视文件；未来加入文件监视必须复用 [`read_snapshot`] 门禁。

pub mod discovery;
pub mod output_commit;
pub mod path_policy;
pub mod read_snapshot;

// 唯一允许 unsafe 的模块：Windows Known Folder 与文件身份的最小 FFI
// （见各模块文档与 crate Cargo.toml 的 lints 说明；代码块均带 SAFETY 注释）。
#[allow(unsafe_code)]
mod windows_file;
#[allow(unsafe_code)]
mod windows_known_folder;
