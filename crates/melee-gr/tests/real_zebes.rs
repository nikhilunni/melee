//! Brinstar's models and parameters come from GrZe.dat.
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
fn brinstar_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrZe.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_zebes(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Zebes);
    // Map 1's shakes, map 4's two turns, map 5's splash, the parts' idle
    // loop, crack, growth, sinking and rising; grZe_803E1A10 binds the
    // collision, not the archive.
    let animations: Vec<_> = stage.models.iter().map(|m| m.animations.len()).collect();
    assert_eq!(animations, [0, 3, 0, 0, 2, 1, 5, 5, 0, 0]);
    assert!(stage.models.iter().all(|m| m.joint_mappings.is_empty()));
    let p = desc::read_zebes_parameters(&archive).unwrap();
    assert_eq!(p.island_wait, [600.0, 600.0]);
    assert_eq!(p.pulse_wait, [200.0, 300.0]);
    assert_eq!(p.regrowth_step, 15);
    assert_eq!(p.plate_break_damage, 30.0);
    assert_eq!((p.acid_top, p.acid_bottom), (90.0, -320.0));
    let levels: Vec<_> = p
        .acid_levels
        .iter()
        .take_while(|l| !l.is_end())
        .map(|l| l.level)
        .collect();
    assert_eq!(levels.len(), 18);
    assert_eq!((levels[0], levels[17]), (-140, -250));
    // The acid's hit (lbColl_80008D30_arg1): 14 damage, 90 degrees, fire.
    let r = archive.reader();
    let words: Vec<u32> = (0..8).map(|i| r.u32(p.acid_hit + 4 * i).unwrap()).collect();
    assert_eq!(words, [1, 14, 90, 35, 0, 110, 1, 1]);
}
