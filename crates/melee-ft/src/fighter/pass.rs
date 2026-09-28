//! Deliberately leaving a soft platform: ftCommon/ftCo_Pass.c.
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use melee_types::CommonMotionState;

impl Fighter {
    /// ftCo_8009A134 (0x8009A134, ftCo_Pass.c:64): when the supporting line
    /// is a platform (mpColl_IsOnPlatform), skip it for floor queries and
    /// report true. Up-specials use this to launch through soft platforms;
    /// the next Fighter_ChangeMotionState clears the skip (fighter.c:1080).
    /// mpColl_IsOnPlatform re-reads the line's flags from the map: a
    /// dynamic platform's bit (mpJointUpdateDynamics, Pokemon Stadium's
    /// forms) is not in the floor flags cached at contact.
    pub fn skip_platform_floor(&mut self, map: &melee_mp::CollMap) -> bool {
        let cd = &mut self.core.collision.data;
        if !map.is_on_platform(cd) {
            return false;
        }
        melee_mp::update_floor_skip(cd);
        true
    }
    /// ftCo_8009A228 (0x8009A228): leave ground, clamp drift, attach Pass,
    /// then skip the supporting line. Fighter_ChangeMotionState clears that
    /// skip on the next transition (fighter.c:1080), including Fall or Landing.
    pub fn enter_pass(&mut self, assets: &FighterAssets) -> Result<()> {
        // Pass writes no scratch: the union keeps the source's second word
        // (SquatWait's drop timer, Guard's smoothed tilt magnitude).
        let retained_drop_timer = self.retained_drop_timer();
        self.leave_ground();
        let maximum = self.core.attributes.air.air_drift_max;
        self.core.physics.self_velocity.x =
            self.core.physics.self_velocity.x.clamp(-maximum, maximum);
        self.core.physics.self_velocity.y = assets.movement.platform_drop_velocity;
        self.change_motion_state(CommonMotionState::Pass.into(), assets)?;
        melee_mp::update_floor_skip(&mut self.core.collision.data);
        self.core.input.vertical.tilt = 0xFE;
        self.core.state_data = MotionData::Pass {
            retained_drop_timer,
        };
        Ok(())
    }
}
