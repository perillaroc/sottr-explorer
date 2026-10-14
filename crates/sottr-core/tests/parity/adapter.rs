//! 规范化适配器（SX-010）：Python v0.2 原始输出与 Rust 核心观测的逐层对照。
//!
//! 规则（开发方案 SX-010）：
//! - 规范化只处理路径（报告内不含绝对路径）、键顺序（BTreeMap/确定性迭代）与
//!   展示精度（float32 原值：两边都由同一 f32 提升为 f64，按精确值 + 位模式比较，
//!   不是两位小数）；十六进制沿用各方既定格式（同一数据同格式）；
//! - 绝不忽略状态或完整性差异：任何不同都产生一条 [`Difference`]；
//!   设计内的强化行为必须带 `adjudication` 文本，否则计为未解释差异并使回归失败；
//! - 对照范围按夹具种类分列；范围外字段必须在 [`scope_exclusions`] 中给出原因。
//!
//! 各层对照（战役）：
//! - `container`：有效性 / EOF / 错误码 / 压缩与解压字节数 / 尾随字节（含前缀与哈希）；
//! - `hashes_observations`：压缩与解压 SHA-256、零字节数、前缀十六进制（原始观察）；
//! - `sections`：声明长度、起点、终点、逐区段（偏移 / 类型 / DWORD 数 / 字节数）；
//! - `inventory`：前缀与溢出记录序列（表、槽号、ID、偏移、伴随字、最低两位）、
//!   零槽位、唯一集合与重复计数；
//! - `overall`：原始 4 字节、float32 精确值与位模式、状态映射；
//! - `counters`：43 项稀疏计数（偏移 / 行 / 相对偏移 / 来源目标 / 原始读数 /
//!   候选值 / 是否在来源目标内 / 剩余量），并核对元数据规格与 Python 事实；
//! - `reference_diff`：四个共同类别的参考集合（计数与 ID 集合）、present/absent
//!   集合与参考外同类 ID（与 Python 同口径：字典名称前缀）。
//!
//! profile 只对照容器、哈希与原始观察，并断言双方都不产生战役语义。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use sottr_core::container::{Decompression, decompress};
use sottr_core::domain::progress::{
    OverallReadout, RegionCounterReadout, RegionCounterSpec, overall_completion, region_counters,
};
use sottr_core::hash::sha256_hex;
use sottr_core::inventory::{InventoryRecovery, TYPE28_TOTAL_BYTES, recover};
use sottr_core::limits::Limits;
use sottr_core::metadata::{MetadataBundle, ReferenceSet};
use sottr_core::report::{InventoryObservation, InventoryTable, SupportStatus};
use sottr_core::sections::SectionStream;

use crate::fixtures::{Expectations, FixtureKind};

/// Python 参考类别前缀 → 元数据类别 ID。
///
/// 与 `data/metadata/build_metadata.py` 的 `REFERENCE_CATEGORY` 一致；
/// 适配器同时断言该表与元数据参考集合的类别双向覆盖。
pub const REFERENCE_CATEGORY_MAPPING: [(&str, &str); 4] = [
    ("CollectibleTypeSurvivalCache_", "survival_caches"),
    ("RareResourceCacheCollectible_", "rare_resource_caches"),
    ("CollectibleTypeMonolith_", "monoliths"),
    ("CollectibleTypeCryptTreasure_", "crypt_treasures"),
];

// ---------------------------------------------------------------------------
// 报告结构
// ---------------------------------------------------------------------------

/// 一条差异；`adjudication` 为空表示未解释（回归失败）。
#[derive(Debug, Clone, Serialize)]
pub struct Difference {
    /// 差异位置（层内路径，如 `records[3].raw_following_word`）。
    pub location: String,
    /// Python v0.2 侧的值；内部断言中为 null。
    pub python: Value,
    /// Rust 核心侧的值（内部断言中为观测值）。
    pub rust: Value,
    /// 设计内差异的判定说明；未解释差异为 null。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjudication: Option<String>,
}

/// 一个对照分组的统计。
#[derive(Debug, Default, Clone, Serialize)]
pub struct GroupReport {
    /// 已对照项数。
    pub checked: u64,
    /// 差异列表；空即一致。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub differences: Vec<Difference>,
}

/// 业务期望核对结果。
#[derive(Debug, Clone, Serialize)]
pub struct ExpectationResult {
    /// 期望名（如 `overall_f32`）。
    pub name: String,
    /// 是否通过。
    pub passed: bool,
    /// 期望值。
    pub expected: Value,
    /// 实际值（Rust 侧）。
    pub actual: Value,
}

/// 单夹具对照报告。
#[derive(Debug, Clone, Serialize)]
pub struct FixtureReport {
    /// 文件名。
    pub name: String,
    /// 归档内/集合内相对路径（不含绝对路径）。
    pub relative_path: String,
    /// 夹具种类。
    pub kind: &'static str,
    /// 状态：`compared` 或 `skipped`。
    pub status: &'static str,
    /// 跳过原因（未验证/未提供/不在对照范围）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    /// 分组对照。
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub groups: BTreeMap<String, GroupReport>,
    /// 信息性说明（如强化行为适用）。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// 业务期望核对。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expectations: Vec<ExpectationResult>,
}

impl FixtureReport {
    /// 全部差异数。
    pub fn differences(&self) -> usize {
        self.groups
            .values()
            .map(|group| group.differences.len())
            .sum()
    }

    /// 未解释差异数。
    pub fn unexplained(&self) -> usize {
        self.groups
            .values()
            .flat_map(|group| &group.differences)
            .filter(|difference| difference.adjudication.is_none())
            .count()
    }

    /// 期望失败数。
    pub fn expectation_failures(&self) -> usize {
        self.expectations
            .iter()
            .filter(|entry| !entry.passed)
            .count()
    }

    /// 是否实际对照。
    pub fn compared(&self) -> bool {
        self.status == "compared"
    }
}

