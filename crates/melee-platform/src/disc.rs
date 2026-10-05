//! The opened disc and the files fetched from it. Hosts do the reading
//! (a native file, `Blob.slice()` on the web); this module says which bytes
//! to read and keeps them across matches.
use gc_disc::{Disc, DiscHeader};
use melee_lib::{FileSource, GameAssets, MatchConfig};
use std::{collections::BTreeMap, ops::Range};

/// One disc file a match still needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRequest {
    pub name: &'static str,
    pub range: Range<u64>,
}
impl FileRequest {
    pub fn len(&self) -> u64 {
        self.range.end - self.range.start
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct DiscFiles {
    disc: Disc,
    cache: BTreeMap<String, Vec<u8>>,
    #[cfg(not(target_arch = "wasm32"))]
    reader: Option<std::fs::File>,
}

impl DiscFiles {
    /// Validate a disc from its header ([`gc_disc::HEADER_LEN`] bytes at 0),
    /// its file table (the header's FST range) and the image length.
    pub fn from_parts(header: &[u8], fst: &[u8], image_len: u64) -> Result<Self, String> {
        let header = DiscHeader::parse(header).map_err(|e| e.to_string())?;
        let disc = Disc::from_parts(header, fst, image_len).map_err(|e| e.to_string())?;
        Ok(Self {
            disc,
            cache: BTreeMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            reader: None,
        })
    }

    /// Open an image file; later reads seek in it.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_path(path: &std::path::Path) -> Result<Self, String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        let disc = Disc::open(len, |range| {
            let mut bytes = Vec::new();
            file.seek(SeekFrom::Start(range.start))?;
            // A header read may come up short on a tiny file; parsing reports it.
            (&mut file)
                .take(range.end - range.start)
                .read_to_end(&mut bytes)?;
            Ok::<_, std::io::Error>(bytes)
        })
        .map_err(|e| e.to_string())?;
        Ok(Self {
            disc,
            cache: BTreeMap::new(),
            reader: Some(file),
        })
    }

    pub fn header(&self) -> &DiscHeader {
        &self.disc.header
    }

    /// The files `config` needs that are not fetched yet, in load order.
    pub fn requests(&self, config: &MatchConfig) -> Result<Vec<FileRequest>, String> {
        let names = GameAssets::files(config).map_err(|e| e.to_string())?;
        names
            .into_iter()
            .filter(|name| !self.cache.contains_key(*name))
            .map(|name| {
                let entry = self
                    .disc
                    .fst
                    .file(name)
                    .ok_or_else(|| format!("{name} is not on this disc"))?;
                Ok(FileRequest {
                    name,
                    range: entry.range(),
                })
            })
            .collect()
    }

    /// Keep a fetched file. Its length must match the file table.
    pub fn insert(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), String> {
        let entry = self
            .disc
            .fst
            .file(name)
            .ok_or_else(|| format!("{name} is not on this disc"))?;
        if bytes.len() as u64 != u64::from(entry.len) {
            return Err(format!(
                "{name}: read {} bytes, the disc says {}",
                bytes.len(),
                entry.len
            ));
        }
        self.cache.insert(name.to_owned(), bytes);
        Ok(())
    }

    /// Read one request from the image file opened by [`DiscFiles::open_path`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_native(&mut self, request: &FileRequest) -> Result<Vec<u8>, String> {
        use std::io::{Read, Seek, SeekFrom};
        let file = self
            .reader
            .as_mut()
            .ok_or("this disc was opened without a file to read")?;
        let mut bytes = vec![0; request.len() as usize];
        file.seek(SeekFrom::Start(request.range.start))
            .and_then(|_| file.read_exact(&mut bytes))
            .map_err(|e| format!("{}: {e}", request.name))?;
        Ok(bytes)
    }

    pub fn cached_files(&self) -> usize {
        self.cache.len()
    }
    pub fn cached_bytes(&self) -> u64 {
        self.cache.values().map(|b| b.len() as u64).sum()
    }
}

/// Matches load from fetched files only.
impl FileSource for DiscFiles {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        FileSource::read(&self.cache, name)
    }
}
