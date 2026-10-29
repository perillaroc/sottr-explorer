//! 真实地图基底层（P2-01/P2-02）：`data/map/basemaps.json` 的严格加载与校验。
//!
//! 边界与不变量（与元数据 SX-008、图鉴 SX-021 同一哲学）：
//! - 只接收调用方给的字节；不做文件系统、网络或目录发现，**不读图片**。
//! - 反序列化拒绝未知字段（`deny_unknown_fields`）；键一致性、单位唯一性、
//!   变换矩阵与图片尺寸的对应、来源引用可解析、状态与误差字段的搭配、
//!   以及 D5 精度门槛都有独立校验，失败给出稳定错误码。
//!
//! 坐标语义：图鉴 `position` 是**每单位独立**的归一化空间，原点在取景框西北角、
//! y 向下增大；首版变换恒为比例映射 `u = x·W`、`v = y·H`。`transform_basis = analytic`
//! 表示该映射由游戏数据（gameflow 取景框 + 营地归一化坐标配对）解析得出，
//! 因此 `median_error_px` 等是**验证结果**，不是拟合残差。
//!
//! 图片本身不进本模块：`image` 是相对 `image_root` 的路径，存在性与 SHA-256
//! 由构建脚本/测试负责（与既有加载器一致的边界）。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;

use crate::metadata::MetadataLimits;

/// 基底层 JSON 结构非法（反序列化失败）。
pub const CODE_BASEMAP_JSON_INVALID: &str = "BASEMAP_JSON_INVALID";
/// 基底层超过资源上限。
pub const CODE_BASEMAP_LIMIT: &str = "BASEMAP_LIMIT";
/// `schema_version` / `basemap_version` 非法。
pub const CODE_BASEMAP_VERSION_INVALID: &str = "BASEMAP_VERSION_INVALID";
/// 键与 `basemap_id` 不一致，或 id / 区域 slug 格式非法。
pub const CODE_BASEMAP_ID_INVALID: &str = "BASEMAP_ID_INVALID";
/// 同一个 unit 出现在多条记录里。
pub const CODE_BASEMAP_UNIT_DUPLICATE: &str = "BASEMAP_UNIT_DUPLICATE";
/// 变换矩阵与图片尺寸不匹配，或字段组合非法。
pub const CODE_BASEMAP_TRANSFORM_INVALID: &str = "BASEMAP_TRANSFORM_INVALID";
/// `source_refs` 为空或无法解析到顶层 `sources`。
pub const CODE_BASEMAP_SOURCE_MISSING: &str = "BASEMAP_SOURCE_MISSING";
/// 状态与误差字段不配套（如 `unverified` 却带误差）。
pub const CODE_BASEMAP_STATUS_INVALID: &str = "BASEMAP_STATUS_INVALID";
/// `image` 路径、尺寸或哈希字段非法。
pub const CODE_BASEMAP_IMAGE_INVALID: &str = "BASEMAP_IMAGE_INVALID";
/// 声明的 `status` 达不到对应精度门槛（D5）。
pub const CODE_BASEMAP_PRECISION_INSUFFICIENT: &str = "BASEMAP_PRECISION_INSUFFICIENT";

/// 基底层加载/校验错误（`code` 为稳定错误码）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasemapError {
    code: &'static str,
    message: String,
}

impl BasemapError {
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

impl fmt::Display for BasemapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BasemapError {}

/// 图片根目录的归属方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageRootKind {
    /// 图片在仓库外，由用户指向本地目录。
    External,
    /// 图片随应用分发。
    Bundled,
}

/// 分发授权状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Redistributable {
    /// 未决。
    Pending,
    /// 允许分发。
    Allowed,
    /// 不允许分发。
    Denied,
}

/// 变换依据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformBasis {
    /// 由游戏数据解析出的确定性映射。
    Analytic,
    /// 由控制点拟合。
    Measured,
}

