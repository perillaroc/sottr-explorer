# 接口契约 1.0.0（SX-003 冻结）

本文件描述 sottr-explorer 的版本化接口契约。契约由以下内容共同冻结，任何字段含义、
枚举或单位的变更都必须升级 `schema_version`（见文末“版本策略”）：

| 载体 | 位置 |
| --- | --- |
| JSON Schema | `schemas/report.schema.json`、`schemas/batch.schema.json`、`schemas/desktop-result.schema.json`、`schemas/export.schema.json`、`schemas/metadata.schema.json` |
| 契约夹具 | `fixtures/contract/{valid,invalid,representation,adapted}/` |
| Rust 类型 | `crates/sottr-core/src/report/`（Report、信封）、`crates/sottr-core/src/metadata.rs` |
| 前端类型 | `apps/desktop/src/contract/*.d.ts`（由 schema 生成，勿手改） |
| 验证命令 | `cargo test -p sottr-core`；`cd apps/desktop && pnpm contract:check` |

`schema_version` 固定字符串 `"1.0.0"`。它与 Python v0.2 研究报告的 `schema_version: 2`
无关；`tool_version`、`parser_profile_id`、`metadata_version`、`reference_set_version`
独立记录，未知版本为 `null` 并在 `diagnostics` 中给出原因，不虚构构建号。

## 文件与版本规则

- 所有对象都要求显式列出全部字段；`additionalProperties: false`，契约外字段被拒绝。
- 可空字段用 `null` 表达；未知不是零、不是空数组。
- 内部 32 位 ID 固定 `0x` 加 8 位大写十六进制（`^0x[0-9A-F]{8}$`）。
- 64 位资源/本地化键一律用字符串往返（十进制或 `0x` 十六进制），禁止经 JavaScript Number。
- 有界字节偏移用整数并附十六进制显示（`offset` + `offset_hex`）；无偏移为 `null`。
- JSON 拒绝 `NaN`、`Infinity` 与重复键：schema 无法表达这两条，由解析落点任务
  （SX-004/SX-008/SX-011，验收矩阵 T10）在读取处强制；契约测试以表示规则夹具固定边界。

## Report（`schemas/report.schema.json`）

必填字段：`schema_version`、`tool_version`、`parser_profile_id`、`metadata_version`、
`reference_set_version`、`input_digest`、`context`、`capabilities`、`coverage`、
`observations`、`assessments`、`locations`、`evidence`、`diagnostics`。

| 字段 | 类型 | 语义与未知表达 |
| --- | --- | --- |
| `input_digest` | object | `compressed_sha256`、`expanded_sha256`（可空）、压缩/解压字节数；个人绝对路径不是契约字段 |
| `context` | object | `platform`（可空）、`mode`、`build`（可空，不虚构）、`dlc`（可空数组）、`source`、`limitations` |
| `capabilities` | object | 分列 `container`、`sections`、`inventory`、`counters`、`reference_difference`、`location`，每项 `status` + `reason_codes` |
| `coverage` | object | `supported_category_ids`、`reference_category_ids`、`supported_region_counter_keys`、`unavailable_reason_codes`、`evidence_refs`；43 项稀疏计数不用数组填充变成完整矩阵 |
| `observations` | object | `container`、`sections`、`overall_completion`、`inventory`、`region_counters` |
| `assessments` | array | 逐项 `internal_id` + `observation` + `assessment` + `rule_id` + `evidence_refs` + `limitations` |
| `locations` | array | LocationHint（见下） |
| `evidence` | array | Evidence（见下） |
| `diagnostics` | array | `code`、`severity`、`scope`、`message`、`evidence_refs`；UI 只看 code/severity/scope，不解析自然语言 |

### 值与未知规则

- `ProgressValue`（`observations.overall_completion`）：`value`、`unit`、`support_status`、
  `reason`、`evidence_level`、`evidence_refs`、`limitations`。`value` 为 `null` 时
  `reason` 必填且非空；合法零保留 `0`。
- `RegionCounter.value` 同规则：原始读数超过来源目标时保留 `raw_value`/`raw_value_hex`，
  `value` 与剩余置 `null`，不裁剪到目标值。
- `observations.inventory`：`status`、`inventory_complete`、`completeness_scope`、
  `records`、`record_count`、`unique_id_count`、`duplicate_record_count`。完整性只指当前
  parser profile 下表恢复完整，不代表全游戏全集完备；异常 type28 时必须
  `inventory_complete = false` 且阻断缺席推断（新产品强化规则，见 SX-005/SX-007）。
