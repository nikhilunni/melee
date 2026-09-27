//! Deterministic robustness corpus. Exactness requires separate retail replay.
//! cargo run -p melee-replay --release --example explore -- <assets> <output> [count [skip]]
//!
//! With `count`, version 3 explores that many further input seeds (a
//! xorshift sequence from `EXTRA_SEED_START`) instead of version 2's eight;
//! `skip` continues the sequence past seeds an earlier batch explored.
//!
//! Version 2 starts every case from a retail match-start boundary
//! (`harness/boundaries.toml`), so `harness/replay_to_scenario.py` can replay
//! any case in Dolphin. The game seed is therefore fixed per port layout; the
//! eight seeds below vary only the explorer's own input choices.
use melee_lib::{
    Buttons, Character, ControllerState, FighterObservation, GameAssets, Inputs, Match,
    MatchConfig, PlayerConfig, Port, Seed, Stage, Stick,
};
use melee_replay::Recording;
use serde::Serialize;
use std::{collections::BTreeSet, path::PathBuf};

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
/// Boundary seeds of `start_fd_fox4` (Fox P1) and `start_fd_marth4` (Marth P1).
const BOUNDARY_SEEDS: [u32; 2] = [2_477_457_595, 629_775_590];

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
    } else if !f.grounded() && (f.position().y < -12.0 || f.position().x.abs() > 80.0) {
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

#[derive(Serialize)]
struct ResultRow {
    name: String,
    seed: u32,
    swapped: bool,
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
    let mut arguments = std::env::args().skip(1);
    let directory = PathBuf::from(arguments.next().ok_or("missing asset directory")?);
    let output = PathBuf::from(arguments.next().ok_or("missing output directory")?);
    let (version, seeds): (u32, Vec<u32>) = match arguments.next() {
        None => (CORPUS_VERSION, SEEDS.to_vec()),
        Some(count) => {
            let mut next = Choices(EXTRA_SEED_START);
            let count: usize = count.parse()?;
            let skip: usize = arguments.next().map_or(Ok(0), |skip| skip.parse())?;
            (
                CORPUS_VERSION + 1,
                (0..skip + count).map(|_| next.next()).skip(skip).collect(),
            )
        }
    };
    if output.exists() {
        return Err("output directory must be new to preserve prior evidence".into());
    }
    std::fs::create_dir_all(&output)?;
    let mut report = Report {
        version,
        ticks_per_case: TICKS,
        results: Vec::new(),
    };
    for swapped in [false, true] {
        let characters = if swapped {
            [Character::Marth, Character::Fox]
        } else {
            [Character::Fox, Character::Marth]
        };
        let base = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, characters[0]),
                PlayerConfig::new(Port::P2, characters[1]),
            ],
        )
        .with_stocks(4);
        let assets = GameAssets::load(&directory, &base)?;
        for &seed in &seeds {
            for profile in 0u32..3 {
                let config = base
                    .clone()
                    .with_seed(Seed(BOUNDARY_SEEDS[usize::from(swapped)]));
                let mut game = Match::new(&assets, config.clone())?;
                let mut recording = Recording::new(&config, &assets);
                let mut held = [Held::default(); 2];
                let mut choices = Choices(seed ^ (profile + 1).wrapping_mul(0x9e3779b9));
                let name = format!(
                    "v{version}-swap{}-explore{seed:08x}-profile{profile}",
                    u8::from(swapped)
                );
                let mut row = ResultRow {
                    name: name.clone(),
                    seed,
                    swapped,
                    profile,
                    ticks: 0,
                    status: String::new(),
                    actions: Default::default(),
                    transitions: Default::default(),
                    fault: None,
                };
                let mut previous = [None; 2];
                while game.status().is_running() && game.tick().0 < TICKS {
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
                                held[i] =
                                    choose(fighters[i], fighters[1 - i], &mut choices, profile);
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
