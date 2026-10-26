//! The turtle (map 1, grGreatBay_801F4694 / 801F499C): it waits at the
//! surface, dives, stays under for a while, and comes up again at one of
//! three places (its back to the stage, or facing away from either side),
//! with its shell (joints 1..4) a platform.
use super::{rand_range, Parameters};
use gekko_math::HsdRng;

/// grGreatBay_801F499C's states (xC4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// 0: floating, its wait counting down.
    Surface,
    /// 1: diving (animation 1).
    Dive,
    /// 2: coming up (animation 2).
    Rise,
    /// 3: under, hidden.
    Under,
}

/// The way it faces once up (xC5 b0123456, grGreatBay_801F62F8's result).
pub const FACING_STAGE: u8 = 0;
pub const FACING_FRONT: u8 = 1;
pub const FACING_BACK: u8 = 3;

/// The turtle's shell joints: B11 (mpLib_80058044) while it dives.
pub const SHELL_JOINTS: [i32; 4] = [2, 3, 4, 1];
/// The bone whose translation y the animation drives (Ground_801C3FA4(gobj, 2)).
pub const BODY_BONE: usize = 2;
/// The bone turned to face front or back (Ground_801C3FA4(gobj, 3)).
pub const HEAD_BONE: usize = 3;
/// grLib_801C97DC's splash bones as it rises.
pub const SPLASH_BONES: [usize; 2] = [34, 38];
/// grLib_801C96F8(0x7531, 30): the wake generator; 0x7533 the splashes.
pub const WAKE_PARTICLE: u32 = 0x7531;
pub const SPLASH_PARTICLE: u32 = 0x7533;
/// HSD_JObjSetRotationY's quarter turn (1.5707964f).
pub const QUARTER_TURN: f32 = 1.570_796_4;

/// What the scene does to the model after a step.
pub trait TurtleEngine {
    /// The stream the turtle's draws take (its effects draw from it too).
    fn rng(&mut self) -> &mut HsdRng;
    /// grAnime_801C84A4(gobj, 0, 7): the first animation looped this frame.
    fn rewound(&mut self) -> bool;
    /// grAnime_801C83D0(gobj, 0, 7): the first animation ended.
    fn ended(&mut self) -> bool;
    /// grAnime_801C8138(gobj, map, index).
    fn play(&mut self, index: usize);
    /// grAnime_801C7A04(gobj, 0, 7, 0.0): every animation stops.
    fn freeze(&mut self);
    /// HSD_JObjSetFlagsAll / ClearFlagsAll(JOBJ_HIDDEN) on the model.
    fn set_hidden(&mut self, hidden: bool);
    /// mpLib_80058044 / 80057FDC on a joint (set or clear B11).
    fn set_b11(&mut self, joint: i32, set: bool);
    /// mpJointListAdd / mpLib_80057BC0.
    fn enable_joint(&mut self, joint: i32, enabled: bool);
    /// HSD_JObjSetTranslateX on the root (the GObj's JObj).
    fn set_root_x(&mut self, x: f32);
    /// The wake (grLib_801C96F8 at the root) starts; any earlier one goes.
    fn start_wake(&mut self);
    fn stop_wake(&mut self);
    /// grLib_801C97DC(0x7533, 30, bone).
    fn splash(&mut self, bone: usize);
    /// HSD_JObjGetTranslationY / SetTranslateY on the body bone.
    fn body_y(&mut self) -> f32;
    fn set_body_y(&mut self, y: f32);
    /// HSD_JObjSetRotationY on a bone.
    fn set_rotation_y(&mut self, bone: usize, angle: f32);
}

/// Map 1's `grGreatBay_GroundVars`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turtle {
    pub phase: Phase,
    /// xC5 b0123456: how it faces once up.
    pub facing: u8,
    /// xC5 b7: the item draw for this rise is done (or not owed).
    pub item_checked: bool,
    /// xC6.
    pub timer: i16,
    /// xC8 != NULL: the wake generator lives.
    pub wake: bool,
    /// xCC: the body bone's last scaled height.
    pub body_y: f32,
}

impl Turtle {
    /// grGreatBay_801F4694's state, once the model is placed: its wait.
    pub fn new(parameters: &Parameters, body_y: f32, rng: &mut HsdRng) -> Self {
        let [a, b] = parameters.turtle_wait;
        Self {
            phase: Phase::Surface,
            facing: FACING_STAGE,
            item_checked: true,
            timer: rand_range(i32::from(b), i32::from(a), rng) as i16,
            wake: false,
            body_y,
        }
    }

