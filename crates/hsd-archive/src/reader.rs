//! Bounds-checked big-endian reads at 32-bit offsets.
//!
//! The Gekko is big-endian and HSD archives store every multi-byte field in
//! that order. A [`Reader`] wraps a byte slice (normally an archive's data
//! section) and decodes scalars at an offset, returning [`Error::OutOfBounds`]
//! rather than panicking when the offset runs off the end.
//!
//! Offsets are `u32` because that is what the archive stores: 32-bit
//! pointers, un-relocated, relative to the start of the data section.

use crate::error::{Error, Result};

/// A big-endian view over a byte slice.
#[derive(Debug, Clone, Copy)]
pub struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Wrap a byte slice.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Length of the underlying slice in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// `true` if the underlying slice is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The whole underlying slice.
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// `len` bytes starting at `offset`.
    pub fn slice(&self, offset: u32, len: u32) -> Result<&'a [u8]> {
        let start = offset as usize;
        let end = start
            .checked_add(len as usize)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(Error::OutOfBounds {
                offset,
                len,
                available: self.bytes.len(),
            })?;
        Ok(&self.bytes[start..end])
    }

    /// Fixed-size array of bytes at `offset`.
    pub fn array<const N: usize>(&self, offset: u32) -> Result<[u8; N]> {
        let s = self.slice(offset, N as u32)?;
        let mut out = [0u8; N];
        out.copy_from_slice(s);
        Ok(out)
    }

    /// Unsigned 8-bit value at `offset`.
    pub fn u8(&self, offset: u32) -> Result<u8> {
        Ok(self.array::<1>(offset)?[0])
    }

    /// Big-endian unsigned 16-bit value at `offset`.
    pub fn u16(&self, offset: u32) -> Result<u16> {
        Ok(u16::from_be_bytes(self.array(offset)?))
    }

    /// Big-endian unsigned 32-bit value at `offset`.
    pub fn u32(&self, offset: u32) -> Result<u32> {
        Ok(u32::from_be_bytes(self.array(offset)?))
    }

    /// Signed 8-bit value at `offset`.
    pub fn s8(&self, offset: u32) -> Result<i8> {
        Ok(self.u8(offset)? as i8)
    }

    /// Big-endian signed 16-bit value at `offset`.
    pub fn s16(&self, offset: u32) -> Result<i16> {
        Ok(i16::from_be_bytes(self.array(offset)?))
    }

    /// Big-endian signed 32-bit value at `offset`.
    pub fn s32(&self, offset: u32) -> Result<i32> {
        Ok(i32::from_be_bytes(self.array(offset)?))
    }

    /// Big-endian IEEE single at `offset`. This is a bit reinterpretation
    /// only; no arithmetic is performed.
    pub fn f32(&self, offset: u32) -> Result<f32> {
        Ok(f32::from_be_bytes(self.array(offset)?))
    }

    /// An un-relocated pointer field at `offset`: a big-endian `u32` data
    /// offset, where `0` means null and yields `None`.
    ///
    /// This is the convention retail files follow for the overwhelming
    /// majority of pointer fields, but it is a heuristic: a genuine link to
    /// the struct at data offset 0 also stores `0`. The authoritative test
    /// is whether the field's own offset appears in the relocation table;
    /// use [`Archive::link`](crate::Archive::link) when that matters.
    pub fn offset(&self, offset: u32) -> Result<Option<u32>> {
        let v = self.u32(offset)?;
        Ok(if v == 0 { None } else { Some(v) })
    }

    /// NUL-terminated string starting at `offset`, without the terminator.
    pub fn cstr(&self, offset: u32) -> Result<&'a str> {
        let start = offset as usize;
        if start > self.bytes.len() {
            return Err(Error::OutOfBounds {
                offset,
                len: 1,
                available: self.bytes.len(),
            });
        }
        let rest = &self.bytes[start..];
        let end = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(Error::UnterminatedString { offset })?;
        core::str::from_utf8(&rest[..end]).map_err(|_| Error::InvalidUtf8 { offset })
    }
}

/// `base + add` as a `u32` data offset, or [`Error::OffsetOverflow`].
///
/// Higher layers use this when stepping from a struct's base offset to one
/// of its fields so that a corrupt base cannot wrap around silently.
pub fn add_offset(base: u32, add: u32) -> Result<u32> {
    base.checked_add(add)
        .ok_or(Error::OffsetOverflow { offset: base, add })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BYTES: [u8; 12] = [
        0x80, 0x01, // u16 0x8001 / s16 -32767
        0xFF, 0x7F, // s8 -1, u8 0x7F
        0x3F, 0x80, 0x00, 0x00, // f32 1.0
        0xFF, 0xFF, 0xFF, 0xFE, // s32 -2 / u32 0xFFFFFFFE
    ];

    #[test]
    fn scalar_reads() {
        let r = Reader::new(&BYTES);
        assert_eq!(r.len(), 12);
        assert!(!r.is_empty());
        assert_eq!(r.u16(0).unwrap(), 0x8001);
        assert_eq!(r.s16(0).unwrap(), -32767);
        assert_eq!(r.s8(2).unwrap(), -1);
        assert_eq!(r.u8(3).unwrap(), 0x7F);
        assert_eq!(r.f32(4).unwrap().to_bits(), 1.0f32.to_bits());
        assert_eq!(r.s32(8).unwrap(), -2);
        assert_eq!(r.u32(8).unwrap(), 0xFFFF_FFFE);
        assert_eq!(r.array::<2>(2).unwrap(), [0xFF, 0x7F]);
        assert_eq!(r.slice(4, 4).unwrap(), &BYTES[4..8]);
        assert_eq!(r.slice(12, 0).unwrap(), &[] as &[u8]);
    }

    #[test]
    fn offset_null_is_none() {
        let bytes = [0, 0, 0, 0, 0, 0, 0, 0x10];
        let r = Reader::new(&bytes);
        assert_eq!(r.offset(0).unwrap(), None);
        assert_eq!(r.offset(4).unwrap(), Some(0x10));
    }

    #[test]
    fn out_of_bounds_is_an_error_not_a_panic() {
        let r = Reader::new(&BYTES);
        assert_eq!(
            r.u32(9),
            Err(Error::OutOfBounds {
                offset: 9,
                len: 4,
                available: 12
            })
        );
        assert_eq!(
            r.u8(12),
            Err(Error::OutOfBounds {
                offset: 12,
                len: 1,
                available: 12
            })
        );
        // An offset near u32::MAX must not wrap when the length is added.
        assert!(matches!(
            r.slice(u32::MAX, 4),
            Err(Error::OutOfBounds { .. })
        ));
        assert!(matches!(r.cstr(13), Err(Error::OutOfBounds { .. })));
    }

    #[test]
    fn cstr_reads() {
        let bytes = b"abc\0def\xFF\0ghi";
        let r = Reader::new(bytes);
        assert_eq!(r.cstr(0).unwrap(), "abc");
        assert_eq!(r.cstr(3).unwrap(), "");
        assert_eq!(r.cstr(4), Err(Error::InvalidUtf8 { offset: 4 }));
        assert_eq!(r.cstr(9), Err(Error::UnterminatedString { offset: 9 }));
        assert_eq!(r.cstr(12), Err(Error::UnterminatedString { offset: 12 }));
    }

    #[test]
    fn add_offset_checks_overflow() {
        assert_eq!(add_offset(0x10, 0x20).unwrap(), 0x30);
        assert_eq!(
            add_offset(u32::MAX, 1),
            Err(Error::OffsetOverflow {
                offset: u32::MAX,
                add: 1
            })
        );
    }
}
