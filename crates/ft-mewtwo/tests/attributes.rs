mod support;
use ft_mewtwo::{
    attributes::{read_mewtwo_attributes, MewtwoAttributes},
    init::Mewtwo,
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_attributes_keep_each_specials_floats_and_integers() {
    let mut data = vec![0; 0x90];
    word(&mut data, 0x00, 7.0_f32.to_bits());
    word(&mut data, 0x08, (-0.4_f32).to_bits());
    word(&mut data, 0x0C, 16);
    word(&mut data, 0x18, 1.5_f32.to_bits());
    word(&mut data, 0x1C, 2); // ReflectDesc joint
    word(&mut data, 0x20, 50); // ReflectDesc max damage
    word(&mut data, 0x50, 10);
    word(&mut data, 0x68, 45);
    word(&mut data, 0x74, 30.0_f32.to_bits());
    word(&mut data, 0x84, 4.5_f32.to_bits());
    // ftData at +88 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x8C], Some(("ftDataMewtwo", 0x88)));
    let attrs = read_mewtwo_attributes(&source).unwrap();
    assert_eq!(attrs.shadow_ball.full_charge, 7.0);
    assert_eq!(attrs.shadow_ball.air_recoil, -0.4);
    assert_eq!(attrs.shadow_ball.frames_per_charge, 16);
    assert_eq!(attrs.confusion.air_boost, 1.5);
    assert_eq!(attrs.confusion.reflection.joint, 2);
    assert_eq!(attrs.confusion.reflection.max_damage, 50);
    assert_eq!(attrs.teleport.travel_frames, 10);
    assert_eq!(attrs.teleport.surface_angle_degrees, 45);
    assert_eq!(attrs.teleport.landing_lag, 30.0);
    assert_eq!(attrs.disable.offset_y, 4.5);
    assert!(read_mewtwo_attributes(&archive(&data, &[], Some(("ftDataMewtwo", 0x88)))).is_err());
    assert!(read_mewtwo_attributes(&archive(&data, &[0x8C], Some(("ftDataFox", 0x88)))).is_err());
    assert!(MewtwoAttributes::read(&archive(&data[..0x87], &[], None), 0).is_err());
}

fn mewtwo() -> Mewtwo {
    let data = vec![0; 0x88];
    Mewtwo::new(MewtwoAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_every_special_and_the_root_motion_compensation() {
    let mut mewtwo = mewtwo();
    let mut capabilities = Capabilities::default();
    mewtwo.on_load(&mut capabilities);
    assert_eq!(mewtwo.kind(), FighterKind::Mewtwo);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(capabilities.air_specials, None);
    assert!(capabilities.compensates_root_motion);
    assert_eq!(mewtwo.aerial_jump_style(), AerialJumpStyle::Mewtwo);
    assert_eq!(mewtwo.dynamics_first_force_bone(0, 1), 8);
}

#[test]
fn landing_restores_the_confusion_lift_and_death_drops_the_charge() {
    let mut mewtwo = mewtwo();
    let mut raw = vec![0; 0x2240];
    word(&mut raw, 0x2234, 3);
    word(&mut raw, 0x223C, 1);
    mewtwo.restore_saved(&raw);
    assert_eq!(mewtwo.shadow_ball_charge, 3);
    assert!(mewtwo.confusion_boost_used);
    mewtwo.on_landing(true);
    assert!(!mewtwo.confusion_boost_used);
    assert_eq!(mewtwo.shadow_ball_charge, 3);
    mewtwo.confusion_boost_used = true;
    mewtwo.model_group = 1;
    mewtwo.on_reset();
    assert_eq!(
        (
            mewtwo.shadow_ball_charge,
            mewtwo.confusion_boost_used,
            mewtwo.model_group
        ),
        (0, false, 0)
    );
}
