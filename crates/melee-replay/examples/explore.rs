//! Deterministic robustness corpus. Exactness requires separate retail replay.
//! cargo run -p melee-replay --release --example explore -- <assets> <output> [count [skip]]
//!     [sudden-death] [--boundary <name>]...
//!
//! Every case starts from a retail match-start boundary registered in
//! `harness/boundaries.toml` (`harness/make_boundary.py` adds them), so
//! `harness/replay_to_scenario.py` can replay any case in Dolphin. The game
//! seed is the boundary's; the input seeds vary only the explorer's choices.
//!
//! `--boundary` picks the boundaries (repeatable; any registered stage and
//! character pair the port runs). Without it, the versioned Fox-Marth workload
//! of `docs/MATCHUP_COMPLETENESS.md` runs: `start_fd_fox4` and
//! `start_fd_marth4`, or with `sudden-death` the Sudden Death boundary
//! `sudden_death_start_fd_marth` (one stock at 300%, the Bob-omb rain).
//! A Sudden Death boundary always uses the Sudden Death policy.
//!
//! With `count`, version 3 explores that many input seeds (a xorshift sequence
//! from `EXTRA_SEED_START`) instead of version 2's eight; `skip` continues the
//! sequence past seeds an earlier batch explored.
use melee_lib::{
    Buttons, ControllerState, FighterObservation, GameAssets, Inputs, Match, Port, Stage, Stick,
};
use melee_replay::{boundary, Recording};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

const CORPUS_VERSION: u32 = 2;
const SEEDS: [u32; 8] = [
    1,
    42,
    73,
    1337,
    0x12345678,
    0xdeadbeef,
    0x80000000,
    u32::MAX,
];
const TICKS: u64 = 6000;
const EXTRA_SEED_START: u32 = 0x00C0_FFEE;
/// A Sudden Death case: its countdown and GO take about 1200 ticks.
const SUDDEN_DEATH_TICKS: u64 = 3000;
/// The default workloads and their case-name tags, fixed so that earlier
/// batches keep their names (`swap1` is Marth on P1).
const FOX_MARTH_FD: [(&str, &str); 2] = [("start_fd_fox4", "swap0"), ("start_fd_marth4", "swap1")];
const FOX_MARTH_FD_SUDDEN_DEATH: [(&str, &str); 1] = [("sudden_death_start_fd_marth", "swap1")];

/// Where the policies treat a fighter as near an edge or off the stage: the
/// main platform's ledge x minus a margin. Final Destination keeps the values
/// the recorded corpus batches ran with.
#[derive(Clone, Copy)]
struct StageEdges {
    /// Off stage beyond this |x| (or below y = -12): recover.
    offstage_x: f32,
    /// Sudden Death: beyond this |x| every move heads inward.
    sudden_death_x: f32,
}
fn stage_edges(stage: Stage) -> StageEdges {
    // Ledge x: FD 85.57, BF 68.40, YS 56.00, DL 77.27, FoD 63.35, PS 87.75
    // (the stages' ledge lines).
    let (offstage_x, sudden_death_x) = match stage {
        Stage::FinalDestination => (80.0, 55.0),
        Stage::Battlefield => (63.0, 38.0),
        Stage::YoshisStory => (51.0, 26.0),
        Stage::DreamLand => (72.0, 47.0),
        Stage::FountainOfDreams => (58.0, 33.0),
        Stage::PokemonStadium => (82.0, 57.0),
    };
    StageEdges {
        offstage_x,
        sudden_death_x,
    }
}

// Caller-owned randomness must never consume the simulated game's RNG.
struct Choices(u32);
impl Choices {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}
#[derive(Clone, Copy, Default)]
struct Held {
    pad: ControllerState,
    remaining: u32,
}

