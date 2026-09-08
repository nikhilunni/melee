//! Round-trip tests against synthetic archives.
//!
//! There is no disc image in CI, so every archive here is built in-test by
//! [`Builder`], which lays sections out exactly as `HSD_ArchiveParse`
//! expects them (see the crate docs for the layout).

use hsd_archive::{Archive, ArchiveHeader, Error, SymbolKind};

/// Assembles a `.dat` file from its parts.
#[derive(Default)]
struct Builder {
    data: Vec<u8>,
    relocs: Vec<u32>,
    /// `(data offset, string-table offset)` pairs.
    publics: Vec<(u32, u32)>,
    /// `(data offset, string-table offset)` pairs.
    externs: Vec<(u32, u32)>,
    strings: Vec<u8>,
    version: [u8; 4],
    /// Overrides `file_size` in the header when set, to test the mismatch path.
    file_size_override: Option<u32>,
}

impl Builder {
    fn new() -> Self {
        Self {
            version: *b"001B",
            ..Self::default()
        }
    }

    /// Current data offset (where the next `push_*` lands).
    fn here(&self) -> u32 {
        self.data.len() as u32
    }

    fn push_u32(&mut self, v: u32) -> u32 {
        let at = self.here();
        self.data.extend_from_slice(&v.to_be_bytes());
        at
    }

    fn push_f32(&mut self, v: f32) -> u32 {
        self.push_u32(v.to_bits())
    }

    fn push_s16(&mut self, v: i16) -> u32 {
        let at = self.here();
        self.data.extend_from_slice(&v.to_be_bytes());
        at
    }

    /// Write a pointer field holding `target` and register it for relocation.
    fn push_link(&mut self, target: u32) -> u32 {
        let at = self.push_u32(target);
        self.relocs.push(at);
        at
    }

    fn align(&mut self, n: usize) {
        while !self.data.len().is_multiple_of(n) {
            self.data.push(0);
        }
    }

    /// Add a NUL-terminated name to the string table, returning its offset.
    fn string(&mut self, s: &str) -> u32 {
        let at = self.strings.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        self.strings.push(0);
        at
    }

    fn public(&mut self, data_offset: u32, name: &str) {
        let sym = self.string(name);
        self.publics.push((data_offset, sym));
    }

    fn extern_sym(&mut self, head: u32, name: &str) {
        let sym = self.string(name);
        self.externs.push((head, sym));
    }

    fn build(&self) -> Vec<u8> {
        let body_len = self.data.len()
            + self.relocs.len() * 4
            + self.publics.len() * 8
            + self.externs.len() * 8
            + self.strings.len();
        let file_size = (ArchiveHeader::SIZE + body_len) as u32;
        let header = ArchiveHeader {
            file_size: self.file_size_override.unwrap_or(file_size),
            data_size: self.data.len() as u32,
            nb_reloc: self.relocs.len() as u32,
            nb_public: self.publics.len() as u32,
            nb_extern: self.externs.len() as u32,
            version: self.version,
        };
        let mut out = Vec::with_capacity(file_size as usize);
        out.extend_from_slice(&header.to_bytes());
        out.extend_from_slice(&self.data);
        for r in &self.relocs {
            out.extend_from_slice(&r.to_be_bytes());
        }
        for (off, sym) in self.publics.iter().chain(&self.externs) {
            out.extend_from_slice(&off.to_be_bytes());
            out.extend_from_slice(&sym.to_be_bytes());
        }
        out.extend_from_slice(&self.strings);
        out
    }
}

/// Two structs linked by a pointer, one public root, one string.
///
/// ```text
/// data 0x00  Child  { u32 magic = 0xCAFEBABE; f32 scale = 2.5; s16 hp = -7; pad }
/// data 0x10  Parent { u32 flags = 0x1234; Child* child -> 0x00; u32 zero = 0 }
/// public "root" -> 0x10
/// reloc [0x14]
/// ```
fn two_structs() -> (Builder, u32, u32) {
    let mut b = Builder::new();
    let child = b.push_u32(0xCAFE_BABE);
    b.push_f32(2.5);
    b.push_s16(-7);
    b.align(0x10);
    assert_eq!(b.here(), 0x10);
    let parent = b.here();
    b.push_u32(0x1234);
    b.push_link(child);
    b.push_u32(0); // an un-relocated zero: a null pointer
    b.public(parent, "root");
    (b, child, parent)
}

