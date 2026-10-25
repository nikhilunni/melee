//! A fighter's model scale other than 1 (Player_GetModelScale: Giant and
//! Tiny Melee, the Mushrooms): ftchangeparam.c.
//!
//! PlCo's scale curves (Fighter_804D6524) say how strongly each attribute
//! follows the scale: ftCo_CalcYScaledKnockback grows a value linearly with
//! the scale, or for a fighter smaller than 1 with a curve above 1, divides
//! the shrink by the curve; a negative curve inverts the relation.
use crate::desc::attributes::FighterAttributes;
use hsd_archive::Archive;

/// Fighter_804D6524 (PlCo `ftLoadCommonData` entry 12): one curve per
/// scaled quantity, by offset.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleCurves {
    curves: [f32; CURVE_COUNT],
}

/// x0 through x98.
const CURVE_COUNT: usize = 0x9C / 4;

impl ScaleCurves {
    pub fn read(common: &Archive) -> super::assets::Result<Self> {
        let root = common
            .public("ftLoadCommonData")
            .ok_or("missing PlCo root")?;
        let table = common.link(root + 12 * 4)?.ok_or("missing scale curves")?;
        let r = common.reader();
        let mut curves = [0.0; CURVE_COUNT];
        for (index, curve) in curves.iter_mut().enumerate() {
            *curve = r.f32(table + 4 * index as u32)?;
        }
        Ok(Self { curves })
    }

    fn at(&self, offset: usize) -> f32 {
        self.curves[offset / 4]
    }

    /// x0: the knockback a scaled victim takes (ftCo_Damage.c:140).
    pub fn knockback(&self) -> f32 {
        self.at(0)
    }

    /// x4: the damage a scaled attacker deals (ftcoll.c:2996, itcoll.c:1413).
    pub fn damage(&self) -> f32 {
        self.at(4)
    }

    /// x8: the item pickup animation's rate (ftpickupitem.c:213).
    pub fn pickup_rate(&self) -> f32 {
        self.at(8)
    }

    /// xC: Walk's acceleration (ftCo_Walk.c:65).
    pub fn walk_acceleration(&self) -> f32 {
        self.at(0xC)
    }

    /// x30: gravity's curve, which ftCo_800D0EC8 also takes.
    pub fn gravity(&self) -> f32 {
        self.at(0x30)
    }

    /// ftCo_800CF6E8 (0x800CF6E8): every attribute ftCo_800D105C scales,
    /// in its order.
    pub fn scale_attributes(&self, a: &mut FighterAttributes, scale: f32) {
        if scale == 1.0 {
            return;
        }
        let s = |value: &mut f32, offset: usize| *value = scaled(*value, scale, self.at(offset));
        s(&mut a.walking.slow_walk_max, 0x10);
        s(&mut a.walking.mid_walk_point, 0x14);
        s(&mut a.walking.fast_walk_min, 0x18);
        s(&mut a.running.dash_max_velocity, 0x1C);
        s(&mut a.running.run_animation_scaling, 0x20);
        s(&mut a.jumping.jump_startup_time, 0x24);
        s(&mut a.jumping.jump_v_initial_velocity, 0x28);
        s(&mut a.jumping.hop_v_initial_velocity, 0x2C);
        s(&mut a.air.gravity, 0x30);
        s(&mut a.air.terminal_velocity, 0x34);
        s(&mut a.air.air_drift_stick_mul, 0x38);
        s(&mut a.air.aerial_drift_base, 0x3C);
        s(&mut a.air.air_drift_max, 0x40);
        s(&mut a.air.fast_fall_velocity, 0x44);
        s(&mut a.size.weight, 0x48);
        s(&mut a.shield.shield_break_initial_velocity, 0x4C);
        s(&mut a.ledge.ledge_jump_horizontal_velocity, 0x50);
        s(&mut a.ledge.ledge_jump_vertical_velocity, 0x54);
        s(&mut a.items.item_throw_velocity_multiplier, 0x58);
        s(&mut a.items.heavy_throw_velocity_multiplier, 0x5C);
        s(&mut a.yoshi_egg.size, 0x60);
        s(&mut a.kirby_throw.star_scale, 0x64);
        s(&mut a.combat.kirby_b_star_damage, 0x68);
        s(&mut a.landing.normal_landing_lag, 0x6C);
        s(&mut a.landing.landingairn_lag, 0x70);
        s(&mut a.landing.landingairf_lag, 0x74);
        s(&mut a.landing.landingairb_lag, 0x78);
        s(&mut a.landing.landingairhi_lag, 0x7C);
        s(&mut a.landing.landingairlw_lag, 0x80);
        s(&mut a.size.name_tag_height, 0x84);
        s(&mut a.items.screw_attack_launch_velocity, 0x88);
        s(&mut a.ice.damageicejump_vel_y, 0x8C);
        s(&mut a.ice.damageicejump_vel_x_mult, 0x90);
        s(&mut a.camera.zoom_offset.x, 0x94);
        s(&mut a.camera.zoom_offset.y, 0x94);
        s(&mut a.camera.zoom_offset.z, 0x94);
        s(&mut a.camera.damage_camera_y_offset, 0x98);
    }
}

impl super::FighterCore {
    /// ftCo_800D0EC8 (0x800D0EC8): the fall speed (negative) below which a
    /// landing lags, PlCo +310 scaled along gravity's curve.
    pub fn soft_landing_speed(&self, assets: &super::assets::FighterAssets) -> f32 {
        -scaled(
            assets.soft_landing_base,
            self.player.scale,
            assets.scale_curves.gravity(),
        )
    }
}

/// ftCo_CalcYScaledKnockback (0x800CF594).
pub use melee_coll::damage::y_scaled as scaled;

#[cfg(test)]
mod tests {
    use super::scaled;

    #[test]
    fn a_zero_curve_keeps_the_value_and_a_negative_one_inverts_it() {
        assert_eq!(scaled(3.0, 1.8, 0.0), 3.0);
        let grown = scaled(1.0, 1.8, 0.5);
        assert_eq!(grown, 1.4);
        assert_eq!(scaled(3.0, 1.8, -0.5), 3.0 / grown);
    }

    #[test]
    fn a_small_fighter_along_a_steep_curve_divides_its_shrink() {
        assert_eq!(scaled(2.0, 0.5, 2.0), 2.0 * 0.5 / 2.0);
        assert_eq!(scaled(2.0, 0.5, 0.5), 1.5);
    }
}
