//! GrI2.dat's Birdo and egg articles.
use melee_it::desc::ItemAssets;
use melee_types::ItemKind;
use std::path::PathBuf;

fn archive() -> Option<hsd_archive::Archive> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let path = root.join("harness/roms/files/GrI2.dat");
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(hsd_archive::Archive::parse(&std::fs::read(&path).unwrap()).unwrap())
}

#[test]
fn birdo_and_her_egg_come_from_the_stage_archive() {
    let Some(archive) = archive() else {
        return;
    };
    let birdo = ItemAssets::from_stage_item(
        &archive,
        ItemKind::Kyasarin,
        &it_kyasarin::ARTICLE_STATES,
        it_kyasarin::SPECIAL_ATTRIBUTES,
        &[],
    )
    .unwrap()
    .expect("GrI2.dat itemdata has Birdo");
    let egg = ItemAssets::from_stage_item(
        &archive,
        ItemKind::KyasarinEgg,
        &it_kyasarin::egg::ARTICLE_STATES,
        it_kyasarin::egg::SPECIAL_ATTRIBUTES,
        &[],
    )
    .unwrap()
    .expect("GrI2.dat itemdata has the egg");
    assert_eq!(birdo.special_attributes.len(), it_kyasarin::SPECIAL_ATTRIBUTES as usize);
    assert_eq!(egg.special_attributes.len(), it_kyasarin::egg::SPECIAL_ATTRIBUTES as usize);
    assert_eq!(birdo.animation_ends.len(), 4);
    assert!(!egg.animation_ends.is_empty());
}
