use super::*;

fn parameters() -> Parameters {
    // GrGd.dat yakumono_param.
    Parameters {
        water_level: -40.0,
        current: -3.0,
        cranky_rest_loops: 2,
        cranky_busy_loops: 8,
        klaptrap_wait_minimum: 80,
        klaptrap_wait_range: 640,
        klaptrap_x: [-30.0, 30.0],
    }
}

#[test]
fn cranky_waits_out_its_loops_then_steps_through_four_animations() {
    let p = parameters();
    let mut rng = HsdRng::new(1);
    let mut cranky = Cranky {
        animation: 0,
        loops_left: 1,
    };
    assert_eq!(cranky.tick(&p, false, &mut rng), None);
    assert_eq!(cranky.tick(&p, true, &mut rng), None);
    assert_eq!(cranky.loops_left, 0);
    let seed = rng.seed;
    assert_eq!(cranky.tick(&p, true, &mut rng), Some(1));
    assert_eq!(rng.seed, seed, "animation 1 draws nothing");
    assert_eq!(cranky.tick(&p, true, &mut rng), Some(2));
    assert!((1..=8).contains(&cranky.loops_left));
    cranky.loops_left = 0;
    assert_eq!(cranky.tick(&p, true, &mut rng), Some(3));
    assert_eq!(cranky.tick(&p, true, &mut rng), Some(0));
    assert!((1..=2).contains(&cranky.loops_left));
}

#[test]
fn klaptrap_leaps_at_zero_and_its_counter_runs_on_after_landing() {
    let p = parameters();
    let mut rng = HsdRng::new(5);
    let mut klap = Klaptrap {
        leaping: false,
        timer: 1,
    };
    assert_eq!(klap.tick(&p, || unreachable!(), &mut rng), KlaptrapStep::default());
    let step = klap.tick(&p, || unreachable!(), &mut rng);
    assert_eq!(step.item_state, Some(klaptrap_item::BITE));
    assert!(step.start_leap);
    let x = Klaptrap::leap_x(&p, &mut rng);
    assert!((-30.0..30.0).contains(&x) && x.fract() == 0.0);
    assert!(klap.leaping && klap.timer == 0);
    klap.tick(&p, || false, &mut rng);
    assert_eq!(klap.timer, 1);
    let step = klap.tick(&p, || true, &mut rng);
    assert_eq!(step.item_state, Some(klaptrap_item::IDLE));
    assert!(step.remove_animation && !klap.leaping);
    assert!((81..=720).contains(&klap.timer));
}

#[test]
fn the_river_carries_below_its_line_and_splashes_on_entry() {
    let water = Water {
        level: -40.0,
        current: -3.0,
    };
    let above = Vec3::new(10.0, -39.0, 0.0);
    let below = Vec3::new(10.0, -41.0, 0.0);
    assert_eq!(water.sample(above, above, 10.0), (Vec3::ZERO, None));
    let (push, splash) = water.sample(below, above, 12.0);
    assert_eq!(push, Vec3::new(-3.0, 0.0, 0.0));
    let splash = splash.unwrap();
    assert_eq!(splash.position, Vec3::new(10.0, -30.0, 0.0));
    assert_eq!(splash.scale, 12.0 / 10.0);
    assert_eq!(water.sample(below, below, 12.0).1, None);
}
