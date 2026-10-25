//! GrFz.dat's tool article: its hitbox commands decide which of the shared
//! item events can reach a tool.
use melee_it::desc::ItemAssets;
use melee_types::ItemKind;
use std::path::PathBuf;

fn archive() -> Option<hsd_archive::Archive> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let path = root.join("harness/roms/files/GrFz.dat");
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(hsd_archive::Archive::parse(&std::fs::read(&path).unwrap()).unwrap())
}

#[test]
fn tool_hitboxes_cannot_be_reflected_absorbed_or_glance_off_shields() {
    let Some(archive) = archive() else {
        return;
    };
    let assets = ItemAssets::from_stage_item(
        &archive,
        ItemKind::Tools,
        &it_tools::ARTICLE_STATES,
        it_tools::SPECIAL_ATTRIBUTES,
        &[],
    )
    .unwrap()
    .expect("GrFz.dat itemdata has the tools");
    // The falling states' hitbox (it_802790C0's flag word): neither a
    // reflector, an absorber nor a glancing shield can take it, so
    // itTools_Logic22_Reflected, it_2725_Logic22_Absorbed and
    // itTools_Logic22_ShieldBounced cannot run.
    for state in 0..5 {
        let flags = assets.hit_flags[state]
            .iter()
            .flatten()
            .next()
            .expect("a falling tool's hitbox");
        assert!(!flags.reflectable && !flags.absorbable && !flags.shield_bounce);
        assert!(flags.shieldable && flags.hits_hurtboxes && !flags.hits_items);
    }
    // Landed tools spawn no hitbox.
    for state in 5..10 {
        assert!(assets.hit_flags[state].iter().all(Option::is_none));
    }
    // itToolsAttributes: lifetimes 180 and 30, damage x2 capped at 50, and
    // the first tool's motion row.
    let a = &assets.special_attributes;
    assert_eq!((a[0], a[1], a[2], a[3].to_bits()), (180.0, 30.0, 2.0, 50));
    assert_eq!(&a[4..11], &[1.1, 0.02, 1.5, 0.174_532_92, 0.5, 4.0, 0.1]);
}
