//! Green Greens' models and parameters come from GrGr.dat.
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
fn greens_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrGr.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_greens(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Greens);
    // Whispy (map 5) and his face (map 4) play the same seventeen
    // animations; map 6's thirty block slots are joints 0..29 on bones 5..34.
    let animations: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(animations, [0, 0, 1, 0, 17, 17, 0]);
    let slots: Vec<(i16, i16)> = stage.models[6]
        .joint_mappings
        .iter()
        .map(|m| (m.joint_index, m.extra))
        .collect();
    assert_eq!(slots, (0..30).map(|i| (i, i + 5)).collect::<Vec<_>>());
    assert_eq!(stage.parameters.map_scale, 0.75);
    let p = desc::read_greens_parameters(&archive).unwrap();
    assert_eq!((p.block_wait, p.bomb_odds), ([30, 150], 12));
    assert_eq!((p.column_weights, p.empty_column_weight), ([20, 10, 5, 1, 0], 40));
    assert_eq!((p.idle_wait, p.wind_speed), ([800, 1800], 0.24));
    assert_eq!((p.apple_count, p.apple_start, p.apple_interval), ([2, 5], 40, 16));
    assert_eq!((p.apple_kept, p.apple_heal), (0.7, 0.2));
}
