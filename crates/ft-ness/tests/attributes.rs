mod support;
use ft_ness::{
    attributes::{read_ness_attributes, NessAttributes},
    init::Ness,
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_attributes_keep_integers_floats_and_both_volumes() {
    let mut data = vec![0; 0xE4];
    word(&mut data, 0x00, 30);
    word(&mut data, 0x0C, 25);
    word(&mut data, 0x20, (-0.5_f32).to_bits());
    word(&mut data, 0x40, 24);
    word(&mut data, 0x54, 3.5_f32.to_bits());
    word(&mut data, 0x84, 4);
    word(&mut data, 0x94, 2.0_f32.to_bits());
    word(&mut data, 0x98, 1);
    word(&mut data, 0xA0, 6.5_f32.to_bits());
    word(&mut data, 0xA8, 8.5_f32.to_bits());
    word(&mut data, 0xB0, 350.0_f32.to_bits());
    word(&mut data, 0xB8, 4);
    word(&mut data, 0xBC, 50);
    word(&mut data, 0xD0, 1.5_f32.to_bits());
    // ftData at +DC has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0xE0], Some(("ftDataNess", 0xDC)));
    let attrs = read_ness_attributes(&source).unwrap();
    assert_eq!(attrs.pk_flash.ground_loop_frames, 30);
    assert_eq!(attrs.pk_flash.release_delay, 25);
    assert_eq!(attrs.pk_fire.air_angle, -0.5);
    assert_eq!(attrs.pk_thunder.loop_frames, 24);
    assert_eq!(attrs.pk_thunder_2.speed, 3.5);
    assert_eq!(attrs.magnet.gravity_delay, 4);
    assert_eq!(attrs.magnet.heal_multiplier, 2.0);
    assert_eq!(attrs.magnet.volume.bone, 1);
    assert_eq!(attrs.magnet.volume.offset.y, 6.5);
    assert_eq!(attrs.magnet.volume.radius, 8.5);
    assert_eq!(attrs.yoyo.damage_multiplier, 350.0);
    assert_eq!(attrs.bat_reflection.joint, 4);
    assert_eq!(attrs.bat_reflection.max_damage, 50);
    assert_eq!(attrs.bat_reflection.damage_multiplier, 1.5);
    assert!(read_ness_attributes(&archive(&data, &[], Some(("ftDataNess", 0xDC)))).is_err());
    assert!(read_ness_attributes(&archive(&data, &[0xE0], Some(("ftDataFox", 0xDC)))).is_err());
    assert!(NessAttributes::read(&archive(&data[..0xDB], &[], None), 0).is_err());
}

fn ness() -> Ness {
    let data = vec![0; 0xDC];
    Ness::new(NessAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_every_special_and_the_curved_double_jump() {
    let mut ness = ness();
    let mut capabilities = Capabilities::default();
    ness.on_load(&mut capabilities);
    assert_eq!(ness.kind(), FighterKind::Ness);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(capabilities.air_specials, None);
    assert_eq!(ness.aerial_jump_style(), AerialJumpStyle::Ness);
}
