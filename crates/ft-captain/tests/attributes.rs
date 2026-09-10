mod support;
use ft_captain::{
    attributes::{read_captain_attributes, CaptainAttributes},
    init::{CaptainFalcon, DESCRIPTOR},
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_zero_preserves_float_payloads_and_integer_signedness() {
    let mut data = vec![0; 0x94];
    word(&mut data, 0, 0x8000_0000);
    word(&mut data, 0x34, 0x7FC0_1234);
    word(&mut data, 0x64, (-3_i32) as u32);
    word(&mut data, 0x68, 2); // types.h specifies f32, even for a subnormal payload.
    word(&mut data, 0x6C, u32::MAX);
    word(&mut data, 0x78, (-4_i32) as u32);
    // ftData at +8C has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x90], Some(("ftDataCaptain", 0x8C)));
    let attrs = read_captain_attributes(&source).unwrap();
    assert_eq!(
        attrs.falcon_punch.downward_stick_threshold.to_bits(),
        0x8000_0000
    );
    assert_eq!(
        attrs.raptor_boost.unused_parameters[5].to_bits(),
        0x7FC0_1234
    );
    assert_eq!(attrs.falcon_dive.initial_air_counter, -3);
    assert_eq!(attrs.falcon_kick.unused_parameter.to_bits(), 2);
    assert_eq!(attrs.falcon_kick.unused_word, u32::MAX);
    assert_eq!(attrs.falcon_kick.hit_slowdown_counter_limit, -4);
    assert!(read_captain_attributes(&archive(&data, &[], Some(("ftDataCaptain", 0x8C)))).is_err());
    assert!(read_captain_attributes(&archive(&data, &[0x90], Some(("ftDataFox", 0x8C)))).is_err());
    assert!(CaptainAttributes::read(&archive(&data[..0x8B], &[], None), 0).is_err());
}

#[test]
fn load_restore_and_death_reset_effect_flags_without_altering_attributes() {
    let attrs = CaptainAttributes::read(&archive(&[0; 0x8C], &[], None), 0).unwrap();
    let mut falcon = CaptainFalcon::new(attrs.clone());
    let mut capabilities = Capabilities::default();
    falcon.on_load(&mut capabilities);
    assert_eq!(falcon.kind(), FighterKind::Captain);
    assert!(capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(falcon.aerial_jump_style(), AerialJumpStyle::Basic);
    let mut raw = vec![0; 0x2234];
    word(&mut raw, 0x222C, 1);
    word(&mut raw, 0x2230, 2);
    falcon.restore_saved(&raw);
    assert!(falcon.raptor_boost_start_effect_active);
    assert!(falcon.raptor_boost_lunge_effect_active);
    falcon.model_group = 3;
    falcon.on_reset();
    assert!(!falcon.raptor_boost_start_effect_active);
    assert!(!falcon.raptor_boost_lunge_effect_active);
    assert_eq!(falcon.model_group, 0);
    assert_eq!(falcon.attributes, attrs);
}

#[test]
fn disc_attributes_parts_and_empty_dynamics() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [files.join(DESCRIPTOR.data_file), files.join("PlCo.dat")];
    if !melee_test_support::require_files(paths.iter()) {
        return;
    }
    let source = hsd_archive::Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap();
    let attrs = read_captain_attributes(&source).unwrap();
    for (actual, expected) in [
        (attrs.falcon_punch.aerial_speed, 0x3FF9_999A),
        (attrs.raptor_boost.gravity, 0x3D4C_CCCD),
        (attrs.raptor_boost.hit_landing_lag, 0x4220_0000),
        (attrs.falcon_dive.freefall_mobility, 0x3F38_51EC),
        (attrs.falcon_dive.initial_command_value, 0x4170_0000),
        (attrs.falcon_kick.unused_parameter, 2),
        (attrs.falcon_kick.flame_angle_degrees, 0x4270_0000),
        (
            attrs.falcon_kick.air_landing_traction_multiplier,
            0x4040_0000,
        ),
    ] {
        assert_eq!(actual.to_bits(), expected);
    }
    assert_eq!(attrs.falcon_dive.initial_air_counter, 0);
    assert_eq!(attrs.falcon_kick.unused_word, 4);
    assert_eq!(attrs.falcon_kick.hit_slowdown_counter_limit, 4);
    let root = source.public(DESCRIPTOR.data_symbol).unwrap();
    assert!(melee_ft::dynamics::read_sets(&source, root)
        .unwrap()
        .is_empty());
    let bones =
        melee_ft::desc::read_fighter_bones(&source, root, DESCRIPTOR.part_animation_count).unwrap();
    assert!(bones.dynamics_collision.is_empty());
    assert_eq!(bones.animation_sets.iter().flatten().count(), 3);
    assert_eq!(bones.ecb.joints, [39, 47, 25, 14, 8, 4]);
    let common = hsd_archive::Archive::parse(&std::fs::read(&paths[1]).unwrap()).unwrap();
    let parts =
        melee_ft::desc::read_part_table(&common, DESCRIPTOR.kind, DESCRIPTOR.part_count).unwrap();
    assert_eq!(parts.joint_count(), 63);
    assert_eq!(parts.part_to_joint.len(), 54);
}
