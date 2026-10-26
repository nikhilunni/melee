//! The ship's moving parts: the deck's tilt (grRCruise_8020071C /
//! 80200578), the cloud blocks (80200C04 / 802010A4), the swinging
//! platforms (80200154 / fn_80200460) and the appearing blocks (80201410 /
//! 80201588).
use super::Parameters;
use gekko_math::{
    fma::{fmadds, fnmsubs},
    msl::{fctiwz, sqrtf},
    HsdRng,
};

/// grRCruise_80200578's pivot: map 1's bone 8, which tilts (and map 5's).
pub const TILT_BONE: usize = 8;
/// The deck's collision joint (mpJointSetCb1(5)).
pub const DECK_JOINT: i32 = 5;
/// MTXDegToRad(1).
const DEGREE: f32 = 0.017453292;

/// The deck's tilt (`rcruise` x14..x38).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tilt {
    /// x18: the angle in degrees.
    pub angle: f32,
    /// x1C: the turn's sign.
    pub direction: f32,
    /// x20: degrees per tick.
    pub speed: f32,
    /// x24 / x28: the weight right and left of the pivot.
    pub right: f32,
    pub left: f32,
    /// x2C: the mode (only 0 is ever set).
    pub mode: i32,
    /// x30 / x34: last tick's and this tick's callers.
    pub last_callers: i32,
    pub callers: i32,
    /// x38.
    pub timer: i32,
}

impl Tilt {
    /// grRCruise_80200578 (0x80200578): a fighter, item or enemy on the
    /// deck (joint 5) at `position`, weighing `x50`, with the pivot at
    /// `pivot`.
    pub fn stand(&mut self, collision_type: u8, x50: i32, position: (f32, f32), pivot: (f32, f32)) {
        if !(1..=3).contains(&collision_type) {
            return;
        }
        let mut weight = x50;
        if weight as f32 > 1000.0 {
            weight = 1000;
        }
        let dx = pivot.0 - position.0;
        let dy = pivot.1 - position.1;
        // retail 0x8020061C: fmadds; the inline sqrtf's three fnmsubs.
        let distance = sqrtf(fmadds(dy, dy, dx * dx));
        if distance > 4.0 {
            let share = weight as f32 / 1000.0;
            // retail 0x802006B8 / 0x802006EC: fmadds.
            if pivot.0 < position.0 {
                self.right = fmadds(distance, share, self.right);
            } else {
                self.left = fmadds(distance, share, self.left);
            }
        }
        self.callers += 1;
    }

    /// grRCruise_8020071C (0x8020071C): the turn; returns bone 8's new
    /// rotation z (map 1's and map 5's).
    pub fn step(&mut self, p: &Parameters) -> f32 {
        let magnitude = if self.angle < 0.0 { -self.angle } else { self.angle };
        // retail 0x802007B0: fnmsubs over the truncated turn count.
        let turns = fctiwz(magnitude / 360.0) as f32;
        let wrapped = fnmsubs(360.0, turns, magnitude);
        match self.mode {
            0 => {
                if self.callers == 0 {
                    self.right_itself(p, wrapped);
                } else if self.right < self.left {
                    self.direction = 1.0;
                    self.speed = p.tilt_gain * (self.left - self.right);
                    if self.speed >= p.tilt_max {
                        self.speed = p.tilt_max;
                    }
                } else {
                    self.direction = -1.0;
                    self.speed = -p.tilt_gain * (self.right - self.left);
                    if self.speed <= -p.tilt_max {
                        self.speed = -p.tilt_max;
                    }
                }
            }
            1 => unimplemented!("grrcruise.c:743: the tilt's mode 1 (x2C) is never set"),
            _ => {}
        }
        self.angle += self.speed;
        self.last_callers = self.callers;
        self.callers = 0;
        self.left = 0.0;
        self.right = 0.0;
        DEGREE * self.angle
    }

    fn right_itself(&mut self, p: &Parameters, wrapped: f32) {
        self.direction = if self.angle < 0.0 { 1.0 } else { -1.0 };
        self.speed = p.tilt_return * self.direction;
        if wrapped <= 0.2 {
            self.speed = 0.0;
            self.mode = 0;
        }
    }
}

/// grRc_803E4FF0: the seventeen cloud blocks' bones on map 1.
pub const CLOUD_BONES: [usize; 17] = [
    0x20, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26,
    0x27, 0x28,
];

/// One cloud block (`grRCruise_Entry`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cloud {
    pub state: u8,
    /// pad_01 b0: shown in the return blink.
    pub blink: bool,
    /// x2: its collision joint.
    pub joint: i32,
    pub timer: i32,
    /// x8: fighters on it this tick.
    pub count: i32,
    /// xC: its bone's rest height; x10 its fall speed.
    pub base_y: f32,
    pub speed: f32,
}

