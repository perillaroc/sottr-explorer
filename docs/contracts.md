# 接口契约 1.0.0（SX-003 冻结）

本文件描述 sottr-explorer 的版本化接口契约。契约由以下内容共同冻结，任何字段含义、
枚举或单位的变更都必须升级 `schema_version`（见文末“版本策略”）：

| 载体 | 位置 |
| --- | --- |
| JSON Schema | `schemas/report.schema.json`、`schemas/batch.schema.json`、`schemas/desktop-result.schema.json`、`schemas/export.schema.json`、`schemas/metadata.schema.json` |
| 契约夹具 | `fixtures/contract/{valid,invalid,representation,adapted}/` |
| Rust 类型 | `crates/sottr-core/src/report/`（Report、信封）、`crates/sottr-core/src/metadata.rs` |
| 前端类型 | `apps/desktop/src/contract/*.d.ts`（由 schema 生成，勿手改） |
| 验证命令 | `cargo test`；`cd apps/desktop && pnpm contract:check`；CLI 输出复核见“分析管线与命令行” |

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
  不自造正式名称；人工简名必须标注不是游戏正式名称。`categories`、`regions`、
  `guide_groups` 同样携带 `name_kind` 以区分来源本地化名称与人工简名。
- `guide_groups`（稳定组 ID、`member_ids`、`target_count`、`reference_set_id`、
  `counter_keys`、来源）：成员关系与精确条目映射分开；`counter_keys` 是整组判断所需的
  计数证据，`group_actionability` 由领域层按当前报告评估（见下），元数据不写死静态 true。
- `reference_sets`：`category_ids` 与 `reference_ids` 逐类别参考 ID（键集合必须一致），
  `applicability`（游戏/平台/模式/构建范围/DLC 与备注）、来源存档哈希（压缩文件小写
  十六进制 SHA-256）、去重规则。
- `region_counters`：43 项稀疏计数映射，键为 `counter:<region>:<category>`；条目含
  `serialized_offset`、行号、相对偏移、`source_total` 与 `counts_toward`（该计数对应的
  参考类别合并口径，如贮藏点计数同时对应普通与稀有贮藏点）。缺项不是 0；偏移必须满足
  `0x10BC + 行号 × 104 + 相对偏移` 恒等式且在 4 MiB 战役家族内（加载器强制）。
- `sources`（URL 仅允许 http/https、固定提交、访问日期、许可、再分发状态、方法、审核）。
- 许可状态非 `yes` 的来源数据不得进入发布包（SX-018 复核）；元数据不执行脚本、不自动联网更新。

### 元数据加载与校验（SX-008）

加载器位于 `crates/sottr-core/src/metadata.rs`，只处理不可变字节、不访问文件系统；
仓库内数据集在 `data/metadata/`（三个文件作为一组加载，说明见该目录 README）。

- 严格 JSON：拒绝重复键、`NaN`/`Infinity`、非 http/https 链接、映射键与条目 ID
  不一致、计数偏移恒等式不符与跨文件同名条目冲突；空输入返回空元数据包。
- 限制（`MetadataLimits` 默认，可按调用方评审调整）：单文件 4 MiB、单字符串 4096 B、
  单数组 8192 项、单对象 8192 条、嵌套 64 层。
- 稳定错误码：`METADATA_TOO_LARGE`、`METADATA_INVALID_JSON`、
  `METADATA_NON_STANDARD_NUMBER`、`METADATA_DUPLICATE_KEY`、`METADATA_LIMIT_EXCEEDED`、
  `METADATA_INVALID_VALUE`（含越界 ID）、`METADATA_INVALID_URL`、`METADATA_KEY_MISMATCH`、
  `METADATA_COUNTER_MAPPING_INVALID`、`METADATA_UNRESOLVED_REFERENCE`、
  `METADATA_VERSION_CONFLICT`、`METADATA_ENTRY_CONFLICT`。
- 合并规则：多个文件必须有相同的 `metadata_version` 与命名空间；相同条目去重，
  冲突条目报错。引用（来源、类别、区域、组、参考集合、计数键）必须可解析。
- 内容哈希：加载结果记录每个文件的小写十六进制 SHA-256；`data/metadata/manifest.json`
  记录同一组哈希，Rust 测试对照防漂移。
