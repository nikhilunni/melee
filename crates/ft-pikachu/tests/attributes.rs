mod support;
use ft_pikachu::{
    attributes::{read_pikachu_attributes, PikachuAttributes},
    init::{Pikachu, DESCRIPTOR},
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_zero_preserves_float_payloads_and_integer_signedness() {
    let mut data = vec![0; 0x100];
    word(&mut data, 0, 0x8000_0000);
    word(&mut data, 0x5C, (-3_i32) as u32);
    word(&mut data, 0xA8, (-4_i32) as u32);
    word(&mut data, 0xDC, 0x51);
    word(&mut data, 0xF4, 0x7FC0_1234);
    // ftData at +F8 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0xFC], Some(("ftDataPikachu", 0xF8)));
    let attrs = read_pikachu_attributes(&source).unwrap();
    assert_eq!(
        attrs.thunder_jolt.ground_spawn_offset.x.to_bits(),
        0x8000_0000
    );
    assert_eq!(attrs.quick_attack.air_start_hang_frames, -3);
    assert_eq!(attrs.quick_attack.second_zip_angle_degrees, -4);
    assert_eq!(attrs.thunder.bolt_item, 0x51);
    assert_eq!(attrs.quick_attack_box.right.y.to_bits(), 0x7FC0_1234);
    assert!(read_pikachu_attributes(&archive(&data, &[], Some(("ftDataPikachu", 0xF8)))).is_err());
    assert!(read_pikachu_attributes(&archive(&data, &[0xFC], Some(("ftDataFox", 0xF8)))).is_err());
    assert!(PikachuAttributes::read(&archive(&data[..0xF7], &[], None), 0).is_err());
}

#[test]
fn load_and_death_reset_model_groups_without_altering_attributes() {
    let attrs = PikachuAttributes::read(&archive(&[0; 0xF8], &[], None), 0).unwrap();
    let mut pikachu = Pikachu::new(attrs.clone());
    let mut capabilities = Capabilities::default();
    pikachu.on_load(&mut capabilities);
    assert_eq!(pikachu.kind(), FighterKind::Pikachu);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(pikachu.aerial_jump_style(), AerialJumpStyle::Basic);
    pikachu.model_groups = [2, 3];
    pikachu.on_reset();
    assert_eq!(pikachu.model_groups, [0, 0]);
    assert_eq!(pikachu.attributes, attrs);
}

#[test]
fn disc_attributes_and_parts() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [files.join(DESCRIPTOR.data_file), files.join("PlCo.dat")];
    if !melee_test_support::require_files(paths.iter()) {
        return;
    }
    let source = hsd_archive::Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap();
    let attrs = read_pikachu_attributes(&source).unwrap();
    // ftPk_Init_OnLoad registers these kinds (it_8026B3F8).
    assert_eq!(attrs.thunder_jolt.ground_item, 0x59);
    assert_eq!(attrs.thunder_jolt.air_item, 0x5A);
    assert_eq!(attrs.thunder.bolt_item, 0x51);
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
    assert_eq!(parts.joint_count(), 53);
    assert_eq!(bones.ecb.joints, [19, 24, 8, 41, 33, 4]);
    for (actual, expected) in [
        (attrs.skull_bash.maximum_charge, 90.0),
        (attrs.skull_bash.entry_velocity_divisor, 1.25),
        (attrs.quick_attack.base_speed, 5.9),
        (attrs.quick_attack.landing_lag, 24.0),
        (attrs.thunder.spawn_height, 150.0),
        (attrs.quick_attack_box.top, 8.0),
    ] {
        assert_eq!(actual.to_bits(), f32::to_bits(expected));
    }
    assert_eq!(attrs.quick_attack.zip_frames, 5);
    assert_eq!(attrs.quick_attack.second_zip_angle_degrees, 38);
    assert_eq!(attrs.thunder.bolt_parameters, [4, 8]);
}
