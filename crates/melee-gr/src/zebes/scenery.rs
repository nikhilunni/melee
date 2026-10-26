//! Brinstar's display-only maps that still keep clocks: map 4
//! (grZebes_801D9508 / 801D95B8), which rides the top platform and plays
//! its two animations in turn, drawing its waits; and map 1
//! (grZebes_801D9F30 / 801D9F84), whose bone 3 shakes when the acid
//! rises past two heights.
use gekko_math::HsdRng;

/// Map 4's controller (`u.zebes` on map 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ornament {
    /// x0_b0: set at creation, cleared by the deferred fn_801D94F0 at the
    /// stage's start.
    pub waiting: bool,
    /// x8 (s16): 1 before animation 0 plays, -1 before animation 1.
    pub next: i16,
    /// xA (s16): ticks before the next animation, 0 while one plays.
    pub timer: i16,
}

/// The wait between two animations: `HSD_Randi(600) + 3000`.
fn wait(rng: &mut HsdRng) -> i16 {
    (rng.randi(600) + 3000) as i16
}

/// What map 4's step asks of its model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrnamentStep {
    Nothing,
    /// grAnime_801C8138(gobj, 4, animation).
    Play(usize),
}

impl Ornament {
    /// grZebes_801D9508 (0x801D9508): its first wait; no animation yet.
    pub fn create(rng: &mut HsdRng) -> Self {
        Self {
            waiting: true,
            next: 1,
            timer: wait(rng),
        }
    }

    /// grZebes_801D95B8 (0x801D95B8) after it copies map 6's bone 14
    /// translation: `ended` is grAnime_801C83D0(gobj, 0, 7).
    pub fn tick(&mut self, ended: bool, rng: &mut HsdRng) -> OrnamentStep {
        if self.waiting {
            return OrnamentStep::Nothing;
        }
        if self.timer > 0 {
            self.timer -= 1;
            if self.timer == 0 {
                if self.next > 0 {
                    self.next = -1;
                    return OrnamentStep::Play(0);
                }
                self.next = 1;
                return OrnamentStep::Play(1);
            }
        } else if ended {
            self.timer = wait(rng);
        }
        OrnamentStep::Nothing
    }
}

/// Map 1's controller (`u.zebes2.xC4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AcidReaction {
    /// 1 while bone 3's shake plays.
    pub shaking: bool,
}

/// Map 1's bone 3 animations: still, a small and a large shake.
pub const REACTION_BONE: usize = 3;
pub const STILL: usize = 0;
pub const SMALL_SHAKE: usize = 1;
pub const LARGE_SHAKE: usize = 2;

impl AcidReaction {
    /// grZebes_801D9F84 (0x801D9F84): the shake to start this tick, if
    /// any (grAnime_801C7FF8(gobj, 3, 7, n, 0, 1) and its loop cleared),
    /// or the still pose once one `ended`.
    pub fn tick(&mut self, floor: f32, ended: bool) -> Option<usize> {
        if !self.shaking {
            let shake = if floor > 5.0 {
                LARGE_SHAKE
            } else if floor > -20.0 {
                SMALL_SHAKE
            } else {
                return None;
            };
            // Ground_801C53EC(400005): a sound.
            self.shaking = true;
            Some(shake)
        } else if ended {
            self.shaking = false;
            Some(STILL)
        } else {
            None
        }
    }
}
