//! Ground and aerial jumps: ftCommon/ftCo_KneeBend.c and ftCo_Jump{,Aerial}.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::input::{Buttons, WaitContext, WaitPredicate as P, WaitTransition as T};
use hsd_types::Vec3;
use melee_types::CommonMotionState;

/// PlCo +78/+7C and +88/+8C, loaded at the archive boundary.
#[derive(Clone, Copy, Debug)]
pub struct JumpParameters {
    pub backward_threshold: f32,
    /// PlCo +258, ftCo_JumpAerialF1_Phys.
    pub multi_jump_drift_threshold: f32,
    pub release_threshold: f32,
    pub fast_fall_threshold: f32,
    pub fast_fall_window: i32,
}
/// ftCo_Jump_GetInput (800CAE80); stick takes priority over buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpInput {
    Stick,
    Buttons,
    CStick,
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
    /// mv.co.jump.x4: the C flag word (0 until physics starts, then 1;
    /// Thunder's ftPk_SpecialLw_SetState_Unk0 may store 3).
    pub physics_started: u32,
    pub multiplier: f32,
}
impl Fighter {
    /// fn_800CAF78 (800CAF78), ftCo_Jump.c:62-88: running jump cancel.
    pub(super) fn try_running_jump(&mut self, assets: &FighterAssets) -> Result<bool> {
        let input = if self.core.input.current.stick.y >= assets.running.relaxed_jump_threshold
            && i32::from(self.core.input.vertical.tilt) < assets.input.thresholds.tap_jump_window
        {
            JumpInput::Stick
        } else if self.core.input.pressed.intersects(Buttons::XY) {
            JumpInput::Buttons
        } else {
            return Ok(false);
        };
        self.core.state_data = MotionData::KneeBend(KneeBendState {
            short_hop: false,
            input,
        });
        self.change_motion_state(CommonMotionState::KneeBend.into(), assets)?;
        Ok(true)
    }
    /// ftCo_KneeBend_Enter (800CB4E0), ftCo_Jump_GetInput (800CAE80).
    pub fn enter_knee_bend(&mut self, assets: &FighterAssets) -> Result<()> {
        let input = if self.core.input.current.stick.y >= assets.input.thresholds.tap_jump_threshold
            && i32::from(self.core.input.vertical.tilt) < assets.input.thresholds.tap_jump_window
        {
            JumpInput::Stick
        } else if self.core.input.pressed.intersects(Buttons::XY) {
            JumpInput::Buttons
        } else {
            unimplemented!("ftCo_Jump.c:69-98: relaxed/C-stick jump entry")
        };
        self.enter_knee_bend_with_input(assets, input)
    }
    /// ftCo_KneeBend_Enter (800CB4E0) takes an explicit input source from
    /// ftCo_800CB024 so C-stick release controls its short-hop decision.
    pub(super) fn enter_knee_bend_with_input(
        &mut self,
        assets: &FighterAssets,
        input: JumpInput,
    ) -> Result<()> {
        self.core.state_data = MotionData::KneeBend(KneeBendState {
            short_hop: false,
            input,
        });
        self.change_motion_state(CommonMotionState::KneeBend.into(), assets)
    }
    /// ftCo_KneeBend_Anim (800CB528), ftCo_Jump_Enter (800CB250).
    pub(super) fn knee_bend_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.core.animation.frame < self.core.attributes.jumping.jump_startup_time
            && self.core.animation.frames_remaining(&self.core.skeleton)
        {
            return Ok(());
        }
        let MotionData::KneeBend(squat) = &self.core.state_data else {
            panic!("KneeBend data missing")
        };
        let short_hop = squat.short_hop;
        let state = self.jump_direction(assets, false);
        self.leave_ground();
        self.change_motion_state(state.into(), assets)?;
        // ftCo_800CB110: retail 800CB140/144,174/180,18C/198,1A8,1B8;
        // products and sum are separately rounded, with no fusion.
        let attrs = &self.core.attributes.jumping;
        let multiplier = 1.0;
        let momentum = self.core.physics.self_velocity.x
            * (attrs.ground_to_air_jump_momentum_multiplier * multiplier);
        let horizontal = momentum
            + multiplier * (self.core.input.current.stick.x * attrs.jump_h_initial_velocity);
        let maximum = attrs.jump_h_max_velocity * multiplier;
        self.core.physics.self_velocity = Vec3::new(
            horizontal.clamp(-maximum, maximum),
            (if short_hop {
                attrs.hop_v_initial_velocity
            } else {
                attrs.jump_v_initial_velocity
            }) * multiplier,
            0.0,
        );
        self.core.input.vertical.tilt = 0xFE;
        self.core.state_data = MotionData::Jump(JumpState {
            short_hop,
            physics_started: 0,
            multiplier,
        });
        Ok(())
    }
    /// ftCo_800CB870 (800CB870), item-free aerial jump check and entry.
    /// Specials use the same button/tap predicate and character jump hook.
    pub fn try_aerial_jump(&mut self, assets: &FighterAssets) -> Result<bool> {
        if !self.aerial_jump_requested(assets) {
            return Ok(false);
        }
        self.enter_aerial_jump(assets)?;
        Ok(true)
    }

    /// ftCo_JumpAerial.c:103-119 character dispatch, then
    /// ftCo_JumpAerial_Enter_Basic (800CBBC0) -> ftCo_800CBAC4 for the
    /// default arm. Characters with their own double jump override
    /// `CharacterCallbacks::aerial_jump_style`.
    pub(super) fn enter_aerial_jump(&mut self, assets: &FighterAssets) -> Result<()> {
        let style = self.character.aerial_jump_style();
        if style == super::AerialJumpStyle::MultiJump {
            return self.enter_multi_jump(assets);
        }
        if !matches!(
            style,
            super::AerialJumpStyle::Basic
                | super::AerialJumpStyle::Peach
                | super::AerialJumpStyle::Yoshi
        ) {
            unimplemented!("ftCo_JumpAerial.c:104-116: {style:?} double jump entry");
        }
        let state = if style == super::AerialJumpStyle::Yoshi {
            CommonMotionState::JumpAerialF
        } else {
            self.jump_direction(assets, true)
        };
        self.leave_ground();
        let retained_drop_timer = self.retained_drop_timer();
        self.core.commands.variables[0] = 1;
        // ftCo_800CBAC4: x2221_b7, a parasol may open without the stick.
        self.core.parasol.without_stick = true;
        let attrs = &self.core.attributes.jumping;
        // retail 800CBC44/4C; Peach horizontal multiply 800CC15C:
        // separate fmuls, no fusion.
        let velocity = Vec3::new(
            self.core.input.current.stick.x * attrs.air_jump_h_multiplier,
            // ftPe_JumpAerial_Enter stores +0 Y/Z at 800CC198..1A0;
            // ftYs_JumpAerial_Enter likewise uses animation-driven vertical motion.
            if matches!(
                style,
                super::AerialJumpStyle::Peach | super::AerialJumpStyle::Yoshi
            ) {
                0.0
            } else {
                attrs.jump_v_initial_velocity * attrs.air_jump_v_multiplier
            },
            0.0,
        );
        self.change_motion_state(state.into(), assets)?;
        self.core.physics.self_velocity = velocity;
        self.core.input.vertical.tilt = 0xFE;
        self.core.physics.jumps_used += 1;
        self.core.state_data = MotionData::JumpAerial {
            retained_drop_timer,
        };
        (self.character.table().aerial_jump_entered)(self);
        Ok(())
    }
    /// ftCo_Jump_Anim (800CB2F8), ftCo_JumpAerial_Anim (800CC388).
    pub(super) fn jump_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            if matches!(
                self.core.motion_state.id,
                CommonMotionState::JumpAerialF | CommonMotionState::JumpAerialB
            ) {
                self.change_motion_state(CommonMotionState::FallAerial.into(), assets)?;
                (self.character.table().aerial_jump_animated)(self);
                return Ok(());
            }
            self.change_motion_state(CommonMotionState::Fall.into(), assets)?;
        }
        if matches!(self.core.state_data, MotionData::JumpAerial { .. }) {
            (self.character.table().aerial_jump_animated)(self);
        }
        Ok(())
    }
    /// ftCo_Jump_Phys_Inner (800CB438), ft_80084DB0 and CheckFallFast (8007D528).
    #[inline(always)]
    pub(super) fn airborne_physics(&mut self, assets: &FighterAssets) {
        let animation_driven = matches!(self.core.state_data, MotionData::JumpAerial { .. })
            && matches!(
                self.character.aerial_jump_style(),
                super::AerialJumpStyle::Peach | super::AerialJumpStyle::Yoshi
            );
        self.core.airborne_physics(assets, animation_driven);
    }
    /// ftCo_KneeBend_IASA (800CB5FC), Check_ShortHop (800CB59C).
    pub(super) fn knee_bend_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        let transition = self.first_ground_transition(assets, context, &[P::SpecialUp, P::Grab]);
        // ftCo_Attack100_CheckInput (800D695C) directly selects SpecialHi.
        // Ordinary special selection would prefer Side on diagonal input.
        if matches!(transition, T::Special(_)) {
            (self.character.table().enter_special)(self, super::SpecialSlot::Up, false, assets);
            return Ok(());
        }
        // ftCo_KneeBend_IASA -> ftCo_Catch_CheckInput (800D8990): the
        // standing Catch entry wins over up-smash and short-hop bookkeeping.
        if transition == T::Grab {
            return self.enter_catch(assets);
        }
        // ftCo_AttackHi4_CheckInputNoD0 (8008C948): the jump squat ignores
        // the ordinary up-smash stick timer. C-stick still requires an edge.
        let input = &self.core.input;
        let threshold = assets.input.up_smash_threshold;
        let cstick_up = input.previous.cstick.y < threshold && input.current.cstick.y >= threshold;
        if (input.pressed.intersects(Buttons::A) && input.current.stick.y >= threshold) || cstick_up
        {
            // With an item in hand, A (ftCo_80094E54) or the C-stick
            // (ftCo_800DF30C) throws it upward instead.
            if self.core.held_item.is_some() && (self.core.item_throw_pressed() || cstick_up) {
                return self.enter_item_throw(CommonMotionState::LightThrowHi4, assets);
            }
            return self.enter_simple_attack(CommonMotionState::AttackHi4, assets);
        }
        let MotionData::KneeBend(squat) = &mut self.core.state_data else {
            panic!("KneeBend data missing")
        };
        squat.short_hop |= match squat.input {
            JumpInput::Buttons => !self.core.input.current.held.intersects(Buttons::XY),
            JumpInput::Stick => self.core.input.current.stick.y < assets.jumping.release_threshold,
            JumpInput::CStick => {
                self.core.input.current.cstick.y < assets.jumping.release_threshold
            }
        };
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_Jump_Enter / JumpAerial_Enter_Basic: separate fmuls at 800CBC0C.
    pub(super) fn jump_direction(&self, assets: &FighterAssets, aerial: bool) -> CommonMotionState {
        // ftCo_Jump_Enter (800CB250), JumpAerial_Enter_Basic (800CBBC0):
        // ftPe_JumpAerial_Enter (800CC130) also uses separate fmuls.
        // Choosing the backward animation does not turn facing.
        let backward =
            self.input.current.stick.x * self.physics.facing <= -assets.jumping.backward_threshold;
        match (aerial, backward) {
            (false, false) => CommonMotionState::JumpF,
            (false, true) => CommonMotionState::JumpB,
            (true, false) => CommonMotionState::JumpAerialF,
            (true, true) => CommonMotionState::JumpAerialB,
        }
    }

    /// ftCommon_CheckFallFast (8007D528), FallFast (8007D4E4), Fall (8007D494).
    /// Shared by ordinary airborne physics and ft_80084E1C's multijump drift.
    pub(super) fn apply_fall_gravity(&mut self, assets: &FighterAssets) {
        if !self.physics.fast_fall
            && self.physics.self_velocity.y < 0.0
            && self.input.current.stick.y <= -assets.jumping.fast_fall_threshold
            && i32::from(self.input.vertical.tilt) < assets.jumping.fast_fall_window
        {
            self.physics.fast_fall = true;
            self.input.vertical.tilt = 0xFE;
        }
        let air = &self.attributes.air;
        self.physics.self_velocity.y = if self.physics.fast_fall {
            -air.fast_fall_velocity
        } else {
            crate::physics::airborne::gravity(
                self.physics.self_velocity.y,
                air.gravity,
                air.terminal_velocity,
            )
        };
    }
}

