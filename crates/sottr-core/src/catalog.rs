//! 全收集图鉴（SX-021 建目录；SX-022 升到 schema 2.0.0）：
//! `data/catalog/collectibles_catalog.json` 的严格加载与校验。
//!
//! 边界与不变量（与元数据（SX-008）同一哲学）：
//! - 只接收调用方给的字节；不做文件系统、网络或目录发现。
//! - 反序列化拒绝未知字段（`deny_unknown_fields`）；键格式、总数与类别计数、
//!   必收不变量、共用 ID 记录、区域 slug 与坐标范围均有独立校验，失败给出稳定错误码。
//! - 任务目标（missions）语义未闭合：目录只记录，不参与任何缺失结论（校验不为其
//!   赋予必收含义）；挑战古墓等未闭合类别以 `required_for_100 = null` 表示，
//!   任何把这类条目标为必收的数据都会被拒绝。
//! - SX-022 新增的区域 / 子关卡 / 中英名 / 坐标 / 获取方式均为**展示与定位线索**，
//!   坐标空间显式标为未校准（`unit_normalized_unverified`），不得当作已验证地图坐标。
//!
//! 数据来源与门禁见 `data/catalog/README.md`、`research/s022/README.md` 与
//! `research/s020/README.md`。

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::metadata::MetadataLimits;

/// 目录 JSON 结构非法（反序列化失败）。
pub const CODE_CATALOG_JSON_INVALID: &str = "CATALOG_JSON_INVALID";
/// 目录超过资源上限。
pub const CODE_CATALOG_LIMIT: &str = "CATALOG_LIMIT";
/// 版本字段为空或非法。
pub const CODE_CATALOG_VERSION_INVALID: &str = "CATALOG_VERSION_INVALID";
/// 条目键不符合 `0x` + 8 位大写十六进制。
pub const CODE_CATALOG_ID_INVALID: &str = "CATALOG_ID_INVALID";
/// totals 与条目实际统计不一致。
pub const CODE_CATALOG_TOTALS_MISMATCH: &str = "CATALOG_TOTALS_MISMATCH";
/// 类别重复或类别的计数/必收计数与条目不一致。
pub const CODE_CATALOG_CATEGORY_MISMATCH: &str = "CATALOG_CATEGORY_MISMATCH";
/// 未闭合类别被标为必收（或条目必收与类别声明冲突）。
pub const CODE_CATALOG_REQUIRED_CONFLICT: &str = "CATALOG_REQUIRED_CONFLICT";
/// 共用 ID 记录缺失 `shared_names`。
pub const CODE_CATALOG_SHARED_ID_INVALID: &str = "CATALOG_SHARED_ID_INVALID";
/// 区域 slug 未在 `regions` 中声明，或区域记录自身非法。
pub const CODE_CATALOG_REGION_INVALID: &str = "CATALOG_REGION_INVALID";
/// 坐标越界或条目引用了不存在的子关卡。
pub const CODE_CATALOG_POSITION_INVALID: &str = "CATALOG_POSITION_INVALID";
/// 条目引用了未在 `acquisition_rules` 中声明的规则。
pub const CODE_CATALOG_RULE_MISSING: &str = "CATALOG_RULE_MISSING";

/// 目录加载/校验错误（`code` 为稳定错误码）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError {
    code: &'static str,
    message: String,
}

impl CatalogError {
    /// 构造错误。
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 稳定错误码。
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// 人类可读说明。
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CatalogError {}

/// 条目种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    /// 故事收集品（Artifacts/Documents 等）。
    Story,
    /// 非故事收集品（世界中的可计数对象）。
    Nonstory,
}

/// 关卡内相对位置（SX-022）。
///
/// `space` 固定为 `unit_normalized_unverified`：坐标来自游戏数据的关卡归一化空间，
/// **未校准**为地图投影，只能作相对位置线索展示（见 `research/s022/README.md`）。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogPosition {
    /// 关卡内归一化 X（0–1）。
    pub x: f64,
    /// 关卡内归一化 Y（0–1）。
    pub y: f64,
    /// 坐标空间标识（本模块按 `unit_normalized_unverified` 校验）。
    pub space: String,
}

