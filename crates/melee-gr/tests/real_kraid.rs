//! Brinstar Depths' models and parameters come from GrKr.dat.
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
fn brinstar_depths_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrKr.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_kraid(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Kraid);
    // Kraid and his effects play seven animations each; the stage three
    // (the still loop, the shake, bone 13's loop). The stage's own joint
    // table (grKr_803E4C78) binds its collision, not the archive.
    let animations: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(animations, [0, 7, 0, 3, 7]);
    assert_eq!(stage.models[3].animation_loops, [true, false, true]);
    assert!(stage.models.iter().all(|m| m.joint_mappings.is_empty()));
    let p = desc::read_kraid_parameters(&archive).unwrap();
    assert_eq!((p.map_time_min, p.map_time_max, p.map_time_acl), (150, 240, 180));
    assert_eq!(p.map_rot_spd_min.to_bits(), 0.2_f32.to_bits());
    assert_eq!(p.map_rot_spd_max.to_bits(), 0.3_f32.to_bits());
    assert_eq!((p.kraid_wait_time, p.kraid_wait_time_add), (120, 300));
    assert_eq!(p.kraid_pos_x, [-60.0, -30.0, 0.0, 30.0, 60.0, 0.0]);
}
