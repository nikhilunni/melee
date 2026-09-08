//! Whole-file archive parsing: header, data section, relocation table,
//! symbol tables, and string table.
//!
//! Ported from `HSD_ArchiveParse`, `HSD_ArchiveGetPublicAddress`,
//! `HSD_ArchiveGetExtern` and `HSD_ArchiveLocateExtern` in decomp
//! `src/sysdolphin/baselib/archive.c`. The one deliberate departure is that
//! retail relocates in place (`Locate` adds the data base address to every
//! slot named by the relocation table) whereas this crate leaves the data
//! section untouched and answers "where does this field point" on demand
//! through [`Archive::link`]. That keeps the data section a plain owned
//! `Vec<u8>` that can be read from any offset without pointer-width
//! assumptions leaking upward.

use crate::error::{Error, Result, SymbolKind};
use crate::header::ArchiveHeader;
use crate::reader::Reader;

/// One `HSD_ArchivePublicInfo` entry: a named root exported by the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicSymbol {
    /// Offset of the root struct in the data section.
    pub offset: u32,
    /// Symbol name, resolved from the string table.
    pub name: String,
}

/// One `HSD_ArchiveExternInfo` entry: a named import that the loader patches
/// into the data section after parsing.
///
/// `offset` is the head of a singly linked chain of patch sites: each slot
/// holds the offset of the next slot, terminated by `0xFFFF_FFFF` or by any
/// value at or beyond `data_size`. See [`Archive::extern_sites`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternSymbol {
    /// Offset of the first patch site in the data section.
    pub offset: u32,
    /// Symbol name, resolved from the string table.
    pub name: String,
}

/// A parsed `.dat` archive with its data section still in on-disc form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive {
    header: ArchiveHeader,
    data: Vec<u8>,
    /// Relocation slots, sorted ascending so membership is a binary search.
    relocs: Vec<u32>,
    publics: Vec<PublicSymbol>,
    externs: Vec<ExternSymbol>,
}

/// Size of one `HSD_ArchiveRelocationInfo`.
const RELOC_ENTRY: u64 = 4;
/// Size of one `HSD_ArchivePublicInfo` / `HSD_ArchiveExternInfo`.
const SYMBOL_ENTRY: u64 = 8;

impl Archive {
    /// Parse a complete `.dat` file.
    ///
    /// Mirrors `HSD_ArchiveParse`: the header's `file_size` must equal
    /// `bytes.len()`, then the data section, relocation table, public table,
    /// extern table, and string table are laid out back to back with no
    /// alignment padding between them. Beyond what retail checks, this also
    /// rejects tables that run past `file_size`, relocation slots outside
    /// the data section, and symbol names outside the string table, since
    /// each of those would make the file unusable anyway.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = ArchiveHeader::parse(bytes)?;
        if header.file_size as usize != bytes.len() {
            return Err(Error::FileSizeMismatch {
                header: header.file_size,
                actual: bytes.len(),
            });
        }

