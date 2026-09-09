//! Shared physics callbacks, preserving the former dispatch operation order.
use crate::fighter::state::PhysicsPhase;
use crate::fighter::*;
use crate::physics::grounded::{step_wait, GroundedParameters};
/// ftData_MotionStateList: ftCo_MS_CapturePulledLw (226), ftCo_MS_CaptureWaitLw (227),
/// ftCo_MS_ThrownB (240).
pub fn capture<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets: _,
        map: _,
        wind,
    } = phase;
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftData_MotionStateList: ftCo_MS_DeadDown (0).
pub fn dead<C: CharacterCallbacks>(_fighter: &mut Fighter<C>, _phase: PhysicsPhase<'_>) {}

/// ftData_MotionStateList: ftCo_MS_Rebirth (12), ftCo_MS_RebirthWait (13).
pub fn revival<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.revival_physics(assets, wind)
}

/// ftData_MotionStateList: ftCo_MS_DownBoundD (191), ftCo_MS_DownWaitD (192).
pub fn down<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.down_physics(assets, map, wind)
}

/// ftData_MotionStateList: ftCo_MS_Catch (212), ftCo_MS_CatchPull (213), ftCo_MS_CatchWait
/// (216).
pub fn catch<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.catch_physics(assets, map, wind)
}

/// ftData_MotionStateList: ftCo_MS_DamageFall (38), ftCo_MS_DamageHi3 (77), ftCo_MS_DamageN1
/// (78), ftCo_MS_DamageN2 (79), ftCo_MS_DamageFlyN (88).
pub fn damage<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.damage_physics(assets, map, wind)
}

/// ftData_MotionStateList: ftCo_MS_Attack11 (44), ftCo_MS_AttackS4S (60), ftCo_MS_PassiveStandB
/// (201), ftCo_MS_ThrowB (220).
pub fn jab<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.jab_physics(assets, map, wind)
}

/// ftData_MotionStateList: ftCo_MS_Wait (14), ftCo_MS_SquatWait (40).
pub fn wait<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    step_wait(
        &mut fighter.physics,
        &fighter.collision.data,
        &GroundedParameters::from_attributes(&fighter.attributes, &assets.common),
        map,
        wind,
    )
}

/// ftData_MotionStateList: ftCo_MS_Turn (18), ftCo_MS_KneeBend (24), ftCo_MS_Squat (39),
/// ftCo_MS_SquatRv (41), ftCo_MS_Landing (42), ftCo_MS_LandingFallSpecial (43),
/// ftCo_MS_AttackHi3 (56), ftCo_MS_GuardOn (178), ftCo_MS_Guard (179), ftCo_MS_GuardOff (180),
/// ftCo_MS_GuardSetOff (181), ftCo_MS_GuardReflect (182), ftCo_MS_EscapeN (235).
pub fn guard_on<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    let params = GroundedParameters::from_attributes(&fighter.attributes, &assets.common);
    crate::physics::grounded::friction_physics(
        &mut fighter.physics,
        &params,
        fighter.collision.data.floor.normal,
        map.floor_speed_scale(&fighter.collision.data),
    );
    crate::physics::grounded::finish_ground_update(
        &mut fighter.physics,
        &fighter.collision.data,
        &params,
        map,
        wind,
    );
}

/// ftData_MotionStateList: ftCo_MS_EscapeF (233), ftCo_MS_EscapeB (234).
pub fn escape<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.escape_physics(map);
    crate::physics::grounded::finish_ground_update(
        &mut fighter.physics,
        &fighter.collision.data,
        &GroundedParameters::from_attributes(&fighter.attributes, &assets.common),
        map,
        wind,
    );
}

/// ftData_MotionStateList: ftCo_MS_TurnRun (19).
pub fn turn_run<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.turn_run_physics(assets);
    crate::physics::grounded::apply_ground_movement(
        &mut fighter.physics,
        fighter.collision.data.floor.normal,
        map.floor_speed_scale(&fighter.collision.data),
    );
    crate::physics::grounded::finish_ground_update(
        &mut fighter.physics,
        &fighter.collision.data,
        &GroundedParameters::from_attributes(&fighter.attributes, &assets.common),
        map,
        wind,
    );
}

