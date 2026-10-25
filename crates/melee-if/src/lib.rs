//! RNG-relevant percent HUD state; if/ifstatus.c, without rendering.
use gekko_math::{msl::fctiwz, rng::HsdRng};
use hsd_types::Vec2;

/// IfDamageState.damage_percent before the first sample after a reset.
const UNKNOWN_PERCENT: i32 = -1;

/// IfDamageState: the four digits draw even when hundreds/tens are hidden.
#[derive(Clone)]
pub struct PercentDisplay {
    pub death_velocity: Option<[Vec2; 4]>,
    pub percent: i32,
    pub damage_from_last_attack: u8,
    pub shake_remaining: u8,
    pub offsets: [Vec2; 4],
}
impl PercentDisplay {
    pub fn new(percent: f32) -> Self {
        Self {
            death_velocity: None,
            percent: fctiwz(percent).clamp(0, 999),
            damage_from_last_attack: 0,
            shake_remaining: 0,
            offsets: [Vec2::ZERO; 4],
        }
    }
    /// ifStatus_PercentOnDeathAnimationThink (802F491C): initialize four departing digits.
    pub fn set_dead(&mut self, dead: bool, rng: &mut HsdRng) {
        if !dead {
            if self.death_velocity.take().is_some() {
                // ifStatus_802F6508 on revival: forget the percent (-1) and
                // any shake the death interrupted.
                self.percent = UNKNOWN_PERCENT;
                self.shake_remaining = 0;
                self.offsets = [Vec2::ZERO; 4];
            }
            return;
        }
        if let Some(velocities) = &mut self.death_velocity {
            for (position, velocity) in self.offsets.iter_mut().zip(velocities.iter_mut()) {
                if gekko_math::msl::fabsf(position.x) < 100.0 {
                    position.x += velocity.x;
                }
                if position.y > -100.0 {
                    position.y += velocity.y;
                    velocity.y -= 0.2028;
                }
            }
        } else {
            self.death_velocity = Some(std::array::from_fn(|i| {
                // 802F4974 fmuls then fadds; alternating signs share one toggle.
                let x = 0.6083 * rng.randf() + 0.3041;
                // retail 802F49A0 fmadds.
                let y = gekko_math::fma::fmadds(0.811, rng.randf(), 1.2165);
                Vec2::new(if i % 2 == 0 { x } else { -x }, y)
            }));
        }
    }
    /// ifStatus_802F5B48 then ifStatus_802F4EDC, both s_link 17, p_link 15.
    /// While the digits explode (`set_dead`), ifStatus_802F4EDC returns
    /// before starting or advancing a shake, so an increase that tick is lost.
    pub fn tick(&mut self, percent: f32, rng: &mut HsdRng) {
        let percent = fctiwz(percent).clamp(0, 999);
        let increased = self.percent != UNKNOWN_PERCENT && percent > self.percent;
        if increased {
            self.damage_from_last_attack = (percent - self.percent) as u8;
        }
        self.percent = percent;
        if self.death_velocity.is_some() {
            return;
        }
        if increased {
            self.shake_remaining = 10;
        }
        self.shake(rng);
    }
    /// ifStatus_802F4B84 (802F4B84); no fused instructions in retail audit.
    fn shake(&mut self, rng: &mut HsdRng) {
        match self.shake_remaining {
            0 => return,
            1 => {
                self.offsets.fill(Vec2::ZERO);
                self.shake_remaining = 0;
                return;
            }
            _ => {}
        }
        let magnitude = (0.1014 * f32::from(self.damage_from_last_attack)).clamp(0.1014, 1.5207);
        for offset in &mut self.offsets {
            // Calls at 802F4D44/4D54 (LR +1C4/+1D4); ledger sites +1C0/+1D0.
            let x = magnitude * (2.0 * (rng.randf() - 0.5));
            let y = magnitude * (2.0 * (rng.randf() - 0.5));
            offset.x = if x < 0.0 { x - 0.2028 } else { x + 0.2028 };
            offset.y = if y < 0.0 { y - 0.2028 } else { y + 0.2028 };
        }
        self.shake_remaining -= 1;
    }
}

/// ifStock_802F8298 (802F8298): five icon slots and their loss animation.
#[derive(Clone)]
pub struct StockDisplay {
    pub icon_positions: [hsd_types::Vec3; 5],
    pub animation_frames: [u8; 5],
    pub animate_losses: bool,
    /// The HUD place's root translation (x4[0]) and each icon's local
    /// translation at animation frame 0, where ifStock_802F7EFC poses
    /// them before reading a stolen stock's path.
    pub root: hsd_types::Vec3,
    pub icon_locals: [hsd_types::Vec3; 5],
    /// x204.anim[5..7] and steal[2]: stocks flying from this player's HUD
    /// to a teammate's (a team battle's stock steal).
    pub steals: [Option<StockSteal>; 2],
}

/// One stolen stock's flight (IfStockStealAnim and its anim counter).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StockSteal {
    pub start: hsd_types::Vec3,
    pub mid: hsd_types::Vec3,
    pub end: hsd_types::Vec3,
    /// x204.x3[i]: the player who takes the stock.
    pub thief: u8,
    /// x204.anim[5 + i]: 1..=10 in flight.
    pub frame: u8,
}

/// The particle generators ifStock_802F8298's efSync_Spawn calls make
/// (efAsync_80067688): 0x474 an icon's loss, 0x475 a stolen stock leaving,
/// 0x476 it arriving.
pub const STOCK_LOST_GENERATOR: u32 = 0xF7;
pub const STOCK_STEAL_START_GENERATOR: u32 = 0xFC;
pub const STOCK_STEAL_END_GENERATOR: u32 = 0xFF;

