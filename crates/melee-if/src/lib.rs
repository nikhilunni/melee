//! RNG-relevant percent HUD state; if/ifstatus.c, without rendering.
use gekko_math::{msl::fctiwz, rng::HsdRng};
use hsd_types::Vec2;

/// IfDamageState: the four digits draw even when hundreds/tens are hidden.
pub struct PercentDisplay {
    pub percent: i32,
    pub damage_from_last_attack: u8,
    pub shake_remaining: u8,
    pub offsets: [Vec2; 4],
}
impl PercentDisplay {
    pub fn new(percent: f32) -> Self {
        Self {
            percent: fctiwz(percent).clamp(0, 999),
            damage_from_last_attack: 0,
            shake_remaining: 0,
            offsets: [Vec2::ZERO; 4],
        }
    }
    /// ifStatus_802F5B48 then ifStatus_802F4EDC, both s_link 17, p_link 15.
    pub fn tick(&mut self, percent: f32, rng: &mut HsdRng) {
        let percent = fctiwz(percent).clamp(0, 999);
        if percent > self.percent {
            self.damage_from_last_attack = (percent - self.percent) as u8;
            self.shake_remaining = 10;
        }
        self.percent = percent;
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