    /// grGreatBay_801F62F8 (0x801F62F8): a facing by kame_dir_prob
    /// (subtracted in the order 2, 1, 0, 3), drawn again while it is the
    /// current one, at most 1001 times (then 0).
    fn draw_facing(current: u8, parameters: &Parameters, rng: &mut HsdRng) -> u8 {
        let p = parameters.turtle_dir_prob.map(i32::from);
        let mut tries = 0;
        loop {
            let total = p[0] + (p[2] + (p[1] + p[3]));
            let mut r = if total != 0 { rng.randi(total) } else { 0 };
            r -= p[2];
            let result = if r < 0 {
                1
            } else {
                r -= p[1];
                if r < 0 {
                    2
                } else {
                    r -= p[0];
                    if r < 0 {
                        0
                    } else {
                        r -= p[3];
                        if r < 0 {
                            3
                        } else {
                            unreachable!("grgreatbay.c:1277: kame_dir_prob")
                        }
                    }
                }
            };
            if u8::try_from(result).unwrap() != current {
                return result as u8;
            }
            let previous = tries;
            tries += 1;
            if previous > 1000 {
                return 0;
            }
        }
    }

    /// grGreatBay_801F499C's state switch and its body and head writes
    /// (the collision update, the shell's vertices and the item draw are
    /// the scene's).
    pub fn step(&mut self, parameters: &Parameters, engine: &mut impl TurtleEngine) {
        match self.phase {
            Phase::Surface => {
                if self.timer < 0 {
                    if engine.rewound() {
                        self.phase = Phase::Dive;
                        engine.play(1);
                        self.timer = 0;
                        for joint in SHELL_JOINTS {
                            engine.set_b11(joint, true);
                        }
                    }
                } else {
                    self.timer -= 1;
                }
            }
            Phase::Dive => {
                let old = self.timer;
                self.timer = old.wrapping_add(1);
                if old == 90 {
                    // Ground_801C53EC(0x57E42): sound.
                    engine.start_wake();
                    self.wake = true;
                }
                if engine.ended() {
                    self.phase = Phase::Under;
                    let [a, b] = parameters.turtle_rebirth;
                    self.timer = rand_range(i32::from(b), i32::from(a), engine.rng()) as i16;
                    engine.freeze();
                    engine.set_hidden(true);
                    if std::mem::take(&mut self.wake) {
                        engine.stop_wake();
                    }
                }
            }
            Phase::Rise => {
                engine.set_hidden(false);
                self.timer = self.timer.wrapping_add(1);
                if self.timer == 60 {
                    // Ground_801C53EC(0x57E41): sound.
                    engine.start_wake();
                    self.wake = true;
                }
                if matches!(self.timer, 60 | 120) {
                    engine.splash(SPLASH_BONES[0]);
                }
                if matches!(self.timer, 90 | 150) {
                    engine.splash(SPLASH_BONES[1]);
                }
                if engine.ended() {
                    self.phase = Phase::Surface;
                    let [a, b] = parameters.turtle_wait;
                    self.timer = rand_range(i32::from(b), i32::from(a), engine.rng()) as i16;
                    engine.play(0);
                    for joint in SHELL_JOINTS {
                        engine.set_b11(joint, false);
                    }
                    if std::mem::take(&mut self.wake) {
                        engine.stop_wake();
                    }
                }
            }
            Phase::Under => {
                let old = self.timer;
                self.timer = old.wrapping_sub(1);
                if old < 0 {
                    self.rise(parameters, engine);
                }
            }
        }
        // The body's height scaled by kame_ud_scale whenever the animation
        // moved it (0x801F5190: fmuls).
        let y = engine.body_y();
        if self.body_y != y {
            let y = y * parameters.turtle_ud_scale;
            engine.set_body_y(y);
            self.body_y = y;
        }
        match self.facing {
            FACING_FRONT => engine.set_rotation_y(HEAD_BONE, QUARTER_TURN),
            FACING_STAGE => engine.set_rotation_y(BODY_BONE, 0.0),
            FACING_BACK => engine.set_rotation_y(HEAD_BONE, -QUARTER_TURN),
            _ => unreachable!("grgreatbay.c:692: turtle facing {}", self.facing),
        }
    }

    /// grGreatBay_801F499C case 3's end: animation 2, a new facing, and a
    /// place for it (kame_x plus rand_range of the offsets, each fctiwz'd;
    /// 0x801F4E4C: fsubs, fadds), the shell or the head a platform.
    fn rise(&mut self, parameters: &Parameters, engine: &mut impl TurtleEngine) {
        self.phase = Phase::Rise;
        engine.play(2);
        self.timer = 0;
        self.item_checked = false;
        self.facing = Self::draw_facing(self.facing, parameters, engine.rng());
        let offsets = match self.facing {
            FACING_FRONT | FACING_BACK => parameters.turtle_x_fb_offset,
            FACING_STAGE => parameters.turtle_x_lr_offset,
            _ => unreachable!("grgreatbay.c:652: turtle facing {}", self.facing),
        };
        let [a, b] = offsets.map(gekko_math::msl::fctiwz);
        let offset = rand_range(b, a, engine.rng());
        engine.set_root_x(parameters.turtle_x + offset as f32);
        let back = self.facing == FACING_STAGE;
        for joint in [2, 3, 4] {
            engine.enable_joint(joint, back);
        }
        engine.enable_joint(1, !back);
    }
}
