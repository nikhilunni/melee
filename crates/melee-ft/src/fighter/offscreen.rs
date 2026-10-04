//! Off-screen damage: while the magnifier bubble shows a fighter, it takes a
//! point of damage every PlCo +0x7AC ticks below PlCo +0x7B0 percent
//! (Fighter_8006A360, fighter.c:1595-1610).
use super::assets::Result;
use super::*;
use hsd_archive::Archive;

/// PlCo +0x7AC..+0x7B4, stored as integers.
#[derive(Clone, Copy, Debug)]
pub struct MagnifierDamage {
    /// +0x7AC: magnified ticks per point of damage.
    pub ticks_per_hit: i32,
    /// +0x7B0: no damage at or above this percent.
    pub percent_limit: i32,
    /// +0x7B4: the damage per hit.
    pub damage: i32,
}

impl MagnifierDamage {
    pub fn read(common: &Archive, common_data: u32) -> Result<Self> {
        let r = common.reader();
        Ok(Self {
            ticks_per_hit: r.s32(common_data + 0x7AC)?,
            percent_limit: r.s32(common_data + 0x7B0)?,
            damage: r.s32(common_data + 0x7B4)?,
        })
    }
}

impl FighterCore {
    /// fighter.c:1595-1610. A player's second fighter (x221F_b4, Nana) takes
    /// none and counts nothing: retail 0x8006A830 tests the bit before the
    /// camera's zoom.
    pub(crate) fn apply_magnifier_damage(&mut self, parameters: &MagnifierDamage) {
        if self.player.secondary || !self.offscreen.camera_unzoomed {
            return;
        }
        if self.physics.percent >= parameters.percent_limit as f32 {
            return;
        }
        let offscreen = &mut self.offscreen;
        if offscreen.magnified && offscreen.damage_enabled {
            offscreen.magnified_ticks += 1;
        } else {
            offscreen.magnified_ticks = 0;
        }
        if offscreen.magnified_ticks >= parameters.ticks_per_hit {
            self.take_percent_damage(parameters.damage as f32);
            self.offscreen.magnified_ticks = 0;
        }
    }

    /// The subaction self-damage (ftAction_80072BF4) the last command step
    /// ran, in script order.
    pub(super) fn apply_script_damage(&mut self) {
        for amount in self.commands.self_damage.take_all() {
            self.take_percent_damage(amount);
        }
    }

    /// Fighter_TakeDamage_8006CC7C (0x8006CC7C): add percent, capped at 999.
    /// Metal and stamina bookkeeping (x2226_b4, metal_health, x2034/x2038,
    /// ftCo_800C8C84) belong to modes the port does not reach.
    pub(crate) fn take_percent_damage(&mut self, amount: f32) {
        self.physics.percent += amount;
        if self.physics.percent > MAX_PERCENT {
            self.physics.percent = MAX_PERCENT;
        }
    }
}

/// The damage meter's ceiling.
const MAX_PERCENT: f32 = 999.0;