#[test]
fn round_trips_two_linked_structs() {
    let (b, child, parent) = two_structs();
    let bytes = b.build();
    let a = Archive::parse(&bytes).expect("parse");

    // Header.
    let h = a.header();
    assert_eq!(h.file_size as usize, bytes.len());
    assert_eq!(h.data_size, 0x1C);
    assert_eq!(h.nb_reloc, 1);
    assert_eq!(h.nb_public, 1);
    assert_eq!(h.nb_extern, 0);
    assert_eq!(h.version, *b"001B");

    // Data section is verbatim.
    assert_eq!(a.data(), &b.data[..]);

    // Public symbol resolves to the parent.
    assert_eq!(a.public("root"), Some(parent));
    assert_eq!(a.public("missing"), None);
    assert_eq!(a.publics().len(), 1);
    assert_eq!(a.publics()[0].name, "root");
    assert!(a.externs().is_empty());
    assert_eq!(a.extern_name(0), None);

    // Relocation table.
    assert_eq!(a.reloc_targets(), &[parent + 4]);
    assert!(a.is_relocated_offset(parent + 4));
    assert!(!a.is_relocated_offset(parent));
    assert!(!a.is_relocated_offset(parent + 8));

    // Follow the link and read the child's fields. The child lives at data
    // offset 0, so the stored link value is 0: indistinguishable from a null
    // pointer by value alone. `Archive::link` consults the relocation table
    // and gets it right; `Reader::offset`'s "0 means null" heuristic does not.
    let r = a.reader();
    assert_eq!(child, 0);
    assert_eq!(r.u32(parent).unwrap(), 0x1234);
    assert_eq!(a.link(parent + 4).unwrap(), Some(child));
    assert_eq!(r.offset(parent + 4).unwrap(), None);
    // The zero field is not relocated: null either way.
    assert_eq!(a.link(parent + 8).unwrap(), None);
    assert_eq!(r.offset(parent + 8).unwrap(), None);
    // A non-pointer field is not a link even though it is non-zero.
    assert_eq!(a.link(parent).unwrap(), None);

    assert_eq!(r.u32(child).unwrap(), 0xCAFE_BABE);
    assert_eq!(r.f32(child + 4).unwrap().to_bits(), 2.5f32.to_bits());
    assert_eq!(r.s16(child + 8).unwrap(), -7);
    assert!(matches!(r.u32(0x1C), Err(Error::OutOfBounds { .. })));
    assert!(matches!(a.link(0x1C), Err(Error::OutOfBounds { .. })));
}

#[test]
fn extern_chain_is_walked_like_locate_extern() {
    let mut b = Builder::new();
    // Three slots chained head -> mid -> tail -> 0xFFFFFFFF.
    let tail = b.push_u32(u32::MAX);
    let mid = b.push_u32(tail);
    let head = b.push_u32(mid);
    // A second extern whose chain ends by running past data_size.
    let lone = b.push_u32(0x1000);
    b.extern_sym(head, "HSD_Extern");
    b.extern_sym(lone, "Other");
    b.public(0, "root");
    let a = Archive::parse(&b.build()).unwrap();

    assert_eq!(a.header().nb_extern, 2);
    assert_eq!(a.extern_name(0), Some("HSD_Extern"));
    assert_eq!(a.extern_name(1), Some("Other"));
    assert_eq!(a.extern_name(2), None);
    assert_eq!(a.extern_sites("HSD_Extern").unwrap(), vec![head, mid, tail]);
    assert_eq!(a.extern_sites("Other").unwrap(), vec![lone]);
    assert_eq!(a.extern_sites("nope").unwrap(), Vec::<u32>::new());
}

#[test]
fn extern_chain_cycle_is_an_error() {
    let mut b = Builder::new();
    let a0 = b.push_u32(4);
    b.push_u32(a0); // points back to a0
    b.extern_sym(a0, "loop");
    let a = Archive::parse(&b.build()).unwrap();
    assert_eq!(
        a.extern_sites("loop"),
        Err(Error::ExternChainCycle {
            name: "loop".into(),
            offset: a0
        })
    );
}

#[test]
fn empty_data_section_and_empty_string_table() {
    // Retail leaves data NULL when data_size == 0 and symbols NULL when the
    // tables run right up to file_size. Both must parse.
    let b = Builder::new();
    let a = Archive::parse(&b.build()).unwrap();
    assert!(a.data().is_empty());
    assert!(a.reloc_targets().is_empty());
    assert!(a.publics().is_empty());
    assert_eq!(a.public("root"), None);
}

