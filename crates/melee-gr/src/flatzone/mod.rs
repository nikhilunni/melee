//! Flat Zone, gr/grflatzone.c (NTSC 1.02): the Game & Watch screen whose
//! platforms switch between drawn layouts, and its two hazards, the oil
//! man who spills a slippery puddle and the falling tools.
//!
//! The maps (grFlatzone_80216E78 creates 0, 1, 2, 3, 4, 5, 6 and 8):
//! - map 2, the controller (grFlatzone_802171D4): waits `event_wait`, then
//!   starts one hazard, alternating between them; when that hazard reports
//!   back (its `ready` flag) it picks a new platform layout and waits again.
//! - map 5, the platforms (grFlatzone_802174EC): a layout is a row of
//!   `LAYOUTS`, its column 0 the bare transition and columns 1..4 variants
//!   it cycles through at random every `variant_wait` frames. Each
//!   animation change re-stitches the collision joints (mpLib_80058560).
//! - map 3, the oil man (grFlatzone_802176BC).
//! - map 8, the tools (grFlatzone_80217EF0), an empty JObj.
//! - maps 4 and 6 (grFlatzone_80217D30) are the hazards of the controller's
//!   cases 1 and 2, which its draw `{3, 4}[HSD_Randi(2)]` never picks.
pub mod procs;

use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;

/// `grFlatzone_YakumonoParam` (grflatzone.c:20-37, GrFz.dat
/// `yakumono_param`): every field a word.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    /// +0x4 / +0x0: `rand_int(unk4, unk0)`, the controller's event wait.
    pub event_wait: [i32; 2],
    /// +0xC / +0x8: `rand_int(unkC, unk8)`, the platforms' variant wait.
    pub variant_wait: [i32; 2],
    /// +0x10: how long the tools fall.
    pub tools_frames: i32,
    /// +0x14: one tool every this many frames.
    pub tools_interval: i32,
    /// +0x18 / +0x1C: the right and left bounds of a tool's x.
    pub tools_right: i32,
    pub tools_left: i32,
    /// +0x20: the oil man's walk-in frames.
    pub oil_walk_in: i32,
    /// +0x24 / +0x28: the oil man's start x facing right and left.
    pub oil_right_x: f32,
    pub oil_left_x: f32,
    /// +0x2C: frames per step.
    pub oil_step_frames: i32,
    /// +0x30 / +0x34: bounds of the step count.
    pub oil_steps: [i32; 2],
    /// +0x38: the step's length.
    pub oil_step: f32,
    /// +0x3C: how long the spill stays.
    pub oil_frames: i32,
}

/// The controller's xC5 as Ground_GetStageGObj leaves it: 0 in the
/// w25_start_fz_fox_marth4_v01 boundary. Only a stale 3 or 4 would change
/// the first event's draws (grFlatzone_802171D4 redraws while equal).
pub const BOOT_STALE_EVENT: u8 = 0;
/// grFlatzone_80216E78: the bottom blast zone is lowered by this much.
pub const BLAST_BOTTOM_DROP: f32 = 50.0;
/// The maps.
pub const CONTROLLER: u8 = 2;
pub const OIL: u8 = 3;
pub const PLATFORMS: u8 = 5;
pub const TOOLS: u8 = 8;

/// `grFz_803E7A68` as `s16[][5]`: the platform animation of layout `row`,
/// column 0 the transition, 1..4 the variants (-1: none). Row 8 is all
/// zeros (the table's padding), reached only as `xC8`'s "no layout".
pub const LAYOUTS: [[i16; 5]; 9] = [
    [0, 1, 2, -1, -1],
    [3, 4, 5, 6, 7],
    [8, 9, 10, 11, 12],
    [13, 14, 15, 16, 17],
    [18, -1, -1, -1, -1],
    [19, -1, -1, -1, -1],
    [20, -1, -1, -1, -1],
    [21, -1, -1, -1, -1],
    [0, 0, 0, 0, 0],
];
/// `xC7 == 8`: the platforms are hidden (grFlatzone_80218260).
const HIDDEN_LAYOUT: u8 = 8;
/// `@331`: the controller's hazards, the tools (3) and the oil man (4).
const EVENTS: [u8; 2] = [3, 4];
const TOOLS_EVENT: u8 = 3;
const OIL_EVENT: u8 = 4;

