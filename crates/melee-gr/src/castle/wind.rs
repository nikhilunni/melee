//! The castle's wind (grCastle_801CD37C's lb_80011A50 field, steered by
//! grCastle_801D0BBC and re-gusted by grCastle_801D0D24 once per flag).
//! The flags it blows are drawn only; their draws are what the match sees.
use gekko_math::{fma::fmadd, HsdRng};
use hsd_types::Vec3;
use melee_lb::{dynamics::ForceField, radial_force::DirectionalGust};

/// grCs_804D45E0's initial value: one degree, the field's turn per tick.
pub const TURN: f32 = 0.017453292;
/// grCastle_801D0BBC's limits: compared in double, stored in single.
const LIMIT_DOUBLE: f64 = 1.0471975430846214;
const LIMIT: f32 = core::f32::consts::FRAC_PI_3;
/// The flags (grCs_803B7EA8: map 3 bones 66, 76, 85 and 94, each with
/// dynamicsdata_flag6), each re-gusting the field once per tick.
pub const FLAG_COUNT: usize = 4;

/// grCastle_801CD37C (0x801CD37C): `lb_80011A50(&{1, 0, 0}, -1, 0.5, 0.0,
/// 1.0471976f, -10000, 10000, 10000, -10000)`.
pub fn field() -> DirectionalGust {
    DirectionalGust {
        direction: Vec3::new(1.0, 0.0, 0.0),
        frames: -1,
        strength: 0.5,
        decay: 0.0,
        phase_step: LIMIT,
        rectangle: [-10000.0, 10000.0, 10000.0, -10000.0],
    }
}

/// The two statics grCastle_801D0BBC keeps (they outlive the match).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gusts {
    /// grCs_804D45E0: the turn per tick, one degree either way.
    pub turn: f32,
    /// grCs_804D45E4: strong gusts.
    pub strong: bool,
}

impl Default for Gusts {
    /// The statics as the DOL loads them.
    fn default() -> Self {
        Self {
            turn: TURN,
            strong: true,
        }
    }
}

impl Gusts {
    /// grCastle_801D0BBC (0x801D0BBC), after lb_800115F4 advanced the
    /// field's phase: turn it a degree (clamped to +-60), flip the turn
    /// on a coin every 30 ticks, toggle strong gusts every 300, then gust.
    pub fn steer(&mut self, field: &mut ForceField, rng: &mut HsdRng) {
        self.steer_with(field, rng, Self::calm_castle);
    }

    /// The castle's calm gust: `0.05 * r` (fmul in double).
    fn calm_castle(r: f64) -> f64 {
        0.05 * r
    }

    /// [`Self::steer`] with the calm gust a stage gives (Rainbow Cruise's
    /// grRCruise_80201110 has its own).
    pub fn steer_with(&mut self, field: &mut ForceField, rng: &mut HsdRng, calm: fn(f64) -> f64) {
        let direction = &mut field.direction_or_center;
        let mut angle = melee_lb::trigf::atan2f(direction.z, direction.x) + self.turn;
        if f64::from(angle) > LIMIT_DOUBLE {
            angle = LIMIT;
        }
        if f64::from(angle) < -LIMIT_DOUBLE {
            angle = -LIMIT;
        }
        direction.x = gekko_math::msl::cosf(angle);
        direction.z = gekko_math::msl::sinf(angle);
        let phase = field.phase;
        if phase % 30 == 0 && f64::from(rng.randf()) > 0.5 {
            self.turn = -self.turn;
        }
        if phase % 300 == 0 {
            if self.strong {
                self.strong = false;
            } else if f64::from(rng.randf()) > 0.5 {
                self.strong = true;
            }
        }
        self.gust_with(field, rng, calm);
    }

    /// grCastle_801D0D24 (0x801D0D24): the field's strength, `0.3 * r +
    /// 0.2` (retail 0x801D0D4C: fmadd in double) or `0.05 * r`.
    pub fn gust(&self, field: &mut ForceField, rng: &mut HsdRng) {
        self.gust_with(field, rng, Self::calm_castle);
    }

    fn gust_with(&self, field: &mut ForceField, rng: &mut HsdRng, calm: fn(f64) -> f64) {
        let r = f64::from(rng.randf());
        field.strength = if self.strong {
            fmadd(0.3, r, 0.2) as f32
        } else {
            calm(r) as f32
        };
    }
}
