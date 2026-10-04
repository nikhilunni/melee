mod support;
use ft_gamewatch::{
    attributes::{read_gamewatch_attributes, GameWatchAttributes},
    init::GameWatch,
};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;
use support::{archive, word};

#[test]
fn relocated_attributes_keep_colours_floats_the_judge_table_and_the_absorb_bubble() {
    let mut data = vec![0; 0x9C];
    word(&mut data, 0x00, 0.01_f32.to_bits());
    word(&mut data, 0x08, 0x6E00_00FF);
    word(&mut data, 0x14, 0xFFFF_FF80);
    word(&mut data, 0x18, 3.0_f32.to_bits());
    word(&mut data, 0x1C, 5.0_f32.to_bits());
    for face in [0, 2, 8] {
        word(&mut data, 0x34 + 4 * face, 1);
    }
    word(&mut data, 0x60, 40.0_f32.to_bits());
    word(&mut data, 0x7C, 6.0_f32.to_bits());
    word(&mut data, 0x80, 6);
    word(&mut data, 0x84, (-4.5_f32).to_bits());
    word(&mut data, 0x90, 5.5_f32.to_bits());
    // ftData at +94 has a relocated ext_attr pointer to offset zero.
    let source = archive(&data, &[0x98], Some(("ftDataGamewatch", 0x94)));
    let attrs = read_gamewatch_attributes(&source).unwrap();
    assert_eq!(attrs.width, 0.01);
    assert_eq!(attrs.colors[1], [0x6E, 0, 0, 0xFF]);
    assert_eq!(attrs.outline, [0xFF, 0xFF, 0xFF, 0x80]);
    assert_eq!(attrs.chef.loop_frame, 3.0);
    assert_eq!(attrs.chef.max_sausages, 5.0);
    assert_eq!(
        attrs.judge.enabled,
        [true, false, true, false, false, false, false, false, true]
    );
    assert_eq!(attrs.rescue.landing_lag, 40.0);
    assert_eq!(attrs.panic.turn_frames, 6.0);
    assert_eq!(attrs.panic.absorb.bone, 6);
    assert_eq!(attrs.panic.absorb.offset.x, -4.5);
    assert_eq!(attrs.panic.absorb.size, 5.5);
    assert!(
        read_gamewatch_attributes(&archive(&data, &[], Some(("ftDataGamewatch", 0x94)))).is_err()
    );
    assert!(
        read_gamewatch_attributes(&archive(&data, &[0x98], Some(("ftDataFox", 0x94)))).is_err()
    );
    assert!(GameWatchAttributes::read(&archive(&data[..0x93], &[], None), 0).is_err());
}

fn gamewatch() -> GameWatch {
    let data = vec![0; 0x94];
    GameWatch::new(GameWatchAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn load_enables_walljump_and_every_special_with_an_empty_bucket() {
    let mut gw = gamewatch();
    gw.panic_charge = 2;
    let mut capabilities = Capabilities::default();
    gw.on_load(&mut capabilities);
    assert_eq!(gw.kind(), FighterKind::GameWatch);
    assert!(capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(gw.panic_charge, 0);
}

#[test]
fn death_resets_the_draw_histories_but_keeps_the_bucket_level() {
    let mut gw = gamewatch();
    let mut raw = vec![0; 0x2270];
    word(&mut raw, 0x222C, 7);
    word(&mut raw, 0x2230, 4);
    word(&mut raw, 0x2234, 1);
    word(&mut raw, 0x2238, 2);
    word(&mut raw, 0x223C, 31);
    word(&mut raw, 0x2240, 4);
    word(&mut raw, 0x2244, 0);
    gw.restore_saved(&raw);
    assert_eq!((gw.judge_last, gw.judge_previous), (7, 4));
    assert_eq!((gw.panic_charge, gw.panic_damage), (2, 31));
    gw.on_reset();
    assert_eq!((gw.judge_last, gw.judge_previous, gw.x2234), (1, 0, 0));
    assert_eq!((gw.chef_last, gw.chef_previous), (1, 3));
    assert_eq!((gw.panic_charge, gw.panic_damage), (2, 0));
}

#[test]
fn landing_restores_the_aerial_judgment_hop() {
    let mut gw = gamewatch();
    gw.x2234 = 1;
    gw.on_landing(true);
    assert_eq!(gw.x2234, 0);
}

/// A Mr. Game & Watch with every Judgment face enabled.
fn gamewatch_with_every_face() -> GameWatch {
    let mut data = vec![0; 0x94];
    for face in 0..9 {
        word(&mut data, 0x34 + 4 * face, 1);
    }
    GameWatch::new(GameWatchAttributes::read(&archive(&data, &[], None), 0).unwrap())
}

#[test]
fn judgment_never_repeats_either_of_the_last_two_faces() {
    let mut gw = gamewatch_with_every_face();
    let mut rng = gekko_math::HsdRng::new(0x1234_5678);
    for _ in 0..2000 {
        let (last, previous) = (gw.judge_last, gw.judge_previous);
        let face = ft_gamewatch::special_s::draw_face(&mut gw, &mut rng);
        assert!((0..9).contains(&face));
        assert!(face != last && face != previous);
        assert_eq!((gw.judge_last, gw.judge_previous), (face, last));
    }
}

#[test]
fn judgment_draws_one_value_and_skips_disabled_faces() {
    let mut data = vec![0; 0x94];
    for face in [3, 5, 8] {
        word(&mut data, 0x34 + 4 * face, 1);
    }
    let mut gw = GameWatch::new(GameWatchAttributes::read(&archive(&data, &[], None), 0).unwrap());
    let mut rng = gekko_math::HsdRng::new(7);
    for _ in 0..200 {
        let mut expected = rng;
        expected.randi(1);
        let before = (gw.judge_last, gw.judge_previous);
        let face = ft_gamewatch::special_s::draw_face(&mut gw, &mut rng);
        assert_eq!(rng.seed, expected.seed, "one HSD_Randi per Judgment");
        assert!([3, 5, 8].contains(&face));
        assert!(face != before.0 && face != before.1);
    }
}

#[test]
fn chef_picks_among_the_three_foods_not_thrown_last() {
    let mut gw = gamewatch_with_every_face();
    let mut rng = gekko_math::HsdRng::new(99);
    let mut seen = [false; 5];
    for _ in 0..500 {
        let (last, previous) = (gw.chef_last, gw.chef_previous);
        let food = ft_gamewatch::special_n::draw_food(&mut gw, &mut rng);
        assert!((0..5).contains(&food));
        assert!(food != last && food != previous);
        assert_eq!((gw.chef_last, gw.chef_previous), (food, last));
        seen[food as usize] = true;
    }
    assert_eq!(seen, [true; 5]);
}
