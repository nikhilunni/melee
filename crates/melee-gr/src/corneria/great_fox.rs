//! The Great Fox (map 3, `u.corneria`): its drift, its descent over the
//! sea and its cannon.
use super::Parameters;
use gekko_math::{fma, msl::fctiwz, HsdRng};
use hsd_types::Vec3;

/// grCorneria_801DD674: the ship's rest position (`base_x`, `base_y`).
pub const BASE: [f32; 2] = [0.0, 250.0];

/// grCorneria_801E2228's step for the descent speed (and its floor), and
/// its top speed.
const DESCENT_STEP: f32 = 0.005;
const DESCENT_TOP_SPEED: f32 = 3.0;
/// `-0.0025f`: half the step, negated (the braking distance's term).
const DESCENT_BRAKE: f32 = -0.0025;
/// The ticks the ship stays down before it rises again (`x100 < 0x384`).
const DESCENT_TICKS: i32 = 900;

/// The drift (801E1970) and the descent (801E2228): `u.corneria` from
/// 0xD0 to 0x104 and the flags `xC4` b1 and b2.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GreatFox {
    /// xD0: how far the ship has lowered itself (zero or below).
    pub descent: f32,
    /// xDC / xE0: the drift's offset from [`BASE`].
    pub offset: [f32; 2],
    /// xE4.x / xE4.y: the offset before this tick's step
    /// (grCorneria_801DDCF0 reads `offset - previous`).
    pub previous_offset: [f32; 2],
    /// xE4.z / xF0: the drift's velocity.
    pub velocity: [f32; 2],
    /// xF4 / xF8: its acceleration.
    pub acceleration: [f32; 2],
    /// xFC: the descent's speed.
    pub descent_speed: f32,
    /// x100: ticks since the descent began.
    pub descent_clock: i32,
    /// x104: ticks to the drift's next draws.
    pub drift_wait: i32,
    /// xC4 b1: lowering (or rising back).
    pub descending: bool,
    /// xC4 b2: the camera is over the sea and its draw is spent.
    pub sea_seen: bool,
}

/// Which background the camera's interest is over (grCorneria_801E08CC).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenery {
    City,
    Coast,
    Sea,
}

impl GreatFox {
    /// The GObj's translation: [`BASE`] plus the drift
    /// (grCorneria_801DD674, 801E1970).
    pub fn translation(&self) -> Vec3 {
        Vec3::new(BASE[0] + self.offset[0], BASE[1] + self.offset[1], 0.0)
    }

    /// grCorneria_801DDCF0 (0x801DDCF0): this tick's drift, which a ship's
    /// aimed laser follows (itArwinglaser_UnkMotion3_Phys).
    pub fn drift_step(&self) -> Vec3 {
        Vec3::new(
            self.offset[0] - self.previous_offset[0],
            self.offset[1] - self.previous_offset[1],
            0.0,
        )
    }

    /// grCorneria_801E1970 (0x801E1970): every wait, three draws pick the
    /// next wait and the acceleration on each axis; the velocity is
    /// clamped, the offset moves, and past a limit the acceleration turns
    /// back.
    pub fn drift(&mut self, p: &Parameters, rng: &mut HsdRng) {
        if self.drift_wait <= 0 {
            // retail 0x801E19AC: fmadds, then fctiwz.
            let span = p.drift_wait[1] - p.drift_wait[0];
            self.drift_wait = fctiwz(fma::fmadds(span, rng.randf(), p.drift_wait[0]));
            for axis in 0..2 {
                // fsubs, fmuls 2.0, fmuls x8.
                let swing = 2.0 * (rng.randf() - 0.5);
                self.acceleration[axis] = p.drift_accel * swing;
            }
        }
        self.drift_wait -= 1;
        for axis in 0..2 {
            self.velocity[axis] += self.acceleration[axis];
        }
        for axis in 0..2 {
            let top = p.drift_speed;
            if self.velocity[axis] > top {
                self.velocity[axis] = top;
            } else if self.velocity[axis] < -top {
                self.velocity[axis] = -top;
            }
        }
        self.previous_offset = self.offset;
        for axis in 0..2 {
            self.offset[axis] += self.velocity[axis];
        }
        for axis in 0..2 {
            let limit = p.drift_limit[axis];
            if self.offset[axis] > limit {
                self.acceleration[axis] = -p.drift_accel;
            } else if self.offset[axis] < -limit {
                self.acceleration[axis] = p.drift_accel;
            }
        }
    }

