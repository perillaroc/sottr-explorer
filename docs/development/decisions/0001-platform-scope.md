# ADR-0001 平台范围：仅 Windows Steam 版

> 状态：已采纳（长期有效）。来源：[已知限制](../limitations.md)、[安全模型](../security.md)。

## 背景

存档发现、稳定快照读取与路径策略都依赖 Windows 能力：Known Folder 与 `%USERPROFILE%`
下的受限位置、本地卷判断、文件身份与写入时间。非 Steam 安装、修改过的存档与其它平台
不在验证范围内。2026-10-17 起移除了 OneDrive 目录发现。

## 选项

| 选项 | 说明 | 结果 |
| --- | --- | --- |
| Windows Steam 单一目标 | 只针对 Steam 版实现目录发现、快照与路径策略 | **采纳** |
| 多平台 / 其它商店 | 需要额外的目录发现、快照与测试矩阵，并维护多平台分支 | 不采纳 |

## 决定

仅支持 Windows Steam 版；不维护多平台分支。

## 后果

- 存档发现只查受限位置，不做全盘扫描；输入输出只支持本地卷，拒绝 UNC 与设备路径。
- 手工验收以 Windows WebView2 为准（见 P5-2）。
- 其它平台、非 Steam 安装与修改过的存档不保证结果，也不作为缺陷处理。

## 关联文档

- [已知限制](../limitations.md)
- [安全模型](../security.md)
- [快速开始](/guide/getting-started)
