//! Headless simulator. Runs a scenario and writes a canonical trace.
//!
//! Usage (target shape, not all implemented yet):
//!   melee-sim --scenario harness/scenarios/idle_fd_fox.toml --assets <disc files dir> --out actual.jsonl
//!   melee-diff expected.jsonl actual.jsonl
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Scenario file (TOML) describing seed, stage, fighters, and the input script.
    #[arg(long)]
    scenario: PathBuf,
    /// Directory containing the game's `files/` tree (Pl*.dat, Gr*.dat, ...) from your own disc.
    #[arg(long)]
    assets: PathBuf,
    /// Output trace path (JSONL).
    #[arg(long)]
    out: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    anyhow::bail!(
        "melee-sim is not implemented yet. scenario={} assets={} out={}\n\
         Milestone 1 (math + rng) is done; milestone 2 (archive + animation) is next. See docs/PLAN.md.",
        args.scenario.display(),
        args.assets.display(),
        args.out.display()
    )
}
