//! Shared map callbacks; each row supplies its ground/air collision routine.
use crate::collision::{
    air,
    ground::{map_escape, map_ground_action, map_wait, EnvironmentCollision, WaitGroundResult},
};
use crate::fighter::assets::{FighterAssets, Result};
use crate::fighter::state::CollisionPhase;
use crate::fighter::*;
use crate::physics::FighterPhysics;
use hsd_anim::jobj::{JObjId, JObjTree};
use melee_mp::CollMap;

type GroundCollision = fn(
    &mut FighterPhysics,
    &mut EnvironmentCollision,
    &mut CollMap,
    &mut JObjTree,
    JObjId,
    f32,
) -> WaitGroundResult;
type AirCollision = fn(
    &mut FighterPhysics,
    &mut EnvironmentCollision,
    &mut CollMap,
    &mut JObjTree,
    JObjId,
    bool,
) -> bool;

/// ft_80084280 / ft_800844EC / ft_80083F88: preserve the old grounded API's
/// departure assertion when the caller has not supplied motion assets.
fn finish_ground<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    assets: Option<&FighterAssets>,
    map: &mut CollMap,
    collide: GroundCollision,
    running: bool,
) -> Result<()> {
    match collide(
        &mut fighter.core.physics,
        &mut fighter.core.collision,
        map,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
        fighter.core.input.current.stick.x,
    ) {
        WaitGroundResult::Supported => {
            if running {
                // ft_800844EC -> ftCo_8009EDA4 (ftCo_StopWall.c:16-30).
                let wall = if fighter.core.physics.facing < 0.0 {
                    melee_types::mp::collide::RIGHT_WALL_HUG
                } else {
                    melee_types::mp::collide::LEFT_WALL_HUG
                };
                if fighter.core.collision.data.env_flags as u32 & wall != 0
                    && gekko_math::msl::fabsf(fighter.core.physics.ground_velocity)
                        > fighter.core.attributes.walking.walk_max_vel
                {
                    unimplemented!("ftCo_StopWall.c:25-26: running wall impact -> StopWall");
                }
            }
        }
        WaitGroundResult::EnterFall => {
            let assets = assets.expect("ground departure needs proc_map_with_assets");
            fighter.leave_ground();
            fighter.change_motion_state(melee_types::CommonMotionState::Fall, assets)?;
        }
        WaitGroundResult::EnterTeeter => unimplemented!("ft_081B.c:1092: Wait -> Ottotto"),
    }
    Ok(())
}

/// ftCo_Fall_Coll / ftCo_Pass_Coll / ftCo_Jump_Coll: ledge grab precedes StopCeil.
fn fall_collision<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    assets: &FighterAssets,
    map: &mut CollMap,
    collide: AirCollision,
    special_landing: bool,
    stop_at_ceiling: bool,
) -> Result<()> {
    air::begin_map(
        &fighter.core.physics,
        &mut fighter.core.collision,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
    );
    if collide(
        &mut fighter.core.physics,
        &mut fighter.core.collision,
        map,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
        fighter.core.status.ledge_cooldown == 0,
    ) {
        if special_landing {
            fighter.land_from_special_fall(assets)?;
        } else if fighter.core.physics.self_velocity.y > assets.soft_landing_speed {
            fighter.land();
            fighter.change_motion_state(melee_types::CommonMotionState::Wait, assets)?;
        } else {
            fighter.enter_landing(assets)?;
        }
    } else if fighter.try_grab_ledge(assets, map)? {
        // ft_800835B0: grabbing precedes the ceiling check.
    } else if stop_at_ceiling
        && fighter.core.collision.data.env_flags as u32 & melee_types::mp::collide::CEILING_HUG != 0
    {
        unimplemented!("ft_081B.c:792-803 / ftCo_StopCeil.c:16-22: jump ceiling impact");
    }
    Ok(())
}
/// ftData_MotionStateList: ftCo_MS_Rebirth (12), ftCo_MS_RebirthWait (13).
pub fn revival<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    fighter.core.collision_revival(phase)
}

/// ftData_MotionStateList: ftCo_MS_DeadDown (0), ftCo_MS_ThrownB (240).
pub fn thrown<C: CharacterCallbacks>(
    _fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    _fighter.core.collision_thrown(phase)
}

/// ftData_MotionStateList: ftCo_MS_CapturePulledLw (226), ftCo_MS_CaptureWaitLw (227).
pub fn capture<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fighter.capture_collision(assets, map)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_DownBoundD (191), ftCo_MS_DownWaitD (192), ftCo_MS_Catch
/// (212), ftCo_MS_CatchPull (213), ftCo_MS_CatchWait (216), ftCo_MS_ThrowB (220).
pub fn catch<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fighter.catch_collision(assets, map)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_DamageFall (38), ftCo_MS_DamageHi3 (77), ftCo_MS_DamageN1
/// (78), ftCo_MS_DamageN2 (79), ftCo_MS_DamageFlyN (88).
pub fn damage<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fighter.damage_collision(assets, map)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_Entry (322), ftCo_MS_EntryStart (323), ftCo_MS_EntryEnd
/// (324).
pub fn entry<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    fighter.core.collision_entry(phase)
}

