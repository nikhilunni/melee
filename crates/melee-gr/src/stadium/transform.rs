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

/// Polls of `grStadium_801D42B8` until the DVD callback of each form
/// archive's read (lbFile_80016580, fn_801D4220) has run, counting the
/// successful one; indexed by `Form::archive`. Retail's latency is the
/// emulated disc's: these are measured from retail recordings (the poll
/// that succeeds is the tick before the announcement), not derived.
const LOAD_POLLS: [Option<u32>; 4] = [Some(23), Some(18), Some(23), Some(21)];

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
    /// Map 2's xC4_b1 as polls left: the form archive's read in flight.
    pub load_polls_left: u32,
    /// xD0: the registered form archive (grDatFiles_801C6478), until the
    /// next form is chosen (grAnime_801C65B0).
    pub archive: Option<Form>,
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
            load_polls_left: 0,
            archive: None,
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
                self.load_polls_left -= 1;
                if self.load_polls_left == 0 {
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
        // Signed post-decrement: a timer of zero waits one more tick.
        let old = self.timer;
        self.timer -= 1;
        if old >= 0 {
            return;
        }
        let next = if self.form == Form::Base {
            loop {
                let form = FORMS[engine.rng().randi(FORMS.len() as i32) as usize];
                if self.before_previous != Some(form) {
                    break form;
                }
            }
        } else {
            Form::Base
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
        let file = next.archive().expect("a form's archive");
        self.load_polls_left = LOAD_POLLS[file].unwrap_or_else(|| {
            unimplemented!(
                "grpstadium.c:2096: DVD read latency of GrPs{}.dat",
                file + 1
            )
        });
        self.phase = Phase::Loading;
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
