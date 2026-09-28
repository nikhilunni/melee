//! Headless simulator. Runs a scenario and writes a canonical trace.
//!
//! Usage:
//!   melee-sim bones --fighter fox --anim Wait1 --frame 0 --frames 2
//!   melee-sim run harness/scenarios/idle_fd_fox.toml --out actual.jsonl
//!   melee-sim gate harness/scenarios/idle_fd_fox.toml
//!   melee-diff expected.jsonl actual.jsonl
use clap::{Parser, Subcommand};
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Cold-start a Slippi replay and report its first divergence or port boundary.
    Replay {
        file: PathBuf,
        #[arg(long, action = clap::ArgAction::Set)]
        all_characters_unlocked: Option<bool>,
        /// Independently measured pre-music seed (decimal).
        #[arg(long)]
        boundary_seed: Option<u32>,
    },
    /// Run the imported savestate and emit one canonical record per tick.
    Run {
        scenario: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Run a tick-clock scenario's own inputs from a recording of the same
    /// savestate, with item state; searches inputs before recording them.
    DryRun {
        scenario: PathBuf,
        /// A recorded scenario from the same savestate.
        #[arg(long)]
        state_from: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Run and compare every key against the scenario's canonical tick trace.
    Gate { scenario: PathBuf },
    /// Gate a scenario and, at its first divergence, report every differing
    /// key, the motion history, items, RNG writers, particle call sites and
    /// generators, and bones where those captures exist.
    Triage { scenario: PathBuf },
    /// Search tick-clock input edits (a TOML spec, see search.rs) for ones
    /// that reach a goal in the port, branching by cloning the match.
    Search {
        scenario: PathBuf,
        /// A recorded scenario from the same savestate.
        #[arg(long)]
        state_from: PathBuf,
        #[arg(long)]
        spec: PathBuf,
        /// Write the earliest success as a scenario file.
        #[arg(long)]
        out: Option<PathBuf>,
        /// The written scenario's name (default: the base scenario's).
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value_t = 8)]
        jobs: usize,
    },
    /// Gate a scenario and record external particle spawn/joint/flag inputs.
    FixtureSpawns {
        scenario: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Export only this many initial ticks; the entire scenario is still gated.
        #[arg(long)]
        ticks: Option<u64>,
    },
    /// Replay a scenario against its retail bone dump and report the first
    /// differing bone per fighter and tick.
    BonesDiff {
        scenario: PathBuf,
        #[arg(long, default_value_t = 12)]
        limit: usize,
        /// List every differing word of this one tick.
        #[arg(long)]
        tick: Option<u64>,
    },
    /// Replay each tick of a retail camera dump (`record.py --camera`) through
    /// the ported camera in isolation and report differing fields.
    CameraDiff {
        scenario: PathBuf,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Compare the port's particle system with the retail particle dump per tick.
    ParticlesDiff {
        scenario: PathBuf,
        #[arg(long, default_value_t = 0)]
        from: u64,
        #[arg(long)]
        to: u64,
    },
    /// Compare the port's particle RNG call sites with the retail ledger per tick.
    ParticleSites {
        scenario: PathBuf,
        #[arg(long, default_value_t = 0)]
        from: u64,
        #[arg(long)]
        to: u64,
        /// The ledger capture's suffix (`ledger600` for a start scene).
        #[arg(long, default_value = "ledger")]
        ledger: String,
    },
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
    match args.command {
        Command::Replay {
            file,
            all_characters_unlocked,
            boundary_seed,
        } => {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            let report = melee_sim::replay::run_file(
                &file,
                &root,
                melee_sim::replay::Setup {
                    all_characters_unlocked,
                    boundary_seed,
                },
            )?;
            println!("{report}");
            anyhow::ensure!(
                !matches!(report.stop, melee_sim::replay::Stop::Diverged(_)),
                "ported-state replay mismatch"
            );
            Ok(())
        }
        Command::Bones {
            frame,
            frames,
            assets,
            pos,
            facing,
            model_scale,
            bone_scales,
            ..
        } => {
            let pose = melee_sim::bones::FighterPose {
                position: pos,
                facing_dir: facing,
                model_scale,
                bone_scales,
            };
            melee_sim::bones::write_fox_wait1_bones(
                &assets,
                frame,
                frames,
                &pose,
                io::BufWriter::new(io::stdout().lock()),
            )
        }
        Command::DryRun {
            scenario,
            state_from,
            out,
        } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            let reference = melee_sim::scenario::Scenario::load(&state_from)?;
            melee_sim::trace::write_dry_run(
                &scenario,
                &reference,
                io::BufWriter::new(std::fs::File::create(out)?),
            )
        }
        Command::Run { scenario, out } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            melee_sim::trace::write_run(&scenario, io::BufWriter::new(std::fs::File::create(out)?))
        }
        Command::BonesDiff {
            scenario,
            limit,
            tick,
        } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            let report = melee_sim::bones::bones_diff(&scenario, limit, tick)?;
            for line in &report {
                println!("{line}");
            }
            anyhow::ensure!(report.is_empty(), "bone mismatches");
            println!("bones match");
            Ok(())
        }
        Command::CameraDiff { scenario, limit } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            let (samples, report) = melee_sim::camera::camera_diff(&scenario, limit)?;
            for line in &report {
                println!("{line}");
            }
            println!("{samples} samples, {} mismatches listed", report.len());
            Ok(())
        }
        Command::ParticlesDiff { scenario, from, to } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            let report = melee_sim::trace::particle_state_diff(&scenario, from, to)?;
            for line in &report {
                println!("{line}");
            }
            println!("{} differing ticks", report.len());
            Ok(())
        }
        Command::ParticleSites {
            scenario,
            from,
            to,
            ledger,
        } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            let report = melee_sim::trace::particle_site_diff(&scenario, from, to, &ledger)?;
            for line in &report {
                println!("{line}");
            }
            println!("{} differing ticks", report.len());
            Ok(())
        }
        Command::Search {
            scenario: path,
            state_from,
            spec,
            out,
            name,
            jobs,
        } => {
            let scenario = melee_sim::scenario::Scenario::load(&path)?;
            let reference = melee_sim::scenario::Scenario::load(&state_from)?;
            let spec: melee_sim::search::Spec = toml::from_str(&std::fs::read_to_string(&spec)?)?;
            let t0 = std::time::Instant::now();
            let (found, tried) = melee_sim::search::search(&scenario, &reference, &spec, jobs)?;
            println!(
                "{tried} candidates simulated in {:.1}s; {} reached the goal",
                t0.elapsed().as_secs_f64(),
                found.len()
            );
            for f in &found {
                let edits: Vec<String> = f
                    .edits
                    .iter()
                    .map(|e| {
                        let hold = e.hold.map_or(String::new(), |h| format!(" for {h}"));
                        format!(
                            "port {} @{} {}{hold}",
                            e.port,
                            e.tick,
                            melee_sim::search::inline_raw(&e.raw)
                        )
                    })
                    .collect();
                println!("reached {:?}: {}", f.reached, edits.join("; "));
            }
            if let (Some(out), Some(best)) = (out, found.first()) {
                let text = melee_sim::search::write_scenario(
                    &std::fs::read_to_string(&path)?,
                    best,
                    &spec,
                    name.as_deref(),
                )?;
                std::fs::write(&out, text)?;
                println!("wrote {}", out.display());
            }
            anyhow::ensure!(!found.is_empty(), "no candidate reached the goal");
            Ok(())
        }
        Command::Triage { scenario } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            print!("{}", melee_sim::triage::triage(&scenario)?);
            Ok(())
        }
        Command::Gate { scenario } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            melee_sim::trace::gate_items(&scenario)?;
            println!(
                "{} ticks, {} keys, 0 divergences",
                scenario.frames,
                melee_sim::trace::compared_keys(&scenario)?
            );
            Ok(())
        }
        Command::FixtureSpawns {
            scenario,
            out,
            ticks,
        } => {
            let scenario = melee_sim::scenario::Scenario::load(&scenario)?;
            melee_sim::trace::fixture_spawns(&scenario, &out, ticks)?;
            println!(
                "{} ticks, {} keys, 0 divergences; fixture {}",
                scenario.frames,
                melee_sim::trace::compared_keys(&scenario)?,
                out.display()
            );
            Ok(())
        }
    }
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
