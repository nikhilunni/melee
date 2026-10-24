//! Yoshi's Island N64, gr/groldyoshi.c (NTSC 1.02): three clouds that sink
//! under a fighter's weight, fade away after carrying one for a while and
//! come back; and a background guest drawn at random intervals.
//!
//! The clouds are map 2 (grOldYoshi_8020EC10). Each cloud's JObj carries a
//! collision joint (map 2's archive bindings); its sink depth is subtracted
//! from the JObj's rest height, and Ground_801C2FE0 re-transforms the
//! joints after all three move. A fighter landing or standing on a cloud
//! sets its flag through the joint's `cb_0` (fn_8020F2A8). The guest is map
//! 3 (grOldYoshi_8020F088): it only draws, but its waits draw the RNG.
pub mod procs;
use gekko_math::{fma::fmadds, HsdRng};

/// `grOldYoshi_YakumonoParam` (GrOy.dat `yakumono_param`), as retail reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    /// +0x0 (s16): a carrying cloud fades once its counter passes this.
    pub vanish_after: i16,
    /// +0x2 (s16): a faded cloud returns once its counter passes this.
    pub return_after: i16,
    /// +0x4: the sink speed's bound, both ways.
    pub maximum_speed: f32,
    /// +0x8: the sink speed's change per frame.
    pub acceleration: f32,
    /// +0xC: how deep a carrying cloud sinks.
    pub sink_depth: f32,
    /// +0x10 (s16): the fading counter value that removes the collision joint.
    pub disable_frame: i16,
    /// +0x12 (s16): the returning counter value that restores it.
    pub enable_frame: i16,
    /// +0x16, +0x14 (s16): `rand_range`'s bounds of the guest's wait, in
    /// retail's argument order.
    pub guest_wait: [i16; 2],
    /// +0x18: the guest's height range. Retail reads it with `lha`
    /// (0x8020F138), as an s16, whatever the decomp's float field says.
    pub guest_height: i16,
}

/// `grOy_803E6574`: each cloud's collision joint and its JObj (the
/// Ground_801C3FA4 descendant index of map 2).
pub const CLOUDS: [Cloud; 3] = [Cloud::at(0, 1), Cloud::at(1, 5), Cloud::at(2, 9)];

/// A cloud's place in map 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cloud {
    pub joint: i32,
    pub bone: usize,
}
impl Cloud {
    const fn at(joint: i32, bone: usize) -> Self {
        Self { joint, bone }
    }
}

/// `grOldYoshi_Cloud.xC4_0123`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudPhase {
    /// 0: sinking under a fighter or floating back up.
    Floating = 0,
    /// 1: fading out (model animation 1's materials).
    Fading = 1,
    /// 2: gone.
    Gone = 2,
    /// 3: fading back in (model animation 2's materials).
    Returning = 3,
}
impl CloudPhase {
    pub fn from_saved(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Floating,
            1 => Self::Fading,
            2 => Self::Gone,
            3 => Self::Returning,
            _ => return None,
        })
    }
}

/// One cloud's state: `grOldYoshi_Cloud` (gr/types.h:1780), 0x14 bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudState {
    pub phase: CloudPhase,
    /// xC4_567: an 8-bit counter.
    pub counter: u8,
    /// xCC: the JObj's translation Y at creation.
    pub rest_y: f32,
    /// xD0: how far the cloud has sunk.
    pub depth: f32,
    /// xD4: the sink speed.
    pub speed: f32,
}

/// The guest: `grOldYoshi_Guest_GroundVars`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guest {
    /// xC4 (s16): frames until the next guest.
    pub timer: i16,
    /// xC6 (s16): the descendant shown, or -1 while waiting.
    pub shown: i16,
}
/// grOy_803B83F0: the guest models, Ground_801C3FA4 descendants of map 3.
const GUESTS: [i16; 5] = [1, 2, 3, 4, 5];
const WAITING: i16 = -1;

