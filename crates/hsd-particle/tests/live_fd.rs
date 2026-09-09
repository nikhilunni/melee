//! Replay the loaded savestate, not the first completed tick's seed/state.
mod common;
#[path = "support/restore.rs"]
mod restore;
use common::RetailTrig;
use gekko_math::rng::HsdRng;
use hsd_archive::Archive;
use hsd_particle::{
    bank::ParticleBank,
    rng_sites::{DrawLog, EMISSION_COUNT, FD_EMISSION},
};
use hsd_types::Mtx;
use serde_json::Value as Json;
use std::{fs, path::PathBuf};

const SAVESTATE_SEED: u32 = 1_286_746_018;
const RANDF_PC: u32 = 0x8038_054c;
const RANDI_PC: u32 = 0x8038_059c;

struct Capture {
    bank: ParticleBank,
    initial: melee_diff::Record,
    states: Vec<melee_diff::Record>,
    ticks: Vec<melee_diff::Record>,
    ledger: Vec<Json>,
    joint: Mtx,
}
fn capture() -> Option<Capture> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness");
    let names = [
        "particles.jsonl.initial.jsonl",
        "particles.jsonl.initial.jsonl.meta.json",
        "particles.jsonl",
        "ledger600.raw.jsonl",
        "tick.expected.jsonl",
    ];
    let paths = names.map(|name| root.join(format!("traces/idle_fd_fox.{name}")));
    let archive_path = root.join("roms/files/GrNLa.dat");
    for path in paths.iter().chain([&archive_path]) {
        if !path.exists() {
            eprintln!("skipping live FD: {} is absent", path.display());
            return None;
        }
    }
    let archive = Archive::parse(&fs::read(archive_path).unwrap()).unwrap();
    let bank = ParticleBank::from_archive(&archive, "map_ptcl", "map_texg").unwrap();
    let initial = restore::read(&paths[0]);
    assert_eq!(initial.len(), 1);
    let metadata: Json = serde_json::from_slice(&fs::read(&paths[1]).unwrap()).unwrap();
    assert_eq!(
        metadata["particles"]["seed"].as_u64().unwrap(),
        u64::from(SAVESTATE_SEED)
    );
    assert_eq!(metadata["sampling"], "savestate_loaded_before_first_tick");
    // The attached cached JObj is outside canonical records. Restore its
    // actual matrix from metadata, resolving the pointer instead of assuming
    // identity. FD animation 0's only track on this joint is the spawn key.
    let particle_meta = &metadata["particles"];
    let joint_pointer = &particle_meta["generators"][0]["fields"]["jobj"];
    let joint = particle_meta["joints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| &j["pointer"] == joint_pointer)
        .unwrap();
    let words = joint["fields"]["matrix"].as_array().unwrap();
    let joint = Mtx(std::array::from_fn(|row| {
        std::array::from_fn(|col| f32::from_bits(words[row * 4 + col].as_u64().unwrap() as u32))
    }));
    let states = restore::read(&paths[2]);
    let ledger = fs::read_to_string(&paths[3])
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect::<Vec<_>>();
    let ticks = restore::read(&paths[4]);
    assert_eq!(states.len(), 8);
    assert_eq!(ledger.len(), 600);
    assert_eq!(ticks.len(), 600);
    Some(Capture {
        bank,
        initial: initial[0].clone(),
        states,
        ticks,
        ledger,
        joint,
    })
}

fn word(draw: &Json, key: &str) -> u32 {
    draw[key].as_u64().unwrap().try_into().unwrap()
}
fn particle_draw(draw: &Json) -> bool {
    let site = word(draw, "lr") - 4;
    if site == EMISSION_COUNT || FD_EMISSION.contains(&site) {
        assert_eq!(
            word(draw, "pc"),
            RANDF_PC,
            "particle RNG variant at {site:#010x}"
        );
        true
    } else {
        // Exact external callers, not a broad PC range that could hide new
        // particle draws. grLast_8021ADD0 + 0x22C also occurs at tick 509.
        match site {
            0x8008_a8bc => assert_eq!(word(draw, "pc"), RANDI_PC),
            0x8021_b040 | 0x8021_af0c | 0x8021_affc => assert_eq!(word(draw, "pc"), RANDF_PC),
            _ => panic!("unclassified RNG caller {site:#010x}"),
        }
        false
    }
}