    /// grCorneria_801E2228 (0x801E2228): the first time the camera is over
    /// the sea after the coast, one draw in three lowers the ship; it
    /// brakes into its lowest point (`50 * scale - 250`), stays 900 ticks
    /// in all, then rises back to zero the same way. `scenery` is
    /// grCorneria_801E08CC, called only where retail calls it.
    pub fn descend(
        &mut self,
        map_scale: f32,
        mut scenery: impl FnMut() -> Scenery,
        rng: &mut HsdRng,
    ) {
        if !self.descending {
            if !self.sea_seen && scenery() == Scenery::Sea {
                if rng.randi(3) == 0 {
                    self.descending = true;
                    self.descent_clock = 0;
                    self.descent_speed = 0.0;
                }
                self.sea_seen = true;
            }
            if scenery() == Scenery::Coast {
                self.sea_seen = false;
            }
            return;
        }
        let clock = self.descent_clock;
        self.descent_clock += 1;
        let speed = self.descent_speed;
        let t = speed / DESCENT_STEP;
        // retail 0x801E2300..2308: fmuls, fmuls, fmadds.
        let braking = fma::fmadds(speed, t, DESCENT_BRAKE * t * t);
        if clock < DESCENT_TICKS {
            // retail 0x801E22FC: fmsubs.
            let floor = fma::fmsubs(50.0, map_scale, 250.0);
            if self.descent - floor > braking {
                self.descent_speed += DESCENT_STEP;
                if self.descent_speed > DESCENT_TOP_SPEED {
                    self.descent_speed = DESCENT_TOP_SPEED;
                }
            } else {
                self.descent_speed -= DESCENT_STEP;
                if self.descent_speed < DESCENT_STEP {
                    self.descent_speed = DESCENT_STEP;
                }
            }
            self.descent -= self.descent_speed;
            // retail 0x801E237C / 2394: fmsubs again.
            if self.descent < fma::fmsubs(50.0, map_scale, 250.0) {
                self.descent = fma::fmsubs(50.0, map_scale, 250.0);
            }
        } else {
            if -self.descent > braking {
                self.descent_speed = speed + DESCENT_STEP;
                if self.descent_speed > DESCENT_TOP_SPEED {
                    self.descent_speed = DESCENT_TOP_SPEED;
                }
            } else {
                self.descent_speed = speed - DESCENT_STEP;
                if self.descent_speed < DESCENT_STEP {
                    self.descent_speed = DESCENT_STEP;
                }
            }
            self.descent += self.descent_speed;
            if self.descent > 0.0 {
                self.descent = 0.0;
                self.descending = false;
            }
        }
    }

    /// grCorneria_801E2FCC (0x801E2FCC): the lowest eye height, 5 above the
    /// sea (retail 0x801E3008: fmsubs).
    pub fn lowest_eye_height(&self, map_scale: f32) -> f32 {
        fma::fmsubs(map_scale, -35.0, self.descent) + 5.0
    }

    /// The zako generators' height (grCorneria_801DD9A8, retail 0x801DD9F8:
    /// fmsubs): the sea's surface.
    pub fn sea_level(&self, map_scale: f32) -> f32 {
        fma::fmsubs(-35.0, map_scale, self.descent)
    }
}

/// The cannon's phase (`x108`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CannonPhase {
    /// 0: resting out `x110`.
    #[default]
    Resting,
    /// 1: charging for `x10C` ticks.
    Charging,
    /// 2: aimed, waiting `x10C` ticks or for a fighter at its height.
    Aiming,
    /// 3: firing `x118` shots, ten ticks apart.
    Firing,
    /// 4: destroyed this tick.
    Breaking,
    /// 5: gone.
    Broken,
}

