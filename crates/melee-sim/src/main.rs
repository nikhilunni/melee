//! Headless simulator. Runs a scenario and writes a canonical trace.
//!
//! Usage (target shape, not all implemented yet):
//!   melee-sim bones --fighter fox --anim Wait1 --frame 0 --frames 2
//!   melee-sim --scenario harness/scenarios/idle_fd_fox.toml --assets <disc files dir> --out actual.jsonl
//!   melee-diff expected.jsonl actual.jsonl
use clap::{Parser, Subcommand};
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about,
    subcommand_negates_reqs = true,
    args_conflicts_with_subcommands = true
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    /// Scenario file (TOML) describing seed, stage, fighters, and the input script.
    #[arg(long, required = true)]
    scenario: Option<PathBuf>,
    /// Directory containing the game's `files/` tree (Pl*.dat, Gr*.dat, ...) from your own disc.
    #[arg(long, required = true)]
    assets: Option<PathBuf>,
    /// Output trace path (JSONL).
    #[arg(long, required = true)]
    out: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Emit Fox Wait1 bone matrices and SRT with an identity world transform.
    Bones {
        #[arg(long, value_parser = ["fox"])]
        fighter: String,
        #[arg(long, value_parser = ["Wait1"])]
        anim: String,
        /// Animation time of the first sample (trace ordinals start at zero).
        #[arg(long)]
        frame: f32,
        /// Number of samples, advancing normally at rate 1 after the request.
        #[arg(long, default_value_t = 1)]
        frames: u64,
        /// Extracted disc files; defaults to harness/roms/files in this repo.
        #[arg(long, default_value_os_t = melee_sim::bones::default_assets())]
        assets: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(Command::Bones {
        frame,
        frames,
        assets,
        ..
    }) = args.command
    {
        return melee_sim::bones::write_fox_wait1_bones(
            &assets,
            frame,
            frames,
            io::BufWriter::new(io::stdout().lock()),
        );
    }
    anyhow::bail!(
        "scenario simulation is not implemented yet. scenario={} assets={} out={}\n\
         The bones subcommand is available for Milestone 2. See docs/M2_GATE.md.",
        args.scenario.expect("required by clap").display(),
        args.assets.expect("required by clap").display(),
        args.out.expect("required by clap").display()
    )
}