/// ftCo_Dash / Run / RunBrake_Phys: shared running movement.
pub fn running<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.running_physics(assets);
    crate::physics::grounded::apply_ground_movement(
        &mut fighter.physics,
        fighter.collision.data.floor.normal,
        map.floor_speed_scale(&fighter.collision.data),
    );
    crate::physics::grounded::finish_ground_update(
        &mut fighter.physics,
        &fighter.collision.data,
        &GroundedParameters::from_attributes(&fighter.attributes, &assets.common),
        map,
        wind,
    );
}

/// ftData_MotionStateList: ftCo_MS_WalkSlow (15), ftCo_MS_WalkMiddle (16), ftCo_MS_WalkFast
/// (17).
pub fn walk<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    let MotionData::Walk(walk) = &mut fighter.state_data else {
        panic!("walk data missing")
    };
    walk.slippery_animation_velocity = crate::physics::grounded::walk_physics(
        &mut fighter.physics,
        &fighter.attributes,
        &assets.movement,
        fighter.input.current.stick.x,
        walk.acceleration_multiplier,
    );
    crate::physics::grounded::apply_ground_movement(
        &mut fighter.physics,
        fighter.collision.data.floor.normal,
        map.floor_speed_scale(&fighter.collision.data),
    );
    crate::physics::grounded::finish_ground_update(
        &mut fighter.physics,
        &fighter.collision.data,
        &GroundedParameters::from_attributes(&fighter.attributes, &assets.common),
        map,
        wind,
    );
}

/// ftData_MotionStateList: ftCo_MS_CliffClimbQuick (255), ftCo_MS_CliffEscapeQuick (259).
pub fn cliff_climb<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter
        .cliff_climb_physics(assets, map)
        .expect("ledge option physics");
    if fighter.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        crate::physics::grounded::finish_ground_update(
            &mut fighter.physics,
            &fighter.collision.data,
            &GroundedParameters::from_attributes(&fighter.attributes, &assets.common),
            map,
            wind,
        );
    } else {
        crate::physics::integrate::integrate_velocity(&mut fighter.physics);
        crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
    }
}

/// ftData_MotionStateList: ftCo_MS_CliffCatch (252), ftCo_MS_CliffWait (253),
/// ftCo_MS_CliffJumpSlow1 (260), ftCo_MS_CliffJumpQuick1 (262).
pub fn cliff_catch<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.ledge_physics(assets, map).expect("ledge physics");
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftData_MotionStateList: ftCo_MS_CliffJumpSlow2 (261), ftCo_MS_CliffJumpQuick2 (263).
pub fn cliff_jump2<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.ledge_jump_physics(assets);
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftData_MotionStateList: ftCo_MS_EscapeAir (236).
pub fn escape_air<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.air_dodge_physics(assets);
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftData_MotionStateList: ftCo_MS_Pass (244).
pub fn pass<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets: _,
        map: _,
        wind,
    } = phase;
    crate::physics::airborne::fall_physics(
        &mut fighter.physics,
        &fighter.attributes.air,
        fighter.input.current.stick.x,
    );
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftCo_JumpAerialF1_Phys (800D7634), ftPr_Init_MotionStateTable[0..5]:
/// Jigglypuff actions 341..345 retain gravity, drift, decay, then integration.
pub fn multi_jump<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.multi_jump_physics(assets);
    fighter.decay_air_knockback(assets);
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftData_MotionStateList: ftCo_MS_FallSpecial (35).
pub fn fall_special<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.special_fall_physics(assets);
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftData_MotionStateList: ftCo_MS_JumpF (25), ftCo_MS_JumpB (26), ftCo_MS_JumpAerialF (27),
/// ftCo_MS_JumpAerialB (28), ftCo_MS_Fall (29), ftCo_MS_FallAerial (32).
pub fn fall<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    assert_eq!(
        fighter.physics.shield_knockback_velocity,
        Vec3::ZERO,
        "air shield knockback decay needs damage physics"
    );
    fighter.airborne_physics(assets);
    fighter.decay_air_knockback(assets);
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}

/// ftData_MotionStateList: ftCo_MS_Entry (322), ftCo_MS_EntryStart (323), ftCo_MS_EntryEnd
/// (324).
pub fn entry<C: CharacterCallbacks>(fighter: &mut Fighter<C>, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.entry_physics(assets.entry);
    crate::physics::integrate::integrate_velocity(&mut fighter.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.physics, None, wind);
}
