//! The transformation controller (map 2, `grStadium_801D4548`): the base
//! arena sinks, a form rises in its place, and later the base returns.
use super::{Parameters, ScreenMode};
use crate::rand::range;
use gekko_math::{fma::fnmsubs, HsdRng};
use hsd_types::Vec3;

/// A form's map id (`grStadium_GroundVars::xDE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Fire = 3,
    Grass = 4,
    Base = 5,
    Rock = 6,
    Water = 9,
}
impl Form {
    pub fn map(self) -> u8 {
        self as u8
    }
    pub fn from_map(map: i16) -> Option<Self> {
        Some(match map {
            3 => Self::Fire,
            4 => Self::Grass,
            5 => Self::Base,
            6 => Self::Rock,
            9 => Self::Water,
            _ => return None,
        })
    }
    /// grpstadium.c:1912 `datfiles`: the archive a form's models live in.
    pub fn archive(self) -> Option<usize> {
        match self {
            Self::Fire => Some(0),
            Self::Grass => Some(1),
            Self::Water => Some(2),
            Self::Rock => Some(3),
            Self::Base => None,
        }
    }
    /// grpstadium.c:2112-2126: the screen's announcement.
    fn announcement(self) -> ScreenMode {
        match self {
            Self::Fire => ScreenMode::AnnounceFire,
            Self::Grass => ScreenMode::AnnounceGrass,
            Self::Water => ScreenMode::AnnounceWater,
            Self::Rock => ScreenMode::AnnounceRock,
            Self::Base => ScreenMode::AnnounceBase,
        }
    }
}

/// grpstadium.c:2063: the forms the base arena turns into.
const FORMS: [Form; 4] = [Form::Fire, Form::Grass, Form::Rock, Form::Water];
/// Scale factor of a sunk form (grpstadium.c:2147-2152) and the depth a
/// rising form starts below its place.
const SUNK_SCALE: f32 = 0.05;
const RISE_SCALE: f32 = 0.95;
const RISE_DEPTH: f32 = -10.0;

/// `grStadium_GroundVars::xDC`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Counting down the current form's duration.
    Waiting = 0,
    /// The next form's archive is being read (lbFile_80016580).
    Loading = 1,
    /// Tell the screen.
    Announcing = 2,
    /// The announcement plays before the arena moves.
    Delay = 3,
    /// The standing form sinks, then waits sunk; the next form is created.
    Sinking = 4,
    /// The next form rises while the old one drops away.
    Rising = 5,
    /// The new form takes over its collision.
    Settling = 6,
}
impl Phase {
    pub fn from_saved(value: i16) -> Option<Self> {
        (value == 0).then_some(Self::Waiting)
    }
}

/// What the controller asks of the engine, in C order.
pub trait StadiumEngine {
    fn rng(&mut self) -> &mut HsdRng;
    /// `grStadium_801D42B8` (0x801D42B8): whether `form`'s archive read
    /// (lbFile_80016580) has completed by this poll, the `poll`-th of the
    /// read (from 1). Retail knows through the DVD callback fn_801D4220,
    /// which clears map 2's xC4_b1; the port takes it as an external event
    /// (`ExternalEvents` in melee-lib) or from [`default_read_completed`].
    fn read_completed(&mut self, form: Form, poll: u32) -> bool;
    /// grStadium_801D2528 on map 1 with `mode`.
    fn announce(&mut self, mode: ScreenMode);
    /// The standing form's xC4_b1 (the water form drops its windmill).
    fn warn_standing(&mut self, form: Form);
    /// HSD_JObjGetScaleY of a form's root JObj (the map-scale wrapper).
    fn scale_y(&mut self, form: Form) -> f32;
    fn set_scale_y(&mut self, form: Form, scale: f32);
    fn set_translate_y(&mut self, form: Form, translate: f32);
    /// grAnime_801C7A04(gobj, 0, 7, 0.0): freeze the form's animation.
    fn freeze(&mut self, form: Form);
    /// grStadium_801D10F8: create the form's map and run its init.
    fn create(&mut self, form: Form);
    /// Ground_801C4A08: destroy the form's map.
    fn destroy(&mut self, form: Form);
    /// The new form's xC4_b0 (the water form adds its windmill).
    fn settle(&mut self, form: Form);
    fn enable_line(&mut self, line: i32);
    fn disable_line(&mut self, line: i32);
    /// mpLib_80058560.
    fn stitch_all(&mut self);
    /// grLib_801C96F8(0x7548, 30, position): one sparkle, before map scale.
    fn sparkle(&mut self, position: Vec3);
    /// Camera_RequestQuake(QuakeKind_Loop).
    fn quake(&mut self);
    fn map_scale(&self) -> f32;
}

