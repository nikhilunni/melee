//! Mushroom Kingdom (`gr/grinishie1.c`, GrI1.dat, StKind 0x13). Map 3 is
//! the stage: nineteen brick blocks (one collision joint and one stage
//! material item each) that break when bumped from below or hit and come
//! back after a random wait, and the two lifts on a pulley that tip under
//! weight, fall past the blast zone when they tip too far and return.
//! With items on, `?` blocks appear among the bricks and drop items; every
//! ported boundary has items off, so that half stays fail-closed.
pub mod procs;
use gekko_math::HsdRng;

/// `grInishie1_YakumonoParam` (grinishie1.c:117-136, GrI1.dat).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x00 / +0x04: the first `?` block's wait (floats read as ints).
    pub hatena1_wait: [f32; 2],
    /// +0x08 / +0x0C: the second `?` block's wait.
    pub hatena2_wait: [f32; 2],
    /// +0x10: the all-`?` event's cooldown.
    pub all_hatena_cooldown: f32,
    /// +0x14: the window in which both `?` waits ending turns every block.
    pub all_hatena_window: i16,
    /// +0x16: a `?` block's flicker.
    pub hatena_flicker: i16,
    /// +0x18: a returning brick's flicker.
    pub return_flicker: u16,
    /// +0x1A / +0x1C: a broken brick's wait, `rand_range(1C, 1A)`.
    pub respawn_wait: [i16; 2],
    /// +0x20: the lifts' largest offset before they fall.
    pub lift_max_offset: f32,
    /// +0x24: weight difference to acceleration.
    pub lift_acceleration: f32,
    /// +0x28: the lifts' top speed.
    pub lift_max_speed: f32,
    /// +0x2C: the speed's damping with weight on both lifts.
    pub lift_damping_both: f32,
    /// +0x30: the damping otherwise.
    pub lift_damping_one: f32,
    /// +0x34: the friction subtracted from the speed otherwise.
    pub lift_friction: f32,
    /// +0x38: the return speed with no weight on either lift.
    pub lift_return_speed: f32,
    /// +0x3C: the weight a lift ignores.
    pub lift_free_weight: f32,
    /// +0x40: a standing collider's weight multiplier (`ground_kind` 1).
    pub lift_standing_weight: f32,
    /// +0x44 / +0x48: a falling lift's acceleration and top speed.
    pub lift_fall_acceleration: f32,
    pub lift_fall_speed: f32,
    /// +0x4C: the ticks a fallen lift stays away.
    pub lift_away_ticks: f32,
    /// +0x50: the returning lifts' speed.
    pub lift_settle_speed: f32,
}

/// `rand_range(a, b)` (gr/inlines.h) as grInishie1_SetRespawnTimer
/// writes it: `a + HSD_Randi(b - a)` between the two, else `b`.
fn rand_range(min: i32, max: i32, rng: &mut HsdRng) -> i32 {
    let draw = |n: i32, rng: &mut HsdRng| if n != 0 { rng.randi(n) } else { 0 };
    if max > min {
        min + draw(max - min, rng)
    } else if max < min {
        max + draw(min - max, rng)
    } else {
        max
    }
}

/// A brick's `x2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrickPhase {
    /// 0: broken, waiting `respawn` ticks and then for room.
    Broken,
    /// 1: back, flickering.
    Returning,
    /// 2: solid.
    Solid,
}
impl BrickPhase {
    pub fn from_saved(value: i16) -> Option<Self> {
        Some(match value {
            0 => Self::Broken,
            1 => Self::Returning,
            2 => Self::Solid,
            // 3 and 4 (the bump grInishie1_801FB3F0 animates) have no
            // writer in grinishie1.c.
            _ => return None,
        })
    }
}

/// One brick (`grInishie1_Block`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brick {
    /// x2.
    pub phase: BrickPhase,
    /// x4: counts down while solid (nothing reads it).
    pub cooldown: i16,
    /// x8: the bone's translation y at creation.
    pub base_y: f32,
    /// xC / x10: the bump's offset and speed (unreachable).
    pub offset: f32,
    pub speed: f32,
    /// x18 `status`: 0 a brick, 1..3 a `?` block.
    pub hatena: i16,
    /// x20: ticks before a broken brick may return.
    pub respawn: i16,
    /// x22: flicker ticks left.
    pub flicker: i16,
}

/// What the bricks' step does to the stage.
pub trait BrickEngine {
    /// grLib_801C9EE8(lb_8000B1CC(bone), 15.0): something stands there.
    fn occupied(&mut self, brick: usize) -> bool;
    /// The brick shows again (DObj and JObj flags) and takes hits
    /// (grMaterial_801C8E08).
    fn show(&mut self, brick: usize);
}

