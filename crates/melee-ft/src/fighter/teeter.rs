//! Ottotto / OttottoWait (ftCo_Ottotto.c): teetering at a floor edge.
use super::{
    assets::{FighterAssets, Result},
    Fighter,
};
use crate::input::{WaitContext, WaitTransition};
use hsd_types::Vec3;
use melee_types::CommonMotionState as S;

/// ftCo_SM_Ottotto / ftCo_SM_OttottoWait: the motions these rows play.
pub const MOTIONS: &[u32] = &[210, 211, 215];

/// PlCo teeter parameters.
#[derive(Clone, Copy, Debug)]
pub struct TeeterParameters {
    /// +474: `teeter_walk_threshold`, the extra stick test for walking off the edge.
    pub walk_threshold: f32,
    /// +478 / +47C: distance from the floor endpoint at which teetering ends.
    pub edge_distance: f32,
    pub edge_margin: f32,
    /// ft_data->x4C_sfx->x18: the OttottoWait sound.
    pub sound: u32,
}

impl Fighter {
    /// ftCo_8009A410: enter Ottotto (245) from the Wait ground test; velocities cleared.
    pub(super) fn enter_teeter(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state(S::Ottotto.into(), assets)?;
        self.core.physics.self_velocity = Vec3::new(0.0, 0.0, 0.0);
        self.core.physics.ground_velocity = 0.0;
        Ok(())
    }

    /// ftCo_Ottotto_Anim: the animation ends into OttottoWait (ftCo_8009A6B8).
    pub(super) fn teeter_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        self.step_animation(assets);
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::OttottoWait.into(), assets)?;
            // ft_80088770 / ft_800887CC stop the fighter's voice/effect channels
            // (x2144/x2148 = -1; lbAudioAx_80023870 0x83D61): playback state only.
            // ft_80088328(fp, x4C_sfx->x18, 127, 64): the teeter sound.
            self.core
                .commands
                .footstep_sounds
                .push(super::commands::FootstepSound {
                    channel: super::commands::SoundChannel::Ordinary,
                    id: assets.teeter.sound,
                    volume: 127,
                    pan: 64,
                });
        }
        Ok(())
    }

    /// ftCo_Ottotto_IASA (also OttottoWait_IASA): Wait's list without the
    /// spot dodge and Fox taunt checks, and Walk gated by the teeter threshold.
    pub(super) fn teeter_input(&mut self, assets: &FighterAssets) -> Result<()> {
        let context = WaitContext {
            facing: self.core.physics.facing,
            specials_available: self.core.capabilities.specials,
            shield_health: self.core.status.shield_health,
            ..WaitContext::default()
        };
        let transition = crate::input::iasa_with_predicates(
            crate::input::OTTOTTO_PREDICATES,
            &self.core.input,
            &assets.input,
            &context,
        );
        if transition != WaitTransition::None {
            return self.apply_ground_transition(assets, transition);
        }
        // ftCo_Walk_CheckInput_Ottotto: teeter threshold, then the ordinary walk test.
        let toward = self.core.input.current.stick.x * self.core.physics.facing;
        if toward >= assets.teeter.walk_threshold
            && toward >= assets.input.thresholds.walk_stick_threshold
        {
            return self.enter_walk(assets, 0.0);
        }
        Ok(())
    }

    /// ftCo_Ottotto_Coll / ftCo_OttottoWait_Coll: stop-at-edge ground test, else
    /// Fall; standing farther than +478 + +47C from the endpoint returns to Wait.
    pub(super) fn teeter_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        let result = crate::collision::ground::map_stop_at_edge(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
            self.core.input.current.stick.x,
        );
        match result {
            crate::collision::ground::WaitGroundResult::Supported => {
                let floor = self.core.collision.data.floor.index;
                let endpoint = if self.core.physics.facing > 0.0 {
                    map.floor_get_right(floor)
                } else {
                    map.floor_get_left(floor)
                };
                let distance = gekko_math::msl::fabsf(self.core.physics.position.x - endpoint.x);
                if distance > assets.teeter.edge_distance + assets.teeter.edge_margin {
                    // ft_8008A2BC
                    self.change_motion_state(S::Wait.into(), assets)?;
                }
                Ok(())
            }
            _ => {
                self.leave_ground();
                self.change_motion_state(S::Fall.into(), assets)
            }
        }
    }
}

impl Fighter {
    /// ftCo_8009F39C: backward floor departure during damage.
    pub(super) fn enter_missed_footing(&mut self, assets: &FighterAssets) -> Result<()> {
        self.physics.knockback_velocity.y = 0.0;
        self.change_motion_state(S::MissFoot.into(), assets)?;
        let maximum = self.attributes.air.air_drift_max;
        self.physics.self_velocity.x = self.physics.self_velocity.x.clamp(-maximum, maximum);
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            self.leave_ground();
        }
        Ok(())
    }
}
/// ftCo_MissFoot_Anim -> ftCo_80090780: finish into controllable tumble.
pub(super) fn missed_footing_animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    if !fighter.animation.frames_remaining(&fighter.skeleton) {
        fighter.change_motion_state(S::DamageFall.into(), phase.assets)?;
        let maximum = fighter.attributes.air.air_drift_max;
        fighter.physics.self_velocity.x = fighter.physics.self_velocity.x.clamp(-maximum, maximum);
        fighter.state_data = super::MotionData::Damage(super::damage::DamageState {
            hitstun: 0.0,
            jump_buffer: 0.0,
            trail_timer: 0,
            influence: phase.assets.damage.influence,
        });
    }
    Ok(None)
}
