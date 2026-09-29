//! ftCo_Landing.c:41-163, including direct crouch after landing lag.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use melee_types::{mp::coll_data_x130, CommonMotionState, GroundOrAir};

/// ftCo_Landing_IASA (0x800D5D78): lag gates interruptions, not animation end.
pub fn can_interrupt(frame: f32, landing_lag: f32, allow_interrupt: bool) -> bool {
    frame >= landing_lag && allow_interrupt
}
/// ftCo_Landing_IASA: ordered item-free predicates, with the one-frame
/// SquatWait opportunity after landing lag. This state has no Escape predicate.
pub fn iasa(
    input: &crate::input::FighterInput,
    common: &crate::input::InputCommonData,
    context: &crate::input::WaitContext,
    frame: f32,
    speed: f32,
    lag: f32,
    allow: bool,
) -> crate::input::WaitTransition {
    use crate::input::{WaitPredicate as P, WaitTransition as T};
    if !can_interrupt(frame, lag, allow) {
        return T::None;
    }
    for predicate in [
        P::SpecialSide,
        P::SpecialUp,
        P::SpecialNeutral,
        P::SpecialDown,
        P::Grab,
        P::SmashSide,
        P::SmashUp,
        P::SmashDown,
        P::TiltSide,
        P::TiltUp,
        P::TiltDown,
        P::Jab,
        P::Shield,
        P::Taunt,
        P::Jump,
        P::Dash,
        P::Squat,
        P::Turn,
        P::Walk,
    ] {
        if predicate == P::Squat && frame >= speed + lag {
            continue;
        }
        let transition = crate::input::iasa::evaluate(predicate, input, common, context);
        if transition != T::None {
            return transition;
        }
    }
    T::None
}
impl Fighter {
    /// ftCo_LandingFallSpecial_Enter (800D5CB0), ftCo_Landing.c:103-113.
    pub fn enter_special_landing(
        &mut self,
        assets: &FighterAssets,
        allow_interrupt: bool,
        lag: f32,
    ) -> Result<()> {
        let retained_drop_timer = self.retained_drop_timer();
        self.land();
        // Fighter.x2EC is the normal Landing animation length (fighter.c:836).
        // Retail 800D5D08 fadds then 800D5D14 fdivs; no fusion.
        let rate = (0.1 + assets.motions[&35].animation.frames) / lag;
        self.change_motion_state_with_rate(
            CommonMotionState::LandingFallSpecial.into(),
            assets,
            0.0,
            rate,
        )?;
        self.core.state_data = MotionData::Landing {
            allow_interrupt,
            retained_drop_timer,
        };
        self.character.on_landing(allow_interrupt);
        (self.character.table().landing_articles)(self, allow_interrupt);
        Ok(())
    }
    /// ftCo_LandingFallSpecial_Enter_Basic (800D5C54): LandingFallSpecial
    /// at its own rate, without the landing interrupt.
    pub(super) fn enter_basic_special_landing(&mut self, assets: &FighterAssets) -> Result<()> {
        let retained_drop_timer = self.retained_drop_timer();
        self.land();
        self.change_motion_state(CommonMotionState::LandingFallSpecial.into(), assets)?;
        self.core.state_data = MotionData::Landing {
            allow_interrupt: false,
            retained_drop_timer,
        };
        self.character.on_landing(false);
        (self.character.table().landing_articles)(self, false);
        Ok(())
    }
    /// ftCo_Landing_Enter_Basic -> ftCo_Landing_Enter (0x800D5AEC).
    pub fn enter_landing(&mut self, assets: &FighterAssets) -> Result<()> {
        self.enter_landing_with(assets, true)
    }
    /// ftCo_Landing_Enter (0x800D5AEC) on Landing at frame 0, rate 1, with
    /// the caller's mv.co.landing.allow_interrupt (ftCo_AirCatch_Coll
    /// passes false).
    pub fn enter_landing_with(
        &mut self,
        assets: &FighterAssets,
        allow_interrupt: bool,
    ) -> Result<()> {
        let retained_drop_timer = self.retained_drop_timer();
        self.land();
        self.change_motion_state(CommonMotionState::Landing.into(), assets)?;
        self.character.on_landing(allow_interrupt);
        (self.character.table().landing_articles)(self, allow_interrupt);
        self.core.state_data = MotionData::Landing {
            allow_interrupt,
            retained_drop_timer,
        };
        Ok(())
    }
    /// ftCo_Landing_IASA -> fn_800D62C4 (800D62C4): direct SquatWait.
    pub(super) fn enter_landing_squat(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Landing {
            allow_interrupt,
            retained_drop_timer,
        } = self.core.state_data
        else {
            panic!("landing scratch missing")
        };
        self.change_motion_state(CommonMotionState::SquatWait.into(), assets)?;
        // SquatWait entry preserves both words; no Squat initialization runs.
        self.core.state_data = MotionData::Squat(super::squat::SquatState {
            platform_drop_pending: allow_interrupt,
            platform_drop_timer: retained_drop_timer,
        });
        self.core.status.name_tag_timer = assets.name_tag_duration;
        Ok(())
    }
    /// ftCo_Landing_Anim (0x800D5D3C): complete animation -> Wait.
    pub(super) fn landing_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(CommonMotionState::Wait.into(), assets)?;
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftCommon_8007D7FC -> 8007D6A4 (ftcommon.c:550-595).
    /// Landing keeps vertical self velocity until the next ground Phys callback.
    pub fn land(&mut self) {
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let offset = self
                .animation
                .root_motion
                .as_ref()
                .expect("landing TransN")
                .primary_history
                .offset
                .z;
            // Retail 8007D6CC fmuls: root motion supplies horizontal velocity.
            self.physics.self_velocity.x = offset * self.physics.facing;
        }
        // Retail clamps the OLD gr_vel (8007D6D4..6F8), then overwrites it
        // from self_vel (8007D704/708). It does not clamp the new velocity.
        self.physics.ground_or_air = GroundOrAir::Ground;
        self.physics.ground_velocity = self.physics.self_velocity.x;
        self.physics.jumps_used = 0;
        self.status.wall_jump.used = 0; // ftCommon_8007D6A4, Fighter +1969.
        self.collision.lock_frames = 0;
        self.collision.data.x130_flags &= !coll_data_x130::LOCKED;
    }
    /// The second motion scratch word (mv+4) as common states leave it, or
    /// `None` when the state's scratch there is not modelled.
    fn common_scratch_word(&self) -> Option<f32> {
        Some(match &self.state_data {
            // ftCo_800D5600: mv.common.x4 is the
            // platform target vector; an aerial entry retains its first word.
            MotionData::Life(super::life::LifeState::PlatformWait { target, .. }) => target.x,
            MotionData::Damage(_) => 0.0, // ftCo_Damage.mv.x4: low-knockback collision flag.
            MotionData::EscapeAir(dodge) => dodge.saved_velocity.x,
            MotionData::MultiJump(jump) => jump.retained_drop_timer,
            MotionData::CaptureJump(jump) => jump.retained_drop_timer,
            MotionData::CliffJump(jump) => jump.retained_wait_frames,
            MotionData::Jump(jump) => f32::from_bits(jump.physics_started),
            MotionData::Aerial {
                retained_drop_timer,
            }
            | MotionData::Pass {
                retained_drop_timer,
            }
            | MotionData::JumpAerial {
                retained_drop_timer,
            }
            | MotionData::Landing {
                retained_drop_timer,
                ..
            } => *retained_drop_timer,
            MotionData::Fall(fall) => fall.blend,
            // mv.co.itemthrow4.anim_spd (ftCo_800957F4 writes it first).
            MotionData::ItemThrow(throw) => throw.rate,
            MotionData::ShieldBreak { retained_word } => (*retained_word)?,
            MotionData::Dizzy(dizzy) => dizzy.retained_word?,
            // mv.co.guard.x4: the shield's smoothed tilt magnitude.
            MotionData::Guard(guard) => guard.tilt_magnitude,
            // A roll writes only mv.co.escape.x0 (ftCo_80099314); x4 is the
            // predecessor's word, the shield's tilt out of Guard.
            MotionData::Escape(escape) => escape.retained_word?,
            MotionData::FallSpecial(fall) => fall.animation.blend,
            // mv.co.common.x4.x: the spawn height (ftCo_800C61B0), kept
            // through EntryStart/EntryEnd and into Wait.
            MotionData::Entry(entry) => entry.origin_y,
            // mv.co.rebound.anim_speed (ftCo_80099D9C).
            MotionData::Rebound(rebound) => rebound.animation_rate,
            // mv.co.cliff.x4: CliffWait's countdown (ftCo_8009A804), which
            // the climb, roll, attack and jump states leave in place.
            // CliffCatch writes only the ledge id and keeps an unmodelled word.
            MotionData::Cliff(cliff)
                if self.motion_state.id != melee_types::CommonMotionState::CliffCatch =>
            {
                cliff.wait_frames
            }
            // mv.co.attack100.x4: the A-edge latch (ftCo_800D6B00 clears it).
            MotionData::RapidJab(rapid) => f32::from_bits(u32::from(rapid.edge_pressed)),
            MotionData::Parasol(parasol) => parasol.retained_word?,
            // The +2344 words of the ground states (ftCommon types.h): an
            // int or enum is its bit pattern, as for Jump above.
            MotionData::WallJump(wall_jump) => f32::from_bits(wall_jump.retained_zero as u32),
            MotionData::Squat(squat) => squat.platform_drop_timer,
            MotionData::Turn(turn) => turn.facing_after,
            // mv.co.walk.msid: ftCo_Walk_Enter passes the base walk state.
            MotionData::Walk(walk) => f32::from_bits(walk.base_motion as u32),
            MotionData::Dash(dash) => f32::from_bits(u32::from(dash.early_interrupts)),
            MotionData::Run(run) => run.slippery_animation_velocity,
            MotionData::RunBrake(brake) => brake.remaining_frames,
            // TurnRun writes only +234C and +2354 (ftCo_TurnRun.c:48-51).
            MotionData::TurnRun(turn) => turn.retained_word?,
            MotionData::Smash { retained_word }
            | MotionData::Tilt { retained_word }
            | MotionData::Catch { retained_word }
            | MotionData::DownTilt { retained_word, .. }
            | MotionData::DashAttack { retained_word, .. }
            | MotionData::ItemGet { retained_word, .. }
            | MotionData::Down { retained_word, .. }
            | MotionData::Jab(super::attack::JabState { retained_word, .. }) => (*retained_word)?,
            // mv.co.kneebend.jump_input: ftCo_JumpInput (LStick 1, CStick 2, XY 3).
            MotionData::KneeBend(knee_bend) => f32::from_bits(match knee_bend.input {
                super::jump::JumpInput::Stick => 1,
                super::jump::JumpInput::CStick => 2,
                super::jump::JumpInput::Buttons => 3,
            }),
            _ => return None,
        })
    }
}
impl Fighter {
    /// The current second motion scratch word (mv+4): a character special's
    /// typed scratch reports it through its table, common states directly.
    /// `None` when the port does not model the state's word.
    pub fn inherited_scratch_word(&self) -> Option<f32> {
        self.character
            .retained_scratch_word(self.core.motion_state.action)
            .or_else(|| self.core.common_scratch_word())
    }