/// 跳过的夹具条目。
pub fn skipped_fixture(
    name: &str,
    relative_path: &str,
    kind: FixtureKind,
    reason: impl Into<String>,
) -> FixtureReport {
    FixtureReport {
        name: name.to_string(),
        relative_path: relative_path.to_string(),
        kind: kind.as_str(),
        status: "skipped",
        skip_reason: Some(reason.into()),
        groups: BTreeMap::new(),
        notes: Vec::new(),
        expectations: Vec::new(),
    }
}

/// 对照范围与排除说明。
#[derive(Debug, Clone, Serialize)]
pub struct ScopeDoc {
    /// 战役夹具对照层。
    pub compared_campaign: Vec<&'static str>,
    /// profile 夹具对照层。
    pub compared_profile: Vec<&'static str>,
    /// 范围外字段及原因。
    pub excluded: Vec<ExclusionDoc>,
}

/// 范围外字段。
#[derive(Debug, Clone, Serialize)]
pub struct ExclusionDoc {
    /// Python 字段。
    pub field: &'static str,
    /// 排除原因。
    pub reason: &'static str,
}

/// 对照范围声明。
pub fn scope_exclusions() -> ScopeDoc {
    ScopeDoc {
        compared_campaign: vec![
            "container",
            "hashes_observations",
            "sections",
            "inventory",
            "overall",
            "counters",
            "reference_diff",
            "cross_file.duplicate_groups",
        ],
        compared_profile: vec!["container", "hashes_observations", "profile_closed"],
        excluded: vec![
            ExclusionDoc {
                field: "records[].candidate_names / source / license",
                reason: "候选名称与许可是元数据侧概念，不进入核心观测（SX-003 契约）；名称一致性由元数据构建与契约测试负责",
            },
            ExclusionDoc {
                field: "records[].presence_only / collectible_state / region_inference",
                reason: "Python v0.2 固定为常量（true/unknown/unknown）；Rust 以契约观测与评估表达，不做常量对照",
            },
            ExclusionDoc {
                field: "ascii_strings",
                reason: "候选字符串扫描（有截断上限的展示用途），不在 SX-010 列举的对照层内；核心未实现",
            },
            ExclusionDoc {
                field: "candidate_header_numbers / candidate_header_filetime",
                reason: "无既定语义的候选数值解释，不在对照层内",
            },
            ExclusionDoc {
                field: "catalog_candidate_scan",
                reason: "全文件字节模式扫描（展示用途），不在对照层内",
            },
            ExclusionDoc {
                field: "pairwise_comparisons",
                reason: "产品决定不做默认两两比较（SX-009）；回归以 --max-pairs 0 关闭，差异研究为独立开关",
            },
            ExclusionDoc {
                field: "known_observations 中除重复组外的条目 / limits / scope / unsupported / profiles",
                reason: "v0.2 报告元数据与跨文件聚合，非观测层；产品对应字段由 SX-003 契约与 SX-011 导出承担",
            },
            ExclusionDoc {
                field: "decompression.warning / record.file（绝对值）",
                reason: "展示文本与绝对路径：不进入对照；本报告不含绝对路径",
            },
            ExclusionDoc {
                field: "sections.end_exclusive（无效流时）",
                reason: "长度非法时 Python 仍展示起点+长度、Rust 以 0 表示无效；有效性、原因与部分区段均已对照",
            },
            ExclusionDoc {
                field: "reference_present_user_absent[].candidate_names / collectible_state / region_inference",
                reason: "与记录字段同理：名称属元数据侧，状态由评估层表达",
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// 参照字典
// ---------------------------------------------------------------------------

/// 研究参照字典（仓库副本；仅用于与 Python 同口径的名称前缀判定）。
pub struct Catalog {
    /// ID → 候选名称列表（重复 ID 合并，与 Python `load_catalog` 相同）。
    pub mapping: BTreeMap<u32, Vec<String>>,
    /// 字典文件 SHA-256。
    pub sha256: String,
}

impl Catalog {
    /// 加载字典；键支持 `0x…` 十六进制或十进制（与 Python `int(key, 0)` 相同）。
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|error| format!("读取字典失败：{error}"))?;
        let document: Value =
            serde_json::from_slice(&bytes).map_err(|error| format!("解析字典失败：{error}"))?;
        let Some(object) = document.get("mapping").and_then(Value::as_object) else {
            return Err("字典缺少 mapping 对象".to_string());
        };
        let mut mapping: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        for (key, names) in object {
            let id = parse_u32_key(key).ok_or_else(|| format!("字典 ID 无法解析：{key:?}"))?;
            let Some(names) = names.as_array() else {
                return Err(format!("字典条目 {key:?} 的名称不是数组"));
            };
            let entry = mapping.entry(id).or_default();
            for name in names {
                let Some(name) = name.as_str() else {
                    return Err(format!("字典条目 {key:?} 含非字符串名称"));
                };
                entry.push(name.to_string());
            }
        }
        Ok(Catalog {
            mapping,
            sha256: sha256_hex(&bytes),
        })
    }

    /// 某 ID 的候选名称（可为空）。
    pub fn names(&self, id: u32) -> &[String] {
        self.mapping.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }
}

fn parse_u32_key(key: &str) -> Option<u32> {
    if let Some(hex) = key.strip_prefix("0x").or_else(|| key.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        key.parse::<u32>().ok()
    }
}

// ---------------------------------------------------------------------------
// 对照分组的构造器
// ---------------------------------------------------------------------------

struct Comparer {
    groups: BTreeMap<String, GroupReport>,
    current: String,
    notes: Vec<String>,
}

impl Comparer {
    fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            current: String::new(),
            notes: Vec::new(),
        }
    }

    fn group(&mut self, name: &str) {
        self.current = name.to_string();
        self.groups.entry(name.to_string()).or_default();
    }

    fn note(&mut self, text: impl Into<String>) {
        self.notes.push(text.into());
    }

    /// 对照一项；不同即记录差异。
    fn eq(&mut self, location: impl Into<String>, python: Value, rust: Value) {
        self.push(location.into(), python, rust, None);
    }

    /// 对照一项；差异按设计规则判定（强化行为等）。
    fn eq_adjudicated(
        &mut self,
        location: impl Into<String>,
        python: Value,
        rust: Value,
        adjudication: impl Into<String>,
    ) {
        self.push(location.into(), python, rust, Some(adjudication.into()));
    }

    /// 内部断言（Python 输出形状 / Rust 契约期望）；不成立即未解释差异。
    fn check(&mut self, location: impl Into<String>, passed: bool, observed: Value) {
        let group = self.groups.entry(self.current.clone()).or_default();
        group.checked += 1;
        if !passed {
            group.differences.push(Difference {
                location: location.into(),
                python: Value::Null,
                rust: observed,
                adjudication: None,
            });
        }
    }

    fn push(&mut self, location: String, python: Value, rust: Value, adjudication: Option<String>) {
        let group = self.groups.entry(self.current.clone()).or_default();
        group.checked += 1;
        if python != rust {
            group.differences.push(Difference {
                location,
                python,
                rust,
                adjudication,
            });
        }
    }
}

