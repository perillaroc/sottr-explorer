# ADR-0004 证据分层：观察、评估、证据等级三者独立

> 状态：已采纳（长期有效）。来源：[架构总览](../architecture.md)、[接口契约](/contracts)。

## 背景

“ID 出现在物品表里”经常被直接读成“已收集”。但原始观察、业务评估与证据强度是
三件不同的事，合并成一个徽标会让错误结论无法追溯。

## 选项

| 选项 | 说明 | 结果 |
| --- | --- | --- |
| 三个字段独立 | `observation`、`assessment`、`evidence_level` 各表一件事 | **采纳** |
| 合并成单一“可信”标记 | 展示简单，但丢失推理过程与规则来源 | 拒绝 |

## 决定

三组概念相互独立：

- `observation`：`present`、`reference_present_current_absent`、`unknown`；
- `assessment`：`likely_collected`、`likely_missing`、`unknown`、`verified_collected`、`verified_uncollected`；
- `evidence_level`：`observed`、`community_correlated`、`game_validated`。

`likely_*` 与 `verified_*` 必须带 `rule_id`；没有新的游戏内证据时不产生 `verified_*`。

## 后果

- 界面分开呈现观察与评估，不把 `present` 写成“已收集”。
- 参考差集本身只构成观察，不自动成为 `likely_missing`；由规则与门禁决定。
- 计数交叉检查不一致时撤销 `likely_*` 并降级，保留原始观测。

## 关联文档

- [架构总览](../architecture.md)（三组概念）
- [接口契约](/contracts)
- [术语表](/reference/glossary)