- 无兜底：缺少目录/空输入时计数规格为空（能力矩阵按 `COUNTER_MAPPING_UNAVAILABLE`
  降级）、参考视图不存在、条目与攻略组为空；没有任何内置字典或偏移回退。
- 领域视图转换：`region_counter_specs()`（按偏移排序）、`reference_view()`（按当前
  上下文计算适用性：平台/构建/DLC 未知时保留限制文本，明确不匹配时 `applicable = false`）、
  `classify_ids()`（仅按内部 ID 分类）、`counter_scope_totals()`（按 `counts_toward`
  作用域汇总，供交叉检查组装层使用）。

### 组级定位门禁（`domain/location.rs`）

`whole_group_check` 只在快照为 `current`、普通战役上下文、表恢复完整、参考集合适用、
组成员落于参考范围、计数键解析且来源目标之和等于组目标数、组计数为零时给出。
稳定原因码：`COMPLETE_TABLE`、`ZERO_GROUP_COUNT`、`REFERENCE_MATCH`（整组可查）；
`GROUP_COUNT_NONZERO`、`GROUP_COUNT_UNKNOWN`、`GROUP_COUNT_CONFLICT`、
`REFERENCE_NOT_APPLICABLE`、`GROUP_ASSOCIATION_UNCONFIRMED`（降为 `general_reference`）；
`INVENTORY_INCOMPLETE`、`INVENTORY_UNAVAILABLE`、`CONTEXT_NOT_CAMPAIGN`、
`SNAPSHOT_NOT_CURRENT`（`unavailable`，关闭行动摘要）。输出绑定
`evaluated_input_digest`，每次刷新/切槽必须重新评估。

## 分析管线与命令行（SX-009）

`crates/sottr-core/src/analyze.rs` 提供同核心分析管线：

- `analyze_bytes(bytes, declared, metadata, limits) -> Result<Report, InputError>`：
  只接收不可变字节；容器无效或超出资源限制返回输入级错误
  （`DECOMPRESSION_FAILED` / `RESOURCE_LIMIT`，`retryable = false`），不产生报告、
  不做语义解释；局部失败留在报告能力矩阵与 `diagnostics`。
- 报告装配顺序：容器 → 区段 → 记录 → 上下文 → 整体候选/43 项计数 → 参考视图与
  分类 → 交叉检查 → 参考评估 → 定位（组门禁按当前快照评估）→ 证据（计数与参考集合
  来源接线）→ 覆盖范围 → 诊断。上下文不适用时整体候选与区域计数按 `unsupported` +
  原因关闭，原始身份观测保留。
- 参考集合选择：当前规则为“恰好一个”（`MetadataBundle::primary_reference_set_id`）；
  零个或多个时不猜选择规则，只保留原始身份观测并给出 `METADATA_UNAVAILABLE` /
  `METADATA_CONFLICT` 诊断。
- 计数交叉检查：`counts_toward` 为单类别的作用域按 `category_counter_totals`
  逐类别比较；多类别合并作用域（如贮藏点计数同时覆盖普通与稀有类别）按
  `counter_scope_checks` 比较观察并集；任一处不一致即 `COUNTER_MISMATCH` 降级，
  不给 `likely_*`。贮藏点合并口径的对照关系为 77 = 65 + 12。
- 分析器诊断码：上下文发现（`MODE_*`/`BUILD_UNKNOWN_*`/`DLC_UNKNOWN_*`）、
  `CONTAINER_TRAILING_BYTES`、`COUNTER_MAPPING_INVALID`、`METADATA_UNAVAILABLE`，
  以及既有 `UNSUPPORTED_LAYOUT`/`UNSUPPORTED_CONTEXT`/`COUNTER_MISMATCH`/
  `METADATA_CONFLICT`/`LOCATION_UNVERIFIED`；同一（code, scope, message）去重。

CLI（`crates/sottr-cli`，最终语法）：

```text
sottr-cli analyze --input <文件或目录> --context <campaign|unknown> \
    --output <新目录> --metadata <元数据目录>
```

- 目录输入收集 `*.dat`（≤128 个，按名称排序）；单个文件输入等价于单文件批量。
- `profile.dat` 始终按资料文件处理（仅容器观察），批量 `--context` 不覆盖该排除；
  `--context campaign` 是用户声明，不是格式识别证明。