    /// Another object's write of the second motion scratch word (mv+4)
    /// while a common state owns it (ftPk_SpecialLw_SetState_Unk0 from a
    /// Thunder bolt's end). The state then runs on with that word: Jump's
    /// flag (ftCo_Jump.c:185-197), Fall's blend (ftCo_Fall.c:186), Run's
    /// slippery velocity (ftCo_Run.c:84) and Squat's drop timer
    /// (ftCo_Squat.c:82-85) read it as their own. Walk's word is its base
    /// motion id (ftwalkcommon.c:115, 137): its IASA then re-enters Walk,
    /// and its animation callback fails closed on the uninitialised rate.
    /// Unmodelled states fail closed.
    pub fn overwrite_common_scratch_word(&mut self, word: f32) {
        match &mut self.core.state_data {
            MotionData::Jump(jump) => jump.physics_started = word.to_bits(),
            MotionData::Fall(fall) => fall.blend = word,
            MotionData::Run(run) => run.slippery_animation_velocity = word,
            MotionData::Squat(squat) => squat.platform_drop_timer = word,
            MotionData::Walk(walk) => walk.base_motion = word.to_bits() as i32,
            MotionData::Aerial {
                retained_drop_timer,
            }
            | MotionData::Pass {
                retained_drop_timer,
            }
            | MotionData::JumpAerial {
                retained_drop_timer,
            }
            | MotionData::Landing {
                retained_drop_timer,
                ..
            } => *retained_drop_timer = word,
            MotionData::Smash { retained_word }
            | MotionData::Tilt { retained_word }
            | MotionData::Catch { retained_word }
            | MotionData::DownTilt { retained_word, .. }
            | MotionData::DashAttack { retained_word, .. }
            | MotionData::ItemGet { retained_word, .. }
            | MotionData::Down { retained_word, .. }
            | MotionData::Jab(super::attack::JabState { retained_word, .. }) => {
                *retained_word = Some(word)
            }
            _ => unimplemented!(
                "mv+4 written from outside during motion {:?}",
                self.core.motion_state.action
            ),
        }
    }