        // Section boundaries, in u64 so hostile counts cannot wrap.
        let file_size = header.file_size as u64;
        let mut cursor = ArchiveHeader::SIZE as u64;
        let mut take = |section: &'static str, len: u64| -> Result<(usize, usize)> {
            let start = cursor;
            let end = start + len;
            if end > file_size {
                return Err(Error::SectionOutOfBounds {
                    section,
                    start,
                    len,
                    file_size: header.file_size,
                });
            }
            cursor = end;
            Ok((start as usize, end as usize))
        };
        let data_range = take("data", header.data_size as u64)?;
        let reloc_range = take("relocation", header.nb_reloc as u64 * RELOC_ENTRY)?;
        let public_range = take("public", header.nb_public as u64 * SYMBOL_ENTRY)?;
        let extern_range = take("extern", header.nb_extern as u64 * SYMBOL_ENTRY)?;
        // Retail: `if (offset < file_size) symbols = src + offset;` so the
        // string table is simply whatever remains, possibly nothing.
        let strings = Reader::new(&bytes[cursor as usize..]);

        let data = bytes[data_range.0..data_range.1].to_vec();

        let reloc_table = Reader::new(&bytes[reloc_range.0..reloc_range.1]);
        let mut relocs = Vec::with_capacity(header.nb_reloc as usize);
        for i in 0..header.nb_reloc {
            let offset = reloc_table.u32(i * RELOC_ENTRY as u32)?;
            // The slot itself is a u32 that retail rewrites; it must lie
            // wholly inside the data section.
            if (offset as u64) + RELOC_ENTRY > header.data_size as u64 {
                return Err(Error::RelocationOutOfBounds {
                    index: i,
                    offset,
                    data_size: header.data_size,
                });
            }
            relocs.push(offset);
        }
        relocs.sort_unstable();

        let publics = parse_symbols(
            SymbolKind::Public,
            Reader::new(&bytes[public_range.0..public_range.1]),
            header.nb_public,
            strings,
        )?
        .into_iter()
        .map(|(offset, name)| PublicSymbol { offset, name })
        .collect();

        let externs = parse_symbols(
            SymbolKind::Extern,
            Reader::new(&bytes[extern_range.0..extern_range.1]),
            header.nb_extern,
            strings,
        )?
        .into_iter()
        .map(|(offset, name)| ExternSymbol { offset, name })
        .collect();

        Ok(Self {
            header,
            data,
            relocs,
            publics,
            externs,
        })
    }

    /// The decoded header.
    pub fn header(&self) -> &ArchiveHeader {
        &self.header
    }

    /// The data section, exactly as stored on disc (offsets, not pointers).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// A big-endian reader over the data section.
    pub fn reader(&self) -> Reader<'_> {
        Reader::new(&self.data)
    }

    /// Data offsets of every `u32` slot the relocation table says holds a
    /// data offset (that is, every pointer field retail would have patched
    /// in `Locate`). Sorted ascending; duplicates, if the file has any, are
    /// preserved.
    pub fn reloc_targets(&self) -> &[u32] {
        &self.relocs
    }

    /// `true` if `offset` is named by the relocation table, meaning the
    /// `u32` stored there is a data offset rather than a plain integer.
    pub fn is_relocated_offset(&self, offset: u32) -> bool {
        self.relocs.binary_search(&offset).is_ok()
    }

    /// Follow a pointer field at data offset `offset`.
    ///
    /// Returns `Some(target)` if the slot is in the relocation table (it is
    /// a real link, and `target` is the data offset it points at), `None` if
    /// the slot is not relocated (a null pointer, or not a pointer at all).
    /// Errors only if `offset` is out of bounds, which cannot happen for a
    /// relocated slot because [`Archive::parse`] checked them all.
    pub fn link(&self, offset: u32) -> Result<Option<u32>> {
        if !self.is_relocated_offset(offset) {
            // Still bounds-check so a bad offset is reported, not hidden.
            self.reader().u32(offset)?;
            return Ok(None);
        }
        Ok(Some(self.reader().u32(offset)?))
    }

    /// All public symbols in table order.
    pub fn publics(&self) -> &[PublicSymbol] {
        &self.publics
    }

    /// Data offset of the public symbol called `name`, or `None` if the
    /// archive exports no such root. First match wins, as in
    /// `HSD_ArchiveGetPublicAddress`.
    pub fn public(&self, name: &str) -> Option<u32> {
        self.publics
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.offset)
    }

    /// All extern symbols in table order.
    pub fn externs(&self) -> &[ExternSymbol] {
        &self.externs
    }

    /// Name of the extern at `index`, as `HSD_ArchiveGetExtern`.
    pub fn extern_name(&self, index: usize) -> Option<&str> {
        self.externs.get(index).map(|e| e.name.as_str())
    }

    /// Data offsets of every slot that must be patched with the address of
    /// the extern called `name`, in chain order; empty if there is no such
    /// extern.
    ///
    /// Walks the chain exactly as `HSD_ArchiveLocateExtern`: starting at the
    /// extern's `offset`, each slot's current value is the next slot, and
    /// the walk stops at `0xFFFF_FFFF` or any offset at or beyond
    /// `data_size`. A revisited slot is reported as
    /// [`Error::ExternChainCycle`] instead of looping forever.
    pub fn extern_sites(&self, name: &str) -> Result<Vec<u32>> {
        let Some(head) = self.externs.iter().find(|e| e.name == name) else {
            return Ok(Vec::new());
        };
        let reader = self.reader();
        let mut sites = Vec::new();
        let mut offset = head.offset;
        while offset != u32::MAX && offset < self.header.data_size {
            if sites.contains(&offset) {
                return Err(Error::ExternChainCycle {
                    name: name.to_owned(),
                    offset,
                });
            }
            sites.push(offset);
            offset = reader.u32(offset)?;
        }
        Ok(sites)
    }
}

/// Decode `count` `{u32 offset, u32 symbol}` entries and resolve each
/// `symbol` (an offset into the string table) to an owned name.
fn parse_symbols(
    kind: SymbolKind,
    table: Reader<'_>,
    count: u32,
    strings: Reader<'_>,
) -> Result<Vec<(u32, String)>> {
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        let base = i * SYMBOL_ENTRY as u32;
        let offset = table.u32(base)?;
        let symbol = table.u32(base + 4)?;
        if symbol as usize >= strings.len() {
            return Err(Error::SymbolOutOfBounds {
                kind,
                index: i,
                symbol,
                strings_len: strings.len(),
            });
        }
        let name = strings.cstr(symbol)?.to_owned();
        out.push((offset, name));
    }
    Ok(out)
}
