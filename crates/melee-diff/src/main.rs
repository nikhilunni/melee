use anyhow::Context;
use clap::Parser;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

/// Compare two canonical JSONL state traces and report the first divergence.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Trace produced by the Dolphin oracle (ground truth).
    expected: PathBuf,
    /// Trace produced by the Rust port.
    actual: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let exp = melee_diff::read_trace(BufReader::new(
        File::open(&args.expected).with_context(|| format!("open {}", args.expected.display()))?,
    ))?;
    let act = melee_diff::read_trace(BufReader::new(
        File::open(&args.actual).with_context(|| format!("open {}", args.actual.display()))?,
    ))?;
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
