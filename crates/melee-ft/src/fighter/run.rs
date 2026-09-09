//! Run and RunBrake state callbacks (ftCo_Run.c / ftCo_RunBrake.c).
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use crate::{
    input::{WaitContext, WaitPredicate as P},
    physics::{friction::friction_acceleration, grounded},
};
use gekko_math::msl::fabsf;
use melee_types::CommonMotionState;
#[derive(Clone, Debug)]
pub struct RunState {
    /// mv.co.run.x0 (+2340): turn/brake lockout countdown.
    pub interrupt_delay: f32,
    /// mv.co.run.x4 (+2344): animation speed estimate on slippery terrain.
    pub slippery_animation_velocity: f32,
}
#[derive(Clone, Debug)]
pub struct RunBrakeState {
    /// mv.co.runbrake.x0 (+2340): animation has paused at the skid pose.
    pub animation_paused: bool,
    /// mv.co.runbrake.frames (+2344): maximum remaining skid duration.
    pub remaining_frames: f32,
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// fn_800CA5F0 / ftCo_Run_Enter / ftCo_Run_Enter_Full (0x800CA5F0/800CA6F4/800CA71C).
    pub(super) fn enter_run(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state(CommonMotionState::Run, assets)?;
        self.state_data = MotionData::Run(RunState {
            interrupt_delay: 0.0,
            slippery_animation_velocity: self.physics.ground_velocity,
        });
        Ok(())
    }
    /// ftCo_Run_Anim (0x800CA77C).
    pub(super) fn run_animation(&mut self) {
        let MotionData::Run(run) = &mut self.state_data else {
            panic!("run data missing")
        };
        let velocity = if grounded::floor_friction(&self.collision.data) < 1.0 {
            run.slippery_animation_velocity
        } else {
            self.physics.ground_velocity
        };
        // Retail separate fmuls comparison and fdivs; no eligible FMA.
        let rate = if velocity * self.physics.facing <= 0.0 {
            0.0
        } else {
            fabsf(velocity) / self.attributes.running.run_animation_scaling
        };
        self.animation.set_rate(&mut self.skeleton, rate, false);
        if run.interrupt_delay > 0.0 {
            run.interrupt_delay -= 1.0;
        }
    }
    /// ftCo_Run_IASA (0x800CA830).
    pub(super) fn run_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        self.reject_running_actions(
            assets,
            context,
            &[
                P::SpecialSide,
                P::SpecialUp,
                P::SpecialNeutral,
                P::SpecialDown,
                P::Grab,
            ],
            "ftCo_Run.c:121-125",
        );
        self.reject_dash_attack("ftCo_Run.c:127-130");
        self.reject_running_actions(
            assets,
            context,
            &[P::Shield, P::Taunt],
            "ftCo_Run.c:133-140",
        );
        self.reject_running_jump(assets);
        let MotionData::Run(run) = &self.state_data else {
            panic!("run data missing")
        };
        if run.interrupt_delay <= 0.0 {
            if self.try_turn_run(assets, 0.0)? {
                return Ok(());
            }
            if fabsf(self.input.current.stick.x) < assets.running.run_threshold {
                self.enter_run_brake(assets)?;
            }
        }
        Ok(())
    }
    /// ftCo_RunBrake_Enter (0x800CAC18).
    fn enter_run_brake(&mut self, assets: &FighterAssets) -> Result<()> {
        self.commands.variables[0] = 0;
        self.commands.variables[1] = 0;
        self.change_motion_state(CommonMotionState::RunBrake, assets)?;
        self.state_data = MotionData::RunBrake(RunBrakeState {
            animation_paused: false,
            remaining_frames: self.attributes.running.max_run_brake_frames,
        });
        Ok(())
    }
    /// ftCo_RunBrake_Anim (0x800CAC9C).
    pub(super) fn run_brake_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::RunBrake(brake) = &mut self.state_data else {
            panic!("brake data missing")
        };
        if self.commands.variables[1] != 0 {
            let speed = fabsf(self.physics.ground_velocity);
            if !brake.animation_paused {
                if speed >= assets.running.brake_pause_speed {
                    self.animation.set_rate(&mut self.skeleton, 0.0, false);
                    brake.animation_paused = true;
                }
            } else if speed <= assets.running.brake_pause_speed {
                self.animation.set_rate(&mut self.skeleton, 1.0, false);
                self.commands.variables[1] = 0;
            }
        }
        if brake.remaining_frames != 0.0 {
            brake.remaining_frames = (brake.remaining_frames - 1.0).max(0.0);
        }
        if !(self.animation.frames_remaining(&self.skeleton) && brake.remaining_frames != 0.0) {
            self.change_motion_state(CommonMotionState::Wait, assets)?;
        }
        Ok(())
    }
    /// ftCo_RunBrake_IASA (0x800CADB0).
    pub(super) fn run_brake_input(&mut self, assets: &FighterAssets) -> Result<()> {
        self.reject_running_jump(assets);
        if self.commands.variables[0] != 0 && self.try_turn_run(assets, self.animation.frame)? {
            return Ok(());
        }
        if self.input.current.stick.y < -assets.input.thresholds.squat_stick_threshold {
            self.enter_squat(assets)?;
        }
        Ok(())
    }
    /// ftCo_Dash_Phys (0x800CA53C), ftCo_Run_Phys (0x800CA95C),
    /// ftCo_RunBrake_Phys (0x800CAE18). Retail --fused: no fused sites.
    pub(super) fn running_physics(&mut self, assets: &FighterAssets) {
        let friction = assets.running.friction_multiplier * self.attributes.ground.ground_friction;
        match &mut self.state_data {
            MotionData::RunBrake(_) => {
                self.physics.ground_acceleration =
                    friction_acceleration(self.physics.ground_velocity, friction)
            }
            MotionData::Dash(dash) if dash.initial_acceleration != 0.0 => {
                dash.initial_acceleration = 0.0
            }
            MotionData::Dash(_) | MotionData::Run(_) => {
                let stick = self.input.current.stick.x;
                let attrs = &self.attributes.running;
                let mut acceleration = stick * attrs.dash_accel_mul;
                acceleration += if stick > 0.0 {
                    attrs.dash_accel_base
                } else {
                    -attrs.dash_accel_base
                };
                let target = stick * attrs.dash_max_velocity;
                if let MotionData::Run(run) = &mut self.state_data {
                    if target != 0.0 {
                        let ratio = self.physics.ground_velocity / target;
                        if ratio > 0.0 && ratio < 1.0 {
                            acceleration *= (1.0 - ratio) * assets.running.acceleration_taper;
                        }
                    }
                    run.slippery_animation_velocity =
                        target * assets.movement.slippery_animation_multiplier;
                }
                grounded::accelerate_toward(
                    &mut self.physics,
                    acceleration,
                    target,
                    friction,
                    self.attributes.ground.ground_max_horizontal_velocity,
                );
            }
            _ => unreachable!("running physics without running state"),
        }
    }
}
