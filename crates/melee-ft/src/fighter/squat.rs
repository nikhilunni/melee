//! Crouch entry, hold and release: ftCo_Squat.c / SquatWait.c / SquatRv.c.
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::input::{WaitContext, WaitPredicate as P, WaitTransition as T};
use melee_types::{mp::line_flag, CommonMotionState};

/// Fighter.mv.co.squat / pass (+2340/+2344).
#[derive(Clone, Debug, Default)]
pub struct SquatState {
    pub platform_drop_pending: bool,
    pub platform_drop_timer: f32,
}
impl Fighter {
    /// ftCo_Squat_Enter (0x800D600C): immediate step, clear pass, show nametag.
    pub(super) fn enter_squat(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state(CommonMotionState::Squat.into(), assets)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Squat(SquatState::default());
        self.core.status.name_tag_timer = assets.name_tag_duration;
        Ok(())
    }

    /// ftCo_Squat_Anim (0x800D607C), ftCo_800D638C (0x800D638C),
    /// ftCo_SquatRv_Anim (0x800D6658). Hold entry preserves the nametag timer.
    pub(super) fn squat_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(
                (if self.core.motion_state.id == CommonMotionState::Squat {
                    CommonMotionState::SquatWait
                } else {
                    CommonMotionState::Wait
                })
                .into(),
                assets,
            )?;
        }
        Ok(())
    }

    /// ftCo_Squat_IASA (0x800D60B8), SquatWait_IASA (0x800D6474),
    /// SquatRv_IASA (0x800D6694): distinct retail predicate order per state.
    pub(super) fn squat_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        let entering = self.core.motion_state.id == CommonMotionState::Squat;
        let releasing = self.core.motion_state.id == CommonMotionState::SquatRv;
        let predicates: &[P] = if entering {
            &[
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
            ]
        } else {
            &[
                P::SpecialDown,
                P::SpecialUp,
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
            ]
        };
        let transition = self.first_ground_transition(assets, context, predicates);
        if transition != T::None {
            return self.apply_ground_transition(assets, transition);
        }
        if releasing {
            let transition = self.first_ground_transition(assets, context, &[P::Walk]);
            return self.apply_ground_transition(assets, transition);
        }
        // ftCo_80099F9C (0x80099F9C): arm the delayed platform drop.
        let on_platform = self.core.collision.data.floor.flags & line_flag::PLATFORM != 0;
        let MotionData::Squat(squat) = &mut self.core.state_data else {
            panic!("squat data missing")
        };
        if !squat.platform_drop_pending
            && self.core.input.current.stick.y <= -assets.movement.platform_drop_threshold
            && f32::from(self.core.input.vertical.tilt) < assets.movement.platform_drop_window
            && on_platform
        {
            squat.platform_drop_pending = true;
            squat.platform_drop_timer = assets.movement.platform_drop_delay;
            return Ok(());
        }
        if entering {
            if squat.platform_drop_pending && squat.platform_drop_timer != 0.0 {
                squat.platform_drop_timer -= 1.0;
                if squat.platform_drop_timer == 0.0 && on_platform {
                    return self.enter_pass(assets);
                }
            }
        } else {
            let transition = self.first_ground_transition(assets, context, &[P::Dash]);
            if transition != T::None {
                return self.apply_ground_transition(assets, transition);
            }
            if self.core.input.current.stick.y > -assets.movement.squat_release_threshold {
                // ftCo_SquatRv_CheckInput/Enter (0x800D65D8/0x800D6620).
                self.change_motion_state(CommonMotionState::SquatRv.into(), assets)?;
            }
        }
        Ok(())
    }
}
