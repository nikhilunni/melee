//! Corneria's models and parameters come from GrCn.dat.
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
fn corneria_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrCn.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_corneria(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Corneria);
    assert_eq!(stage.parameters.map_scale, 0.54);
    // Eleven models: the paths' eleven animations (map 1), the Arwing's
    // and Wolfen's six (maps 2 and 10), the Great Fox's loop (map 3) and
    // the city's (map 8, which its init never plays). Maps 11..18 are
    // identity roots; grCn_803E1D38 binds the collision, not the archive.
    let animations: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(animations, [0, 11, 6, 1, 0, 3, 1, 0, 1, 0, 6]);
    assert!(stage.models.iter().all(|m| m.joint_mappings.is_empty()));
    // Every marker is on map 0, which rides the Great Fox.
    assert!(stage.position_bindings.iter().all(|b| b.model_id == 0));
    let p = desc::read_corneria_parameters(&archive).unwrap();
    assert_eq!(p.drift_wait, [30.0, 180.0]);
    assert_eq!((p.drift_accel, p.drift_speed), (0.002, 0.1));
    assert_eq!(p.drift_limit, [20.0, 20.0]);
    assert_eq!((p.cannon_rest_range, p.cannon_rest, p.cannon_aim), (1200.0, 1200.0, 180.0));
    assert_eq!(p.cannon_volley_shots, [8.0, 12.0]);
    assert_eq!(p.cannon_health, 200.0);
    assert_eq!((p.first_wait, p.wait), ([600.0, 600.0], [960.0, 1800.0]));
    assert_eq!((p.wolfen_chance, p.ship_scale), (0.15, 0.5));
    assert_eq!((p.fire_period, p.fire_odds), (16, 8));
    assert_eq!(p.scroll_speed, 2.0);
}