#[test]
fn several_publics_first_match_wins() {
    let mut b = Builder::new();
    b.push_u32(0);
    b.push_u32(0);
    b.public(0, "a");
    b.public(4, "b");
    b.public(0, "b"); // duplicate name: retail returns the first
    let a = Archive::parse(&b.build()).unwrap();
    assert_eq!(a.public("a"), Some(0));
    assert_eq!(a.public("b"), Some(4));
    assert_eq!(a.publics().len(), 3);
}

#[test]
fn relocs_are_reported_sorted() {
    let mut b = Builder::new();
    for _ in 0..4 {
        b.push_u32(0);
    }
    b.relocs = vec![12, 0, 8, 4];
    let a = Archive::parse(&b.build()).unwrap();
    assert_eq!(a.reloc_targets(), &[0, 4, 8, 12]);
    for off in [0, 4, 8, 12] {
        assert!(a.is_relocated_offset(off));
    }
    assert!(!a.is_relocated_offset(2));
}

#[test]
fn truncated_header() {
    assert_eq!(
        Archive::parse(&[0u8; 8]),
        Err(Error::TruncatedHeader { available: 8 })
    );
}

#[test]
fn file_size_mismatch_is_rejected() {
    let (mut b, _, _) = two_structs();
    b.file_size_override = Some(0x9999);
    let bytes = b.build();
    assert_eq!(
        Archive::parse(&bytes),
        Err(Error::FileSizeMismatch {
            header: 0x9999,
            actual: bytes.len()
        })
    );
    // Also when the buffer is shorter than the header claims.
    let (b, _, _) = two_structs();
    let bytes = b.build();
    let short = &bytes[..bytes.len() - 1];
    assert!(matches!(
        Archive::parse(short),
        Err(Error::FileSizeMismatch { .. })
    ));
}

#[test]
fn sections_past_file_size_are_rejected() {
    let (b, _, _) = two_structs();
    let mut bytes = b.build();
    // Claim a data section larger than the whole file.
    let bad = ArchiveHeader {
        data_size: 0x10_0000,
        ..*Archive::parse(&bytes).unwrap().header()
    };
    bytes[..ArchiveHeader::SIZE].copy_from_slice(&bad.to_bytes());
    assert!(matches!(
        Archive::parse(&bytes),
        Err(Error::SectionOutOfBounds {
            section: "data",
            ..
        })
    ));

    // Claim more relocations than fit; must not overflow.
    let bad = ArchiveHeader {
        nb_reloc: u32::MAX,
        ..*Archive::parse(&b.build()).unwrap().header()
    };
    bytes[..ArchiveHeader::SIZE].copy_from_slice(&bad.to_bytes());
    assert!(matches!(
        Archive::parse(&bytes),
        Err(Error::SectionOutOfBounds {
            section: "relocation",
            ..
        })
    ));
}

#[test]
fn relocation_outside_data_is_rejected() {
    let mut b = Builder::new();
    b.push_u32(0);
    b.push_u32(0);
    b.relocs.push(6); // slot [6, 10) straddles data_size 8
    assert_eq!(
        Archive::parse(&b.build()),
        Err(Error::RelocationOutOfBounds {
            index: 0,
            offset: 6,
            data_size: 8
        })
    );
}

#[test]
fn symbol_outside_string_table_is_rejected() {
    let mut b = Builder::new();
    b.push_u32(0);
    b.public(0, "ok");
    b.publics.push((0, 0x40)); // past the 3-byte string table
    assert_eq!(
        Archive::parse(&b.build()),
        Err(Error::SymbolOutOfBounds {
            kind: SymbolKind::Public,
            index: 1,
            symbol: 0x40,
            strings_len: 3
        })
    );

    let mut b = Builder::new();
    b.push_u32(0);
    b.externs.push((0, 0)); // there is no string table at all
    assert_eq!(
        Archive::parse(&b.build()),
        Err(Error::SymbolOutOfBounds {
            kind: SymbolKind::Extern,
            index: 0,
            symbol: 0,
            strings_len: 0
        })
    );
}

#[test]
fn unterminated_symbol_is_rejected() {
    let mut b = Builder::new();
    b.push_u32(0);
    b.public(0, "root");
    b.strings.pop(); // drop the NUL
    assert_eq!(
        Archive::parse(&b.build()),
        Err(Error::UnterminatedString { offset: 0 })
    );
}

#[test]
fn errors_display_and_implement_std_error() {
    let e: Box<dyn std::error::Error> = Box::new(Error::FileSizeMismatch {
        header: 0x10,
        actual: 0x20,
    });
    let msg = e.to_string();
    assert!(msg.contains("0x10"), "{msg}");
    assert!(msg.contains("0x20"), "{msg}");
}
