mod support;
use ft_mars::{
    attributes::{read_mars_attributes, MarsAttributes},
    init::{Marth, DESCRIPTOR},
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use support::{archive, word};

#[test]
fn relocated_zero_signed_fields_float_bits_and_sword_padding() {
    let mut data = vec![0; 0xA0];
    word(&mut data, 0, (-4_i32) as u32);
    word(&mut data, 0xC, 0x8000_0000);
    word(&mut data, 0x10, 0x7FC0_1234);
    word(&mut data, 0x64, (-3_i32) as u32);
    data[0x80..0x8C].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0xFF, 0xFE, 0xFD]);
    word(&mut data, 0x8C, 75);
    // ftData.ext_attr is a relocated pointer to offset zero.
    let source = archive(&data, &[0x9C], Some(("ftDataMars", 0x98)));
    let attrs = read_mars_attributes(&source).unwrap();
    assert_eq!(attrs.shield_breaker.maximum_charge_levels, -4);
    assert_eq!(attrs.shield_breaker.momentum_divisor.to_bits(), 0x8000_0000);
    assert_eq!(attrs.shield_breaker.friction.to_bits(), 0x7FC0_1234);
    assert_eq!(attrs.counter_volume.bone, -3);
    assert_eq!(attrs.sword.appearance, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(attrs.sword.bone, 75);
    assert!(read_mars_attributes(&archive(&data, &[], Some(("ftDataMars", 0x98)))).is_err());
    assert!(MarsAttributes::read(&archive(&data[..0x97], &[], None), 0).is_err());
}

#[test]
fn disc_attributes_and_character_resets() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !melee_test_support::require_files(["PlMs.dat", "PlCo.dat"].map(|name| files.join(name))) {
        return;
    }
    let archive =
        hsd_archive::Archive::parse(&std::fs::read(files.join(DESCRIPTOR.data_file)).unwrap())
            .unwrap();
    let attrs = read_mars_attributes(&archive).unwrap();
    assert_eq!(
        (
            attrs.shield_breaker.maximum_charge_levels,
            attrs.shield_breaker.base_damage,
            attrs.shield_breaker.damage_per_level
        ),
        (4, 7, 5)
    );
    for (actual, expected) in [
        (attrs.shield_breaker.momentum_divisor, 0x3FA00000),
        (attrs.dancing_blade.fall_acceleration, 0x3D75C28F),
        (attrs.dolphin_slash.freefall_mobility, 0x3ECCCCCD),
        (attrs.counter.momentum_divisor, 0x40000000),
        (attrs.counter_volume.radius, 0x41080000),
    ] {
        assert_eq!(actual.to_bits(), expected);
    }
    assert_eq!(attrs.counter_volume.bone, 3);
    assert_eq!(attrs.sword.bone, 75);
    let mut marth = Marth::new(attrs);
    let mut capabilities = Capabilities::default();
    marth.on_load(&mut capabilities);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    marth.specials.side_special_boost_used = true;
    marth.model_groups = [1, 2];
    marth.on_landing(false);
    assert!(!marth.specials.side_special_boost_used);
    assert_eq!(marth.model_groups, [1, 2]);
    marth.specials.side_special_boost_used = true;
    marth.on_reset();
    assert!(!marth.specials.side_special_boost_used);
    assert_eq!(marth.model_groups, [0; 2]);
}

#[test]
fn shield_hook_selects_model_and_queues_sound_once_per_entry() {
    let attrs = MarsAttributes::read(&archive(&[0; 0x98], &[], None), 0).unwrap();
    let marth = Marth::new(attrs);
    let mut commands = melee_ft::fighter::commands::CommandState::default();
    commands.model_selections.insert(0, 3);
    marth.guard_variant(&mut commands);
    assert_eq!(commands.model_selections.get(&0), Some(&3));
    assert_eq!(commands.model_selections.get(&1), Some(&1));
    assert_eq!(commands.footstep_sounds.len(), 1);
    let sound = &commands.footstep_sounds[0];
    assert_eq!((sound.id, sound.volume, sound.pan), (190115, 127, 64));
}
