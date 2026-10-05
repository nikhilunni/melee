//! Throughput and lifecycle benchmark for the melee-lib public API.
//!
//! Drives `Match::step` with deterministic pseudo-random controller inputs, the
//! way an AI training loop would: no rendering, no audio, one `Inputs` per tick.
//!
//!   cargo run --release -p melee-lib --example sim_bench -- throughput <files> [ticks]          per-config single-thread stats
//!   cargo run --release -p melee-lib --example sim_bench -- parallel   <files> <threads> [ticks] N independent matches on N threads
//!   cargo run --release -p melee-lib --example sim_bench -- lifecycle  <files>                  load / new / clone / reset costs
//!   cargo run --release -p melee-lib --example sim_bench -- run        <files> [ticks]          one FD Fox-Marth run (for /usr/bin/time -l)
use melee_lib::{
    Buttons, Character, GameAssets, Inputs, Match, MatchConfig, PlayerConfig, Port, Seed, Stage,
    Stick,
};
use std::{hint::black_box, time::Instant};

/// xorshift64*: deterministic, cheap, no dependencies.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick(&mut self, values: &[f32]) -> f32 {
        values[self.below(values.len() as u64) as usize]
    }
}

/// A random policy that holds each sampled controller state for 1-12 ticks,
/// like an agent acting at a few decisions per second.
struct RandomPads {
    rng: Rng,
    held: [(Inputs, u32); 2],
    neutral: bool,
}
impl RandomPads {
    fn new(seed: u64) -> Self {
        Self {
            rng: Rng(seed | 1),
            held: [(Inputs::default(), 0); 2],
            neutral: std::env::var_os("BENCH_NEUTRAL").is_some(),
        }
    }
    fn sample(&mut self, port: Port) -> melee_lib::ControllerState {
        const AXIS: [f32; 9] = [-1.0, -0.7, -0.3, 0.0, 0.0, 0.0, 0.3, 0.7, 1.0];
        let r = &mut self.rng;
        let mut pad = Inputs::default()[port];
        pad.stick = Stick {
            x: r.pick(&AXIS),
            y: r.pick(&AXIS),
        };
        if r.below(6) == 0 {
            pad.cstick = Stick {
                x: r.pick(&AXIS),
                y: r.pick(&AXIS),
            };
        }
        let mut buttons = 0u32;
        for (bit, one_in) in [
            (Buttons::A.0, 4),
            (Buttons::B.0, 6),
            (Buttons::X.0, 8),
            (Buttons::Z.0, 20),
        ] {
            if r.below(one_in) == 0 {
                buttons |= bit;
            }
        }
        if r.below(12) == 0 {
            buttons |= Buttons::L.0;
            pad.left_trigger = 1.0;
        } else if r.below(12) == 0 {
            pad.right_trigger = 0.5;
        }
        pad.buttons = Buttons(buttons);
        pad
    }
    fn next(&mut self) -> Inputs {
        let mut out = Inputs::default();
        // BENCH_NEUTRAL=1: neutral pads every tick (idle fighters), to compare
        // with a neutral-input Dolphin run.
        if self.neutral {
            return out;
        }
        for (i, port) in [Port::P1, Port::P2].into_iter().enumerate() {
            if self.held[i].1 == 0 {
                let pad = self.sample(port);
                self.held[i].0[port] = pad;
                self.held[i].1 = 1 + self.rng.below(12) as u32;
            }
            self.held[i].1 -= 1;
            out[port] = self.held[i].0[port];
        }
        out
    }
}