impl Brick {
    pub fn new(base_y: f32) -> Self {
        Self {
            phase: BrickPhase::Solid,
            cooldown: 0,
            base_y,
            offset: 0.0,
            speed: 0.0,
            hatena: 0,
            respawn: 0,
            flicker: 0,
        }
    }

    /// grInishie1_801FB3F0's per-block step (0x801FB3F0).
    pub fn step<E: BrickEngine>(&mut self, index: usize, p: &Parameters, engine: &mut E) {
        match self.phase {
            BrickPhase::Solid => {
                if self.cooldown != 0 {
                    self.cooldown -= 1;
                }
            }
            BrickPhase::Broken => {
                if self.respawn != 0 {
                    self.respawn -= 1;
                } else if !engine.occupied(index) {
                    engine.show(index);
                    self.phase = BrickPhase::Returning;
                    self.flicker = p.return_flicker as i16;
                }
                assert_eq!(self.hatena, 0, "grinishie1.c:704: a `?` block's model");
            }
            BrickPhase::Returning => {
                if self.flicker > 0 {
                    // The DObjs blink (drawing only).
                    assert_eq!(self.hatena, 0, "grinishie1.c:723: a `?` block's flicker");
                    self.flicker -= 1;
                } else {
                    self.phase = BrickPhase::Solid;
                    self.cooldown = 0;
                }
            }
        }
    }

    /// grInishie1_801FB0AC then 801FBCEC's controller half (0x801FBCEC):
    /// broken, with its wait drawn. Returns nothing; the scene hides the
    /// model, stops its hits and makes the effect and the quake.
    pub fn break_apart(&mut self, p: &Parameters, rng: &mut HsdRng) {
        assert_eq!(self.hatena, 0, "grinishie1.c:640: a `?` block breaks");
        self.phase = BrickPhase::Broken;
        // x20 is an s16.
        self.respawn =
            rand_range(i32::from(p.respawn_wait[0]), i32::from(p.respawn_wait[1]), rng) as i16;
    }
}

/// The lifts (`GroundVars_inishie1` xE0..x10C).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lifts {
    /// xEE: 0 balancing, 1 falling, 2 away, 3 returning.
    pub phase: LiftPhase,
    /// xE0 / xE4: this tick's weights on lifts 0x14 and 0x15.
    pub weight: [f32; 2],
    /// xE8 / xEA: this tick's callers.
    pub count: [i16; 2],
    /// xF0: lift 0x14's offset (0x15 moves the other way).
    pub offset: f32,
    /// xF4: its speed.
    pub speed: f32,
    /// xEC: ticks away.
    pub away: i16,
    /// xFC: the lifts' translation y at creation.
    pub base_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiftPhase {
    Balancing,
    Falling,
    Away,
    Returning,
}
impl LiftPhase {
    pub fn from_saved(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Balancing,
            1 => Self::Falling,
            2 => Self::Away,
            3 => Self::Returning,
            _ => return None,
        })
    }
}

/// What the lifts' step does to their bones.
pub trait LiftEngine {
    /// HSD_JObjSetTranslateY on the two lifts (bones 0x1A and 0x1C).
    fn set_lift_y(&mut self, lift: usize, y: f32);
    /// HSD_JObjAddTranslationY on each lift's child.
    fn drop_children(&mut self, dy: f32);
    /// lb_8000B1CC's y of each lift's child.
    fn child_heights(&mut self) -> [f32; 2];
    /// HSD_JObjSetTranslateY(child, 0) and mpLib_80055E9C / 80057424 on
    /// joints 0x14 and 0x15.
    fn reset_children(&mut self);
    /// Stage_GetBlastZoneBottomOffset.
    fn blast_zone_bottom(&self) -> f32;
}

/// grInishie1_801FC110: weights past this saturate.
const WEIGHT_LIMIT: f32 = 100000.0;
/// grInishie1_801FC664: a falling lift is away 30 below the blast zone.
const AWAY_BELOW: f32 = -30.0;

impl Lifts {
    pub fn new(base_y: f32) -> Self {
        Self {
            phase: LiftPhase::Balancing,
            weight: [0.0; 2],
            count: [0; 2],
            offset: 0.0,
            speed: 0.0,
            away: 0,
            base_y,
        }
    }

    /// fn_801FC9AC (0x801FC9AC), joints 0x14 and 0x15's floor callback:
    /// a standing collider (`ground_kind` 1) weighs `x50 * +0x40`, a
    /// landing one `x50`.
    pub fn weigh(&mut self, lift: usize, ground_kind: i32, coll_x50: i32, p: &Parameters) {
        self.count[lift] += 1;
        if ground_kind == 1 {
            // 0x801FC9F4: fmadds x50 * +0x40 + weight.
            self.weight[lift] =
                gekko_math::fma::fmadds(coll_x50 as f32, p.lift_standing_weight, self.weight[lift]);
        } else {
            self.weight[lift] += coll_x50 as f32;
        }
    }