fn v<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("适配器值必须可序列化")
}

fn id_hex(id: u32) -> String {
    format!("0x{id:08X}")
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_id_hex(value: &Value) -> Option<u32> {
    let text = value.as_str()?;
    u32::from_str_radix(
        text.strip_prefix("0x")
            .or_else(|| text.strip_prefix("0X"))?,
        16,
    )
    .ok()
}

fn sorted_ids(ids: &BTreeSet<u32>) -> Vec<String> {
    ids.iter().copied().map(id_hex).collect()
}

// ---------------------------------------------------------------------------
// 夹具输入与主入口
// ---------------------------------------------------------------------------

/// 单个夹具的对照输入。
pub struct FixtureInput<'a> {
    /// 文件名。
    pub name: &'a str,
    /// 相对路径（报告用）。
    pub relative_path: &'a str,
    /// 种类。
    pub kind: FixtureKind,
    /// 文件字节。
    pub bytes: &'a [u8],
    /// 业务期望。
    pub expect: &'a Expectations,
}

/// 对照一个夹具：Python 记录 vs Rust 核心观测。
pub fn compare_fixture(
    input: &FixtureInput<'_>,
    python_record: &Value,
    bundle: &MetadataBundle,
    catalog: &Catalog,
) -> FixtureReport {
    let limits = Limits::default();
    let decompression = decompress(input.bytes, &limits);
    let data: &[u8] = &decompression.data;
    let sections = SectionStream::walk(data, &limits);
    let inventory = recover(data, &sections);
    let overall = overall_completion(data, &sections);
    let specs = bundle.region_counter_specs();
    let counters = region_counters(data, &sections, &specs);
    let reference = ReferenceParity::compute(bundle, catalog, &inventory.observation);

    let mut comparer = Comparer::new();
    compare_container(&mut comparer, &decompression, input.bytes, python_record);
    compare_hashes(&mut comparer, &decompression, input.bytes, python_record);
    match input.kind {
        FixtureKind::Campaign => {
            compare_sections(&mut comparer, &sections, python_record);
            compare_inventory(&mut comparer, &inventory, data, python_record);
            compare_overall(&mut comparer, &overall, python_record);
            compare_counters(&mut comparer, &counters, &specs, bundle, python_record);
            compare_reference_diff(&mut comparer, &reference, python_record);
            note_strengthened_type28(&mut comparer, &sections);
        }
        FixtureKind::Profile => {
            compare_profile_closed(
                &mut comparer,
                &sections,
                &inventory,
                &overall,
                &counters,
                python_record,
            );
        }
        FixtureKind::ChallengeTomb => {}
    }

    let expectations = check_expectations(
        input,
        &decompression,
        &inventory,
        &overall,
        &reference,
        catalog,
    );

    FixtureReport {
        name: input.name.to_string(),
        relative_path: input.relative_path.to_string(),
        kind: input.kind.as_str(),
        status: "compared",
        skip_reason: None,
        groups: comparer.groups,
        notes: comparer.notes,
        expectations,
    }
}

// ---------------------------------------------------------------------------
// 容器与原始观察
// ---------------------------------------------------------------------------

fn compare_container(
    comparer: &mut Comparer,
    decompression: &Decompression,
    bytes: &[u8],
    python_record: &Value,
) {
    comparer.group("container");
    let state = &python_record["decompression"];
    comparer.eq(
        "decompression.valid",
        state["valid"].clone(),
        v(decompression.valid),
    );
    comparer.eq(
        "decompression.eof",
        state["eof"].clone(),
        v(decompression.eof),
    );
    let rust_error = match &decompression.error {
        Some(error) => v(error.reason_code()),
        None => Value::Null,
    };
    comparer.eq("decompression.error", state["error"].clone(), rust_error);
    comparer.eq(
        "record.compressed_bytes",
        python_record["compressed_bytes"].clone(),
        v(decompression.compressed_bytes),
    );
    if decompression.valid {
        comparer.eq(
            "decompression.trailing_bytes",
            state["trailing_bytes"].clone(),
            v(decompression.trailing_bytes),
        );
        comparer.eq(
            "decompression.compressed_stream_bytes",
            state["compressed_stream_bytes"].clone(),
            v(bytes.len() as u64 - decompression.trailing_bytes),
        );
        comparer.eq(
            "record.uncompressed_bytes",
            python_record["uncompressed_bytes"].clone(),
            v(decompression.expanded_bytes),
        );
        if decompression.trailing_bytes > 0 {
            comparer.eq(
                "decompression.trailing_prefix_hex",
                state["trailing_prefix_hex"].clone(),
                v(&decompression.trailing_prefix_hex),
            );
            let trailing = &bytes[bytes.len() - decompression.trailing_bytes as usize..];
            comparer.eq(
                "decompression.trailing_sha256",
                state["trailing_sha256"].clone(),
                v(sha256_hex(trailing)),
            );
        }
    }

    // Python 输出形状常量（适配器契约；不成立即未解释差异）。
    let limits = Limits::default();
    comparer.check(
        "shape.format",
        state["format"].as_str() == Some("zlib"),
        state["format"].clone(),
    );
    comparer.check(
        "shape.stream_offset",
        state["stream_offset"].as_u64() == Some(0),
        state["stream_offset"].clone(),
    );
    comparer.check(
        "shape.maximum_expanded_bytes",
        state["maximum_expanded_bytes"].as_u64() == Some(limits.max_expanded_bytes),
        state["maximum_expanded_bytes"].clone(),
    );
}

