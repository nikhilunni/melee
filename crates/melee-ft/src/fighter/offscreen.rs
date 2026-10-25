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

    /// Slippi's netplay code `Online/Core/BrawlOffscreenDamage.asm`
    /// (slippi-ssbm-asm 0e90140; logic ported, not copied; GPL-3) replaces
    /// the magnifier's answer at 0x8006A880 (`bl ifMagnify_802FC998`) with
    /// one no camera or display pass enters: a fighter is off screen while
    /// its position is outside the stage's camera limits
    /// (Stage_GetCamBounds*Offset, 0x80224A54..0x80224A98), unless it has
    /// vanished (x221F_b1) or is in DeadUpStar or DeadUpFall. The four
    /// tests are `fcmpo` with `blt` / `bgt`, in the order left, right, top,
    /// bottom; a NaN position is inside.
    pub fn outside_camera_limits(&self, limits: &melee_cm::StageCamera) -> bool {
        use melee_types::CommonMotionState as S;
        if self.vanished() || matches!(self.motion_state.id, S::DeadUpStar | S::DeadUpFall) {
            return false;
        }
        let position = self.physics.position;
        position.x < limits.left()
            || position.x > limits.right()
            || position.y > limits.top()
            || position.y < limits.bottom()
    }

    /// x221F_b1 (+221F mask 40): set after the motion change by the match
    /// entry (ft_0C31.c:46), Sleep (ftcolanim.c:46), DeadDown,
    /// DeadLeft, DeadRight and DeadUp at their entry (ft_0D31.c:219, 325,
    /// 424, 523) and the star and screen KO when the fighter vanishes
    /// (ft_0D31.c:681, 865); cleared by every motion change (fighter.c:1066).
    fn vanished(&self) -> bool {
        use super::life::{LifeState, ScreenKoPhase};
        use melee_types::CommonMotionState as S;
        match &self.state_data {
            // The wait for the respawn is Sleep (ftCo_800BFD9C).
            MotionData::Life(LifeState::Dead { .. } | LifeState::AwaitingRespawn) => true,
            MotionData::Life(LifeState::ScreenKo(ko)) => ko.phase == ScreenKoPhase::Vanished,
            _ => matches!(self.motion_state.id, S::Entry | S::Sleep),
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
        // Player_SetHPByIndex then ftCo_800C8C84 (fighter.c:2663-2667):
        // a Stamina fighter whose hit points are gone.
        if self.stamina_spent() {
            unimplemented!("ftCo_800C8C84 (ft_0C8C.c:29): a Stamina fighter's KO");
        }
    }

    /// ftCo_800C8C84's test (x18F0, the healing timer, is never set in
    /// the ported modes): a Stamina fighter not yet out whose
    /// Player_GetRemainingHPByIndex (hit points less the truncated percent,
    /// floored at 0) is 0.
    pub fn stamina_spent(&self) -> bool {
        self.player.stamina.is_some_and(|hp| {
            let spent = gekko_math::msl::fctiwz(self.physics.percent) as i16;
            !self.stamina_out && i32::from(hp) - i32::from(spent) <= 0
        })
    }

    /// ftColl_GetDamageCount (ftcoll.c:2363): the damage count the knockback
    /// formula reads: the percent, or a Stamina fighter's PlCo constant.
    pub(crate) fn knockback_percent(&self, damage: &super::damage::DamageParameters) -> f32 {
        match (self.player.stamina, self.stamina_out) {
            (None, _) => self.physics.percent,
            (Some(_), false) => damage.stamina_count as f32,
            (Some(_), true) => damage.stamina_out_count as f32,
        }
    }
}

/// The damage meter's ceiling.
const MAX_PERCENT: f32 = 999.0;