/// What a cloud step asks of the stage.
pub trait CloudEngine {
    fn translate_y(&self, cloud: usize) -> f32;
    fn set_translate_y(&mut self, cloud: usize, y: f32);
    /// grRCruise_80201B60: the DObjs shown or hidden (drawing only).
    fn show(&mut self, cloud: usize, shown: bool);
    fn enable_joint(&mut self, joint: i32);
    fn disable_joint(&mut self, joint: i32);
    /// mpLib_80055E9C then mpLib_80057424.
    fn snap_joint(&mut self, joint: i32);
    /// Stage_GetCamBoundsBottomOffset.
    fn camera_bottom(&self) -> f32;
}

impl Cloud {
    /// grRCruise_80200B48 (0x80200B48).
    pub fn new(joint: i32, base_y: f32) -> Self {
        Self {
            state: 0,
            blink: false,
            joint,
            timer: 0,
            count: 0,
            base_y,
            speed: 0.0,
        }
    }

    /// grRCruise_802010A4 (0x802010A4): a fighter stands on it.
    pub fn stand(&mut self, collision_type: u8) {
        if collision_type != 1 {
            return;
        }
        if self.state == 0 {
            self.timer = 0;
            self.state = 1;
        }
        self.count += 1;
    }

    /// grRCruise_80200C04 (0x80200C04) for one cloud.
    pub fn step<E: CloudEngine>(&mut self, index: usize, p: &Parameters, rng: &mut HsdRng, engine: &mut E) {
        match self.state {
            1 => {
                if self.count == 0 {
                    self.timer = 0;
                    self.state = 0;
                } else if self.timer >= p.cloud_wait {
                    self.state = 2;
                } else {
                    self.timer += 1;
                }
            }
            2 => {
                if self.timer % p.cloud_shake_period == 0 {
                    let range = 100.0 * (p.cloud_shake[1] - p.cloud_shake[0]);
                    let span = fctiwz(range);
                    let draw = if span != 0 { rng.randi(span) } else { 0 };
                    let y = self.base_y + (p.cloud_shake[0] + draw as f32 / 100.0);
                    engine.set_translate_y(index, y);
                }
                if self.timer >= p.cloud_shake_time {
                    self.speed = 0.0;
                    self.state = 3;
                }
                self.timer += 1;
            }
            3 => {
                self.speed -= p.cloud_fall_acceleration;
                let magnitude = if self.speed < 0.0 { -self.speed } else { self.speed };
                if magnitude > p.cloud_fall_max {
                    self.speed = -p.cloud_fall_max;
                }
                let y = engine.translate_y(index) + self.speed;
                engine.set_translate_y(index, y);
                if engine.translate_y(index) <= engine.camera_bottom() {
                    self.speed = 0.0;
                    engine.show(index, false);
                    engine.disable_joint(self.joint);
                    self.timer = 0;
                    self.state = 4;
                }
            }
            4 => {
                if self.timer >= p.cloud_away {
                    engine.set_translate_y(index, self.base_y);
                    engine.snap_joint(self.joint);
                    engine.show(index, true);
                    engine.enable_joint(self.joint);
                    self.timer = 0;
                    self.blink = false;
                    self.state = 5;
                }
                self.timer += 1;
            }
            5 => {
                if self.timer % p.cloud_blink == 0 {
                    self.blink = !self.blink;
                    engine.show(index, !self.blink);
                }
                if self.timer >= p.cloud_blink_time {
                    engine.show(index, true);
                    self.timer = 0;
                    self.state = 0;
                }
                self.timer += 1;
            }
            _ => {}
        }
        self.count = 0;
    }
}

/// grRc_804D4790: the swinging platforms' bones on map 4.
pub const SWING_BONES: [usize; 3] = [0x01, 0x1A, 0x33];
/// The swing's animation (grAnime_801C8098(gobj, bone, 7, 1, 0, 0)).
pub const SWING_ANIMATION: usize = 1;

/// One swinging platform (`rcruise.x3C[i]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swing {
    pub state: u8,
    pub blink: bool,
    pub joint: i32,
    pub timer: i32,
    pub count: i32,
}

/// What a swing step asks of the stage.
pub trait SwingEngine {
    /// grAnime_801C7A94(gobj, bone, 1, rate): the swing's joint clocks.
    fn set_rate(&mut self, swing: usize, rate: f32);
    /// grAnime_801C7BA0(gobj, bone, 1, frame).
    fn request_frame(&mut self, swing: usize, frame: f32);
    /// grAnime_801C83D0(gobj, bone, 7).
    fn ended(&self, swing: usize) -> bool;
    /// grRCruise_80201B60 on the bone's child (drawing only).
    fn show(&mut self, swing: usize, shown: bool);
    fn enable_joint(&mut self, joint: i32);
    fn disable_joint(&mut self, joint: i32);
    fn snap_joint(&mut self, joint: i32);
}

