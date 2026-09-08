//! Error type for archive parsing and bounds-checked reads.
//!
//! Plain enum, no dependencies. Every variant carries the numbers needed to
//! locate the fault in the file so a bad `.dat` can be diagnosed from the
//! message alone.

use core::fmt;

/// Which of the two symbol tables an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// `HSD_ArchivePublicInfo`: a named root exported by the archive.
    Public,
    /// `HSD_ArchiveExternInfo`: a named import the loader must patch in.
    Extern,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Public => f.write_str("public"),
            SymbolKind::Extern => f.write_str("extern"),
        }
    }
}

/// Everything that can go wrong while parsing an archive or reading from
/// one. Parsing never panics on malformed input; it returns one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A read of `len` bytes at `offset` ran past the end of a buffer of
    /// `available` bytes.
    OutOfBounds {
        offset: u32,
        len: u32,
        available: usize,
    },
    /// An offset arithmetic step overflowed `u32`. Only reachable with a
    /// hostile or corrupt file.
    OffsetOverflow { offset: u32, add: u32 },
    /// The buffer is shorter than the 0x20-byte `HSD_ArchiveHeader`.
    TruncatedHeader { available: usize },
    /// `header.file_size` disagrees with the buffer length. Retail reports
    /// this as a "byte-order mismatch" and refuses the file.
    FileSizeMismatch { header: u32, actual: usize },
    /// The header's section sizes add up to more than `file_size`.
    SectionOutOfBounds {
        section: &'static str,
        start: u64,
        len: u64,
        file_size: u32,
    },
    /// Relocation entry `index` names a slot at `offset` that does not fit
    /// in the `data_size`-byte data section. Retail would have written
    /// outside the archive here.
    RelocationOutOfBounds {
        index: u32,
        offset: u32,
        data_size: u32,
    },
    /// Symbol table entry `index` names a string at `symbol` that lies past
    /// the end of the `strings_len`-byte string table.
    SymbolOutOfBounds {
        kind: SymbolKind,
        index: u32,
        symbol: u32,
        strings_len: usize,
    },
    /// A string starting at `offset` has no NUL terminator before the end
    /// of its buffer.
    UnterminatedString { offset: u32 },
    /// A string starting at `offset` is not valid UTF-8. HSD symbol names
    /// are ASCII in practice, so this indicates a corrupt table.
    InvalidUtf8 { offset: u32 },
    /// Walking an extern's chain of patch sites revisited an offset, so the
    /// chain is cyclic and would never terminate.
    ExternChainCycle { name: String, offset: u32 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OutOfBounds {
                offset,
                len,
                available,
            } => write!(
                f,
                "read of {len} bytes at offset {offset:#x} exceeds buffer of {available:#x} bytes"
            ),
            Error::OffsetOverflow { offset, add } => {
                write!(f, "offset {offset:#x} + {add:#x} overflows u32")
            }
            Error::TruncatedHeader { available } => write!(
                f,
                "archive header needs 0x20 bytes but only {available:#x} are available"
            ),
            Error::FileSizeMismatch { header, actual } => write!(
                f,
                "header file_size {header:#x} does not match buffer length {actual:#x} (byte-order mismatch?)"
            ),
            Error::SectionOutOfBounds {
                section,
                start,
                len,
                file_size,
            } => write!(
                f,
                "{section} section at {start:#x} with length {len:#x} exceeds file_size {file_size:#x}"
            ),
            Error::RelocationOutOfBounds {
                index,
                offset,
                data_size,
            } => write!(
                f,
                "relocation entry {index} at data offset {offset:#x} exceeds data_size {data_size:#x}"
            ),
            Error::SymbolOutOfBounds {
                kind,
                index,
                symbol,
                strings_len,
            } => write!(
                f,
                "{kind} symbol {index} names string offset {symbol:#x} beyond string table of {strings_len:#x} bytes"
            ),
            Error::UnterminatedString { offset } => {
                write!(f, "string at offset {offset:#x} has no NUL terminator")
            }
            Error::InvalidUtf8 { offset } => {
                write!(f, "string at offset {offset:#x} is not valid UTF-8")
            }
            Error::ExternChainCycle { name, offset } => write!(
                f,
                "extern symbol {name:?} patch chain revisits data offset {offset:#x}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias used throughout the crate.
pub type Result<T> = core::result::Result<T, Error>;
