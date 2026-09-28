use super::*;

fn parameters() -> Parameters {
    // GrIz.dat yakumono_param.
    Parameters {
        initial_heights: [20.0, 28.0],
        rest_height: 25.0,
        step: [5.0, 10.0],
        highest_target: 35.0,
        lowest_target: 15.0,
        rise_speed: 0.15,
        sink_speed: 0.1,
        sink_chance_below_rest: 0.25,
        rise_chance_above_rest: 0.25,
        wait_frames: [1080.0, 600.0],
        submerge_weight: 4.0,
        stay_weight: 10.0,
        step_weight: 8.0,
        submerged_frames: [1680.0, 480.0],
    }
}

fn platform(phase: PlatformPhase, height: f32, target: f32) -> Platform {
    Platform {
        phase,
        timer: 0,
        collision_joint: 0,
        height,
        target,
        full_height: 20.0,
        rest_height: 25.0,
        origin_y: 12.5,
    }
}

#[test]
fn arrival_draws_the_wait_from_reversed_bounds_and_rescales_the_pillar() {
    let p = parameters();
    let mut a = platform(PlatformPhase::Arrived, 20.0, 20.0);
    let mut rng = HsdRng::new(7);
    let step = a.tick(&p, &mut rng);
    let mut expected = HsdRng::new(7);
    assert_eq!(i32::from(a.timer), 600 + expected.randi(480));
    assert_eq!(rng.seed, expected.seed);
    assert_eq!(a.phase, PlatformPhase::Waiting);
    // Height 20 over a unit height of 20: ratio 1, scale (1, 1).
    assert_eq!(step.pillar_scale, Some((1.0, 1.0)));
    assert_eq!(step.collision_y, Some(20.0));
    assert_eq!(step.visibility, Visibility::Unchanged);
}

#[test]
fn waiting_decides_only_after_the_timer_went_negative() {
    let p = parameters();
    let mut a = platform(PlatformPhase::Waiting, 20.0, 20.0);
    let mut rng = HsdRng::new(7);
    // A timer of zero still waits one more tick (signed post-decrement).
    let step = a.tick(&p, &mut rng);
    assert_eq!((a.timer, rng.seed), (-1, 7));
    assert_eq!(step.collision_y, None);
    a.tick(&p, &mut rng);
    assert_ne!(rng.seed, 7);
}

#[test]
fn moving_lands_exactly_on_the_target_and_sinking_below_the_lowest_submerges() {
    let p = parameters();
    let cases = [
        // (height, target, next height, next phase)
        (20.0, 20.1, 20.1, PlatformPhase::Arrived),
        (20.0, 21.0, 20.15, PlatformPhase::Moving),
        (20.0, 19.0, 19.9, PlatformPhase::Moving),
        (15.05, 15.0, 15.0, PlatformPhase::Arrived),
        (-0.95, -1.0, -1.0, PlatformPhase::Sunk),
        (20.0, 20.0, 20.0, PlatformPhase::Arrived),
    ];
    for (height, target, next, phase) in cases {
        let mut a = platform(PlatformPhase::Moving, height, target);
        let step = a.tick(&p, &mut HsdRng::new(1));
        assert_eq!((a.height, a.phase), (next, phase), "{height} -> {target}");
        assert_eq!(step.collision_y, Some(next));
    }
}

#[test]
fn a_sunk_platform_hides_below_its_origin_then_resurfaces_to_rest() {
    let p = parameters();
    let mut a = platform(PlatformPhase::Sunk, -1.0, -1.0);
    let mut rng = HsdRng::new(3);
    let step = a.tick(&p, &mut rng);
    assert_eq!(a.phase, PlatformPhase::Submerged);
    assert!((480..1680).contains(&i32::from(a.timer)));
    assert_eq!(step.visibility, Visibility::Hide);
    assert_eq!(step.collision_y, Some(11.5));
    assert_eq!(step.pillar_scale, None);
    a.timer = -1;
    let step = a.tick(&p, &mut rng);
    assert_eq!((a.phase, a.target), (PlatformPhase::Moving, 25.0));
    assert_eq!(step.visibility, Visibility::ShowAndAnimate);
    // The pillar never shrinks below 1% of its unit height.
    assert_eq!(step.pillar_scale, Some((fmadds(0.5, 0.01, 0.5), 0.01)));
}

#[test]
fn equal_rand_range_bounds_do_not_draw() {
    let mut rng = HsdRng::new(5);
    assert_eq!(rand_range(&mut rng, [9.7, 9.2]), 9);
    assert_eq!(rng.seed, 5);
}

#[test]
fn saved_grounds_map_to_scheduler_keys() {
    use procs::{saved_key, PLATFORMS, STAR};
    assert_eq!(saved_key(-1, 0), Some(STAR));
    assert_eq!(saved_key(4, 0), Some(PLATFORMS[0]));
    assert_eq!(saved_key(4, 1), Some(PLATFORMS[1]));
    assert_eq!(saved_key(3, 0), Some(3));
    assert_eq!(saved_key(4, 2), None);
}
