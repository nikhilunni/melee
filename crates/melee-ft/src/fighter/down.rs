//! Tumble landing and prone recovery, ftCo_DownBound.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, FtPart};

/// Shared getup animations from ftData_MotionStateList, prepared at asset load.
pub(super) const MOTIONS: &[u32] = &[186, 187, 188, 189, 194, 195, 196, 197, 199, 200];

impl Fighter {
    /// ftCo_80098400 / ftCo_800984D4: buffered AB at bounce completion,
    /// fresh AB during DownWait; an upward C-stick crossing works in both.
    fn down_attack_input(&mut self, assets: &FighterAssets, buffered: bool) -> Result<bool> {
        let input = &self.core.input;
        let pressed = if buffered {
            f32::from(input.buttons.attack) < assets.damage.down_attack_buffer
                || f32::from(input.buttons.special) < assets.damage.down_attack_buffer
        } else {
            input
                .pressed
                .intersects(crate::input::Buttons::A | crate::input::Buttons::B)
        };
        let threshold = assets.damage.down_cstick_attack_threshold;
        let cstick_attack =
            input.previous.cstick.y < threshold && input.current.cstick.y >= threshold;
        if !(pressed || cstick_attack) {
            return Ok(false);
        }
        let up = if buffered {
            S::DownBoundU
        } else {
            S::DownWaitU
        };
        let state = if self.core.motion_state.id == up {
            S::DownAttackU
        } else {
            S::DownAttackD
        };
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        Ok(true)
    }

    /// ftCo_Down_CheckInput (80098214): C-stick crossing before held main stick.
    fn down_roll_input(&mut self, assets: &FighterAssets) -> Result<bool> {
        use gekko_math::msl::fabsf;
        let input = &self.core.input;
        let threshold = assets.damage.down_roll_threshold;
        let horizontal = |stick: crate::input::Stick| {
            fabsf(stick.x) >= threshold
                && melee_lb::trigf::atan2f(stick.y, fabsf(stick.x)) < assets.input.tilt_angle
        };
        let stick =
            if fabsf(input.previous.cstick.x) < threshold && horizontal(input.current.cstick) {
                input.current.cstick.x
            } else if horizontal(input.current.stick) {
                input.current.stick.x
            } else {
                return Ok(false);
            };
        // Retail deliberately tests DownWaitU even when called by DownBound.
        let up = self.core.motion_state.id == S::DownWaitU;
        let forward = stick * self.core.physics.facing >= 0.0;
        let state = match (up, forward) {
            (true, true) => S::DownFowardU,
            (true, false) => S::DownBackU,
            (false, true) => S::DownFowardD,
            (false, false) => S::DownBackD,
        };
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        Ok(true)
    }

    /// ftCo_DownWait_IASA (8009802C): attack, roll, then stand.
    fn down_wait_input(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.down_attack_input(assets, false)? || self.down_roll_input(assets)? {
            return Ok(());
        }
        let input = &self.core.input;
        let stick = input.current.stick;
        if (stick.y >= assets.damage.down_stand_threshold
            && melee_lb::trigf::atan2f(stick.y, gekko_math::msl::fabsf(stick.x))
                >= assets.input.tilt_angle)
            || input.pressed.intersects(crate::input::Buttons::SHIELD)
        {
            let state = if self.core.motion_state.id == S::DownWaitU {
                S::DownStandU
            } else {
                S::DownStandD
            };
            self.change_motion_state(state.into(), assets)?;
        }
        Ok(())
    }
    /// ftCo_800986B0 / ftCo_80098928 (800986B0 / 80098928): digital edge window.
    pub(super) fn try_tech(&mut self, assets: &FighterAssets) -> Result<bool> {
        let timers = &self.core.input.buttons;
        if f32::from(timers.digital_shield) >= assets.damage.tech_window
            || i32::from(timers.previous_digital_shield) < assets.damage.tech_lockout
        {
            return Ok(false);
        }
        let stick = self.core.input.current.stick.x;
        let state = if gekko_math::msl::fabsf(stick) < assets.damage.tech_roll_threshold {
            S::Passive
        } else if stick * self.core.physics.facing >= 0.0 {
            S::PassiveStandF
        } else {
            S::PassiveStandB
        };
        self.land();
        self.change_motion_state(state.into(), assets)?;
        if state == S::Passive {
            self.core.project_ground_knockback(assets);
            self.commands
                .color_animations
                .push(melee_cmd::ColorAnimationRequest {
                    id: 120,
                    duration: 0,
                });
        }
        self.core
            .effects
            .push_after_graphics(melee_ef::request::EffectRequest::CaptureFlash { bone: 0 });
        Ok(true)
    }
    /// ftCo_8009794C (8009794C): choose face-up/down from the animated HipN.
    pub(super) fn enter_down_bound(&mut self, assets: &FighterAssets) -> Result<()> {
        self.land();
        let hip = self.core.animation.parts
            [usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"))]
        .joint;
        let matrix = self.core.skeleton.get_mtx(hip);
        let state = if matrix.0[1][1] > 0.0 {
            S::DownBoundU
        } else {
            S::DownBoundD
        };
        self.change_motion_state(state.into(), assets)?;
        self.core.state_data = MotionData::Down {
            wait_remaining: 0.0,
        };
        self.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        self.core.input.buttons.attack = 255;
        self.core.input.buttons.special = 255;
        let normal = self.core.collision.data.floor.normal;
        let floor_angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
        // ftCo_800978D4: direct async kind 4 has no randomized offset.
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::Graphics {
                id: 0x406,
                bone: 0,
                offset: Vec3::ZERO,
                facing: self.core.physics.facing,
                floor_angle,
            });
        // ftCo_800976A4 -> ftCo_8009F834: three draws, even for zero ranges.
        self.core
            .commands
            .graphics
            .push(melee_types::combat::GraphicsCommand {
                id: 0x407,
                bone: 0,
                common_bone: false,
                item_bone: false,
                destroy_on_state_change: false,
                parameter: 0.0,
                offset: Vec3::ZERO,
                range: Vec3::ZERO,
            });
        self.core.project_ground_knockback(assets);
        Ok(())
    }

