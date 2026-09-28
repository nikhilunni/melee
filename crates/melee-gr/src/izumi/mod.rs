//! Fountain of Dreams, gr/grizumi.c (NTSC 1.02): the two side platforms
//! that rise, sink and submerge on random schedules.
//!
//! Each platform is a map-4 Ground whose controller (grIzumi_801CC358)
//! moves a height; the height is written to the translation of a map-3 JObj
//! that carries the platform's collision joint (grIz_803E0D60), then
//! mpLib_80055E9C re-transforms that joint's lines. The map-4 model itself
//! only shows the pillar under the platform.
pub mod procs;
use gekko_math::{fma::fmadds, msl::fctiwz, HsdRng};

/// `grIzumi_YakumonoParam` (GrIz.dat `yakumono_param`).
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    /// +0x0 / +0x8: left and right platform heights at creation.
    pub initial_heights: [f32; 2],
    /// +0xC: height a platform returns to after surfacing (gp+DC).
    pub rest_height: f32,
    /// +0x18 / +0x1C: bounds of one random height step.
    pub step: [f32; 2],
    /// +0x20 / +0x24: clamp of a stepped target. Arriving below the lowest
    /// target submerges the platform.
    pub highest_target: f32,
    pub lowest_target: f32,
    /// +0x28 / +0x2C: height change per frame while moving.
    pub rise_speed: f32,
    pub sink_speed: f32,
    /// +0x30: below the rest height, chance that a step goes further down.
    pub sink_chance_below_rest: f32,
    /// +0x34: above the rest height, chance that a step goes further up.
    pub rise_chance_above_rest: f32,
    /// +0x3C / +0x38: rand_range bounds of the wait between decisions
    /// (retail passes 0x3C first).
    pub wait_frames: [f32; 2],
    /// +0x40 / +0x44 / +0x48: weights of submerging, waiting again, and
    /// stepping, in the order retail subtracts them.
    pub submerge_weight: f32,
    pub stay_weight: f32,
    pub step_weight: f32,
    /// +0x50 / +0x4C: rand_range bounds of the time spent submerged.
    pub submerged_frames: [f32; 2],
}

/// Platform Ground state gp+C4 (s16).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformPhase {
    /// 0: arrived; draw the wait.
    Arrived = 0,
    /// 1: count the wait down, then decide.
    Waiting = 1,
    /// 2: move towards the target.
    Moving = 2,
    /// 3: sank below the lowest target; hide and submerge.
    Sunk = 3,
    /// 4: submerged; count down, then resurface towards the rest height.
    Submerged = 4,
}
impl PlatformPhase {
    pub fn from_saved(value: i16) -> Self {
        match value {
            0 => Self::Arrived,
            1 => Self::Waiting,
            2 => Self::Moving,
            3 => Self::Sunk,
            4 => Self::Submerged,
            _ => panic!("invalid Fountain of Dreams platform phase {value}"),
        }
    }
}

/// Target of a submerge decision (grizumi.c:554, "@531").
const SUBMERGE_TARGET: f32 = -1.0;
/// Floor of the pillar's height ratio (grizumi.c:667, "@534").
const MINIMUM_RATIO: f32 = 0.01;
/// The submerged collision joint sits this far below the platform model's
/// origin (grizumi.c:651, "@533", added in double precision).
const SUBMERGED_OFFSET: f64 = -1.0;

/// One side platform: `GroundVars_izumi3` (gr/types.h:209).
#[derive(Clone, Debug, PartialEq)]
pub struct Platform {
    pub phase: PlatformPhase,
    /// gp+C6 (s16).
    pub timer: i16,
    /// gp+C8: the collision joint this platform moves.
    pub collision_joint: i16,
    /// gp+D0.
    pub height: f32,
    /// gp+D4.
    pub target: f32,
    /// gp+D8: height at which the pillar has unit scale.
    pub full_height: f32,
    /// gp+DC.
    pub rest_height: f32,
    /// Translation Y of the platform model's root (its creation position).
    pub origin_y: f32,
}

/// Engine operations one controller step requests, applied in this order
/// after the step: model visibility, pillar scale, collision JObj height,
/// then mpLib_80055E9C on `collision_joint` (always).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformStep {
    pub visibility: Visibility,
    /// Scale (x, y) of the model root's first child.
    pub pillar_scale: Option<(f32, f32)>,
    /// New translation Y of the collision JObj (map 3).
    pub collision_y: Option<f32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Unchanged,
    /// HSD_JObjSetFlagsAll(hidden), then HSD_JObjRemoveAnimAll.
    Hide,
    /// HSD_JObjClearFlagsAll(hidden), then grAnime_801C7FF8(gobj, 0, 7, 0,
    /// 0.0, 1.0): animation 0 restarts on the whole model.
    ShowAndAnimate,
}

/// `rand_range` (gr/inlines.h:193) over retail's fctiwz'd float bounds.
fn rand_range(rng: &mut HsdRng, [a, b]: [f32; 2]) -> i16 {
    let (a, b) = (fctiwz(a), fctiwz(b));
    let draw = |rng: &mut HsdRng, n: i32| if n != 0 { rng.randi(n) } else { 0 };
    let value = if a > b {
        b + draw(rng, a - b)
    } else if a < b {
        a + draw(rng, b - a)
    } else {
        a
    };
    // extsh: the Ground field is an s16.
    value as i16
}

