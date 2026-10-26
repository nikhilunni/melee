//! Mushroom Kingdom II's models and parameters come from GrI2.dat.
use hsd_archive::Archive;
use hsd_types::Vec3;
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
fn mushroom_kingdom_2_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrI2.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_inishie2(&archive).unwrap();
    assert_eq!(stage.parameters.map_scale, 0.7);
    let p = desc::read_inishie2_parameters(&archive).unwrap();
    assert_eq!(p.lift_waits, [[50, 450], [600, 300]]);
    assert_eq!(p.birdo_wait, [600, 600]);
    assert_eq!(p.carpet_wait, [600, 600]);
    assert_eq!(p.lift_delays, [265, 410]);
    assert_eq!(p.birdo_spawn, [Vec3::new(30.0, 28.5, 0.0), Vec3::new(-30.0, 28.5, 0.0)]);
    assert_eq!(p.birdo_platform_height, 0.0);
    assert_eq!(p.carpet_spawn, [Vec3::new(30.0, 80.0, 0.0), Vec3::new(-30.0, 80.0, 0.0)]);
    assert_eq!(p.carpet_turn_odds, [5, 3]);
}