/// The cannon (grCorneria_801E1348): `u.corneria` x108..x11A and the beam
/// damage x114 (fn_801E12D4).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cannon {
    pub phase: CannonPhase,
    /// x10C: the charge or the aim's countdown.
    pub countdown: i32,
    /// x110: the rest, or the ticks to the next shot.
    pub wait: i32,
    /// x114: the beam damage taken.
    pub damage: f32,
    /// x118: the shots left (an s8).
    pub shots: i8,
    /// x119: the laser's type (0 the big shot, 1 a volley's).
    pub laser: i8,
    /// x11A: which cannon fires next (nonzero the left, bone 2).
    pub side: u8,
}

/// The Great Fox's cannon bones (Ground_801C3FA4(gobj, 2) and 3).
pub const LEFT_CANNON_BONE: usize = 2;
pub const RIGHT_CANNON_BONE: usize = 3;
/// The intact and broken cannon models (bones 6 and 5).
pub const INTACT_BONE: usize = 6;
pub const BROKEN_BONE: usize = 5;
/// The cannon's collision joint, off once it breaks.
pub const CANNON_JOINT: i32 = 4;
/// The common-bank effects on the cannons: charging (0xBC), a shot's muzzle
/// flash (0xC0) and the break (0xE3).
pub const CHARGE_EFFECT: u32 = 0xBC;
pub const MUZZLE_EFFECT: u32 = 0xC0;
pub const BREAK_EFFECT: u32 = 0xE3;
/// The material items' states (ALDYakuAll): 2 while charging, 0 after.
pub const CHARGING_STATE: u16 = 2;
pub const RESTING_STATE: u16 = 0;

/// What one step of the cannon asks of the scene, in retail order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CannonStep {
    Nothing,
    /// 0 -> 1: the charge effects on both cannons, the items to state 2,
    /// their hurt capsules off.
    Charge,
    /// 1 -> 2: the cannons' generators stop (hsd_8039D580), the items to
    /// state 0.
    Aim,
    /// 2 -> 3: the items' hurt capsules on again.
    Open,
    /// A shot: the muzzle flash on `bone`, then it_802EAF34 at `offset`
    /// (in the GObj's frame) with laser type `laser`.
    Fire { bone: usize, offset: Vec3, laser: i32 },
    /// 4 -> 5: the break effects, a large quake at `quake_at`, the broken
    /// model shown and joint 4 off.
    Break,
}

/// grCn_803E1F70[1]: the hull point a shot leaves from, before its offsets
/// (`20 + x`, `2 + y`, `+-9`).
pub const MUZZLE_POINT: Vec3 = Vec3::new(-200.81, -20.1, 0.0);

