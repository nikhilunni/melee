//! ftCo_Landing.c:107-163, neutral-input path.
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
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
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCommon_8007D7FC -> 8007D6A4 (ftcommon.c:550-595).
    /// Landing keeps vertical self velocity until the next ground Phys callback.
    pub fn land(&mut self) {
        let max = self.attributes.ground.ground_max_horizontal_velocity;
        self.physics.self_velocity.x = self.physics.self_velocity.x.clamp(-max, max);
        self.physics.ground_or_air = GroundOrAir::Ground;
        self.physics.ground_velocity = self.physics.self_velocity.x;
        self.physics.jumps_used = 0;
        self.collision.lock_frames = 0;
        self.collision.data.x130_flags &= !coll_data_x130::LOCKED;
    }
    /// ftCo_Landing_Enter_Basic -> ftCo_Landing_Enter (0x800D5AEC).
    pub(super) fn enter_landing(&mut self, assets: &FighterAssets) -> Result<()> {
        self.land();
        self.change_motion_state(CommonMotionState::Landing, assets)?;
        self.state_data = MotionData::Landing {
            allow_interrupt: true,
        };
        Ok(())
    }
    /// ftCo_Landing_Anim (0x800D5D3C): complete animation -> Wait.
    pub(super) fn landing_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.animation.frames_remaining(&self.skeleton) {
            self.change_motion_state(CommonMotionState::Wait, assets)?;
        }
        Ok(())
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