/// 校准状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BasemapStatus {
    /// 未验证。
    Unverified,
    /// 近似（中位 ≤ 2% 对角线）。
    Approximate,
    /// 已校准（中位 ≤ 0.5% 且 P95 ≤ 1% 对角线）。
    Calibrated,
}

/// 误差来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSource {
    /// 逐单位抽样验证。
    Measured,
    /// 使用其它单位的汇总估计。
    GlobalEstimate,
}

/// 图鉴坐标的 y 方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoordinateYAxis {
    /// y 向上。
    Up,
    /// y 向下（本数据集的实际约定）。
    Down,
}

/// 复核状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewState {
    /// 草案。
    Draft,
    /// 已复核。
    Reviewed,
}

/// 图片根目录。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRoot {
    /// 归属方式。
    pub kind: ImageRootKind,
    /// `image` 的相对根目录。
    pub path: String,
    /// 人类可读说明。
    #[serde(default)]
    pub note: Option<String>,
}

/// 图片来源。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasemapSource {
    /// 原始来源页。
    #[serde(default)]
    pub url: Option<String>,
    /// 固定版本。
    #[serde(default)]
    pub fixed_revision: Option<String>,
    /// 访问日期（`YYYY-MM-DD`）。
    #[serde(default)]
    pub accessed: Option<String>,
    /// 许可说明。
    pub license: String,
    /// 分发授权状态。
    pub redistributable: Redistributable,
    /// 获取方式。
    pub method: String,
    /// 复核说明。
    #[serde(default)]
    pub review: Option<String>,
}

/// 仿射变换参数。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    /// 恒为 `affine`（首版不支持单应）。
    pub kind: TransformKind,
    /// `u = a*x + b*y + c`、`v = d*x + e*y + f`，顺序 `[a, b, c, d, e, f]`。
    pub matrix: Vec<f64>,
    /// 图鉴坐标的 y 方向。
    pub coordinate_y_axis: CoordinateYAxis,
    /// 图片来源（恒为左上角）。
    pub image_origin: ImageOrigin,
}

/// 变换种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransformKind {
    /// 仿射。
    Affine,
}

/// 图片原点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageOrigin {
    /// 左上角。
    TopLeft,
}

/// 复核记录。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// 状态。
    pub state: ReviewState,
    /// 说明。
    #[serde(default)]
    pub note: Option<String>,
    /// 日期（`YYYY-MM-DD`）。
    #[serde(default)]
    pub date: Option<String>,
}

/// 单条基底层记录。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Basemap {
    /// 与 `basemaps` 的键一致。
    pub basemap_id: String,
    /// 记录版本。
    pub version: String,
    /// 区域 slug（无区域时 `unnamed`）。
    pub region_id: String,
    /// 子关卡 zone id。
    pub unit_ids: Vec<u32>,
    /// 游戏内地图路径。
    #[serde(default)]
    pub map_path: Option<String>,
    /// 对应关卡 DRM 名。
    #[serde(default)]
    pub level_drm: Option<String>,
    /// 来源贴图编号。
    #[serde(default)]
    pub source_sheet: Option<String>,
    /// 相对 `image_root.path` 的图片路径。
    pub image: String,
    /// 图片宽度（像素）。
    pub image_width: u32,
    /// 图片高度（像素）。
    pub image_height: u32,
    /// 变换参数。
    pub transform: Transform,
    /// 变换依据。
    pub transform_basis: TransformBasis,
    /// 校准状态。
    pub status: BasemapStatus,
    /// 中位误差（像素）。
    #[serde(default)]
    pub median_error_px: Option<f64>,
    /// P95 误差（像素）。
    #[serde(default)]
    pub p95_error_px: Option<f64>,
    /// 最大误差（像素）。
    #[serde(default)]
    pub max_error_px: Option<f64>,
    /// 误差来源。
    #[serde(default)]
    pub error_source: Option<ErrorSource>,
    /// 验证样本数。
    #[serde(default)]
    pub validation_samples: Option<u32>,
    /// 引用的来源键。
    pub source_refs: Vec<String>,
    /// 图片 SHA-256（小写十六进制）。
    #[serde(default)]
    pub image_sha256: Option<String>,
    /// 图片字节数。
    #[serde(default)]
    pub image_bytes: Option<u64>,
    /// 复核记录。
    pub review: Review,
}