impl Cannon {
    /// grCorneria_801E1348 (0x801E1348). `fighter_at_height` is phase 2's
    /// scan of the fighters (|cannon y - fighter y| <= 10 for any).
    pub fn step(
        &mut self,
        p: &Parameters,
        rng: &mut HsdRng,
        fighter_at_height: impl FnOnce() -> bool,
    ) -> CannonStep {
        match self.phase {
            CannonPhase::Resting => {
                if self.wait != 0 {
                    self.wait -= 1;
                    return CannonStep::Nothing;
                }
                if rng.randf() <= p.cannon_burst_chance {
                    self.laser = 0;
                    self.shots = 1;
                    self.countdown = fctiwz(p.cannon_burst_charge);
                } else {
                    let low = fctiwz(p.cannon_volley_shots[0]);
                    let high = fctiwz(p.cannon_volley_shots[1]);
                    self.laser = 1;
                    self.shots = crate::rand::range(rng, [low, high]) as i8;
                    self.countdown = fctiwz(p.cannon_volley_charge);
                }
                self.phase = CannonPhase::Charging;
                CannonStep::Charge
            }
            CannonPhase::Charging => {
                if self.countdown != 0 {
                    self.countdown -= 1;
                    return CannonStep::Nothing;
                }
                self.countdown = fctiwz(p.cannon_aim);
                self.phase = CannonPhase::Aiming;
                CannonStep::Aim
            }
            CannonPhase::Aiming => {
                if fighter_at_height() {
                    self.countdown = 0;
                }
                if self.countdown == 0 {
                    self.phase = CannonPhase::Firing;
                    return CannonStep::Open;
                }
                self.countdown -= 1;
                CannonStep::Nothing
            }
            CannonPhase::Firing => {
                if self.shots != 0 {
                    if self.wait != 0 {
                        self.wait -= 1;
                        return CannonStep::Nothing;
                    }
                    let x = 20.0 + MUZZLE_POINT.x;
                    let y = 2.0 + MUZZLE_POINT.y;
                    let (bone, z) = if self.side != 0 {
                        (LEFT_CANNON_BONE, 9.0)
                    } else {
                        (RIGHT_CANNON_BONE, -9.0)
                    };
                    self.side ^= 1;
                    self.wait = 10;
                    self.shots -= 1;
                    return CannonStep::Fire {
                        bone,
                        offset: Vec3::new(x, y, z),
                        laser: i32::from(self.laser),
                    };
                }
                // retail 0x801E1748..17A4: x1C + (float) HSD_Randi(x18).
                let range = fctiwz(p.cannon_rest_range);
                let drawn = if range != 0 { rng.randi(range) } else { 0 };
                self.wait = fctiwz(p.cannon_rest + drawn as f32);
                self.countdown = 0;
                self.phase = CannonPhase::Resting;
                CannonStep::Nothing
            }
            CannonPhase::Breaking => {
                self.phase = CannonPhase::Broken;
                CannonStep::Break
            }
            CannonPhase::Broken => CannonStep::Nothing,
        }
    }

    /// fn_801E12D4 (0x801E12D4), a cannon's material item hit for `damage`:
    /// past `x38` both cannons stop taking hits and the next step breaks
    /// them. Returns whether they broke.
    pub fn take_damage(&mut self, p: &Parameters, damage: f32) -> bool {
        self.damage += damage;
        if self.damage >= p.cannon_health {
            self.phase = CannonPhase::Breaking;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> Parameters {
        crate::corneria::test_parameters()
    }

    #[test]
    fn the_drift_stays_near_its_limits() {
        let p = parameters();
        let mut ship = GreatFox::default();
        let mut rng = HsdRng::new(7);
        for _ in 0..100_000 {
            ship.drift(&p, &mut rng);
            assert!(ship.offset[0].abs() < 25.0 && ship.offset[1].abs() < 25.0);
        }
    }

    #[test]
    fn the_descent_reaches_its_floor_and_returns() {
        let mut ship = GreatFox::default();
        let mut rng = HsdRng::new(1);
        // Draw until the descent starts.
        while !ship.descending {
            ship.sea_seen = false;
            ship.descend(0.54, || Scenery::Sea, &mut rng);
        }
        let floor = fma::fmsubs(50.0, 0.54, 250.0);
        let mut lowest: f32 = 0.0;
        let mut ticks = 0;
        while ship.descending {
            ship.descend(0.54, || Scenery::Sea, &mut rng);
            lowest = gekko_math::cmp::min(lowest, ship.descent);
            ticks += 1;
        }
        assert_eq!(lowest, floor);
        assert_eq!(ship.descent, 0.0);
        assert!(ticks > DESCENT_TICKS as usize);
    }

    #[test]
    fn a_volley_fires_its_shots_ten_ticks_apart_from_alternate_cannons() {
        let p = parameters();
        let mut cannon = Cannon {
            phase: CannonPhase::Firing,
            shots: 3,
            laser: 1,
            side: 1,
            ..Cannon::default()
        };
        let mut rng = HsdRng::new(3);
        let mut shots = Vec::new();
        for tick in 0..24 {
            if let CannonStep::Fire { bone, .. } = cannon.step(&p, &mut rng, || false) {
                shots.push((tick, bone));
            }
        }
        assert_eq!(
            shots,
            [(0, LEFT_CANNON_BONE), (11, RIGHT_CANNON_BONE), (22, LEFT_CANNON_BONE)]
        );
        assert_eq!(cannon.phase, CannonPhase::Resting);
        assert!((1200..2400).contains(&cannon.wait), "{cannon:?}");
    }
}