/// `rand_int(a, b)` (grflatzone.c:68): `b` plus a draw below `a - b` when
/// `a > b`, `a` plus one below `b - a` when `a < b`, else `a`.
pub fn rand_int(a: i32, b: i32, rng: &mut HsdRng) -> i32 {
    let draw = |rng: &mut HsdRng, n: i32| if n != 0 { rng.randi(n) } else { 0 };
    if a > b {
        b + draw(rng, a - b)
    } else if a < b {
        a + draw(rng, b - a)
    } else {
        a
    }
}

/// Map 2's `GroundVars_flatzone` (gr/types.h:221).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Controller {
    /// xC4: waiting for the match to start (fn_8021706C clears it).
    pub waiting: bool,
    /// xC5: the current hazard. Ground_GetStageGObj leaves it as stale
    /// heap; the first event compares against that byte.
    pub event: u8,
    /// xC6: the previous hazard.
    pub last_event: u8,
    /// xC7: counting down (the running hazard sets it when it is done).
    pub ready: bool,
    /// xC8 (s16): frames to the next hazard; 0 once one started.
    pub timer: i16,
}

/// Map 5's `GroundVars_flatzone3` with `xCC`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Platforms {
    /// xC4: waiting for the match to start (fn_80217440 clears it).
    pub waiting: bool,
    /// xC5: the animation playing.
    pub animation: u8,
    /// xC6: the animation the collision was last stitched for.
    pub stitched: u8,
    /// xC7: the layout row (8: hidden).
    pub layout: u8,
    /// xC8: a row the draws avoid; always 8.
    pub avoided: u8,
    /// xC9: the variant column (0: the transition, no cycling).
    pub variant: u8,
    /// xCA: the previous variant.
    pub previous_variant: u8,
    /// xCC (s16): frames to the next variant.
    pub timer: i16,
}

/// A grDynamicAttr_8049EFA0 slot, the oil spill's.
pub type SpillHandle = u8;

/// Map 3's `GroundVars_flatzone2`.
#[derive(Clone, Debug, PartialEq)]
pub struct OilMan {
    /// xC4 (u8): the controller started him.
    pub triggered: bool,
    /// xC8: his facing, 1.0 (right) or -1.0.
    pub facing: f32,
    /// xCC: his spill's grDynamicAttr slot.
    pub spill: Option<SpillHandle>,
    /// xD0: the phase.
    pub phase: OilPhase,
    /// xD4: the phase's countdown.
    pub timer: i32,
}

/// `xD0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OilPhase {
    /// -1: off screen.
    Idle,
    /// 0: walking in.
    WalkIn,
    /// 1: stepping across.
    Step,
    /// 2: pouring, until the animation ends.
    Pour,
    /// 3: the spill stays.
    Spilled,
    /// 4: gone.
    Leave,
}
impl OilPhase {
    pub fn from_saved(value: i32) -> Option<Self> {
        Some(match value {
            -1 => Self::Idle,
            0 => Self::WalkIn,
            1 => Self::Step,
            2 => Self::Pour,
            3 => Self::Spilled,
            4 => Self::Leave,
            _ => return None,
        })
    }
}

/// Map 8's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tools {
    /// xC4 (u8): the controller started them.
    pub triggered: bool,
    /// xC8 (s32): frames left.
    pub frames: i32,
}

/// The engine operations of the Flat Zone procs, in the order asked.
pub trait FlatZoneEngine {
    /// `grAnime_801C8138(map's gobj, map, animation)`.
    fn play(&mut self, map: u8, animation: usize);
    /// `HSD_JObjRemoveAnimAll(gobj jobj)`.
    fn remove_animation(&mut self, map: u8);
    /// `HSD_JObjSetFlagsAll(gobj jobj, JOBJ_HIDDEN)`.
    fn hide(&mut self, map: u8);
    /// `HSD_JObjGetTranslation(gobj jobj)`.
    fn translation(&mut self, map: u8) -> Vec3;
    /// `HSD_JObjSetTranslate(gobj jobj)`.
    fn set_translation(&mut self, map: u8, translation: Vec3);
    /// `grAnime_801C83D0(gobj, 0, 1)`: the joint animation ended.
    fn animation_ended(&mut self, map: u8) -> bool;
    /// `mpCheckMultiple(x, y0, x, y1, .., 1, -1, 0x32)`: the floor of joint
    /// 50 below, its point and line.
    fn find_floor(&mut self, x: f32, from_y: f32, to_y: f32) -> Option<(Vec3, i32)>;
    /// `grDynamicAttr_801CA0F8(0x11, position, line, radius, frames)`.
    fn spill(&mut self, position: Vec3, line: i32, radius: f32, frames: i32) -> Option<SpillHandle>;
    /// `grDynamicAttr_801CA1C0(handle)`.
    fn release_spill(&mut self, handle: Option<SpillHandle>);
    /// `mpLib_80058560`: stitch the enabled joints.
    fn stitch(&mut self);
    /// `Stage_GetBlastZoneTopOffset()`.
    fn blast_top(&self) -> f32;
    /// `it_802EEFA8(position, kind, facing)`, then the new tool's
    /// `joint_id_only = 0x32`. Nothing after it in the proc draws, so the
    /// engine may create the item once the proc returns.
    fn spawn_tool(&mut self, position: Vec3, kind: i32, facing: f32);
}

