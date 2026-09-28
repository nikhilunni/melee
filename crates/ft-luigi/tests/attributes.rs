mod support;
use ft_luigi::{
    attributes::{read_luigi_attributes, LuigiAttributes},
    init::Luigi,
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_attributes_keep_float_payloads_and_integers() {
    let mut data = vec![0; 0xA0];
    word(&mut data, 0x00, 0x8000_0000);
    word(&mut data, 0x44, 8.0_f32.to_bits()); // misfire odds
    word(&mut data, 0x6C, 0x7FC0_1234);
    word(&mut data, 0x88, (-4_i32) as u32);
    word(&mut data, 0x94, 7);
    // ftData at +98 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x9C], Some(("ftDataLuigi", 0x98)));
    let attrs = read_luigi_attributes(&source).unwrap();
    assert_eq!(attrs.green_missile.unknown.to_bits(), 0x8000_0000);
    assert_eq!(attrs.green_missile.misfire_odds, 8.0);
    assert_eq!(attrs.super_jump_punch.vel_mul.to_bits(), 0x7FC0_1234);
    assert_eq!(attrs.cyclone.unknown, -4);
    assert_eq!(attrs.cyclone.landing_lag, 7);
    assert!(read_luigi_attributes(&archive(&data, &[], Some(("ftDataLuigi", 0x98)))).is_err());
    assert!(read_luigi_attributes(&archive(&data, &[0x9C], Some(("ftDataMario", 0x98)))).is_err());
    assert!(LuigiAttributes::read(&archive(&data[..0x97], &[], None), 0).is_err());
}

fn luigi() -> Luigi {
    let data = vec![0; 0x98];
    Luigi::new(LuigiAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_every_special_without_walljump() {
    let mut luigi = luigi();
    let mut capabilities = Capabilities::default();
    luigi.on_load(&mut capabilities);
    assert_eq!(luigi.kind(), FighterKind::Luigi);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(luigi.aerial_jump_style(), AerialJumpStyle::Basic);
}

#[test]
fn only_the_cyclone_landing_clears_its_charge() {
    let mut luigi = luigi();
    let mut raw = vec![0; 0x2244];
    word(&mut raw, 0x222C, 1);
    word(&mut raw, 0x2230, 0xDEAD_BEEF);
    word(&mut raw, 0x2234, 5);
    luigi.restore_saved(&raw);
    assert!(luigi.cyclone_charged);
    assert_eq!((luigi.unknown_2230, luigi.unknown_2234), (0xDEAD_BEEF, 5));
    // ftCo_Landing.c has no Luigi arm.
    luigi.on_landing(false);
    assert!(luigi.cyclone_charged);
    luigi.model_group = 2;
    luigi.on_reset();
    assert_eq!((luigi.model_group, luigi.unknown_2234), (0, 0));
    assert!(luigi.cyclone_charged);
}