impl Swing {
    /// grRCruise_80200074 (0x80200074).
    pub fn new(joint: i32) -> Self {
        Self {
            state: 0,
            blink: false,
            joint,
            timer: 0,
            count: 0,
        }
    }

    /// fn_80200460 (0x80200460): a fighter lands on it; the swing starts
    /// (or starts again) at once.
    pub fn stand<E: SwingEngine>(&mut self, index: usize, collision_type: u8, engine: &mut E) {
        if collision_type != 1 {
            return;
        }
        if matches!(self.state, 1 | 3 | 4) {
            self.timer = 0;
            engine.show(index, true);
            engine.set_rate(index, 1.0);
            self.state = 2;
        }
        self.count += 1;
    }

    /// grRCruise_80200154 (0x80200154) for one swing.
    pub fn step<E: SwingEngine>(&mut self, index: usize, p: &Parameters, engine: &mut E) {
        match self.state {
            0 => {
                engine.enable_joint(self.joint);
                engine.show(index, true);
                self.timer = 0;
                self.state = 1;
            }
            2 => {
                if self.count == 0 {
                    self.timer = 0;
                    engine.set_rate(index, 0.0);
                    self.state = 3;
                } else if engine.ended(index) {
                    self.timer = 0;
                    self.state = 5;
                }
            }
            3 => {
                if self.timer >= p.swing_wait {
                    self.timer = 0;
                    self.blink = false;
                    self.state = 4;
                }
                self.timer += 1;
            }
            4 => {
                if self.timer % p.swing_blink == 0 {
                    self.blink = !self.blink;
                    engine.show(index, !self.blink);
                }
                if self.timer >= p.swing_blink_time {
                    self.reset(index, engine);
                }
                self.timer += 1;
            }
            5 => {
                engine.show(index, self.timer % p.swing_end_blink != 0);
                if self.timer >= p.swing_end_time {
                    self.reset(index, engine);
                }
                self.timer += 1;
            }
            _ => {}
        }
        self.count = 0;
    }

    /// Back to the start: joint off, hidden, the swing at frame 0 and
    /// stopped, the joint moved there.
    fn reset<E: SwingEngine>(&mut self, index: usize, engine: &mut E) {
        engine.disable_joint(self.joint);
        engine.show(index, false);
        engine.request_frame(index, 0.0);
        engine.set_rate(index, 0.0);
        engine.snap_joint(self.joint);
        self.state = 0;
    }
}

/// `Map_VanishDesc` lbl_803E5014: (map 1 bone, collision joint, always
/// there).
pub const VANISH: [(usize, i32, bool); 20] = [
    (6, 4, false),
    (8, 5, false),
    (16, 10, false),
    (11, 6, false),
    (12, 7, false),
    (13, 8, false),
    (14, 9, false),
    (15, 8, false),
    (19, 12, true),
    (20, 13, true),
    (21, 14, true),
    (30, 15, true),
    (22, 16, true),
    (23, 17, true),
    (24, 18, true),
    (25, 19, true),
    (26, 20, true),
    (27, 21, true),
    (28, 22, true),
    (29, 23, true),
];
/// Camera_8003118C(&pos, -20): inside the bounds by 20.
pub const VANISH_MARGIN: i32 = -20;

/// What the appearing blocks ask of the stage.
pub trait VanishEngine {
    /// lb_8000B1CC of the block's bone, inside the camera bounds by 20.
    fn on_screen(&mut self, block: usize) -> bool;
    /// grAnime_801C7FF8(gobj, bone, 2, index, 0, 1): its material clocks.
    fn play(&mut self, block: usize, index: usize);
    /// grAnime_801C83D0(gobj, bone, 2).
    fn ended(&self, block: usize) -> bool;
    fn enable_joint(&mut self, joint: i32);
    fn disable_joint(&mut self, joint: i32);
    fn snap_joint(&mut self, joint: i32);
}

/// grRCruise_80201588 (0x80201588): each block comes in when its bone is
/// in the camera bounds and goes when it leaves; a joint two blocks share
/// goes only once both have.
pub fn step_vanish<E: VanishEngine>(states: &mut [u8; 20], engine: &mut E) {
    for i in 0..VANISH.len() {
        let (_, joint, always) = VANISH[i];
        if always {
            continue;
        }
        match states[i] {
            0 => {
                if engine.on_screen(i) {
                    states[i] = 1;
                    engine.play(i, 2);
                    engine.enable_joint(joint);
                    engine.snap_joint(joint);
                }
            }
            1 => {
                if engine.ended(i) {
                    states[i] = 2;
                }
            }
            2 => {
                if !engine.on_screen(i) {
                    states[i] = 3;
                    engine.play(i, 3);
                }
            }
            3 if engine.ended(i) => {
                states[i] = 0;
                engine.play(i, 1);
                let shared = (0..VANISH.len()).any(|j| VANISH[j].1 == joint && states[j] != 0);
                if !shared {
                    engine.disable_joint(joint);
                }
            }
            _ => {}
        }
    }
}
