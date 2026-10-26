//! The Bullet Bill (maps 8..16, grCastle_801CE260 / 801CE578), the
//! roof's countdown to it (grCastle_801CDFD8 / 801CE19C), the cannon it
//! launches (map 2, grCastle_801CE7E8 / 801CE860) and its explosion
//! (map 1, grCastle_801CE8E8 / 801CE9E8).
use super::{randi_or_zero, Parameters};
use gekko_math::HsdRng;

/// The first flight's map; flight `n` is map `FIRST_FLIGHT_MAP + n`.
pub const FIRST_FLIGHT_MAP: u8 = 8;
pub const FLIGHTS: usize = 9;
pub const CANNON_MAP: u8 = 2;
pub const EXPLOSION_MAP: u8 = 1;
/// The material items' states (grMaterial_801C8CFC(0, 3 / 4)).
pub const BILL_ITEM_STATE: u16 = 3;
pub const EXPLOSION_ITEM_STATE: u16 = 4;
/// grCastle_801CE578's lb_800119DC(&pos, 0xB4, 20.0, 0.1, 1.0471976f).
pub const BLAST_FRAMES: i32 = 0xB4;
pub const BLAST_STRENGTH: f32 = 20.0;
pub const BLAST_DECAY: f32 = 0.1;
pub const BLAST_PHASE_STEP: f32 = core::f32::consts::FRAC_PI_3;
/// grCastle_801CDF54: a block within this of the Bill stays hidden.
pub const BLOCK_CLEARANCE: f32 = 40.0;

/// Map 3's countdown (`castle9`/`castle4`: xD4..xDE).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BillTimer {
    /// xDE b0: counting down (cleared while a Bill flies).
    pub counting: bool,
    /// xD4.
    pub timer: i16,
    /// xD6: the slot the next flight is remembered in.
    pub slot: i16,
    /// xD8 / xDA / xDC: the last three flights, never repeated.
    pub recent: [i16; 3],
}

impl BillTimer {
    /// grCastle_801CDFD8 (0x801CDFD8).
    pub fn new(p: &Parameters, rng: &mut HsdRng) -> Self {
        let timer = (p.first_bill_wait[0] + randi_or_zero(p.first_bill_wait[1], rng)) as i16;
        Self {
            counting: true,
            timer,
            slot: 0,
            recent: [-1; 3],
        }
    }

    /// grCastle_801CE19C (0x801CE19C): the flight to launch now, if any
    /// (grCastle_801CE054 draws it).
    pub fn step(&mut self, rng: &mut HsdRng) -> Option<usize> {
        if !self.counting {
            return None;
        }
        let timer = self.timer;
        self.timer = timer.wrapping_sub(1);
        if timer >= 0 {
            return None;
        }
        let flight = self.pick(rng);
        self.counting = false;
        Some(flight)
    }

    /// grCastle_801CE054 (0x801CE054): any flight but the last three.
    fn pick(&mut self, rng: &mut HsdRng) -> usize {
        let mut choices = [0i16; FLIGHTS];
        let mut count = 0;
        for flight in 0..FLIGHTS as i16 {
            if !self.recent.contains(&flight) {
                choices[count] = flight;
                count += 1;
            }
        }
        let index = randi_or_zero(count as i32, rng) as usize;
        let picked = choices[index];
        self.recent[self.slot as usize] = picked;
        self.slot = if self.slot == 2 { 0 } else { self.slot + 1 };
        picked as usize
    }

    /// grCastle_801CE578's restart after an explosion.
    pub fn restart(&mut self, p: &Parameters, rng: &mut HsdRng) {
        self.counting = true;
        self.timer = (p.next_bill_wait[0] + randi_or_zero(p.next_bill_wait[1], rng)) as i16;
    }
}

/// One flying Bill (`castle11`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bill {
    /// xC6: the flight (its map is `FIRST_FLIGHT_MAP + flight`).
    pub flight: usize,
    /// xC4 b0: the flight's animation ended; the fuse burns.
    pub landed: bool,
    /// xC8: the fuse.
    pub fuse: i16,
    /// xCA: ticks until the cannon appears.
    pub cannon_delay: i16,
    /// xCC: the cannon was made (it stays set once the cannon is removed).
    pub cannon_made: bool,
}

/// What a Bill step asks of the stage, in retail order.
pub trait BillEngine {
    /// The subject at the Bill's first bone (lb_8000B1CC).
    fn follow_subject(&mut self);
    /// grCastle_801CD4D0(2), Ground_801C53EC, then grCastle_801CE3AC.
    fn make_cannon(&mut self);
    /// grAnime_801C83D0(gobj, 0, 1).
    fn flight_ended(&self) -> bool;
    /// grMaterial_801C9604(x114) (a colour script: drawing only), then
    /// Ground_801C4A08 on the cannon if it was made.
    fn land(&mut self, cannon_made: bool);
    /// The explosion: the Bill's item destroyed, map 1 at the Bill's first
    /// bone, lb_800119DC there, the countdown restarted, the subject
    /// removed and the Bill's map with it.
    fn explode(&mut self);
}

impl Bill {
    /// grCastle_801CE260 (0x801CE260).
    pub fn new(flight: usize, p: &Parameters) -> Self {
        Self {
            flight,
            landed: false,
            fuse: 0,
            cannon_delay: p.flights[flight].cannon_delay as i16,
            cannon_made: false,
        }
    }

    /// grCastle_801CE578 (0x801CE578). True once the Bill is gone.
    pub fn step<E: BillEngine>(&mut self, p: &Parameters, engine: &mut E) -> bool {
        engine.follow_subject();
        if !self.cannon_made {
            let delay = self.cannon_delay;
            self.cannon_delay = delay.wrapping_sub(1);
            if delay < 0 {
                self.cannon_made = true;
                engine.make_cannon();
            }
        }
        if !self.landed {
            if engine.flight_ended() {
                self.fuse = p.bill_fuse as i16;
                self.landed = true;
                engine.land(self.cannon_made);
            }
            return false;
        }
        let fuse = self.fuse;
        self.fuse = fuse.wrapping_sub(1);
        if fuse < 0 {
            engine.explode();
            return true;
        }
        false
    }
}

/// The cannon (map 2, `castle.xC4` / `xC8`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cannon {
    /// xC4: its animation (0..3).
    pub animation: i32,
    /// xC8: ticks to the next.
    pub timer: i16,
}

impl Cannon {
    /// grCastle_801CE7E8 (0x801CE7E8): animation 0.
    pub fn new(p: &Parameters) -> Self {
        Self {
            animation: 0,
            timer: p.cannon_waits[0] as i16,
        }
    }

    /// grCastle_801CE860 (0x801CE860): the next animation to play, if any.
    /// A Loop quake follows every step.
    pub fn step(&mut self, p: &Parameters) -> Option<usize> {
        if self.animation < 3 {
            let timer = self.timer;
            self.timer = timer.wrapping_sub(1);
            if timer < 0 {
                self.animation += 1;
                self.timer = p.cannon_waits[self.animation as usize] as i16;
                return Some(self.animation as usize);
            }
        }
        None
    }
}
