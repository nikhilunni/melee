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
        /// Fighter position written onto the root, as `x,y,z` (fp->cur_pos).
        #[arg(long, value_parser = parse_vec3)]
        pos: Option<hsd_types::Vec3>,
        /// Facing direction, +1 or -1, applied as a root Y rotation.
        #[arg(long)]
        facing: Option<f32>,
        /// Uniform model scale on the root (Fox: 0.96).
        #[arg(long)]
        model_scale: Option<f32>,
        /// Extra per-bone uniform scale, `INDEX=SCALE`; repeatable.
        #[arg(long = "bone-scale", value_parser = parse_bone_scale)]
        bone_scales: Vec<(usize, f32)>,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(Command::Bones {
        frame,
        frames,
        assets,
        pos,
        facing,
        model_scale,
        bone_scales,
        ..
    }) = args.command
    {
        let pose = melee_sim::bones::FighterPose {
            position: pos,
            facing_dir: facing,
            model_scale,
            bone_scales,
        };
        return melee_sim::bones::write_fox_wait1_bones(
            &assets,
            frame,
            frames,
            &pose,
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

fn parse_vec3(text: &str) -> Result<hsd_types::Vec3, String> {
    let parts: Vec<f32> = text
        .split(',')
        .map(|p| p.trim().parse::<f32>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    match parts[..] {
        [x, y, z] => Ok(hsd_types::Vec3::new(x, y, z)),
        _ => Err("expected x,y,z".into()),
    }
}

fn parse_bone_scale(text: &str) -> Result<(usize, f32), String> {
    let (index, scale) = text.split_once('=').ok_or("expected INDEX=SCALE")?;
    Ok((
        index
            .trim()
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?,
        scale
            .trim()
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?,
    ))
}
