//! Fourside's models and parameters come from GrFs.dat.
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
fn fourside_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrFs.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_fourside(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Fourside);
    // The crane's two swings, the helicopter's three animations and the
    // UFO's four per building; no archive collision bindings (the stage's
    // own table, grFs_803E3CE8, binds them) and no particle bank.
    let counts: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(counts, [0, 2, 1, 3, 0, 12, 0]);
    assert!(stage.models.iter().all(|m| m.joint_mappings.is_empty()));
    assert!(archive.public("map_ptcl").is_none());
    let p = desc::read_fourside_parameters(&archive).unwrap();
    assert_eq!((p.heli_wait, p.heli_wait_add), (600, 900));
    assert_eq!((p.crane_wait, p.crane_wait_add), (900, 600));
    assert_eq!((p.crane_iron_wait, p.crane_iron_wait_add), (600, 300));
    assert_eq!(p.crane_iron_up, [5.0, 10.0]);
    assert_eq!(p.crane_iron_down, [-5.0, -8.0]);
    assert_eq!(p.crane_iron_speed.to_bits(), 0.2_f32.to_bits());
    assert_eq!(p.crane_iron_stop_acceleration.to_bits(), (-0.08_f32).to_bits());
    assert_eq!((p.ufo_wait, p.ufo_stay_time, p.ufo_stay_time_add), (300, 600, 1200));
    assert_eq!(p.ufo_camera_offset, 30.0);
    assert_eq!(
        (p.ufo_challenge, p.ufo_standing_odds, p.ufo_challenged_odds),
        (4, 4, 2)
    );
}
