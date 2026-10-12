mod support;
use ft_kirby::{
    attributes::{read_kirby_attributes, KirbyAttributes, KIRBY_ATTRIBUTES_SIZE},
    init::Kirby,
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

const SIZE: usize = KIRBY_ATTRIBUTES_SIZE as usize;

#[test]
fn relocated_attributes_keep_the_multijump_words_with_their_consumers_types() {
    let mut data = vec![0; SIZE + 8];
    word(&mut data, 0x00, 12);
    word(&mut data, 0x04, 0.3_f32.to_bits());
    word(&mut data, 0x08, 0.5_f32.to_bits());
    word(&mut data, 0x0C, 0.8_f32.to_bits());
    word(&mut data, 0x10, (-0.0_f32).to_bits());
    for (i, impulse) in [2.0_f32, 2.0, 1.73, 1.56, 1.33].into_iter().enumerate() {
        word(&mut data, 0x14 + 4 * i, impulse.to_bits());
    }
    word(&mut data, 0x28, 5);
    word(&mut data, 0x2C, 341);
    word(&mut data, 0x30, 346);
    // ftData at +SIZE has a relocated ext_attr pointer to offset zero.
    let root = SIZE as u32;
    let source = archive(&data, &[root + 4], Some(("ftDataKirby", root)));
    let attrs = read_kirby_attributes(&source).unwrap();
    let jump = &attrs.multi_jump;
    assert_eq!(jump.turn_frames, 12);
    assert_eq!(jump.reverse_threshold, 0.3);
    assert_eq!(jump.horizontal_impulse, 0.5);
    assert_eq!(jump.acceleration_multiplier, 0.8);
    assert_eq!(jump.speed_multiplier.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(jump.vertical_impulses, [2.0, 2.0, 1.73, 1.56, 1.33]);
    assert_eq!(jump.state_count, 5);
    assert_eq!(jump.first_actions, [341, 346]);
    assert!(jump.contains_action(345) && jump.contains_action(350));
    assert!(!jump.contains_action(351));
    assert!(read_kirby_attributes(&archive(&data, &[], Some(("ftDataKirby", root)))).is_err());
    assert!(
        read_kirby_attributes(&archive(&data, &[root + 4], Some(("ftDataPurin", root)))).is_err()
    );
    assert!(KirbyAttributes::read(&archive(&data[..SIZE - 1], &[], None), 0).is_err());
}

fn kirby() -> Kirby {
    let data = vec![0; SIZE];
    Kirby::new(KirbyAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_every_special_and_the_shared_multijump() {
    let mut kirby = kirby();
    let mut capabilities = Capabilities::default();
    kirby.on_load(&mut capabilities);
    assert_eq!(kirby.kind(), FighterKind::Kirby);
    assert!(!capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(kirby.aerial_jump_style(), AerialJumpStyle::MultiJump);
    assert!(kirby.multi_jump_attributes().is_some());
    assert_eq!(kirby.multi_jump_family(), 0);
    assert_eq!(kirby.multi_jump_animation(4), 299);
}

#[test]
fn a_reset_drops_the_ability_and_draws_a_stone_form_between_one_and_five() {
    let mut state = kirby().into_state();
    let draw = <Kirby as CharacterCallbacks>::RESET_DRAW.unwrap();
    assert_eq!(<Kirby as CharacterCallbacks>::RESET_DRAW_COUNT, 1);
    let mut rng = gekko_math::HsdRng::new(1);
    for _ in 0..64 {
        let before = rng.seed;
        state.on_reset();
        draw(&mut state, &mut rng);
        let mut one = gekko_math::HsdRng::new(before);
        one.rand();
        assert_eq!(rng.seed, one.seed, "one draw per reset");
        let kirby = state.get::<Kirby>();
        assert!((1..=5).contains(&kirby.stone_form));
        assert_eq!(kirby.copied, FighterKind::Kirby);
    }
}

#[test]
fn a_landing_restores_the_once_per_flight_lifts() {
    let mut kirby = kirby();
    kirby.hammer_lift_used = true;
    kirby.ice_shot_lift_used = true;
    kirby.toad_lift_used = true;
    kirby.on_landing(true);
    assert!(!kirby.hammer_lift_used && !kirby.ice_shot_lift_used && !kirby.toad_lift_used);
}
