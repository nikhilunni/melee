//! Body-grab startup, ftCo_Catch.c. Linked capture is an explicit proc boundary.
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::CommonMotionState as S;

impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_800D8C54 (800D8C54): Catch begins at frame zero without an immediate step.
    pub(super) fn enter_catch(&mut self, assets: &FighterAssets) -> Result<()> {
        self.character.catch_variant();
        self.physics.animation_velocity = Vec3::ZERO;
        self.change_motion_state(S::Catch, assets)?;
        self.state_data = MotionData::Catch;
        Ok(())
    }

    /// ftCo_Catch_Anim (800D8CC8): item/tether callbacks are character hooks.
    pub(super) fn catch_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.animation.frames_remaining(&self.skeleton) {
            self.change_motion_state(S::Wait, assets)?;
        }
        Ok(())
    }

    /// ftCo_Catch_Phys (800D8D88): separate multiplier product; no fused sites.
    pub(super) fn catch_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Vec3,
    ) {
        use crate::physics::{
            friction::friction_acceleration,
            grounded::{self, GroundedParameters},
        };
        self.physics.ground_acceleration = friction_acceleration(
            self.physics.ground_velocity,
            assets.grab_friction_multiplier * self.attributes.ground.ground_friction,
        );
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
    }

    /// ftCo_Catch_Coll (800D8E08) -> ft_800841B8: departure during startup.
    pub(super) fn catch_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        use crate::collision::ground::{map_ground_action, WaitGroundResult};
        if map_ground_action(
            &mut self.physics,
            &mut self.collision,
            map,
            &mut self.skeleton,
            self.animation.root,
            self.input.current.stick.x,
        ) == WaitGroundResult::EnterFall
        {
            self.leave_ground();
            self.change_motion_state(S::Fall, assets)?;
        }
        Ok(())
    }
}
