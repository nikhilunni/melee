//! HSD `.dat` archive parsing.
//!
//! Retail Melee loads archives into RAM, relocates the internal 32-bit
//! offsets into pointers in place, and overlays C structs directly on the
//! bytes. This crate is the **only** place in the port that knows the
//! on-disc layout. It reads big-endian fields into owned Rust types, and
//! nothing above it may depend on field offsets, struct sizes, or pointer
//! width.
//!
//! Reference: decomp `src/sysdolphin/baselib/archive.c` and the
//! `ASSERT_SIZE`/`ASSERT_OFFSET` annotations throughout `src/melee/*/types.h`.

pub mod header {
    /// The 0x20-byte archive header. See `HSD_ArchiveHeader` in the decomp.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ArchiveHeader {
        pub file_size: u32,
        pub data_size: u32,
        pub nb_reloc: u32,
        pub nb_public: u32,
        pub nb_extern: u32,
        pub version: u32,
    }

    impl ArchiveHeader {
        pub const SIZE: usize = 0x20;

        pub fn parse(bytes: &[u8]) -> Option<Self> {
            if bytes.len() < Self::SIZE {
                return None;
            }
            let be = |o: usize| u32::from_be_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
            Some(Self {
                file_size: be(0x00),
                data_size: be(0x04),
                nb_reloc: be(0x08),
                nb_public: be(0x0C),
                nb_extern: be(0x10),
                version: be(0x14),
            })
        }
    }
}