/// grFlatzone_802176BC's `36.0f`, `27.0f` and `5.0f`: where the spill is
/// sought from the oil man's position.
const SPILL_REACH: f32 = 36.0;
const SPILL_DROP: f32 = 27.0;
const SPILL_PROBE: f32 = 5.0;
/// `22.0f`: the spill's radius.
const SPILL_RADIUS: f32 = 22.0;
/// The spill's material (mpLib material 0x11, a tenth of the traction).
pub const SPILL_MATERIAL: i32 = 0x11;
/// grFlatzone_80217EF0: tools appear this far below the top blast zone.
const TOOLS_DROP: f32 = 5.0;
/// it_802EEFA8's kinds: HSD_Randi(5).
const TOOL_KINDS: i32 = 5;

#[derive(Clone, Debug)]
pub struct FlatZone {
    pub parameters: Parameters,
    pub controller: Controller,
    pub platforms: Platforms,
    pub oil: OilMan,
    pub tools: Tools,
    /// Maps 4 and 6's xC4: set by the controller's unreachable cases.
    pub unused_triggers: [bool; 2],
}

impl FlatZone {
    /// The on_init callbacks of grFlatzone_80216E78's creation order
    /// (maps 2, 3, 5, 8; 0, 1, 4 and 6 draw nothing), with map 2's draw.
    /// `stale_event` is the controller's uncleared xC5.
    pub fn create(parameters: Parameters, stale_event: u8, rng: &mut HsdRng) -> Self {
        // grFlatzone_8021707C (0x8021707C).
        let [a, b] = parameters.event_wait;
        let timer = rand_int(a, b, rng) as i16;
        Self {
            controller: Controller {
                waiting: true,
                event: stale_event,
                last_event: 0,
                ready: true,
                timer,
            },
            // grFlatzone_80217450 (0x80217450).
            platforms: Platforms {
                waiting: true,
                animation: 0,
                stitched: 0xFF,
                layout: HIDDEN_LAYOUT,
                avoided: HIDDEN_LAYOUT,
                variant: 0,
                previous_variant: 0,
                timer: 0,
            },
            // grFlatzone_80217648 (0x80217648).
            oil: OilMan {
                triggered: false,
                facing: 1.0,
                spill: None,
                phase: OilPhase::Idle,
                timer: 0,
            },
            // grFlatzone_80217E90 (0x80217E90).
            tools: Tools {
                triggered: false,
                frames: 0,
            },
            unused_triggers: [false; 2],
            parameters,
        }
    }

    /// fn_8021706C and fn_80217440, which Ground_801C10B8 deferred to the
    /// match start (run newest first: map 5's, then map 2's).
    pub fn start(&mut self) {
        self.platforms.waiting = false;
        self.controller.waiting = false;
    }

    /// grFlatzone_802171D4 (0x802171D4) up to Ground_801C2FE0.
    pub fn tick_controller(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        let c = &mut self.controller;
        if c.waiting || !c.ready {
            return;
        }
        if c.timer != 0 {
            c.timer -= 1;
            if c.timer != 0 {
                return;
            }
            c.ready = false;
            c.last_event = c.event;
            loop {
                c.event = EVENTS[rng.randi(2) as usize];
                if c.last_event != c.event {
                    break;
                }
            }
            match c.event {
                TOOLS_EVENT => self.tools.triggered = true,
                OIL_EVENT => self.oil.triggered = true,
                // Cases 1 and 2 (maps 4 and 6, grFlatzone_80218260): @331
                // holds only 3 and 4.
                event => unreachable!("grFlatzone_802171D4: event {event}"),
            }
            self.change_layout(engine, rng);
        } else {
            self.randomize_layout(engine, rng);
            let [a, b] = self.parameters.event_wait;
            self.controller.timer = rand_int(a, b, rng) as i16;
        }
    }

