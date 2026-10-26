//! Great Bay (grgreatbay.c, GrGb.dat, StKind 0x0D).
//!
//! The lab's floating platform (map 2, bone 49) tilts, slides and sinks
//! under whoever lands on it; the turtle (map 1) dives and surfaces at one
//! of three places, its shell and head a platform; the moon and the four
//! giants (maps 9, 5..8) wait thousands of frames to move; Tingle (map 10,
//! an item) floats over the stage on his balloon.
pub mod procs;
pub mod turtle;

use gekko_math::{fma::fmadds, HsdRng};

/// `grGb_StageAttr` (grgreatbay.c:153-190).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// moon_fall_wait_a / _b.
    pub moon_fall_wait: [i16; 2],
    /// floatfloor_landing_rate: a landing's weight scale.
    pub float_landing_rate: f32,
    /// floatfloor_slant_mul, _add, _limit, _rate, _reb_rate.
    pub float_slant: [f32; 5],
    /// floatfloor_slide_mul, _add, _limit, _rate, _reb_rate.
    pub float_slide: [f32; 5],
    /// floatfloor_down_mul, _add, _limit, _up_rate, _down_rate.
    pub float_down: [f32; 5],
    /// kame_wait_frame_a / _b.
    pub turtle_wait: [i16; 2],
    /// kame_rebirth_frame_a / _b.
    pub turtle_rebirth: [i16; 2],
    pub turtle_x: f32,
    pub turtle_y: f32,
    pub turtle_x_offset_init: f32,
    /// kame_x_lr_offset_a / _b and kame_x_fb_offset_a / _b.
    pub turtle_x_lr_offset: [f32; 2],
    pub turtle_x_fb_offset: [f32; 2],
    pub turtle_scale: f32,
    pub turtle_ud_scale: f32,
    pub turtle_dir_prob: [i16; 4],
    pub turtle_item_prob: f32,
    /// (kind, weight) of the items the turtle may bring up.
    pub turtle_items: [(i16, i16); 10],
}

/// gr/inlines.h `rand_range(a, b)`: `b + rand(a - b)` or `a + rand(b - a)`,
/// no draw for equal ends.
pub fn rand_range(a: i32, b: i32, rng: &mut HsdRng) -> i32 {
    if a > b {
        b + rng.randi(a - b)
    } else if a < b {
        a + rng.randi(b - a)
    } else {
        a
    }
}

const LIMIT: usize = 2;
const RATE: usize = 3;
const REBOUND_RATE: usize = 4;
/// fn_801F60C4's degrees to radians (@854).
const DEGREES: f32 = 0.017_453_292;
/// fn_801F60C4: a lander this far from the centre tips it fully (@853).
const FULL_TIP_DISTANCE: f32 = 15.0;

/// The lab's floating platform (map 2's `grGreatBay_GroundVars3`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloatFloor {
    /// translation.x / .y: bone 49's authored translation.
    pub rest_x: f32,
    pub rest_y: f32,
    /// translation.z, xD0, xD4: this frame's tilt, slide and sink targets.
    pub slant_target: f32,
    pub slide_target: f32,
    pub down_target: f32,
    /// xD8, xDC, xE0: where it stands.
    pub slant: f32,
    pub slide: f32,
    pub down: f32,
}

impl FloatFloor {
    /// grGreatBay_801F5D4C: at rest at bone 49's translation.
    pub fn new(rest_x: f32, rest_y: f32) -> Self {
        Self {
            rest_x,
            rest_y,
            ..Self::default()
        }
    }

    /// grGreatBay_801F60C4 (0x801F60C4), the joint's floor callback for a
    /// fighter or stage enemy (x34 b1234 1 or 3): a landing weighs x50
    /// times floatfloor_landing_rate (fctiwz); off-centre landings tip and
    /// slide it toward their side (0x801F61DC, 0x801F6200: fmadds), every
    /// one sinks it (0x801F620C: fmadds), each clamped at its limit.
    /// `platform_x` is bone 49's world x (lb_8000B1CC).
    pub fn weigh(
        &mut self,
        parameters: &Parameters,
        x50: i32,
        landing: bool,
        lander_x: f32,
        platform_x: f32,
    ) {
        let x50 = if landing {
            gekko_math::msl::fctiwz(x50 as f32 * parameters.float_landing_rate)
        } else {
            x50
        };
        let dx = lander_x - platform_x;
        let distance = if dx < 0.0 { -dx } else { dx };
        let mut t = distance / FULL_TIP_DISTANCE;
        if t > 1.0 {
            t = 1.0;
        }
        let weight = x50 as f32;
        let [slant_mul, slant_add, ..] = parameters.float_slant;
        let [slide_mul, slide_add, ..] = parameters.float_slide;
        let [down_mul, down_add, ..] = parameters.float_down;
        let rotation = t * (DEGREES * fmadds(weight, slant_mul, slant_add));
        let displacement = t * fmadds(weight, slide_mul, slide_add);
        let sink = fmadds(weight, down_mul, down_add);
        let slant_limit = parameters.float_slant[LIMIT];
        let slide_limit = parameters.float_slide[LIMIT];
        if dx < 0.0 {
            self.slant_target += rotation;
            if self.slant_target > slant_limit {
                self.slant_target = slant_limit;
            }
            self.slide_target += displacement;
            if self.slide_target > slide_limit {
                self.slide_target = slide_limit;
            }
        } else {
            self.slant_target -= rotation;
            if self.slant_target < -slant_limit {
                self.slant_target = -slant_limit;
            }
            self.slide_target -= displacement;
            if self.slide_target < -slide_limit {
                self.slide_target = -slide_limit;
            }
        }
        self.down_target -= sink;
        let down_limit = parameters.float_down[LIMIT];
        if self.down_target < -down_limit {
            self.down_target = -down_limit;
        }
    }

    /// grGreatBay_801F5E28 (0x801F5E28): each of tilt and slide eases toward
    /// its target at `rate` moving away from rest and `reb_rate` back
    /// (0x801F5E8C, 0x801F5ECC: fmadds); the sink at down_rate going down,
    /// up_rate coming up (0x801F5F08: fmadds). Returns bone 49's rotation z
    /// and translation x, y (0x801F5FAC, 0x801F6028: fadds); the targets
    /// clear for the next frame's landings.
    pub fn step(&mut self, parameters: &Parameters) -> (f32, f32, f32) {
        let ease = |current: f32, target: f32, rates: [f32; 5]| {
            let delta = target - current;
            let rate = if -current * delta < 0.0 {
                rates[RATE]
            } else {
                rates[REBOUND_RATE]
            };
            fmadds(delta, rate, current)
        };
        self.slant = ease(self.slant, self.slant_target, parameters.float_slant);
        self.slide = ease(self.slide, self.slide_target, parameters.float_slide);
        let delta = self.down_target - self.down;
        // floatfloor_down_up_rate (+0x3C) is index 3, _down_rate (+0x40) 4.
        let rate = if delta < 0.0 {
            parameters.float_down[4]
        } else {
            parameters.float_down[3]
        };
        self.down = fmadds(delta, rate, self.down);
        let x = self.rest_x + self.slide;
        let y = self.rest_y + self.down;
        self.slant_target = 0.0;
        self.slide_target = 0.0;
        self.down_target = 0.0;
        (self.slant, x, y)
    }
}
