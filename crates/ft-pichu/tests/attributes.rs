mod support;
use ft_pichu::{
    attributes::{read_pichu_attributes, PikachuAttributes},
    init::{Pichu, DESCRIPTOR},
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn attributes_come_from_ftdatapichu() {
    let mut data = vec![0; 0x100];
    word(&mut data, 0xDC, 0x52);
    let source = archive(&data, &[0xFC], Some(("ftDataPichu", 0xF8)));
    assert_eq!(
        read_pichu_attributes(&source).unwrap().thunder.bolt_item,
        0x52
    );
    let pikachu = archive(&data, &[0xFC], Some(("ftDataPikachu", 0xF8)));
    assert!(read_pichu_attributes(&pikachu).is_err());
}

#[test]
fn load_sets_walljump_and_every_special() {
    let attrs = PikachuAttributes::read(&archive(&[0; 0xF8], &[], None), 0).unwrap();
    let mut pichu = Pichu::new(attrs.clone());
    let mut capabilities = Capabilities::default();
    pichu.on_load(&mut capabilities);
    assert_eq!(pichu.kind(), FighterKind::Pichu);
    assert!(capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(pichu.attributes, attrs);
}

#[test]
fn death_shows_only_the_costume_accessory() {
    let attrs = PikachuAttributes::read(&archive(&[0; 0xF8], &[], None), 0).unwrap();
    let source = archive(&[0; 4], &[], None);
    for (costume, groups) in [
        (0, [0, -1, -1, -1]),
        (1, [0, 0, -1, -1]),
        (2, [0, -1, 0, -1]),
        (3, [0, -1, -1, 0]),
    ] {
        let mut pichu = Pichu::new(attrs.clone());
        pichu.on_costume_loaded(&source, costume).unwrap();
        pichu.model_groups = [5; 4];
        pichu.on_reset();
        assert_eq!(pichu.model_groups, groups, "costume {costume}");
    }
}

#[test]
fn disc_attributes_and_parts() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [files.join(DESCRIPTOR.data_file), files.join("PlCo.dat")];
    if !melee_test_support::require_files(paths.iter()) {
        return;
    }
    let source = hsd_archive::Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap();
    let attrs = read_pichu_attributes(&source).unwrap();
    // ftPc_Init_OnLoad registers these kinds (it_8026B3F8).
    assert_eq!(attrs.thunder_jolt.ground_item, 0x5B);
    assert_eq!(attrs.thunder_jolt.air_item, 0x5C);
    assert_eq!(attrs.thunder.bolt_item, 0x52);
    let root = source.public(DESCRIPTOR.data_symbol).unwrap();
    let bones =
        melee_ft::desc::read_fighter_bones(&source, root, DESCRIPTOR.part_animation_count).unwrap();
    assert_eq!(
        bones.animation_sets.iter().flatten().count(),
        DESCRIPTOR.part_animation_count
    );
    let common = hsd_archive::Archive::parse(&std::fs::read(&paths[1]).unwrap()).unwrap();
    let parts =
        melee_ft::desc::read_part_table(&common, DESCRIPTOR.kind, DESCRIPTOR.part_count).unwrap();
    assert_eq!(parts.part_to_joint.len(), DESCRIPTOR.part_count as usize);
    assert_eq!(parts.joint_count(), 46);
}
