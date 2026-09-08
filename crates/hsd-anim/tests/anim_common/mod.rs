//! Shared helpers for the `anim_*` tests: a keyframe-stream encoder that
//! mirrors the format documented at the top of `src/fobj.rs`, and a
//! deterministic RNG for the random sweeps.

#![allow(dead_code)]

use hsd_anim::fobj::{FracType, HSD_A_FRAC_FLOAT};

/// Builder for an FObj keyframe byte stream.
#[derive(Debug, Default, Clone)]
pub struct Stream {
    pub bytes: Vec<u8>,
}

impl Stream {
    pub fn new() -> Stream {
        Stream::default()
    }

    /// Pack header: `op` in the low nibble, `count - 1` split as three bits
    /// in bits 4-6 and then seven bits per continuation byte.
    pub fn pack(&mut self, op: u8, count: u32) -> &mut Self {
        assert!(count >= 1);
        let n = count - 1;
        let mut rest = n >> 3;
        let mut b0 = (op & 0xF) | (((n & 7) as u8) << 4);
        if rest != 0 {
            b0 |= 0x80;
        }
        self.bytes.push(b0);
        while rest != 0 {
            let mut b = (rest & 0x7F) as u8;
            rest >>= 7;
            if rest != 0 {
                b |= 0x80;
            }
            self.bytes.push(b);
        }
        self
    }

    /// A value record in the `frac` encoding. For integer encodings `v` is
    /// the raw integer numerator (the decoded value is `v / 2^shift`).
    pub fn raw(&mut self, frac: u8, v: i32) -> &mut Self {
        match frac & 0xE0 {
            0x60 | 0x80 => self.bytes.push(v as u8),
            0x20 | 0x40 => self.bytes.extend_from_slice(&(v as u16).to_le_bytes()),
            _ => panic!("raw() needs an integer frac, got {frac:#x}"),
        }
        self
    }

    /// A raw little-endian f32 record (`frac == HSD_A_FRAC_FLOAT`).
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.bytes.extend_from_slice(&v.to_bits().to_le_bytes());
        self
    }

    /// LEB128 wait var-int.
    pub fn wait(&mut self, mut w: u32) -> &mut Self {
        loop {
            let mut b = (w & 0x7F) as u8;
            w >>= 7;
            if w != 0 {
                b |= 0x80;
            }
            self.bytes.push(b);
            if w == 0 {
                break;
            }
        }
        self
    }

    pub fn finish(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

/// The value `parse_float` produces for an integer record: the same f32
/// operation sequence as the C (`(f32)numer / (f32)(1 << shift)`).
pub fn decoded(frac: u8, raw: i32) -> f32 {
    assert_ne!(frac, HSD_A_FRAC_FLOAT);
    let denom = 1i32.wrapping_shl((frac & 0x1F) as u32);
    raw as f32 / denom as f32
}

/// Integer range representable by a frac type.
pub fn raw_range(ty: FracType) -> (i32, i32) {
    match ty {
        FracType::S8 => (-128, 127),
        FracType::U8 => (0, 255),
        FracType::S16 => (-32768, 32767),
        FracType::U16 => (0, 65535),
        FracType::Float => (i32::MIN, i32::MAX),
    }
}

/// Deterministic 64-bit LCG (Knuth MMIX constants).
pub struct Lcg(pub u64);
impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    pub fn u32(&mut self) -> u32 {
        (self.next() >> 32) as u32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.u32() as u64 * n as u64) >> 32) as u32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u32) as i32
    }
    /// Uniform f32 in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}
