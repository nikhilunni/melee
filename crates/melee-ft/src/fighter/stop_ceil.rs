//! ftCo_StopCeil: ceiling impact, fixed velocity and unlocked-ECB collision.
use super::state::{AnimationPhase, CollisionPhase, PhysicsPhase};
use super::{
    assets::{FighterAssets, Result},
    Fighter,
};
use crate::collision::{air, ecb::EcbPose};
use melee_mp::CollMap;
use melee_types::CommonMotionState as S;

impl Fighter {
    /// ftCo_8009EFA4 (8009EFA4..F044), called only after CeilingHug.
    pub(super) fn enter_stop_ceil(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<()> {
        // Full throw_flags word reset before motion entry and its initial script.
        self.commands.throw_accessory = false;
        self.commands.throw_reverse = false;
        self.commands.grab_release = false;
        self.commands.rapid_jab_loop_end = false;
        let top = self.collision.data.ecb.top.y;
        // ftKb_SpecialN_800F1F1C is a Kirby-only effect; neither scoped kind calls efAsync.
        self.change_motion_state(S::StopCeil.into(), assets)?;
        let old_y = self.physics.position.y;
        let trans_y = self
            .animation
            .root_motion
            .as_ref()
            .expect("fighter TransN owner")
            .primary_history
            .position
            .y;
        // 8009F014 / 8009F018: two ordered fadds, no multiply-add.
        self.physics.position.y = trans_y + (old_y + top);
        // ft_80082D40 immediately reruns collision with ECB0x12 unlocked.
        // It does not perform another procMap unlock/decrement.
        let core = &mut self.core;
        let cd = &mut core.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = core.physics.position;
        let pose = EcbPose::read(&mut core.skeleton, core.animation.root, cd);
        let landed = map.air_collide_ecb18(cd, Some(&|i| pose.position(i)));
        core.physics.position = cd.cur_pos;
        if landed {
            self.stop_ceil_land(assets)?;
        }
        // These writes happen after the nested landing callback, even if it changed state.
        self.physics.self_velocity.z = 0.0;
        self.physics.self_velocity.y = 0.0;
        self.core
            .skeleton
            .set_translate(self.core.animation.root, &self.core.physics.position);
        Ok(())
    }

    /// ft_80082B1C (80082B1C) for a character row's landing: a slow
    /// descent lands into Wait, a faster one into Landing.
    pub fn land_from_air(&mut self, assets: &FighterAssets) -> Result<()> {
        self.stop_ceil_land(assets)
    }

    /// ft_80082B1C, also the landing arm of ft_80082D40.
    fn stop_ceil_land(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.physics.self_velocity.y > assets.soft_landing_speed {
            self.land();
            self.change_motion_state(S::Wait.into(), assets)
        } else {
            self.enter_landing(assets)
        }
    }
}

pub fn animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    // ftCheckThrowB3 consumes this flag, even when the animation also ended.
    let end = std::mem::take(&mut fighter.commands.grab_release);
    if end {
        fighter.commands.rapid_jab_loop_end = false;
    }
    if end || !fighter.animation.frames_remaining(&fighter.skeleton) {
        fighter.change_motion_state(S::Fall.into(), phase.assets)?;
    }
    Ok(None)
}

/// ftCo_StopCeil_Coll -> ft_80083464: landing, walljump, ledge in that order.
pub fn collision(fighter: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    let assets = phase.assets.expect("StopCeil collision assets");
    let stick_y = fighter.core.input.current.stick.y;
    let drop_threshold = assets.input.platform_drop_threshold;
    let mut accept_floor = air::platform_floor_filter(stick_y, drop_threshold);
    let core = &mut fighter.core;
    air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    let cd = &mut core.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = core.physics.position;
    let can_grab = core.status.ledge_cooldown == 0;
    if can_grab {
        melee_mp::set_facing_dir(cd, if core.physics.facing < 0.0 { -1 } else { 1 });
    }
    let pose = EcbPose::read(&mut core.skeleton, core.animation.root, cd);
    let position = |i| pose.position(i);
    let landed = if can_grab {
        phase.map.air_collide_platform_pass_ledge_ecb18(
            cd,
            Some(&mut accept_floor),
            Some(&position),
        )
    } else {
        phase
            .map
            .air_collide_platform_pass_ecb18(cd, Some(&mut accept_floor), Some(&position))
    };
    core.physics.position = cd.cur_pos;
    if landed {
        fighter.stop_ceil_land(assets)?;
    } else if !fighter.try_wall_jump(assets, phase.map)? {
        fighter.try_grab_ledge(assets, phase.map)?;
    }
    fighter
        .core
        .skeleton
        .set_translate(fighter.core.animation.root, &fighter.core.physics.position);
    Ok(())
}

/// StopCeil's callback is empty, but Fighter_procUpdate still integrates retained
/// horizontal velocity, knockback and environmental movement afterward.
pub fn physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.free_flight_physics(phase.assets, phase.wind);
}
