//! The Screw Attack's fighter side (ftCo_ItemScrew.c 800D2D50..800D2F78,
//! ftCo_DamageScrew.c 800D2FA4..800D3104): holding it, a jump becomes
//! ItemScrew or ItemScrewAir; its hits (HitElement_Scball) throw the
//! victim straight up in DamageScrew.

use super::assets::{FighterAssets, Result};
use super::{Fighter, MotionData};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, GroundOrAir, HitElement, ItemKind};

/// The motion groups (x2071_b0_3) a Screw Attack hit leaves to the
/// ordinary reaction (ftCo_800D2FA4).
// TODO(meaning): what groups 11 and 12 are beyond their rows' flag words.
const UNSCREWED_MOTION_GROUPS: [u32; 2] = [11, 12];

impl Fighter {
    /// ft_800D2D0C (800D2D0C): the held item is a Screw Attack.
    pub fn holding_screw_attack(&self) -> bool {
        self.core
            .held_item
            .is_some_and(|held| held.kind == ItemKind::ScBall)
    }

    /// ft_800D2E7C (800D2E7C): the aerial jump with a Screw Attack, from
    /// ftCo_JumpAerial_CheckInput. The impulse is scaled by PlCo +800
    /// (fmuls each), the fighter leaves the ground anew (jumps used 1) and
    /// ftCo_800CBAC4 enters ItemScrewAir counting the jump.
    pub(super) fn enter_screw_air_jump(&mut self, assets: &FighterAssets) -> Result<()> {
        let attrs = &self.core.attributes.jumping;
        // ftCo_JumpAerial.c:95-97: separate fmuls.
        let impulse = Vec3::new(
            self.core.input.current.stick.x * attrs.air_jump_h_multiplier,
            attrs.jump_v_initial_velocity * attrs.air_jump_v_multiplier,
            0.0,
        );
        let multiplier = assets.screw_jump_multiplier;
        let velocity = Vec3::new(impulse.x * multiplier, impulse.y * multiplier, 0.0);
        // ftCo_ItemScrewAir writes no mv field: the landing's drop timer
        // (mv+4) is the predecessor's.
        let retained_drop_timer = self.retained_drop_timer();
        self.leave_ground();
        self.change_motion_state(S::ItemScrewAir.into(), assets)?;
        // ftCo_800CBAC4: x2221_b7, the velocity, the tilt age, the jump.
        self.core.parasol.without_stick = true;
        self.core.physics.self_velocity = velocity;
        self.core.input.vertical.tilt = 0xFE;
        self.core.physics.jumps_used += 1;
        self.core.state_data = MotionData::Inherited(retained_drop_timer);
        Ok(())
    }

    /// ftCo_800D2FA4 (800D2FA4): a Screw Attack's hit, outside motion
    /// groups 11 and 12, takes DamageScrew instead of the launch.
    pub(super) fn screwed(&self, element: HitElement) -> bool {
        if element != HitElement::ScrewAttack {
            return false;
        }
        let flags = self.motion_flags().unwrap_or_else(|| {
            unimplemented!(
                "ftCo_800D2FA4: x2071_b0_3 of motion {} is not carried",
                self.core.motion_state.action.0
            )
        });
        !UNSCREWED_MOTION_GROUPS.contains(&((flags >> 20) & 0xF))
    }

    /// ftCo_800D3004 (800D3004), after Fighter_UnkTakeDamage_8006CC30: the
    /// action stops, the fighter leaves the ground and rises at its
    /// screw_attack_launch_velocity in DamageScrew or DamageScrewAir.
    pub(super) fn enter_damage_screw(
        &mut self,
        percent: f32,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.core.add_hit_percent(percent);
        self.interrupt_actions(assets);
        // ftCo_8009750C: a heavy item would be dropped.
        if self.core.held_item.as_ref().is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_8009750C (ftCo_DamageScrew.c:41): screwed with a heavy item");
        }
        // ftCo_800DD168: a grab pair would be released.
        if self.core.combat.grab.is_some() {
            unimplemented!("ftCo_800DD168 (ftCo_DamageScrew.c:42): screwed in a grab pair");
        }
        self.core.commands.variables[0] = 0;
        // DamageScrew writes no mv field.
        let retained = self.inherited_scratch_word();
        let state = if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.leave_ground();
            S::DamageScrew
        } else {
            S::DamageScrewAir
        };
        self.change_motion_state(state.into(), assets)?;
        self.core.state_data = retained.map_or(MotionData::None, MotionData::Inherited);
        self.core.physics.self_velocity =
            Vec3::new(0.0, self.core.attributes.items.screw_attack_launch_velocity, 0.0);
        Ok(())
    }
}

impl Fighter {
    /// ftCo_ItemScrewAir_Anim (800D2EF0): FallAerial at the end.
    pub fn screw_air_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::FallAerial.into(), assets)?;
        }
        Ok(())
    }

    /// ftCo_DamageScrew_Anim (800D30A0): the ordinary special fall at the
    /// end (ftCo_800968C8).
    pub fn damage_screw_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.enter_ordinary_special_fall(assets)?;
        }
        Ok(())
    }

    /// ft_80083B68 (80083B68), DamageScrew's collision until cmd_vars[0]:
    /// airborne, never landing.
    pub fn damage_screw_collision(&mut self, map: &mut melee_mp::CollMap) {
        self.core.stay_airborne(map, false);
    }
}
