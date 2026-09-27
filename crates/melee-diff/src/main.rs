use anyhow::Context;
use clap::Parser;
use std::path::{Path, PathBuf};

/// Compare two canonical JSONL state traces and report the first divergence.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Trace produced by the Dolphin oracle (ground truth).
    expected: PathBuf,
    /// Trace produced by the Rust port.
    actual: PathBuf,
}

/// A trace named by its plain path; a `.zst` sibling is read transparently.
fn read(path: &Path) -> anyhow::Result<Vec<melee_diff::Record>> {
    melee_diff::read_trace(
        melee_trace_io::open(path).with_context(|| format!("open {}", path.display()))?,
    )
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let exp = read(&args.expected)?;
    let act = read(&args.actual)?;
    match melee_diff::first_divergence(&exp, &act) {
        None => {
            println!("OK: {} records match", exp.len());
            Ok(())
        }
        Some(d) => {
            println!("{d}");
            std::process::exit(1)
        }
    }
}