fn replay(capture: Capture, ticks: usize, compare_state: bool) {
    let mut system = restore::restore(&capture.initial, &capture.bank);
    assert_eq!(system.generators.len(), 1);
    system.generators[0].joint_matrix = Some(capture.joint);
    let mut rng = HsdRng::new(SAVESTATE_SEED);
    assert_eq!(
        restore::uint(&capture.initial, "rng.seed"),
        u64::from(rng.seed)
    );
    let mut counts = Vec::new();
    let mut external_ticks = 0;
    for tick in 0..ticks {
        let ledger = &capture.ledger[tick];
        assert_eq!(capture.ticks[tick].frame, tick as u64);
        assert_eq!(ledger["frame"].as_u64(), Some(tick as u64));
        let expected = ledger["rng_draws"].as_array().unwrap();
        // Independently verify every captured post-draw seed from the initial
        // seed and then the prior simulated tick; never reset from the oracle.
        let mut lcg = rng;
        for (ordinal, draw) in expected.iter().enumerate() {
            lcg.rand();
            assert_eq!(
                lcg.seed,
                word(draw, "seed"),
                "LCG tick {tick} draw {ordinal}"
            );
        }
        let external = expected.iter().take_while(|d| !particle_draw(d)).count();
        assert!(
            expected[external..].iter().all(particle_draw),
            "external draws must precede the particle procs at tick {tick}"
        );
        if external != 0 {
            external_ticks += 1;
        }
        // Model only external draw count/variant at its observed scheduler
        // position. No ledger value controls an emission or particle state.
        for draw in &expected[..external] {
            match word(draw, "pc") {
                RANDI_PC => {
                    rng.randi(100);
                } // ftwaitanim.c:50, inlined in ftCo_8008A7A8
                RANDF_PC => {
                    rng.randf();
                }
                _ => unreachable!(),
            }
        }
        let mut draws = DrawLog::default();
        system
            .proc_main::<RetailTrig>(&mut rng, &mut draws)
            .unwrap();
        system.proc_aux::<RetailTrig>(&mut rng, &mut draws).unwrap();
        let expected_sites = expected[external..]
            .iter()
            .map(|d| word(d, "lr") - 4)
            .collect::<Vec<_>>();
        assert_eq!(
            draws.0.len(),
            expected_sites.len(),
            "tick {tick} particle draw count"
        );
        for (ordinal, (actual, expected)) in draws.0.iter().zip(&expected_sites).enumerate() {
            assert_eq!(actual, expected, "tick {tick} draw {ordinal} site");
        }
        assert_eq!(
            rng.seed,
            restore::uint(&capture.ticks[tick], "rng.seed") as u32,
            "tick {tick} final RNG seed"
        );
        counts.push(draws.0.len());
        if compare_state {
            restore::assert_state(
                &capture.states[tick],
                &restore::snapshot(&system, rng.seed, tick as u64, &capture.bank),
            );
        }
    }
    eprintln!("FD matched {ticks} ticks; particle draws per tick: {counts:?}; external-draw ticks modeled: {external_ticks}; final seed: {:#010x}", rng.seed);
}

#[test]
fn live_fd_eight_ticks_match_every_dumped_field() {
    if let Some(capture) = capture() {
        replay(capture, 8, true);
    }
}
#[test]
fn live_fd_600_ticks_match_rng_counts_sites_and_seeds() {
    if let Some(capture) = capture() {
        replay(capture, 600, false);
    }
}