    /// grInishie1_801FC664 (0x801FC664).
    pub fn step<E: LiftEngine>(&mut self, p: &Parameters, engine: &mut E) {
        match self.phase {
            LiftPhase::Balancing => self.balance(p, engine),
            LiftPhase::Falling => {
                self.speed += p.lift_fall_acceleration;
                if self.speed > p.lift_fall_speed {
                    self.speed = p.lift_fall_speed;
                }
                engine.drop_children(-self.speed);
                let bottom = AWAY_BELOW + engine.blast_zone_bottom();
                let [left, right] = engine.child_heights();
                if left < bottom && right < bottom {
                    self.phase = LiftPhase::Away;
                    self.speed = 0.0;
                    self.away = gekko_math::msl::fctiwz(p.lift_away_ticks) as i16;
                }
            }
            LiftPhase::Away => {
                self.away -= 1;
                if self.away == 0 {
                    self.phase = LiftPhase::Returning;
                    engine.reset_children();
                }
            }
            LiftPhase::Returning => self.settle(p, engine),
        }
    }

    fn place<E: LiftEngine>(&self, engine: &mut E) {
        engine.set_lift_y(0, self.base_y + self.offset);
        engine.set_lift_y(1, self.base_y - self.offset);
    }

    /// grInishie1_801FC110 (0x801FC110).
    fn balance<E: LiftEngine>(&mut self, p: &Parameters, engine: &mut E) {
        if self.weight[0] > WEIGHT_LIMIT || self.weight[1] > WEIGHT_LIMIT {
            self.weight = [WEIGHT_LIMIT; 2];
        }
        let mut low = self.weight[0] - p.lift_free_weight;
        if low < 0.0 {
            low = 0.0;
        }
        let mut high = self.weight[1] - p.lift_free_weight;
        if high < 0.0 {
            high = 0.0;
        }
        if low == 0.0 && high == 0.0 {
            let magnitude = if self.offset < 0.0 { -self.offset } else { self.offset };
            if magnitude > 0.0 {
                if self.offset < 0.0 {
                    self.offset += p.lift_return_speed;
                    if self.offset > 0.0 {
                        self.offset = 0.0;
                    }
                } else {
                    self.offset -= p.lift_return_speed;
                    if self.offset < 0.0 {
                        self.offset = 0.0;
                    }
                }
            }
            self.speed = 0.0;
        } else {
            // 0x801FC230: fmadds (high - low) * +0x24 + speed.
            self.speed = gekko_math::fma::fmadds(high - low, p.lift_acceleration, self.speed);
            if self.speed > p.lift_max_speed {
                self.speed = p.lift_max_speed;
            } else if self.speed < -p.lift_max_speed {
                self.speed = -p.lift_max_speed;
            }
            if low != 0.0 && high != 0.0 && self.speed != 0.0 {
                self.speed *= p.lift_damping_both;
            } else {
                self.speed *= p.lift_damping_one;
                if self.speed > 0.0 {
                    self.speed -= p.lift_friction;
                    if self.speed < 0.0 {
                        self.speed = 0.0;
                    }
                } else if self.speed < 0.0 {
                    self.speed += p.lift_friction;
                    if self.speed > 0.0 {
                        self.speed = 0.0;
                    }
                }
            }
            self.offset += self.speed;
        }
        let magnitude = if self.offset < 0.0 { -self.offset } else { self.offset };
        if magnitude > p.lift_max_offset {
            self.offset = if self.offset < 0.0 {
                -p.lift_max_offset
            } else {
                p.lift_max_offset
            };
            self.phase = LiftPhase::Falling;
            self.speed = 0.0;
        }
        self.place(engine);
        self.count = [0; 2];
        self.weight = [0.0; 2];
    }

    /// grInishie1_801FC4A0 (0x801FC4A0).
    fn settle<E: LiftEngine>(&mut self, p: &Parameters, engine: &mut E) {
        let mut reached = false;
        if self.offset != 0.0 {
            if self.offset < 0.0 {
                self.offset += p.lift_settle_speed;
                if self.offset >= 0.0 {
                    reached = true;
                }
            } else {
                self.offset -= p.lift_settle_speed;
                if self.offset <= 0.0 {
                    reached = true;
                }
            }
        }
        if reached {
            self.offset = 0.0;
            self.speed = 0.0;
            self.weight = [0.0; 2];
            self.phase = LiftPhase::Balancing;
        }
        self.place(engine);
    }
}
