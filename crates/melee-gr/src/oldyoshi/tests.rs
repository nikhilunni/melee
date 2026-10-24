use super::*;

fn parameters() -> Parameters {
    Parameters {
        vanish_after: 120,
        return_after: 180,
        maximum_speed: 0.15,
        acceleration: 0.15,
        sink_depth: 6.0,
        disable_frame: 30,
        enable_frame: 30,
        guest_wait: [4000, 3000],
        guest_height: 30,
    }
}

#[derive(Default)]
struct Log {
    calls: Vec<String>,
    ended: bool,
}
impl CloudEngine for Log {
    fn materials_ended(&mut self, bone: usize) -> bool {
        self.calls.push(format!("ended {bone}"));
        self.ended
    }
    fn play_materials(&mut self, bone: usize, animation: usize) {
        self.calls.push(format!("play {bone} {animation}"));
    }
    fn spawn_puff(&mut self, bone: usize) {
        self.calls.push(format!("puff {bone}"));
    }
    fn remove_joint(&mut self, joint: i32) {
        self.calls.push(format!("remove {joint}"));
    }
    fn add_joint(&mut self, joint: i32) {
        self.calls.push(format!("add {joint}"));
    }
    fn set_hidden(&mut self, bone: usize, hidden: bool) {
        self.calls.push(format!("hidden {bone} {hidden}"));
    }
    fn set_translate_y(&mut self, _bone: usize, _y: f32) {}
}

#[test]
fn creation_draws_the_guest_wait_once() {
    let mut rng = HsdRng::new(5);
    let stage = OldYoshi::create(parameters(), [0.0; 3], &mut rng);
    let mut expected = HsdRng::new(5);
    assert_eq!(stage.guest.timer, (3000 + expected.randi(1000)) as i16);
    assert_eq!(rng.seed, expected.seed);
    assert_eq!(stage.guest.shown, -1);
}

#[test]
fn a_carried_cloud_sinks_to_its_depth_then_fades() {
    let mut stage = OldYoshi::create(parameters(), [10.0; 3], &mut HsdRng::new(0));
    let mut log = Log::default();
    for _ in 0..=121 {
        stage.tick_clouds([true, false, false], &mut log);
    }
    assert_eq!(stage.clouds[0].depth, 6.0);
    assert_eq!(stage.clouds[0].phase, CloudPhase::Fading);
    assert_eq!(log.calls, ["play 1 1", "puff 1"]);
    assert_eq!(stage.clouds[1].depth, 0.0);
}

#[test]
fn a_fading_cloud_loses_collision_at_the_disable_frame_and_when_gone() {
    let mut stage = OldYoshi::create(parameters(), [0.0; 3], &mut HsdRng::new(0));
    stage.clouds[2].phase = CloudPhase::Fading;
    let mut log = Log::default();
    for _ in 0..31 {
        stage.tick_clouds([false; 3], &mut log);
    }
    assert!(log.calls.contains(&"remove 2".to_owned()));
    log.ended = true;
    log.calls.clear();
    stage.tick_clouds([false; 3], &mut log);
    assert_eq!(log.calls, ["ended 9", "remove 2", "hidden 9 true"]);
    assert_eq!(stage.clouds[2].phase, CloudPhase::Gone);
    assert_eq!(stage.clouds[2].counter, 0);
}

#[test]
fn sink_speed_brakes_inside_the_stopping_distance() {
    // Moving down towards 6 from 5.99 at full speed: brake.
    assert_eq!(sink_speed(0.15, 0.15, 5.99, 6.0, 0.15, -0.15), 0.0);
    // Far away: accelerate, clamped.
    assert_eq!(sink_speed(0.1, 0.15, 0.0, 6.0, 0.15, -0.15), 0.15);
    // At the target: no change.
    assert_eq!(sink_speed(0.0, 0.15, 0.0, 0.0, 0.15, -0.15), 0.0);
    // A NaN speed takes the minimum (fcmpo unordered).
    assert_eq!(sink_speed(f32::NAN, 0.15, 0.0, 0.0, 0.15, -0.15), -0.15);
}

#[test]
fn the_guest_draws_every_waiting_frame() {
    let mut stage = OldYoshi::create(parameters(), [0.0; 3], &mut HsdRng::new(0));
    stage.guest.timer = 0;
    let mut rng = HsdRng::new(9);
    assert!(matches!(
        stage.tick_guest(false, &mut rng),
        GuestStep::Restart { .. }
    ));
    assert_eq!(stage.guest.shown, -1);
    // The timer went negative: the guest is chosen, then the height drawn.
    let mut expected = HsdRng::new(rng.seed);
    let step = stage.tick_guest(false, &mut rng);
    let shown = GUESTS[expected.randi(5) as usize];
    let r = expected.randf();
    assert_eq!(stage.guest.shown, shown);
    assert_eq!(
        step,
        GuestStep::Restart {
            height: 30.0 * gekko_math::fma::fmsubs(2.0, r, 1.0)
        }
    );
    let before = rng.seed;
    assert_eq!(
        stage.tick_guest(false, &mut rng),
        GuestStep::Shown {
            bone: shown as usize,
            ended: false
        }
    );
    assert_eq!(rng.seed, before);
    stage.tick_guest(true, &mut rng);
    assert_eq!(stage.guest.shown, -1);
    assert!((3000..4000).contains(&stage.guest.timer));
}
