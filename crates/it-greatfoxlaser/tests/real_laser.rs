//! GrCn.dat's Great Fox laser article: its attributes and its hitbox.
use std::path::PathBuf;

fn archive() -> Option<hsd_archive::Archive> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let path = root.join("harness/roms/files/GrCn.dat");
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(hsd_archive::Archive::parse(&std::fs::read(&path).unwrap()).unwrap())
}

#[test]
fn the_great_fox_lasers_attributes_and_hitbox() {
    let Some(archive) = archive() else {
        return;
    };
    let assets = it_greatfoxlaser::load(&archive)
        .unwrap()
        .expect("GrCn.dat's Great Fox laser");
    // The big shot: 50 damage at 20 a tick; a volley's shot: 10 at 20.
    let a = &assets.special_attributes;
    assert_eq!((a[0], a[1], a[2], a[3]), (50.0, 20.0, 10.0, 20.0));
    // Both states' animations play seven frames (the charge).
    assert_eq!(assets.animation_ends[..2], [Some(7.0), Some(7.0)]);
    // One hitbox; it can be reflected, absorbed and shielded, and only the
    // reflected state's strikes items.
    let charged = assets.hit_flags[0].iter().flatten().next().unwrap();
    let turned = assets.hit_flags[1].iter().flatten().next().unwrap();
    assert!(charged.reflectable && charged.absorbable && charged.shieldable);
    assert!(!charged.hits_items && turned.hits_items);
}