impl FighterCore {
    /// ftCo_JumpAerial_Phys_Cb (800CC6C8): drift, then TransN's vertical
    /// delta. Also Peach's FloatFall physics (ftPe_FloatFall_Phys).
    pub fn root_motion_aerial_physics(&mut self, assets: &FighterAssets) {
        self.airborne_physics(assets, true);
    }
    /// ftCo_Jump_Phys_Inner (800CB438): calculation after jump-style selection.
    fn airborne_physics(&mut self, assets: &FighterAssets, animation_driven: bool) {
        if animation_driven {
            // ftCo_JumpAerial_Phys_Cb (800CC6C8): ftCommon_8007D268 drift,
            // then ft_800851D0 (800851F8/FC) copies TransN's vertical delta.
            self.physics.animation_velocity.x = crate::physics::airborne::drift(
                self.physics.self_velocity.x,
                self.input.current.stick.x,
                &self.attributes.air,
            );
            self.physics.self_velocity.y = self
                .animation
                .root_motion
                .as_ref()
                .expect("aerial jump root motion")
                .primary_history
                .offset
                .y;
            return;
        }
        // ftCo_Jump_Phys skips its first frame; other states sharing this
        // physics (Fall, aerials, Fire Fox's fall) may carry stale Jump
        // scratch and never skip.
        let jumping = matches!(
            self.motion_state.id,
            CommonMotionState::JumpF | CommonMotionState::JumpB
        );
        if let MotionData::Jump(jump) = &mut self.state_data {
            if jumping && jump.physics_started == 0 {
                jump.physics_started = 1;
                return;
            }
        }
        self.apply_fall_gravity(assets);
        self.physics.animation_velocity.x = crate::physics::airborne::drift(
            self.physics.self_velocity.x,
            self.input.current.stick.x,
            &self.attributes.air,
        );
    }
}