/// 单个收集品条目。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogItem {
    /// `story` / `nonstory`。
    pub kind: ItemKind,
    /// 内部候选名（MIT 字典优先；不是游戏正式名）。
    pub name: Option<String>,
    /// 游戏英文名（故事条目来自资源明文）。
    pub name_en: Option<String>,
    /// 官方中文名（与英文名同表下标对齐得到；可空）。
    pub name_zh: Option<String>,
    /// 名称来源：`game_locale` 为游戏本地化，其余为派生/回退。
    pub name_source: Option<String>,
    /// 故事合集名（story）；非故事条目为空。
    pub collection: Option<String>,
    /// 非故事条目原始 type（story 为空）。
    #[serde(rename = "type")]
    pub type_id: Option<i32>,
    /// 地图标记图标。
    pub marker: Option<String>,
    /// 是否属于 DLC。
    pub is_dlc: bool,
    /// 是否“全收集”必收；`null` 表示未闭合（不得用于缺失结论）。
    pub required_for_100: Option<bool>,
    /// 游戏区域哈希（如 `C5B30CDB`）。
    pub region_hash: Option<String>,
    /// 区域 slug（已绑定区域，如 `paititi`；特殊区域为 `_special` 等）。
    pub region_slug: Option<String>,
    /// 类别 ID（见 `categories`）。
    pub category: String,
    /// 故事条目子类别（`relics` / `documents` / `murals`）；非故事条目为空。
    pub subcategory: Option<String>,
    /// 单位/关卡 ID。
    pub unit: Option<u32>,
    /// 关卡内相对位置（未校准）；越界或全零时为 `null`。
    pub position: Option<CatalogPosition>,
    /// 逐项获取注记（人工整理，可空；为空时用类别的通用规则）。
    pub acquisition_note: Option<String>,
    /// 多个条目共用同一 ID 时置真。
    #[serde(default)]
    pub shared_id: Option<bool>,
    /// 共用 ID 下的全部名称（取决于观测 ID 粒度，仅作展示）。
    #[serde(default)]
    pub shared_names: Option<Vec<String>>,
}

/// 类别下的子类别计数（故事条目细分）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSubcategory {
    /// 子类别 ID。
    pub id: String,
    /// 展示名。
    pub name: String,
    /// 条目数。
    pub count: usize,
    /// 子类别获取规则键（覆盖类别的默认规则，如遗物 / 文献 / 壁画各不相同）。
    pub acquisition_rule_key: String,
}

/// 类别清单项。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogCategory {
    /// 稳定类别 ID。
    pub id: String,
    /// 展示名。
    pub name: String,
    /// 条目数。
    pub count: usize,
    /// `true` 必收 / `false` 可选 / `null` 未闭合。
    pub required_for_100: Option<bool>,
    /// 其中必收条目数。
    pub required_count: usize,
    /// 类别默认获取规则键（见 `acquisition_rules`）。
    pub acquisition_rule_key: String,
    /// 未闭合等补充说明。
    #[serde(default)]
    pub note: Option<String>,
    /// 子类别计数（目前仅 `artifacts`）。
    #[serde(default)]
    pub subcategories: Option<Vec<CatalogSubcategory>>,
}

/// 获取方式规则（SX-022）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogAcquisitionRule {
    /// 中文说明。
    pub text_zh: String,
    /// 英文说明。
    pub text_en: String,
    /// 来源：`curated`（人工整理）或 `game_data`。
    pub source: String,
    /// 已知局限。
    #[serde(default)]
    pub limitations: Option<Vec<String>>,
}