fn direction(choice: u32) -> Stick {
    match choice % 8 {
        0 => Stick { x: 1.0, y: 0.0 },
        1 => Stick { x: -1.0, y: 0.0 },
        2 => Stick { x: 0.0, y: 1.0 },
        3 => Stick { x: 0.0, y: -1.0 },
        4 => Stick { x: 0.7, y: 0.7 },
        5 => Stick { x: -0.7, y: 0.7 },
        6 => Stick { x: 0.7, y: -0.7 },
        _ => Stick { x: -0.7, y: -0.7 },
    }
}

fn choose(
    f: FighterObservation<'_>,
    other: FighterObservation<'_>,
    choices: &mut Choices,
    profile: u32,
    edges: StageEdges,
) -> Held {
    let choice = choices.next();
    let mut pad = ControllerState::default();
    let toward = if other.position().x >= f.position().x {
        1.0
    } else {
        -1.0
    };
    let inward = if f.position().x >= 0.0 { -1.0 } else { 1.0 };
    let duration = [1, 2, 3, 5, 8, 13, 21, 34][(choices.next() % 8) as usize];
    if f.holding_fighter() {
        if choice.is_multiple_of(5) {
            pad.buttons = Buttons::A;
        } else {
            pad.cstick = direction(choice);
        }
    } else if f.grabbed() {
        pad.buttons = if choice & 1 == 0 {
            Buttons::X
        } else {
            Buttons::A
        };
        pad.stick = direction(choice);
    } else if f.on_ledge() {
        match choice % 7 {
            0 => {}
            1 => pad.buttons = Buttons::A,
            2 => pad.buttons = Buttons::X,
            3 => pad.buttons = Buttons::L,
            4 => pad.stick.x = inward,
            5 => pad.stick.y = -1.0,
            _ => pad.cstick = direction(choice),
        }
    } else if !f.grounded() && (f.position().y < -12.0 || f.position().x.abs() > edges.offstage_x) {
        match choice % 5 {
            0 => {
                pad.buttons = Buttons::X;
                pad.stick.x = inward;
            }
            1 | 2 => {
                pad.buttons = Buttons::B;
                pad.stick = Stick {
                    x: inward * 0.4,
                    y: 0.9,
                };
            }
            3 => {
                pad.buttons = Buttons::B;
                pad.stick.x = inward;
            }
            _ => pad.stick.x = inward,
        }
    } else if f.shield_active() {
        match choice % 8 {
            0 => {}
            1 => pad.buttons = Buttons::Z,
            2 => pad.buttons = Buttons::L | Buttons::A,
            3 => pad.buttons = Buttons::X | Buttons::L,
            4 => {
                pad.buttons = Buttons::L;
                pad.stick.y = -1.0;
            }
            5 => {
                pad.buttons = Buttons::L;
                pad.stick.x = toward;
            }
            _ => {
                pad.left_trigger = [0.3, 0.5, 0.8][(choice % 3) as usize];
            }
        }
    } else if f.grounded()
        && (f.position().x - other.position().x).abs() > 25.0
        && !choice.is_multiple_of(4)
        && profile != 2
    {
        pad.stick.x = toward;
    } else {
        match choice.wrapping_add(profile) % 16 {
            0 => {}
            1 | 2 => {
                pad.stick.x = toward;
                pad.buttons = Buttons::A;
            }
            3 => pad.buttons = Buttons::Z,
            4 => pad.buttons = Buttons::X,
            5 => pad.buttons = Buttons::UP,
            6 => {
                pad.buttons = Buttons::L;
                pad.stick = direction(choice >> 4);
            }
            7 | 8 => {
                pad.buttons = Buttons::B;
                pad.stick = direction(choice >> 4);
            }
            9 => pad.buttons = Buttons::B,
            10 => pad.cstick = direction(choice >> 4),
            11 => {
                pad.buttons = Buttons::A;
                pad.stick.y = -0.5;
            }
            12 => {
                pad.buttons = Buttons::A;
                pad.stick.y = 1.0;
            }
            13 => {
                pad.stick.x = if profile == 2 { -inward } else { toward };
            }
            14 => {
                pad.buttons = Buttons::L | Buttons::A;
            }
            _ => {
                pad.buttons = Buttons::X | Buttons::B;
                pad.stick.y = 1.0;
            }
        }
    }
    if pad.buttons.intersects(Buttons::L) {
        pad.left_trigger = 1.0;
    }
    Held {
        pad,
        remaining: duration,
    }
}

