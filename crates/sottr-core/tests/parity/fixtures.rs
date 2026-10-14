//! 差分回归夹具清单（SX-010）。
//!
//! - 私有五文件：名称、种类与业务期望（期望值来自已公开的 SX-001 基线记录）；
//!   样本字节与哈希不随仓库分发。
//! - 公开样本：20 个战役样本 + 额外条目（公开 NG+ 资料文件、挑战古墓文件）；
//!   由 `fixtures/build_public_inventory.py` 从研究包公开证据生成（仅公开数据）。

use std::collections::BTreeMap;

use serde::Deserialize;

/// 夹具种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureKind {
    /// 普通战役存档（4 MiB 家族）。
    Campaign,
    /// 资料文件（仅容器/哈希/原始观察）。
    Profile,
    /// 挑战古墓（Python v0.2 名称模式不匹配，不在差分对照）。
    ChallengeTomb,
}

impl FixtureKind {
    /// 稳定的字符串名（进入报告）。
    pub fn as_str(self) -> &'static str {
        match self {
            FixtureKind::Campaign => "campaign",
            FixtureKind::Profile => "profile",
            FixtureKind::ChallengeTomb => "challenge_tomb",
        }
    }
}

/// 业务期望（对照双方实现之外，另行固定的基线值）。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Expectations {
    /// 记录总数（前缀 + 溢出）。
    pub record_count: Option<u64>,
    /// 唯一 ID 数。
    pub unique_id_count: Option<u64>,
    /// 整体完成度候选的真实 float32 原值（以 f64 书写，必须精确相等）。
    pub overall_f32: Option<f64>,
    /// 解压字节数。
    pub expanded_bytes: Option<u64>,
    /// 参考差集合计（四个共同类别）。
    pub absent_total: Option<usize>,
    /// 参考差集分类别计数（元数据类别 ID）。
    pub absent_by_category: Option<BTreeMap<String, usize>>,
    /// 前缀表非零 ID 数。
    pub prefix_nonzero: Option<u64>,
    /// 前缀表命中字典的 ID 数。
    pub prefix_known: Option<u64>,
    /// 解压字节 SHA-256（公开证据核对）。
    pub raw_sha256: Option<String>,
}

/// 私有夹具条目。
#[derive(Debug, Clone, Deserialize)]
pub struct PrivateFixture {
    /// 文件名（不含目录）。
    pub name: String,
    /// 种类。
    pub kind: FixtureKind,
    /// 业务期望。
    #[serde(default)]
    pub expect: Expectations,
}

#[derive(Deserialize)]
struct PrivateDocument {
    samples: Vec<PrivateFixture>,
}

/// 私有五文件清单。
pub fn private_fixtures() -> Vec<PrivateFixture> {
    serde_json::from_str::<PrivateDocument>(include_str!("fixtures/private_samples.json"))
        .expect("私有夹具清单必须可解析")
        .samples
}

/// 公开归档来源（URL、ZIP 哈希与成员；用于来源核对，不随仓库分发存档字节）。
#[derive(Debug, Clone, Deserialize)]
pub struct PublicArchive {
    /// 归档名（公开分析证据中的路径前缀）。
    pub name: String,
    /// 下载 URL。
    pub url: String,
    /// ZIP SHA-256。
    pub sha256: String,
    /// ZIP 字节数。
    pub bytes: u64,
    /// 成员文件名。
    pub members: Vec<String>,
}

/// 公开样本条目。
#[derive(Debug, Clone, Deserialize)]
pub struct PublicSample {
    /// 所属归档名。
    pub archive: String,
    /// 归档内相对路径。
    pub relative_path: String,
    /// 文件名。
    pub name: String,
    /// 种类。
    pub kind: FixtureKind,
    /// 压缩文件 SHA-256；缺失表示公开证据未单列（按名称匹配）。
    pub compressed_sha256: Option<String>,
    /// 解压字节 SHA-256。
    pub raw_sha256: Option<String>,
    /// 解压字节数。
    pub raw_length: Option<u64>,
    /// 整体完成度候选（真实 float32 原值，以 f64 书写）。
    pub completion_candidate: Option<f64>,
    /// 前缀表命中字典的 ID 数。
    pub prefix_known: Option<u64>,
    /// 前缀表非零 ID 数。
    pub prefix_nonzero: Option<u64>,
    /// 备注（种类说明）。
    pub note: Option<String>,
}

/// 公开样本清单。
#[derive(Debug, Clone, Deserialize)]
pub struct PublicInventory {
    /// 归档来源。
    pub archives: Vec<PublicArchive>,
    /// 样本条目。
    pub samples: Vec<PublicSample>,
}

/// 公开样本清单（20 个战役样本 + 额外条目）。
pub fn public_inventory() -> PublicInventory {
    serde_json::from_str::<PublicInventory>(include_str!("fixtures/public_samples.json"))
        .expect("公开夹具清单必须可解析")
}