impl Basemap {
    /// 图片对角线长度（像素）。
    pub fn diagonal_px(&self) -> f64 {
        let w = f64::from(self.image_width);
        let h = f64::from(self.image_height);
        (w * w + h * h).sqrt()
    }

    /// 把归一化坐标映射到图片像素。
    ///
    /// 返回 `None` 表示变换无法求值（理论上校验后不会发生）。
    pub fn to_pixel(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let m = &self.transform.matrix;
        if m.len() != 6 {
            return None;
        }
        let u = m[0] * x + m[1] * y + m[2];
        let v = m[3] * x + m[4] * y + m[5];
        Some((u, v))
    }
}

/// 基底层数据集（加载后的完整内容）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasemapDataset {
    /// 结构版本（本模块按 `"1.0.0"` 校验）。
    pub schema_version: String,
    /// 数据版本。
    pub basemap_version: String,
    /// 生成说明。
    #[serde(default)]
    pub generated: Option<String>,
    /// 图片根目录。
    pub image_root: ImageRoot,
    /// 面向维护者的说明。
    #[serde(default)]
    pub notes: Option<Vec<String>>,
    /// 来源表。
    pub sources: BTreeMap<String, BasemapSource>,
    /// 基底层记录表。
    pub basemaps: BTreeMap<String, Basemap>,
}

impl BasemapDataset {
    /// 从字节加载并校验（唯一入口）。
    pub fn from_json(bytes: &[u8], limits: &MetadataLimits) -> Result<Self, BasemapError> {
        if bytes.len() > limits.max_file_bytes {
            return Err(BasemapError::new(
                CODE_BASEMAP_LIMIT,
                format!(
                    "基底层 {} 字节，超过上限 {} 字节",
                    bytes.len(),
                    limits.max_file_bytes
                ),
            ));
        }
        let dataset: BasemapDataset = serde_json::from_slice(bytes).map_err(|error| {
            BasemapError::new(
                CODE_BASEMAP_JSON_INVALID,
                format!("基底层 JSON 非法：{error}"),
            )
        })?;
        dataset.validate(limits)?;
        Ok(dataset)
    }

    /// 数据版本。
    pub fn version(&self) -> &str {
        &self.basemap_version
    }

    /// 按子关卡查找基底层记录。
    pub fn for_unit(&self, unit: u32) -> Option<&Basemap> {
        self.basemaps
            .values()
            .find(|basemap| basemap.unit_ids.contains(&unit))
    }

    /// 是否所有图片都必须随应用分发。
    pub fn images_bundled(&self) -> bool {
        self.image_root.kind == ImageRootKind::Bundled
    }