- 物品记录保留 `raw_companion_u32`、`raw_companion_hex`、`raw_low_two_bits`、槽号与偏移；
  重复 ID 保留全部原始记录，未知非零 ID 不丢弃，伴随字不赋状态语义。

### 三组概念相互独立（不得合并为单一“徽标”）

| 概念 | 字段 | 取值 |
| --- | --- | --- |
| 业务观测 | `observation` | `present`、`reference_present_current_absent`、`unknown` |
| 业务评估 | `assessment` | `likely_collected`、`likely_missing`、`unknown`、`verified_collected`、`verified_uncollected` |
| 证据等级 | `evidence_level` | `observed`、`community_correlated`、`game_validated` |
| 上下文 | `context.mode` / `context.source` | `campaign`/`profile`/`unknown`/`unsupported`；`user_declared`/`fixture_provenance`/`unknown` |
| 定位精度 | `locations[].precision` | `region`、`guide_group`、`guide_item`、`verified_coordinates` |

约束：

- `assessment` 为 `likely_*` 或 `verified_*` 时 `rule_id` 必填；原始 `present` 默认仅表示已观察。
- 首版无新游戏内证据时不产生 `verified_*`；禁止无依据概率分数（没有 confidence 小数）。
- 差集为空不等于全游戏收集完成；结构完整不是全集完整证明。

### LocationHint

- `precision` 与 `mapping_kind` 分列；`member_ids`（组成员关系）与 `guide_item`（精确条目映射）分开。
- `group_actionability`：`whole_group_check` / `general_reference` / `unavailable`，
  携带 `reason_codes`、`evidence_refs` 与 `evaluated_input_digest`（与当前快照绑定）。
  `whole_group_check` 只在完整性、兼容计数、零当前计数、参考范围与组关联全部匹配时给出；
  非零、未知、冲突或变化时必须撤销并重新评估，前端不得从文案/计数自行推导。
- `coordinates`：无已验证映射时为 `null`；`precision` 为 `verified_coordinates` 时必须有
  坐标对象（`x`、`y`、`z`、`coordinate_space`、`unit`、`level_id`）。`map_transform`
  的 `transform_parameters` 序列化形式待 SX-020 定义，首版恒为 `null`。

### Evidence

必含 `evidence_id`（稳定）、`method`、`source_refs`、`sample_scope`、`limitations`；
按类型附 `offset`/`offset_hex`、`record_index`、`reference_set_id`、`cross_checks`、
`source_versions`（来源标识 → 固定提交/文档版本）。

## 信封（batch / desktop-result / export）

- **批量外层** `BatchReport`：`results[]`，每个结果 `{input_ref, report|null, error|null}`；
  `report` 与 `error` 恰有一个非空；`input_ref` 为文件名或脱敏引用，不含个人绝对路径。
- **桌面结果信封** `DesktopResult`：`request_id`、`input_ref`、`snapshot_id`、`snapshot_time`、
  `freshness`、`last_refresh_error`、`report`、`error`。信封与核心 Report 分离：
  - `freshness` ∈ `current` / `refreshing` / `stale` / `none`，只说明本应用最近分析/刷新状态，
    不声称与游戏实时同步；非 `none` 时必须携带快照身份。
  - 取消与晚到结果由 `request_id` 与当前选定请求比对识别，不混入报告业务值；
    旧快照必须标 `stale` 并保留 `last_refresh_error`。
- **导出信封** `ExportDocument`：`document_kind = "report_export"`、快照身份、
  `freshness`（不含 `none`）、`last_refresh_error_code`、`input_digest` 与 `report`。
  过期导出的 JSON 文件本身带 stale 标识；CSV 导出以明确列保留同样的快照身份与刷新错误码
  （列定义在 SX-011 实现并验收）。普通核心 JSON 与导出信封各自有 schema。

## MetadataDataset（`schemas/metadata.schema.json`）

- `items`：键为内部 ID；条目含 `internal_id`、`aliases`（全部保留）、`display_name`（可空）、
  `display_language`、`name_source`、`name_kind`（`source_localized`/`curated_short`/
  `machine_translated`/`unknown`）、`category_id`、`region_id`、`applicability`、
  `location_mappings`、`source_refs`、`review`。
