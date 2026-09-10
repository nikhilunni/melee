mod support;
use ft_purin::{
    attributes::{read_purin_attributes, PurinAttributes},
    init::{Jigglypuff, DESCRIPTOR},
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use support::{archive, word};

#[test]
fn shared_multijump_types_preserve_integer_and_float_words() {
    let mut data = vec![0; 0x108];
    word(&mut data, 0, 12);
    word(&mut data, 0x14, 0x3FD33333);
    word(&mut data, 0x1C, 0x80000000);
    word(&mut data, 0x24, 0x7FC01234);
    word(&mut data, 0x28, 5);
    word(&mut data, 0x2C, 341);
    word(&mut data, 0x30, u32::MAX);
    let source = archive(&data, &[0x104], Some(("ftDataPurin", 0x100)));
    let attributes = read_purin_attributes(&source).unwrap();
    let jumps = attributes.multi_jump;
    assert_eq!(jumps.turn_frames, 12);
    assert_eq!(jumps.vertical_impulses[0].to_bits(), 0x3FD33333);
    assert_eq!(jumps.vertical_impulses[2].to_bits(), 0x80000000);
    assert_eq!(jumps.vertical_impulses[4].to_bits(), 0x7FC01234);
    assert_eq!(jumps.first_actions, [341, -1]);
    for action in 340..=346 {
        assert_eq!(jumps.contains_action(action), (341..=345).contains(&action));
    }
    assert!(read_purin_attributes(&archive(&data, &[], Some(("ftDataPurin", 0x100)))).is_err());
    assert!(PurinAttributes::read(&archive(&data[..0xFF], &[], None), 0).is_err());
}

#[test]
fn disc_multijump_impulses_capabilities_and_death_model_reset() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/roms/files")
        .join(DESCRIPTOR.data_file);
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let data = hsd_archive::Archive::parse(&std::fs::read(path).unwrap()).unwrap();
    let mut puff = Jigglypuff::from_archive(&data).unwrap();
    let jumps = &puff.attributes.multi_jump;
    assert_eq!(jumps.turn_frames, 12);
    assert_eq!(jumps.state_count, 5);
    assert_eq!(jumps.first_actions, [341, -1]);
    assert_eq!(
        jumps.vertical_impulses.map(f32::to_bits),
        [0x3FD33333, 0x3FCB851F, 0x3FBC28F6, 0x3FAE147B, 0x3FA00000]
    );
    assert_eq!(jumps.horizontal_impulse.to_bits(), 0x3F000000);
    assert_eq!(puff.attributes.rollout.duration, 90);
    assert_eq!(puff.attributes.pound.air_speed.to_bits(), 0x400CCCCD);
    let mut capabilities = Capabilities::default();
    puff.on_load(&mut capabilities);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(puff.aerial_jump_style(), AerialJumpStyle::MultiJump);
    puff.model_group = 2;
    puff.on_reset();
    assert_eq!(puff.model_group, 0);
}
