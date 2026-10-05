//! GameCube disc images: the boot header and the file table (FST).
//!
//! Sans IO: the host reads byte ranges from wherever the image lives (a
//! native `File`, a browser `Blob.slice()`) and hands them in. Reading a
//! disc is three steps:
//!
//! 1. read [`HEADER_LEN`] bytes at offset 0 and [`DiscHeader::parse`] them;
//! 2. read [`DiscHeader::fst_range`] and [`Fst::parse`] it;
//! 3. read each [`FileEntry::range`] the caller needs.
//!
//! Layout (all fields big-endian; measured numbers in `docs/DISC.md`):
//!
//! | Offset  | Size    | Field                                   |
//! |---------|---------|-----------------------------------------|
//! | `0x000` | 6       | game id: 4-char game code, 2-char maker |
//! | `0x006` | 1       | disc number                             |
//! | `0x007` | 1       | revision                                |
//! | `0x018` | u32     | Wii magic `0x5D1C9EA3`                  |
//! | `0x01C` | u32     | GameCube magic `0xC2339F3D`             |
//! | `0x020` | `0x3E0` | title, NUL padded                       |
//! | `0x420` | u32     | `main.dol` offset                       |
//! | `0x424` | u32     | FST offset                              |
//! | `0x428` | u32     | FST size                                |
//!
//! FST: 12-byte entries, then a NUL-separated string table. Entry 0 is the
//! root directory and its third word is the entry count. A file entry is
//! `(flags 0, name offset, disc offset, size)`; a directory entry is
//! `(flags 1, name offset, parent index, next-sibling index)`, and its
//! descendants are the entries before that next-sibling index.
use std::{collections::BTreeMap, fmt, ops::Range};

/// Bytes of the boot header (`boot.bin`) a host reads first.
pub const HEADER_LEN: u64 = 0x440;

const GAMECUBE_MAGIC: u32 = 0xC233_9F3D;
const WII_MAGIC: u32 = 0x5D1C_9EA3;
const GAME_ID: Range<usize> = 0..6;
const DISC_NUMBER: usize = 0x06;
const REVISION: usize = 0x07;
const WII_MAGIC_AT: usize = 0x18;
const GAMECUBE_MAGIC_AT: usize = 0x1C;
const TITLE: Range<usize> = 0x20..0x400;
const DOL_OFFSET_AT: usize = 0x420;
const FST_OFFSET_AT: usize = 0x424;
const FST_SIZE_AT: usize = 0x428;
const FST_ENTRY_LEN: usize = 12;
/// Melee's FST is about 30 KB; anything past this is not a real table.
const FST_MAX_LEN: u32 = 16 << 20;

/// Super Smash Bros. Melee, NTSC-U, the only image this port runs.
pub const MELEE_GAME_ID: &str = "GALE01";
/// Revision byte of NTSC-U 1.02.
pub const MELEE_REVISION: u8 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscError {
    /// Fewer bytes than the structure needs.
    Truncated {
        what: &'static str,
        needed: u64,
        found: u64,
    },
    /// A compressed or container format, not a plain disc image.
    Container(&'static str),
    /// A Wii disc.
    Wii { game_id: String, title: String },
    /// No GameCube magic: not a disc image at all.
    NotGameCube,
    /// A GameCube disc of another game.
    WrongGame { game_id: String, title: String },
    /// Melee, but not the NTSC-U release.
    WrongRegion {
        game_id: String,
        region: &'static str,
    },
    /// NTSC-U Melee, but not revision 1.02.
    WrongRevision { revision: u8 },
    /// The file table is malformed.
    BadFst(String),
    /// A file runs past the end of the image (a truncated download).
    OutsideImage {
        name: String,
        end: u64,
        image_len: u64,
    },
}

impl fmt::Display for DiscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated {
                what,
                needed,
                found,
            } => write!(
                f,
                "the file is too short for a disc image ({what}: needed {needed} bytes, found {found})"
            ),
            Self::Container(kind) => write!(
                f,
                "this is a {kind}, not a plain disc image. Use an uncompressed .iso (or .gcm) of \
                 Super Smash Bros. Melee NTSC-U 1.02; Dolphin can convert it \
                 (right-click the game, Convert File, format ISO)"
            ),
            Self::Wii { game_id, title } => write!(
                f,
                "this is a Wii disc ({game_id}, \"{title}\"), not Super Smash Bros. Melee"
            ),
            Self::NotGameCube => write!(
                f,
                "this is not a GameCube disc image (no GameCube disc magic at 0x1C)"
            ),
            Self::WrongGame { game_id, title } => write!(
                f,
                "this disc is {game_id} \"{title}\", not Super Smash Bros. Melee (GALE01)"
            ),
            Self::WrongRegion { game_id, region } => write!(
                f,
                "this is the {region} release of Melee ({game_id}); only NTSC-U 1.02 (GALE01) is supported"
            ),
            Self::WrongRevision { revision } => write!(
                f,
                "this is Melee NTSC-U {} (revision {revision}); only 1.02 (revision 2) is supported",
                version_name(*revision)
            ),
            Self::BadFst(message) => write!(f, "the disc's file table is malformed: {message}"),
            Self::OutsideImage {
                name,
                end,
                image_len,
            } => write!(
                f,
                "{name} ends at byte {end} but the image has only {image_len} bytes; the file is truncated"
            ),
        }
    }
}
impl std::error::Error for DiscError {}