impl Platform {
    /// grIzumi_801CCBDC (0x801CCBDC) before its first controller step. A
    /// negative height starts submerged-to-be; otherwise the platform starts
    /// arrived and its model animation is attached.
    pub fn new(
        height: f32,
        collision_joint: i16,
        rest_height: f32,
        origin_y: f32,
        full_height: f32,
    ) -> Self {
        Self {
            phase: if height < 0.0 {
                PlatformPhase::Sunk
            } else {
                PlatformPhase::Arrived
            },
            timer: 0,
            collision_joint,
            height,
            target: height,
            full_height,
            rest_height,
            origin_y,
        }
    }

    /// grIzumi_801CC358 (0x801CC358).
    pub fn tick(&mut self, p: &Parameters, rng: &mut HsdRng) -> PlatformStep {
        let mut step = PlatformStep {
            visibility: Visibility::Unchanged,
            pillar_scale: None,
            collision_y: None,
        };
        let moved = match self.phase {
            PlatformPhase::Arrived => {
                self.phase = PlatformPhase::Waiting;
                self.timer = rand_range(rng, p.wait_frames);
                true
            }
            PlatformPhase::Waiting => {
                if self.count_down() {
                    self.decide(p, rng);
                }
                false
            }
            PlatformPhase::Moving => {
                self.approach(p);
                true
            }
            PlatformPhase::Sunk => {
                self.phase = PlatformPhase::Submerged;
                self.timer = rand_range(rng, p.submerged_frames);
                step.visibility = Visibility::Hide;
                // 801CC774: lfs, fadd (double), frsp.
                step.collision_y = Some((f64::from(self.origin_y) + SUBMERGED_OFFSET) as f32);
                false
            }
            PlatformPhase::Submerged => {
                if self.count_down() {
                    self.phase = PlatformPhase::Moving;
                    self.target = self.rest_height;
                    step.visibility = Visibility::ShowAndAnimate;
                    true
                } else {
                    false
                }
            }
        };
        if moved {
            // 801CC87C: fdiv in double precision, then frsp.
            let mut ratio = (f64::from(self.height) / f64::from(self.full_height)) as f32;
            if ratio < MINIMUM_RATIO {
                ratio = MINIMUM_RATIO;
            }
            // retail 0x801CC8AC: fmadds 0.5 * ratio + 0.5
            step.pillar_scale = Some((fmadds(0.5, ratio, 0.5), ratio));
            step.collision_y = Some(self.height);
        }
        step
    }

    /// Signed s16 post-decrement: true once the timer was already negative.
    fn count_down(&mut self) -> bool {
        let old = self.timer;
        self.timer = old.wrapping_sub(1);
        old < 0
    }

    /// grizumi.c:541-587: submerge, step, or wait again.
    fn decide(&mut self, p: &Parameters, rng: &mut HsdRng) {
        // 801CC468-801CC474: (x40 + x44) + x48, times Randf, minus x40;
        // separate fadds/fmuls/fsubs, compared against 0.0 in double.
        let total = (p.submerge_weight + p.stay_weight) + p.step_weight;
        let mut roll = total * rng.randf() - p.submerge_weight;
        if roll < 0.0 {
            self.phase = PlatformPhase::Moving;
            self.target = SUBMERGE_TARGET;
            return;
        }
        roll -= p.step_weight;
        if roll >= 0.0 {
            self.timer = rand_range(rng, p.wait_frames);
            return;
        }
        let r = rng.randf();
        // retail 0x801CC4C4: fmadds (x1C - x18) * r + x18
        let size = fmadds(p.step[1] - p.step[0], r, p.step[0]);
        self.phase = PlatformPhase::Moving;
        let up = if self.height < self.rest_height {
            rng.randf() >= p.sink_chance_below_rest
        } else if self.height > self.rest_height {
            rng.randf() < p.rise_chance_above_rest
        } else {
            f64::from(rng.randf()) < 0.5
        };
        if up {
            self.target += size;
        } else {
            self.target -= size;
        }
        if self.target > p.highest_target {
            self.target = p.highest_target;
        } else if self.target < p.lowest_target {
            self.target = p.lowest_target;
        }
    }

    /// grizumi.c:589-618: move one speed step, landing exactly on the target.
    fn approach(&mut self, p: &Parameters) {
        let distance = self.target - self.height;
        if distance > 0.0 {
            if distance < p.rise_speed {
                self.height = self.target;
                self.phase = PlatformPhase::Arrived;
            } else {
                self.height += p.rise_speed;
            }
        } else if distance < 0.0 {
            if -distance < p.sink_speed {
                self.height = self.target;
                self.phase = if self.height < p.lowest_target {
                    PlatformPhase::Sunk
                } else {
                    PlatformPhase::Arrived
                };
            } else {
                self.height -= p.sink_speed;
            }
        } else {
            self.phase = PlatformPhase::Arrived;
        }
    }
}

/// Fountain of Dreams stage state: both platforms, left (joint 0) first.
/// Map 3's lights are animated LObjs (grIzumi_OnLoad loops them); they only
/// light the scene and are not loaded here.
#[derive(Clone, Debug)]
pub struct Izumi {
    pub parameters: Parameters,
    pub platforms: [Platform; 2],
}

#[cfg(test)]
mod tests;