/// ftData_MotionStateList: ftCo_MS_TurnRun (19).
pub fn turn_run<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fighter.turn_run_collision(assets, map)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_CliffClimbQuick (255), ftCo_MS_CliffEscapeQuick (259).
pub fn cliff_climb<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fighter.ledge_collision(assets, map)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_CliffCatch (252), ftCo_MS_CliffWait (253),
/// ftCo_MS_CliffJumpSlow1 (260), ftCo_MS_CliffJumpQuick1 (262).
pub fn cliff_catch<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");

    fighter.ledge_collision(assets, map)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_EscapeAir (236).
pub fn escape_air<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");

    air::begin_map(
        &fighter.core.physics,
        &mut fighter.core.collision,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
    );
    if air::collide_air_dodge(
        &mut fighter.core.physics,
        &mut fighter.core.collision,
        map,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
    ) {
        fighter.enter_special_landing(assets, false, assets.air_dodge.landing_lag)?;
    };
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_Wait (14), ftCo_MS_WalkSlow (15), ftCo_MS_WalkMiddle (16),
/// ftCo_MS_WalkFast (17), ftCo_MS_RunBrake (23), ftCo_MS_Landing (42),
/// ftCo_MS_LandingFallSpecial (43).
pub fn ground_wait<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    finish_ground(fighter, assets, map, map_wait, false)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_Turn (18), ftCo_MS_KneeBend (24), ftCo_MS_Squat (39),
/// ftCo_MS_SquatWait (40), ftCo_MS_SquatRv (41), ftCo_MS_GuardOn (178), ftCo_MS_Guard (179),
/// ftCo_MS_GuardOff (180), ftCo_MS_GuardReflect (182).
pub fn ground_action<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    finish_ground(fighter, assets, map, map_ground_action, false)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_Attack11 (44), ftCo_MS_AttackHi3 (56), ftCo_MS_AttackS4S
/// (60), ftCo_MS_PassiveStandB (201), ftCo_MS_EscapeF (233), ftCo_MS_EscapeB (234),
/// ftCo_MS_EscapeN (235).
pub fn escape<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    finish_ground(fighter, assets, map, map_escape, false)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_Dash (20), ftCo_MS_Run (21).
pub fn running<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    finish_ground(fighter, assets, map, map_ground_action, true)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_GuardSetOff (181).
pub fn guard_set_off<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let collide = if fighter.core.shield.allow_sdi {
        map_escape
    } else {
        map_ground_action
    };
    finish_ground(fighter, assets, map, collide, false)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_Fall (29), ftCo_MS_FallAerial (32).
pub fn fall<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fall_collision(fighter, assets, map, air::collide_fall, false, false)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_Pass (244).
pub fn pass<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fall_collision(fighter, assets, map, air::collide_pass, false, false)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_FallSpecial (35).
pub fn fall_special<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fall_collision(fighter, assets, map, air::collide_fall, true, false)?;
    Ok(())
}

/// ftData_MotionStateList: ftCo_MS_JumpF (25), ftCo_MS_JumpB (26), ftCo_MS_JumpAerialF (27),
/// ftCo_MS_JumpAerialB (28), ftCo_MS_CliffJumpSlow2 (261), ftCo_MS_CliffJumpQuick2 (263).
pub fn jump<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    phase: CollisionPhase<'_>,
) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("airborne map needs proc_map_with_assets");
    fall_collision(fighter, assets, map, air::collide_fall, false, true)?;
    Ok(())
}

impl FighterCore {
    fn collision_revival(&mut self, phase: CollisionPhase<'_>) -> Result<()> {
        let CollisionPhase { assets, map } = phase;
        let _assets = assets.expect("airborne map needs proc_map_with_assets");

        air::begin_map(
            &self.physics,
            &mut self.collision,
            &mut self.skeleton,
            self.animation.root,
        );
        let cd = &mut self.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = self.physics.position;
        let pose =
            crate::collision::ecb::EcbPose::read(&mut self.skeleton, self.animation.root, cd);
        if self.motion_state.id == melee_types::CommonMotionState::Rebirth {
            map.air_collide_stay_ecb5(cd, Some(&|i| pose.position(i)));
        } else if map.air_collide_ecb5(cd, Some(&|i| pose.position(i))) {
            unimplemented!("ftCoD5A30: revival platform reaches floor");
        }
        self.physics.position = cd.cur_pos;
        Ok(())
    }
    fn collision_thrown(&mut self, phase: CollisionPhase<'_>) -> Result<()> {
        let CollisionPhase { assets, map: _ } = phase;
        assert!(assets.is_some(), "airborne map needs proc_map_with_assets");
        Ok(())
    }
    fn collision_entry(&mut self, phase: CollisionPhase<'_>) -> Result<()> {
        let CollisionPhase { assets, map } = phase;
        let _assets = assets.expect("airborne map needs proc_map_with_assets");

        air::begin_map(
            &self.physics,
            &mut self.collision,
            &mut self.skeleton,
            self.animation.root,
        );
        if self.motion_state.id != melee_types::CommonMotionState::Entry {
            let MotionData::Entry(entry) = &self.state_data else {
                panic!("entry data missing")
            };
            let was_airborne = self.physics.ground_or_air == melee_types::GroundOrAir::Air;
            let supported = air::collide_entry(
                &mut self.physics,
                &mut self.collision,
                map,
                entry.collision_box,
            );
            if was_airborne && supported {
                self.land();
            } else if !was_airborne && !supported {
                self.leave_ground();
            }
        }
        self.skeleton
            .set_translate(self.animation.root, &self.physics.position);
        Ok(())
    }
}
