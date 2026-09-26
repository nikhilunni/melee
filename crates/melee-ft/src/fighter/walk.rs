//! WalkSlow/Middle/Fast: ftCo_Walk.c and ftwalkcommon.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::desc::common::MovementParameters;
use crate::input::{WaitContext, WaitPredicate as P, WaitTransition as T};
use gekko_math::{
    fma::fnmsubs,
    msl::{fabsf, fctiwz},
};
use melee_types::CommonMotionState;

/// ftwalkcommon.c, Fighter.mv.co.walk (+2340..2360).
#[derive(Clone, Debug)]
pub struct WalkState {
    pub slippery_animation_velocity: f32,
    pub acceleration_multiplier: f32,
}

/// ftWalkCommon_GetWalkType_800DFBF8 (0x800DFBF8).
fn walk_state(
    speed: f32,
    max_speed: f32,
    multiplier: f32,
    common: &MovementParameters,
) -> CommonMotionState {
    // retail 0x800DFF14/18 and 0x800DFF34/38: two rounded fmuls.
    if fabsf(speed) >= multiplier * (common.fast_threshold * max_speed) {
        CommonMotionState::WalkFast
    } else if fabsf(speed) >= multiplier * (common.middle_threshold * max_speed) {
        CommonMotionState::WalkMiddle
    } else {
        CommonMotionState::WalkSlow
    }
}

impl Fighter {
    /// ftCo_800DEAE8 (800DEAE8): clear IASA, select authored left variant,
    /// ordinary motion entry; unlike attack entry, no ftAnim_8006EBA4 call.
    pub fn enter_common_taunt(&mut self, assets: &FighterAssets) -> Result<()> {
        self.commands.allow_interrupt = false;
        let state = if self.physics.facing == -1.0 && assets.left_taunt_available {
            CommonMotionState::AppealSL
        } else {
            CommonMotionState::AppealSR
        };
        self.change_motion_state(state.into(), assets)
    }

    /// ftCo_Walk_Enter (0x800C9528) -> ftWalkCommon_800DFCA4 (0x800DFCA4).
    pub(super) fn enter_walk(&mut self, assets: &FighterAssets, frame: f32) -> Result<()> {
        // Metal/status interactions are rejected by Status::require_supported;
        // scaled-player modifiers are rejected during Fighter::prepare.
        let multiplier = 1.0;
        let state = walk_state(
            self.core.physics.ground_velocity,
            self.core.attributes.walking.walk_max_vel,
            multiplier,
            &assets.movement,
        );
        self.change_motion_state_at(state.into(), assets, frame)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Walk(WalkState {
            slippery_animation_velocity: self.core.physics.ground_velocity,
            acceleration_multiplier: multiplier,
        });
        Ok(())
    }

