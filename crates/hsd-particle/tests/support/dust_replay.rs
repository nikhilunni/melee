//! Shared full-field replay; display-cache exclusion is the original dash helper.
use crate::{common::RetailTrig, restore, spawns};
use gekko_math::rng::HsdRng;
use hsd_archive::Archive;
use hsd_particle::{bank::ParticleBank, rng_sites::DrawLog};
use serde_json::Value as Json;
use std::{collections::BTreeMap, fs, path::PathBuf};

/// AppSRT fields written by the *display* pass (`psDispSubAppSRT`,
/// psdisp.c:1400-1439): the cached model matrix, the camera-view product with
/// its column lengths, the display status bits and `psFrameNum`. They need the
/// active camera and the render schedule, neither of which the simulation
/// owns (rendering is Milestone 8), and no simulation field depends on them:
/// every other generator/particle/AppSRT field matches over all captured ticks.
/// Same reasoning as the render-time JObj matrix caches excluded by the bone
/// oracle (melee-ft/src/dynamics/README.md).
fn is_display_cache(field: &str) -> bool {
    field.starts_with("particles.appsrt[")
        && [
            "frame_count",
            "matrix",
            "scale_x",
            "scale_y",
            "unknown_float",
            "status",
        ]
        .iter()
        .any(|suffix| {
            field
                .rsplit_once("].")
                .is_some_and(|(_, name)| name == *suffix || name.starts_with(&format!("{suffix}[")))
        })
}

fn word(draw: &Json, key: &str) -> u32 {
    draw[key].as_u64().unwrap().try_into().unwrap()
}
fn particle_draw(draw: &Json) -> bool {
    let site = word(draw, "lr") - 4;
    match site {
        0x801c_26ac | 0x8006_3990 | 0x8006_3b70 | 0x802f_4d44 | 0x802f_4d54 | 0x8008_a8bc
        | 0x8009_fcdc | 0x8009_fd00 | 0x8009_fd24 | 0x8021_affc | 0x8021_aec8 | 0x8021_b040
        | 0x8021_af0c | 0x801e_348c | 0x801e_34dc | 0x801e_3534 | 0x801e_3560 | 0x801e_3578
        | 0x8021_1478 | 0x8021_1550 | 0x8021_1644 | 0x801e_3594 | 0x801e_35a4 | 0x801e_3610
        | 0x801e_36b0 => false,
        // Full symbol extents of interpreter, emitter, generator pass and constructor.
        0x8039_9114 | 0x8039_930c..=0x8039_ceab | 0x8039_dad4..=0x8039_f6cb => {
            assert_eq!(word(draw, "pc"), 0x8038_054c);
            true
        }
        _ => panic!("unclassified draw {site:#010x}"),
    }
}
pub fn replay(name: &str, tick_count: usize) -> usize {
    replay_prefix(name, tick_count, tick_count)
}