fn version_name(revision: u8) -> String {
    format!("1.{revision:02}")
}

/// Recognise compressed and container formats from their first bytes, so a
/// user who drops one gets told what it is.
pub fn container_kind(first_bytes: &[u8]) -> Option<&'static str> {
    const SIGNATURES: [(&[u8], &str); 11] = [
        (b"PK\x03\x04", ".zip archive"),
        (b"7z\xBC\xAF\x27\x1C", ".7z archive"),
        (b"Rar!", ".rar archive"),
        (b"\x1F\x8B", ".gz archive"),
        (b"CISO", "CISO (.ciso) compressed image"),
        (b"RVZ\x01", "Dolphin RVZ (.rvz) compressed image"),
        (b"WIA\x01", "WIA (.wia) compressed image"),
        (b"\x01\xC0\x0B\xB1", "Dolphin GCZ (.gcz) compressed image"),
        (b"WBFS", "WBFS (.wbfs) image"),
        (b"\xAE\x0F\x38\xA2", "TGC (.tgc) image"),
        (b"NKIT", "NKit image"),
    ];
    SIGNATURES
        .iter()
        .find(|(magic, _)| first_bytes.starts_with(magic))
        .map(|&(_, kind)| kind)
}

fn be_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
}

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

/// The fields of the boot header this crate uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscHeader {
    /// Six characters, e.g. `GALE01`: game code (`GAL` + region letter) and maker.
    pub game_id: String,
    pub disc_number: u8,
    pub revision: u8,
    pub title: String,
    pub dol_offset: u32,
    pub fst_offset: u32,
    pub fst_size: u32,
}

impl DiscHeader {
    /// Parse the first [`HEADER_LEN`] bytes of an image. Only checks that
    /// this is a GameCube disc; [`DiscHeader::require_melee`] checks which.
    pub fn parse(bytes: &[u8]) -> Result<Self, DiscError> {
        if let Some(kind) = container_kind(bytes) {
            return Err(DiscError::Container(kind));
        }
        if (bytes.len() as u64) < HEADER_LEN {
            return Err(DiscError::Truncated {
                what: "disc header",
                needed: HEADER_LEN,
                found: bytes.len() as u64,
            });
        }
        let game_id = text(&bytes[GAME_ID]);
        let title = text(&bytes[TITLE]);
        if be_u32(bytes, WII_MAGIC_AT) == WII_MAGIC {
            return Err(DiscError::Wii { game_id, title });
        }
        if be_u32(bytes, GAMECUBE_MAGIC_AT) != GAMECUBE_MAGIC {
            return Err(DiscError::NotGameCube);
        }
        Ok(Self {
            game_id,
            disc_number: bytes[DISC_NUMBER],
            revision: bytes[REVISION],
            title,
            dol_offset: be_u32(bytes, DOL_OFFSET_AT),
            fst_offset: be_u32(bytes, FST_OFFSET_AT),
            fst_size: be_u32(bytes, FST_SIZE_AT),
        })
    }