    /// Fox JumpAerial and Landing leave the second motion scratch word untouched.
    /// Direct SquatWait entry inherits it as the inactive platform-drop timer
    /// (ftCo_JumpAerial.c:147-182, ftCo_Landing.c:41-50, SquatWait.c:55-88).
    pub(super) fn retained_drop_timer(&self) -> f32 {
        self.inherited_scratch_word().unwrap_or_else(|| {
            unimplemented!(
                "ftCo_Landing.c:41-50: scratch inheritance from unsupported source {:?}",
                self.core.motion_state.action
            )
        })
    }
}
// S2: aerial landing lag and autocancel.
impl Fighter {
    /// ftCo_LandingAir_EnterWithLag (8008D5FC), ftCo_LandingAir.c:14-63.
    pub(super) fn land_from_aerial(&mut self, assets: &FighterAssets) -> Result<()> {
        use CommonMotionState as S;
        if self.core.commands.variables[0] == 0 {
            return self.enter_landing(assets);
        }
        let landing = &self.core.attributes.landing;
        let (state, lag) = match self.core.motion_state.id {
            S::AttackAirN => (S::LandingAirN, landing.landingairn_lag),
            S::AttackAirF => (S::LandingAirF, landing.landingairf_lag),
            S::AttackAirB => (S::LandingAirB, landing.landingairb_lag),
            S::AttackAirHi => (S::LandingAirHi, landing.landingairhi_lag),
            S::AttackAirLw => (S::LandingAirLw, landing.landingairlw_lag),
            _ => return self.enter_landing(assets),
        };
        let lag = cancelled_lag(lag, self.core.input.buttons.shield, &assets.input);
        // ftCo_LandingAir_EnterWithMsidLag (8008D708): install at rate 1,
        // then change the rate. Do not run Landing_Enter's character hook.
        self.land();
        self.change_motion_state(state.into(), assets)?;
        // retail 8008D764 fadds, 8008D768 fdivs: separate single operations.
        let frames = assets.motions[&self.core.motion_state.animation]
            .animation
            .frames;
        let rate = (frames + 0.1) / lag;
        self.core
            .animation
            .set_rate(&mut self.core.skeleton, rate, false);
        Ok(())
    }
}

/// Retail 8008D690 reads x67F, 8008D6A4 fdivs then 8008D6A8 fctiwz.
pub fn cancelled_lag(lag: f32, age: u8, common: &crate::input::InputCommonData) -> f32 {
    if i32::from(age) < common.l_cancel_window {
        let frames = gekko_math::msl::fctiwz(lag / common.l_cancel_divisor);
        if frames == 0 {
            1.0
        } else {
            frames as f32
        }
    } else {
        lag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn landing_lag_unlocks_on_the_attribute_frame() {
        for frame in 0..30 {
            assert_eq!(can_interrupt(frame as f32, 4.0, true), frame >= 4);
        }
        assert!(!can_interrupt(30.0, 4.0, false));
    }
}
