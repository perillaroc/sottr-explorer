# 开发文档

这一组文档面向维护者：先读架构与数据管线，再按需要查测试、构建、发布与决策记录。

| 文档 | 解决什么问题 |
| --- | --- |
| [架构总览](/development/architecture) | 代码怎么分层，一次分析怎么流动，有哪些不变量 |
| [数据管线](/development/data-pipeline) | 每个数据集谁生成、谁消费、是否进产品包 |
| [测试](/development/testing) | 有哪些测试、怎么跑、哪些会跳过 |
| [构建](/development/build) | 环境、命令、契约校验、文档站构建 |
| [发布](/development/release) | 版本号、打包清单、发布前检查 |
| [安全模型](/development/security) | 只读、路径策略、网络、IPC 权限与分享脱敏 |
| [已知限制](/development/limitations) | 解析、数据、地图、界面、平台各自做不到什么 |
| [决策记录](/development/decisions/) | 已经定下的关键取舍及理由 |

## 先看这两个约束

1. **核心是纯库**：`sottr-core` 只接收不可变字节，不访问文件系统、网络或 UI。
   任何“读文件”的能力都在 `sottr-platform` 或命令层。
2. **契约先于实现**：字段含义、枚举与单位变更都必须升级契约版本；代码、schema、
   夹具、前端类型四者要一起动。见[契约 1.0.0](/contracts)。
