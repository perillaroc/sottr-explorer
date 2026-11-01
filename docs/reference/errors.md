# 错误码与诊断

所有码都是稳定标识；界面只看 code / severity / scope，不解析自然语言消息。完整定义以
[契约](/contracts)与源码枚举为准，本页做分组索引。

## 输入级错误

容器无效或超出资源限制时返回，**不产生报告**，`retryable = false`。

| 码 | 含义 |
| --- | --- |
| `INPUT_UNREADABLE` | 文件不可读或路径策略拒绝 |
| `DECOMPRESSION_FAILED` | zlib 解压失败、校验和或 EOF 无效 |
| `RESOURCE_LIMIT` | 压缩或解压超过 64 MiB、文件数超限 |

## 分析诊断

| 组 | 码 |
| --- | --- |
| 上下文发现 | `MODE_*`、`BUILD_UNKNOWN_*`、`DLC_UNKNOWN_*` |
| 容器 | `CONTAINER_TRAILING_BYTES` |
| 计数 | `COUNTER_MAPPING_INVALID`、`COUNTER_MISMATCH` |
| 元数据 | `METADATA_UNAVAILABLE`、`METADATA_CONFLICT` |
| 布局 | `UNSUPPORTED_LAYOUT`、`UNSUPPORTED_CONTEXT` |
| 定位 | `LOCATION_UNVERIFIED` |

同一 `(code, scope, message)` 会去重。

## 组级定位门禁

决定 `group_actionability` 的原因码：

| 结果 | 码 |
| --- | --- |
| 整组可查（`whole_group_check`） | `COMPLETE_TABLE`、`ZERO_GROUP_COUNT`、`REFERENCE_MATCH` |
| 降为 `general_reference` | `GROUP_COUNT_NONZERO`、`GROUP_COUNT_UNKNOWN`、`GROUP_COUNT_CONFLICT`、`REFERENCE_NOT_APPLICABLE`、`GROUP_ASSOCIATION_UNCONFIRMED` |
| 关闭行动摘要（`unavailable`） | `INVENTORY_INCOMPLETE`、`INVENTORY_UNAVAILABLE`、`CONTEXT_NOT_CAMPAIGN`、`SNAPSHOT_NOT_CURRENT` |

输出绑定 `evaluated_input_digest`，每次刷新或切槽都必须重新评估。

## 元数据加载

`METADATA_TOO_LARGE`、`METADATA_INVALID_JSON`、`METADATA_NON_STANDARD_NUMBER`、
`METADATA_DUPLICATE_KEY`、`METADATA_LIMIT_EXCEEDED`、`METADATA_INVALID_VALUE`、
`METADATA_INVALID_URL`、`METADATA_KEY_MISMATCH`、`METADATA_COUNTER_MAPPING_INVALID`、
`METADATA_UNRESOLVED_REFERENCE`、`METADATA_VERSION_CONFLICT`、`METADATA_ENTRY_CONFLICT`。

## 桌面命令

`INVALID_PARAMS`、`INTERNAL`、`METADATA_UNAVAILABLE`、`SNAPSHOT_UNKNOWN`、
`SNAPSHOT_MISMATCH`、`LINK_LOCATION_UNKNOWN`、`LINK_SOURCE_UNKNOWN`、
`LINK_URL_UNSUPPORTED`、`LINK_OPEN_FAILED`、`UNSUPPORTED_LOCATION`、
`DISCOVERY_*`、`PATH_*`、`OUTPUT_*`。

## 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 全部成功 |
| `1` | 至少一个输入级错误（批量 JSON 仍写出全部结果） |
| `2` | 用法或配置错误 |
