//! Mushroom Kingdom II (grinishie2.c, GrI2.dat, StKind 0x14).
//!
//! Map 2's controller counts four timers down. Each one, spent, makes an
//! event map: the two lifts (maps 3 and 4) that rise out of the waterfall,
//! Birdo's platform (map 15, which makes Birdo herself, an item), and
//! Pidgit's carpet (map 1). An event map that ends rearms its timer.
pub mod procs;

use gekko_math::HsdRng;
use hsd_types::Vec3;

/// `grInishie2_YakumonoParam` (grinishie2.c:166-181).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// unk0/2 and unk4/6: each lift's wait, `base + rand(range)`.
    pub lift_waits: [[i16; 2]; 2],
    /// unk8/A: Birdo's wait.
    pub birdo_wait: [i16; 2],
    /// unkC/E: the carpet's wait.
    pub carpet_wait: [i16; 2],
    /// unk10: each lift's delay before its collision joint joins.
    pub lift_delays: [i16; 2],
    /// unk14: Birdo's spawn offset from the right (0) and left (1) blast zone.
    pub birdo_spawn: [Vec3; 2],
    /// unk2C: Birdo's platform's height above her.
    pub birdo_platform_height: f32,
    /// unk30: the carpet's spawn offset from the right and left blast zone.
    pub carpet_spawn: [Vec3; 2],
    /// unk48/4A: the carpet's odds of keeping and of turning its course.
    pub carpet_turn_odds: [i16; 2],
}

/// grinishie2.c's `test_random`: no draw for an empty range.
pub fn test_random(range: i32, rng: &mut HsdRng) -> i32 {
    if range != 0 {
        rng.randi(range)
    } else {
        0
    }
}

/// One of map 2's countdowns (an `xC4` flag and its s16 count).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timer {
    pub armed: bool,
    pub ticks: i16,
}

impl Timer {
    /// grInishie2_801FD018 / 801FD368 / 801FD744 and the rearms:
    /// `base + test_random(range)`, truncated to s16.
    pub fn arm(&mut self, [base, range]: [i16; 2], rng: &mut HsdRng) {
        self.armed = true;
        self.ticks = (i32::from(base) + test_random(i32::from(range), rng)) as i16;
    }
    /// `s16 old = x--; if (old < 0)`: true once the count was spent, which
    /// disarms it.
    pub fn tick(&mut self) -> bool {
        if !self.armed {
            return false;
        }
        let old = self.ticks;
        self.ticks = old.wrapping_sub(1);
        if old < 0 {
            self.armed = false;
            return true;
        }
        false
    }
}

/// What map 2's proc makes this tick (grInishie2_801FCFCC), in order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spawns {
    pub lifts: [bool; 2],
    pub birdo: bool,
    pub carpet: bool,
}

/// Map 2's `GroundVars_inishie2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Controller {
    /// b1/xC6 and b2/xC8.
    pub lifts: [Timer; 2],
    /// b3/xCA.
    pub birdo: Timer,
    /// b4/xCC.
    pub carpet: Timer,
    /// b0: grInishie2_801FCF2C decided the music (sound only).
    pub music_decided: bool,
}

impl Controller {
    /// grInishie2_801FCE04's draws: 801FD018 (lift A, then lift B),
    /// 801FD368 (Birdo), 801FD744 (the carpet).
    pub fn new(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        let mut controller = Self {
            lifts: [Timer::default(); 2],
            birdo: Timer::default(),
            carpet: Timer::default(),
            music_decided: false,
        };
        for (lift, wait) in controller.lifts.iter_mut().zip(parameters.lift_waits) {
            lift.arm(wait, rng);
        }
        controller.birdo.arm(parameters.birdo_wait, rng);
        controller.carpet.arm(parameters.carpet_wait, rng);
        controller
    }
    /// grInishie2_801FD0B4, 801FD3CC and 801FD7A8's countdowns. The scene
    /// makes each map as its timer fires, in this order.
    pub fn lift_due(&mut self, lift: usize) -> bool {
        self.lifts[lift].tick()
    }
    pub fn birdo_due(&mut self) -> bool {
        self.birdo.tick()
    }
    pub fn carpet_due(&mut self) -> bool {
        self.carpet.tick()
    }
}

/// A lift's own Ground (maps 3 and 4, grInishie2_801FD198).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lift {
    /// xC8: 0 for map 3, 1 for map 4.
    pub index: usize,
    /// xC6: ticks before its collision joint joins.
    pub delay: i16,
    /// b0: the joint joined.
    pub joined: bool,
}