    /// DownBound_Anim (80097DE8), DownWait_Anim (80097FD0).
    pub(super) fn down_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if matches!(self.core.motion_state.id, S::DownBoundU | S::DownBoundD) {
            if !self.core.animation.frames_remaining(&self.core.skeleton) {
                if self.down_attack_input(assets, true)? || self.down_roll_input(assets)? {
                    return Ok(());
                }
                if self.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
                    self.land();
                }
                self.change_motion_state(
                    (if self.core.motion_state.id == S::DownBoundU {
                        S::DownWaitU
                    } else {
                        S::DownWaitD
                    })
                    .into(),
                    assets,
                )?;
                self.core.state_data = MotionData::Down {
                    wait_remaining: assets.damage.down_wait_frames,
                };
                self.step_animation(assets);
                self.core.status.grab_exclusions = super::ledge::GrabExclusions(1);
            }
        } else {
            let MotionData::Down { wait_remaining } = &mut self.core.state_data else {
                panic!("down scratch");
            };
            *wait_remaining -= 1.0;
            if *wait_remaining <= 0.0 {
                let state = if self.core.motion_state.id == S::DownWaitU {
                    S::DownStandU
                } else {
                    S::DownStandD
                };
                self.change_motion_state(state.into(), assets)?;
            }
        }
        Ok(())
    }
}

/// ftCo_DownWait_IASA (8009802C), installed only on prone wait rows.
pub(super) fn wait_input(fighter: &mut Fighter, phase: super::state::InputPhase<'_>) {
    fighter
        .down_wait_input(phase.assets)
        .expect("down wait input");
}

/// DownStand/DownAttack/Down_Anim: finish the getup at Wait.
pub(super) fn recovery_animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.change_motion_state(S::Wait.into(), phase.assets)?;
    }
    Ok(None)
}
impl FighterCore {
    /// ftCommon_8007CCE8 (8007CCE8): clamp and project residual ground knockback.
    fn project_ground_knockback(&mut self, assets: &FighterAssets) {
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground
            && self.physics.ground_knockback_velocity == 0.0
        {
            let normal = self.collision.data.floor.normal;
            let limit = assets.damage.ground_knockback_limit;
            let speed = self.physics.knockback_velocity.x.clamp(-limit, limit);
            self.physics.ground_knockback_velocity = speed;
            self.physics.knockback_velocity.x = normal.y * speed;
            self.physics.knockback_velocity.y = -normal.x * speed;
        }
    }
    /// ftCo_DownBound_Phys -> ft_80084F3C, even while the bounce script marks air.
    pub(super) fn down_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Vec3,
    ) {
        use crate::physics::{
            friction::{friction_acceleration, wait_friction},
            grounded::{self, GroundedParameters},
            integrate,
        };
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        let friction = wait_friction(
            self.physics.ground_velocity,
            params.friction,
            params.walk_max_velocity,
            params.above_walk_multiplier,
        );
        self.physics.ground_acceleration =
            friction_acceleration(self.physics.ground_velocity, friction);
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            grounded::finish_ground_update(
                &mut self.physics,
                &self.collision.data,
                &params,
                map,
                wind,
            );
        } else {
            self.decay_air_knockback(assets);
            integrate::integrate_velocity(&mut self.physics);
            integrate::integrate_environment(&mut self.physics, None, wind);
        }
    }
}
