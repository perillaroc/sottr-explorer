# 术语表

## 观测与评估

| 术语 | 定义 |
| --- | --- |
| `observation` | 业务观测：`present`、`reference_present_current_absent`、`unknown` |
| `assessment` | 业务评估：`likely_collected`、`likely_missing`、`unknown`、`verified_collected`、`verified_uncollected` |
| `evidence_level` | 证据等级：`observed`、`community_correlated`、`game_validated` |
| `rule_id` | 产生 `likely_*` / `verified_*` 的规则标识；这些评估必须携带 |
| `unknown` | 正式结论，不是失败；不得用 `0` 或空清单替代 |
| `inventory` | 当前 parser profile 下恢复出的物品记录集合 |
| `inventory_complete` | 表恢复是否完整；异常时阻断缺席推断 |
| `coverage` | 支持类别、参考类别、支持计数键与不可用原因 |

## 参考与计数

| 术语 | 定义 |
| --- | --- |
| `reference_set` | 逐类别参考 ID 集合，来自标注 100% 的参考存档 |
| `region_counter` | 存档中的区域计数读数；43 项稀疏映射，缺项不是 0 |
| `counts_toward` | 一个计数对应的参考类别合并口径（如贮藏点 77 = 65 + 12） |
| `guide_group` | 攻略站上的组级引用；成员关系与精确条目映射分开 |
| `group_actionability` | `whole_group_check`、`general_reference`、`unavailable` |
| `evaluated_input_digest` | 组级定位判断所绑定的输入快照摘要 |

## 位置与地图

| 术语 | 定义 |
| --- | --- |
| `location.precision` | `region`、`guide_group`、`guide_item`、`verified_coordinates` |
| `mapping_kind` | 位置映射的类型，与 `precision` 分列 |
| `unit` / 子关卡 | 图鉴的关卡单位；65 个命名单位 |
| `zone` / 区域 | 游戏内区域；26 个主区域 + 特殊区域 |
| `basemap` | 每个子关卡一张底图及其变换 |
| `Transform` | `kind=affine`，矩阵 `[a,b,c,d,e,f]`；首版恒为 `[W,0,0,0,H,0]` |
| `calibrated` | 中位误差 ≤ 0.5% 且 P95 ≤ 1% 图片对角线 |
| `approximate` | 中位误差 ≤ 2% 对角线，或条目数不足以统计验证 |
| `unverified` | 三个误差字段均为 `null` |

## 快照与信封

| 术语 | 定义 |
| --- | --- |
| `snapshot` | 一次稳定读取得到的不可变输入状态 |
| `freshness` | `current`、`refreshing`、`stale`、`none`；只说明本应用状态 |
| `stale` | 旧快照；必须同时保留 `last_refresh_error` |
| `DesktopResult` | 桌面端结果信封：`request_id`、快照身份、`freshness`、`report`/`error` |
| `ExportDocument` | 导出信封：`document_kind = "report_export"`，含快照身份与报告 |
| `share` | 分享脱敏：去目录、替换路径样片段；不等于内容可公开 |

## 数据集

| 术语 | 定义 |
| --- | --- |
| `catalog` / 图鉴 | 全收集口径的条目、区域、单位、位置与获取方式 |
| `metadata` | 内部 ID、名称、43 项计数映射、参考集合与攻略组 |
| `basemap` | 子关卡底图 + 归一化坐标到像素的变换 |
| `manifest` | 元数据输入与输出哈希清单，用于漂移检测 |

## 枚举值速查

契约与数据集中会直接出现的取值。完整定义见[契约](/contracts)对应字段。

| 组 | 取值 |
| --- | --- |
| 支持状态 | `supported`、`partial`、`unsupported` |
| 上下文模式 | `campaign`、`profile`、`unknown`、`unsupported` |
| 上下文来源 | `user_declared`、`fixture_provenance`、`unknown` |
| 观测 | `present`、`reference_present_current_absent`、`unknown` |
| 评估 | `likely_collected`、`likely_missing`、`unknown`、`verified_collected`、`verified_uncollected` |
| 证据等级 | `observed`、`community_correlated`、`game_validated` |
| 严重级别 | `info`、`warning`、`error` |
| 诊断作用域 | `input`、`container`、`sections`、`inventory`、`counters`、`reference_difference`、`location`、`metadata` |
| 清单表 | `prefix`、`type28_overflow` |
| 定位精度 | `region`、`guide_group`、`guide_item`、`verified_coordinates` |
| 映射类型 | `region_clue`、`guide_group_membership`、`exact_guide_item`、`verified_coordinates`、`none` |
| 组行动性 | `whole_group_check`、`general_reference`、`unavailable` |
| 快照新鲜度 | `current`、`refreshing`、`stale`、`none` |
| 名称种类 | `source_localized`、`curated_short`、`machine_translated`、`unknown` |
| 审核状态 | `draft`、`reviewed`、`withdrawn` |
| 可再分发 | `yes`、`no`、`pending` |
| 来源介质 | `text`、`video` |
| 来源角色 | `guide`、`tool`、`save`、`research`、`other` |
| 底图根目录 | `external`、`bundled` |
| 变换依据 / 误差来源 | `analytic`、`measured`、`global_estimate` |
| 底图状态 | `unverified`、`approximate`、`calibrated` |
| 坐标 y 轴 | `up`、`down` |
| 底图来源许可 | `pending`、`allowed`、`denied` |
| 图鉴条目种类 | `story`、`nonstory` |
| 获取方式来源 | `curated`、`game_data` |
| 名称来源 | `game_locale`、`curated`、`zone_name` |
