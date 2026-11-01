# 架构总览

## 分层

| 层 | 位置 | 职责 | 明确不做 |
| --- | --- | --- | --- |
| 核心 | `crates/sottr-core` | 容器解析、记录恢复、上下文、计数、参考评估、定位门禁、报告装配、导出序列化 | 不访问文件系统、网络或 UI |
| 平台 | `crates/sottr-platform` | Windows 目录发现、路径策略、稳定快照读取、受控输出提交、快照时钟 | 不做语义解释 |
| 命令行 | `crates/sottr-cli` | `analyze` / `export` / `plan` / `missing` | 不重复核心逻辑 |
| 桌面端 | `apps/desktop` | Tauri 最小权限命令层 + Vue 界面 | 前端不做解析或领域计算 |

核心只接收不可变字节：`analyze_bytes(bytes, declared, metadata, limits) -> Result<Report, InputError>`。
容器无效或超出资源限制返回输入级错误，不产生报告、不做语义解释；局部失败留在报告的能力矩阵
与 `diagnostics` 中。

## 一次分析的流水线

```text
容器 → 区段 → 记录 → 上下文 → 整体候选 / 43 项区域计数
     → 参考视图与分类 → 交叉检查 → 参考评估
     → 定位（组门禁按当前快照评估）
     → 证据 → 覆盖范围 → 诊断 → Report
```

几个顺序上的约束：

- 上下文不适用时，整体候选与区域计数按 `unsupported` + 原因关闭，原始身份观测保留；
- 参考集合当前规则是“恰好一个”；零个或多个时不猜选择规则，只保留原始观测并给诊断；
- 计数交叉检查任一不一致即 `COUNTER_MISMATCH` 降级，不给 `likely_*`；
- 同一 `(code, scope, message)` 诊断去重。

## 三组概念相互独立

| 概念 | 字段 | 取值 |
| --- | --- | --- |
| 业务观测 | `observation` | `present`、`reference_present_current_absent`、`unknown` |
| 业务评估 | `assessment` | `likely_collected`、`likely_missing`、`unknown`、`verified_collected`、`verified_uncollected` |
| 证据等级 | `evidence_level` | `observed`、`community_correlated`、`game_validated` |

不能把它们合并成一个“徽标”：ID 出现只表示已观察；`likely_*` 或 `verified_*` 必须带 `rule_id`；
没有新的游戏内证据时不产生 `verified_*`；不允许无依据的概率分数。完整定义见[契约](/contracts)。

## 报告与信封

核心 `Report` 与三种信封分离：

- `BatchReport`：批量分析外层，每个结果 `report` 与 `error` 恰有一个非空；
- `DesktopResult`：桌面端快照信封，携带 `request_id`、`snapshot_id`、`freshness`、
  最近刷新错误与报告；
- `ExportDocument`：导出信封，`document_kind = "report_export"`，JSON 与 CSV 都保留快照身份。

信封只说明本应用最近的分析/刷新状态，不声称与游戏实时同步。

## 桌面端请求与快照

- 每个分析请求以 `request_id` 隔离；晚到的旧结果不写入快照存储。
- 取消标记与结果提交在同一把会话锁内判定：取消先到绝不产生成功报告。
- 内存中每个输入保留最近一次成功快照（上限 16 个输入）；失败刷新把它标为 `stale`
  并记录错误码。
- 输入路径只用于路径策略、去重与输出重叠校验，不出现在信封、分享导出与外部链接中。
- 权限由两个 capability 收窄：`default`（主窗口）与 `detail`（详情弹窗）；
  未列入的命令会被 ACL 拒绝。

组件级清单与界面细节见
[apps/desktop/README.md](https://github.com/perillaroc/sottr-explorer/blob/main/apps/desktop/README.md)，
这里只保留跨模块约束。

## 不变量

1. 只读：读取稳定快照，前后哈希一致才生成报告；不写回、不修复、不改名、不删除。
2. 未知不是零：可空字段用 `null` 表达，且必须带原因；合法零保留 `0`。
3. 不做无依据推断：没有证据不给 `verified_*`，没有完整性不推断缺席。
4. 契约全字段显式：`additionalProperties: false`，契约外字段被拒绝。
5. 资源有上限：压缩/解压各 64 MiB、批量最多 128 个文件；超限是输入级错误。

相关：[数据管线](/development/data-pipeline)、[测试](/development/testing)、
[契约 1.0.0](/contracts)。