impl Lift {
    pub fn new(index: usize, parameters: &Parameters) -> Self {
        Self {
            index,
            delay: parameters.lift_delays[index],
            joined: false,
        }
    }
    /// grInishie2_801FD224's countdown: true on the tick the collision
    /// joint joins (mpJointListAdd 0xC or 0xD).
    pub fn step(&mut self) -> bool {
        if self.joined {
            return false;
        }
        let old = self.delay;
        self.delay = old.wrapping_sub(1);
        if old < 0 {
            self.joined = true;
            return true;
        }
        false
    }
}

/// Where a horizontal position stands against the camera's bounds
/// (grInishie2_801FD9EC's inline): 2 past the right, 1 past the left.
pub fn outside_camera(x: f32, left: f32, right: f32) -> u8 {
    if x > right {
        2
    } else if x < left {
        1
    } else {
        0
    }
}

/// Pidgit's carpet (map 1, `grInishie2_GroundVars3`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Carpet {
    /// b0: on its way out; leaving the blast zones ends it.
    pub leaving: bool,
    /// b1: still flying in from beyond the camera.
    pub entering: bool,
    /// b2: it has chosen to fly out.
    pub exiting: bool,
    /// b3: the animation it plays (0 or 1), a one-bit field.
    pub animation: u8,
    /// xCA: flights finished since it came in.
    pub flights: i16,
    /// xCC: bone 1's translation at the last unfinished tick.
    pub step: Vec3,
    /// xD8: where the model's root stands.
    pub root: Vec3,
}

impl Carpet {
    /// grInishie2_801FD824 (0x801FD824): HSD_Randi(2) picks the side; it
    /// starts beyond the blast zone on that side at `carpet_spawn`, facing
    /// in (animation 1 from the right, 0 from the left).
    pub fn spawn(parameters: &Parameters, blast_left: f32, blast_right: f32, rng: &mut HsdRng) -> Self {
        let side = rng.randi(2);
        let offset = parameters.carpet_spawn[side as usize];
        let x = if side == 0 {
            offset.x + blast_right
        } else {
            offset.x + blast_left
        };
        Self {
            leaving: false,
            entering: true,
            exiting: false,
            animation: u8::from(side == 0),
            flights: 0,
            step: Vec3::ZERO,
            root: Vec3::new(x, offset.y, offset.z),
        }
    }
    /// grInishie2_801FD9EC's choice when a flight's animation ends
    /// (0x801FDA30..0x801FDB80): an entering carpet keeps on until it is
    /// within the camera; then the first two flights turn back, and later
    /// ones leave once outside the camera or turn by the odds
    /// (`test_random(keep + turn) >= keep`).
    pub fn flight_ended(&mut self, parameters: &Parameters, left: f32, right: f32, rng: &mut HsdRng) {
        // retail 0x801FDA48 / 0x801FDAC8: fadds.
        let x = self.root.x + self.step.x;
        if self.entering {
            if outside_camera(x, left, right) == 0 {
                self.entering = false;
            }
            return;
        }
        if !self.exiting {
            if self.flights < 2 {
                self.animation ^= 1;
            } else {
                let side = outside_camera(x, left, right);
                if side != 0 {
                    self.exiting = true;
                    // rlwimi into the one-bit field: 2 stores 0.
                    self.animation = side & 1;
                    self.leaving = true;
                } else {
                    let [keep, turn] = parameters.carpet_turn_odds;
                    let range = i32::from((i32::from(keep) + i32::from(turn)) as i16);
                    if test_random(range, rng) >= i32::from(keep) {
                        self.animation ^= 1;
                    }
                }
            }
        }
        self.flights = self.flights.wrapping_add(1);
    }
    /// The root after a flight: the old root plus the flight's last step
    /// (retail 0x801FDC68 / 0x801FDCE4 / 0x801FDD60: fadds per axis).
    pub fn advance(&mut self) -> Vec3 {
        self.root = Vec3::new(
            self.root.x + self.step.x,
            self.root.y + self.step.y,
            self.root.z + self.step.z,
        );
        self.root
    }
}

/// grInishie2_801FD4F0's side: HSD_Randi(2), 0 on the right walking left.
pub fn birdo_spawn(
    parameters: &Parameters,
    blast_left: f32,
    blast_right: f32,
    rng: &mut HsdRng,
) -> (Vec3, f32) {
    let side = rng.randi(2);
    let offset = parameters.birdo_spawn[side as usize];
    let x = if side == 0 {
        offset.x + blast_right
    } else {
        offset.x + blast_left
    };
    // sign_inline: side 0 faces left.
    let facing = if side == 0 { -1.0 } else { 1.0 };
    (Vec3::new(x, offset.y, offset.z), facing)
}
