mod support;
use ft_peach::{
    attributes::{read_peach_attributes, PeachAttributes},
    init::{Peach, DESCRIPTOR},
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};
use support::{archive, word};

#[test]
fn relocated_zero_preserves_payloads_and_checks_the_whole_attribute_block() {
    let mut data = vec![0; 0xC8];
    word(&mut data, 0, 0x8000_0000);
    word(&mut data, 8, 0x7FC0_1234);
    word(&mut data, 0x10, (-3_i32) as u32);
    word(&mut data, 0x18, 2);
    word(&mut data, 0x1C, ItemKind::BombHei as u32);
    word(&mut data, 0x90, (-4_i32) as u32);
    // Relocated ext_attr at ftData+C4 points to data offset zero.
    let source = archive(&data, &[0xC4], Some(("ftDataPeach", 0xC0)));
    let attrs = read_peach_attributes(&source).unwrap();
    assert_eq!(attrs.float.forward_fall_start.to_bits(), 0x8000_0000);
    assert_eq!(attrs.float.fall_start_offset.to_bits(), 0x7FC0_1234);
    assert_eq!(attrs.vegetable.rare_item_count, -3);
    assert_eq!(attrs.vegetable.rare_items[0].weight, 2);
    assert_eq!(attrs.vegetable.rare_items[0].kind, ItemKind::BombHei);
    assert_eq!(attrs.parasol.open_duration, -4);
    assert!(read_peach_attributes(&archive(&data, &[], Some(("ftDataPeach", 0xC0)))).is_err());
    assert!(PeachAttributes::read(&archive(&data[..0xBF], &[], None), 0).is_err());
    word(&mut data, 0x1C, 0x7FFF_FFFF);
    assert!(PeachAttributes::read(&archive(&data, &[], None), 0).is_err());
}

#[test]
fn death_restores_costume_models_and_airtime_resources_but_preserves_float_time() {
    let attrs = PeachAttributes::read(&archive(&[0; 0xC0], &[], None), 0).unwrap();
    let mut peach = Peach::new(attrs.clone());
    let mut capabilities = Capabilities::default();
    peach.on_load(&mut capabilities);
    assert_eq!(peach.kind(), FighterKind::Peach);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(peach.aerial_jump_style(), AerialJumpStyle::Peach);
    assert_eq!(
        peach.registered_items,
        [
            ItemKind::PeachExplode,
            ItemKind::PeachTurnip,
            ItemKind::PeachParasol,
            ItemKind::PeachToad,
            ItemKind::PeachToadSpore
        ]
    );
    for (costume, expected) in [(0, [0, 0, 0, -1, 0, -1, 0]), (1, [0, -1, 0, -1, 0, 0, -1])] {
        let mut raw = vec![0; 0x224C];
        raw[0x619] = costume;
        word(&mut raw, 0x2230, 37.0_f32.to_bits());
        for offset in [0x2234, 0x2238, 0x223C, 0x2240, 0x2244, 0x2248] {
            word(&mut raw, offset, 1);
        }
        peach.restore_saved(&raw);
        assert!(!peach.has_float);
        assert!(peach.aerial_toad_used);
        assert_eq!(peach.items.parasol, [true; 2]);
        peach.on_reset();
        assert!(peach.has_float);
        assert_eq!(peach.smash_motion, -1);
        assert!(!(peach.aerial_toad_used || peach.items.toad || peach.items.vegetable));
        assert_eq!(peach.items.parasol, [false; 2]);
        assert_eq!(peach.float_remaining.to_bits(), 37.0_f32.to_bits());
        assert_eq!(peach.model_groups, expected);
        assert_eq!(peach.attributes, attrs);
    }
    peach.has_float = false;
    peach.on_grounded_motion();
    assert!(peach.has_float);
    peach.aerial_toad_used = true;
    peach.on_landing(false);
    assert!(!peach.aerial_toad_used);
    assert_eq!(
        (0..9)
            .map(|i| peach.dynamics_first_force_bone(i, 9))
            .collect::<Vec<_>>(),
        [3, 3, 3, 3, 3, 3, 3, 3, 0]
    );
}

