mod support;
use ft_koopa::{
    attributes::{read_koopa_attributes, KoopaAttributes},
    init::Koopa,
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_attributes_keep_floats_and_integers_apart() {
    let mut data = vec![0; 0xA8];
    word(&mut data, 0x00, 5.0_f32.to_bits());
    word(&mut data, 0x04, 40);
    word(&mut data, 0x10, 360.0_f32.to_bits());
    word(&mut data, 0x18, 380.0_f32.to_bits());
    word(&mut data, 0x20, 30);
    word(&mut data, 0x2C, 3);
    word(&mut data, 0x4C, 250.0_f32.to_bits());
    word(&mut data, 0x54, 1.78_f32.to_bits());
    word(&mut data, 0x7C, 50.0_f32.to_bits());
    word(&mut data, 0x94, (-7.5_f32).to_bits());
    // ftData at +A0 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0xA4], Some(("ftDataKoopa", 0xA0)));
    let attrs = read_koopa_attributes(&source).unwrap();
    assert_eq!(attrs.armor, 5.0);
    assert_eq!(attrs.fire_breath.minimum_ticks, 40);
    assert_eq!(attrs.fire_breath.reach_max, 360.0);
    assert_eq!(attrs.fire_breath.life_max, 380.0);
    assert_eq!(attrs.fire_breath.quake_interval, 30);
    assert_eq!(attrs.klaw.bite_damage, 3);
    assert_eq!(attrs.klaw.hold_time, 250.0);
    assert_eq!(attrs.fortress.air_rise, 1.78);
    assert_eq!(attrs.fortress.landing_lag, 50.0);
    assert_eq!(attrs.bomb.drop_velocity, -7.5);
    assert!(read_koopa_attributes(&archive(&data, &[], Some(("ftDataKoopa", 0xA0)))).is_err());
    assert!(read_koopa_attributes(&archive(&data, &[0xA4], Some(("ftDataFox", 0xA0)))).is_err());
    assert!(KoopaAttributes::read(&archive(&data[..0x9F], &[], None), 0).is_err());
}

fn koopa() -> Koopa {
    let mut data = vec![0; 0xA0];
    word(&mut data, 0x00, 2.0_f32.to_bits());
    word(&mut data, 0x10, 360.0_f32.to_bits());
    word(&mut data, 0x18, 380.0_f32.to_bits());
    Koopa::new(KoopaAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_every_special_the_inverted_down_bound_and_the_armour() {
    let mut koopa = koopa();
    let mut capabilities = Capabilities::default();
    koopa.on_load(&mut capabilities);
    assert_eq!(koopa.kind(), FighterKind::Koopa);
    assert!(!capabilities.can_walljump);
    assert!(capabilities.grounded_down_bound);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(capabilities.armor, 2.0);
}

#[test]
fn death_refills_both_breath_fuels() {
    let mut koopa = koopa();
    let mut raw = vec![0; 0x2234];
    word(&mut raw, 0x222C, 41.0_f32.to_bits());
    word(&mut raw, 0x2230, 61.0_f32.to_bits());
    koopa.restore_saved(&raw);
    assert_eq!((koopa.breath_reach, koopa.breath_life), (41.0, 61.0));
    koopa.on_reset();
    assert_eq!((koopa.breath_reach, koopa.breath_life), (360.0, 380.0));
}