/// `lbl_803E1630`: the loop the sparkles run along (corner, leg length).
const SPARKLE_PATH: [(Vec3, f32); 9] = [
    (Vec3::new(-50.0, 5.0, 50.0), 100.0),
    (Vec3::new(50.0, 5.0, 50.0), 28.28),
    (Vec3::new(70.0, 5.0, 30.0), 53.85),
    (Vec3::new(70.0, 5.0, -30.0), 5.0),
    (Vec3::new(50.0, 5.0, -50.0), 100.0),
    (Vec3::new(-50.0, 5.0, -50.0), 28.28),
    (Vec3::new(-70.0, 5.0, -30.0), 60.0),
    (Vec3::new(-70.0, 5.0, 30.0), 28.28),
    (Vec3::new(-50.0, 5.0, 50.0), 0.0),
];
/// Sparkles per tick while the arena moves.
const SPARKLES_PER_TICK: usize = 5;
/// The last leg the walk may stop on (grpstadium.c:1986 `var_r4 < 8`).
const LAST_LEG: usize = 8;

/// Lines disabled while the base form stands (see procs::PIT_LINES).
const PIT_LINES: [i32; 2] = super::procs::PIT_LINES;

/// The default read-latency policy: the poll of `grStadium_801D42B8` that
/// finds each form archive's read complete (counting from 1), indexed by
/// `Form::archive` (GrPs1-4.dat: fire, grass, water, rock).
///
/// Retail's latency is the disc's (in Dolphin, its emulated DVD timing), so
/// no game state predicts it. These are the polls measured for each file's
/// first read of a match from start_ps_fox_marth4 (the poll that succeeds is
/// the tick before the announcement). Recordings vary: 21-23 polls for the
/// same first read, 12 for GrPs1.dat right after GrPs4.dat. A standalone run
/// (explorer, native app) uses this table for every read, a plausible sample
/// rather than a prediction; replaying a recording takes the recorded
/// completion instead. A fixed table keeps the policy free of hidden state:
/// no second random stream to seed, clone or record.
pub const DEFAULT_READ_POLLS: [u32; 4] = [23, 18, 23, 21];

/// The default policy for `StadiumEngine::read_completed`.
pub fn default_read_completed(form: Form, poll: u32) -> bool {
    poll >= DEFAULT_READ_POLLS[form.archive().expect("a form's archive")]
}

/// Slippi's "Preload Stadium Transformations" code (slippi-ssbm-asm
/// `Common/Preload Stadium Transformations`, in the console core and
/// netplay sets): the next form is chosen, and its archive read started, on
/// the base form's first waiting tick instead of at the end of the wait, so
/// the form is announced on the tick the wait ends with no disc latency.
/// The code keeps two fields in map 2's spare user data: isLoaded (+0xF0)
/// and TransformationID (+0xEC).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preload {
    /// Retail: the code is not installed.
    #[default]
    Off,
    /// isLoaded clear: Init isLoaded Bool.asm (0x801D14C8, map 2's init) or
    /// Reset isLoaded.asm (0x801D4F14, the base form settling).
    Pending,
    /// isLoaded set; TransformationID holds the form chosen ahead.
    Chosen(Form),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transformation {
    /// Map 2's xC4_b0: set at creation, cleared by the deferred start
    /// callback fn_801D13C8; while set the controller does not run.
    pub waiting_for_start: bool,
    pub phase: Phase,
    /// xD8: the phase's counter.
    pub timer: i32,
    /// xDE: the form being shown or coming.
    pub form: Form,
    /// xE0; `None` is retail's -1.
    pub previous: Option<Form>,
    /// xE2.
    pub before_previous: Option<Form>,
    /// xE4's map: the form standing now.
    pub standing: Form,
    /// xE8's map: the form rising.
    pub rising: Option<Form>,
    /// xD4: the sparkle path's length, summed on first use.
    pub sparkle_path: f32,
    /// Polls of the form archive's read in flight made so far.
    pub read_polls: u32,
    /// xD0: the registered form archive (grDatFiles_801C6478), until the
    /// next form is chosen (grAnime_801C65B0).
    pub archive: Option<Form>,
    /// Slippi's preload code; `Off` is retail.
    pub preload: Preload,
    /// Slippi's Frozen Stadium code (slippi-ssbm-asm `External/Frozen PS`,
    /// FreezePokemon.asm): 0x801D45FC's `bge` becomes an unconditional
    /// branch to the function's end, so the wait never ends.
    pub frozen: bool,
    /// "Disable Pokemon Stadium Transformations" (0x801D1548,
    /// grStadium_801D1520's `bl grStadium_801D4548`, is a nop): the
    /// controller never runs, so neither the timer nor Slippi's preload
    /// draw (hooked inside it) happens.
    pub disabled: bool,
}

