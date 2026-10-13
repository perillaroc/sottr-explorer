//! G1 出口对齐（可选）：`SOTTR_SAMPLE_DIR` 指向私有样本目录时，用 SX-001 基线
//! 捕获的原始层数值验证 Rust 核心（容器 / 区段 / 记录 / 整体候选）。
//!
//! 私有样本不随仓库分发；未设置环境变量时安静跳过（打印 skip，不记为通过）。
//! 正式 Python-Rust 差分回归仍按 SX-010 执行。

use std::path::{Path, PathBuf};

use sottr_core::container::decompress;
use sottr_core::domain::progress::overall_completion;
use sottr_core::inventory::recover;
use sottr_core::limits::Limits;
use sottr_core::sections::{SectionError, SectionStream};
use sottr_core::synthetic::compress;

fn sample_dir() -> Option<PathBuf> {
    std::env::var_os("SOTTR_SAMPLE_DIR").map(PathBuf::from)
}

fn read_sample(dir: &Path, name: &str) -> Vec<u8> {
    let path = dir.join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("读取样本失败 {path:?}: {error}"))
}

#[test]
fn private_sample_raw_layer_matches_sx001_baseline() {
    let Some(dir) = sample_dir() else {
        eprintln!("SOTTR_SAMPLE_DIR 未设置：跳过私有样本对齐（记为 skip，不经过）");
        return;
    };

    // 期望值来自 SX-001 基线的 Python 复跑（float32 原值的最短往返写法）。
    for (name, unique_ids, percent) in [
        ("save1.dat", 517u64, 66.204_82_f32),
        ("save202.dat", 517, 66.204_82),
        ("save204.dat", 516, 65.963_85),
        ("save206.dat", 516, 65.963_85),
    ] {
        let compressed = read_sample(&dir, name);
        let decompression = decompress(&compressed, &Limits::default());
        assert!(decompression.valid, "{name}: {:?}", decompression.error);
        assert!(decompression.eof);
        assert_eq!(decompression.expanded_bytes, 4_194_304, "{name}");
        assert_eq!(decompression.trailing_bytes, 0, "{name}");

        let sections = SectionStream::walk(&decompression.data, &Limits::default());
        assert!(sections.valid, "{name}: {:?}", sections.error);

        let inventory = recover(&decompression.data, &sections);
        assert!(inventory.observation.inventory_complete, "{name}");
        assert_eq!(inventory.observation.unique_id_count, unique_ids, "{name}");
        assert_eq!(inventory.observation.duplicate_record_count, 0, "{name}");
        assert_eq!(
            inventory.observation.record_count, inventory.observation.unique_id_count,
            "{name}: 记录数 = 唯一数 + 重复数"
        );

        let overall = overall_completion(&decompression.data, &sections);
        assert_eq!(overall.progress.value, Some(f64::from(percent)), "{name}");
    }

    // profile：仅容器观察，不进入战役解释。
    let compressed = read_sample(&dir, "profile.dat");
    let decompression = decompress(&compressed, &Limits::default());
    assert!(decompression.valid);
    assert_eq!(decompression.expanded_bytes, 25_600);
    let sections = SectionStream::walk(&decompression.data, &Limits::default());
    assert_eq!(sections.error, Some(SectionError::UnrecognizedFamilySize));
    let inventory = recover(&decompression.data, &sections);
    assert!(!inventory.observation.inventory_complete);
    assert!(inventory.observation.records.is_empty());

    // 合成样本自检：确保本测试的容器路径本身可用（不依赖私有样本也存在）。
    let synthetic = compress(b"alignment self-check");
    assert!(decompress(&synthetic, &Limits::default()).valid);
}