fn compare_hashes(
    comparer: &mut Comparer,
    decompression: &Decompression,
    bytes: &[u8],
    python_record: &Value,
) {
    comparer.group("hashes_observations");
    if !decompression.valid {
        return;
    }
    let data = &decompression.data;
    comparer.eq(
        "record.compressed_sha256",
        python_record["compressed_sha256"].clone(),
        v(sha256_hex(bytes)),
    );
    comparer.eq(
        "record.uncompressed_sha256",
        python_record["uncompressed_sha256"].clone(),
        v(sha256_hex(data)),
    );
    comparer.eq(
        "record.zero_bytes",
        python_record["zero_bytes"].clone(),
        v(data.iter().filter(|byte| **byte == 0).count()),
    );
    let prefix = &data[..data.len().min(64)];
    comparer.eq(
        "record.prefix_hex",
        python_record["prefix_hex"].clone(),
        v(hex_lower(prefix)),
    );
}

// ---------------------------------------------------------------------------
// 区段
// ---------------------------------------------------------------------------

fn compare_sections(comparer: &mut Comparer, sections: &SectionStream, python_record: &Value) {
    comparer.group("sections");
    let stream = &python_record["candidate_packed_stream"];
    comparer.check(
        "candidate_packed_stream.present",
        !stream.is_null(),
        stream.clone(),
    );
    if stream.is_null() {
        return;
    }
    comparer.eq("valid", stream["valid"].clone(), v(sections.valid));
    comparer.eq(
        "length_offset",
        stream["length_offset"].clone(),
        v(sections.length_offset),
    );
    comparer.eq("start", stream["start"].clone(), v(sections.start_offset));
    comparer.eq(
        "byte_length",
        stream["byte_length"].clone(),
        v(sections.byte_length),
    );
    if sections.valid {
        comparer.eq(
            "end_exclusive",
            stream["end_exclusive"].clone(),
            v(sections.end_exclusive),
        );
        comparer.eq(
            "section_count",
            stream["section_count"].clone(),
            v(sections.sections.len()),
        );
    } else {
        let reason = sections.error.map(|error| error.message().to_string());
        comparer.eq("reason", stream["reason"].clone(), v(reason));
    }
    let python_sections = stream["sections"].as_array().cloned().unwrap_or_default();
    comparer.eq(
        "sections.length",
        v(python_sections.len()),
        v(sections.sections.len()),
    );
    for (index, rust_section) in sections.sections.iter().enumerate() {
        let Some(python_section) = python_sections.get(index) else {
            break;
        };
        let prefix = format!("sections[{index}]");
        comparer.eq(
            format!("{prefix}.offset"),
            python_section["offset"].clone(),
            v(rust_section.offset),
        );
        comparer.eq(
            format!("{prefix}.type"),
            python_section["type"].clone(),
            v(rust_section.section_type),
        );
        comparer.eq(
            format!("{prefix}.dwords"),
            python_section["dwords"].clone(),
            v(rust_section.dwords),
        );
        comparer.eq(
            format!("{prefix}.bytes"),
            python_section["bytes"].clone(),
            v(rust_section.bytes),
        );
    }
}

// ---------------------------------------------------------------------------
// 记录与伴随字
// ---------------------------------------------------------------------------