/// Sudden Death's policy: at 300% one hit ends the match, so attacks wait
/// until the opponent is far away, where A and Z mostly pick up, throw or
/// drop Bob-ombs and the C-stick throws them.
fn choose_sudden_death(
    f: FighterObservation<'_>,
    other: FighterObservation<'_>,
    choices: &mut Choices,
    edges: StageEdges,
) -> Held {
    let choice = choices.next();
    let mut pad = ControllerState::default();
    let far = (other.position().x - f.position().x).abs() > 35.0;
    let inward = if f.position().x >= 0.0 { -1.0 } else { 1.0 };
    let duration = [1, 2, 3, 5, 8, 13, 21, 34][(choices.next() % 8) as usize];
    // Stay on the stage: near an edge every move heads inward.
    let near_edge = f.position().x.abs() > edges.sudden_death_x || f.position().y < -5.0;
    let side = if near_edge || choice & 0x100 != 0 {
        inward
    } else {
        -inward
    };
    match choice % 12 {
        0 => {}
        1 | 2 => pad.stick.x = side,
        3 => pad.stick.x = 0.5 * side,
        4 if !near_edge => pad.buttons = Buttons::X,
        5 => pad.buttons = Buttons::L,
        6 => pad.stick.y = -1.0,
        7 if far => pad.buttons = Buttons::A,
        8 if far => pad.buttons = Buttons::Z,
        9 if far => pad.cstick = direction(choice >> 4),
        10 => pad.buttons = Buttons::UP,
        _ => pad.stick.x = 0.7 * side,
    }
    if pad.buttons.intersects(Buttons::L) {
        pad.left_trigger = 1.0;
    }
    Held {
        pad,
        remaining: duration,
    }
}