    /// Accept only Super Smash Bros. Melee NTSC-U 1.02, naming what was found otherwise.
    pub fn require_melee(&self) -> Result<(), DiscError> {
        let code = self.game_id.as_bytes();
        if !self.game_id.starts_with("GAL") || code.len() != 6 {
            return Err(DiscError::WrongGame {
                game_id: self.game_id.clone(),
                title: self.title.clone(),
            });
        }
        if self.game_id != MELEE_GAME_ID {
            let region = match code[3] {
                b'J' => "Japanese",
                b'P' => "PAL",
                b'E' => "NTSC-U (unofficial maker code)",
                _ => "unknown-region",
            };
            return Err(DiscError::WrongRegion {
                game_id: self.game_id.clone(),
                region,
            });
        }
        if self.revision != MELEE_REVISION {
            return Err(DiscError::WrongRevision {
                revision: self.revision,
            });
        }
        Ok(())
    }

    /// The byte range of the file table, to read next.
    pub fn fst_range(&self) -> Result<Range<u64>, DiscError> {
        if self.fst_size < FST_ENTRY_LEN as u32 || self.fst_size > FST_MAX_LEN {
            return Err(DiscError::BadFst(format!(
                "declared size {} bytes",
                self.fst_size
            )));
        }
        let start = u64::from(self.fst_offset);
        Ok(start..start + u64::from(self.fst_size))
    }
}

/// One file's place on the disc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub offset: u32,
    pub len: u32,
}
impl FileEntry {
    pub fn range(self) -> Range<u64> {
        let start = u64::from(self.offset);
        start..start + u64::from(self.len)
    }
}

/// The disc's file table: `/`-separated path (no leading slash) to entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fst {
    files: BTreeMap<String, FileEntry>,
}

impl Fst {
    pub fn parse(bytes: &[u8]) -> Result<Self, DiscError> {
        let bad = |message: String| DiscError::BadFst(message);
        if bytes.len() < FST_ENTRY_LEN {
            return Err(bad("shorter than one entry".into()));
        }
        if bytes[0] != 1 {
            return Err(bad("entry 0 is not the root directory".into()));
        }
        let count = be_u32(bytes, 8) as usize;
        let strings = count
            .checked_mul(FST_ENTRY_LEN)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| {
                bad(format!(
                    "{count} entries do not fit in {} bytes",
                    bytes.len()
                ))
            })?;
        let name = |offset: usize| -> Result<&str, DiscError> {
            let start = strings + offset;
            let rest = bytes
                .get(start..)
                .ok_or_else(|| bad(format!("name offset {offset:#x} is past the string table")))?;
            let end = rest
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| bad(format!("name at {offset:#x} is not terminated")))?;
            let name = std::str::from_utf8(&rest[..end])
                .map_err(|_| bad(format!("name at {offset:#x} is not ASCII")))?;
            if name.is_empty() || name.contains('/') || name == "." || name == ".." {
                return Err(bad(format!("unsafe entry name {name:?}")));
            }
            Ok(name)
        };
        let mut files = BTreeMap::new();
        // Open directories: (one past the last descendant, path prefix).
        let mut open: Vec<(usize, String)> = vec![(count, String::new())];
        for index in 1..count {
            while open.last().is_some_and(|&(end, _)| index >= end) {
                open.pop();
            }
            let (parent_end, prefix) = open
                .last()
                .cloned()
                .ok_or_else(|| bad(format!("entry {index} is outside the root")))?;
            let at = index * FST_ENTRY_LEN;
            let word = be_u32(bytes, at);
            let path = format!("{prefix}{}", name((word & 0x00FF_FFFF) as usize)?);
            let (second, third) = (be_u32(bytes, at + 4), be_u32(bytes, at + 8));
            if word >> 24 == 1 {
                let end = third as usize;
                if end <= index || end > parent_end {
                    return Err(bad(format!(
                        "directory {path} at entry {index} ends at {end}, outside {}..={parent_end}",
                        index + 1
                    )));
                }
                open.push((end, format!("{path}/")));
            } else {
                files.insert(
                    path,
                    FileEntry {
                        offset: second,
                        len: third,
                    },
                );
            }
        }
        Ok(Self { files })
    }

    /// The entry for `path` (`PlCo.dat`, `audio/us/1padv.ssm`).
    pub fn file(&self, path: &str) -> Option<FileEntry> {
        self.files.get(path).copied()
    }

    pub fn files(&self) -> impl ExactSizeIterator<Item = (&str, FileEntry)> {
        self.files
            .iter()
            .map(|(name, &entry)| (name.as_str(), entry))
    }

    /// Fail if any file runs past the end of an image of `image_len` bytes.
    pub fn check_within(&self, image_len: u64) -> Result<(), DiscError> {
        match self
            .files()
            .find(|(_, entry)| entry.range().end > image_len)
        {
            Some((name, entry)) => Err(DiscError::OutsideImage {
                name: name.to_owned(),
                end: entry.range().end,
                image_len,
            }),
            None => Ok(()),
        }
    }
}