#[test]
fn disc_attributes_parts_and_nine_dynamic_chains() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [
        files.join(DESCRIPTOR.data_file),
        files.join("PlCo.dat"),
        files.join(DESCRIPTOR.animation_file),
    ];
    if !melee_test_support::require_files(paths.iter()) {
        return;
    }
    let source = hsd_archive::Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap();
    let attrs = read_peach_attributes(&source).unwrap();
    for (actual, expected) in [
        (attrs.float.forward_fall_start, 0),
        (attrs.float.backward_fall_start, 0),
        (attrs.float.fall_start_offset, 0x40A0_0000),
        (attrs.float.duration, 0x4316_0000),
        (attrs.bomber.start_gravity, 0x3DA3_D70A),
        (attrs.bomber.rebound_horizontal_speed, 0xBF80_0000),
        (attrs.parasol.startup_momentum_multiplier, 0x3F2A_A64C),
        (attrs.toad.air_friction, 0x3B23_D70A),
        (attrs.toad_volume.radius, 0x40C0_0000),
    ] {
        assert_eq!(actual.to_bits(), expected);
    }
    assert_eq!(attrs.vegetable.rare_item_count, 3);
    assert_eq!(attrs.vegetable.rare_item_odds, 128);
    assert_eq!(
        attrs
            .vegetable
            .rare_items
            .iter()
            .map(|v| (v.weight, v.kind as i32))
            .collect::<Vec<_>>(),
        [(2, 6), (3, 7), (1, 12)]
    );
    assert_eq!(attrs.parasol.open_duration, 600);
    assert_eq!(attrs.toad_volume.bone, 3);
    let root = source.public(DESCRIPTOR.data_symbol).unwrap();
    let sets = melee_ft::dynamics::read_sets(&source, root).unwrap();
    assert_eq!(
        sets.iter().map(|s| s.root).collect::<Vec<_>>(),
        [19, 31, 61, 49, 37, 43, 55, 25, 88]
    );
    assert!(sets.iter().all(|s| s.springs.len() == 5));
    let bones =
        melee_ft::desc::read_fighter_bones(&source, root, DESCRIPTOR.part_animation_count).unwrap();
    assert!(bones.dynamics_collision.is_empty());
    assert_eq!(bones.ecb.joints, [87, 97, 71, 13, 7, 4]);
    let common = hsd_archive::Archive::parse(&std::fs::read(&paths[1]).unwrap()).unwrap();
    let parts =
        melee_ft::desc::read_part_table(&common, DESCRIPTOR.kind, DESCRIPTOR.part_count).unwrap();
    assert_eq!(parts.joint_count(), 114);
    assert_eq!(parts.part_to_joint.len(), 54);
    let assets = melee_ft::fighter::assets::FighterAssets::load(
        &DESCRIPTOR,
        &source,
        &common,
        &std::fs::read(&paths[2]).unwrap(),
    )
    .unwrap();
    let mut peach = Peach::new(attrs);
    peach.on_resources_loaded(
        &assets,
        &melee_ft::fighter::PlayerSlot {
            id: 0,
            secondary: false,
            control: melee_types::PlayerKind::Human,
            costume: 0,
            stocks: 4,
            position: hsd_types::Vec3::ZERO,
            facing: 1.0,
            scale: 1.0,
            damage: 0.0,
            cpu_mode: 0,
            cpu_level: 9,
        },
    );
    assert_eq!(
        peach.attributes.float.forward_fall_start.to_bits(),
        assets.motions[&18].animation.frames.to_bits()
    );
    assert_eq!(
        peach.attributes.float.backward_fall_start.to_bits(),
        assets.motions[&19].animation.frames.to_bits()
    );
}
