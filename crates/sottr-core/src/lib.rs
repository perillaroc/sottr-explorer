//! sottr-core：SOTTR Explorer 的只读解析与领域核心。
//!
//! 边界与约束（自 SX-002 起保持）：
//! - 纯库：不访问 UI、网络或全盘目录；不接收路径，输入为调用方提供的不可变字节；
//! - 解析与领域模块不依赖 Tauri；CLI 与桌面端调用同一核心；
//! - 未知与不适用显式为 unknown / unsupported，不用 0 或空清单冒充结论；
//! - observation 与 assessment 分离，证据等级可追溯（见开发方案 G1）。
//!
//! 模块划分（开发方案建议）：
//! - [`limits`]：资源上限（有界解压、循环与列表）
//! - [`container`]：zlib 容器与有界解压
//! - [`sections`]：主区段流步进与边界检查
//! - [`inventory`]：前缀与溢出记录恢复、完整性判定
//! - [`domain`]：上下文、进度能力、参考差集与证据评估
//! - [`metadata`]：版本化元数据加载与校验
//! - [`report`]：报告契约类型、信封与序列化
//! - [`synthetic`]：合成夹具生成器（测试专用）
//!
//! 当前状态：SX-004 实现容器与区段步进，SX-005 实现记录恢复与完整性；
//! 上下文与评估见 G1 后续任务（SX-006/SX-007）。

pub mod container;
pub mod domain;
pub mod inventory;
pub mod limits;
pub mod metadata;
pub mod report;
pub mod sections;
pub mod synthetic;
