//! ftCo_StopWall (ftCo_StopWall.c): a dash or run into a wall.
use super::state::{AnimationPhase, CollisionPhase, PhysicsPhase};
use super::{
    assets::{FighterAssets, Result},
    Fighter,
};
use crate::anim::WaitChoice;
use crate::collision::{air, ecb::EcbPose, ground};
use melee_mp::CollMap;
use melee_types::mp::collide;
use melee_types::CommonMotionState as S;

impl Fighter {
    /// ftCo_8009EDA4 (8009EDA4, ftCo_StopWall.c:16-30): after a supported
    /// Dash/Run ground pass (ft_800844EC), a wall hugged on the facing side at
    /// more than walking speed stops the fighter.
    pub(crate) fn try_stop_at_wall(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<bool> {
        let facing = self.core.physics.facing;
        let env = self.core.collision.data.env_flags as u32;
        let hugged = (facing == -1.0 && env & collide::RIGHT_WALL_HUG != 0)
            || (facing == 1.0 && env & collide::LEFT_WALL_HUG != 0);
        if hugged
            && gekko_math::msl::fabsf(self.core.physics.ground_velocity)
                > self.core.attributes.walking.walk_max_vel
        {
            self.enter_stop_wall(assets, map)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// ftCo_8009EE30 (8009EE30..8009EF00).
    fn enter_stop_wall(&mut self, assets: &FighterAssets, map: &mut CollMap) -> Result<()> {
        let cd = &self.core.collision.data;
        // 8009EE54: rlwinm. 0xFC0 (Collide_RightWallMask) picks the ECB side.
        let side = if cd.env_flags as u32 & collide::RIGHT_WALL_MASK != 0 {
            cd.ecb.left
        } else {
            cd.ecb.right
        };
        // ftKb_SpecialN_800F1F1C is a Kirby-only effect at `side`; no scoped
        // kind spawns it.
        self.change_motion_state(S::StopWall.into(), assets)?;
        let trans = self
            .core
            .animation
            .root_motion
            .as_ref()
            .expect("StopWall TransN")
            .primary_history
            .position;
        // retail 8009EED4 / 8009EED8: fadds, then fnmsubs with the negated facing.
        let p = &mut self.core.physics;
        p.position.x = gekko_math::fma::fnmsubs(trans.z, -p.facing, p.position.x + side.x);
        self.stop_wall_ground(assets, map)?;
        self.core.clear_movement(); // ftCommon_8007E2FC
        self.core
            .skeleton
            .set_translate(self.core.animation.root, &self.core.physics.position);
        Ok(())
    }

    /// ft_800843FC (800843FC, ft_081B.c:1100-1127): mpColl_8004B5C4 (ECB
    /// load 9, teetering), then Ottotto at an edge or Fall.
    fn stop_wall_ground(&mut self, assets: &FighterAssets, map: &mut CollMap) -> Result<()> {
        let core = &mut self.core;
        let pose = EcbPose::read(&mut core.skeleton, core.animation.root, &core.collision.data);
        match ground::collide_stop_wall(
            &mut core.physics,
            &mut core.collision,
            map,
            &pose,
            core.input.current.stick.x,
        ) {
            ground::WaitGroundResult::Supported => Ok(()),
            ground::WaitGroundResult::EnterTeeter => self.enter_teeter(assets),
            ground::WaitGroundResult::EnterFall => {
                self.leave_ground();
                self.change_motion_state(S::Fall.into(), assets)
            }
        }
    }
}

/// ftCo_StopWall_Anim (8009EF04): Wait when the animation ends (ft_8008A2BC).
pub fn animation(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    if !fighter.animation.frames_remaining(&fighter.skeleton) {
        fighter.change_motion_state(S::Wait.into(), phase.assets)?;
    }
    Ok(None)
}

/// ftCo_StopWall_Phys (8009EF44) is empty; Fighter_procUpdate still runs its
/// grounded tail.
pub fn physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    super::state::callbacks::physics::ottotto(fighter, phase);
}

/// ftCo_StopWall_Coll (8009EF48) -> ft_800843FC, inside Fighter_procMap.
pub fn collision(fighter: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    let assets = phase.assets.expect("StopWall collision assets");
    let core = &mut fighter.core;
    air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    fighter.stop_wall_ground(assets, phase.map)?;
    fighter
        .core
        .skeleton
        .set_translate(fighter.core.animation.root, &fighter.core.physics.position);
    Ok(())
}
