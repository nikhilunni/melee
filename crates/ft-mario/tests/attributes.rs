mod support;
use ft_mario::{
    attributes::{read_mario_attributes, MarioAttributes},
    init::Mario,
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};
use support::{archive, word};

#[test]
fn relocated_attributes_keep_float_payloads_integers_and_the_cape_reflector() {
    let mut data = vec![0; 0x8C];
    word(&mut data, 0x00, 0x8000_0000);
    word(&mut data, 0x14, 0x53); // It_Kind_Mario_Cape
    word(&mut data, 0x34, 0x7FC0_1234);
    word(&mut data, 0x50, (-4_i32) as u32);
    word(&mut data, 0x5C, 7);
    word(&mut data, 0x60, 4); // ReflectDesc joint
    word(&mut data, 0x64, 50); // max damage
    word(&mut data, 0x74, 6.5_f32.to_bits());
    data[0x80] = 1;
    // ftData at +84 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x88], Some(("ftDataMario", 0x84)));
    let attrs = read_mario_attributes(&source).unwrap();
    assert_eq!(attrs.cape.horizontal_velocity_decay.to_bits(), 0x8000_0000);
    assert_eq!(attrs.cape.cape_kind, ItemKind::MarioCape);
    assert_eq!(attrs.super_jump_punch.vel_mul.to_bits(), 0x7FC0_1234);
    assert_eq!(attrs.tornado.unknown, -4);
    assert_eq!(attrs.tornado.landing_lag, 7);
    assert_eq!(attrs.cape_reflection.joint, 4);
    assert_eq!(attrs.cape_reflection.max_damage, 50);
    assert_eq!(attrs.cape_reflection.size, 6.5);
    assert_eq!(attrs.cape_reflection.skip_ownership_change, 1);
    assert!(read_mario_attributes(&archive(&data, &[], Some(("ftDataMario", 0x84)))).is_err());
    assert!(read_mario_attributes(&archive(&data, &[0x88], Some(("ftDataFox", 0x84)))).is_err());
    assert!(MarioAttributes::read(&archive(&data[..0x83], &[], None), 0).is_err());
}

fn mario() -> Mario {
    let mut data = vec![0; 0x84];
    word(&mut data, 0x14, 0x53);
    Mario::new(MarioAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_walljump_and_every_special() {
    let mut mario = mario();
    let mut capabilities = Capabilities::default();
    mario.on_load(&mut capabilities);
    assert_eq!(mario.kind(), FighterKind::Mario);
    assert!(capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(mario.aerial_jump_style(), AerialJumpStyle::Basic);
}

#[test]
fn restore_then_death_and_landing_reset_the_fighter_vars() {
    let mut mario = mario();
    let mut raw = vec![0; 0x2244];
    word(&mut raw, 0x222C, 3);
    word(&mut raw, 0x2230, 5);
    word(&mut raw, 0x2234, 1);
    word(&mut raw, 0x2238, 1);
    mario.restore_saved(&raw);
    assert_eq!((mario.specials.vitamin_current, mario.specials.vitamin_previous), (3, 5));
    assert!(mario.specials.tornado_charged && mario.specials.cape_boosted);
    mario.on_landing(false);
    assert!(!mario.specials.tornado_charged && !mario.specials.cape_boosted);
    mario.restore_saved(&raw);
    mario.model_group = 2;
    mario.on_reset();
    assert_eq!((mario.specials.vitamin_current, mario.specials.vitamin_previous), (9, 9));
    assert!(!mario.specials.tornado_charged && !mario.specials.cape_boosted);
    assert_eq!(mario.model_group, 0);
}
