//! 全收集目录（SX-021）：`data/catalog/collectibles_catalog.json` 的严格加载与校验。
//!
//! 边界与不变量（与元数据（SX-008）同一哲学）：
//! - 只接收调用方给的字节；不做文件系统、网络或目录发现。
//! - 反序列化拒绝未知字段（`deny_unknown_fields`）；键格式、总数与类别计数、
//!   必收不变量、共用 ID 记录均有独立校验，失败给出稳定错误码。
//! - 任务目标（missions）语义未闭合：目录只记录，不参与任何缺失结论（校验不为其
//!   赋予必收含义）；挑战古墓等未闭合类别以 `required_for_100 = null` 表示，
//!   任何把这类条目标为必收的数据都会被拒绝。
//!
//! 数据来源与门禁见 `data/catalog/README.md` 与 `research/s020/README.md`。

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

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

/// 单个收集品条目。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogItem {
    /// `story` / `nonstory`。
    pub kind: ItemKind,
    /// 展示名称（MIT 字典优先，可空）。
    pub name: Option<String>,
    /// 故事合集名（story）；非故事条目为空。
    pub collection: Option<String>,
    /// 非故事条目原始 type（story 为空）。
    #[serde(rename = "type")]
    pub type_id: Option<i32>,
    /// 地图标记图标（非故事）。
    pub marker: Option<String>,
    /// 是否属于 DLC。
    pub is_dlc: bool,
    /// 是否“全收集”必收；`null` 表示未闭合（不得用于缺失结论）。
    pub required_for_100: Option<bool>,
    /// 游戏区域哈希（如 `C5B30CDB`）。
    pub region_hash: Option<String>,
    /// 单位/关卡 ID（非故事）。
    pub unit: Option<u32>,
    /// 类别 ID（见 `categories`）。
    pub category: String,
    /// 多个条目共用同一 ID 时置真。
    #[serde(default)]
    pub shared_id: Option<bool>,
    /// 共用 ID 下的全部名称（取决于观测 ID 粒度，仅作展示）。
    #[serde(default)]
    pub shared_names: Option<Vec<String>>,
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
    /// 未闭合等补充说明。
    #[serde(default)]
    pub note: Option<String>,
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

/// 区域组成记录。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRegion {
    /// 游戏区域哈希。
    pub region_hash: String,
    /// 是否 DLC 区域。
    pub is_dlc: bool,
    /// 是否计入完成度。
    pub counts_toward_percent: bool,
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
}

/// 全收集目录（加载后的完整数据集）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// 目录结构版本（本模块按 `"1.0.0"` 校验）。
    pub catalog_schema_version: String,
    /// 目录数据版本。
    pub catalog_version: String,
    /// 生成来源（游戏、提取哈希、日期、许可决策；自由结构）。
    pub generated_from: serde_json::Value,
    /// 总数统计。
    pub totals: CatalogTotals,
    /// 类别清单。
    pub categories: Vec<CatalogCategory>,
    /// 故事合集清单。
    pub collections: Vec<CatalogCollection>,
    /// 区域组成。
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

    /// 校验语义不变量。
    fn validate(&self) -> Result<(), CatalogError> {
        if self.catalog_schema_version != "1.0.0" || self.catalog_version.is_empty() {
            return Err(CatalogError::new(
                CODE_CATALOG_VERSION_INVALID,
                format!(
                    "catalog_schema_version={}（期望 1.0.0），catalog_version={}",
                    self.catalog_schema_version, self.catalog_version
                ),
            ));
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
        let totals = self.totals;
        if totals.items != self.items.len()
            || totals.story != story
            || totals.nonstory != nonstory
            || totals.required_for_100 != required
            || totals.dlc != dlc
        {
            return Err(CatalogError::new(
                CODE_CATALOG_TOTALS_MISMATCH,
                format!(
                    "totals 声明 条目{} 故事{} 非故事{} 必收{} DLC{}；实际 条目{} 故事{} 非故事{} 必收{} DLC{}",
                    totals.items,
                    totals.story,
                    totals.nonstory,
                    totals.required_for_100,
                    totals.dlc,
                    self.items.len(),
                    story,
                    nonstory,
                    required,
                    dlc
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