/// Explicit partial-port evidence. The complete recording length is still checked;
/// existing full replays always compare every frame through `replay` above.
pub fn replay_prefix(name: &str, recording_ticks: usize, tick_count: usize) -> usize {
    assert!(tick_count > 0 && tick_count <= recording_ticks);
    // `name` is the full scene name (e.g. "dash_fd_fox", "start_bf_fox").
    let battlefield = name.contains("_bf_");
    let scene = name;
    // A match-start savestate sits before its first tick's procs.
    let story = name.contains("_ys_");
    let match_start = name.starts_with("start_");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness");
    let paths = [
        "particles.jsonl.initial.jsonl",
        "particles.jsonl",
        if name.starts_with("idle_") || name.starts_with("start_") {
            "ledger600.raw.jsonl"
        } else {
            "ledger.raw.jsonl"
        },
        "tick.expected.jsonl",
        "particles.jsonl.initial.jsonl.meta.json",
    ]
    .map(|s| root.join(format!("traces/{name}.{s}")));
    let archives = [
        if name.contains("_dl_") {
            "GrOp.dat"
        } else if story {
            "GrSt.dat"
        } else if battlefield {
            "GrNBa.dat"
        } else {
            "GrNLa.dat"
        },
        "EfCoData.dat",
    ]
    .map(|s| root.join(format!("roms/files/{s}")));
    for path in paths.iter().chain(archives.iter()) {
        if !path.exists() {
            eprintln!("skipping {scene}: {} absent", path.display());
            return 0;
        }
    }
    let stage = Archive::parse(&fs::read(&archives[0]).unwrap()).unwrap();
    let effect = Archive::parse(&fs::read(&archives[1]).unwrap()).unwrap();
    let banks = BTreeMap::from([
        (
            30,
            ParticleBank::from_archive(&stage, "map_ptcl", "map_texg").unwrap(),
        ),
        (0, common_bank(&effect)),
    ]);
    let initial = restore::read(&paths[0]);
    let states = restore::read(&paths[1]);
    let ledger: Vec<Json> = fs::read_to_string(&paths[2])
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let ticks = restore::read(&paths[3]);
    assert_eq!(
        (states.len(), ledger.len(), ticks.len()),
        (recording_ticks, recording_ticks, recording_ticks)
    );
    assert_eq!(initial.len(), 1);
    let mut system = restore::restore(&initial[0], &banks);
    // Restore the attached stage JObj from initial metadata, as live_fd does.
    let metadata: Json = serde_json::from_slice(&fs::read(&paths[4]).unwrap()).unwrap();
    let particle_meta = &metadata["particles"];
    for (generator, captured) in system
        .generators
        .iter_mut()
        .zip(particle_meta["generators"].as_array().unwrap())
    {
        if battlefield {
            // Production restore resolves the sole captured BF attachment to
            // map 1, descendant 2 (grAnime excludes Ground's wrapper joint).
            generator.attachment_id = Some(102);
        }
        let pointer = &captured["fields"]["jobj"];
        if pointer == 0 {
            continue;
        }
        let joint = particle_meta["joints"]
            .as_array()
            .unwrap()
            .iter()
            .find(|j| &j["pointer"] == pointer)
            .unwrap();
        let words = joint["fields"]["matrix"].as_array().unwrap();
        generator.joint_matrix = Some(hsd_types::Mtx(std::array::from_fn(|row| {
            std::array::from_fn(|col| f32::from_bits(words[row * 4 + col].as_u64().unwrap() as u32))
        })));
    }
    let mut rng = HsdRng::new(restore::uint(&initial[0], "rng.seed") as u32);
    let mut spawns = spawns::Spawns::new();
    let mut particle_draws = 0;
    let mut field_count = 0;
    let mut mismatches = BTreeMap::<String, usize>::new();
    let mut display_cache_fields = 0usize;
    for tick in 0..tick_count {
        assert_eq!(ticks[tick].frame, tick as u64);
        assert_eq!(states[tick].frame, tick as u64);
        assert_eq!(states[tick].phase, "particles");
        assert_eq!(ledger[tick]["frame"].as_u64(), Some(tick as u64));
        // The idle savestate resumes a partial tick; subsequent ticks have a display pass.
        if tick > 0 {
            system.sort_for_display(7);
        }
        let expected = ledger[tick]["rng_draws"].as_array().unwrap();
        let mut lcg = rng;
        for (ordinal, draw) in expected.iter().enumerate() {
            lcg.rand();
            assert_eq!(
                lcg.seed,
                word(draw, "seed"),
                "tick {tick} ledger draw {ordinal}"
            );
        }
        // Interface s_link 17 follows the particle s_link 15 passes. Keep its
        // external RNG input separate and reject every other interleaving.
        let interface = expected
            .iter()
            .position(|d| matches!(word(d, "lr") - 4, 0x802f_4d44 | 0x802f_4d54))
            .unwrap_or(expected.len());
        let external = expected[..interface]
            .iter()
            .take_while(|d| !particle_draw(d))
            .count();
        assert!(
            expected[external..interface].iter().all(particle_draw),
            "tick {tick} external draws interleaved"
        );
        for draw in &expected[..external] {
            match word(draw, "pc") {
                0x8038_059c => {
                    rng.randi(100);
                }
                0x8038_054c => {
                    rng.randf();
                }
                pc => panic!("unknown RNG variant {pc:#x}"),
            }
        }
        let mut draws = DrawLog::default();
        spawns.before_main(tick, &mut system, &banks, &mut rng, &mut draws);
        if !(match_start && tick == 0) {
            system
                .proc_main::<RetailTrig>(&mut rng, &mut draws)
                .unwrap_or_else(|e| panic!("tick {tick} main: {e}"));
            system
                .proc_aux::<RetailTrig>(&mut rng, &mut draws)
                .unwrap_or_else(|e| panic!("tick {tick} aux: {e}"));
        }
        let sites = expected[external..interface]
            .iter()
            .map(|d| word(d, "lr") - 4)
            .collect::<Vec<_>>();
        for (ordinal, (actual, expected)) in draws.0.iter().zip(&sites).enumerate() {
            assert_eq!(actual, expected, "tick {tick} draw {ordinal}");
        }
        assert_eq!(draws.0.len(), sites.len(), "tick {tick} draw count");
        assert_eq!(
            (expected.len() - interface) % 8,
            0,
            "four HUD digits per player"
        );
        for (ordinal, draw) in expected[interface..].iter().enumerate() {
            assert_eq!(word(draw, "pc"), 0x8038_054c);
            assert_eq!(
                word(draw, "lr") - 4,
                [0x802f_4d44, 0x802f_4d54][ordinal % 2],
                "tick {tick} interface draw {ordinal}"
            );
            rng.randf();
        }
        particle_draws += draws.0.len();
        field_count += states[tick].state.len();
        assert_eq!(
            rng.seed,
            restore::uint(&ticks[tick], "rng.seed") as u32,
            "tick {tick} seed"
        );
        let actual = restore::snapshot(&system, rng.seed, tick as u64, &banks);
        assert_eq!(
            actual.state.len(),
            states[tick].state.len(),
            "tick {tick} field coverage"
        );
        for (field, expected) in &states[tick].state {
            if is_display_cache(field) {
                display_cache_fields += 1;
                continue;
            }
            if actual.state.get(field) != Some(expected) {
                let count = mismatches.entry(field.clone()).or_default();
                if *count == 0 {
                    eprintln!(
                        "first mismatch tick {tick} {field}: expected {expected:?}, actual {:?}",
                        actual.state.get(field)
                    );
                }
                *count += 1;
            }
        }
    }
    eprintln!(
        "Compared {} fields across {tick_count} ticks and {particle_draws} ordered particle draws; mismatches: {}; AppSRT display-cache fields skipped: {display_cache_fields}",
        field_count - display_cache_fields,
        mismatches.values().sum::<usize>()
    );

    assert!(mismatches.is_empty(), "mismatched fields: {mismatches:?}");
    eprintln!("{name} matched {tick_count}/{recording_ticks} ticks: {} fields, {particle_draws} ordered particle draws, all final seeds; final seed {:#010x}", field_count - display_cache_fields, rng.seed);
    if name == "dash_fd_fox" {
        assert!(
            display_cache_fields > 0,
            "the dump carries AppSRT display caches"
        );
    }
    display_cache_fields
}

pub fn common_bank(archive: &Archive) -> ParticleBank {
    // efAsync_LoadSync (efasync.c:1287-1316): effCommonDataTable begins with command/texture pointers.
    let table = archive.public("effCommonDataTable").unwrap();
    let commands = archive.link(table).unwrap().unwrap() as usize;
    let textures = archive.link(table + 4).unwrap().unwrap() as usize;
    ParticleBank::from_bytes(
        &archive.data()[commands..textures],
        &archive.data()[textures..],
    )
    .unwrap()
}
