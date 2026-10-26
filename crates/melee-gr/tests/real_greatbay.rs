//! Great Bay's models and parameters come from GrGb.dat.
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
fn great_bay_models_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrGb.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_greatbay(&archive).unwrap();
    // Ten models; the moon's (map 9) single animation does not loop, so its
    // fall ends on AOBJ_NO_ANIM (grgreatbay.c:834 stays unreachable).
    assert_eq!(stage.models.len(), 10);
    assert!(stage.models[9].present);
    assert_eq!(stage.models[9].animation_loops, [false]);
    let p = desc::read_greatbay_parameters(&archive).unwrap();
    assert_eq!(p.moon_fall_wait, [5000, 8000]);
    assert_eq!(p.float_landing_rate, 1.5);
    assert_eq!(p.turtle_wait, [1200, 2400]);
    assert_eq!(p.turtle_rebirth, [1200, 1800]);
    assert_eq!(p.turtle_scale, 0.85);
    assert_eq!(p.turtle_dir_prob, [4, 0, 2, 1]);
    assert_eq!(p.turtle_item_prob, 0.25);
    assert_eq!(p.turtle_items[..4], [(3, 5), (2, 3), (9, 1), (6, 3)]);
}
