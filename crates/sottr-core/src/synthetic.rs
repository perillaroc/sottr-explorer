//! 合成夹具生成器（测试与夹具专用；不属于分析 API）。
//!
//! 用于构造 4 MiB 样本家族缓冲区、主流区段与 zlib 容器，驱动容器/区段/记录
//! 等模块的边界测试。生成过程完全在内存中，不做磁盘与网络访问。
//!
//! 注意：合成数据只用于验证解析边界，不代表真实存档内容或语义。

use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::sections::{
    CAMPAIGN_FAMILY_EXPANDED_BYTES, MAIN_STREAM_LENGTH_OFFSET, MAIN_STREAM_START,
};

/// 把字节压缩为 zlib 流。
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .expect("in-memory zlib encoding cannot fail");
    encoder
        .finish()
        .expect("in-memory zlib encoding cannot fail")
}

/// 已写入区段的句柄（用于写负载与断言偏移）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionHandle {
    /// 区段头的字节偏移。
    pub offset: usize,
    /// 负载区起始偏移。
    pub payload_offset: usize,
    /// 观察类型。
    pub section_type: u8,
    /// 含头的字节数。
    pub bytes: u32,
}

/// 4 MiB 样本家族缓冲区构建器；主流起点默认在 `0xBFD4`。
pub struct SampleBuilder {
    data: Vec<u8>,
    cursor: usize,
}

impl SampleBuilder {
    /// 新建全零的 4 MiB 缓冲区，写入游标位于主流起点。
    pub fn new() -> Self {
        Self {
            data: vec![0u8; CAMPAIGN_FAMILY_EXPANDED_BYTES],
            cursor: MAIN_STREAM_START,
        }
    }

    /// 完整缓冲区可变视图（写前缀、浮点字段、区段负载等）。
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// 在小端偏移处写入 u32。
    pub fn write_u32_le(&mut self, offset: usize, value: u32) -> &mut Self {
        let end = offset.checked_add(4).expect("offset overflow");
        self.data
            .get_mut(offset..end)
            .expect("write offset must be inside the family buffer")
            .copy_from_slice(&value.to_le_bytes());
        self
    }

    /// 写入前缀表的第 `index` 个 8 字节槽（内部 ID + 伴随字）。
    ///
    /// # Panics
    ///
    /// `index` 超出 512 槽时 panic（测试构造用）。
    pub fn write_prefix_slot(
        &mut self,
        index: usize,
        internal_id: u32,
        companion: u32,
    ) -> &mut Self {
        assert!(index < 512, "prefix slot index must be below 512");
        let offset = crate::inventory::PREFIX_OFFSET + index * 8;
        self.write_u32_le(offset, internal_id);
        self.write_u32_le(offset + 4, companion)
    }

    /// 追加一个合法区段：类型 + 负载字节数（负载须为 4 的倍数）。
    ///
    /// # Panics
    ///
    /// 负载未按 4 字节对齐或超出缓冲区时 panic（用于测试构造，不用于运行路径）。
    pub fn push_section(&mut self, section_type: u8, payload_bytes: u32) -> SectionHandle {
        assert!(
            payload_bytes.is_multiple_of(4),
            "payload must be 4-byte aligned"
        );
        let total = payload_bytes.checked_add(4).expect("section size overflow");
        let dwords = total / 4;
        assert!(
            dwords <= 0xFF_FFFF,
            "section dword count must fit in 24 bits"
        );
        let header = (dwords << 8) | u32::from(section_type);
        let offset = self.push_raw_header(header);
        let payload_offset = offset + 4;
        let end = self
            .cursor
            .checked_add(payload_bytes as usize)
            .expect("cursor overflow");
        assert!(
            end <= self.data.len(),
            "section payload exceeds the family buffer"
        );
        self.cursor = end;
        SectionHandle {
            offset,
            payload_offset,
            section_type,
            bytes: total,
        }
    }

    /// 写入一个原始 DWORD 段头（用于构造零长度/越界等异常流），仅前进 4 字节。
    ///
    /// 返回写入偏移。
    pub fn push_raw_header(&mut self, header: u32) -> usize {
        let offset = self.cursor;
        let end = offset.checked_add(4).expect("cursor overflow");
        assert!(
            end <= self.data.len(),
            "section header exceeds the family buffer"
        );
        self.data[offset..end].copy_from_slice(&header.to_le_bytes());
        self.cursor = end;
        offset
    }

    /// 结束构建：按已写入字节数写长度字段。
    pub fn finish(self) -> Vec<u8> {
        let written = (self.cursor - MAIN_STREAM_START) as u32;
        self.finish_with_declared_length(written)
    }

    /// 结束构建并写入指定的声明长度（用于构造异常流）。
    pub fn finish_with_declared_length(mut self, byte_length: u32) -> Vec<u8> {
        self.data[MAIN_STREAM_LENGTH_OFFSET..MAIN_STREAM_START]
            .copy_from_slice(&byte_length.to_le_bytes());
        self.data
    }
}

impl Default for SampleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::decompress;
    use crate::limits::Limits;
    use crate::sections::SectionStream;

    #[test]
    fn generated_sample_round_trips_through_container_and_sections() {
        let mut builder = SampleBuilder::new();
        let handle = builder.push_section(28, 4108);
        builder.data_mut()[handle.payload_offset..handle.payload_offset + 8]
            .copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let raw = builder.finish();

        let decompressed = decompress(&compress(&raw), &Limits::default());
        assert!(decompressed.valid, "{:?}", decompressed.error);
        assert_eq!(decompressed.data, raw);

        let stream = SectionStream::walk(&decompressed.data, &Limits::default());
        assert!(stream.valid, "{:?}", stream.error);
        assert_eq!(stream.section_count(), 1);
        assert_eq!(stream.sections[0].offset, handle.offset as u64);
        assert_eq!(stream.sections[0].bytes, 4112);
    }

    #[test]
    fn write_u32_le_is_little_endian() {
        let mut builder = SampleBuilder::new();
        builder.write_u32_le(0x10, 0xAABBCCDD);
        assert_eq!(&builder.data_mut()[0x10..0x14], &[0xDD, 0xCC, 0xBB, 0xAA]);
    }
}