fn compare_inventory(
    comparer: &mut Comparer,
    recovery: &InventoryRecovery,
    data: &[u8],
    python_record: &Value,
) {
    comparer.group("inventory");
    let python_inventory = &python_record["present_item_id_inventory"];
    comparer.check(
        "present_item_id_inventory.present",
        !python_inventory.is_null(),
        python_inventory.clone(),
    );
    if python_inventory.is_null() {
        return;
    }
    let observation = &recovery.observation;
    let python_recovered = python_inventory["status"].as_str() == Some("candidate");
    let rust_recovered = observation.status != SupportStatus::Unsupported;
    comparer.eq("status.recovered", v(python_recovered), v(rust_recovered));
    if !python_recovered || !rust_recovered {
        let python_len = python_inventory["records"].as_array().map_or(0, Vec::len);
        comparer.eq(
            "records.length",
            v(python_len),
            v(observation.records.len()),
        );
        return;
    }

    let prefix_records = observation
        .records
        .iter()
        .filter(|record| record.table == InventoryTable::Prefix)
        .count();
    let overflow_records = observation.records.len() - prefix_records;
    comparer.eq(
        "prefix_nonzero_records",
        python_inventory["prefix_nonzero_records"].clone(),
        v(prefix_records),
    );
    comparer.eq(
        "overflow_nonzero_records",
        python_inventory["overflow_nonzero_records"].clone(),
        v(overflow_records),
    );
    comparer.eq(
        "total_nonzero_records",
        python_inventory["total_nonzero_records"].clone(),
        v(observation.record_count),
    );
    comparer.eq(
        "unique_id_count",
        python_inventory["unique_id_count"].clone(),
        v(observation.unique_id_count),
    );
    comparer.eq(
        "duplicate_id_record_count",
        python_inventory["duplicate_id_record_count"].clone(),
        v(observation.duplicate_record_count),
    );
    // 强化规则对照点：异常 type28 时 Python 无完整性标记（仍为 candidate），
    // Rust 标记表不完整并阻断缺席推断（SX-005）；该差异按规则判定，不复制旧输出。
    comparer.eq_adjudicated(
        "inventory.completeness",
        v(python_inventory["status"].as_str() == Some("candidate")),
        v(observation.inventory_complete),
        "SX-005 强化规则：异常 type28 段使表不完整；Python v0.2 无完整性标记（仅跳过该段）",
    );

    // 零槽位为原始观察；由同一前缀区计算（0x44 + 8 × 槽号）。
    if let Some(prefix) = data.get(0x44..0x44 + 512 * 8) {
        let zero_slots = (0..512)
            .filter(|index| {
                let offset = index * 8;
                u32::from_le_bytes(prefix[offset..offset + 4].try_into().expect("4 bytes")) == 0
            })
            .count();
        comparer.eq(
            "zero_id_slots",
            python_inventory["zero_id_slots"].clone(),
            v(zero_slots),
        );
    }
    comparer.eq("offset", python_inventory["offset"].clone(), v(0x44u64));
    comparer.eq("stride", python_inventory["stride"].clone(), v(8));
    // Python v0.2 报告把 `capacity` 重命名为 `prefix_capacity`（run_analysis 内 pop），
    // 规范化以报告实际键名为准。
    comparer.eq(
        "prefix_capacity",
        python_inventory["prefix_capacity"].clone(),
        v(512),
    );

    let python_records = python_inventory["records"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    comparer.eq(
        "records.length",
        v(python_records.len()),
        v(observation.records.len()),
    );
    for (index, rust_record) in observation.records.iter().enumerate() {
        let Some(python) = python_records.get(index) else {
            break;
        };
        let prefix = format!("records[{index}]");
        let rust_table = match rust_record.table {
            InventoryTable::Prefix => "candidate_prefix",
            InventoryTable::Type28Overflow => "candidate_type28_overflow",
        };
        comparer.eq(
            format!("{prefix}.table"),
            python["table"].clone(),
            v(rust_table),
        );
        comparer.eq(
            format!("{prefix}.index"),
            python["index"].clone(),
            v(rust_record.index),
        );
        comparer.eq(
            format!("{prefix}.id"),
            python["id"].clone(),
            v(id_hex(rust_record.internal_id.raw())),
        );
        comparer.eq(
            format!("{prefix}.id_offset"),
            python["id_offset"].clone(),
            v(rust_record.id_offset),
        );
        comparer.eq(
            format!("{prefix}.following_word_offset"),
            python["following_word_offset"].clone(),
            v(rust_record.id_offset + 4),
        );
        comparer.eq(
            format!("{prefix}.raw_following_word"),
            python["raw_following_word"].clone(),
            v(rust_record.raw_companion_u32),
        );
        comparer.eq(
            format!("{prefix}.raw_following_word_hex"),
            python["raw_following_word_hex"].clone(),
            v(&rust_record.raw_companion_hex),
        );
        comparer.eq(
            format!("{prefix}.raw_low_two_bits"),
            python["raw_low_two_bits"].clone(),
            v(rust_record.raw_low_two_bits),
        );
    }
}

// ---------------------------------------------------------------------------
// 整体候选
// ---------------------------------------------------------------------------

fn compare_overall(comparer: &mut Comparer, overall: &OverallReadout, python_record: &Value) {
    comparer.group("overall");
    let python_overall = &python_record["derived_candidates"]["overall_completion"];
    comparer.check(
        "overall_completion.present",
        !python_overall.is_null(),
        python_overall.clone(),
    );
    if python_overall.is_null() {
        return;
    }
    comparer.eq("offset", python_overall["offset"].clone(), v(0x28u64));
    comparer.eq(
        "encoding",
        python_overall["encoding"].clone(),
        v("float32_le"),
    );
    if !python_overall["raw_hex"].is_null() {
        let rust_raw = match &overall.raw_bytes_hex {
            Some(hex) => v(hex),
            None => Value::Null,
        };
        comparer.eq("raw_hex", python_overall["raw_hex"].clone(), rust_raw);
    }
    // 真实 float32 原值：两边同为 f64::from(f32)，按精确值比较（不是两位小数）。
    let rust_value = match overall.progress.value {
        Some(value) => json!(value),
        None => Value::Null,
    };
    comparer.eq(
        "candidate_percent",
        python_overall["candidate_percent"].clone(),
        rust_value,
    );
    // 独立核对 float32 位模式。
    let python_bits = match python_overall["candidate_percent"].as_f64() {
        Some(value) => json!((value as f32).to_bits()),
        None => Value::Null,
    };
    let rust_bits = match overall.raw_value {
        Some(value) => json!(value.to_bits()),
        None => Value::Null,
    };
    comparer.eq("candidate_percent.f32_bits", python_bits, rust_bits);
    let python_supported = python_overall["status"].as_str() == Some("community_correlated");
    comparer.eq(
        "status.community_correlated",
        v(python_supported),
        v(overall.progress.support_status == SupportStatus::Supported),
    );
}

// ---------------------------------------------------------------------------
// 区域计数
// ---------------------------------------------------------------------------

fn compare_counters(
    comparer: &mut Comparer,
    counters: &RegionCounterReadout,
    specs: &[RegionCounterSpec],
    bundle: &MetadataBundle,
    python_record: &Value,
) {
    comparer.group("counters");
    let python_counters = &python_record["derived_candidates"]["region_counters"];
    comparer.check(
        "region_counters.present",
        !python_counters.is_null(),
        python_counters.clone(),
    );
    if python_counters.is_null() {
        return;
    }
    let python_entries = python_counters["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    comparer.eq(
        "entries.count",
        v(python_entries.len()),
        v(counters.counters.len()),
    );
    if python_entries.is_empty() || counters.counters.is_empty() {
        return;
    }

    let python_by_offset: BTreeMap<u64, &Value> = python_entries
        .iter()
        .filter_map(|entry| {
            entry["serialized_offset"]
                .as_u64()
                .map(|offset| (offset, entry))
        })
        .collect();

    // 元数据规格（转换产物）与 Python 事实（原始数据）逐项对照。
    if let Some(dataset) = bundle.dataset() {
        comparer.eq(
            "specs.count",
            v(python_entries.len()),
            v(dataset.region_counters.len()),
        );
        for (counter_key, entry) in &dataset.region_counters {
            let Some(python) = python_by_offset.get(&entry.serialized_offset) else {
                comparer.eq(
                    format!("specs[{counter_key}].serialized_offset"),
                    Value::Null,
                    v(entry.serialized_offset),
                );
                continue;
            };
            comparer.eq(
                format!("specs[{counter_key}].serialized_row_index"),
                python["serialized_row_index"].clone(),
                v(entry.serialized_row_index),
            );
            comparer.eq(
                format!("specs[{counter_key}].serialized_relative_offset"),
                python["serialized_relative_offset"].clone(),
                v(entry.serialized_relative_offset),
            );
            comparer.eq(
                format!("specs[{counter_key}].source_total"),
                python["source_total"].clone(),
                v(entry.source_total),
            );
        }
    }

    comparer.check(
        "spec_errors.empty",
        counters.spec_errors.is_empty(),
        v(counters.spec_errors.len()),
    );
    for counter in &counters.counters {
        let Some(spec) = specs
            .iter()
            .find(|spec| spec.counter_key == counter.counter_key)
        else {
            continue;
        };
        let Some(python) = python_by_offset.get(&spec.serialized_offset) else {
            comparer.eq(
                format!("entries[{}].serialized_offset", counter.counter_key),
                Value::Null,
                v(spec.serialized_offset),
            );
            continue;
        };
        let prefix = format!("entries[{}]", counter.counter_key);
        comparer.eq(
            format!("{prefix}.raw_u32_le"),
            python["raw_u32_le"].clone(),
            v(counter.raw_value),
        );
        let rust_value = match counter.value {
            Some(value) => json!(value),
            None => Value::Null,
        };
        comparer.eq(
            format!("{prefix}.candidate_count"),
            python["candidate_count"].clone(),
            rust_value,
        );
        comparer.eq(
            format!("{prefix}.within_source_target"),
            python["within_source_target"].clone(),
            v(counter.within_source_target),
        );
        comparer.eq(
            format!("{prefix}.source_total"),
            python["source_total"].clone(),
            v(counter.source_target),
        );
        let rust_remaining = match (counter.within_source_target, counter.source_target) {
            (true, Some(total)) => json!(total - counter.raw_value),
            _ => Value::Null,
        };
        comparer.eq(
            format!("{prefix}.candidate_remaining_to_source_target"),
            python["candidate_remaining_to_source_target"].clone(),
            rust_remaining,
        );
        comparer.eq(
            format!("{prefix}.status.community_correlated"),
            v(python["status"].as_str() == Some("community_correlated")),
            v(counter.value.is_some()),
        );
    }
}

// ---------------------------------------------------------------------------
// 参考差集
// ---------------------------------------------------------------------------

struct CategoryParity {
    category_id: String,
    python_prefix: &'static str,
    reference_ids: BTreeSet<u32>,
    present: BTreeSet<u32>,
    absent: BTreeSet<u32>,
    outside: BTreeSet<u32>,
}

struct ReferenceParity {
    set_count: usize,
    categories: Vec<CategoryParity>,
    metadata_category_ids: Vec<String>,
}

impl ReferenceParity {
    fn compute(
        bundle: &MetadataBundle,
        catalog: &Catalog,
        observation: &InventoryObservation,
    ) -> Self {
        let observed: BTreeSet<u32> = observation
            .records
            .iter()
            .map(|record| record.internal_id.raw())
            .collect();
        let sets: Vec<&ReferenceSet> = bundle
            .dataset()
            .map(|dataset| dataset.reference_sets.values().collect())
            .unwrap_or_default();
        let reference_set = sets.first().copied();
        let categories = REFERENCE_CATEGORY_MAPPING
            .iter()
            .map(|(python_prefix, category_id)| {
                let reference_ids: BTreeSet<u32> = reference_set
                    .and_then(|set| set.reference_ids.get(*category_id))
                    .map(|ids| ids.iter().map(|id| id.raw()).collect())
                    .unwrap_or_default();
                let present = reference_ids.intersection(&observed).copied().collect();
                let absent = reference_ids.difference(&observed).copied().collect();
                let outside = observed
                    .iter()
                    .copied()
                    .filter(|id| {
                        !reference_ids.contains(id)
                            && catalog
                                .names(*id)
                                .iter()
                                .any(|name| name.starts_with(python_prefix))
                    })
                    .collect();
                CategoryParity {
                    category_id: (*category_id).to_string(),
                    python_prefix,
                    reference_ids,
                    present,
                    absent,
                    outside,
                }
            })
            .collect();
        let metadata_category_ids = reference_set
            .map(|set| set.category_ids.clone())
            .unwrap_or_default();
        ReferenceParity {
            set_count: sets.len(),
            categories,
            metadata_category_ids,
        }
    }

    fn category(&self, category_id: &str) -> Option<&CategoryParity> {
        self.categories
            .iter()
            .find(|category| category.category_id == category_id)
    }

    fn absent_total(&self) -> usize {
        self.categories
            .iter()
            .map(|category| category.absent.len())
            .sum()
    }
}

fn compare_reference_diff(
    comparer: &mut Comparer,
    reference: &ReferenceParity,
    python_record: &Value,
) {
    comparer.group("reference_diff");
    let python_diff = &python_record["reference_presence_differences"];
    comparer.check(
        "reference_presence_differences.present",
        !python_diff.is_null(),
        python_diff.clone(),
    );
    if python_diff.is_null() {
        return;
    }
    let python_groups_value = python_diff["groups"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    comparer.check(
        "groups.non_empty",
        !python_groups_value.is_empty(),
        v(python_groups_value.len()),
    );
    if python_groups_value.is_empty() {
        return;
    }
    let python_groups: BTreeMap<String, &Value> = python_groups_value
        .iter()
        .filter_map(|group| {
            group["internal_name_prefix"]
                .as_str()
                .map(|prefix| (prefix.to_string(), group))
        })
        .collect();

    comparer.eq(
        "metadata.reference_sets.count",
        v(1usize),
        v(reference.set_count),
    );
    let mut mapping_categories: Vec<String> = REFERENCE_CATEGORY_MAPPING
        .iter()
        .map(|(_, category_id)| (*category_id).to_string())
        .collect();
    mapping_categories.sort();
    let mut metadata_categories = reference.metadata_category_ids.clone();
    metadata_categories.sort();
    comparer.eq(
        "metadata.categories.coverage",
        v(mapping_categories),
        v(metadata_categories),
    );

    for category in &reference.categories {
        let Some(python) = python_groups.get(category.python_prefix) else {
            comparer.eq(
                format!("groups[{}].present", category.python_prefix),
                Value::Null,
                v(category.reference_ids.len()),
            );
            continue;
        };
        comparer.eq(
            format!("groups[{}].reference_count", category.python_prefix),
            python["reference_count"].clone(),
            v(category.reference_ids.len()),
        );
        let python_present: BTreeSet<u32> = python["present_reference_ids"]
            .as_array()
            .map(|ids| ids.iter().filter_map(parse_id_hex).collect())
            .unwrap_or_default();
        comparer.eq(
            format!("groups[{}].present_reference_ids", category.python_prefix),
            v(sorted_ids(&python_present)),
            v(sorted_ids(&category.present)),
        );
        comparer.eq(
            format!("groups[{}].absent_reference_count", category.python_prefix),
            python["absent_reference_count"].clone(),
            v(category.absent.len()),
        );
        let python_absent: BTreeSet<u32> = python["reference_present_user_absent"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| parse_id_hex(&item["id"]))
                    .collect()
            })
            .unwrap_or_default();
        comparer.eq(
            format!(
                "groups[{}].reference_present_user_absent",
                category.python_prefix
            ),
            v(sorted_ids(&python_absent)),
            v(sorted_ids(&category.absent)),
        );
        let python_outside: BTreeSet<u32> = python["observed_category_ids_outside_reference"]
            .as_array()
            .map(|ids| ids.iter().filter_map(parse_id_hex).collect())
            .unwrap_or_default();
        comparer.eq(
            format!(
                "groups[{}].observed_category_ids_outside_reference",
                category.python_prefix
            ),
            v(sorted_ids(&python_outside)),
            v(sorted_ids(&category.outside)),
        );
    }
}