/// The engine operations a cloud step performs, in the order it asks.
pub trait CloudEngine {
    /// `grAnime_801C83D0(gobj, bone, 2)`: the subtree's materials ended.
    fn materials_ended(&mut self, bone: usize) -> bool;
    /// `grAnime_801C7FF8(gobj, bone, 2, animation, 0.0, 1.0)`.
    fn play_materials(&mut self, bone: usize, animation: usize);
    /// `grLib_801C97DC(30000, 30, jobj)`: the stage-bank puff at the JObj.
    fn spawn_puff(&mut self, bone: usize);
    /// `mpLib_80057BC0`.
    fn remove_joint(&mut self, joint: i32);
    /// `mpJointListAdd`.
    fn add_joint(&mut self, joint: i32);
    /// `HSD_JObjSetFlagsAll` / `HSD_JObjClearFlagsAll(jobj, JOBJ_HIDDEN)`.
    fn set_hidden(&mut self, bone: usize, hidden: bool);
    /// `HSD_JObjSetTranslateY(jobj, y)`.
    fn set_translate_y(&mut self, bone: usize, y: f32);
}

/// The fading animation (model 2's animation 1) and the returning one (2).
const FADE_ANIMATION: usize = 1;
const RETURN_ANIMATION: usize = 2;
/// grOldYoshi_8020F31C's braking distance factor (@452).
const HALF: f32 = 0.5;

#[derive(Clone, Debug)]
pub struct OldYoshi {
    pub parameters: Parameters,
    pub clouds: [CloudState; 3],
    pub guest: Guest,
}

impl OldYoshi {
    /// grOldYoshi_8020EAFC (0x8020EAFC) for the clouds and
    /// grOldYoshi_8020EFCC (0x8020EFCC) for the guest, in creation order
    /// (map 2, then 3). `rest_y` is each cloud JObj's translation Y.
    pub fn create(parameters: Parameters, rest_y: [f32; 3], rng: &mut HsdRng) -> Self {
        let clouds = rest_y.map(|rest_y| CloudState {
            phase: CloudPhase::Floating,
            counter: 0,
            rest_y,
            depth: 0.0,
            speed: 0.0,
        });
        let timer = guest_wait(&parameters, rng);
        Self {
            parameters,
            clouds,
            guest: Guest {
                timer,
                shown: WAITING,
            },
        }
    }

    /// grOldYoshi_8020EC10 (0x8020EC10): each cloud in turn. `carried[i]`
    /// is cloud `i`'s xC4_4, which fn_8020F2A8 set since the last step;
    /// each cloud clears it after its step.
    pub fn tick_clouds(&mut self, carried: [bool; 3], engine: &mut impl CloudEngine) {
        let p = &self.parameters;
        for (index, cloud) in self.clouds.iter_mut().enumerate() {
            let at = CLOUDS[index];
            match cloud.phase {
                CloudPhase::Floating => {
                    if carried[index] {
                        cloud.speed = sink_speed(
                            cloud.speed,
                            p.acceleration,
                            cloud.depth,
                            p.sink_depth,
                            p.maximum_speed,
                            -p.maximum_speed,
                        );
                        cloud.depth += cloud.speed;
                        if cloud.depth > p.sink_depth {
                            cloud.depth = p.sink_depth;
                        }
                        let old = cloud.counter;
                        cloud.counter = old.wrapping_add(1);
                        if i32::from(old) > i32::from(p.vanish_after) {
                            cloud.phase = CloudPhase::Fading;
                            engine.play_materials(at.bone, FADE_ANIMATION);
                            engine.spawn_puff(at.bone);
                            cloud.counter = 0;
                        }
                    } else {
                        cloud.speed = sink_speed(
                            cloud.speed,
                            p.acceleration,
                            cloud.depth,
                            0.0,
                            p.maximum_speed,
                            -p.maximum_speed,
                        );
                        cloud.depth += cloud.speed;
                        if cloud.depth < 0.0 {
                            cloud.depth = 0.0;
                        }
                        cloud.counter = 0;
                    }
                }
                CloudPhase::Fading => {
                    if i32::from(cloud.counter) == i32::from(p.disable_frame) {
                        engine.remove_joint(at.joint);
                    }
                    cloud.counter = cloud.counter.wrapping_add(1);
                    if engine.materials_ended(at.bone) {
                        cloud.phase = CloudPhase::Gone;
                        engine.remove_joint(at.joint);
                        engine.set_hidden(at.bone, true);
                        cloud.counter = 0;
                    }
                }
                CloudPhase::Gone => {
                    let old = cloud.counter;
                    cloud.counter = old.wrapping_add(1);
                    if i32::from(old) > i32::from(p.return_after) {
                        cloud.phase = CloudPhase::Returning;
                        engine.play_materials(at.bone, RETURN_ANIMATION);
                        cloud.counter = 0;
                    }
                }
                CloudPhase::Returning => {
                    if cloud.counter == 0 {
                        engine.set_hidden(at.bone, false);
                    }
                    if i32::from(cloud.counter) == i32::from(p.enable_frame) {
                        engine.add_joint(at.joint);
                    }
                    cloud.counter = cloud.counter.wrapping_add(1);
                    if engine.materials_ended(at.bone) {
                        cloud.phase = CloudPhase::Floating;
                        cloud.counter = 0;
                    }
                }
            }
            // 0x8020EF10: fsubs.
            engine.set_translate_y(at.bone, cloud.rest_y - cloud.depth);
        }
    }

