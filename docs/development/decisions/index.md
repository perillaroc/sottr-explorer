# 决策记录

这里只记录**已经定下、长期有效**的结论与理由；过程性方案与讨论留在开发工作区的
`doc/<日期>-<主题>/`，不进入站点。每条决策有独立的 ADR 页（背景、选项、决定、后果、
关联文档），本页是稳定入口。

| 决策 | 结论 | ADR |
| --- | --- | --- |
| <a id="platform-scope"></a>平台范围 | 仅 Windows Steam 版 | [ADR-0001](./0001-platform-scope) |
| <a id="read-only"></a>只读 | 不提供修复、写回或改名 | [ADR-0002](./0002-read-only) |
| <a id="unknown-as-null"></a>未知表达 | `null` + 原因，不用 0 或空清单 | [ADR-0003](./0003-unknown-as-null) |
| <a id="evidence-layers"></a>证据分层 | observation / assessment / evidence_level 三者独立 | [ADR-0004](./0004-evidence-layers) |
| <a id="no-probabilities"></a>不做概率 | 不输出 confidence 小数或评分 | [ADR-0005](./0005-no-probabilities) |
| <a id="reference-set"></a>参考集合 | 当前要求“恰好一个” | [ADR-0006](./0006-reference-set-exactly-one) |
| <a id="unit-coordinates"></a>坐标空间 | 每单位独立归一化，无全局地图 | [ADR-0007](./0007-unit-local-coordinates) |
| <a id="basemap-bundling"></a>地图分发 | D4=A，底图随包分发 | [ADR-0008](./0008-bundled-basemaps) |
| <a id="dual-datasets"></a>双数据集 | catalog 负责图鉴/位置，metadata 负责参考/计数 | [ADR-0009](./0009-dual-datasets) |
| <a id="contract-shape"></a>契约形态 | 全字段显式 + 禁止额外字段 | [ADR-0010](./0010-strict-contract) |
| <a id="python-oracle"></a>Python 参照 | v0.2 只作差分回归参照 | [ADR-0011](./0011-python-oracle) |