#[derive(Serialize)]
struct ResultRow {
    name: String,
    boundary: String,
    seed: u32,
    profile: u32,
    ticks: u64,
    status: String,
    actions: [BTreeSet<u16>; 2],
    transitions: [BTreeSet<(u16, u16)>; 2],
    fault: Option<String>,
}
#[derive(Serialize)]
struct Report {
    version: u32,
    ticks_per_case: u64,
    results: Vec<ResultRow>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut positional = Vec::new();
    let mut chosen = Vec::new();
    let mut sudden_death = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--boundary" => chosen.push(arguments.next().ok_or("--boundary needs a name")?),
            "sudden-death" => sudden_death = true,
            _ => positional.push(argument),
        }
    }
    let mut positional = positional.into_iter();
    let directory = PathBuf::from(positional.next().ok_or("missing asset directory")?);
    let output = PathBuf::from(positional.next().ok_or("missing output directory")?);
    let (version, seeds): (u32, Vec<u32>) = match positional.next() {
        None => (CORPUS_VERSION, SEEDS.to_vec()),
        Some(count) => {
            let mut next = Choices(EXTRA_SEED_START);
            let count: usize = count.parse()?;
            let skip: usize = positional.next().map_or(Ok(0), |skip| skip.parse())?;
            (
                CORPUS_VERSION + 1,
                (0..skip + count).map(|_| next.next()).skip(skip).collect(),
            )
        }
    };
    if let Some(extra) = positional.next() {
        return Err(format!("unexpected argument {extra}").into());
    }
    let registry = boundary::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/boundaries.toml"),
    )?;
    let defaults: &[(&str, &str)] = if sudden_death {
        &FOX_MARTH_FD_SUDDEN_DEATH
    } else {
        &FOX_MARTH_FD
    };
    let names: Vec<String> = if chosen.is_empty() {
        defaults.iter().map(|(name, _)| name.to_string()).collect()
    } else {
        chosen
    };
    let mut runs = Vec::new();
    for name in &names {
        let found = registry
            .iter()
            .find(|b| &b.name == name)
            .ok_or_else(|| format!("{name} is not in harness/boundaries.toml"))?;
        if found.sudden_death != sudden_death {
            return Err(format!(
                "{name}: pass `sudden-death` exactly when every boundary is a Sudden Death one"
            )
            .into());
        }
        // Default workloads keep their historical tags; others use the name.
        let tag = FOX_MARTH_FD
            .iter()
            .chain(&FOX_MARTH_FD_SUDDEN_DEATH)
            .find(|(n, _)| n == name)
            .map_or_else(
                || name.strip_prefix("start_").unwrap_or(name).to_string(),
                |(_, tag)| tag.to_string(),
            );
        runs.push((found.clone(), tag));
    }
    let ticks = if sudden_death {
        SUDDEN_DEATH_TICKS
    } else {
        TICKS
    };
    if output.exists() {
        return Err("output directory must be new to preserve prior evidence".into());
    }
    std::fs::create_dir_all(&output)?;
    let mut report = Report {
        version,
        ticks_per_case: ticks,
        results: Vec::new(),
    };
    for (boundary, tag) in &runs {
        let base = boundary.config()?;
        let edges = stage_edges(base.stage);
        let assets = GameAssets::load(&directory, &base)?;
        for &seed in &seeds {
            for profile in 0u32..3 {
                let config = base.clone();
                let mut game = Match::new(&assets, config.clone())?;
                let mut recording = Recording::new(&config, &assets);
                let mut held = [Held::default(); 2];
                let mut choices = Choices(seed ^ (profile + 1).wrapping_mul(0x9e3779b9));
                let name = format!(
                    "v{version}{}-{tag}-explore{seed:08x}-profile{profile}",
                    if sudden_death { "sd" } else { "" },
                );
                let mut row = ResultRow {
                    name: name.clone(),
                    boundary: boundary.name.clone(),
                    seed,
                    profile,
                    ticks: 0,
                    status: String::new(),
                    actions: Default::default(),
                    transitions: Default::default(),
                    fault: None,
                };
                let mut previous = [None; 2];
                while game.status().is_running() && game.tick().0 < ticks {
                    let mut inputs = Inputs::default();
                    {
                        let view = game.observe()?;
                        let fighters = [
                            view.fighter(Port::P1).unwrap(),
                            view.fighter(Port::P2).unwrap(),
                        ];
                        for i in 0..2 {
                            let action = fighters[i].action().0;
                            row.actions[i].insert(action);
                            if let Some(old) = previous[i] {
                                if old != action {
                                    row.transitions[i].insert((old, action));
                                }
                            }
                            previous[i] = Some(action);
                            if held[i].remaining == 0 {
                                held[i] = if sudden_death {
                                    choose_sudden_death(
                                        fighters[i],
                                        fighters[1 - i],
                                        &mut choices,
                                        edges,
                                    )
                                } else {
                                    choose(
                                        fighters[i],
                                        fighters[1 - i],
                                        &mut choices,
                                        profile,
                                        edges,
                                    )
                                };
                            }
                            inputs.0[i] = held[i].pad;
                            held[i].remaining -= 1;
                        }
                    }
                    recording.push(inputs)?;
                    if let Err(error) = game.step(&inputs) {
                        let message = format!("{error:?}");
                        recording.fail(message.clone());
                        row.fault = Some(message);
                        break;
                    }
                }
                row.ticks = game.tick().0;
                row.status = format!("{:?}", game.status());
                recording.save_new(&output.join(format!("{name}.json")))?;
                println!(
                    "{name}: ticks={} status={} fault={:?}",
                    row.ticks, row.status, row.fault
                );
                report.results.push(row);
                std::fs::write(
                    output.join("report.json"),
                    serde_json::to_vec_pretty(&report)?,
                )?;
            }
        }
    }
    if report.results.iter().any(|r| r.fault.is_some()) {
        return Err("corpus contains reproduced faults".into());
    }
    Ok(())
}
