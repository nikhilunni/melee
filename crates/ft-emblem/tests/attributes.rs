mod support;
use ft_emblem::{
    attributes::{read_roy_attributes, MarsAttributes},
    init::{Roy, DESCRIPTOR},
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use support::{archive, word};

#[test]
fn attributes_come_from_ftdataemblem_in_marths_layout() {
    let mut data = vec![0; 0xA0];
    word(&mut data, 0, 5);
    word(&mut data, 0x5C, 0x3FC0_0000);
    // ftData.ext_attr is a relocated pointer to offset zero.
    let source = archive(&data, &[0x9C], Some(("ftDataEmblem", 0x98)));
    let attrs = read_roy_attributes(&source).unwrap();
    assert_eq!(attrs.shield_breaker.maximum_charge_levels, 5);
    assert_eq!(attrs.counter.damage_multiplier.to_bits(), 0x3FC0_0000);
    // Marth's symbol is not Roy's.
    assert!(read_roy_attributes(&archive(&data, &[0x9C], Some(("ftDataMars", 0x98)))).is_err());
}

#[test]
fn death_resets_three_model_groups_and_the_side_special_boost() {
    let attrs = MarsAttributes::read(&archive(&[0; 0x98], &[], None), 0).unwrap();
    let mut roy = Roy::new(attrs);
    assert_eq!(roy.model_groups, [0, 0, -1]);
    let mut capabilities = Capabilities::default();
    roy.on_load(&mut capabilities);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    roy.specials.side_special_boost_used = true;
    roy.on_landing(false);
    assert!(!roy.specials.side_special_boost_used);
    roy.specials.side_special_boost_used = true;
    roy.model_groups = [1, 1, 1];
    roy.on_reset();
    assert!(!roy.specials.side_special_boost_used);
    assert_eq!(roy.model_groups, [0, 0, -1]);
}

#[test]
fn guard_keeps_the_default_sword_model() {
    // ftCo_800923B4 / 800939B4 switch only on FTKIND_MARS.
    let attrs = MarsAttributes::read(&archive(&[0; 0x98], &[], None), 0).unwrap();
    let roy = Roy::new(attrs);
    let mut commands = melee_ft::fighter::commands::CommandState::default();
    roy.guard_variant(&mut commands);
    assert!(commands.model_selections.get(&1).is_none());
    assert!(commands.footstep_sounds.is_empty());
}

#[test]
fn disc_attributes() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !melee_test_support::require_files([files.join(DESCRIPTOR.data_file)]) {
        return;
    }
    let archive =
        hsd_archive::Archive::parse(&std::fs::read(files.join(DESCRIPTOR.data_file)).unwrap())
            .unwrap();
    let attrs = read_roy_attributes(&archive).unwrap();
    assert!(attrs.shield_breaker.maximum_charge_levels > 0);
    assert!(attrs.counter.damage_multiplier > 1.0);
}