fn configs() -> Vec<(&'static str, MatchConfig)> {
    let versus = |stage, a, b| {
        MatchConfig::versus(
            stage,
            [PlayerConfig::new(Port::P1, a), PlayerConfig::new(Port::P2, b)],
        )
    };
    vec![
        (
            "Fox vs Marth, Final Destination",
            versus(Stage::FinalDestination, Character::Fox, Character::Marth),
        ),
        (
            "Fox vs Marth, Battlefield",
            versus(Stage::Battlefield, Character::Fox, Character::Marth),
        ),
        (
            "Fox vs Marth, Pokemon Stadium",
            versus(Stage::PokemonStadium, Character::Fox, Character::Marth),
        ),
        (
            "Ice Climbers vs Peach, Pokemon Stadium",
            versus(Stage::PokemonStadium, Character::IceClimbers, Character::Peach),
        ),
        (
            "Samus vs Link, Yoshi's Story",
            versus(Stage::YoshisStory, Character::Samus, Character::Link),
        ),
        (
            "Zelda vs Pikachu, Fountain of Dreams",
            versus(Stage::FountainOfDreams, Character::Zelda, Character::Pikachu),
        ),
    ]
}

#[derive(Default)]
struct RunStats {
    nanos: Vec<u32>,
    matches_finished: u32,
    faults: Vec<String>,
    resets: u32,
}

/// Step `ticks` ticks; a finished or faulted match is reset with the next seed
/// (reset time is excluded from the per-tick samples and reported apart).
fn drive(
    assets: &GameAssets,
    config: &MatchConfig,
    ticks: usize,
    seed: u64,
    observe: bool,
    record: bool,
) -> RunStats {
    let mut stats = RunStats::default();
    if record {
        stats.nanos.reserve_exact(ticks);
    }
    let mut game = Match::new(assets, config.clone().with_seed(Seed(seed as u32))).unwrap();
    let mut pads = RandomPads::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut next_seed = seed as u32;
    let mut checksum = 0f32;
    for _ in 0..ticks {
        if !game.status().is_running() {
            stats.matches_finished += 1;
            next_seed = next_seed.wrapping_add(1);
            game.reset(Seed(next_seed)).unwrap();
            stats.resets += 1;
        }
        let inputs = pads.next();
        let start = Instant::now();
        let result = game.step(&inputs);
        if observe {
            if let Ok(view) = game.observe() {
                for f in view.fighters() {
                    checksum += f.position().x + f.percent();
                }
            }
        }
        let elapsed = start.elapsed().as_nanos() as u32;
        if record {
            stats.nanos.push(elapsed);
        }
        if let Err(e) = result {
            stats.faults.push(format!("{e:?}"));
            next_seed = next_seed.wrapping_add(1);
            game.reset(Seed(next_seed)).unwrap();
            stats.resets += 1;
        }
    }
    black_box(checksum);
    stats
}

fn percentile(sorted: &[u32], p: f64) -> f64 {
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i] as f64 / 1000.0
}

fn throughput(files: &str, ticks: usize) {
    println!(
        "| Config | ticks | mean µs/tick | p50 | p99 | max | ticks/s | x real-time | step+observe mean µs | matches finished | faults |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (name, config) in configs() {
        let assets = GameAssets::load(files, &config).unwrap();
        // Warm-up: caches, page faults, branch predictors.
        drive(&assets, &config, 2_000, 7, false, false);
        let stats = drive(&assets, &config, ticks, 1, false, true);
        let observed = drive(&assets, &config, ticks, 1, true, true);
        let mut sorted = stats.nanos.clone();
        sorted.sort_unstable();
        let total: u64 = stats.nanos.iter().map(|&n| n as u64).sum();
        let mean_us = total as f64 / ticks as f64 / 1000.0;
        let obs_total: u64 = observed.nanos.iter().map(|&n| n as u64).sum();
        let obs_mean = obs_total as f64 / ticks as f64 / 1000.0;
        let tps = 1e6 / mean_us;
        println!(
            "| {name} | {ticks} | {mean_us:.1} | {:.1} | {:.1} | {:.0} | {:.0} | {:.0}x | {obs_mean:.1} | {} | {} |",
            percentile(&sorted, 0.5),
            percentile(&sorted, 0.99),
            *sorted.last().unwrap() as f64 / 1000.0,
            tps,
            tps / 60.0,
            stats.matches_finished,
            stats.faults.len(),
        );
        for f in stats.faults.iter().take(3) {
            eprintln!("  fault in {name}: {f}");
        }
    }
}

