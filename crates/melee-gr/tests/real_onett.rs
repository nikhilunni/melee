//! Onett's models and parameters come from GrOt.dat.
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
fn onett_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrOt.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_onett(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Onett);
    // Map 1 is the warning sign's model (un_802FD8A0); map 4, the roof,
    // plays six animations; the awnings (joints 0, 1) and the roof (3, 4)
    // are the only moving collision.
    let animations: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(animations, [0, 1, 1, 1, 6, 0]);
    let joints = |map: usize| -> Vec<(i16, i16)> {
        stage.models[map]
            .joint_mappings
            .iter()
            .map(|m| (m.joint_index, m.extra))
            .collect()
    };
    assert_eq!(joints(4), [(3, 1), (4, 3)]);
    assert_eq!(joints(5), [(0, 14), (1, 15)]);
    assert_eq!(stage.parameters.map_scale, 0.9);
    let p = desc::read_onett_parameters(&archive).unwrap();
    assert_eq!((p.awning_initial, p.awning_delta), (-1.5, -2.0));
    assert_eq!((p.car_wait, p.car_announce, p.car_delay_middle, p.car_delay_last), (450, 400, 16, 30));
    assert_eq!((p.roof_crack, p.roof_break), (5.0, 7.0));
    assert_eq!((p.warning_ticks, p.horn_lead), (60, 320.0));
}
