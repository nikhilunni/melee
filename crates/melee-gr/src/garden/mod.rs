//! Jungle Japes, gr/grgarden.c (NTSC 1.02): Cranky Kong's hut, the
//! Klaptrap that leaps out of the river, and the river's current.
//!
//! Map 1 (Cranky) cycles four animations, waiting a random number of
//! loops in the two looping ones. Map 3 (the Klaptrap) waits a random
//! time, then plays its leap at a random x while a stage-material item
//! (ityaku.c, kind 0xA0) follows its jaw and carries the bite's hitbox.
//! Map 2 registers a fighter device (ftCo_800C06E8, type 4) at the match
//! start: below the water line a fighter drifts with the current, and
//! entering the water splashes.
pub mod procs;
use gekko_math::{msl::fctiwz, HsdRng};
use hsd_types::Vec3;

/// `grGarden_YakumonoParam` (grgarden.c:26, GrGd.dat `yakumono_param`).
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    /// +0x0: the water line a fighter below is carried by the current.
    pub water_level: f32,
    /// +0x4: the current's x speed.
    pub current: f32,
    /// +0x8: the range of Cranky's waiting loops in animation 0.
    pub cranky_rest_loops: i32,
    /// +0xC: the range of Cranky's waiting loops in animation 2.
    pub cranky_busy_loops: i32,
    /// +0x10 / +0x14: the Klaptrap's wait, minimum and random range.
    pub klaptrap_wait_minimum: i32,
    pub klaptrap_wait_range: i32,
    /// +0x18 / +0x1C: the bounds of the leap's x position.
    pub klaptrap_x: [f32; 2],
}

/// The Klaptrap item's state machine argument (Item_80268E5C's state).
pub mod klaptrap_item {
    /// grMaterial_801C8CFC(0, 0, ...): the item is created in state 0.
    pub const IDLE: u16 = 0;
    /// grGarden_80203250: state 2 while the leap plays (the bite).
    pub const BITE: u16 = 2;
}

/// The Klaptrap's sound cues (Ground_801C53EC), by leap frame.
const SPLASH_OUT_FRAMES: [i32; 2] = [38, 148];
const SPLASH_IN_FRAMES: [i32; 2] = [110, 210];
/// grGarden_80203624 (0x80203624): the camera eye never goes below this
/// (Camera_8002AF68).
pub const MIN_EYE_HEIGHT: f32 = -20.0;
/// grGarden_80203560: the splash generator (stage bank 30, id 0x753A) of a
/// fighter entering the water, at this height.
pub const SPLASH_BANK: u8 = 30;
pub const SPLASH_PARTICLE: u32 = 0x753A;
const SPLASH_HEIGHT: f32 = -30.0;
/// grGarden_8020349C: the splash's scale is the fighter's camera size
/// (ftLib_80086B80) over this.
const SPLASH_SCALE_DIVISOR: f32 = 10.0;

/// `HSD_Randi(n)` when `n` is not zero, else zero (grgarden.c's inline).
fn randi_or_zero(rng: &mut HsdRng, n: i32) -> i32 {
    if n != 0 {
        rng.randi(n)
    } else {
        0
    }
}

/// Cranky Kong (map 1, `GroundVars_garden`): gp+C4 the animation, gp+C8
/// the loops left before the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cranky {
    pub animation: i32,
    pub loops_left: i32,
}
impl Cranky {
    /// grGarden_80203004 (0x80203004): animation 0 with its random wait.
    pub fn new(p: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            animation: 0,
            loops_left: randi_or_zero(rng, p.cranky_rest_loops) + 1,
        }
    }
    /// grGarden_80203098 (0x80203098). `cycled`: the first AObj ended or
    /// rewound this frame (grAnime_801C83D0 || grAnime_801C84A4). Returns
    /// the animation grAnime_801C8138 starts.
    pub fn tick(&mut self, p: &Parameters, cycled: bool, rng: &mut HsdRng) -> Option<usize> {
        if !cycled {
            return None;
        }
        if self.loops_left != 0 {
            self.loops_left -= 1;
            return None;
        }
        self.animation += 1;
        if self.animation > 3 {
            self.animation = 0;
        }
        match self.animation {
            0 => self.loops_left = randi_or_zero(rng, p.cranky_rest_loops) + 1,
            2 => self.loops_left = randi_or_zero(rng, p.cranky_busy_loops) + 1,
            _ => {}
        }
        Some(self.animation as usize)
    }
}

/// The Klaptrap (map 3, `GroundVars_garden2`): gp+C8 (s8) the phase,
/// gp+CC the frame counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Klaptrap {
    pub leaping: bool,
    pub timer: i32,
}

