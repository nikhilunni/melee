mod support;
use ft_younglink::{
    attributes::{read_young_link_attributes, LinkAttributes},
    init::{YoungLink, DESCRIPTOR},
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_zero_preserves_float_payloads_and_integer_signedness() {
    // ftData at +DC: ext_attr (+4) points at offset zero; x48_items (+48,
    // at +124) at the item table (+128), whose [2] (+130) is the hookshot
    // article (+134), with its special attributes (+4, at +138) at +13C.
    let mut data = vec![0; 0x190];
    word(&mut data, 0, 0x8000_0000);
    word(&mut data, 0x124, 0x128);
    word(&mut data, 0x130, 0x134);
    word(&mut data, 0x138, 0x13C);
    word(&mut data, 0x13C + 0x10, 0x4000_0000);
    let relocs = [0xE0, 0x124, 0x130, 0x138];
    word(&mut data, 0x2C, 0x3C);
    word(&mut data, 0xC4, (-2_i32) as u32);
    word(&mut data, 0xD8, 0x7FC0_1234);
    let source = archive(&data, &relocs, Some(("ftDataClink", 0xDC)));
    let attrs = read_young_link_attributes(&source).unwrap();
    assert_eq!(attrs.bow.max_charge.to_bits(), 0x8000_0000);
    assert_eq!(attrs.boomerang.item, 0x3C);
    assert_eq!(attrs.shield.bone, -2);
    assert_eq!(attrs.shield.damage_scale.to_bits(), 0x7FC0_1234);
    assert_eq!(attrs.hookshot_article[4], 2.0);
    assert!(read_young_link_attributes(&archive(&data, &[], Some(("ftDataClink", 0xDC)))).is_err());
    assert!(
        read_young_link_attributes(&archive(&data, &relocs, Some(("ftDataFox", 0xDC)))).is_err()
    );
    assert!(LinkAttributes::read(&archive(&data[..0xDB], &[], None), 0).is_err());
}

#[test]
fn load_and_death_reset_model_groups_and_articles() {
    let attrs = LinkAttributes::read(&archive(&[0; 0xDC], &[], None), 0).unwrap();
    let mut young_link = YoungLink::new(attrs.clone());
    let mut capabilities = Capabilities::default();
    young_link.on_load(&mut capabilities);
    assert_eq!(young_link.kind(), FighterKind::CLink);
    assert!(capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    young_link.model_groups = [2, 3, 1];
    young_link.specials.vars.used_boomerang = true;
    young_link.on_reset();
    assert_eq!(young_link.model_groups, [0, 0, 0]);
    assert!(!young_link.specials.vars.used_boomerang);
    assert_eq!(young_link.attributes, attrs);
}

#[test]
fn disc_attributes_parts_and_shield_bone() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [files.join(DESCRIPTOR.data_file), files.join("PlCo.dat")];
    if !melee_test_support::require_files(paths.iter()) {
        return;
    }
    let source = hsd_archive::Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap();
    let attrs = read_young_link_attributes(&source).unwrap();
    // ftLk_Init_OnLoad registers these kinds (it_8026B3F8).
    assert_eq!(
        [
            attrs.bomb_item,
            attrs.boomerang.item,
            attrs.hookshot_item,
            attrs.bow.arrow_item,
            attrs.bow.bow_item
        ],
        [0x3B, 0x3D, 0x3F, 0x41, 0x4D]
    );
    // The archive holds zero; OnLoad writes animation 72's end frame.
    assert_eq!(attrs.down_air.hit_frame_end, 0.0);
    assert_eq!(attrs.shield.bone, 72);
    for (actual, expected) in [
        (attrs.bow.max_charge, 45.0),
        (attrs.spin_attack.landing_lag, 24.0),
        (attrs.down_air.hit_vertical_speed, 1.33),
        (attrs.hookshot_height, 9.67),
    ] {
        assert_eq!(actual.to_bits(), f32::to_bits(expected));
    }
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
    // 79 costume joints and the grafted shield bone.
    assert_eq!(parts.joint_count(), 80);
    let conditional = melee_ft::desc::read_conditional_parts(&common, DESCRIPTOR.kind).unwrap();
    assert_eq!(
        conditional,
        [melee_ft::desc::ConditionalPart {
            part: 72,
            parent: 71,
            insertion: 0,
            source_index: 0xFF,
        }]
    );
}