- `display_name` 为 `null` 或 `name_kind = unknown` 时，前端使用带标识的“类别加 ID”回退文案，
  不自造正式名称；人工简名必须标注不是游戏正式名称。
- `categories`、`regions`、`guide_groups`（稳定组 ID、`member_ids`、`target_count`、来源）、
  `reference_sets`（版本、类别范围、来源存档哈希、去重规则）、`sources`（URL 仅允许
  http/https、固定提交、访问日期、许可、再分发状态、方法、审核）。
- 许可状态为 `pending` 的来源数据不得进入发布包（SX-018 复核）；元数据不执行脚本、不自动联网更新。

## Python v0.2 适配（显式、不升级语义）

`derived_candidates` 通过显式适配器进入契约的观测层（实现于 SX-010），示例见
`fixtures/contract/adapted/`。规则：

| Python v0.2 | 契约 1.0.0 | 说明 |
| --- | --- | --- |
| `derived_candidates.overall_completion.candidate_percent` | `observations.overall_completion.value` | `evidence_level = community_correlated`；`warning` 文本进入 `limitations` |
| `region_candidates.entries[]` | `observations.region_counters[]` | `candidate_count` 为 null（超源目标）→ `value = null` 且 reason 必填；`raw_u32_le` 原样保留 |
| `present_item_id_inventory.records[]` | `observations.inventory.records[]` | 表格名/槽号/偏移/伴随字保留；`candidate_names` 属于元数据侧，不进入报告 |
| `reference_presence_differences` | `assessments[]`（`reference_present_current_absent`） | assessment 是否为 `likely_missing` 由规则决定；差集本身不构成评估 |
| `validated` 旧模型层 | 不映射 | 任何 `candidate_*` 都不得因改名升级为 `validated` / `verified_*` |

## 夹具清单

| 夹具 | 覆盖点 |
| --- | --- |
| `valid/report.valid.json` | 完整合法：本地化名、圣胡安 0/8 `whole_group_check`、组多 ID、`likely_missing` |
| `valid/report.degraded.json` | 局部降级：表不完整、超目标计数、`general_reference`→`unavailable`、缺席推断关闭 |
| `valid/report.nonzero-group.json` | 圣胡安非零 3/8 → `general_reference`，撤销整组检查 |
| `valid/metadata.valid.json` | 本地化名 / 仅别名（`unknown`）/ `region_id` 为空 / 同组多 ID / 许可 `pending` |
| `valid/batch.mixed.json` | 合法 + 降级 + 输入错误三结果并存的批量外层 |
| `valid/desktop-result.{current,stale,other-slot}.json` | 当前快照 / 旧快照（`stale` + 刷新错误）/ 跨槽晚返回 |
| `valid/export.stale.json` | 过期导出信封（stale + 最近刷新错误码） |
| `invalid/*` | 未确立枚举值、`null` 缺原因、`verified_coordinates` 缺坐标、非法链接协议 |
| `representation/u64-boundary.json`、`u64-as-number.json` | u64 仅字符串往返；数字形式必须被拒绝 |
| `adapted/*` | Python v0.2 适配输入与输出形状示例 |

## 版本策略

- 变更字段含义、状态枚举或单位 → 升级 `schema_version` 并保留适配说明与迁移段落。
- 新增字段：本契约为“全字段显式 + 禁止额外字段”，因此新增字段同样属于版本变更；
  先评审，再升级。
- 适配旧 Python 输出必须通过显式适配器；原始报告可并存以便追溯差异。
- 未决事项：`map_transform.transform_parameters` 序列化形式（SX-020）、
  重复键与 `NaN`/`Infinity` 的解析级拒绝落点（SX-004/008/011，T10）、
  真实元数据内容与来源许可（SX-008）。

## 已知限制（本任务未宣称）

- 未运行 `pnpm tauri dev` 实机界面；前端仅以类型检查与构建验证可消费性。
- schema 校验与 Rust 反序列化双层校验为互补证据，而非形式化证明；夹具覆盖的用例见上表，
  未出现在夹具中的字段组合仍可能在后续任务中暴露问题（发现即修正并升级契约）。
- `metadata.bad-url.json` 的 URL 协议规则由 JSON Schema 与 SX-008 加载器执行；
  Rust 类型层当前不校验 URL 协议（SX-008 补齐）。