impl Transformation {
    /// `grStadium_801D13E0` (0x801D13E0): the first base-form duration.
    pub fn new(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            waiting_for_start: true,
            phase: Phase::Waiting,
            timer: range(rng, parameters.base_frames),
            form: Form::Base,
            previous: None,
            before_previous: None,
            standing: Form::Base,
            rising: None,
            sparkle_path: 0.0,
            read_polls: 0,
            archive: None,
            preload: Preload::Off,
            frozen: false,
            disabled: false,
        }
    }

    /// `grStadium_801D4548` (0x801D4548), outside Training mode and the
    /// 0xF0 stage kind.
    pub fn tick(&mut self, parameters: &Parameters, engine: &mut impl StadiumEngine) {
        match self.phase {
            Phase::Waiting => self.wait(engine),
            Phase::Loading => {
                // grStadium_801D42B8: once the callback cleared xC4_b1,
                // grDatFiles_801C6478 registers the archive.
                self.read_polls += 1;
                if engine.read_completed(self.form, self.read_polls) {
                    self.archive = Some(self.form);
                    self.phase = Phase::Announcing;
                }
            }
            Phase::Announcing => {
                // grpstadium.c:2109 writes display.xD8 (a pointer) through
                // map 2's union: the stadium timer at the same offset.
                self.timer = 0;
                engine.announce(self.form.announcement());
                self.phase = Phase::Delay;
            }
            Phase::Delay => {
                let old = self.timer;
                self.timer += 1;
                if old > parameters.announce_delay {
                    engine.warn_standing(self.standing);
                    self.phase = Phase::Sinking;
                    self.timer = 0;
                    // Ground_801C53EC(0x75300), (0x75301): sounds.
                }
            }
            Phase::Sinking => {
                self.sink(parameters, engine);
                self.sparkles(engine);
                engine.quake();
            }
            Phase::Rising => {
                self.rise(parameters, engine);
                self.sparkles(engine);
                engine.quake();
            }
            Phase::Settling => {
                engine.settle(self.standing);
                engine.stitch_all();
                if self.form == Form::Base {
                    // Reset isLoaded.asm (0x801D4F14), ahead of the draw.
                    if self.preload != Preload::Off {
                        self.preload = Preload::Pending;
                    }
                    self.timer = range(engine.rng(), parameters.base_frames);
                    // grAnime_801C65B0(xCC) clears the preload buffer's head.
                    for line in PIT_LINES {
                        engine.disable_line(line);
                    }
                } else {
                    self.timer = range(engine.rng(), parameters.form_frames);
                }
                self.phase = Phase::Waiting;
            }
        }
    }

    /// Phase 0: at the end of the duration choose the next form. From the
    /// base, redraw while the choice equals xE2 (the form before the
    /// previous one); a form always returns to the base.
    fn wait(&mut self, engine: &mut impl StadiumEngine) {
        // Load Transformation.asm (0x801D45EC, the phase's first
        // instruction): a waiting tick with isLoaded clear draws the next
        // form by retail's redraw rule and starts its read (lbFile_80016580
        // into xCC, callback fn_801D4220). isLoaded is clear only at the
        // start and once the base form has settled.
        if self.preload == Preload::Pending {
            self.preload = Preload::Chosen(self.draw_form(engine));
        }
        // Signed post-decrement: a timer of zero waits one more tick.
        let old = self.timer;
        self.timer = self.timer.wrapping_sub(1);
        // FreezePokemon.asm (0x801D45FC): the timer still counts down.
        if old >= 0 || self.frozen {
            return;
        }
        let next = match (self.form, self.preload) {
            (Form::Base, Preload::Off) => self.draw_form(engine),
            // GetPreloadedTransition.asm (0x801D460C) loads
            // TransformationID; SkipNormalDecision1.asm (0x801D4610)
            // branches over the draw to 0x801D465C.
            (Form::Base, Preload::Chosen(form)) => form,
            (Form::Base, Preload::Pending) => unreachable!("chosen above"),
            _ => Form::Base,
        };
        self.before_previous = self.previous;
        self.previous = Some(self.form);
        self.form = next;
        if next == Form::Base {
            self.phase = Phase::Announcing;
            return;
        }
        // grAnime_801C65B0(xD0), then lbFile_80016580 on datfiles[form].
        self.archive = None;
        if self.preload != Preload::Off {
            // SkipNormalDecision2.asm (0x801D4724) branches over the read
            // and the phase change to phase 1's poll at 0x801D4760, in this
            // same tick. Nothing set map 2's xC4_b1 (the skipped code does),
            // so grStadium_801D42B8 registers the archive whatever the disc
            // did; the read itself began a whole base duration earlier.
            self.archive = Some(self.form);
            self.phase = Phase::Announcing;
            return;
        }
        self.read_polls = 0;
        self.phase = Phase::Loading;
    }

    /// 0x801D4638: draw among the four forms, again while the draw equals
    /// xE2 (the form before the previous one).
    fn draw_form(&self, engine: &mut impl StadiumEngine) -> Form {
        loop {
            let form = FORMS[engine.rng().randi(FORMS.len() as i32) as usize];
            if self.before_previous != Some(form) {
                break form;
            }
        }
    }

    /// Phase 4 (grpstadium.c:2143-2171): 801D48C8 fnmsubs lowers the scale
    /// by `scale * (0.95 / sink_frames)`; 801D48A4 fmuls gives the floor.
    fn sink(&mut self, parameters: &Parameters, engine: &mut impl StadiumEngine) {
        let scale = engine.map_scale();
        let step = RISE_SCALE / parameters.sink_frames as f32;
        let lowered = fnmsubs(scale, step, engine.scale_y(self.standing));
        let floor = SUNK_SCALE * scale;
        if lowered > floor {
            engine.set_scale_y(self.standing, lowered);
            return;
        }
        engine.set_scale_y(self.standing, SUNK_SCALE);
        let old = self.timer;
        self.timer += 1;
        if old <= parameters.sunk_frames {
            return;
        }
        engine.freeze(self.standing);
        engine.create(self.form);
        engine.set_scale_y(self.form, floor);
        engine.set_translate_y(self.form, RISE_DEPTH);
        self.rising = Some(self.form);
        self.timer = 0;
        self.phase = Phase::Rising;
        for line in PIT_LINES {
            engine.enable_line(line);
        }
    }

    /// Phase 5 (grpstadium.c:2172-2215): no fused arithmetic (801D4B7C
    /// fmuls, fdivs, fadds, then fmuls by the scale; the depths are fmuls of
    /// `-10 * scale` by `1 - ratio`).
    fn rise(&mut self, parameters: &Parameters, engine: &mut impl StadiumEngine) {
        let scale = engine.map_scale();
        let rising = self.rising.expect("rising form");
        self.timer += 1;
        let frames = parameters.sink_frames;
        if self.timer <= frames {
            let grown = RISE_SCALE * self.timer as f32 / frames as f32 + SUNK_SCALE;
            engine.set_scale_y(rising, grown * scale);
            let half = frames / 2;
            let depth = RISE_DEPTH * scale;
            let rising_y = if self.timer < half {
                depth * (1.0 - self.timer as f32 / half as f32)
            } else {
                0.0
            };
            engine.set_translate_y(rising, rising_y);
            let falling_y = if self.timer > half {
                depth * (1.0 - (frames - self.timer) as f32 / (frames - half) as f32)
            } else {
                0.0
            };
            engine.set_translate_y(self.standing, falling_y);
        } else {
            engine.set_scale_y(rising, scale);
            engine.set_translate_y(rising, 0.0);
            self.phase = Phase::Settling;
            engine.destroy(self.standing);
            self.standing = rising;
            self.rising = None;
        }
    }

    /// `grStadium_801D435C` (0x801D435C): five sparkles at random points of
    /// the path. The length is summed with fadds in table order; each point
    /// is walk (fsubs), fdivs, three fmuls, the corner added, three fmuls by
    /// the map scale.
    fn sparkles(&mut self, engine: &mut impl StadiumEngine) {
        if self.sparkle_path == 0.0 {
            for (_, length) in SPARKLE_PATH {
                self.sparkle_path += length;
            }
        }
        for _ in 0..SPARKLES_PER_TICK {
            let mut walk = self.sparkle_path * engine.rng().randf();
            let mut leg = 0;
            while walk > SPARKLE_PATH[leg].1 && leg < LAST_LEG {
                walk -= SPARKLE_PATH[leg].1;
                leg += 1;
            }
            assert!(leg < LAST_LEG, "grpstadium.c:1991: walk past the path");
            let (corner, length) = SPARKLE_PATH[leg];
            let next = SPARKLE_PATH[leg + 1].0;
            let t = walk / length;
            let offset = Vec3::new(
                (next.x - corner.x) * t,
                (next.y - corner.y) * t,
                (next.z - corner.z) * t,
            );
            let point = Vec3::new(
                offset.x + corner.x,
                offset.y + corner.y,
                offset.z + corner.z,
            );
            let scale = engine.map_scale();
            engine.sparkle(Vec3::new(point.x * scale, point.y * scale, point.z * scale));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stadium::ModeWeights;

    /// The poll of the mock disc that finds a read complete.
    const READ_POLLS: u32 = 2;
    const SEED: u32 = 0x1234_5678;

    /// An arena with one scale per map and a disc that answers on a fixed poll.
    struct Arena {
        rng: HsdRng,
        scale_y: [f32; 10],
        polls: u32,
        announcements: Vec<ScreenMode>,
    }
    impl Arena {
        fn new() -> Self {
            Self {
                rng: HsdRng::new(SEED),
                scale_y: [1.0; 10],
                polls: 0,
                announcements: Vec::new(),
            }
        }
    }
    impl StadiumEngine for Arena {
        fn rng(&mut self) -> &mut HsdRng {
            &mut self.rng
        }
        fn read_completed(&mut self, _: Form, poll: u32) -> bool {
            self.polls += 1;
            poll >= READ_POLLS
        }
        fn announce(&mut self, mode: ScreenMode) {
            self.announcements.push(mode);
        }
        fn warn_standing(&mut self, _: Form) {}
        fn scale_y(&mut self, form: Form) -> f32 {
            self.scale_y[usize::from(form.map())]
        }
        fn set_scale_y(&mut self, form: Form, scale: f32) {
            self.scale_y[usize::from(form.map())] = scale;
        }
        fn set_translate_y(&mut self, _: Form, _: f32) {}
        fn freeze(&mut self, _: Form) {}
        fn create(&mut self, _: Form) {}
        fn destroy(&mut self, _: Form) {}
        fn settle(&mut self, _: Form) {}
        fn enable_line(&mut self, _: i32) {}
        fn disable_line(&mut self, _: i32) {}
        fn stitch_all(&mut self) {}
        fn sparkle(&mut self, _: Vec3) {}
        fn quake(&mut self) {}
        fn map_scale(&self) -> f32 {
            1.0
        }
    }

    /// Fixed durations, so only the form choice and the sparkles draw.
    fn parameters() -> Parameters {
        Parameters {
            base_frames: [3, 3],
            form_frames: [2, 2],
            announce_delay: 1,
            sink_frames: 4,
            sunk_frames: 1,
            info_frames: 0,
            defeat_frames: 0,
            announce_frames: 0,
            standings_frames: 0,
            player_camera_frames: [0, 0],
            stage_camera_frames: [0, 0],
            standings_interval: 0,
            mode_weights: ModeWeights {
                player_camera: 0,
                match_info: 0,
                stage_camera: 0,
                picture: 0,
            },
        }
    }

    fn started(preload: Preload, frozen: bool) -> (Transformation, Arena) {
        let mut arena = Arena::new();
        let mut controller = Transformation::new(&parameters(), &mut arena.rng);
        controller.waiting_for_start = false;
        controller.preload = preload;
        controller.frozen = frozen;
        (controller, arena)
    }

    /// The seed after `draws` draws from the start.
    fn seed_after(draws: usize) -> u32 {
        let mut rng = HsdRng::new(SEED);
        for _ in 0..draws {
            rng.randi(FORMS.len() as i32);
        }
        rng.seed
    }

    /// Ticks of a base duration of 3: the timer reads 3, 2, 1, 0, then -1.
    const WAIT_TICKS: usize = 5;

    #[test]
    fn retail_draws_the_form_when_the_wait_ends_and_polls_the_read() {
        let (mut controller, mut arena) = started(Preload::Off, false);
        for _ in 0..WAIT_TICKS - 1 {
            controller.tick(&parameters(), &mut arena);
            assert_eq!((controller.phase, arena.rng.seed), (Phase::Waiting, SEED));
        }
        controller.tick(&parameters(), &mut arena);
        assert_eq!(
            (controller.phase, arena.rng.seed),
            (Phase::Loading, seed_after(1))
        );
        controller.tick(&parameters(), &mut arena);
        assert_eq!(controller.phase, Phase::Loading);
        controller.tick(&parameters(), &mut arena);
        assert_eq!((controller.phase, arena.polls), (Phase::Announcing, 2));
        assert_eq!(controller.archive, Some(controller.form));
    }

    #[test]
    fn preload_draws_on_the_first_waiting_tick_and_announces_without_a_poll() {
        let (mut controller, mut arena) = started(Preload::Pending, false);
        controller.tick(&parameters(), &mut arena);
        let Preload::Chosen(form) = controller.preload else {
            panic!("no form chosen on the first waiting tick");
        };
        assert_eq!(arena.rng.seed, seed_after(1));
        for _ in 1..WAIT_TICKS {
            assert_eq!(controller.phase, Phase::Waiting);
            controller.tick(&parameters(), &mut arena);
        }
        assert_eq!(controller.phase, Phase::Announcing);
        assert_eq!((controller.form, controller.archive), (form, Some(form)));
        assert_eq!((arena.polls, arena.rng.seed), (0, seed_after(1)));
        controller.tick(&parameters(), &mut arena);
        assert_eq!(arena.announcements, [form.announcement()]);
    }

    #[test]
    fn preload_draws_again_the_tick_after_the_base_arena_settles() {
        let (mut controller, mut arena) = started(Preload::Pending, false);
        let mut choices = Vec::new();
        let mut base_settled = None;
        for tick in 0..200 {
            let before = (controller.phase, controller.form, controller.preload);
            controller.tick(&parameters(), &mut arena);
            if before.0 == Phase::Settling && before.1 == Form::Base {
                assert_eq!(controller.preload, Preload::Pending);
                base_settled.get_or_insert(tick);
            }
            if before.2 == Preload::Pending && controller.preload != Preload::Pending {
                choices.push(tick);
            }
            if choices.len() == 2 {
                break;
            }
        }
        let settled = base_settled.expect("the base arena returned");
        assert_eq!(choices, [0, settled + 1]);
        assert_eq!(arena.polls, 0);
    }

    #[test]
    fn frozen_stadium_never_leaves_the_wait() {
        for (preload, draws) in [(Preload::Off, 0), (Preload::Pending, 1)] {
            let (mut controller, mut arena) = started(preload, true);
            for _ in 0..50 {
                controller.tick(&parameters(), &mut arena);
            }
            assert_eq!(
                (controller.phase, controller.form, controller.timer),
                (Phase::Waiting, Form::Base, 3 - 50),
                "{preload:?}"
            );
            assert_eq!(arena.rng.seed, seed_after(draws), "{preload:?}");
            assert!(arena.announcements.is_empty());
        }
    }
}