/// 单位/子关卡命名（SX-022）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogUnit {
    /// 游戏 zone 名（来自 MIT 列表模板）。
    pub zone: Option<String>,
    /// 英文显示名。
    pub display_en: Option<String>,
    /// 中文显示名。
    pub display_zh: Option<String>,
    /// 名称来源：`game_locale` / `curated` / `zone_name`。
    pub name_source: String,
    /// 所属主区域 slug（按 zone 前缀推断；未知为 `null`）。
    pub region_slug: Option<String>,
}

/// 区域绑定证据（SX-022：区域哈希 → ASL slug 的签名匹配）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRegionBinding {
    /// 与目标签名的 L1 距离（SX-022 实测全部为 0）。
    pub signature_distance: i64,
    /// 次优解距离（用于说明唯一性）。
    pub runner_up_distance: Option<i64>,
    /// 图鉴侧区域×类别计数。
    pub signature: BTreeMap<String, i64>,
    /// 存档侧区域计数器来源目标。
    pub source_targets: BTreeMap<String, i64>,
}

/// 故事合集（数据库集合）清单项。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogCollection {
    /// 合集名。
    pub name: String,
    /// 是否 DLC。
    pub is_dlc: bool,
    /// 条目数。
    pub count: usize,
    /// 是否属于必收集合。
    pub required_for_100: bool,
}

/// 区域记录（游戏区域表 + SX-022 绑定结果）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRegion {
    /// 游戏区域哈希。
    pub region_hash: String,
    /// 区域 slug（已绑定区域；未绑定为 `null`）。
    pub slug: Option<String>,
    /// 区域英文名（游戏本地化）。
    pub name_en: Option<String>,
    /// 区域中文名（游戏本地化）。
    pub name_zh: Option<String>,
    /// 名称来源。
    pub name_source: Option<String>,
    /// 归属口径补充说明（特殊区域）。
    pub note: Option<String>,
    /// 是否 DLC 区域。
    pub is_dlc: bool,
    /// 是否计入完成度。
    pub counts_toward_percent: bool,
    /// 该区域下的条目数。
    pub item_count: usize,
    /// 区域绑定证据；特殊/未绑定区域为 `null`。
    pub binding: Option<CatalogRegionBinding>,
    /// 挑战古墓对象 ID 列表。
    pub challenge_tombs: Vec<u32>,
    /// 营地 ID 列表。
    pub campsites: Vec<u32>,
    /// 挑战目标 ID 列表。
    pub challenges: Vec<u32>,
    /// 任务目标 ID 列表（语义未定，不用于缺失）。
    pub missions: Vec<u32>,
}

/// 目录总数统计。
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogTotals {
    /// 条目总数。
    pub items: usize,
    /// 故事条目数。
    pub story: usize,
    /// 非故事条目数。
    pub nonstory: usize,
    /// 必收条目数。
    pub required_for_100: usize,
    /// DLC 条目数。
    pub dlc: usize,
    /// 带关卡内相对位置的条目数。
    pub with_position: usize,
    /// 已确定区域的条目数。
    pub with_region: usize,
}

/// 全收集图鉴（加载后的完整数据集）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// 目录结构版本（本模块按 `"2.0.0"` 校验）。
    pub catalog_schema_version: String,
    /// 目录数据版本。
    pub catalog_version: String,
    /// 生成来源（游戏、提取哈希、日期、许可决策；自由结构）。
    pub generated_from: serde_json::Value,
    /// 总数统计。
    pub totals: CatalogTotals,
    /// 获取方式规则表。
    pub acquisition_rules: BTreeMap<String, CatalogAcquisitionRule>,
    /// 类别清单。
    pub categories: Vec<CatalogCategory>,
    /// 故事合集清单。
    pub collections: Vec<CatalogCollection>,
    /// 单位/子关卡命名。
    pub units: BTreeMap<String, CatalogUnit>,
    /// 区域记录。
    pub regions: Vec<CatalogRegion>,
    /// 条目表：键 `0x` + 8 位大写十六进制。
    pub items: BTreeMap<String, CatalogItem>,
}

