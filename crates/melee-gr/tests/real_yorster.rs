//! Yoshi's Island's models and parameters come from GrYt.dat.
use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

fn archive(name: &str) -> Option<Vec<u8>> {
    let root = std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let path = root.join("harness/roms/files").join(name);
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

#[test]
fn yoshis_island_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrYt.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_yorster(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Yorster);
    // Map 1 is the stage: its model loop (4), bones 23 and 26's loop (3)
    // and the blocks' spin, rest and wobble (0, 1, 2). Collision joints 1..9
    // sit on the blocks' bones 10..18; joint 0 never moves.
    let animations: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(animations, [0, 5]);
    assert_eq!(stage.models[1].animation_loops, [false, false, true, true, true]);
    let bindings: Vec<_> = stage.models[1]
        .joint_mappings
        .iter()
        .map(|m| (m.joint_index, m.extra))
        .collect();
    assert_eq!(bindings, (1..=9).map(|j| (j, j + 9)).collect::<Vec<_>>());
    assert_eq!(stage.parameters.map_scale, 0.85);
    let p = desc::read_yorster_parameters(&archive).unwrap();
    assert_eq!(p.bump_speed.to_bits(), 0.8_f32.to_bits());
    assert_eq!((p.bump_full, p.damage_threshold, p.damage_full), (3.0, 1.0, 30.0));
    assert_eq!((p.bump_freeze, p.hit_freeze, p.flip_delay, p.spin_end_frame), (2, 5, 3, 140));
}