/// What one Klaptrap step asks of the engine, in this order: the item's
/// state, then the model's animation. A leap then draws its x
/// ([`Klaptrap::leap_x`]) after the animation's frame-zero keys.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KlaptrapStep {
    /// Item_80268E5C(item, state, ITEM_ANIM_UPDATE).
    pub item_state: Option<u16>,
    /// grAnime_801C8138(gobj, map, 0), then HSD_JObjSetTranslateX of the
    /// model root at [`Klaptrap::leap_x`].
    pub start_leap: bool,
    /// HSD_JObjRemoveAnimAll on the model root.
    pub remove_animation: bool,
}

impl Klaptrap {
    /// grGarden_802031A4 (0x802031A4), after the item is created.
    pub fn new(p: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            leaping: false,
            timer: p.klaptrap_wait_minimum + randi_or_zero(rng, p.klaptrap_wait_range),
        }
    }
    /// grGarden_80203250's leap position, drawn after grAnime_801C8138:
    /// retail 802032D4..80203334 fsubs, fabs, fctiwz, HSD_Randi, then the
    /// draw's float added to x18 (fadds).
    pub fn leap_x(p: &Parameters, rng: &mut HsdRng) -> f32 {
        let mut span = p.klaptrap_x[1] - p.klaptrap_x[0];
        if span < 0.0 {
            span = -span;
        }
        let offset = randi_or_zero(rng, fctiwz(span));
        p.klaptrap_x[0] + offset as f32
    }
    /// grGarden_80203250 (0x80203250). `ended`: the leap's first AObj
    /// ended (grAnime_801C83D0); read only while leaping.
    pub fn tick(
        &mut self,
        p: &Parameters,
        ended: impl FnOnce() -> bool,
        rng: &mut HsdRng,
    ) -> KlaptrapStep {
        let mut step = KlaptrapStep::default();
        if !self.leaping {
            if self.timer != 0 {
                self.timer -= 1;
                return step;
            }
            step.item_state = Some(klaptrap_item::BITE);
            step.start_leap = true;
            self.timer = 0;
            self.leaping = true;
            return step;
        }
        // Ground_801C53EC at the splash frames: sound only, no draw.
        let _ = (SPLASH_OUT_FRAMES, SPLASH_IN_FRAMES);
        if ended() {
            step.item_state = Some(klaptrap_item::IDLE);
            step.remove_animation = true;
            self.timer = p.klaptrap_wait_minimum + randi_or_zero(rng, p.klaptrap_wait_range);
            self.leaping = false;
        }
        // 80203468: the counter advances after a reset too.
        self.timer += 1;
        step
    }
}

/// The river (map 2's device, grGarden_8020349C): what it does to one
/// fighter this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Water {
    pub level: f32,
    pub current: f32,
}
/// A fighter entering the water: grGarden_80203560's generator, or
/// the castle moat's (grCastle_801D0A9C).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Splash {
    pub position: Vec3,
    pub scale: f32,
    /// The castle moat's (its generator is scaled after creation).
    pub castle: bool,
}
impl Water {
    /// grGarden_8020349C (0x8020349C): below the line the current pushes
    /// along x; crossing the line downward this frame splashes. `size` is
    /// the fighter's camera size (ftLib_80086B80).
    pub fn sample(&self, current: Vec3, previous: Vec3, size: f32) -> (Vec3, Option<Splash>) {
        if current.y >= self.level {
            return (Vec3::ZERO, None);
        }
        let splash = (previous.y > self.level).then(|| Splash {
            position: Vec3::new(current.x, SPLASH_HEIGHT, current.z),
            scale: size / SPLASH_SCALE_DIVISOR,
            castle: false,
        });
        (Vec3::new(self.current, 0.0, 0.0), splash)
    }
}

/// Jungle Japes' stage state.
#[derive(Clone, Debug)]
pub struct Garden {
    pub parameters: Parameters,
    pub cranky: Cranky,
    pub klaptrap: Klaptrap,
    /// Ground_801C10B8 deferred grGarden_80202D9C to the match start: the
    /// river device is registered only then.
    pub water_active: bool,
}
impl Garden {
    /// grGarden_80202B70's on_inits in creation order: Cranky (map 1)
    /// draws first, then the Klaptrap (map 3).
    pub fn initialize(parameters: Parameters, rng: &mut HsdRng) -> Self {
        let cranky = Cranky::new(&parameters, rng);
        let klaptrap = Klaptrap::new(&parameters, rng);
        Self {
            parameters,
            cranky,
            klaptrap,
            water_active: false,
        }
    }
    /// Ground_801C0FB8 at the countdown's end: the deferred
    /// grGarden_80202D9C registers the river (ftCo_800C06E8(gobj, 4, ..)).
    pub fn start(&mut self) {
        self.water_active = true;
    }
    pub fn water(&self) -> Option<Water> {
        self.water_active.then_some(Water {
            level: self.parameters.water_level,
            current: self.parameters.current,
        })
    }
}

#[cfg(test)]
mod tests;
