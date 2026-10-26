//! Icicle Mountain's models and parameters come from GrIm.dat.
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
fn icicle_mountain_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrIm.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_icemt(&archive).unwrap();
    // Maps 0..8: the base, six field segments, the block model, the
    // background; map 9 (the controller) is an identity JObj.
    assert_eq!(stage.models.len(), 9);
    // Only maps 2 and 4 (the platforms that give way) animate.
    let animated: Vec<usize> = (0..9).filter(|&m| !stage.models[m].animations.is_empty()).collect();
    assert_eq!(animated, [2, 4]);
    // The archive binds the blocks' and the platforms' joints.
    let map2: Vec<(i16, i16)> = stage.models[2]
        .joint_mappings
        .iter()
        .map(|j| (j.joint_index, j.extra))
        .collect();
    assert_eq!(map2, [(34, 1), (35, 2), (37, 4), (38, 5)]);
    assert!(stage.parameters.fixed_camera);
    assert_eq!(stage.parameters.map_scale, 0.7);
    let p = desc::read_icemt_parameters(&archive).unwrap();
    assert_eq!(p.segment_cooldown, 1);
    assert_eq!(p.speed_cooldown, 3);
    assert_eq!(
        p.speeds,
        [-0.15, -0.1, 0.0, 0.03, 0.07, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.8]
    );
    assert_eq!(p.ease_frames, 60);
    assert_eq!(p.hold, [360, 900]);
    assert_eq!(p.segment_gap, 25.0);
}