    /// 校验语义不变量。
    fn validate(&self, limits: &MetadataLimits) -> Result<(), BasemapError> {
        if self.schema_version != "1.0.0" || self.basemap_version.is_empty() {
            return Err(BasemapError::new(
                CODE_BASEMAP_VERSION_INVALID,
                format!(
                    "schema_version={:?}、basemap_version={:?}",
                    self.schema_version, self.basemap_version
                ),
            ));
        }
        if self.image_root.path.is_empty() {
            return Err(BasemapError::new(
                CODE_BASEMAP_IMAGE_INVALID,
                "image_root.path 为空",
            ));
        }
        if self.basemaps.len() > limits.max_object_entries {
            return Err(BasemapError::new(
                CODE_BASEMAP_LIMIT,
                format!("basemaps {} 条，超过上限", self.basemaps.len()),
            ));
        }

        let mut units: BTreeSet<u32> = BTreeSet::new();
        for (key, basemap) in &self.basemaps {
            if key != &basemap.basemap_id {
                return Err(BasemapError::new(
                    CODE_BASEMAP_ID_INVALID,
                    format!("键 {key:?} 与 basemap_id {:?} 不一致", basemap.basemap_id),
                ));
            }
            if basemap.region_id.is_empty() || !is_slug(&basemap.region_id) {
                return Err(BasemapError::new(
                    CODE_BASEMAP_ID_INVALID,
                    format!("{key}: region_id {:?} 非法", basemap.region_id),
                ));
            }
            if basemap.unit_ids.is_empty() {
                return Err(BasemapError::new(
                    CODE_BASEMAP_ID_INVALID,
                    format!("{key}: unit_ids 为空"),
                ));
            }
            for unit in &basemap.unit_ids {
                if !units.insert(*unit) {
                    return Err(BasemapError::new(
                        CODE_BASEMAP_UNIT_DUPLICATE,
                        format!("{key}: unit {unit} 已出现在其它记录中"),
                    ));
                }
            }
            if basemap.version.is_empty() {
                return Err(BasemapError::new(
                    CODE_BASEMAP_VERSION_INVALID,
                    format!("{key}: version 为空"),
                ));
            }
            self.validate_image(key, basemap, limits)?;
            self.validate_transform(key, basemap)?;
            self.validate_status(key, basemap)?;
            self.validate_sources(key, basemap)?;
        }
        Ok(())
    }

    fn validate_image(
        &self,
        key: &str,
        basemap: &Basemap,
        limits: &MetadataLimits,
    ) -> Result<(), BasemapError> {
        if basemap.image.is_empty() || basemap.image.len() > limits.max_string_bytes {
            return Err(BasemapError::new(
                CODE_BASEMAP_IMAGE_INVALID,
                format!("{key}: image 为空或过长"),
            ));
        }
        if basemap.image.starts_with('/') || basemap.image.contains("..") {
            return Err(BasemapError::new(
                CODE_BASEMAP_IMAGE_INVALID,
                format!("{key}: image {:?} 必须是相对路径", basemap.image),
            ));
        }
        if basemap.image_width == 0 || basemap.image_height == 0 {
            return Err(BasemapError::new(
                CODE_BASEMAP_IMAGE_INVALID,
                format!("{key}: image_width/height 必须为正"),
            ));
        }
        if let Some(sha) = &basemap.image_sha256 {
            let ok = sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
            if !ok {
                return Err(BasemapError::new(
                    CODE_BASEMAP_IMAGE_INVALID,
                    format!("{key}: image_sha256 必须是 64 位小写十六进制"),
                ));
            }
        } else if self.image_root.kind == ImageRootKind::Bundled {
            return Err(BasemapError::new(
                CODE_BASEMAP_IMAGE_INVALID,
                format!("{key}: 图片随应用分发时必须提供 image_sha256"),
            ));
        }
        if basemap.image_bytes == Some(0) {
            return Err(BasemapError::new(
                CODE_BASEMAP_IMAGE_INVALID,
                format!("{key}: image_bytes 必须为正"),
            ));
        }
        Ok(())
    }

