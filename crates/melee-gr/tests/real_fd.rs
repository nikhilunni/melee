use gekko_math::HsdRng;
use hsd_archive::Archive;
use melee_gr::{
    desc,
    last::{AnimationStatus, FinalDestination},
};
use melee_types::snapshot::Snapshot;
use std::{collections::BTreeMap, path::PathBuf};

fn archive() -> Option<Archive> {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrNLa.dat");
    match std::fs::read(file) {
        Ok(bytes) => Some(Archive::parse(&bytes).expect("real FD archive")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping real FD: harness/roms/files/GrNLa.dat is absent");
            None
        }
        Err(error) => panic!("read GrNLa.dat: {error}"),
    }
}

#[test]
fn real_fd_archive_models_parameters_and_collision() {
    let Some(archive) = archive() else {
        return;
    };
    let desc = desc::read_final_destination(&archive).unwrap();
    assert_eq!(archive.publics().len(), 27);
    assert_eq!(desc.section_counts, [1, 10, 2, 32, 3, 1]);
    assert_eq!(
        desc.models
            .iter()
            .map(|m| m.joint.descendants().len())
            .collect::<Vec<_>>(),
        [22, 1, 1, 5, 17, 9, 5, 6, 5, 24]
    );
    assert_eq!(desc.parameters.map_scale.to_bits(), 1.0_f32.to_bits());
    assert!(!desc.parameters.fixed_camera);
    assert_eq!(
        desc.parameters.stage_param_count, 13,
        "count word at grGroundParam+0xB4 on the retail disc"
    );
    assert_eq!(desc.position_bindings.len(), 21);
    assert_eq!(desc.position_bindings[0].joint_index, 1);
    assert_eq!(desc.position_bindings[0].stage_position, 148);
    assert_eq!(desc.initial_fog, [0, 0, 0]);
    desc::load_collision(&archive, &desc).unwrap();
}

/// One tick's snapshot: field name and value pairs.
type TickSnapshot = Vec<(String, melee_types::snapshot::SnapValue)>;

fn direct_trace(desc: &desc::StageDesc) -> (Vec<u32>, Vec<TickSnapshot>, u32) {
    let mut rng = HsdRng::new(1);
    let mut stage = FinalDestination::initialize(desc, &mut rng);
    assert_eq!(rng.seed, 3_884_216_597); // four map-7 init draws
    stage.ground.start();
    stage.actions.clear();
    let mut counts = Vec::new();
    let mut states = Vec::new();
    for _ in 0..600 {
        let mut draws = 0;
        for map in 0..10 {
            draws += stage.run_stage_proc(map, &AnimationStatus::default(), &mut rng);
        }
        counts.push(draws);
        let mut snapshot = Vec::new();
        stage.ground.snapshot(&mut snapshot);
        states.push(snapshot);
        assert!(
            stage.actions.is_empty(),
            "first 600 direct ticks do not change animation/phase"
        );
    }
    (counts, states, rng.seed)
}

#[test]
fn real_fd_600_direct_ticks_are_deterministic() {
    let Some(archive) = archive() else {
        return;
    };
    let desc = desc::read_final_destination(&archive).unwrap();
    let first = direct_trace(&desc);
    assert_eq!(first, direct_trace(&desc));
    let mut histogram = BTreeMap::new();
    for &draws in &first.0 {
        *histogram.entry(draws).or_insert(0) += 1;
    }
    eprintln!(
        "FD direct grlast draws per tick (seed=1, cold init): {:?}",
        first.0
    );
    eprintln!(
        "FD direct histogram: {histogram:?}; final seed: {:#010x}",
        first.2
    );
}
