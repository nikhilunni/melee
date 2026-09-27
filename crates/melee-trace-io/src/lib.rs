//! Recorded oracle traces on disk, plain or zstd-compressed.
//!
//! The harness records `<name>.jsonl` and then replaces large outputs with
//! `<name>.jsonl.zst` (`harness/trace_io.py`). Readers always name the plain
//! path; [`open`] reads whichever of the two exists, preferring the plain file,
//! and yields the same bytes either way.
//!
//! Tooling and tests only: gameplay crates never depend on this crate.
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

/// Extension appended to a compressed trace (`x.jsonl` -> `x.jsonl.zst`).
pub const COMPRESSED_EXTENSION: &str = "zst";

/// The compressed sibling of a plain trace path: `<path>.zst`.
pub fn compressed_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(COMPRESSED_EXTENSION);
    PathBuf::from(name)
}

fn is_compressed(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == COMPRESSED_EXTENSION)
}

/// The file that holds the trace named by `path`: `path` itself when it is a
/// file, else its `.zst` sibling when that is a file, else `None`.
pub fn resolve(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let compressed = compressed_path(path);
    compressed.is_file().then_some(compressed)
}

/// Whether the trace named by `path` exists, plain or compressed.
pub fn exists(path: &Path) -> bool {
    resolve(path).is_some()
}

/// A buffered reader over a trace's plain bytes, whichever form is on disk.
pub enum TraceReader {
    Plain(BufReader<File>),
    Compressed(BufReader<zstd::stream::read::Decoder<'static, BufReader<File>>>),
}

impl Read for TraceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            TraceReader::Plain(r) => r.read(buf),
            TraceReader::Compressed(r) => r.read(buf),
        }
    }
}

impl BufRead for TraceReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            TraceReader::Plain(r) => r.fill_buf(),
            TraceReader::Compressed(r) => r.fill_buf(),
        }
    }
    fn consume(&mut self, amount: usize) {
        match self {
            TraceReader::Plain(r) => r.consume(amount),
            TraceReader::Compressed(r) => r.consume(amount),
        }
    }
}

/// Open the trace named by `path` (see [`resolve`]). A path that already ends
/// in `.zst` is decompressed directly.
pub fn open(path: &Path) -> io::Result<TraceReader> {
    let Some(found) = resolve(path) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "trace not found: {} (nor {})",
                path.display(),
                compressed_path(path).display()
            ),
        ));
    };
    let file = File::open(&found)
        .map_err(|e| io::Error::new(e.kind(), format!("opening {}: {e}", found.display())))?;
    if is_compressed(&found) {
        let decoder = zstd::stream::read::Decoder::new(file)?;
        Ok(TraceReader::Compressed(BufReader::new(decoder)))
    } else {
        Ok(TraceReader::Plain(BufReader::new(file)))
    }
}

/// The whole trace as bytes.
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    open(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The whole trace as UTF-8 text.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    let mut text = String::new();
    open(path)?.read_to_string(&mut text)?;
    Ok(text)
}

/// The trace's lines, like `BufRead::lines` on the plain file.
pub fn lines(path: &Path) -> io::Result<io::Lines<TraceReader>> {
    Ok(open(path)?.lines())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SAMPLE: &str = "{\"frame\":0}\n{\"frame\":1}\n";

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("melee-trace-io-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_compressed(path: &Path, text: &str) {
        fs::write(
            compressed_path(path),
            zstd::stream::encode_all(text.as_bytes(), 3).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn compressed_sibling_appends_zst() {
        assert_eq!(
            compressed_path(Path::new("t/a.tick.expected.jsonl")),
            PathBuf::from("t/a.tick.expected.jsonl.zst")
        );
    }

    #[test]
    fn plain_and_compressed_read_identically() {
        let dir = scratch("round-trip");
        let plain = dir.join("plain.jsonl");
        let packed = dir.join("packed.jsonl");
        fs::write(&plain, SAMPLE).unwrap();
        write_compressed(&packed, SAMPLE);
        assert!(!packed.exists() && exists(&packed));
        assert_eq!(read_to_string(&plain).unwrap(), SAMPLE);
        assert_eq!(read_to_string(&packed).unwrap(), SAMPLE);
        assert_eq!(read(&packed).unwrap(), SAMPLE.as_bytes());
        let rows: Vec<String> = lines(&packed).unwrap().map(Result::unwrap).collect();
        assert_eq!(rows, ["{\"frame\":0}", "{\"frame\":1}"]);
        // An explicit .zst path decodes too.
        assert_eq!(read_to_string(&compressed_path(&packed)).unwrap(), SAMPLE);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn plain_file_wins_over_compressed_sibling() {
        let dir = scratch("preference");
        let path = dir.join("both.jsonl");
        fs::write(&path, "plain\n").unwrap();
        write_compressed(&path, "compressed\n");
        assert_eq!(resolve(&path), Some(path.clone()));
        assert_eq!(read_to_string(&path).unwrap(), "plain\n");
        fs::remove_file(&path).unwrap();
        assert_eq!(resolve(&path), Some(compressed_path(&path)));
        assert_eq!(read_to_string(&path).unwrap(), "compressed\n");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_trace_names_both_forms() {
        let dir = scratch("missing");
        let path = dir.join("absent.jsonl");
        assert!(!exists(&path));
        let error = open(&path).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("absent.jsonl.zst"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn concatenated_frames_decode_as_one_stream() {
        let dir = scratch("frames");
        let path = dir.join("multi.jsonl");
        let mut bytes = zstd::stream::encode_all(&b"a\n"[..], 1).unwrap();
        bytes.extend(zstd::stream::encode_all(&b"b\n"[..], 1).unwrap());
        fs::write(compressed_path(&path), bytes).unwrap();
        assert_eq!(read_to_string(&path).unwrap(), "a\nb\n");
        fs::remove_dir_all(dir).unwrap();
    }
}
