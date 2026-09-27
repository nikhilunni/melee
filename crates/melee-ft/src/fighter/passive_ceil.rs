//! ftCo_PassiveCeil: a tech against a ceiling (the underside of a stage).
use super::state::AnimationPhase;
use super::{
    assets::{FighterAssets, Result},
    Fighter,
};
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::{mp::collide, CommonMotionState as S};

impl Fighter {
    /// ftCo_800C23A0 (800C23A0): touching a ceiling inside the tech window
    /// techs off it.
    pub(super) fn try_ceiling_tech(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<bool> {
        let env = self.core.collision.data.env_flags as u32;
        if env & collide::CEILING_HUG == 0 || !self.core.tech_window_open(assets) {
            return Ok(false);
        }
        self.enter_passive_ceil(assets, map)?;
        Ok(true)
    }

    /// ftCo_800C23FC (800C23FC): stop, spark at the ECB's top, then
    /// PassiveCeil placed from TransN below the contact, followed by
    /// ft_80081DD4's pass (ftCo_80090574), whose result is unused.
    fn enter_passive_ceil(&mut self, assets: &FighterAssets, map: &mut CollMap) -> Result<()> {
        self.core.clear_movement();
        // throw_flags = 0.
        self.commands.throw_accessory = false;
        self.commands.throw_reverse = false;
        self.commands.grab_release = false;
        self.commands.rapid_jab_loop_end = false;
        let top = self.collision.data.ecb.top.y;
        // ftKb_SpecialN_800F1F1C is Kirby's.
        self.change_motion_state(S::PassiveCeil.into(), assets)?;
        let p = self.core.physics.position;
        let contact_y = p.y + top;
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::WallJump {
                position: Vec3::new(p.x, contact_y, p.z),
            });
        let trans_y = self
            .animation
            .root_motion
            .as_ref()
            .expect("PassiveCeil TransN")
            .primary_history
            .position
            .y;
        self.core.physics.position.y = contact_y + trans_y;
        self.damage_air_pass(assets, map);
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Action,
                id: assets.wall_jump_sound,
                volume: 127,
                pan: 64,
            });
        self.core.shield_sound(3);
        self.core
            .commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: 12,
                duration: 0,
            });
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 120,
                duration: 0,
            });
        Ok(())
    }
}

/// ftCo_PassiveCeil_Anim: throw_flags_b3 sets the drift from the stick
/// (co_attrs.passiveceil_vel_x); the animation's end falls.
pub fn animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    // ftCheckThrowB3 reads and clears the flag.
    if std::mem::take(&mut fighter.commands.grab_release) {
        fighter.commands.rapid_jab_loop_end = false;
        fighter.physics.self_velocity.x =
            fighter.input.current.stick.x * fighter.attributes.wall.passiveceil_vel_x;
    }
    if !fighter.animation.frames_remaining(&fighter.skeleton) {
        fighter.change_motion_state(S::Fall.into(), phase.assets)?;
    }
    Ok(None)
}
