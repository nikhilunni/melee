//! Cold-start FD particle simulation, deliberately independent of the idle
//! savestate's unknown generator accumulator and particles already in flight.
use gekko_math::rng::HsdRng;
use hsd_anim::{
    jobj::{JObjEvent, JObjId, JObjTree},
    load::{attach_anim_joint, load_joint_tree},
    mtx::InverseTrig,
};
use hsd_archive::{
    desc::{AnimJoint, JObjDesc},
    Archive,
};
use hsd_particle::{
    bank::ParticleBank,
    rng_sites::{DrawLog, EMISSION_COUNT, FD_EMISSION},
    system::{ParticleSystem, SpawnRequest},
};
use std::path::PathBuf;

struct RetailTrig;
impl InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}

fn archive() -> Option<Archive> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrNLa.dat");
    match std::fs::read(&path) {
        Ok(bytes) => Some(Archive::parse(&bytes).expect("parse GrNLa.dat")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping real FD particles: {} is absent", path.display());
            None
        }
        Err(error) => panic!("read {}: {error}", path.display()),
    }
}

fn model_four(archive: &Archive) -> (JObjTree, JObjId) {
    // gr/types.h Ground_ModelDesc: map_head's model array is at +8,
    // stride 0x34; joint at +0, AnimJoint pointer list at +4. grlast.c
    // starts model set 4 with animation 0. Its sole DPtcl key targets
    // depth-first joint 16 (the bank/track test checks the raw descriptors).
    let header = archive.public("map_head").unwrap();
    let models = archive.link(header + 8).unwrap().unwrap();
    let model = models + 4 * 0x34;
    let joint = JObjDesc::read(archive, archive.link(model).unwrap().unwrap()).unwrap();
    let animation_list = archive.link(model + 4).unwrap().unwrap();
    let animation =
        AnimJoint::read(archive, archive.link(animation_list).unwrap().unwrap()).unwrap();
    let (mut tree, root) = load_joint_tree(archive, &joint).unwrap();
    attach_anim_joint(&mut tree, root, &animation, archive).unwrap();
    tree.req_anim_all(root, 0.0);
    (tree, root)
}

#[derive(Debug, PartialEq, Eq)]
struct ColdTrace {
    draws: Vec<usize>,
    emissions: Vec<usize>,
    population: Vec<usize>,
    final_seed: u32,
}

fn run_cold(archive: &Archive, ticks: usize) -> ColdTrace {
    let bank = ParticleBank::from_archive(archive, "map_ptcl", "map_texg").unwrap();
    let (mut tree, root) = model_four(archive);
    let mut system = ParticleSystem::default();
    let mut rng = HsdRng::new(1);
    let mut attached_generators = Vec::new();
    let mut trace = ColdTrace {
        draws: Vec::new(),
        emissions: Vec::new(),
        population: Vec::new(),
        final_seed: 0,
    };
    for tick in 0..ticks {
        tree.anim_all::<RetailTrig>(root);
        for event in std::mem::take(&mut tree.events) {
            if let JObjEvent::DPtcl { jobj, lo, hi } = event {
                assert_eq!((tick, lo, hi), (0, 30, 30000));
                assert_eq!(tree.depth_first(root).position(|id| id == jobj), Some(16));
                // hsd_8039F05C consumes a separate creation draw; it is not
                // part of the ledger's steady-state 1+6*emissions pattern.
                let mut creation = DrawLog::default();
                let id = system
                    .spawn::<RetailTrig>(
                        &bank,
                        SpawnRequest::new(lo as u8, hi as u32, 0),
                        &mut rng,
                        &mut creation,
                    )
                    .unwrap()
                    .unwrap();
                assert_eq!(creation.0, [0x8039_f250]);
                attached_generators.push((id, jobj));
            }
        }
        assert_eq!(attached_generators.len(), 1);
        for &(id, joint) in &attached_generators {
            tree.setup_matrix(joint);
            system
                .generator_mut(id)
                .unwrap()
                .attach_joint(tree.get(joint).mtx);
        }
        let mut draws = DrawLog::default();
        system
            .proc_main::<RetailTrig>(&mut rng, &mut draws)
            .unwrap();
        let after_main = draws.0.len();
        system.proc_aux::<RetailTrig>(&mut rng, &mut draws).unwrap();
        assert_eq!(
            draws.0.len(),
            after_main,
            "link 0 is skipped by the aux proc"
        );
        // Creation performs an immediate update, leaving life=4. Every old
        // particle is updated before generators, so life=4 identifies the
        // newly emitted cohort without deriving it from RNG observations.
        let emitted = system.particles[0].iter().filter(|p| p.life == 4).count();
        assert_eq!(draws.0.len(), 1 + 6 * emitted, "tick {tick}");
        assert_eq!(draws.0[0], EMISSION_COUNT);
        for sites in draws.0[1..].chunks_exact(6) {
            assert_eq!(sites, FD_EMISSION);
        }
        trace.draws.push(draws.0.len());
        trace.emissions.push(emitted);
        trace.population.push(system.live_particles());
        let cohorts = &trace.emissions[tick.saturating_sub(3)..=tick];
        assert_eq!(
            system.live_particles(),
            cohorts.iter().sum::<usize>(),
            "four-frame lifetime at tick {tick}"
        );
        for age in 0..=tick.min(3) {
            let count = system.particles[0]
                .iter()
                .filter(|p| usize::from(p.life) == 4 - age)
                .count();
            assert_eq!(
                count,
                trace.emissions[tick - age],
                "cohort age {age} at tick {tick}"
            );
        }
        assert_eq!(system.generators[0].remaining_life, 3998 - tick as u16);
        assert_eq!(
            system.generators[0].children as usize,
            system.live_particles()
        );
    }
    trace.final_seed = rng.seed;
    trace
}

#[test]
fn cold_fd_animation_drives_six_rng_sites_per_emission_and_four_tick_lifetimes() {
    let Some(archive) = archive() else {
        return;
    };
    let trace = run_cold(&archive, 120);
    assert_eq!(trace, run_cold(&archive, 120));
    assert!(
        trace
            .population
            .iter()
            .zip(&trace.emissions)
            .any(|(live, emitted)| live > emitted),
        "steady-state draws count emissions, not all live particles"
    );
    eprintln!(
        "FD cold start seed=1 (creation draw separate), per-tick RNG draws: {:?}",
        trace.draws
    );
    eprintln!("FD newly emitted particles: {:?}", trace.emissions);
    eprintln!("FD live particles after both procs: {:?}", trace.population);
    eprintln!(
        "FD final RNG seed: {:#010x}; no savestate ledger equality claimed",
        trace.final_seed
    );
}