fn parallel(files: &str, threads: usize, ticks: usize) {
    let all = configs();
    let (name, config) = &all[0];
    let assets = GameAssets::load(files, config).unwrap();
    drive(&assets, config, 2_000, 7, false, false);
    let start = Instant::now();
    let results: Vec<(u32, usize)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let assets = assets.clone();
                let config = config.clone();
                scope.spawn(move || {
                    let s = drive(&assets, &config, ticks, 100 + t as u64, false, false);
                    (s.matches_finished, s.faults.len())
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let wall = start.elapsed().as_secs_f64();
    let total = (threads * ticks) as f64;
    let faults: usize = results.iter().map(|r| r.1).sum();
    println!(
        "{name}: {threads} threads x {ticks} ticks in {wall:.2} s = {:.0} ticks/s aggregate ({:.0}x real-time), {:.0} ticks/s per thread, faults {faults}",
        total / wall,
        total / wall / 60.0,
        ticks as f64 / wall
    );
}

fn time_n<T>(n: u32, mut f: impl FnMut() -> T) -> f64 {
    let start = Instant::now();
    for _ in 0..n {
        black_box(f());
    }
    start.elapsed().as_secs_f64() * 1e6 / n as f64
}

fn lifecycle(files: &str) {
    for (i, (name, config)) in configs().into_iter().enumerate() {
        // First load in this process (OS file cache state is whatever it is).
        let start = Instant::now();
        let assets = GameAssets::load(files, &config).unwrap();
        let first_load = start.elapsed().as_secs_f64() * 1e3;
        let load_us = time_n(10, || GameAssets::load(files, &config).unwrap());
        let new_us = time_n(50, || Match::new(&assets, config.clone()).unwrap());
        let mut game = Match::new(&assets, config.clone()).unwrap();
        let mut pads = RandomPads::new(3);
        for _ in 0..600 {
            if !game.status().is_running() {
                break;
            }
            let _ = game.step(&pads.next());
        }
        let clone_us = time_n(200, || game.clone());
        let mut dest = game.clone();
        let clone_from_us = time_n(200, || {
            dest.clone_from(&game);
        });
        let mut seed = 0;
        let reset_us = time_n(50, || {
            seed += 1;
            dest.reset(Seed(seed)).unwrap();
        });
        let file_bytes: u64 = GameAssets::files(&config)
            .unwrap()
            .iter()
            .map(|f| {
                std::fs::metadata(std::path::Path::new(files).join(f))
                    .map(|m| m.len())
                    .unwrap_or(0)
            })
            .sum();
        if i == 0 {
            println!("| Config | first load ms | GameAssets::load ms | Match::new µs | clone µs | clone_from µs | reset µs | disc files read |");
            println!("|---|---:|---:|---:|---:|---:|---:|---:|");
        }
        println!(
            "| {name} | {first_load:.1} | {:.1} | {new_us:.0} | {clone_us:.1} | {clone_from_us:.1} | {reset_us:.0} | {:.1} MB ({} files) |",
            load_us / 1e3,
            file_bytes as f64 / 1e6,
            GameAssets::files(&config).unwrap().len()
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: sim-bench <throughput|parallel|lifecycle|run> <files> [...]";
    let mode = args.get(1).expect(usage).as_str();
    let files = args.get(2).expect(usage).as_str();
    let num = |i: usize, default: usize| args.get(i).map(|s| s.parse().unwrap()).unwrap_or(default);
    match mode {
        "throughput" => throughput(files, num(3, 60_000)),
        "parallel" => parallel(files, num(3, 1), num(4, 60_000)),
        "lifecycle" => lifecycle(files),
        "run" => {
            let all = configs();
            let (name, config) = &all[0];
            let start = Instant::now();
            let assets = GameAssets::load(files, config).unwrap();
            let ticks = num(3, 60_000);
            let s = drive(&assets, config, ticks, 1, false, false);
            println!(
                "{name}: load + {ticks} ticks in {:.2} s, {} matches finished, {} faults",
                start.elapsed().as_secs_f64(),
                s.matches_finished,
                s.faults.len()
            );
        }
        _ => panic!("{usage}"),
    }
}
