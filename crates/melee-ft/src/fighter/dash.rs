//! Initial dash and dash-dance input windows (ftCo_Dash.c).
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use crate::input::{WaitContext, WaitPredicate as P, WaitTransition as T};
use gekko_math::msl::fabsf;
use melee_types::CommonMotionState;

/// PlCo scalar parameters used by Dash/Run/RunBrake.
pub struct RunningParameters {
    /// +38: TurnRun signed stick threshold.
    pub turn_threshold: f32,
    /// +44/+48/+4C: initial interrupt, escape and directional redash windows.
    pub early_interrupt_frames: f32,
    pub early_escape_frames: f32,
    pub redash_frames: f32,
    /// +54: velocity reduction after an early interrupt.
    pub interrupt_friction: f32,
    /// +58/+5C/+60: run stick threshold, acceleration taper, brake friction.
    pub run_threshold: f32,
    pub acceleration_taper: f32,
    pub friction_multiplier: f32,
    /// +80: running tap-jump threshold.
    pub relaxed_jump_threshold: f32,
    /// +42C: brake animation pause/release speed.
    pub brake_pause_speed: f32,
    /// +430: Run interrupt delay after completing TurnRun.
    pub turn_exit_interrupt_delay: f32,
}
#[derive(Clone, Debug)]
pub struct DashState {
    /// mv.co.dash.x0 (+2340): one-shot initial acceleration marker.
    pub initial_acceleration: f32,
    /// mv.co.dash.x4 (+2344): enable initial attack/escape window.
    pub early_interrupts: bool,
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_Dash_Enter (0x800CA120), ftCommon_800804A0 (0x800804A0).
    pub(super) fn enter_dash(
        &mut self,
        assets: &FighterAssets,
        early_interrupts: bool,
    ) -> Result<()> {
        self.core.commands.variables[0] = 0;
        self.change_motion_state(CommonMotionState::Dash, assets)?;
        self.step_animation(assets);
        self.core.input.horizontal.tilt = 0xFE;
        // Retail Dash_Enter: separate fmuls/fsubs; 800804A0 has no fused sites.
        let initial = self.core.physics.facing * self.core.attributes.running.dash_initial_velocity;
        let acceleration = if self.core.physics.ground_velocity * self.core.physics.facing < 0.0 {
            initial
        } else {
            initial - self.core.physics.ground_velocity
        };
        let terrain = crate::physics::grounded::floor_friction(&self.core.collision.data);
        self.core.physics.secondary_ground_acceleration = if terrain < 1.0 {
            acceleration * terrain
        } else {
            acceleration
        };
        self.core.state_data = MotionData::Dash(DashState {
            initial_acceleration: acceleration,
            early_interrupts,
        });
        Ok(())
    }
    /// ftCo_Dash_Anim (0x800CA1F4), ft_8008A2BC (0x8008A2BC).
    pub(super) fn dash_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(CommonMotionState::Wait, assets)?;
        }
        Ok(())
    }
    /// ftCo_Dash_IASA (0x800CA230). Preserve its three distinct input windows.
    pub(super) fn dash_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        let MotionData::Dash(dash) = &self.core.state_data else {
            panic!("dash data missing")
        };
        let frame = self.core.animation.frame;
        let common = &assets.running;
        if dash.early_interrupts && frame <= common.early_interrupt_frames {
            self.reject_running_actions(
                assets,
                context,
                &[P::SpecialSide, P::Grab, P::SmashSide],
                "ftCo_Dash.c:90-98",
            );
            if frame <= common.early_escape_frames {
                self.reject_running_actions(assets, context, &[P::Escape], "ftCo_Dash.c:100-101");
            }
        } else if frame <= common.redash_frames {
            self.reject_running_actions(
                assets,
                context,
                &[P::SpecialSide, P::Grab],
                "ftCo_Dash.c:109-114",
            );
            self.reject_dash_attack("ftCo_Dash.c:111-114");
            if self.core.input.current.stick.x * self.core.physics.facing < 0.0
                && self.try_redash(assets)?
            {
                return Ok(());
            }
            self.reject_running_actions(assets, context, &[P::Shield], "ftCo_Dash.c:121-123");
        } else {
            self.reject_running_actions(assets, context, &[P::Grab], "ftCo_Dash.c:131");
            if self.try_redash(assets)? {
                return Ok(());
            }
            self.reject_running_actions(assets, context, &[P::Shield], "ftCo_Dash.c:133-135");
        }
        self.reject_running_actions(assets, context, &[P::Taunt], "ftCo_Dash.c:139");
        self.reject_running_jump(assets);
        if self.core.commands.variables[0] != 0
            && self.core.input.current.stick.x * self.core.physics.facing >= common.run_threshold
        {
            self.enter_run(assets)?;
        }
        Ok(())
    }
    /// ftCo_Dash_CheckInput (0x800CA094): a fresh opposite smash enters Turn.
    fn try_redash(&mut self, assets: &FighterAssets) -> Result<bool> {
        if fabsf(self.core.input.current.stick.x)
            >= assets.input.thresholds.dash_smash_stick_threshold
            && i32::from(self.core.input.horizontal.tilt)
                < assets.input.thresholds.dash_smash_window
        {
            if self.core.input.current.stick.x * self.core.physics.facing < 0.0 {
                self.enter_turn(assets, true)?;
            } else {
                self.enter_dash(assets, true)?;
            }
            // ftCo_Dash_IASA's tail after a successful dash predicate.
            let terrain = crate::physics::grounded::floor_friction(&self.core.collision.data);
            let reduction =
                -(self.core.physics.ground_velocity * assets.running.interrupt_friction);
            // retail 0x800CA51C: fmadds.
            self.core.physics.ground_velocity =
                gekko_math::fma::fmadds(reduction, terrain, self.core.physics.ground_velocity);
            return Ok(true);
        }
        Ok(false)
    }
}
impl FighterCore {
    pub(super) fn reject_running_actions(
        &self,
        assets: &FighterAssets,
        context: &WaitContext,
        predicates: &[P],
        source: &str,
    ) {
        let transition = self.first_ground_transition(assets, context, predicates);
        if transition != T::None {
            unimplemented!("{source}: {transition:?} transition body");
        }
    }
    pub(super) fn reject_dash_attack(&self, source: &str) {
        if self.input.pressed.intersects(crate::input::Buttons::A) {
            unimplemented!("{source}: attack dash/item throw");
        }
    }
    /// fn_800CAF78 (0x800CAF78), ftCo_Jump.c:62-88.
    pub(super) fn reject_running_jump(&self, assets: &FighterAssets) {
        if (self.input.current.stick.y >= assets.running.relaxed_jump_threshold
            && i32::from(self.input.vertical.tilt) < assets.input.thresholds.tap_jump_window)
            || self.input.pressed.intersects(crate::input::Buttons::XY)
        {
            unimplemented!("ftCo_Jump.c:76-84: running jump-cancel -> KneeBend");
        }
    }
}