/// A validated Melee disc: header and file table.
#[derive(Clone, Debug)]
pub struct Disc {
    pub header: DiscHeader,
    pub fst: Fst,
    pub image_len: u64,
}

impl Disc {
    /// The whole open sequence over a synchronous reader of `(offset, len)`.
    /// Async hosts run the same three steps themselves.
    pub fn open<E>(
        image_len: u64,
        mut read: impl FnMut(Range<u64>) -> Result<Vec<u8>, E>,
    ) -> Result<Self, OpenError<E>> {
        let header_bytes = read(0..HEADER_LEN.min(image_len)).map_err(OpenError::Io)?;
        let header = DiscHeader::parse(&header_bytes)?;
        header.require_melee()?;
        let fst_bytes = read(header.fst_range()?).map_err(OpenError::Io)?;
        Ok(Self::from_parts(header, &fst_bytes, image_len)?)
    }

    /// Finish opening from a parsed header and the FST bytes.
    pub fn from_parts(header: DiscHeader, fst: &[u8], image_len: u64) -> Result<Self, DiscError> {
        header.require_melee()?;
        let range = header.fst_range()?;
        if range.end > image_len {
            return Err(DiscError::Truncated {
                what: "file table",
                needed: range.end,
                found: image_len,
            });
        }
        let fst = Fst::parse(fst)?;
        fst.check_within(image_len)?;
        Ok(Self {
            header,
            fst,
            image_len,
        })
    }
}

/// Failure of [`Disc::open`]: the host's read, or the disc itself.
#[derive(Debug)]
pub enum OpenError<E> {
    Io(E),
    Disc(DiscError),
}
impl<E> From<DiscError> for OpenError<E> {
    fn from(error: DiscError) -> Self {
        Self::Disc(error)
    }
}
impl<E: fmt::Display> fmt::Display for OpenError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not read the disc image: {error}"),
            Self::Disc(error) => error.fmt(f),
        }
    }
}
impl<E: fmt::Debug + fmt::Display> std::error::Error for OpenError<E> {}

/// Build small synthetic images for tests (this crate's and its users').
pub mod synthetic {
    use super::*;

    pub(crate) enum Node {
        File(String, Vec<u8>),
        Dir(String, Vec<Node>),
    }

    /// A GameCube image builder: header fields and a file tree.
    pub struct ImageBuilder {
        pub game_id: [u8; 6],
        pub revision: u8,
        pub title: String,
        pub(crate) root: Vec<Node>,
    }

    impl Default for ImageBuilder {
        fn default() -> Self {
            Self::melee()
        }
    }