// ---------------------------------------------------------------------------
// profile 语义关闭
// ---------------------------------------------------------------------------

fn compare_profile_closed(
    comparer: &mut Comparer,
    sections: &SectionStream,
    recovery: &InventoryRecovery,
    overall: &OverallReadout,
    counters: &RegionCounterReadout,
    python_record: &Value,
) {
    comparer.group("profile_closed");
    for key in [
        "candidate_packed_stream",
        "present_item_id_inventory",
        "derived_candidates",
        "reference_presence_differences",
    ] {
        let present = python_record.get(key).is_some_and(|value| !value.is_null());
        comparer.check(format!("python_record.{key}.absent"), !present, v(present));
    }
    comparer.check("sections.valid", !sections.valid, v(sections.valid));
    comparer.check(
        "sections.error.unrecognized_family",
        matches!(
            sections.error,
            Some(sottr_core::sections::SectionError::UnrecognizedFamilySize)
        ),
        v(sections.error.map(|error| error.message().to_string())),
    );
    comparer.check(
        "inventory.status.unsupported",
        recovery.observation.status == SupportStatus::Unsupported,
        v(SupportStatus2Name(recovery.observation.status)),
    );
    comparer.check(
        "inventory.records.empty",
        recovery.observation.records.is_empty(),
        v(recovery.observation.records.len()),
    );
    comparer.check(
        "overall.value.closed",
        overall.progress.value.is_none(),
        v(overall.progress.value),
    );
    comparer.check(
        "counters.empty",
        counters.counters.is_empty(),
        v(counters.counters.len()),
    );
    comparer.check(
        "counters.guard_failed",
        !counters.guard_passed,
        v(counters.guard_passed),
    );
}