    /// grOldYoshi_8020F088 (0x8020F088). While waiting, every call restarts
    /// the guest animation and draws its height; `Some(height)` asks the
    /// caller to run grAnime_801C8138 and then set the model's translation
    /// Y. Once a guest shows, `ended` (grAnime_801C83D0(gobj, 0, 7)) hides it
    /// again and draws the next wait.
    pub fn tick_guest(&mut self, ended: bool, rng: &mut HsdRng) -> GuestStep {
        if self.guest.shown == WAITING {
            let old = self.guest.timer;
            self.guest.timer = old.wrapping_sub(1);
            if old < 0 {
                self.guest.shown = GUESTS[rng.randi(GUESTS.len() as i32) as usize];
            }
            // retail 0x8020F154: fmsubs 2.0 * r - 1.0, then 0x8020F164
            // fmuls by the s16 range converted to float.
            let r = rng.randf();
            let height =
                f32::from(self.parameters.guest_height) * gekko_math::fma::fmsubs(2.0, r, 1.0);
            return GuestStep::Restart { height };
        }
        let bone = self.guest.shown as usize;
        if ended {
            self.guest.timer = guest_wait(&self.parameters, rng);
            self.guest.shown = WAITING;
        }
        GuestStep::Shown { bone, ended }
    }
}

/// What a guest step asks of map 3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuestStep {
    /// grAnime_801C8138(gobj, map, 0), then HSD_JObjSetTranslateY(jobj, height).
    Restart { height: f32 },
    /// The guest descendant shows (its hidden flag cleared if set); once its
    /// animation `ended`, it hides again.
    Shown { bone: usize, ended: bool },
}

/// `rand_range(x16, x14)` (gr/inlines.h): an s16 Ground field.
fn guest_wait(p: &Parameters, rng: &mut HsdRng) -> i16 {
    let [a, b] = p.guest_wait.map(i32::from);
    let draw = |rng: &mut HsdRng, n: i32| if n != 0 { rng.randi(n) } else { 0 };
    let value = if a > b {
        b + draw(rng, a - b)
    } else if a < b {
        a + draw(rng, b - a)
    } else {
        a
    };
    value as i16
}

/// grOldYoshi_8020F31C (0x8020F31C): accelerate towards `target`, braking
/// when the remaining distance is within the stopping distance, clamped to
/// `[minimum, maximum]` (`fcmpo`: a NaN takes the minimum).
fn sink_speed(
    speed: f32,
    acceleration: f32,
    position: f32,
    target: f32,
    maximum: f32,
    minimum: f32,
) -> f32 {
    let magnitude = if speed < 0.0 { -speed } else { speed };
    // 0x8020F334: fdivs.
    let frames = magnitude / acceleration;
    // 0x8020F360-0x8020F370: fmuls, fmuls, fmuls, then fmadds
    // frames * (0.5 * -acceleration * frames) + frames * |speed|.
    let travelled = frames * magnitude;
    let braking = HALF * -acceleration * frames;
    let stopping = fmadds(frames, braking, travelled);
    let mut distance = target - position;
    if distance < 0.0 {
        distance = -distance;
    }
    let mut speed = speed;
    let closing = (target > position && speed > 0.0) || (target < position && speed < 0.0);
    if distance < stopping && closing {
        if speed > 0.0 {
            speed -= acceleration;
        } else if speed < 0.0 {
            speed += acceleration;
        }
    } else if target > position {
        speed += acceleration;
    } else if target < position {
        speed -= acceleration;
    }
    if speed > maximum {
        return maximum;
    }
    if speed >= minimum {
        return speed;
    }
    minimum
}

#[cfg(test)]
mod tests;
