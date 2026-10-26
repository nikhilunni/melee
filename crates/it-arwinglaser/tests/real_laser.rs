//! GrVe.dat's Arwing laser article: its attributes and the hit flags that
//! decide which shared item events can reach an aimed laser.
use std::path::PathBuf;

fn archive() -> Option<hsd_archive::Archive> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let path = root.join("harness/roms/files/GrVe.dat");
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(hsd_archive::Archive::parse(&std::fs::read(&path).unwrap()).unwrap())
}

#[test]
fn the_aimed_lasers_attributes_and_hitboxes() {
    let Some(archive) = archive() else {
        return;
    };
    let assets = it_arwinglaser::load(&archive).unwrap().expect("GrVe.dat's laser");
    // x0's x4_throw_speed_mul 4 times x4 5: 20 units a tick; x8 6 thick.
    let a = &assets.special_attributes;
    assert_eq!((a[0], a[1], a[2]), (4.0, 5.0, 6.0));
    // The aimed lasers' hitbox (on the tip, bone 4) can be reflected,
    // absorbed, shielded and glance off a shield: those events are guarded
    // until a witness reaches them.
    for state in [1, 3] {
        let flags = assets.hit_flags[state].iter().flatten().next().unwrap();
        assert!(flags.reflectable && flags.absorbable && flags.shieldable && flags.shield_bounce);
    }
    // The tip is the fifth joint of a five-joint chain.
    let pose = assets.pose.as_ref().unwrap();
    assert_eq!(pose.parents, [None, Some(0), Some(1), Some(2), Some(3)]);
}