/// SupportStatus 的序列化助手（用于诊断值）。
struct SupportStatus2Name(SupportStatus);

impl Serialize for SupportStatus2Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

// ---------------------------------------------------------------------------
// 强化行为说明
// ---------------------------------------------------------------------------

fn note_strengthened_type28(comparer: &mut Comparer, sections: &SectionStream) {
    let abnormal: Vec<String> = sections
        .sections
        .iter()
        .filter(|section| section.section_type == 28 && section.bytes != TYPE28_TOTAL_BYTES)
        .map(|section| format!("0x{:X}", section.offset))
        .collect();
    if !abnormal.is_empty() {
        comparer.note(format!(
            "检测到异常 type28 段（{}）：本项目按强化规则标记表不完整并阻断缺席推断（SX-005），\
             Python v0.2 只跳过该段；记录序列仍按共同读取结果对照。",
            abnormal.join("、")
        ));
    }
}

// ---------------------------------------------------------------------------
// 业务期望
// ---------------------------------------------------------------------------

fn check_expectations(
    input: &FixtureInput<'_>,
    decompression: &Decompression,
    recovery: &InventoryRecovery,
    overall: &OverallReadout,
    reference: &ReferenceParity,
    catalog: &Catalog,
) -> Vec<ExpectationResult> {
    let expect = input.expect;
    let mut results: Vec<ExpectationResult> = Vec::new();
    let mut push = |name: &str, expected: Value, actual: Value| {
        let passed = expected == actual;
        results.push(ExpectationResult {
            name: name.to_string(),
            passed,
            expected,
            actual,
        });
    };

    if let Some(expected) = expect.record_count {
        push(
            "record_count",
            v(expected),
            v(recovery.observation.record_count),
        );
    }
    if let Some(expected) = expect.unique_id_count {
        push(
            "unique_id_count",
            v(expected),
            v(recovery.observation.unique_id_count),
        );
    }
    if let Some(expected) = expect.expanded_bytes {
        push(
            "expanded_bytes",
            v(expected),
            v(decompression.expanded_bytes),
        );
    }
    if let Some(expected) = expect.overall_f32 {
        let actual = match overall.progress.value {
            Some(value) => json!(value),
            None => Value::Null,
        };
        push("overall_f32", json!(expected), actual);
    }
    if let Some(expected) = expect.absent_total {
        push("absent_total", v(expected), v(reference.absent_total()));
    }
    if let Some(expected) = &expect.absent_by_category {
        for (category_id, count) in expected {
            let actual = reference
                .category(category_id)
                .map(|category| category.absent.len())
                .unwrap_or(0);
            push(
                &format!("absent_by_category.{category_id}"),
                v(count),
                v(actual),
            );
        }
    }
    if let Some(expected) = expect.prefix_nonzero {
        let actual = recovery
            .observation
            .records
            .iter()
            .filter(|record| record.table == InventoryTable::Prefix)
            .count();
        push("prefix_nonzero", v(expected), v(actual));
    }
    if let Some(expected) = expect.prefix_known {
        let actual = recovery
            .observation
            .records
            .iter()
            .filter(|record| record.table == InventoryTable::Prefix)
            .filter(|record| !catalog.names(record.internal_id.raw()).is_empty())
            .count();
        push("prefix_known", v(expected), v(actual));
    }
    if let Some(expected) = &expect.raw_sha256 {
        let actual = if decompression.valid {
            v(sha256_hex(&decompression.data))
        } else {
            Value::Null
        };
        push("raw_sha256", v(expected), actual);
    }
    results
}

