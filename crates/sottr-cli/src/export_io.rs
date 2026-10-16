//! 导出适配（SX-011 / SX-013 / SX-014）：快照时钟。
//!
//! 时钟实现已移至 `sottr_platform::time`（SX-014 起 CLI 与桌面后端共用同一
//! RFC 3339 UTC 格式化与校验），本模块只保留转出以维持既有调用点。
//! 目标与输入的重叠校验在 `sottr_platform::path_policy`（SX-012），独占新目标
//! 与受控临时文件提交在 `sottr_platform::output_commit`（SX-013），二者与桌面
//! 后端共用；稳定快照读取在 `sottr_platform::read_snapshot`（SX-013）。
//! 本模块不再包含写盘逻辑。

pub use sottr_platform::time::{format_rfc3339_utc_millis, unix_millis_now, validate_rfc3339};
