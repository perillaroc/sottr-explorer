//! 最小 SHA-256（FIPS 180-4）实现：用于元数据内容哈希与输入摘要。
//!
//! 边界（SX-008）：
//! - 纯 Rust、无第三方依赖；只做内容指纹与漂移检测，**不用于安全边界或密钥用途**；
//! - 输出小写十六进制；NIST/FIPS 标准测试向量在单元测试中固定；
//! - 如后续任务（SX-013/SX-018）需要审计级依赖，可在此替换为审计库，
//!   接口保持不变（`sha256_hex`）。

const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

const INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// 增量 SHA-256 计算器。
#[derive(Debug, Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    length_bytes: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// 新建计算器。
    pub fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            buffer: [0u8; 64],
            buffered: 0,
            length_bytes: 0,
        }
    }

    /// 追加输入字节。
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length_bytes = self.length_bytes.wrapping_add(bytes.len() as u64);

        if self.buffered > 0 {
            let take = (64 - self.buffered).min(bytes.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&bytes[..take]);
            self.buffered += take;
            bytes = &bytes[take..];
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while bytes.len() >= 64 {
            let (block, rest) = bytes.split_at(64);
            let mut fixed = [0u8; 64];
            fixed.copy_from_slice(block);
            self.compress(&fixed);
            bytes = rest;
        }
        if !bytes.is_empty() {
            self.buffer[..bytes.len()].copy_from_slice(bytes);
            self.buffered = bytes.len();
        }
    }

    /// 完成计算并返回 32 字节摘要。
    pub fn finish(mut self) -> [u8; 32] {
        let bit_length = self.length_bytes.wrapping_mul(8);
        let mut tail = [0u8; 72];
        tail[0] = 0x80;
        let pad = if self.buffered < 56 {
            56 - self.buffered
        } else {
            120 - self.buffered
        };
        let total = self.buffered + pad + 8;
        let mut data = [0u8; 192];
        data[..self.buffered].copy_from_slice(&self.buffer[..self.buffered]);
        data[self.buffered..self.buffered + pad].copy_from_slice(&tail[..pad]);
        data[self.buffered + pad..total].copy_from_slice(&bit_length.to_be_bytes());

        let mut offset = 0;
        while offset < total {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[offset..offset + 64]);
            self.compress(&block);
            offset += 64;
        }

        let mut digest = [0u8; 32];
        for (chunk, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    fn compress(&mut self, block: &[u8]) {
        let mut schedule = [0u32; 64];
        for (index, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
            schedule[index] = u32::from_be_bytes(*chunk);
        }
        for index in 16..64 {
            let previous_15 = schedule[index - 15];
            let previous_2 = schedule[index - 2];
            let s0 =
                previous_15.rotate_right(7) ^ previous_15.rotate_right(18) ^ (previous_15 >> 3);
            let s1 = previous_2.rotate_right(17) ^ previous_2.rotate_right(19) ^ (previous_2 >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(ROUND_CONSTANTS[index])
                .wrapping_add(schedule[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (state, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
    }
}

/// 计算 SHA-256 并以小写十六进制返回。
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finish();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn matches_fips_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn matches_long_message_vector() {
        let million_a = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256_hex(&million_a),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn block_boundary_lengths_are_consistent() {
        for length in 0..130usize {
            let data: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let digest = hasher.finish();
            // 分块追加与一次性追加必须一致。
            let mut chunked = Sha256::new();
            for chunk in data.chunks(7) {
                chunked.update(chunk);
            }
            assert_eq!(digest, chunked.finish(), "length {length}");
            assert_eq!(hex(&digest).len(), 64);
        }
    }
}