// ---------------------------------------------------------------------------
// Python 报告形状与运行自检
// ---------------------------------------------------------------------------

/// Python v0.2 报告形状与运行自检（适配器契约；不成立即未解释差异）。
pub fn check_python_run(report: &Value) -> GroupReport {
    let mut comparer = Comparer::new();
    comparer.group("python_run");
    comparer.check(
        "schema_version",
        report["schema_version"].as_u64() == Some(2),
        report["schema_version"].clone(),
    );
    comparer.check(
        "tool_version",
        report["tool_version"].as_str() == Some("0.2.0"),
        report["tool_version"].clone(),
    );
    comparer.check(
        "integrity.all_input_hashes_unchanged",
        report["integrity"]["all_input_hashes_unchanged"].as_bool() == Some(true),
        report["integrity"]["all_input_hashes_unchanged"].clone(),
    );
    comparer.check(
        "pairs_analyzed",
        report["known_observations"]["pairs_analyzed"].as_u64() == Some(0),
        report["known_observations"]["pairs_analyzed"].clone(),
    );
    let limits = Limits::default();
    comparer.check(
        "limits.expanded_bytes_per_file",
        report["limits"]["expanded_bytes_per_file"].as_u64() == Some(limits.max_expanded_bytes),
        report["limits"]["expanded_bytes_per_file"].clone(),
    );
    comparer.check(
        "limits.files",
        report["limits"]["files"].as_u64() == Some(128),
        report["limits"]["files"].clone(),
    );
    comparer.groups.remove("python_run").unwrap_or_default()
}

// ---------------------------------------------------------------------------
// 跨文件重复组
// ---------------------------------------------------------------------------

/// 单个夹具的哈希信息（跨文件重复组对照用）。
pub struct FixtureHashes {
    /// 相对路径（报告用，不含绝对路径）。
    pub relative_path: String,
    /// 源文件路径。
    pub source_path: PathBuf,
    /// 压缩字节 SHA-256。
    pub compressed_sha256: String,
    /// 解压字节 SHA-256（未解压为 None）。
    pub expanded_sha256: Option<String>,
}

/// 跨文件重复组对照结果。
pub struct DuplicatesReport {
    /// 两个重复组分组。
    pub groups: BTreeMap<String, GroupReport>,
    /// 规范化说明（未验证成员的剔除）。
    pub notes: Vec<String>,
}

/// 重复组的哈希选择函数。
type HashOf = fn(&FixtureHashes) -> Option<String>;

/// 对照 Python 的重复组与 Rust 侧按同一夹具集合计算的重复组。
pub fn compare_duplicates(fixtures: &[FixtureHashes], python_report: &Value) -> DuplicatesReport {
    let mut groups = BTreeMap::new();
    let mut notes = Vec::new();
    let hash_of: [(HashOf, &str, &str); 2] = [
        (
            |fixture| Some(fixture.compressed_sha256.clone()),
            "duplicate_compressed_groups",
            "压缩哈希重复组",
        ),
        (
            |fixture| fixture.expanded_sha256.clone(),
            "duplicate_uncompressed_groups",
            "解压哈希重复组",
        ),
    ];
    for (hash_of, group_name, label) in hash_of {
        let mut comparer = Comparer::new();
        comparer.group(group_name);
        let rust_groups = duplicate_groups(fixtures, hash_of);
        let python_groups_raw = python_report["known_observations"][group_name]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let (python_groups, dropped) = normalize_python_groups(&python_groups_raw, fixtures);
        if !dropped.is_empty() {
            notes.push(format!(
                "{label}：Python 组内包含未验证成员（{}），已从对照中剔除",
                dropped.join("、")
            ));
        }
        comparer.eq("groups", v(python_groups), v(rust_groups));
        groups.insert(
            group_name.to_string(),
            comparer.groups.remove(group_name).unwrap_or_default(),
        );
    }
    DuplicatesReport { groups, notes }
}

fn duplicate_groups(
    fixtures: &[FixtureHashes],
    hash_of: fn(&FixtureHashes) -> Option<String>,
) -> Vec<Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for fixture in fixtures {
        if let Some(hash) = hash_of(fixture) {
            grouped
                .entry(hash)
                .or_default()
                .push(fixture.relative_path.to_string());
        }
    }
    let mut groups: Vec<Vec<String>> = grouped
        .into_values()
        .filter(|group| group.len() > 1)
        .collect();
    for group in &mut groups {
        group.sort();
    }
    groups.sort();
    groups
}

/// 规范化 Python 重复组：绝对路径映射回相对路径；未验证成员剔除并返回。
fn normalize_python_groups(
    python_groups: &[Value],
    fixtures: &[FixtureHashes],
) -> (Vec<Vec<String>>, Vec<String>) {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut dropped: Vec<String> = Vec::new();
    for group in python_groups {
        let mut members: Vec<String> = Vec::new();
        for path in group.as_array().into_iter().flatten() {
            let Some(path) = path.as_str() else {
                continue;
            };
            let matched = fixtures
                .iter()
                .find(|fixture| same_file(&fixture.source_path, path));
            if let Some(fixture) = matched {
                members.push(fixture.relative_path.to_string());
            } else {
                dropped.push(path.to_string());
            }
        }
        if members.len() > 1 {
            members.sort();
            groups.push(members);
        }
    }
    dropped.sort();
    groups.sort();
    (groups, dropped)
}

fn same_file(source_path: &Path, python_path: &str) -> bool {
    let candidate = Path::new(python_path);
    match (
        std::fs::canonicalize(source_path),
        std::fs::canonicalize(candidate),
    ) {
        (Ok(source), Ok(candidate)) => source == candidate,
        _ => false,
    }
}
