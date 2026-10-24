use super::*;

/// GrOk.dat's yakumono_param.
fn parameters() -> Parameters {
    Parameters {
        bird_wait: [3000, 4000],
        bird_height_range: 20.0,
        shot_delay: [479.0, 480.0],
        catch_radius: 15.0,
        turn_wait: [360.0, 180.0],
        turn_acceleration: 4.0,
        turn_speed: 4.0,
        turn_spin: [110.0, 90.0],
        aim_weights: [1, 1, 10, 50, 10, 1, 1, 1],
        stop_interval: [600, 300],
        move_acceleration: 0.005,
        move_speed: 1.0,
        drift_frames: [6000, 3000],
        shot_damage: 0,
        shot_angle: 361,
        shot_growth: 0,
        shot_set_knockback: 0,
        shot_base_knockback: 180,
        shot_element: 0,
        catch_material: 0x9738,
    }
}

struct Engine {
    shots_started: usize,
    alive: bool,
}
impl BarrelEngine for Engine {
    fn aim_cannon(&mut self, _aim: f32) -> Vec3 {
        Vec3::new(1.0, -30.0, 0.0)
    }
    fn set_rate(&mut self, _rate: f32) {}
    fn start_shot_animation(&mut self) {
        self.shots_started += 1;
    }
    fn holds_fighter(&self, _fighter: usize) -> bool {
        self.alive
    }
}

#[test]
fn creation_draws_the_turn_wait_the_drift_and_the_bird() {
    let p = parameters();
    let mut rng = HsdRng::new(3);
    let stage = OldKongo::create(p, &mut rng);
    let mut expected = HsdRng::new(3);
    assert_eq!(stage.barrel.turn_timer, (180 + expected.randi(180)) as i16);
    assert_eq!(
        stage.barrel.drift_timer,
        (3000 + expected.randi(3000)) as i16
    );
    assert_eq!(stage.bird.timer, 3000 + expected.randi(1000));
    assert_eq!(rng.seed, expected.seed);
    assert_eq!(stage.barrel.drift, Drift::Cruising);
    assert_eq!(stage.barrel.speed, 1.0);
}

#[test]
fn the_aims_follow_their_weights_in_order() {
    let p = parameters();
    for seed in 0..200 {
        let mut rng = HsdRng::new(seed);
        let aim = choose_aim(&p, &mut rng);
        let r = HsdRng::new(seed).randi(75);
        let expected = match r {
            0 => AIMS[0],
            1 => AIMS[1],
            2..=11 => AIMS[2],
            12..=61 => AIMS[3],
            62..=71 => AIMS[4],
            72 => AIMS[5],
            73 => AIMS[6],
            _ => AIMS[7],
        };
        assert_eq!(aim, expected);
    }
}

#[test]
fn a_caught_fighter_is_fired_after_the_delay_and_windup() {
    let p = parameters();
    let mut stage = OldKongo::create(p.clone(), &mut HsdRng::new(0));
    let mut rng = HsdRng::new(11);
    let cannon = Vec3::new(1.0, -30.0, 0.0);
    // Out of reach: no draw.
    assert!(!stage
        .barrel
        .try_catch(&p, cannon, 0, Vec3::new(1.0, -15.0, 0.0), &mut rng));
    assert_eq!(rng.seed, 11);
    assert!(stage
        .barrel
        .try_catch(&p, cannon, 1, Vec3::new(1.0, -20.0, 0.0), &mut rng));
    assert_eq!(stage.barrel.shot_delay, 479);
    assert!(!stage
        .barrel
        .try_catch(&p, cannon, 0, cannon, &mut HsdRng::new(0)));
    stage.barrel.fire_early();
    let mut engine = Engine {
        shots_started: 0,
        alive: true,
    };
    let mut shot = None;
    for tick in 0..12 {
        let (_, fired) = stage.barrel.tick(&p, &mut rng, &mut engine);
        if fired.is_some() {
            assert_eq!(tick, 11);
            shot = fired;
        }
    }
    let shot = shot.expect("fired");
    assert_eq!(shot.fighter, 1);
    assert_eq!(engine.shots_started, 1);
    assert_eq!(stage.barrel.catch, Catch::Fired);
    // A quarter turn past the aim, in degrees.
    assert!((shot.angle - 90.0).abs() < 1.0e-3 || stage.barrel.aim != 0.0);
}

#[test]
fn the_bird_flies_when_its_wait_runs_out_and_waits_again_after() {
    let p = parameters();
    let mut bird = Bird { timer: 2 };
    let mut rng = HsdRng::new(5);
    assert_eq!(bird.tick(&p, false, &mut rng), BirdStep::Idle);
    let mut expected = HsdRng::new(5);
    let r = expected.randf();
    let from_right = expected.randi(2) != 0;
    assert_eq!(
        bird.tick(&p, false, &mut rng),
        BirdStep::Fly {
            height: fmadds(20.0, fmsubs(2.0, r, 1.0), 70.0),
            from_right
        }
    );
    assert_eq!(bird.tick(&p, false, &mut rng), BirdStep::Idle);
    assert_eq!(bird.tick(&p, true, &mut rng), BirdStep::Land);
    assert!((3000..4000).contains(&bird.timer));
}