impl Catalog {
    /// 从字节加载并校验目录（唯一入口）。
    pub fn from_json(bytes: &[u8], limits: &MetadataLimits) -> Result<Self, CatalogError> {
        if bytes.len() > limits.max_file_bytes {
            return Err(CatalogError::new(
                CODE_CATALOG_LIMIT,
                format!(
                    "目录 {} 字节，超过上限 {} 字节",
                    bytes.len(),
                    limits.max_file_bytes
                ),
            ));
        }
        let catalog: Catalog = serde_json::from_slice(bytes).map_err(|error| {
            CatalogError::new(
                CODE_CATALOG_JSON_INVALID,
                format!("目录 JSON 非法：{error}"),
            )
        })?;
        catalog.validate()?;
        Ok(catalog)
    }

    /// 数据版本。
    pub fn version(&self) -> &str {
        &self.catalog_version
    }

    /// 已绑定区域的 slug 是否可解析（供计划模块与界面共用）。
    pub fn region_slug_declared(&self, slug: &str) -> bool {
        self.regions
            .iter()
            .any(|region| region.slug.as_deref() == Some(slug))
    }

    /// 校验语义不变量。
    fn validate(&self) -> Result<(), CatalogError> {
        if self.catalog_schema_version != "2.0.0" || self.catalog_version.is_empty() {
            return Err(CatalogError::new(
                CODE_CATALOG_VERSION_INVALID,
                format!(
                    "catalog_schema_version={}（期望 2.0.0），catalog_version={}",
                    self.catalog_schema_version, self.catalog_version
                ),
            ));
        }

        // 区域 slug 唯一，且绑定证据的距离非负
        let mut slugs = std::collections::BTreeSet::new();
        for region in &self.regions {
            if let Some(slug) = region.slug.as_deref()
                && !slugs.insert(slug)
            {
                return Err(CatalogError::new(
                    CODE_CATALOG_REGION_INVALID,
                    format!("区域 slug {slug} 重复声明"),
                ));
            }
            if let Some(binding) = &region.binding
                && (binding.signature_distance < 0
                    || binding.runner_up_distance.is_some_and(|value| value < 0))
            {
                return Err(CatalogError::new(
                    CODE_CATALOG_REGION_INVALID,
                    format!("区域 {} 的绑定距离为负", region.region_hash),
                ));
            }
        }
        let mut declared_item_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for item in self.items.values() {
            if let Some(slug) = item.region_slug.as_deref() {
                if !slugs.contains(slug) {
                    return Err(CatalogError::new(
                        CODE_CATALOG_REGION_INVALID,
                        format!("条目引用了未声明的区域 slug {slug}"),
                    ));
                }
                *declared_item_counts.entry(slug).or_insert(0) += 1;
            }
        }
        for region in &self.regions {
            let Some(slug) = region.slug.as_deref() else {
                if region.item_count != 0 {
                    return Err(CatalogError::new(
                        CODE_CATALOG_REGION_INVALID,
                        format!("未绑定区域 {} 不应有条目计数", region.region_hash),
                    ));
                }
                continue;
            };
            let actual = declared_item_counts.get(slug).copied().unwrap_or(0);
            if actual != region.item_count {
                return Err(CatalogError::new(
                    CODE_CATALOG_REGION_INVALID,
                    format!(
                        "区域 {slug} 声明条目 {}，实际 {}",
                        region.item_count, actual
                    ),
                ));
            }
        }

        for (id, item) in &self.items {
            validate_id(id)?;
            let region_invalid = item.region_hash.as_deref().is_some_and(|region| {
                region.len() != 8
                    || !region
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
            });
            if region_invalid {
                return Err(CatalogError::new(
                    CODE_CATALOG_ID_INVALID,
                    format!("条目 {id} 的 region_hash 不是 8 位大写十六进制"),
                ));
            }
            if item.shared_id == Some(true) && item.shared_names.as_ref().is_none_or(Vec::is_empty)
            {
                return Err(CatalogError::new(
                    CODE_CATALOG_SHARED_ID_INVALID,
                    format!("条目 {id} 标记共用 ID 但缺少 shared_names"),
                ));
            }
            if let Some(unit) = item.unit
                && unit != 0
                && !self.units.contains_key(&unit.to_string())
            {
                return Err(CatalogError::new(
                    CODE_CATALOG_REGION_INVALID,
                    format!("条目 {id} 的 unit {unit} 未在 units 中声明"),
                ));
            }
            if let Some(position) = &item.position {
                let in_range =
                    (0.0..=1.0).contains(&position.x) && (0.0..=1.0).contains(&position.y);
                if !in_range || position.space != "unit_normalized_unverified" {
                    return Err(CatalogError::new(
                        CODE_CATALOG_POSITION_INVALID,
                        format!(
                            "条目 {id} 的位置越界或坐标空间非法（x={} y={} space={}）",
                            position.x, position.y, position.space
                        ),
                    ));
                }
                if item.unit.is_none_or(|unit| unit == 0) {
                    return Err(CatalogError::new(
                        CODE_CATALOG_POSITION_INVALID,
                        format!("条目 {id} 带位置但没有所属子关卡"),
                    ));
                }
            }
            if let Some(subcategory) = item.subcategory.as_deref() {
                if item.kind != ItemKind::Story {
                    return Err(CatalogError::new(
                        CODE_CATALOG_CATEGORY_MISMATCH,
                        format!("条目 {id} 非故事条目不应有子类别"),
                    ));
                }
                let declared = self
                    .category(&item.category)
                    .and_then(|category| category.subcategories.as_ref())
                    .is_some_and(|list| list.iter().any(|entry| entry.id == subcategory));
                if !declared {
                    return Err(CatalogError::new(
                        CODE_CATALOG_CATEGORY_MISMATCH,
                        format!("条目 {id} 的子类别 {subcategory} 未在类别中声明"),
                    ));
                }
                if !matches!(subcategory, "relics" | "documents" | "murals") {
                    return Err(CatalogError::new(
                        CODE_CATALOG_CATEGORY_MISMATCH,
                        format!("条目 {id} 的子类别 {subcategory} 不是已知取值"),
                    ));
                }
            }
        }

        // totals 一致性
        let story = self
            .items
            .values()
            .filter(|i| i.kind == ItemKind::Story)
            .count();
        let nonstory = self
            .items
            .values()
            .filter(|i| i.kind == ItemKind::Nonstory)
            .count();
        let required = self
            .items
            .values()
            .filter(|i| i.required_for_100 == Some(true))
            .count();
        let dlc = self.items.values().filter(|i| i.is_dlc).count();
        let with_position = self
            .items
            .values()
            .filter(|item| item.position.is_some())
            .count();
        let with_region = self
            .items
            .values()
            .filter(|item| item.region_slug.is_some())
            .count();
        let totals = self.totals;
        if totals.items != self.items.len()
            || totals.story != story
            || totals.nonstory != nonstory
            || totals.required_for_100 != required
            || totals.dlc != dlc
            || totals.with_position != with_position
            || totals.with_region != with_region
        {
            return Err(CatalogError::new(
                CODE_CATALOG_TOTALS_MISMATCH,
                format!(
                    "totals 声明 条目{} 故事{} 非故事{} 必收{} DLC{} 带位置{} 有区域{}；\
                     实际 条目{} 故事{} 非故事{} 必收{} DLC{} 带位置{} 有区域{}",
                    totals.items,
                    totals.story,
                    totals.nonstory,
                    totals.required_for_100,
                    totals.dlc,
                    totals.with_position,
                    totals.with_region,
                    self.items.len(),
                    story,
                    nonstory,
                    required,
                    dlc,
                    with_position,
                    with_region
                ),
            ));
        }

        // 类别唯一性、计数与必收不变量
        let mut by_category: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for item in self.items.values() {
            let entry = by_category.entry(item.category.as_str()).or_insert((0, 0));
            entry.0 += 1;
            if item.required_for_100 == Some(true) {
                entry.1 += 1;
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut declared: BTreeMap<&str, Option<bool>> = BTreeMap::new();
        for category in &self.categories {
            if !seen.insert(category.id.as_str()) {
                return Err(CatalogError::new(
                    CODE_CATALOG_CATEGORY_MISMATCH,
                    format!("类别 {} 重复声明", category.id),
                ));
            }
            let (count, required_count) = by_category
                .get(category.id.as_str())
                .copied()
                .unwrap_or((0, 0));
            if count != category.count || required_count != category.required_count {
                return Err(CatalogError::new(
                    CODE_CATALOG_CATEGORY_MISMATCH,
                    format!(
                        "类别 {} 声明 条目{} 必收{}，实际 条目{} 必收{}",
                        category.id, category.count, category.required_count, count, required_count
                    ),
                ));
            }
            if !self
                .acquisition_rules
                .contains_key(&category.acquisition_rule_key)
            {
                return Err(CatalogError::new(
                    CODE_CATALOG_RULE_MISSING,
                    format!(
                        "类别 {} 引用了未声明的获取规则 {}",
                        category.id, category.acquisition_rule_key
                    ),
                ));
            }
            if let Some(subcategories) = &category.subcategories {
                let mut sub_seen = std::collections::BTreeSet::new();
                for entry in subcategories {
                    if !sub_seen.insert(entry.id.as_str()) {
                        return Err(CatalogError::new(
                            CODE_CATALOG_CATEGORY_MISMATCH,
                            format!("类别 {} 的子类别 {} 重复声明", category.id, entry.id),
                        ));
                    }
                    if !self
                        .acquisition_rules
                        .contains_key(&entry.acquisition_rule_key)
                    {
                        return Err(CatalogError::new(
                            CODE_CATALOG_RULE_MISSING,
                            format!(
                                "类别 {} 的子类别 {} 引用了未声明的获取规则 {}",
                                category.id, entry.id, entry.acquisition_rule_key
                            ),
                        ));
                    }
                    let actual = self
                        .items
                        .values()
                        .filter(|item| {
                            item.category == category.id
                                && item.subcategory.as_deref() == Some(entry.id.as_str())
                        })
                        .count();
                    if actual != entry.count {
                        return Err(CatalogError::new(
                            CODE_CATALOG_CATEGORY_MISMATCH,
                            format!(
                                "类别 {} 子类别 {} 声明 {}，实际 {}",
                                category.id, entry.id, entry.count, actual
                            ),
                        ));
                    }
                }
            }
            declared.insert(category.id.as_str(), category.required_for_100);
        }
        for item in self.items.values() {
            let category_required =
                declared
                    .get(item.category.as_str())
                    .copied()
                    .ok_or_else(|| {
                        CatalogError::new(
                            CODE_CATALOG_CATEGORY_MISMATCH,
                            format!("条目类别 {} 未在 categories 中声明", item.category),
                        )
                    })?;
            if item.required_for_100 == Some(true) && category_required != Some(true) {
                return Err(CatalogError::new(
                    CODE_CATALOG_REQUIRED_CONFLICT,
                    format!(
                        "条目类别 {} 的 required_for_100={category_required:?}，但存在必收条目",
                        item.category
                    ),
                ));
            }
        }
        Ok(())
    }

    /// 类别查询。
    pub fn category(&self, id: &str) -> Option<&CatalogCategory> {
        self.categories.iter().find(|category| category.id == id)
    }

    /// 必收条目数（按条目计算，供对账）。
    pub fn required_item_count(&self) -> usize {
        self.items
            .values()
            .filter(|item| item.required_for_100 == Some(true))
            .count()
    }
}

fn validate_id(id: &str) -> Result<(), CatalogError> {
    let valid = id.len() == 10
        && id.starts_with("0x")
        && id[2..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(CatalogError::new(
            CODE_CATALOG_ID_INVALID,
            format!("条目键 {id} 不是 0x + 8 位大写十六进制"),
        ))
    }
}
