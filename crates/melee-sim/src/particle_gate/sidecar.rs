//! A particle dump read once, as a stream, into a small file beside it.
//!
//! `<name>.particles.digest` holds one line per dump record: its two
//! digests, its end-of-tick seed and the display clock of the dump's own
//! Dolphin pass. It is derived data: its header names the dump files it was
//! read from (length and modification time), and a sidecar of other files,
//! or of another digest version, is rebuilt.
use super::digest::{Digests, StateDigest};
use crate::scenario::Scenario;
use anyhow::{ensure, Context, Result};
use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

/// Bump when the digests or the sidecar's layout change.
const VERSION: u32 = 2;

/// One record of a dump, reduced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DumpTick {
    pub digests: Digests,
    pub seed: u32,
    /// psFrameNum at the tick's end in the dump's own run, when its tracer
    /// recorded it (since 2026-09-26).
    pub display_clock: Option<u64>,
}

#[derive(serde::Deserialize)]
struct DumpLine {
    frame: u64,
    state: StateDigest,
}

/// The display clock of a dump's metadata line; serde skips the rest.
#[derive(serde::Deserialize)]
struct MetaLine {
    #[serde(default)]
    ps_frame: Option<u64>,
}

/// The files a sidecar was read from: name, length, modification time.
type Sources = Vec<(String, u64, u128)>;

#[derive(serde::Serialize, serde::Deserialize)]
struct Header {
    version: u32,
    sources: Sources,
    records: usize,
}

fn fingerprint(paths: &[PathBuf]) -> Sources {
    paths
        .iter()
        .filter_map(|path| {
            let found = melee_trace_io::resolve(path)?;
            let meta = std::fs::metadata(&found).ok()?;
            let modified = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?;
            let name = found.file_name()?.to_string_lossy().into_owned();
            Some((name, meta.len(), modified.as_nanos()))
        })
        .collect()
}

/// The display clock of each record of the dump's pass.
fn display_clocks(meta: &Path) -> Result<Vec<Option<u64>>> {
    let mut clocks = Vec::new();
    if !melee_trace_io::exists(meta) {
        return Ok(clocks);
    }
    for line in melee_trace_io::open(meta)?.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: MetaLine = serde_json::from_str(&line)
            .with_context(|| format!("{}: record {}", meta.display(), clocks.len()))?;
        // An older tracer recorded no display clock on any record: the
        // rest of its metadata holds nothing to read.
        if row.ps_frame.is_none() && clocks.is_empty() {
            break;
        }
        clocks.push(row.ps_frame);
    }
    Ok(clocks)
}

/// Read a dump and its metadata as streams, one line in memory at a time.
fn digest_dump(dump: &Path, meta: &Path) -> Result<Vec<DumpTick>> {
    let clocks = display_clocks(meta)?;
    let mut ticks = Vec::new();
    for line in melee_trace_io::open(dump)?.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let index = ticks.len();
        let row: DumpLine = serde_json::from_str(&line)
            .with_context(|| format!("{}: record {index}", dump.display()))?;
        ensure!(
            row.frame == index as u64,
            "{}: record {index} is frame {}",
            dump.display(),
            row.frame
        );
        ticks.push(DumpTick {
            digests: row.state.digests,
            seed: row
                .state
                .seed
                .with_context(|| format!("{}: record {index} has no seed", dump.display()))?,
            display_clock: clocks.get(index).copied().flatten(),
        });
    }
    Ok(ticks)
}

fn parse_tick(line: &str) -> Option<DumpTick> {
    let mut parts = line.split(' ');
    let ordered = u64::from_str_radix(parts.next()?, 16).ok()?;
    let unordered = u64::from_str_radix(parts.next()?, 16).ok()?;
    let seed = u32::from_str_radix(parts.next()?, 16).ok()?;
    let display_clock = match parts.next()? {
        "-" => None,
        clock => Some(clock.parse().ok()?),
    };
    Some(DumpTick {
        digests: Digests { ordered, unordered },
        seed,
        display_clock,
    })
}

fn read(path: &Path, sources: &Sources) -> Option<Vec<DumpTick>> {
    let mut lines = std::io::BufReader::new(std::fs::File::open(path).ok()?).lines();
    let header: Header = serde_json::from_str(&lines.next()?.ok()?).ok()?;
    if header.version != VERSION || header.sources != *sources {
        return None;
    }
    let mut ticks = Vec::with_capacity(header.records);
    for line in lines {
        ticks.push(parse_tick(&line.ok()?)?);
    }
    (ticks.len() == header.records).then_some(ticks)
}

/// Write the sidecar whole and move it into place, so a reader (another
/// gate of the same scenario) sees the old file or the new one.
fn write(path: &Path, sources: Sources, ticks: &[DumpTick]) -> Result<()> {
    let scratch = path.with_extension(format!(
        "digest.{}.{:?}.tmp",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&scratch)?);
    let header = Header {
        version: VERSION,
        sources,
        records: ticks.len(),
    };
    writeln!(out, "{}", serde_json::to_string(&header)?)?;
    for tick in ticks {
        let clock = tick
            .display_clock
            .map_or("-".to_string(), |clock| clock.to_string());
        let Digests { ordered, unordered } = tick.digests;
        writeln!(out, "{ordered:016x} {unordered:016x} {:08x} {clock}", tick.seed)?;
    }
    out.into_inner()?.sync_all()?;
    std::fs::rename(&scratch, path)?;
    Ok(())
}

/// The scenario's dump, reduced: from its sidecar when that was read from
/// the files now on disk, else from the dump (and the sidecar is written;
/// a folder that cannot be written only costs the next run the same read).
/// `None` without a dump.
pub(super) fn dump_ticks(scenario: &Scenario) -> Result<Option<Vec<DumpTick>>> {
    let dump = scenario.trace_path("particles.jsonl");
    if !melee_trace_io::exists(&dump) {
        return Ok(None);
    }
    let meta = scenario.trace_path("particles.jsonl.meta.jsonl");
    let sidecar = scenario.trace_path("particles.digest");
    let sources = fingerprint(&[dump.clone(), meta.clone()]);
    if let Some(ticks) = read(&sidecar, &sources) {
        return Ok(Some(ticks));
    }
    let ticks = digest_dump(&dump, &meta)?;
    // The files may have changed under the read (a recording in progress):
    // such a sidecar would name sources it was not read from.
    if fingerprint(&[dump, meta]) == sources {
        let _ = write(&sidecar, sources, &ticks);
    }
    Ok(Some(ticks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sidecar_reads_back_only_for_the_files_it_was_read_from() {
        let dir = std::env::temp_dir().join(format!("melee-particle-gate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("scene.particles.digest");
        let ticks = [
            DumpTick {
                digests: Digests {
                    ordered: u64::MAX,
                    unordered: 1,
                },
                seed: 0x8000_0001,
                display_clock: Some(254),
            },
            DumpTick {
                digests: Digests {
                    ordered: 3,
                    unordered: 4,
                },
                seed: 5,
                display_clock: None,
            },
        ];
        let sources = vec![("scene.particles.jsonl.zst".to_string(), 10, 20)];
        write(&path, sources.clone(), &ticks).unwrap();
        assert_eq!(read(&path, &sources).as_deref(), Some(&ticks[..]));
        let rerecorded = vec![("scene.particles.jsonl.zst".to_string(), 11, 20)];
        assert_eq!(read(&path, &rerecorded), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