/// What one stock HUD proc did: particles in call order, and a thief whose
/// stock landed (its x204.flag set again).
pub struct StockTick {
    pub particles: [Option<(u32, hsd_types::Vec3)>; 7],
    pub landed: [Option<u8>; 2],
}

/// lbVector_8000DE38 (8000DE38) for one axis: the quadratic through
/// `start`, `mid` and `end` at `t` (clamped to 0..1), as retail fuses it.
pub fn steal_curve(start: f32, mid: f32, end: f32, t: f32) -> f32 {
    use gekko_math::fma::{fmadds, fmsubs};
    // Retail's two fcmpo tests in order (a NaN passes through).
    #[allow(clippy::manual_clamp)]
    let t = if t > 1.0 {
        1.0
    } else if t < 0.0 {
        0.0
    } else {
        t
    };
    let four_mid = 4.0 * mid;
    // 8000DE78 fmsubs, 8000DE7C fmadds, 8000DE80 fmadds, 8000DE84 fsubs.
    let a = fmadds(2.0, end, fmsubs(2.0, start, four_mid));
    let b = fmadds(-3.0, start, four_mid) - end;
    // 8000DE88/8C fmuls, 8000DE90 fmadds, 8000DE94 fadds.
    start + fmadds(t, a * t, b * t)
}

impl StockDisplay {
    /// ifStock_802F7EFC (802F7EFC) on the donor's display, `donor_stocks`
    /// before its loss: the stolen icon's flight toward `thief`'s HUD,
    /// whose display is `to`. False when both flight slots are busy (it
    /// returns 2 and the steal does not happen).
    pub fn begin_steal(&mut self, donor_stocks: u8, thief: u8, thief_stocks: u8, to: &Self) -> bool {
        use hsd_types::Vec3;
        let Some(index) = self.steals.iter().position(Option::is_none) else {
            return false;
        };
        // The icon that goes does not explode (x0[stocks + 4] = 10).
        if (1..=5).contains(&donor_stocks) {
            self.animation_frames[usize::from(donor_stocks) - 1] = 10;
        }
        let from_icon = if (1..=5).contains(&donor_stocks) {
            usize::from(donor_stocks)
        } else {
            1
        };
        let local = self.icon_locals[from_icon - 1];
        let start = Vec3::new(
            local.x + self.root.x,
            local.y + self.root.y,
            local.z + self.root.z,
        );
        let j = if thief_stocks > 0 && thief_stocks < 5 {
            f32::from(thief_stocks)
        } else {
            0.0
        };
        let first = to.icon_locals[0];
        // retail 802F8224: fmadds (2.4f * j + root.x), then fadds.
        let end = Vec3::new(
            first.x + gekko_math::fma::fmadds(2.4, j, to.root.x),
            start.y,
            first.z + to.root.z,
        );
        let mid = Vec3::new(0.5 * (end.x + start.x), 10.0 + start.y, 0.5 * (end.z + start.z));
        self.steals[index] = Some(StockSteal {
            start,
            mid,
            end,
            thief,
            frame: 1,
        });
        true
    }

    pub fn tick(&mut self, stocks: u8) -> StockTick {
        assert!(
            stocks <= 5,
            "ifStock_802F8298: numeric stock count above five"
        );
        // At most one loss effect per icon; preserve slot order without allocating.
        let mut effects = [None; 7];
        let mut landed = [None; 2];
        for (i, frame) in self.animation_frames.iter_mut().enumerate() {
            if i < usize::from(stocks) {
                *frame = if self.animate_losses { 0 } else { 10 };
            } else {
                if !self.animate_losses {
                    *frame = 10;
                }
                if *frame == 0 {
                    effects[i] = Some((STOCK_LOST_GENERATOR, self.icon_positions[i]));
                }
                if *frame < 10 {
                    *frame += 1;
                }
            }
        }
        // The stolen stocks' flights (anim[5], anim[6]).
        for (slot, steal) in self.steals.iter_mut().enumerate() {
            let Some(flight) = steal else { continue };
            if flight.frame > 10 {
                landed[slot] = Some(flight.thief);
                *steal = None;
                continue;
            }
            let t = 0.1 * f32::from(flight.frame);
            let z = steal_curve(flight.start.z, flight.mid.z, flight.end.z, t) - self.root.z;
            let at = match flight.frame {
                1 => Some((STOCK_STEAL_START_GENERATOR, flight.start)),
                10 => Some((STOCK_STEAL_END_GENERATOR, flight.end)),
                _ => None,
            };
            if let Some((generator, point)) = at {
                effects[5 + slot] = Some((generator, hsd_types::Vec3::new(point.x, point.y, z)));
            }
            flight.frame += 1;
        }
        StockTick {
            particles: effects,
            landed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_increase_shakes_all_four_digits_for_nine_ticks() {
        let mut display = PercentDisplay::new(0.0);
        let mut rng = HsdRng::new(42);
        let mut expected = HsdRng::new(42);
        display.tick(0.0, &mut rng);
        assert_eq!(rng.seed, 42);
        for _ in 0..9 {
            display.tick(4.0, &mut rng);
            for _ in 0..8 {
                expected.randf();
            }
            assert_eq!(rng.seed, expected.seed);
        }
        display.tick(4.0, &mut rng);
        assert_eq!(display.shake_remaining, 0);
        assert_eq!(display.offsets, [Vec2::ZERO; 4]);
        display.tick(3.0, &mut rng);
        assert_eq!(rng.seed, expected.seed);
        display.tick(5.0, &mut rng);
        assert_eq!(display.shake_remaining, 9);
        assert_eq!(display.damage_from_last_attack, 2);
    }
}
