mod support;
use ft_donkey::{
    attributes::{read_donkey_attributes, DonkeyAttributes},
    init::DonkeyKong,
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_attributes_keep_the_carry_rows_floats_and_integers() {
    let mut data = vec![0; 0x7C];
    word(&mut data, 0x00, 341);
    word(&mut data, 0x04, 351);
    word(&mut data, 0x1C, 1.5_f32.to_bits());
    word(&mut data, 0x28, 4.0_f32.to_bits());
    word(&mut data, 0x2C, 10);
    word(&mut data, 0x30, 2);
    word(&mut data, 0x44, 0x8000_0000);
    word(&mut data, 0x64, 20.0_f32.to_bits());
    word(&mut data, 0x70, 7.0_f32.to_bits());
    // ftData at +74 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x78], Some(("ftDataDonkey", 0x74)));
    let attrs = read_donkey_attributes(&source).unwrap();
    assert_eq!(attrs.carry.heavy_first_state, 341);
    assert_eq!(attrs.carry.cargo_first_state, 351);
    assert_eq!(attrs.carry.walk_speeds[2], 1.5);
    assert_eq!(attrs.carry.landing_frames, 4.0);
    assert_eq!(attrs.giant_punch.max_swings, 10);
    assert_eq!(attrs.giant_punch.damage_per_swing, 2);
    assert_eq!(attrs.headbutt.gravity.to_bits(), 0x8000_0000);
    assert_eq!(attrs.spinning_kong.landing_lag, 20.0);
    assert_eq!(attrs.hand_slap.reach, 7.0);
    assert!(read_donkey_attributes(&archive(&data, &[], Some(("ftDataDonkey", 0x74)))).is_err());
    assert!(read_donkey_attributes(&archive(&data, &[0x78], Some(("ftDataFox", 0x74)))).is_err());
    assert!(DonkeyAttributes::read(&archive(&data[..0x73], &[], None), 0).is_err());
}

fn donkey() -> DonkeyKong {
    let data = vec![0; 0x74];
    DonkeyKong::new(DonkeyAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_every_ground_special_and_no_aerial_down_special() {
    let mut donkey = donkey();
    let mut capabilities = Capabilities::default();
    donkey.on_load(&mut capabilities);
    assert_eq!(donkey.kind(), FighterKind::Donkey);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(capabilities.air_specials, Some([true, true, true, false]));
}

#[test]
fn restore_then_death_drops_the_stored_punch() {
    let mut donkey = donkey();
    let mut raw = vec![0; 0x2234];
    word(&mut raw, 0x222C, 7);
    word(&mut raw, 0x2230, 0xDEAD_BEEF);
    donkey.restore_saved(&raw);
    assert_eq!(donkey.punch_swings, 7);
    donkey.model_group = 1;
    donkey.on_reset();
    assert_eq!((donkey.punch_swings, donkey.model_group), (0, 0));
    assert_eq!(donkey.unknown_2230, 0xDEAD_BEEF);
}
