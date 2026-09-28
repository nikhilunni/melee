mod support;
use ft_link::{
    attributes::{read_link_attributes, LinkAttributes},
    init::{Link, DESCRIPTOR},
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_zero_preserves_float_payloads_and_integer_signedness() {
    let mut data = vec![0; 0xE4];
    word(&mut data, 0, 0x8000_0000);
    word(&mut data, 0x2C, 0x3C);
    word(&mut data, 0xC4, (-2_i32) as u32);
    word(&mut data, 0xD8, 0x7FC0_1234);
    // ftData at +DC has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0xE0], Some(("ftDataLink", 0xDC)));
    let attrs = read_link_attributes(&source).unwrap();
    assert_eq!(attrs.bow.max_charge.to_bits(), 0x8000_0000);
    assert_eq!(attrs.boomerang.item, 0x3C);
    assert_eq!(attrs.shield.bone, -2);
    assert_eq!(attrs.shield.damage_scale.to_bits(), 0x7FC0_1234);
    assert!(read_link_attributes(&archive(&data, &[], Some(("ftDataLink", 0xDC)))).is_err());
    assert!(read_link_attributes(&archive(&data, &[0xE0], Some(("ftDataFox", 0xDC)))).is_err());
    assert!(LinkAttributes::read(&archive(&data[..0xDB], &[], None), 0).is_err());
}

#[test]
fn load_and_death_reset_model_groups_and_articles() {
    let attrs = LinkAttributes::read(&archive(&[0; 0xDC], &[], None), 0).unwrap();
    let mut link = Link::new(attrs.clone());
    let mut capabilities = Capabilities::default();
    link.on_load(&mut capabilities);
    assert_eq!(link.kind(), FighterKind::Link);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    link.model_groups = [2, 3, 1];
    link.specials.vars.used_boomerang = true;
    link.on_reset();
    assert_eq!(link.model_groups, [0, 0, 0]);
    assert!(!link.specials.vars.used_boomerang);
    assert_eq!(link.attributes, attrs);
}

#[test]
fn disc_attributes_parts_and_shield_bone() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [files.join(DESCRIPTOR.data_file), files.join("PlCo.dat")];
    if !melee_test_support::require_files(paths.iter()) {
        return;
    }
    let source = hsd_archive::Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap();
    let attrs = read_link_attributes(&source).unwrap();
    // ftLk_Init_OnLoad registers these kinds (it_8026B3F8).
    assert_eq!(
        [
            attrs.bomb_item,
            attrs.boomerang.item,
            attrs.hookshot_item,
            attrs.bow.arrow_item,
            attrs.bow.bow_item
        ],
        [0x3A, 0x3C, 0x3E, 0x40, 0x4C]
    );
    // The archive holds zero; OnLoad writes animation 72's end frame.
    assert_eq!(attrs.down_air.hit_frame_end, 0.0);
    assert_eq!(attrs.shield.bone, 68);
    for (actual, expected) in [
        (attrs.bow.max_charge, 60.0),
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
    // 75 costume joints and the grafted shield bone.
    assert_eq!(parts.joint_count(), 76);
    let conditional = melee_ft::desc::read_conditional_parts(&common, DESCRIPTOR.kind).unwrap();
    assert_eq!(
        conditional,
        [melee_ft::desc::ConditionalPart {
            part: 68,
            parent: 67,
            insertion: 0,
            source_index: 0xFF,
        }]
    );
}
