//! Pure-Rust, zero-dependency streaming MD5, SHA-1 and SHA-256 for Rufus-style
//! image checksums (`#` button and `image.checksum` command).
//!
//! No `unsafe`, no panics, works on desktop and `wasm32-unknown-unknown`.

use serde::{Deserialize, Serialize};

/// Computed hexadecimal digests of an image file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checksums {
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
}

/// Streaming hasher computing MD5, SHA-1 and SHA-256 in a single pass.
pub struct MultiHasher {
    md5: Md5,
    sha1: Sha1,
    sha256: Sha256,
}

impl Default for MultiHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiHasher {
    pub fn new() -> Self {
        Self {
            md5: Md5::new(),
            sha1: Sha1::new(),
            sha256: Sha256::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.md5.update(data);
        self.sha1.update(data);
        self.sha256.update(data);
    }

    pub fn finalize(self) -> Checksums {
        Checksums {
            md5: to_hex(&self.md5.finalize()),
            sha1: to_hex(&self.sha1.finalize()),
            sha256: to_hex(&self.sha256.finalize()),
        }
    }
}

/// Convenience helper for in-memory slices.
pub fn hash_bytes(data: &[u8]) -> Checksums {
    let mut h = MultiHasher::new();
    h.update(data);
    h.finalize()
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

// ── MD5 (RFC 1321) ──────────────────────────────────────────────────────────

struct Md5 {
    state: [u32; 4],
    buf: [u8; 64],
    buf_len: usize,
    total_len: u64,
}

const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

impl Md5 {
    fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            buf: [0u8; 64],
            buf_len: 0,
            total_len: 0,
        }
    }

    fn compress(state: &mut [u32; 4], block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            if let Some(slot) = m.get_mut(i) {
                *slot = u32::from_le_bytes([
                    chunk.first().copied().unwrap_or(0),
                    chunk.get(1).copied().unwrap_or(0),
                    chunk.get(2).copied().unwrap_or(0),
                    chunk.get(3).copied().unwrap_or(0),
                ]);
            }
        }
        let [mut a, mut b, mut c, mut d] = *state;
        for i in 0..64 {
            let (f, g) = if i < 16 {
                ((b & c) | (!b & d), i)
            } else if i < 32 {
                ((d & b) | (!d & c), (5 * i + 1) % 16)
            } else if i < 48 {
                (b ^ c ^ d, (3 * i + 5) % 16)
            } else {
                (c ^ (b | !d), (7 * i) % 16)
            };
            let word = m.get(g).copied().unwrap_or(0);
            let k = MD5_K.get(i).copied().unwrap_or(0);
            let s = MD5_S.get(i).copied().unwrap_or(0);
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(k)
                    .wrapping_add(word)
                    .rotate_left(s),
            );
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            if let (Some(dst), Some(src)) = (
                self.buf.get_mut(self.buf_len..self.buf_len + take),
                data.get(..take),
            ) {
                dst.copy_from_slice(src);
            }
            self.buf_len += take;
            data = data.get(take..).unwrap_or(&[]);
            if self.buf_len == 64 {
                Self::compress(&mut self.state, &self.buf);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut blk = [0u8; 64];
            if let Some(src) = data.get(..64) {
                blk.copy_from_slice(src);
            }
            Self::compress(&mut self.state, &blk);
            data = data.get(64..).unwrap_or(&[]);
        }
        if !data.is_empty() {
            if let Some(dst) = self.buf.get_mut(..data.len()) {
                dst.copy_from_slice(data);
            }
            self.buf_len = data.len();
        }
    }

    fn finalize(mut self) -> [u8; 16] {
        let bit_len = self.total_len.wrapping_mul(8);
        self.update(&[0x80]);
        if self.buf_len > 56 {
            let pad = [0u8; 64];
            let need = 64 - self.buf_len;
            self.update(pad.get(..need).unwrap_or(&[]));
        }
        let pad = [0u8; 56];
        let need = 56usize.saturating_sub(self.buf_len);
        self.update(pad.get(..need).unwrap_or(&[]));
        self.update(&bit_len.to_le_bytes());

        let mut out = [0u8; 16];
        for (i, w) in self.state.iter().enumerate() {
            if let Some(dst) = out.get_mut(i * 4..(i + 1) * 4) {
                dst.copy_from_slice(&w.to_le_bytes());
            }
        }
        out
    }
}

// ── SHA-1 (FIPS 180-4) ──────────────────────────────────────────────────────

struct Sha1 {
    state: [u32; 5],
    buf: [u8; 64],
    buf_len: usize,
    total_len: u64,
}

