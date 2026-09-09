//! Tumble landing and prone recovery, ftCo_DownBound.c.
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, FtPart};

impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_800986B0 / ftCo_80098928 (800986B0 / 80098928): digital edge window.
    pub(super) fn try_tech(&mut self, assets: &FighterAssets) -> Result<bool> {
        let timers = &self.input.buttons;
        if f32::from(timers.digital_shield) >= assets.damage.tech_window
            || i32::from(timers.previous_digital_shield) < assets.damage.tech_lockout
        {
            return Ok(false);
        }
        let stick = self.input.current.stick.x;
        if gekko_math::msl::fabsf(stick) < assets.damage.tech_roll_threshold {
            unimplemented!("ftCo_800987D0: neutral tech");
        }
        if stick * self.physics.facing >= 0.0 {
            unimplemented!("ftCo_800989D4: PassiveStandF");
        }
        self.land();
        self.change_motion_state(S::PassiveStandB, assets)?;
        self.effects
            .push(super::effects::EffectRequest::CaptureFlash { bone: 0 });
        Ok(true)
    }
    /// ftCo_8009794C (8009794C): choose face-up/down from the animated HipN.
    pub(super) fn enter_down_bound(&mut self, assets: &FighterAssets) -> Result<()> {
        self.land();
        let hip = self.animation.parts
            [usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"))]
        .joint;
        let matrix = self.skeleton.get_mtx(hip);
        if matrix.0[1][1] > 0.0 {
            unimplemented!("ftCo_8009794C: DownBoundU");
        }
        self.change_motion_state(S::DownBoundD, assets)?;
        self.state_data = MotionData::Down {
            wait_remaining: 0.0,
        };
        self.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        let normal = self.collision.data.floor.normal;
        let floor_angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
        // ftCo_800978D4: direct async kind 4 has no randomized offset.
        self.effects.push(super::effects::EffectRequest::Graphics {
            id: 0x406,
            bone: 0,
            offset: Vec3::ZERO,
            facing: self.physics.facing,
            floor_angle,
        });
        // ftCo_800976A4 -> ftCo_8009F834: three draws, even for zero ranges.
        self.commands
            .graphics
            .push(super::effects::GraphicsCommand {
                id: 0x407,
                bone: 0,
                common_bone: false,
                item_bone: false,
                destroy_on_state_change: false,
                parameter: 0.0,
                offset: Vec3::ZERO,
                range: Vec3::ZERO,
            });
        // ftCommon_8007CCE8 (8007CCE8 --fused: no sites): project residual KB.
        if self.physics.ground_knockback_velocity == 0.0 {
            let limit = assets.damage.ground_knockback_limit;
            let speed = self.physics.knockback_velocity.x.clamp(-limit, limit);
            self.physics.ground_knockback_velocity = speed;
            self.physics.knockback_velocity.x = normal.y * speed;
            self.physics.knockback_velocity.y = -normal.x * speed;
        }
        Ok(())
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

    /// DownBound_Anim (80097DE8), DownWait_Anim (80097FD0).
    pub(super) fn down_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.motion_state.id == S::DownBoundD {
            if !self.animation.frames_remaining(&self.skeleton) {
                if self.input.current.stick != crate::input::Stick::default()
                    || self.input.pressed.0 != 0
                {
                    unimplemented!("ftCo_DownBound_Anim: recovery input");
                }
                self.change_motion_state(S::DownWaitD, assets)?;
                self.state_data = MotionData::Down {
                    wait_remaining: assets.damage.down_wait_frames,
                };
                self.step_animation(assets);
                self.status.grab_exclusions = super::ledge::GrabExclusions(1);
            }
        } else {
            let MotionData::Down { wait_remaining } = &mut self.state_data else {
                panic!("down scratch");
            };
            *wait_remaining -= 1.0;
            if *wait_remaining <= 0.0 {
                unimplemented!("ftCo_DownWait_Anim: DownStandD");
            }
            if self.input.current.stick != crate::input::Stick::default()
                || self.input.pressed.0 != 0
            {
                unimplemented!("ftCo_DownWait_IASA: recovery input");
            }
        }
        Ok(())
    }
}
