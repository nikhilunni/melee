//! Princess Peach's Castle's models and parameters come from GrCs.dat.
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
fn castle_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrCs.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_castle(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Castle);
    // Map 2 (the cannon) plays four animations, each flight two; only map
    // 6 binds collision (the platforms, joints 4 and 5).
    let animations: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(
        animations,
        [0, 1, 4, 0, 1, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 1, 1, 1]
    );
    let joints: Vec<(i16, i16)> = stage.models[6]
        .joint_mappings
        .iter()
        .map(|m| (m.joint_index, m.extra))
        .collect();
    assert_eq!(joints, [(4, 1), (5, 4)]);
    assert!(stage.models.iter().enumerate().all(|(i, m)| i == 6 || m.joint_mappings.is_empty()));
    assert_eq!(stage.parameters.map_scale, 0.5);
    let p = desc::read_castle_parameters(&archive).unwrap();
    assert_eq!((p.switch_wait, p.switch_freeze, p.switch_repeat_divisor), ([60, 800], 16, 6));
    assert_eq!((p.first_bill_wait, p.next_bill_wait), ([1200, 4200], [1800, 2400]));
    assert_eq!((p.platform_top, p.platform_bottom), (280.0, 205.0));
    assert_eq!((p.blocks_time, p.blocks_blink), ([1200, 1200], 60));
    assert_eq!((p.switch_linger, p.bill_fuse), (60, 60));
    assert_eq!(p.cannon_waits, [20, 30, 50, -1]);
    let delays: Vec<i32> = p.flights.iter().map(|f| f.cannon_delay).collect();
    assert_eq!(delays, [405, 600, 600, 720, 575, 720, 575, 600, 600]);
}
