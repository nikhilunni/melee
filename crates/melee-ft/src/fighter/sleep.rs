//! ftCo_MS_Sleep (ftcolanim.c): the inactive form of a transforming
//! character. Player_80031AD0 creates Zelda's and Sheik's other form beside
//! the one in play; it waits here, hidden and outside every proc, until a
//! transformation (ftCommon_8007EFC8) swaps the two.
use super::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, CameraPhase, CollisionPhase, InputPhase, PhysicsPhase},
    Fighter,
};
use crate::anim::WaitChoice;
use melee_types::CommonMotionState;

impl Fighter {
    /// ftCo_800BFD04 (800BFD04): Sleep, invisible, the camera subject
    /// inactive and every proc suppressed (x221F_b3). x221E_b1/b2, x2219_b1
    /// and x221F_b1 only gate procs x221F_b3 already suppresses.
    pub fn enter_sleep(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state(CommonMotionState::Sleep.into(), assets)?;
        self.core.effect_state.invisible = true;
        self.core.camera.state = melee_cm::SubjectState::Inactive;
        self.core.status.disabled = true;
        Ok(())
    }
}

/// ftCo_Sleep_Anim (800BFE6C).
pub(crate) fn animation(_: &mut Fighter, _: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    Ok(None)
}
/// ftCo_Sleep_IASA (800BFE70).
pub(crate) fn input(_: &mut Fighter, _: InputPhase<'_>) {}
/// ftData_MotionStateList[11] has no physics callback.
pub(crate) fn physics(_: &mut Fighter, _: PhysicsPhase<'_>) {}
/// ftData_MotionStateList[11] has no collision callback.
pub(crate) fn collision(_: &mut Fighter, _: CollisionPhase<'_>) -> Result<()> {
    Ok(())
}
/// ftData_MotionStateList[11] has no camera callback.
pub(crate) fn camera(_: &mut Fighter, _: CameraPhase<'_>) {}
