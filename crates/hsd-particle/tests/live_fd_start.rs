mod common;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/start_fd_spawns.rs"]
mod spawns;
use common::RetailTrig;
use gekko_math::rng::HsdRng;
use hsd_archive::Archive;
use hsd_particle::{bank::ParticleBank, rng_sites::DrawLog};
use serde_json::Value as Json;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn word(draw: &Json, key: &str) -> u32 {
    draw[key].as_u64().unwrap().try_into().unwrap()
}
fn particle_draw(draw: &Json) -> bool {
    let site = word(draw, "lr") - 4;
    match site {
        0x8008_a8bc | 0x8009_fcdc | 0x8009_fd00 | 0x8009_fd24 | 0x8021_affc | 0x8021_aec8 => false,
        // Full symbol extents of interpreter, emitter, generator pass and constructor.
        0x8039_930c..=0x8039_ceab | 0x8039_dad4..=0x8039_f6cb => {
            assert_eq!(word(draw, "pc"), 0x8038_054c);
            true
        }
        _ => panic!("unclassified draw {site:#010x}"),
    }
}
#[test]
fn live_fd_start_600_ticks_match_every_field_and_rng_draw() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness");
    let paths = [
        "particles.jsonl.initial.jsonl",
        "particles.jsonl",
        "ledger600.raw.jsonl",
        "tick.expected.jsonl",
    ]
    .map(|s| root.join(format!("traces/start_fd_fox.{s}")));
    let archives = ["GrNLa.dat", "EfCoData.dat"].map(|s| root.join(format!("roms/files/{s}")));
    for path in paths.iter().chain(archives.iter()) {
        if !path.exists() {
            eprintln!("skipping FD start: {} absent", path.display());
            return;
        }
    }
    let stage = Archive::parse(&fs::read(&archives[0]).unwrap()).unwrap();
    let effect = Archive::parse(&fs::read(&archives[1]).unwrap()).unwrap();
    let banks = BTreeMap::from([
        (
            30,
            ParticleBank::from_archive(&stage, "map_ptcl", "map_texg").unwrap(),
        ),
        (0, spawns::common_bank(&effect)),
    ]);
    let initial = restore::read(&paths[0]);
    let states = restore::read(&paths[1]);
    let ledger: Vec<Json> = fs::read_to_string(&paths[2])
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let ticks = restore::read(&paths[3]);
    assert_eq!((states.len(), ledger.len(), ticks.len()), (600, 600, 600));
    assert_eq!(initial.len(), 1);
    let mut system = restore::restore(&initial[0], &banks);
    let mut rng = HsdRng::new(restore::uint(&initial[0], "rng.seed") as u32);
    let mut spawns = spawns::Spawns::new();
    let mut particle_draws = 0;
    let mut field_count = 0;
    for tick in 0..600 {
        assert_eq!(ticks[tick].frame, tick as u64);
        assert_eq!(ledger[tick]["frame"].as_u64(), Some(tick as u64));
        // Initial frames 0/1 share a VI; subsequent ticks have one display pass.
        if tick > 1 {
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
        let external = expected.iter().take_while(|d| !particle_draw(d)).count();
        assert!(
            expected[external..].iter().all(particle_draw),
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
        system
            .proc_main::<RetailTrig>(&mut rng, &mut draws)
            .unwrap_or_else(|e| panic!("tick {tick} main: {e}"));
        system
            .proc_aux::<RetailTrig>(&mut rng, &mut draws)
            .unwrap_or_else(|e| panic!("tick {tick} aux: {e}"));
        let sites = expected[external..]
            .iter()
            .map(|d| word(d, "lr") - 4)
            .collect::<Vec<_>>();
        for (ordinal, (actual, expected)) in draws.0.iter().zip(&sites).enumerate() {
            assert_eq!(actual, expected, "tick {tick} draw {ordinal}");
        }
        assert_eq!(draws.0.len(), sites.len(), "tick {tick} draw count");
        particle_draws += draws.0.len();
        field_count += states[tick].state.len();
        assert_eq!(
            rng.seed,
            restore::uint(&ticks[tick], "rng.seed") as u32,
            "tick {tick} seed"
        );
        restore::assert_state(
            &states[tick],
            &restore::snapshot(&system, rng.seed, tick as u64, &banks),
        );
    }
    eprintln!("FD start matched 600/600 ticks: {field_count} fields, {particle_draws} ordered particle draws, all final seeds; final seed {:#010x}", rng.seed);
}

#[test]
fn effect_animation_keys_identify_external_generator_requests() {
    use hsd_anim::fobj::FObj;
    use hsd_archive::desc::AnimJoint;
    fn tracks(node: &AnimJoint, output: &mut Vec<hsd_archive::desc::FObjDesc>) {
        for sibling in node.siblings() {
            if let Some(aobj) = &sibling.aobjdesc {
                output.extend(aobj.tracks().filter(|f| f.type_ == 0x28).cloned());
            }
            if let Some(child) = &sibling.child {
                tracks(child, output);
            }
        }
    }
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/EfCoData.dat");
    if !path.exists() {
        eprintln!("skipping: {} absent", path.display());
        return;
    }
    let archive = Archive::parse(&fs::read(path).unwrap()).unwrap();
    for (kind, expected) in [
        (0x24, vec![(0, 0, 445), (0, 0, 449), (9, 0, 448)]),
        (0x18, vec![(0, 0, 10)]),
    ] {
        let desc = archive.public("effCommonDataTable").unwrap() + 8 + kind * 20;
        let offset = archive.link(desc + 8).unwrap().expect("effect animation");
        let anim = AnimJoint::read(&archive, offset).unwrap();
        let mut all = Vec::new();
        tracks(&anim, &mut all);
        let mut events = Vec::new();
        for track in all {
            let mut f = FObj::new(
                &track.ad,
                track.startframe,
                track.type_,
                track.frac_value,
                track.frac_slope,
            );
            f.req_anim(0.0);
            for tick in 0..60 {
                f.interpret_anim(
                    Some(&mut |_, value: f32| {
                        let bits = value.to_bits();
                        events.push((tick, bits & 63, (bits >> 6) & 0xffffff));
                    }),
                    1.0,
                );
            }
        }
        events.sort();
        assert_eq!(events, expected, "effect {kind:#x} spawn keys");
    }
}
