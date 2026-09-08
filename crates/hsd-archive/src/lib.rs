//! HSD `.dat` archive parsing.
//!
//! Retail Melee loads archives into RAM, relocates the internal 32-bit
//! offsets into pointers in place, and overlays C structs directly on the
//! bytes. This crate is the **only** place in the port that knows the
//! on-disc layout. It reads big-endian fields into owned Rust types, and
//! nothing above it may depend on field offsets, struct sizes, or pointer
//! width.
//!
//! Reference: decomp `src/sysdolphin/baselib/archive.{h,c}` and the
//! `ASSERT_SIZE`/`ASSERT_OFFSET` annotations throughout `src/melee/*/types.h`.
//!
//! # On-disc layout
//!
//! From `HSD_ArchiveParse` (archive.c) and the structs in archive.h. The
//! five sections are packed back to back with no alignment padding; each
//! starts where the previous one ends.
//!
//! ```text
//! file offset                                   contents
//! ------------------------------------------    -----------------------------------------
//! 0x00                                          HSD_ArchiveHeader, 0x20 bytes
//!   0x00  u32    file_size                        whole file, header included
//!   0x04  u32    data_size                        byte length of the data section
//!   0x08  u32    nb_reloc                         entries in the relocation table
//!   0x0C  u32    nb_public                        entries in the public symbol table
//!   0x10  u32    nb_extern                        entries in the extern symbol table
//!   0x14  u8[4]  version                          unused by retail
//!   0x18  u32[2] pad
//! 0x20                                          data section, data_size bytes
//!                                                 HSD structs; pointer fields hold u32
//!                                                 offsets relative to this section
//! 0x20 + data_size                              relocation table, nb_reloc x 4 bytes
//!   +0x00 u32    offset                           HSD_ArchiveRelocationInfo: data offset
//!                                                 of a u32 slot that holds a data offset
//! ... + nb_reloc * 4                            public symbol table, nb_public x 8 bytes
//!   +0x00 u32    offset                           HSD_ArchivePublicInfo: data offset of
//!   +0x04 u32    symbol                           the root; string-table offset of its name
//! ... + nb_public * 8                           extern symbol table, nb_extern x 8 bytes
//!   +0x00 u32    offset                           HSD_ArchiveExternInfo: data offset of the
//!   +0x04 u32    symbol                           first patch site; string-table offset
//! ... + nb_extern * 8                           string table, NUL-terminated names,
//!                                                 running to file_size (may be empty)
//! ```
//!
//! Retail's `Locate` then adds the data base address to the `u32` at every
//! relocation slot, turning offsets into pointers. This crate does **not**
//! do that. The data section is kept verbatim in an owned `Vec<u8>` and
//! [`Archive::link`] / [`Archive::is_relocated_offset`] answer, per field,
//! whether a stored `u32` is a data offset. An extern's `offset` heads a
//! chain of slots each holding the next slot's offset, terminated by
//! `0xFFFF_FFFF` or an offset at or past `data_size` (`HSD_ArchiveLocateExtern`);
//! `lbArchive_InitializeDAT` walks every extern and patches its chain to
//! `NULL` immediately after parsing.
//!
//! # Modules
//!
//! - [`header`]: the 0x20-byte [`ArchiveHeader`].
//! - [`reader`]: bounds-checked big-endian [`Reader`].
//! - [`archive`]: [`Archive`], the parsed file.
//! - [`error`]: the crate's [`Error`] enum.

pub mod archive;
pub mod error;
pub mod header;
pub mod reader;

pub use archive::{Archive, ExternSymbol, PublicSymbol};
pub use error::{Error, Result, SymbolKind};
pub use header::ArchiveHeader;
pub use reader::{add_offset, Reader};