- 输出写入必须尚不存在的目录中的 `analysis.json`（`schemas/batch.schema.json`）。
  单文件输入级错误（`INPUT_UNREADABLE`、`DECOMPRESSION_FAILED`、`RESOURCE_LIMIT`；
  适配层对明显超限的文件先做大小预检，避免整份读入）写入对应结果条目，
  不影响其他文件的输出。
- 退出码：全部成功 `0`；至少一个输入级错误 `1`（批量 JSON 仍写出全部结果）；
  用法或配置错误 `2`。纯分析不做全文件两两比较。
- 输出 JSON 的独立复核：`node apps/desktop/scripts/contract/validate-document.mjs
  --schema batch --file <路径>`（或 `pnpm contract:document -- --schema batch --file <路径>`）。

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
| `valid/metadata.valid.json` | 本地化名 / 仅别名（`unknown`）/ `region_id` 为空 / 同组多 ID / 许可 `pending` / 稀疏计数与参考 ID |
| `valid/batch.mixed.json` | 合法 + 降级 + 输入错误三结果并存的批量外层 |
| `valid/desktop-result.{current,stale,other-slot}.json` | 当前快照 / 旧快照（`stale` + 刷新错误）/ 跨槽晚返回 |
| `valid/export.stale.json` | 过期导出信封（stale + 最近刷新错误码） |
| `invalid/*` | 未确立枚举值、`null` 缺原因、`verified_coordinates` 缺坐标、非法链接协议 |
| `representation/u64-boundary.json`、`u64-as-number.json` | u64 仅字符串往返；数字形式必须被拒绝 |
| `adapted/*` | Python v0.2 适配输入与输出形状示例 |
| `data/metadata/*.json`（真实数据） | 由 `pnpm contract:validate` 与 `schemas/metadata.schema.json` 校验；Rust 加载测试另行核对数量、哈希与交叉一致性 |

## 版本策略

- 变更字段含义、状态枚举或单位 → 升级 `schema_version` 并保留适配说明与迁移段落。
- 新增字段：本契约为“全字段显式 + 禁止额外字段”，因此新增字段同样属于版本变更；
  先评审，再升级。SX-008 在契约发布前补全了 `metadata.schema.json` 的
  `region_counters`、名称种类与参考/组字段（未发布版本内的补齐，`schema_version`
  保持 `1.0.0`）；此后对元数据 schema 的字段增删同样受本策略约束。
- 适配旧 Python 输出必须通过显式适配器；原始报告可并存以便追溯差异。
- 未决事项：`map_transform.transform_parameters` 序列化形式（SX-020）；
  报告与导出 JSON 的重复键与 `NaN`/`Infinity` 解析级拒绝落点（SX-004/011，T10；
  元数据侧已由 SX-008 加载器执行）。
- 元数据内容与来源许可已由 SX-008 落地（`data/metadata/`）；`redistributable`
  非 `yes` 的来源在 SX-018 发布复核前不得直接打包。

## 已知限制（本任务未宣称）

- 未运行 `pnpm tauri dev` 实机界面；前端仅以类型检查与构建验证可消费性。
- schema 校验与 Rust 反序列化双层校验为互补证据，而非形式化证明；夹具覆盖的用例见上表，
  未出现在夹具中的字段组合仍可能在后续任务中暴露问题（发现即修正并升级契约）。
- `metadata.bad-url.json` 的 URL 协议规则由 JSON Schema 与 SX-008 加载器双层执行
  （加载器另校验主机名、空白与长度）。
- SX-008 加载器按“一组文件”校验引用完整性；单独加载三个数据文件之一会因跨文件引用
  未解析而失败（见 `data/metadata/README.md`）。
- 元数据计数作用域合计（`counter_scope_totals`）的交叉检查接线已由 SX-009 的组装逻辑
  完成（见“分析管线与命令行”一节）；作用域比较依赖元数据 `counts_toward` 定义，
  新增或变更合并口径必须重新评审。
- CLI 的 Windows 路径安全（ACL、符号链接/重解析点、读取竞态与稳定快照）仍按
  SX-012/SX-013 验收；当前实现为单次读取 + 超限预检，不产生 `INPUT_CHANGED`，
  不声称平台保护已完成。资源上限为默认值（压缩/解压 64 MiB、128 个批量文件），
  可按 SX-012/SX-013 评审调整。