    /// grFlatzone_80218060 (0x80218060) with `arg0 == 0`: a new layout with
    /// variants, a variant and its wait.
    pub fn randomize_layout(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        let p = &mut self.platforms;
        loop {
            p.layout = rng.randi(8) as u8;
            if p.layout == p.avoided {
                continue;
            }
            if LAYOUTS[usize::from(p.layout)][1] != -1 {
                break;
            }
        }
        loop {
            p.variant = (rng.randi(4) + 1) as u8;
            if LAYOUTS[usize::from(p.layout)][usize::from(p.variant)] != -1 {
                break;
            }
        }
        self.finish_layout(engine, rng);
    }

    /// grFlatzone_80218060 (0x80218060) with `arg0 != 0`, at creation:
    /// layout 2, variant 1.
    pub fn first_layout(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        self.platforms.layout = 2;
        self.platforms.variant = 1;
        self.finish_layout(engine, rng);
    }

    /// grFlatzone_80218060's tail.
    fn finish_layout(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        let p = &mut self.platforms;
        p.previous_variant = 0;
        let [a, b] = self.parameters.variant_wait;
        p.timer = rand_int(a, b, rng) as i16;
        p.animation = LAYOUTS[usize::from(p.layout)][usize::from(p.variant)] as u8;
        engine.play(PLATFORMS, usize::from(p.animation));
    }

    /// grFlatzone_802181B4 (0x802181B4): a hazard starts; the platforms
    /// play a layout's transition and stop cycling.
    fn change_layout(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        let p = &mut self.platforms;
        loop {
            p.layout = rng.randi(8) as u8;
            if p.layout != p.avoided {
                break;
            }
        }
        p.variant = 0;
        p.previous_variant = 0;
        p.timer = 0;
        p.animation = LAYOUTS[usize::from(p.layout)][0] as u8;
        engine.play(PLATFORMS, usize::from(p.animation));
    }

    /// grFlatzone_802174EC (0x802174EC) up to Ground_801C2FE0: the next
    /// variant once the wait runs out.
    pub fn tick_platforms(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        let p = &mut self.platforms;
        if p.waiting || p.layout == HIDDEN_LAYOUT || p.variant == 0 {
            return;
        }
        let old = p.timer;
        p.timer = old.wrapping_sub(1);
        if old >= 0 {
            return;
        }
        p.previous_variant = p.variant;
        let animation = loop {
            p.variant = (rng.randi(4) + 1) as u8;
            if p.previous_variant == p.variant {
                continue;
            }
            let entry = LAYOUTS[usize::from(p.layout)][usize::from(p.variant)];
            if entry != -1 {
                break entry;
            }
        };
        p.animation = animation as u8;
        engine.play(PLATFORMS, usize::from(p.animation));
        let [a, b] = self.parameters.variant_wait;
        self.platforms.timer = rand_int(a, b, rng) as i16;
    }

    /// grFlatzone_802174EC's tail, after Ground_801C2FE0: a changed
    /// animation re-stitches the collision.
    pub fn stitch_platforms(&mut self, engine: &mut impl FlatZoneEngine) {
        let p = &mut self.platforms;
        if p.stitched != p.animation {
            p.stitched = p.animation;
            engine.stitch();
        }
    }

