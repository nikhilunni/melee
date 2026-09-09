//! Ground and aerial jumps: ftCommon/ftCo_KneeBend.c and ftCo_Jump{,Aerial}.c.
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use crate::input::{Buttons, WaitContext, WaitPredicate as P, WaitTransition as T};
use hsd_types::Vec3;
use melee_types::CommonMotionState;

/// PlCo +78/+7C and +88/+8C, loaded at the archive boundary.
#[derive(Clone, Copy, Debug)]
pub struct JumpParameters {
    pub backward_threshold: f32,
    pub release_threshold: f32,
    pub fast_fall_threshold: f32,
    pub fast_fall_window: i32,
}
/// ftCo_Jump_GetInput (800CAE80); stick takes priority over buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpInput {
    Stick,
    Buttons,
}
/// Fighter.mv.co.kneebend, +2340/+2344.
#[derive(Clone, Debug)]
pub struct KneeBendState {
    pub short_hop: bool,
    pub input: JumpInput,
}
/// Fighter.mv.co.jump, +2340/+2344/+2348. Entry retains the squat's hop decision.
#[derive(Clone, Debug)]
pub struct JumpState {
    pub short_hop: bool,
    pub physics_started: bool,
    pub multiplier: f32,
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_KneeBend_Enter (800CB4E0), ftCo_Jump_GetInput (800CAE80).
    pub(super) fn enter_knee_bend(&mut self, assets: &FighterAssets) -> Result<()> {
        let input = if self.input.current.stick.y >= assets.input.thresholds.tap_jump_threshold
            && i32::from(self.input.vertical.tilt) < assets.input.thresholds.tap_jump_window
        {
            JumpInput::Stick
        } else if self.input.pressed.intersects(Buttons::XY) {
            JumpInput::Buttons
        } else {
            unimplemented!("ftCo_Jump.c:69-98: relaxed/C-stick jump entry")
        };
        self.state_data = MotionData::KneeBend(KneeBendState {
            short_hop: false,
            input,
        });
        self.change_motion_state(CommonMotionState::KneeBend, assets)
    }
    /// ftCo_KneeBend_Anim (800CB528), ftCo_Jump_Enter (800CB250).
    pub(super) fn knee_bend_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.animation.frame < self.attributes.jumping.jump_startup_time
            && self.animation.frames_remaining(&self.skeleton)
        {
            return Ok(());
        }
        let MotionData::KneeBend(squat) = &self.state_data else {
            panic!("KneeBend data missing")
        };
        let short_hop = squat.short_hop;
        let state = self.jump_direction(assets, false);
        self.leave_ground();
        self.change_motion_state(state, assets)?;
        // ftCo_800CB110: retail 800CB140/144,174/180,18C/198,1A8,1B8;
        // products and sum are separately rounded, with no fusion.
        let attrs = &self.attributes.jumping;
        let multiplier = 1.0;
        let momentum = self.physics.self_velocity.x
            * (attrs.ground_to_air_jump_momentum_multiplier * multiplier);
        let horizontal =
            momentum + multiplier * (self.input.current.stick.x * attrs.jump_h_initial_velocity);
        let maximum = attrs.jump_h_max_velocity * multiplier;
        self.physics.self_velocity = Vec3::new(
            horizontal.clamp(-maximum, maximum),
            (if short_hop {
                attrs.hop_v_initial_velocity
            } else {
                attrs.jump_v_initial_velocity
            }) * multiplier,
            0.0,
        );
        self.input.vertical.tilt = 0xFE;
        self.state_data = MotionData::Jump(JumpState {
            short_hop,
            physics_started: false,
            multiplier,
        });
        Ok(())
    }
    /// ftCo_KneeBend_IASA (800CB5FC), Check_ShortHop (800CB59C).
    pub(super) fn knee_bend_input(&mut self, assets: &FighterAssets, context: &WaitContext) {
        let transition =
            self.first_ground_transition(assets, context, &[P::SpecialUp, P::Grab, P::SmashUp]);
        if transition != T::None {
            unimplemented!("ftCo_KneeBend.c:63-65: jump cancel {transition:?}");
        }
        let MotionData::KneeBend(squat) = &mut self.state_data else {
            panic!("KneeBend data missing")
        };
        squat.short_hop |= match squat.input {
            JumpInput::Buttons => !self.input.current.held.intersects(Buttons::XY),
            JumpInput::Stick => self.input.current.stick.y < assets.jumping.release_threshold,
        };
    }
    /// ftCo_Jump_Enter / JumpAerial_Enter_Basic: separate fmuls at 800CBC0C.
    fn jump_direction(&self, assets: &FighterAssets, aerial: bool) -> CommonMotionState {
        // ftCo_Jump_Enter (800CB250), JumpAerial_Enter_Basic (800CBBC0):
        // separate fmuls; choosing the backward animation does not turn facing.
        let backward =
            self.input.current.stick.x * self.physics.facing <= -assets.jumping.backward_threshold;
        match (aerial, backward) {
            (false, false) => CommonMotionState::JumpF,
            (false, true) => CommonMotionState::JumpB,
            (true, false) => CommonMotionState::JumpAerialF,
            (true, true) => CommonMotionState::JumpAerialB,
        }
    }
    /// ftCo_JumpAerial.c:103-119 character dispatch, then
    /// ftCo_JumpAerial_Enter_Basic (800CBBC0) -> ftCo_800CBAC4 for the
    /// default arm. Characters with their own double jump override
    /// `CharacterCallbacks::aerial_jump_style`; those bodies are unported.
    pub(super) fn enter_aerial_jump(&mut self, assets: &FighterAssets) -> Result<()> {
        let style = self.character.aerial_jump_style();
        if style != super::AerialJumpStyle::Basic {
            unimplemented!("ftCo_JumpAerial.c:104-116: {style:?} double jump entry");
        }
        let state = self.jump_direction(assets, true);
        self.leave_ground();
        let retained_drop_timer = self.retained_drop_timer();
        self.commands.variables[0] = 1;
        let attrs = &self.attributes.jumping;
        // retail 800CBC44/4C: separate fmuls, no fusion.
        let velocity = Vec3::new(
            self.input.current.stick.x * attrs.air_jump_h_multiplier,
            attrs.jump_v_initial_velocity * attrs.air_jump_v_multiplier,
            0.0,
        );
        self.change_motion_state(state, assets)?;
        self.physics.self_velocity = velocity;
        self.input.vertical.tilt = 0xFE;
        self.physics.jumps_used += 1;
        self.state_data = MotionData::JumpAerial {
            retained_drop_timer,
        };
        Ok(())
    }
    /// ftCo_Jump_Anim (800CB2F8), ftCo_JumpAerial_Anim (800CC388).
    pub(super) fn jump_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.animation.frames_remaining(&self.skeleton) {
            if matches!(
                self.motion_state.id,
                CommonMotionState::JumpAerialF | CommonMotionState::JumpAerialB
            ) {
                return self.change_motion_state(CommonMotionState::FallAerial, assets);
            }
            self.change_motion_state(CommonMotionState::Fall, assets)?;
        }
        Ok(())
    }
    /// ftCo_Jump_Phys_Inner (800CB438), ft_80084DB0 and CheckFallFast (8007D528).
    pub(super) fn airborne_physics(&mut self, assets: &FighterAssets) {
        if let MotionData::Jump(jump) = &mut self.state_data {
            if !jump.physics_started {
                jump.physics_started = true;
                return;
            }
        }
        if !self.physics.fast_fall
            && self.physics.self_velocity.y < 0.0
            && self.input.current.stick.y <= -assets.jumping.fast_fall_threshold
            && i32::from(self.input.vertical.tilt) < assets.jumping.fast_fall_window
        {
            self.physics.fast_fall = true;
            self.input.vertical.tilt = 0xFE;
        }
        crate::physics::airborne::fall_physics(
            &mut self.physics,
            &self.attributes.air,
            self.input.current.stick.x,
        );
    }
}
