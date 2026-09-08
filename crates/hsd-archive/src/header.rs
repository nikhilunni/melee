//! The fixed 0x20-byte archive header.
//!
//! Layout, from `HSD_ArchiveHeader` in decomp `src/sysdolphin/baselib/archive.h`
//! (`ASSERT_SIZE(struct HSD_ArchiveHeader, 0x20)`):
//!
//! | offset | type    | field       | meaning                                          |
//! |-------:|---------|-------------|--------------------------------------------------|
//! | `0x00` | `u32`   | `file_size` | total byte length of the file, header included   |
//! | `0x04` | `u32`   | `data_size` | byte length of the data section                  |
//! | `0x08` | `u32`   | `nb_reloc`  | entry count of the relocation table              |
//! | `0x0C` | `u32`   | `nb_public` | entry count of the public symbol table           |
//! | `0x10` | `u32`   | `nb_extern` | entry count of the extern symbol table           |
//! | `0x14` | `u8[4]` | `version`   | four raw bytes; retail never reads them          |
//! | `0x18` | `u32[2]`| `pad`       | unused                                           |
//!
//! `HSD_ArchiveParse` checks only that `file_size` equals the byte count it
//! was handed; a mismatch is reported as a byte-order problem and the file is
//! rejected. Everything else is trusted, which is why the parser in
//! [`crate::archive`] adds its own bounds checks.

use crate::error::{Error, Result};
use crate::reader::Reader;

/// The 0x20-byte archive header. See `HSD_ArchiveHeader` in the decomp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveHeader {
    /// Total file length in bytes, header included.
    pub file_size: u32,
    /// Length of the data section in bytes.
    pub data_size: u32,
    /// Number of `u32` entries in the relocation table.
    pub nb_reloc: u32,
    /// Number of 8-byte entries in the public symbol table.
    pub nb_public: u32,
    /// Number of 8-byte entries in the extern symbol table.
    pub nb_extern: u32,
    /// Raw version bytes (`u8 version[4]`). Retail never inspects them.
    pub version: [u8; 4],
}

impl ArchiveHeader {
    /// `sizeof(HSD_ArchiveHeader)`.
    pub const SIZE: usize = 0x20;

    /// Decode the header from the first 0x20 bytes of `bytes`.
    ///
    /// Only the buffer length is checked here; `file_size` against the
    /// actual buffer length is checked by [`crate::Archive::parse`].
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(Error::TruncatedHeader {
                available: bytes.len(),
            });
        }
        let r = Reader::new(&bytes[..Self::SIZE]);
        Ok(Self {
            file_size: r.u32(0x00)?,
            data_size: r.u32(0x04)?,
            nb_reloc: r.u32(0x08)?,
            nb_public: r.u32(0x0C)?,
            nb_extern: r.u32(0x10)?,
            version: r.array::<4>(0x14)?,
        })
    }

    /// Encode the header as it appears on disc. Used by tests and tooling
    /// that construct archives; padding is written as zero.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0x00..0x04].copy_from_slice(&self.file_size.to_be_bytes());
        out[0x04..0x08].copy_from_slice(&self.data_size.to_be_bytes());
        out[0x08..0x0C].copy_from_slice(&self.nb_reloc.to_be_bytes());
        out[0x0C..0x10].copy_from_slice(&self.nb_public.to_be_bytes());
        out[0x10..0x14].copy_from_slice(&self.nb_extern.to_be_bytes());
        out[0x14..0x18].copy_from_slice(&self.version);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let h = ArchiveHeader {
            file_size: 0x1234,
            data_size: 0x1000,
            nb_reloc: 3,
            nb_public: 2,
            nb_extern: 1,
            version: *b"001B",
        };
        let bytes = h.to_bytes();
        assert_eq!(&bytes[0x18..], &[0u8; 8]);
        assert_eq!(ArchiveHeader::parse(&bytes).unwrap(), h);
    }

    #[test]
    fn truncated() {
        assert_eq!(
            ArchiveHeader::parse(&[0u8; 0x1F]),
            Err(Error::TruncatedHeader { available: 0x1F })
        );
    }
}