    /// grFlatzone_802176BC (0x802176BC) up to Ground_801C2FE0.
    pub fn tick_oil(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        let p = &self.parameters;
        let oil = &mut self.oil;
        let mut done = false;
        if oil.triggered {
            if oil.phase == OilPhase::Spilled {
                engine.hide(OIL);
                // grflatzone.c:395: the test is inverted, so this releases
                // nothing.
                if oil.spill.is_none() {
                    engine.release_spill(oil.spill);
                    oil.spill = None;
                }
            }
            oil.triggered = false;
            oil.facing = if rng.randi(2) != 0 { 1.0 } else { -1.0 };
            oil.phase = OilPhase::WalkIn;
            oil.timer = p.oil_walk_in;
            let mut position = Vec3::ZERO;
            if oil.facing == 1.0 {
                position.x = p.oil_right_x;
                // retail 0x802177A8..C0: int to float, fmuls 0.5, fctiwz.
                oil.timer = gekko_math::msl::fctiwz(oil.timer as f32 * 0.5);
            } else {
                position.x = p.oil_left_x;
                engine.play(CONTROLLER, 1);
            }
            engine.set_translation(OIL, position);
        }
        match oil.phase {
            OilPhase::WalkIn => {
                if oil.timer == p.oil_walk_in / 2 {
                    engine.play(OIL, if oil.facing == 1.0 { 0 } else { 2 });
                    engine.remove_animation(OIL);
                }
                oil.timer -= 1;
                if oil.timer <= 0 {
                    oil.phase = OilPhase::Step;
                    let range = p.oil_steps[1] - p.oil_steps[0];
                    let extra = if range != 0 { rng.randi(range) } else { 0 };
                    oil.timer = p.oil_step_frames.wrapping_mul(p.oil_steps[0] + extra);
                    engine.play(OIL, if oil.facing == 1.0 { 0 } else { 2 });
                }
            }
            OilPhase::Step => {
                if oil.timer % p.oil_step_frames == 1 {
                    let mut position = engine.translation(OIL);
                    // retail 0x80217A08: fmadds step * facing + x.
                    position.x = fmadds(p.oil_step, oil.facing, position.x);
                    engine.set_translation(OIL, position);
                }
                oil.timer -= 1;
                if oil.timer <= 0 {
                    oil.phase = OilPhase::Pour;
                    oil.timer = 0;
                    engine.play(OIL, if oil.facing == 1.0 { 1 } else { 3 });
                }
            }
            OilPhase::Pour => {
                if engine.animation_ended(OIL) {
                    oil.phase = OilPhase::Spilled;
                    oil.timer = p.oil_frames;
                    let mut position = engine.translation(OIL);
                    // retail 0x80217B78: fmadds 36 * facing + x; then
                    // fsubs 27, and the probe 5 above (fadds) and below.
                    position.x = fmadds(SPILL_REACH, oil.facing, position.x);
                    position.y -= SPILL_DROP;
                    let from_y = SPILL_PROBE + position.y;
                    position.y -= SPILL_PROBE;
                    if let Some((point, line)) = engine.find_floor(position.x, from_y, position.y) {
                        if line != -1 {
                            oil.spill = engine.spill(point, line, SPILL_RADIUS, p.oil_frames);
                        }
                    }
                    done = true;
                }
            }
            OilPhase::Spilled => {
                oil.timer -= 1;
                if oil.timer <= 1 {
                    oil.phase = OilPhase::Leave;
                    oil.timer = 0;
                }
            }
            OilPhase::Leave => {
                engine.remove_animation(OIL);
                engine.hide(OIL);
                oil.spill = None;
                oil.phase = OilPhase::Idle;
            }
            OilPhase::Idle => {}
        }
        if done {
            self.controller.ready = true;
            if self.oil.facing != 1.0 {
                engine.play(CONTROLLER, 0);
            }
        }
    }

    /// grFlatzone_80217EF0 (0x80217EF0): a tool every `tools_interval`
    /// frames while they fall.
    pub fn tick_tools(&mut self, engine: &mut impl FlatZoneEngine, rng: &mut HsdRng) {
        let p = &self.parameters;
        let tools = &mut self.tools;
        if tools.triggered {
            tools.frames = p.tools_frames;
            tools.triggered = false;
        }
        if tools.frames <= 0 {
            return;
        }
        if tools.frames % p.tools_interval == 0 {
            let r = rng.randf();
            // retail 0x80217F88..98: the bounds' integers as floats, then
            // fmuls r * (right - left) and fsubs (-left), unfused.
            let width = (p.tools_right - p.tools_left) as f32;
            let left = (-p.tools_left) as f32;
            let x = r * width - left;
            let position = Vec3::new(x, engine.blast_top() - TOOLS_DROP, 0.0);
            let kind = rng.randi(TOOL_KINDS);
            let facing = if rng.randi(2) != 0 { 1.0 } else { -1.0 };
            engine.spawn_tool(position, kind, facing);
        }
        tools.frames -= 1;
        if tools.frames <= 0 {
            self.controller.ready = true;
        }
    }
}

#[cfg(test)]
mod tests;
