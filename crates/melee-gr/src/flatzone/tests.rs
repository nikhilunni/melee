use super::*;

/// Records the engine calls; the floor is at y = 0 on line 7.
#[derive(Default)]
struct Log {
    calls: Vec<String>,
    translation: Vec3,
    ended: bool,
}
impl FlatZoneEngine for Log {
    fn play(&mut self, map: u8, animation: usize) {
        self.calls.push(format!("play {map} {animation}"));
    }
    fn remove_animation(&mut self, map: u8) {
        self.calls.push(format!("remove {map}"));
    }
    fn hide(&mut self, map: u8) {
        self.calls.push(format!("hide {map}"));
    }
    fn translation(&mut self, _map: u8) -> Vec3 {
        self.translation
    }
    fn set_translation(&mut self, _map: u8, translation: Vec3) {
        self.translation = translation;
    }
    fn animation_ended(&mut self, _map: u8) -> bool {
        self.ended
    }
    fn find_floor(&mut self, x: f32, _from: f32, _to: f32) -> Option<(Vec3, i32)> {
        Some((Vec3::new(x, 0.0, 0.0), 7))
    }
    fn spill(&mut self, position: Vec3, line: i32, radius: f32, frames: i32) -> Option<SpillHandle> {
        self.calls
            .push(format!("spill {} {line} {radius} {frames}", position.x));
        Some(0)
    }
    fn release_spill(&mut self, _handle: Option<SpillHandle>) {
        self.calls.push("release".into());
    }
    fn stitch(&mut self) {
        self.calls.push("stitch".into());
    }
    fn blast_top(&self) -> f32 {
        100.0
    }
    fn spawn_tool(&mut self, position: Vec3, kind: i32, facing: f32) {
        self.calls
            .push(format!("tool {} {} {kind} {facing}", position.x, position.y));
    }
}

fn parameters() -> Parameters {
    Parameters {
        event_wait: [3000, 1800],
        variant_wait: [420, 120],
        tools_frames: 1200,
        tools_interval: 60,
        tools_right: 40,
        tools_left: -60,
        oil_walk_in: 130,
        oil_right_x: -72.5,
        oil_left_x: 58.5,
        oil_step_frames: 24,
        oil_steps: [2, 8],
        oil_step: 8.0,
        oil_frames: 1200,
    }
}

#[test]
fn rand_int_draws_only_across_a_nonempty_range() {
    let mut rng = HsdRng::new(1);
    assert_eq!(rand_int(5, 5, &mut rng), 5);
    assert_eq!(rng.seed, 1);
    let mut a = HsdRng::new(1);
    let mut b = HsdRng::new(1);
    assert_eq!(rand_int(3000, 1800, &mut a), 1800 + b.randi(1200));
    let mut a = HsdRng::new(9);
    let mut b = HsdRng::new(9);
    assert_eq!(rand_int(120, 420, &mut a), 120 + b.randi(300));
}

#[test]
fn creation_waits_for_the_match_and_starts_on_layout_two_variant_one() {
    let mut rng = HsdRng::new(5);
    let mut stage = FlatZone::create(parameters(), 0, &mut rng);
    let mut log = Log::default();
    stage.first_layout(&mut log, &mut rng);
    assert_eq!(log.calls, ["play 5 9"]);
    assert!(stage.controller.waiting && stage.platforms.waiting);
    // Nothing counts down before fn_8021706C / fn_80217440.
    let seed = rng.seed;
    stage.tick_controller(&mut log, &mut rng);
    stage.tick_platforms(&mut log, &mut rng);
    assert_eq!(rng.seed, seed);
    stage.stitch_platforms(&mut log);
    assert_eq!(log.calls, ["play 5 9", "stitch"]);
}

#[test]
fn a_variant_changes_after_its_wait_and_restitches_once() {
    let mut rng = HsdRng::new(5);
    let mut stage = FlatZone::create(parameters(), 0, &mut rng);
    let mut log = Log::default();
    stage.first_layout(&mut log, &mut rng);
    stage.start();
    stage.platforms.timer = 0;
    stage.tick_platforms(&mut log, &mut rng);
    assert_eq!(stage.platforms.timer, -1);
    stage.tick_platforms(&mut log, &mut rng);
    let variant = stage.platforms.variant;
    assert_ne!(variant, 1);
    assert_eq!(
        stage.platforms.animation as i16,
        LAYOUTS[2][usize::from(variant)]
    );
    assert!((120..420).contains(&stage.platforms.timer));
}

#[test]
fn the_controller_alternates_tools_and_oil_and_waits_for_them() {
    let mut rng = HsdRng::new(77);
    let mut stage = FlatZone::create(parameters(), 0, &mut rng);
    let mut log = Log::default();
    stage.start();
    stage.controller.timer = 1;
    stage.tick_controller(&mut log, &mut rng);
    let first = stage.controller.event;
    assert!(matches!(first, TOOLS_EVENT | OIL_EVENT));
    assert!(!stage.controller.ready && stage.controller.timer == 0);
    assert_eq!(stage.platforms.variant, 0);
    assert_eq!(stage.tools.triggered, first == TOOLS_EVENT);
    assert_eq!(stage.oil.triggered, first == OIL_EVENT);
    // The hazard reports back; the next step picks a layout and a wait.
    stage.controller.ready = true;
    stage.tick_controller(&mut log, &mut rng);
    assert!((1800..3000).contains(&i32::from(stage.controller.timer)));
    assert_ne!(stage.platforms.variant, 0);
    stage.controller.timer = 1;
    stage.tick_controller(&mut log, &mut rng);
    assert_ne!(stage.controller.event, first);
}

#[test]
fn tools_fall_every_interval_then_free_the_controller() {
    let mut rng = HsdRng::new(3);
    let mut stage = FlatZone::create(parameters(), 0, &mut rng);
    let mut log = Log::default();
    stage.controller.ready = false;
    stage.tools.triggered = true;
    for _ in 0..1200 {
        stage.tick_tools(&mut log, &mut rng);
    }
    assert_eq!(log.calls.iter().filter(|c| c.starts_with("tool")).count(), 20);
    assert!(log.calls.iter().all(|c| c.contains(" 95 ")));
    assert!(stage.controller.ready);
}

#[test]
fn the_oil_man_walks_steps_pours_and_leaves() {
    let mut rng = HsdRng::new(11);
    let mut stage = FlatZone::create(parameters(), 0, &mut rng);
    let mut log = Log::default();
    stage.controller.ready = false;
    stage.oil.triggered = true;
    stage.tick_oil(&mut log, &mut rng);
    assert_eq!(stage.oil.phase, OilPhase::WalkIn);
    while stage.oil.phase != OilPhase::Pour {
        stage.tick_oil(&mut log, &mut rng);
    }
    log.ended = true;
    stage.tick_oil(&mut log, &mut rng);
    assert_eq!(stage.oil.phase, OilPhase::Spilled);
    assert!(stage.controller.ready);
    assert!(log.calls.iter().any(|c| c.starts_with("spill") && c.ends_with(" 7 22 1200")));
    while stage.oil.phase != OilPhase::Idle {
        stage.tick_oil(&mut log, &mut rng);
    }
    assert_eq!(stage.oil.spill, None);
}
