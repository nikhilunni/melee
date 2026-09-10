//! Standing turn, ftCommon/ftCo_Turn.c. TurnRun lives in turn_run.rs.
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::input::{Buttons, WaitContext, WaitPredicate as P, WaitTransition as T};
use melee_types::CommonMotionState;

/// Fighter.mv.co.turn (+2340..235C), ftCo_Turn_Enter (0x800C9840).
#[derive(Clone, Debug)]
pub struct TurnState {
    pub has_turned: bool,
    pub facing_after: f32,
    pub dash_direction: f32,
    pub frames_to_turn: f32,
    pub just_turned: bool,
    pub buffered_buttons: Buttons,
}
impl Fighter {
    /// ftCo_Turn_Enter_Basic (0x800C98AC), ftCo_Turn_Enter_Smash (0x800C9C74).
    pub(super) fn enter_turn(&mut self, assets: &FighterAssets, smash: bool) -> Result<()> {
        self.core.state_data = MotionData::Turn(TurnState {
            has_turned: false,
            just_turned: false,
            facing_after: -self.core.physics.facing,
            dash_direction: if smash { self.core.physics.facing } else { 0.0 },
            frames_to_turn: if smash {
                0.0
            } else {
                self.core.attributes.ground.standing_turn_frames
            },
            buffered_buttons: Buttons(0),
        });
        self.change_motion_state(CommonMotionState::Turn.into(), assets)?;
        self.step_animation(assets);
        Ok(())
    }

    /// ftCo_Turn_Anim (0x800C9970), inner (0x800C9924): check before decrement.
    pub(super) fn turn_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Turn(turn) = &mut self.core.state_data else {
            panic!("turn data missing")
        };
        if turn.frames_to_turn > 0.0 {
            turn.frames_to_turn -= 1.0;
        } else if !turn.has_turned {
            turn.has_turned = true;
            turn.just_turned = true;
            self.core.physics.facing = -self.core.physics.facing;
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(CommonMotionState::Wait.into(), assets)?;
        }
        Ok(())
    }

    /// ftCo_Turn_IASA (0x800C99F8): attacks use the destination facing,
    /// shield/jump use the current facing, then the dash input is buffered.
    pub(super) fn turn_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        let MotionData::Turn(turn) = &self.core.state_data else {
            panic!("turn data missing")
        };
        let has_turned = turn.has_turned;
        if turn.just_turned {
            self.core.input.pressed.0 |= turn.buffered_buttons.0;
        }
        if !has_turned {
            self.core.physics.facing = -self.core.physics.facing;
        }
        let mut attack_context = context.clone();
        attack_context.facing = self.core.physics.facing;
        let transition = self.first_ground_transition(
            assets,
            &attack_context,
            &[
                P::SpecialSide,
                P::SpecialDown,
                P::SpecialUp,
                P::Grab,
                P::SmashSide,
                P::SmashUp,
                P::SmashDown,
                P::TiltSide,
                P::TiltUp,
                P::TiltDown,
                P::Jab,
            ],
        );
        if transition != T::None {
            return self.apply_ground_transition(assets, transition);
        }
        if !has_turned {
            self.core.physics.facing = -self.core.physics.facing;
        }
        let transition =
            self.first_ground_transition(assets, context, &[P::Shield, P::Taunt, P::Jump]);
        if transition != T::None {
            return self.apply_ground_transition(assets, transition);
        }
        let MotionData::Turn(turn) = &mut self.core.state_data else {
            unreachable!()
        };
        let forward = self.core.input.current.stick.x * turn.facing_after
            >= assets.input.thresholds.dash_smash_stick_threshold;
        // fn_800C9C2C (0x800C9C2C).
        if forward
            && i32::from(self.core.input.horizontal.tilt)
                < assets.input.thresholds.dash_smash_window
        {
            turn.dash_direction = turn.facing_after;
        }
        if turn.just_turned && turn.dash_direction != 0.0 && forward {
            // ftCo_Turn.c:139-144 passes 0: the new dash has no initial
            // attack/escape window. Later stores touch inactive union bytes.
            return self.enter_dash(assets, false);
        }
        turn.buffered_buttons.0 |= self.core.input.pressed.0 & (Buttons::A.0 | Buttons::B.0);
        turn.just_turned = false;
        Ok(())
    }
}