    impl ImageBuilder {
        /// Header of NTSC-U 1.02 with no files.
        pub fn melee() -> Self {
            Self {
                game_id: *b"GALE01",
                revision: MELEE_REVISION,
                title: "Super Smash Bros Melee".into(),
                root: Vec::new(),
            }
        }
        /// Add a file; `path` may contain directories (`audio/us/a.ssm`).
        pub fn file(mut self, path: &str, data: impl Into<Vec<u8>>) -> Self {
            let mut parts: Vec<&str> = path.split('/').collect();
            let leaf = parts.pop().expect("non-empty path").to_owned();
            let mut level = &mut self.root;
            for part in parts {
                let position = level
                    .iter()
                    .position(|n| matches!(n, Node::Dir(name, _) if name == part));
                let index = position.unwrap_or_else(|| {
                    level.push(Node::Dir(part.to_owned(), Vec::new()));
                    level.len() - 1
                });
                level = match &mut level[index] {
                    Node::Dir(_, children) => children,
                    Node::File(..) => unreachable!("matched a directory"),
                };
            }
            level.push(Node::File(leaf, data.into()));
            self
        }

        pub fn build(&self) -> Vec<u8> {
            let mut entries: Vec<[u32; 3]> = vec![[1 << 24, 0, 0]];
            let mut strings = vec![0u8];
            let mut blobs: Vec<(usize, &[u8])> = Vec::new();
            fn walk<'a>(
                nodes: &'a [Node],
                parent: usize,
                entries: &mut Vec<[u32; 3]>,
                strings: &mut Vec<u8>,
                blobs: &mut Vec<(usize, &'a [u8])>,
            ) {
                for node in nodes {
                    let name_offset = strings.len() as u32;
                    let name = match node {
                        Node::File(name, _) | Node::Dir(name, _) => name,
                    };
                    strings.extend_from_slice(name.as_bytes());
                    strings.push(0);
                    let index = entries.len();
                    match node {
                        Node::File(_, data) => {
                            entries.push([name_offset, 0, data.len() as u32]);
                            blobs.push((index, data));
                        }
                        Node::Dir(_, children) => {
                            entries.push([1 << 24 | name_offset, parent as u32, 0]);
                            walk(children, index, entries, strings, blobs);
                            entries[index][2] = entries.len() as u32;
                        }
                    }
                }
            }
            walk(&self.root, 0, &mut entries, &mut strings, &mut blobs);
            entries[0][2] = entries.len() as u32;
            let fst_offset = 0x2440u32;
            let fst_len = (entries.len() * FST_ENTRY_LEN + strings.len()) as u32;
            let mut cursor = (fst_offset + fst_len).next_multiple_of(32);
            for &(index, data) in &blobs {
                entries[index][1] = cursor;
                cursor = (cursor + data.len() as u32).next_multiple_of(32);
            }
            let mut image = vec![0u8; cursor as usize];
            image[GAME_ID].copy_from_slice(&self.game_id);
            image[REVISION] = self.revision;
            image[GAMECUBE_MAGIC_AT..GAMECUBE_MAGIC_AT + 4]
                .copy_from_slice(&GAMECUBE_MAGIC.to_be_bytes());
            image[TITLE.start..TITLE.start + self.title.len()]
                .copy_from_slice(self.title.as_bytes());
            image[FST_OFFSET_AT..FST_OFFSET_AT + 4].copy_from_slice(&fst_offset.to_be_bytes());
            image[FST_SIZE_AT..FST_SIZE_AT + 4].copy_from_slice(&fst_len.to_be_bytes());
            let mut at = fst_offset as usize;
            for words in &entries {
                for word in words {
                    image[at..at + 4].copy_from_slice(&word.to_be_bytes());
                    at += 4;
                }
            }
            image[at..at + strings.len()].copy_from_slice(&strings);
            for &(index, data) in &blobs {
                let start = entries[index][1] as usize;
                image[start..start + data.len()].copy_from_slice(data);
            }
            image
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{synthetic::ImageBuilder, *};

    fn open(image: &[u8]) -> Result<Disc, DiscError> {
        Disc::open(image.len() as u64, |range| {
            let end = (range.end as usize).min(image.len());
            Ok::<_, ()>(image[(range.start as usize).min(end)..end].to_vec())
        })
        .map_err(|error| match error {
            OpenError::Disc(error) => error,
            OpenError::Io(()) => unreachable!(),
        })
    }
    fn read(image: &[u8], entry: FileEntry) -> &[u8] {
        let range = entry.range();
        &image[range.start as usize..range.end as usize]
    }

    #[test]
    fn files_in_root_and_nested_directories_resolve_to_their_bytes() {
        let image = ImageBuilder::melee()
            .file("PlCo.dat", b"common".to_vec())
            .file("audio/us/1padv.ssm", b"nested".to_vec())
            .file("audio/main.hps", b"music".to_vec())
            .file("GrNLa.dat", vec![7; 100])
            .build();
        let disc = open(&image).unwrap();
        assert_eq!(disc.header.game_id, "GALE01");
        assert_eq!(disc.header.title, "Super Smash Bros Melee");
        assert_eq!(disc.fst.files().len(), 4);
        for (name, bytes) in [
            ("PlCo.dat", &b"common"[..]),
            ("audio/us/1padv.ssm", b"nested"),
            ("audio/main.hps", b"music"),
            ("GrNLa.dat", &[7; 100]),
        ] {
            assert_eq!(read(&image, disc.fst.file(name).unwrap()), bytes, "{name}");
        }
        assert_eq!(disc.fst.file("audio"), None);
        assert_eq!(disc.fst.file("1padv.ssm"), None);
    }

    #[test]
    fn other_releases_and_formats_are_named_in_the_error() {
        let cases: [(Vec<u8>, &str); 7] = [
            (
                ImageBuilder {
                    revision: 0,
                    ..ImageBuilder::melee()
                }
                .build(),
                "NTSC-U 1.00",
            ),
            (
                ImageBuilder {
                    revision: 1,
                    ..ImageBuilder::melee()
                }
                .build(),
                "NTSC-U 1.01",
            ),
            (
                ImageBuilder {
                    game_id: *b"GALP01",
                    ..ImageBuilder::melee()
                }
                .build(),
                "PAL release",
            ),
            (
                ImageBuilder {
                    game_id: *b"GALJ01",
                    ..ImageBuilder::melee()
                }
                .build(),
                "Japanese release",
            ),
            (
                ImageBuilder {
                    game_id: *b"GM4E01",
                    title: "Mario Kart Double Dash!!".into(),
                    ..ImageBuilder::melee()
                }
                .build(),
                "GM4E01 \"Mario Kart Double Dash!!\", not Super Smash Bros. Melee",
            ),
            (b"PK\x03\x04rest of a zip".to_vec(), ".zip archive"),
            (
                b"RVZ\x01".iter().copied().chain([0; 0x500]).collect(),
                "RVZ",
            ),
        ];
        for (image, expected) in cases {
            let message = open(&image).unwrap_err().to_string();
            assert!(message.contains(expected), "{message:?} lacks {expected:?}");
        }
        assert_eq!(open(&[0; 0x500]).unwrap_err(), DiscError::NotGameCube);
        assert!(matches!(
            open(&[0; 10]).unwrap_err(),
            DiscError::Truncated { .. }
        ));
    }

    #[test]
    fn a_truncated_image_is_rejected_before_any_file_is_read() {
        let image = ImageBuilder::melee()
            .file("PlCo.dat", vec![1; 4096])
            .build();
        let short = &image[..image.len() - 100];
        assert!(matches!(
            open(short).unwrap_err(),
            DiscError::OutsideImage { ref name, .. } if name == "PlCo.dat"
        ));
    }

    #[test]
    fn a_directory_escaping_its_parent_is_malformed() {
        let image = ImageBuilder::melee().file("a/b.dat", b"x".to_vec()).build();
        let header = DiscHeader::parse(&image).unwrap();
        let mut fst = image[header.fst_range().unwrap().start as usize..]
            [..header.fst_size as usize]
            .to_vec();
        fst[12 + 8..12 + 12].copy_from_slice(&99u32.to_be_bytes());
        assert!(matches!(Fst::parse(&fst), Err(DiscError::BadFst(_))));
    }
}