    /// ftCo_Walk_IASA (0x800C9614), including ft_8008A244 and
    /// ftWalkCommon_800DFEC8 (0x800DFEC8) phase-preserving speed selection.
    pub(super) fn walk_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        let transition = self.first_ground_transition(
            assets,
            context,
            &[
                P::Grab,
                P::SpecialSide,
                P::SpecialUp,
                P::SpecialNeutral,
                P::SpecialDown,
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
            ],
        );
        if transition != T::None {
            return self.apply_ground_transition(assets, transition);
        }
        let stick = self.core.input.current.stick.x;
        if stick * self.core.physics.facing < 0.0
            || fabsf(stick) < assets.input.thresholds.walk_stick_threshold
        {
            return self.change_motion_state(CommonMotionState::Wait.into(), assets);
        }
        let MotionData::Walk(walk) = &self.core.state_data else {
            panic!("walk data missing")
        };
        let next = walk_state(
            self.core.physics.ground_velocity,
            self.core.attributes.walking.walk_max_vel,
            walk.acceleration_multiplier,
            &assets.movement,
        );
        if next != self.core.motion_state.id {
            let animation_id = match next {
                CommonMotionState::WalkSlow => 7,
                CommonMotionState::WalkMiddle => 8,
                CommonMotionState::WalkFast => 9,
                _ => unreachable!(),
            };
            let duration = assets.motions[&self.core.animation.motion_id]
                .animation
                .frames;
            let target_duration = assets.motions[&animation_id].animation.frames;
            let quotient = fctiwz(self.core.animation.frame / duration) as f32;
            // retail 0x800E0010: fnmsubs; quotient is converted back to f32.
            let phase = fnmsubs(duration, quotient, self.core.animation.frame);
            let frame = fctiwz(target_duration * (phase / duration)) as f32;
            self.enter_walk(assets, frame)?;
        }
        Ok(())
    }

    /// ftCo_Wait_IASA (0x8008A4D4) movement entry bodies, also used by
    /// the new states at their own ordered predicate call sites.
    pub(super) fn apply_ground_transition(
        &mut self,
        assets: &FighterAssets,
        transition: T,
    ) -> Result<()> {
        match transition {
            T::None | T::Hold => Ok(()),
            T::Special => {
                self.enter_buffered_special(assets, false);
                Ok(())
            }
            T::Attack => self.enter_ground_attack(assets),
            T::Grab => self.enter_catch(assets),
            T::Shield => self.enter_shield(assets),
            T::Taunt => (self.character.table().enter_taunt)(self, assets),
            T::Escape => self.enter_escape(assets, CommonMotionState::EscapeN),
            T::Jump => self.enter_knee_bend(assets),
            T::Dash => self.enter_dash(assets, true),
            T::Squat => self.enter_squat(assets),
            T::Walk => self.enter_walk(assets, 0.0),
            T::Turn => {
                let smash = fabsf(self.core.input.current.stick.x)
                    >= assets.input.thresholds.dash_smash_stick_threshold
                    && i32::from(self.core.input.horizontal.tilt)
                        < assets.input.thresholds.dash_smash_window;
                self.enter_turn(assets, smash)
            }
        }
    }
}
impl FighterCore {
    /// ftCo_Walk_Anim (0x800C95F4) -> ftWalkCommon_800DFDDC (0x800DFDDC).
    pub(super) fn walk_animation(&mut self, assets: &FighterAssets) {
        let MotionData::Walk(walk) = &self.state_data else {
            panic!("walk data missing")
        };
        // ft_GetGroundFrictionMultiplier reads the current floor material.
        let speed = if crate::physics::grounded::floor_friction(&self.collision.data) < 1.0 {
            walk.slippery_animation_velocity
        } else {
            self.physics.ground_velocity
        };
        let rate = if speed * self.physics.facing <= 0.0 {
            0.0
        } else {
            let attrs = &assets.attributes.walking;
            let divisor = match self.motion_state.id {
                CommonMotionState::WalkSlow => attrs.slow_walk_max,
                CommonMotionState::WalkMiddle => attrs.mid_walk_point,
                CommonMotionState::WalkFast => attrs.fast_walk_min,
                _ => unreachable!("walk callback on non-walk state"),
            };
            fabsf(speed) / divisor
        };
        self.animation.set_rate(&mut self.skeleton, rate, false);
    }

    pub(super) fn first_ground_transition(
        &self,
        assets: &FighterAssets,
        context: &WaitContext,
        predicates: &[P],
    ) -> T {
        predicates
            .iter()
            .map(|&p| crate::input::iasa::evaluate(p, &self.input, &assets.input, context))
            .find(|t| *t != T::None)
            .unwrap_or(T::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_tiers_include_the_middle_and_fast_thresholds_in_both_directions() {
        let common = MovementParameters {
            middle_threshold: 0.4,
            fast_threshold: 0.8,
            acceleration_taper: 0.0,
            slippery_animation_multiplier: 0.0,
            squat_release_threshold: 0.0,
            platform_drop_threshold: 0.0,
            platform_drop_window: 0,
            platform_drop_delay: 0.0,
            platform_drop_velocity: 0.0,
        };
        let middle = 0.4_f32;
        let fast = 0.8_f32;
        for direction in [-1.0, 1.0] {
            for (speed, expected) in [
                (
                    f32::from_bits(middle.to_bits() - 1),
                    CommonMotionState::WalkSlow,
                ),
                (middle, CommonMotionState::WalkMiddle),
                (
                    f32::from_bits(fast.to_bits() - 1),
                    CommonMotionState::WalkMiddle,
                ),
                (fast, CommonMotionState::WalkFast),
            ] {
                assert_eq!(walk_state(direction * speed, 1.0, 1.0, &common), expected);
            }
        }
    }
}
