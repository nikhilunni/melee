//! Shared damage and hitlag arithmetic; state transitions remain with owners.
use crate::hurtbox::HurtHeight;
use gekko_math::{fma::fmadds, msl::fctiwz};
use melee_types::combat::HitboxDescriptor;
pub struct ReceivedHit {
    pub descriptor: HitboxDescriptor,
    pub height: HurtHeight,
    pub facing: f32,
    pub knockback: f32,
    pub facing_override: Option<f32>,
}
pub struct KnockbackParameters {
    pub weight_scale: f32,
    pub weight_decay: f32,
    pub fixed_percent: f32,
    pub percent_scale: f32,
    pub damage_scale: f32,
    pub growth_scale: f32,
    pub base: f32,
    pub maximum: f32,
}
impl KnockbackParameters {
    /// ftColl_80079AB0 (80079AB0), ordinary Vs 1.0 attack/defense/stage ratios.
    pub fn knockback(&self, hit: &HitboxDescriptor, percent: f32, weight: f32) -> f32 {
        let w = weight * self.weight_scale;
        let factor = self.weight_decay - (w * self.weight_decay) / (1.0 + w);
        let (p, d) = if hit.weight_knockback != 0 {
            (self.fixed_percent, f32::from(hit.weight_knockback))
        } else {
            (
                fctiwz(percent) as f32 + hit.damage,
                fctiwz(hit.damage) as f32,
            )
        };
        // retail 80079C34 (normal) / 80079B48 (fixed weight): fmadds.
        let inner = fmadds(self.percent_scale, p, self.damage_scale * (d * p));
        // retail 80079C40/C44 (fixed: 9B50/9B54).
        let scaled = fmadds(self.growth_scale, factor * inner, self.base);
        let result = fmadds(
            0.01 * f32::from(hit.growth),
            scaled,
            f32::from(hit.base_knockback),
        );
        if result >= self.maximum {
            self.maximum
        } else {
            result
        }
    }
}
/// ftCommon_CalcHitlag (8007DA74), 8007DAA8 fmadds then fctiwz.
pub fn hitlag(damage: i32, scale: f32, base: f32, maximum: f32) -> f32 {
    (fctiwz(fmadds(damage as f32, scale, base)) as f32).min(maximum)
}
/// Fighter_8006A1BC: report expiry so each owner applies its reaction flags.
pub fn tick_hitlag(remaining: &mut f32) -> bool {
    if *remaining > 0.0 {
        *remaining -= 1.0;
        if *remaining <= 0.0 {
            *remaining = 0.0;
            return true;
        }
    }
    false
}