impl Sha1 {
    fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0],
            buf: [0u8; 64],
            buf_len: 0,
            total_len: 0,
        }
    }

    fn compress(state: &mut [u32; 5], block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            if let Some(slot) = w.get_mut(i) {
                *slot = u32::from_be_bytes([
                    chunk.first().copied().unwrap_or(0),
                    chunk.get(1).copied().unwrap_or(0),
                    chunk.get(2).copied().unwrap_or(0),
                    chunk.get(3).copied().unwrap_or(0),
                ]);
            }
        }
        for i in 16..80 {
            let v = w.get(i - 3).copied().unwrap_or(0)
                ^ w.get(i - 8).copied().unwrap_or(0)
                ^ w.get(i - 14).copied().unwrap_or(0)
                ^ w.get(i - 16).copied().unwrap_or(0);
            if let Some(slot) = w.get_mut(i) {
                *slot = v.rotate_left(1);
            }
        }
        let [mut a, mut b, mut c, mut d, mut e] = *state;
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = if i < 20 {
                ((b & c) | (!b & d), 0x5a827999u32)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ed9eba1u32)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8f1bbcdcu32)
            } else {
                (b ^ c ^ d, 0xca62c1d6u32)
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            if let (Some(dst), Some(src)) = (
                self.buf.get_mut(self.buf_len..self.buf_len + take),
                data.get(..take),
            ) {
                dst.copy_from_slice(src);
            }
            self.buf_len += take;
            data = data.get(take..).unwrap_or(&[]);
            if self.buf_len == 64 {
                Self::compress(&mut self.state, &self.buf);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut blk = [0u8; 64];
            if let Some(src) = data.get(..64) {
                blk.copy_from_slice(src);
            }
            Self::compress(&mut self.state, &blk);
            data = data.get(64..).unwrap_or(&[]);
        }
        if !data.is_empty() {
            if let Some(dst) = self.buf.get_mut(..data.len()) {
                dst.copy_from_slice(data);
            }
            self.buf_len = data.len();
        }
    }

    fn finalize(mut self) -> [u8; 20] {
        let bit_len = self.total_len.wrapping_mul(8);
        self.update(&[0x80]);
        if self.buf_len > 56 {
            let pad = [0u8; 64];
            let need = 64 - self.buf_len;
            self.update(pad.get(..need).unwrap_or(&[]));
        }
        let pad = [0u8; 56];
        let need = 56usize.saturating_sub(self.buf_len);
        self.update(pad.get(..need).unwrap_or(&[]));
        self.update(&bit_len.to_be_bytes());

        let mut out = [0u8; 20];
        for (i, w) in self.state.iter().enumerate() {
            if let Some(dst) = out.get_mut(i * 4..(i + 1) * 4) {
                dst.copy_from_slice(&w.to_be_bytes());
            }
        }
        out
    }
}

// ── SHA-256 (FIPS 180-4) ────────────────────────────────────────────────────

struct Sha256 {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_len: u64,
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: [0u8; 64],
            buf_len: 0,
            total_len: 0,
        }
    }

    fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            if let Some(slot) = w.get_mut(i) {
                *slot = u32::from_be_bytes([
                    chunk.first().copied().unwrap_or(0),
                    chunk.get(1).copied().unwrap_or(0),
                    chunk.get(2).copied().unwrap_or(0),
                    chunk.get(3).copied().unwrap_or(0),
                ]);
            }
        }
        for i in 16..64 {
            let w15 = w.get(i - 15).copied().unwrap_or(0);
            let w2 = w.get(i - 2).copied().unwrap_or(0);
            let s0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3);
            let s1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10);
            let v = w
                .get(i - 16)
                .copied()
                .unwrap_or(0)
                .wrapping_add(s0)
                .wrapping_add(w.get(i - 7).copied().unwrap_or(0))
                .wrapping_add(s1);
            if let Some(slot) = w.get_mut(i) {
                *slot = v;
            }
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let ki = SHA256_K.get(i).copied().unwrap_or(0);
            let wi = w.get(i).copied().unwrap_or(0);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(ki)
                .wrapping_add(wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            if let (Some(dst), Some(src)) = (
                self.buf.get_mut(self.buf_len..self.buf_len + take),
                data.get(..take),
            ) {
                dst.copy_from_slice(src);
            }
            self.buf_len += take;
            data = data.get(take..).unwrap_or(&[]);
            if self.buf_len == 64 {
                Self::compress(&mut self.state, &self.buf);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut blk = [0u8; 64];
            if let Some(src) = data.get(..64) {
                blk.copy_from_slice(src);
            }
            Self::compress(&mut self.state, &blk);
            data = data.get(64..).unwrap_or(&[]);
        }
        if !data.is_empty() {
            if let Some(dst) = self.buf.get_mut(..data.len()) {
                dst.copy_from_slice(data);
            }
            self.buf_len = data.len();
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total_len.wrapping_mul(8);
        self.update(&[0x80]);
        if self.buf_len > 56 {
            let pad = [0u8; 64];
            let need = 64 - self.buf_len;
            self.update(pad.get(..need).unwrap_or(&[]));
        }
        let pad = [0u8; 56];
        let need = 56usize.saturating_sub(self.buf_len);
        self.update(pad.get(..need).unwrap_or(&[]));
        self.update(&bit_len.to_be_bytes());

        let mut out = [0u8; 32];
        for (i, w) in self.state.iter().enumerate() {
            if let Some(dst) = out.get_mut(i * 4..(i + 1) * 4) {
                dst.copy_from_slice(&w.to_be_bytes());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_vectors_match_rfc() {
        let c = hash_bytes(b"");
        assert_eq!(c.md5, "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(c.sha1, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            c.sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn abc_vectors_match_rfc() {
        let c = hash_bytes(b"abc");
        assert_eq!(c.md5, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(c.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            c.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn multi_block_streaming_matches_single_pass() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let single = hash_bytes(&data);
        let mut h = MultiHasher::new();
        for chunk in data.chunks(17) {
            h.update(chunk);
        }
        assert_eq!(h.finalize(), single);
    }
}
