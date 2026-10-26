//! Map 6's proc state (`u.zebes5` on map 6, grZebes_801D881C): the top
//! platform, which sinks once the chain is gone and comes back growing a
//! new one; the bubbles' pulse and loose-bubble clocks.
use super::{
    bubbles::{BubbleEngine, BubbleState, Bubbles},
    Parameters,
};
use gekko_math::{
    fma::{fmadd, fmadds},
    msl::fctiwz,
};

/// The top platform's phase (xC4, s16).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IslandPhase {
    Standing = 0,
    /// Animation 3: it sinks.
    Sinking = 1,
    /// Waiting `island_wait`.
    Away = 2,
    /// Animation 4: it rises, growing the chain from both ends.
    Rising = 3,
}

impl IslandPhase {
    pub fn from_saved(value: i16) -> Option<Self> {
        Some(match value {
            0 => Self::Standing,
            1 => Self::Sinking,
            2 => Self::Away,
            3 => Self::Rising,
            _ => return None,
        })
    }
}

/// The animations grAnime_801C8098 plays on map 6's bone 14 and map 7's
/// bone 1 (their own tracks only).
pub const SINK_ANIMATION: usize = 3;
pub const RISE_ANIMATION: usize = 4;

/// Map 6's xF8 as a match started from the menus finds it: the stale heap
/// word the Ground's block carries. Two retail boundaries with different
/// rosters (start_brin_fox_marth4, start_brin_falco_peach4) both hold -849,
/// so the first tick pulses.
pub const STALE_PULSE_TIMER: i16 = -849;

/// Map 6's controller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Island {
    /// xC4.
    pub phase: IslandPhase,
    /// xC6 (s16): the wait while away.
    pub timer: i16,
    /// xF6 (s16): ticks risen.
    pub risen: i16,
    /// xF8 (s16): ticks to the next pulse.
    pub pulse_timer: i16,
    /// xF4 (s16): ticks to the next loose bubble.
    pub loose_timer: i16,
}

/// What the top platform asks of maps 6 and 7.
pub trait IslandEngine {
    /// grAnime_801C8098(gobj, bone, 1, animation, 0.0, 1.0) then
    /// grAnime_801C7980(gobj, bone, 1) on map 6's bone 14 and map 7's bone 1.
    fn play(&mut self, animation: usize);
    /// grAnime_801C83D0(map 6, 14, 1).
    fn ended(&self) -> bool;
}

impl Island {
    /// grZebes_801D8644: xC4 = 0, xF4 = 0. xC6, xF6 and xF8 are not written:
    /// alloc_user_data_ground returns the block uncleared (ground.c:840).
    /// xC6 and xF6 are written before they are read; xF8 is read on the
    /// first tick, as [`STALE_PULSE_TIMER`].
    pub fn new() -> Self {
        Self {
            phase: IslandPhase::Standing,
            timer: 0,
            risen: 0,
            pulse_timer: STALE_PULSE_TIMER,
            loose_timer: 0,
        }
    }

    /// The pulse clock (0x801D88E4): when it has run out, a pulse
    /// (grZebes_801DAA08) and the next wait (0x801D8910 fmadds, fctiwz).
    pub fn tick_pulse(
        &mut self,
        bubbles: &mut Bubbles,
        parameters: &Parameters,
        engine: &mut impl BubbleEngine,
    ) {
        let timer = self.pulse_timer;
        self.pulse_timer = timer.wrapping_sub(1);
        if timer < 0 {
            bubbles.pulse(engine);
            let [low, high] = parameters.pulse_wait;
            let draw = engine.rng().randf();
            self.pulse_timer = fctiwz(fmadds(high - low, draw, low)) as i16;
        }
    }

    /// grZebes_801DC408 (0x801DC408): loose bubbles grow only while the
    /// platform stands.
    pub fn grow_loose(
        &mut self,
        bubbles: &mut Bubbles,
        parameters: &Parameters,
        engine: &mut impl BubbleEngine,
    ) {
        if self.phase == IslandPhase::Standing {
            bubbles.grow_loose(&mut self.loose_timer, parameters, engine);
        }
    }

    /// The platform's switch (0x801D8944), `broken` from this tick's
    /// grZebes_801DB3CC.
    pub fn tick(
        &mut self,
        broken: bool,
        bubbles: &mut Bubbles,
        parameters: &Parameters,
        engine: &mut impl BubbleEngine,
        models: &mut impl IslandEngine,
    ) {
        match self.phase {
            IslandPhase::Standing => {
                if broken {
                    self.phase = IslandPhase::Sinking;
                    models.play(SINK_ANIMATION);
                }
            }
            IslandPhase::Sinking => {
                if models.ended() {
                    self.phase = IslandPhase::Away;
                    let [low, high] = parameters.island_wait;
                    let draw = engine.rng().randf();
                    // retail 0x801D8A10: fmadds, fctiwz.
                    self.timer = fctiwz(fmadds(high - low, draw, low)) as i16;
                }
            }
            IslandPhase::Away => {
                self.timer = self.timer.wrapping_sub(1);
                if self.timer < 0 {
                    self.phase = IslandPhase::Rising;
                    models.play(RISE_ANIMATION);
                    self.risen = 0;
                    bubbles.cooldown = 0;
                }
            }
            IslandPhase::Rising => {
                self.risen = self.risen.wrapping_add(1);
                let step = parameters.regrowth_step;
                let risen = i32::from(self.risen);
                if risen % step == 0 {
                    regrow(bubbles, risen / step, parameters, engine);
                }
                if models.ended() {
                    self.phase = IslandPhase::Standing;
                    for bubble in bubbles.entries.iter_mut() {
                        if bubble.state == BubbleState::Growing {
                            bubble.state = BubbleState::Held;
                        }
                    }
                }
            }
        }
    }
}

impl Default for Island {
    fn default() -> Self {
        Self::new()
    }
}

/// One regrowth step (0x801D8AD8): link `step` grows where link `step - 1`
/// is, and link `6 - step` where link `7 - step` is, inward from both
/// anchors. (The C reads `grZe_BubbleSpawnPos` rows over grZe_8049F140,
/// which land on the previous link's position.)
fn regrow(
    bubbles: &mut Bubbles,
    step: i32,
    parameters: &Parameters,
    engine: &mut impl BubbleEngine,
) {
    let mirror = 6 - step;
    if step < mirror {
        let draw = engine.rng().randf();
        let [low, high] = parameters.regrowth_size;
        // retail 0x801D8B1C: fmadds.
        let size = fmadds(high - low, draw, low);
        let (x, y) = source(bubbles, step - 1);
        bubbles.spawn_at(step as usize, BubbleState::Growing, x, y, size, engine);
    }
    if step <= mirror {
        let draw = engine.rng().randf();
        // retail 0x801D8B3C: fmadd (double), frsp.
        let size = fmadd(0.5, f64::from(draw), 1.0) as f32;
        let (x, y) = source(bubbles, mirror + 1);
        bubbles.spawn_at(mirror as usize, BubbleState::Growing, x, y, size, engine);
    }
}

/// `grZe_BubbleSpawnPos[row]`'s x and y: for a row of 1 or more, link
/// `row - 1`'s position; for row 0, the right bound's z and the left
/// anchor's x.
fn source(bubbles: &Bubbles, link: i32) -> (f32, f32) {
    if link < 0 {
        // pos[0]: grZe_8049F140 + 0x14 and + 0x18.
        return (bubbles.points.bounds[1].z, bubbles.points.anchors[0].x);
    }
    let bubble = &bubbles.entries[link as usize];
    (bubble.x, bubble.y)
}