    fn validate_transform(&self, key: &str, basemap: &Basemap) -> Result<(), BasemapError> {
        let m = &basemap.transform.matrix;
        if m.len() != 6 || m.iter().any(|v| !v.is_finite()) {
            return Err(BasemapError::new(
                CODE_BASEMAP_TRANSFORM_INVALID,
                format!("{key}: matrix 必须是 6 个有限数"),
            ));
        }
        let expected = [
            f64::from(basemap.image_width),
            0.0,
            0.0,
            0.0,
            f64::from(basemap.image_height),
            0.0,
        ];
        if m.as_slice() != expected {
            return Err(BasemapError::new(
                CODE_BASEMAP_TRANSFORM_INVALID,
                format!(
                    "{key}: matrix {m:?} 与图片尺寸不一致，应为 {expected:?}"
                ),
            ));
        }
        if basemap.transform.kind != TransformKind::Affine {
            return Err(BasemapError::new(
                CODE_BASEMAP_TRANSFORM_INVALID,
                format!("{key}: 首版仅支持 affine"),
            ));
        }
        if basemap.transform_basis == TransformBasis::Analytic
            && basemap.transform.coordinate_y_axis != CoordinateYAxis::Down
        {
            return Err(BasemapError::new(
                CODE_BASEMAP_TRANSFORM_INVALID,
                format!("{key}: transform_basis=analytic 时 y 轴必须为 down"),
            ));
        }
        Ok(())
    }

    fn validate_status(&self, key: &str, basemap: &Basemap) -> Result<(), BasemapError> {
        let errors = [
            basemap.median_error_px,
            basemap.p95_error_px,
            basemap.max_error_px,
        ];
        if basemap.status == BasemapStatus::Unverified {
            if errors.iter().any(Option::is_some) {
                return Err(BasemapError::new(
                    CODE_BASEMAP_STATUS_INVALID,
                    format!("{key}: unverified 不得带误差字段"),
                ));
            }
            return Ok(());
        }
        if errors.iter().any(Option::is_none) {
            return Err(BasemapError::new(
                CODE_BASEMAP_STATUS_INVALID,
                format!("{key}: {:?} 必须提供三个误差字段", basemap.status),
            ));
        }
        if errors
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(BasemapError::new(
                CODE_BASEMAP_STATUS_INVALID,
                format!("{key}: 误差字段必须是非负有限数"),
            ));
        }
        if basemap.error_source == Some(ErrorSource::Measured)
            && basemap.validation_samples.unwrap_or(0) == 0
        {
            return Err(BasemapError::new(
                CODE_BASEMAP_STATUS_INVALID,
                format!("{key}: error_source=measured 时 validation_samples 必须 ≥ 1"),
            ));
        }

        // D5 门槛：声明什么状态，就必须达到对应精度，不做静默降级。
        let diagonal = basemap.diagonal_px();
        let median = basemap.median_error_px.unwrap_or(f64::INFINITY);
        let p95 = basemap.p95_error_px.unwrap_or(f64::INFINITY);
        let (median_limit, p95_limit) = match basemap.status {
            BasemapStatus::Calibrated => (0.005 * diagonal, Some(0.01 * diagonal)),
            BasemapStatus::Approximate => (0.02 * diagonal, None),
            BasemapStatus::Unverified => (f64::INFINITY, None),
        };
        let p95_ok = match p95_limit {
            Some(limit) => p95 <= limit,
            None => true,
        };
        let meets = median <= median_limit && p95_ok;
        if !meets {
            return Err(BasemapError::new(
                CODE_BASEMAP_PRECISION_INSUFFICIENT,
                format!(
                    "{key}: status={:?} 但中位 {median:.2}px / P95 {p95:.2}px 未达门槛（对角线 {diagonal:.1}px）",
                    basemap.status
                ),
            ));
        }
        Ok(())
    }

    fn validate_sources(&self, key: &str, basemap: &Basemap) -> Result<(), BasemapError> {
        if basemap.source_refs.is_empty() {
            return Err(BasemapError::new(
                CODE_BASEMAP_SOURCE_MISSING,
                format!("{key}: source_refs 为空"),
            ));
        }
        for reference in &basemap.source_refs {
            if !self.sources.contains_key(reference) {
                return Err(BasemapError::new(
                    CODE_BASEMAP_SOURCE_MISSING,
                    format!("{key}: 来源 {reference:?} 未在 sources 中声明"),
                ));
            }
        }
        Ok(())
    }
}

/// 只允许小写字母、数字、下划线与连字符。
fn is_slug(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}
