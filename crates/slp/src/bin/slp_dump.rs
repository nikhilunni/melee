//! Summarise a Slippi replay and optionally write the harness scenario TOML
//! and canonical trace JSONL.
//!
//!   slp-dump <replay.slp> [--scenario out.toml] [--trace out.jsonl] [--frames]

use anyhow::{bail, Context, Result};
use slp::{harness_frame, ids, PlayerType, Replay, SLIPPI_FIRST_FRAME};
use std::io::Write;
use std::path::PathBuf;

struct Args {
    input: PathBuf,
    scenario: Option<PathBuf>,
    trace: Option<PathBuf>,
    frames: bool,
}

fn usage() -> ! {
    eprintln!("usage: slp-dump <replay.slp> [--scenario out.toml] [--trace out.jsonl] [--frames]");
    std::process::exit(2)
}

fn parse_args() -> Result<Args> {
    let mut input = None;
    let mut scenario = None;
    let mut trace = None;
    let mut frames = false;
    let mut it = std::env::args_os().skip(1);
    while let Some(a) = it.next() {
        match a.to_str() {
            Some("--scenario") => {
                scenario = Some(PathBuf::from(it.next().context("--scenario needs a path")?))
            }
            Some("--trace") => {
                trace = Some(PathBuf::from(it.next().context("--trace needs a path")?))
            }
            Some("--frames") => frames = true,
            Some("-h") | Some("--help") => usage(),
            Some(s) if s.starts_with('-') => bail!("unknown flag {s}"),
            _ if input.is_none() => input = Some(PathBuf::from(a)),
            _ => bail!("more than one input file"),
        }
    }
    let Some(input) = input else { usage() };
    Ok(Args {
        input,
        scenario,
        trace,
        frames,
    })
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let bytes =
        std::fs::read(&args.input).with_context(|| format!("read {}", args.input.display()))?;
    let replay =
        Replay::parse(&bytes).with_context(|| format!("parse {}", args.input.display()))?;
    let gs = &replay.start;

    println!(
        "file:      {} ({} bytes)",
        args.input.display(),
        bytes.len()
    );
    println!("version:   Slippi {}", gs.version);
    println!(
        "stage:     {} ({})",
        gs.stage,
        ids::stage_name(gs.stage).unwrap_or("unknown")
    );
    println!(
        "mode:      game_mode={} timer_type={} timer={}s teams={} online={}",
        gs.game_mode(),
        gs.timer_type(),
        gs.game_timer,
        gs.is_teams,
        gs.is_online()
    );
    println!(
        "seed:      game_start=0x{:08X} first_frame=0x{:08X}",
        gs.random_seed,
        replay.initial_seed()
    );
    for p in gs.players.iter().filter(|p| p.is_present()) {
        let kind = match p.player_type {
            PlayerType::Human => "human".to_string(),
            PlayerType::Cpu => format!("cpu lv{}", p.cpu_level),
            PlayerType::Demo => "demo".to_string(),
            PlayerType::Empty => unreachable!(),
        };
        println!(
            "port {}:    {} ({}) costume={} stocks={} {}",
            p.port + 1,
            ids::external_character_name(p.character).unwrap_or("unknown"),
            p.character,
            p.costume,
            p.stock_start_count,
            kind
        );
    }
    match (replay.first_frame(), replay.last_frame()) {
        (Some(f), Some(l)) => println!(
            "frames:    {} (slippi {f}..={l}; harness 0..={})",
            replay.frames.len(),
            harness_frame(l).unwrap_or(0)
        ),
        _ => println!("frames:    0"),
    }
    if let Some(mf) = replay.metadata_last_frame() {
        println!("metadata:  lastFrame={mf}");
    }
    if let Some(m) = &replay.metadata {
        if let Some(s) = m.get("startAt").and_then(|v| v.as_str()) {
            println!("startAt:   {s}");
        }
        if let Some(s) = m.get("playedOn").and_then(|v| v.as_str()) {
            println!("playedOn:  {s}");
        }
    }
    match &replay.end {
        Some(e) => println!(
            "end:       method={} lras={:?} placements={:?}",
            e.method, e.lras_initiator, e.placements
        ),
        None => println!("end:       <missing Game End>"),
    }
    if replay.incomplete {
        println!("warning:   stream is incomplete (unfinalised or truncated file)");
    }
    if !replay.skipped_commands.is_empty() {
        let list: Vec<String> = replay
            .skipped_commands
            .iter()
            .map(|c| format!("0x{c:02X}"))
            .collect();
        println!("skipped:   {}", list.join(" "));
    }
    let items: usize = replay.frames.values().map(|f| f.items.len()).sum();
    if items > 0 {
        println!("items:     {items} item updates");
    }

    if args.frames {
        println!();
        println!("harness  slippi  seed        | per port: motion pos(x,y) pct");
        for frame in replay.frames.values() {
            let hf = harness_frame(frame.number)
                .map(|h| h.to_string())
                .unwrap_or_else(|| "-".into());
            let seed = frame
                .start_seed()
                .map(|s| format!("0x{s:08X}"))
                .unwrap_or_else(|| "-".into());
            print!("{hf:>7}  {:>6}  {seed:>10} |", frame.number);
            for (port, pf) in frame.ports.iter().enumerate() {
                if let Some(post) = &pf.leader.post {
                    print!(
                        " p{port}: {:#05X} ({:.3},{:.3}) {:.1}%",
                        post.action_state, post.position_x, post.position_y, post.percent
                    );
                }
            }
            println!();
        }
    }

    if let Some(path) = &args.scenario {
        let name = args
            .input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("slippi_replay");
        let toml = slp::to_scenario_named(&replay, name);
        std::fs::write(path, toml).with_context(|| format!("write {}", path.display()))?;
        println!("wrote scenario {}", path.display());
    }
    if let Some(path) = &args.trace {
        let records = slp::to_trace(&replay);
        let mut w = std::io::BufWriter::new(
            std::fs::File::create(path).with_context(|| format!("create {}", path.display()))?,
        );
        for r in &records {
            serde_json::to_writer(&mut w, r)?;
            w.write_all(b"\n")?;
        }
        w.flush()?;
        println!(
            "wrote trace {} ({} records, harness frame 0 = slippi frame {})",
            path.display(),
            records.len(),
            replay.first_frame().unwrap_or(SLIPPI_FIRST_FRAME)
        );
    }
    Ok(())
}
