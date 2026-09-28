mod support;
use ft_ganon::{
    attributes::{read_ganon_attributes, CaptainAttributes},
    init::{Ganondorf, DESCRIPTOR},
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn attributes_come_from_the_ganon_data_symbol() {
    let mut data = vec![0; 0x94];
    word(&mut data, 0xC, 0x3FF0_0000);
    word(&mut data, 0x64, 7);
    // ftData at +8C has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x90], Some(("ftDataGanon", 0x8C)));
    let attrs = read_ganon_attributes(&source).unwrap();
    assert_eq!(attrs.falcon_punch.aerial_speed.to_bits(), 0x3FF0_0000);
    assert_eq!(attrs.falcon_dive.initial_air_counter, 7);
    let captain = archive(&data, &[0x90], Some(("ftDataCaptain", 0x8C)));
    assert!(read_ganon_attributes(&captain).is_err());
}

#[test]
fn load_has_no_walljump_and_death_resets_both_model_groups_and_effect_flags() {
    let attrs = CaptainAttributes::read(&archive(&[0; 0x8C], &[], None), 0).unwrap();
    let mut ganon = Ganondorf::new(attrs.clone());
    let mut capabilities = Capabilities::default();
    ganon.on_load(&mut capabilities);
    assert_eq!(ganon.kind(), FighterKind::Ganon);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(ganon.aerial_jump_style(), AerialJumpStyle::Basic);
    let mut raw = vec![0; 0x2234];
    word(&mut raw, 0x222C, 1);
    word(&mut raw, 0x2230, 2);
    ganon.restore_saved(&raw);
    assert!(ganon.specials.start_effect_active);
    assert!(ganon.specials.lunge_effect_active);
    ganon.model_groups = [3, 4];
    ganon.on_reset();
    assert!(!ganon.specials.start_effect_active);
    assert!(!ganon.specials.lunge_effect_active);
    assert_eq!(ganon.model_groups, [0, -1]);
    assert_eq!(ganon.attributes, attrs);
}

#[test]
fn disc_attributes_parts_and_part_animations() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [files.join(DESCRIPTOR.data_file), files.join("PlCo.dat")];
    if !melee_test_support::require_files(paths.iter()) {
        return;
    }
    let source = hsd_archive::Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap();
    let attrs = read_ganon_attributes(&source).unwrap();
    for (actual, expected) in [
        (attrs.falcon_punch.aerial_speed, AERIAL_PUNCH_SPEED),
        (attrs.raptor_boost.gravity, BOOST_GRAVITY),
        (attrs.falcon_kick.flame_angle_degrees, KICK_FLAME_ANGLE),
    ] {
        assert_eq!(actual.to_bits(), expected);
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
    assert_eq!(parts.joint_count(), JOINT_COUNT);
    assert_eq!(parts.part_to_joint.len(), DESCRIPTOR.part_count as usize);
}
/// ftDataGanon's ext_attr +0C, +18 and +70 (0.8, 0.05 and 60.0).
const AERIAL_PUNCH_SPEED: u32 = 0x3F4C_CCCD;
const BOOST_GRAVITY: u32 = 0x3D4C_CCCD;
const KICK_FLAME_ANGLE: u32 = 0x4270_0000;
/// PlCo ftPartsTable[25]: Ganondorf's model has 84 joints.
const JOINT_COUNT: usize = 84;
