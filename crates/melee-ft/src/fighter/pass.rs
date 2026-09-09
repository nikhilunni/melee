//! Deliberately leaving a soft platform: ftCommon/ftCo_Pass.c.
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use melee_types::CommonMotionState;

impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_8009A228 (0x8009A228): leave ground, clamp drift, attach Pass,
    /// then skip the supporting line. Fighter_ChangeMotionState clears that
    /// skip on the next transition (fighter.c:1080), including Fall or Landing.
    pub(super) fn enter_pass(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Squat(squat) = &self.state_data else {
            panic!("Pass entry requires crouch scratch");
        };
        let retained_drop_timer = squat.platform_drop_timer;
        self.leave_ground();
        let maximum = self.attributes.air.air_drift_max;
        self.physics.self_velocity.x = self.physics.self_velocity.x.clamp(-maximum, maximum);
        self.physics.self_velocity.y = assets.movement.platform_drop_velocity;
        self.change_motion_state(CommonMotionState::Pass, assets)?;
        melee_mp::update_floor_skip(&mut self.collision.data);
        self.input.vertical.tilt = 0xFE;
        self.state_data = MotionData::Pass {
            retained_drop_timer,
        };
        Ok(())
    }
}
