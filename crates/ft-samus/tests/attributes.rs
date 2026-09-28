mod support;
use ft_samus::{
    attributes::{read_samus_attributes, GrappleTimeline, SamusAttributes},
    init::Samus,
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_attributes_keep_floats_integers_the_ball_box_and_beam_timelines() {
    let mut data = vec![0; 0xDC];
    word(&mut data, 0x00, 0x8000_0000);
    word(&mut data, 0x18, 7.0_f32.to_bits());
    word(&mut data, 0x20, 16);
    word(&mut data, 0x78, 2.0_f32.to_bits());
    word(&mut data, 0x84, 8.0_f32.to_bits());
    word(&mut data, 0x98, 5.0_f32.to_bits());
    for (i, value) in [7, 17, 75, 93, 7, 17, 40, 64, 1, 7, 40, 58].iter().enumerate() {
        word(&mut data, 0x9C + 4 * i, *value);
    }
    word(&mut data, 0xD0, (-3_i32) as u32);
    // ftData at +D4 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0xD8], Some(("ftDataSamus", 0xD4)));
    let attrs = read_samus_attributes(&source).unwrap();
    assert_eq!(attrs.bomb_jump.late_start_frame.to_bits(), 0x8000_0000);
    assert_eq!(attrs.charge_shot.max_charge, 7.0);
    assert_eq!(attrs.charge_shot.frames_per_level, 16);
    assert_eq!(attrs.bomb.spawn_offset.y, 2.0);
    assert_eq!(attrs.morph_ball_box.top, 8.0);
    assert_eq!(attrs.morph_ball_box.right.y, 5.0);
    let timeline = |spawn, extend, retract, remove| GrappleTimeline {
        spawn,
        extend,
        retract,
        remove,
    };
    assert_eq!(attrs.grab_beam, timeline(7, 17, 75, 93));
    assert_eq!(attrs.dash_grab_beam, timeline(7, 17, 40, 64));
    assert_eq!(attrs.air_beam, timeline(1, 7, 40, 58));
    assert_eq!(attrs.unknown_d0, -3);
    assert!(read_samus_attributes(&archive(&data, &[], Some(("ftDataSamus", 0xD4)))).is_err());
    assert!(read_samus_attributes(&archive(&data, &[0xD8], Some(("ftDataFox", 0xD4)))).is_err());
    assert!(SamusAttributes::read(&archive(&data[..0xD3], &[], None), 0).is_err());
}

fn samus() -> Samus {
    let data = vec![0; 0xD4];
    Samus::new(SamusAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_walljump_and_every_special() {
    let mut samus = samus();
    let mut capabilities = Capabilities::default();
    samus.on_load(&mut capabilities);
    assert_eq!(samus.kind(), FighterKind::Samus);
    assert!(capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(samus.aerial_jump_style(), AerialJumpStyle::Basic);
}

#[test]
fn restore_then_death_resets_the_fighter_vars_but_keeps_the_charge_effect_flag() {
    let mut samus = samus();
    let mut raw = vec![0; 0x224C];
    word(&mut raw, 0x2230, 5);
    word(&mut raw, 0x2234, 1);
    word(&mut raw, 0x2238, 2);
    word(&mut raw, 0x2244, 1);
    samus.restore_saved(&raw);
    assert_eq!(samus.charge_level, 5);
    assert_eq!(samus.missiles_fired, 2);
    assert!(samus.charge_effects && samus.screw_effect);
    samus.model_group = 1;
    samus.on_reset();
    assert_eq!((samus.charge_level, samus.missiles_fired, samus.model_group), (0, 0, 0));
    assert!(samus.charge_effects && !samus.screw_effect);
}
