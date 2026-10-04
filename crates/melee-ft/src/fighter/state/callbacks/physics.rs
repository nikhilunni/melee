//! Shared physics callbacks, preserving the former dispatch operation order.
use crate::fighter::state::PhysicsPhase;
use crate::fighter::*;
use crate::physics::grounded::{step_wait, GroundedParameters};
/// ftData_MotionStateList: ftCo_MS_CapturePulledLw (226), ftCo_MS_CaptureWaitLw (227),
/// ftCo_MS_ThrownB (240).
pub fn capture(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_capture(phase)
}

/// ftData_MotionStateList: ftCo_MS_DeadDown (0), ftCo_MS_DeadLeft (1), ftCo_MS_DeadRight (2):
/// no retail callback, and nothing left to integrate.
pub fn dead(_fighter: &mut Fighter, _phase: PhysicsPhase<'_>) {}

/// ftData_MotionStateList: ftCo_MS_DeadUpStar (4): no retail callback; Fighter_procUpdate's
/// tail still integrates the flight velocity.
pub fn dead_star(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.core.free_flight_physics(assets, wind)
}

/// ftData_MotionStateList: ftCo_MS_DeadUpFall (6), ftCo_MS_DeadUpFallHitCamera (7),
/// ftCo_MS_DeadUpFallHitCameraFlat (8): ftCo_DeadUpFall_Phys, then procUpdate's tail.
pub fn dead_screen(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.screen_ko_physics(assets, wind)
}

/// ftData_MotionStateList: ftCo_MS_Rebirth (12), ftCo_MS_RebirthWait (13).
pub fn revival(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_revival(phase)
}

/// ftData_MotionStateList: ftCo_MS_DownBoundD (191), ftCo_MS_DownWaitD (192).
pub fn down(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_down(phase)
}

/// ftData_MotionStateList: ftCo_MS_Catch (212), ftCo_MS_CatchPull (213), ftCo_MS_CatchWait
/// (216).
pub fn catch(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_catch(phase)
}

pub fn catch_dash(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter
        .core
        .dash_catch_physics(phase.assets, phase.map, phase.wind)
}

/// ftData_MotionStateList: ftCo_MS_DamageFall (38), ftCo_MS_DamageHi3 (77), ftCo_MS_DamageN1
/// (78), ftCo_MS_DamageN2 (79), ftCo_MS_DamageFlyN (88).
/// ftCo_DamageFall_Phys (80090940), independent of mv.damage hitstun.
pub fn damage_fall(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.damage_fall_physics(phase.assets, phase.wind);
}
pub fn damage(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.damage_physics(assets, map, wind)
}

/// ftData_MotionStateList: ftCo_MS_Attack11 (44), ftCo_MS_AttackS4S (60), ftCo_MS_PassiveStandB
/// (201), ftCo_MS_ThrowB (220).
pub fn jab(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_jab(phase)
}

/// ftCo_ThrowF_Phys (800DD838) and its siblings on the ground: ft_80085004.
/// The airborne arms (ft_80085134, mv.co.throw) are not ported.
pub fn throw(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    assert_eq!(
        fighter.core.physics.ground_or_air,
        melee_types::GroundOrAir::Ground,
        "ftCo_Throw*_Phys: an airborne throw (ft_80085134)"
    );
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.core.throw_physics(assets, map, wind)
}

/// ftData_MotionStateList: ftCo_MS_Wait (14), ftCo_MS_SquatWait (40).
pub fn wait(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_wait(phase)
}

/// ftData_MotionStateList: ftCo_MS_Turn (18), ftCo_MS_KneeBend (24), ftCo_MS_Squat (39),
/// ftCo_MS_SquatRv (41), ftCo_MS_Landing (42), ftCo_MS_LandingFallSpecial (43),
/// ftCo_MS_AttackHi3 (56), ftCo_MS_GuardOn (178), ftCo_MS_Guard (179), ftCo_MS_GuardOff (180),
/// ftCo_MS_GuardSetOff (181), ftCo_MS_GuardReflect (182), ftCo_MS_EscapeN (235).
pub fn guard_on(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_guard_on(phase)
}

/// ftData_MotionStateList: ftCo_MS_EscapeF (233), ftCo_MS_EscapeB (234).
pub fn escape(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_escape(phase)
}

/// ftData_MotionStateList: ftCo_MS_TurnRun (19).
pub fn turn_run(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_turn_run(phase)
}

/// ftCo_Dash / Run / RunBrake_Phys: shared running movement.
pub fn running(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_running(phase)
}

/// ftData_MotionStateList: ftCo_MS_WalkSlow (15), ftCo_MS_WalkMiddle (16), ftCo_MS_WalkFast
/// (17).
pub fn walk(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_walk(phase)
}

/// ftData_MotionStateList: ftCo_MS_CliffClimbQuick (255), ftCo_MS_CliffEscapeQuick (259).
pub fn cliff_climb(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter
        .cliff_climb_physics(assets, map)
        .expect("ledge option physics");
    if fighter.core.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        crate::physics::grounded::finish_ground_update(
            &mut fighter.core.physics,
            &fighter.core.collision.data,
            &GroundedParameters::from_attributes(&fighter.core.attributes, &assets.common),
            map,
            wind,
        );
    } else {
        fighter.core.finish_air_update(assets, wind);
    }
}

/// ftData_MotionStateList: ftCo_MS_CliffCatch (252), ftCo_MS_CliffWait (253),
/// ftCo_MS_CliffJumpSlow1 (260), ftCo_MS_CliffJumpQuick1 (262).
pub fn cliff_catch(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    fighter.ledge_physics(assets, map).expect("ledge physics");
    fighter.core.finish_air_update(assets, wind);
}

/// ftCo_AirCatchHit_Phys (800C4438), the kinds' AirCatchHit rows: the
/// fighter keeps last frame's motion (self_vel = pos_delta, which the
/// article's swing sets) and falls, then procUpdate's tail.
pub fn air_catch_hit(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let c = &mut fighter.core;
    c.physics.self_velocity = c.physics.position_delta;
    let air = &c.attributes.air;
    c.physics.self_velocity.y = crate::physics::airborne::gravity(
        c.physics.self_velocity.y,
        air.gravity,
        air.terminal_velocity,
    );
    c.finish_air_update(phase.assets, phase.wind);
}

/// ftData_MotionStateList: ftCo_MS_CliffJumpSlow2 (261), ftCo_MS_CliffJumpQuick2 (263).
pub fn cliff_jump2(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.ledge_jump_physics(assets);
    fighter.core.finish_air_update(assets, wind);
}

/// ftData_MotionStateList: ftCo_MS_EscapeAir (236).
pub fn escape_air(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.air_dodge_physics(assets);
    // Fighter_procUpdate (8006B82C): residual hit knockback decays after
    // EscapeAir's self-velocity callback, before position integration.
    fighter.core.finish_air_update(assets, wind);
}

/// ft_80084EEC (80084EEC): gravity and air friction, no drift input.
pub fn air_friction(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    use crate::physics::airborne;
    let core = &mut fighter.core;
    let air = &core.attributes.air;
    core.physics.self_velocity.y = airborne::gravity(
        core.physics.self_velocity.y,
        air.gravity,
        air.terminal_velocity,
    );
    core.physics.animation_velocity.x =
        airborne::drift_acceleration(core.physics.self_velocity.x, 0.0, 0.0, air);
    core.finish_air_update(phase.assets, phase.wind);
}

/// ftData_MotionStateList: ftCo_MS_Pass (244).
pub fn pass(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_pass(phase)
}

/// ftCo_JumpAerialF1_Phys (800D7634), ftPr_Init_MotionStateTable[0..5]:
/// Jigglypuff actions 341..345 retain gravity, drift, decay, then integration.
pub fn multi_jump(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.multi_jump_physics(assets);
    fighter.core.finish_air_update(assets, wind);
}

/// ftData_MotionStateList: ftCo_MS_FallSpecial (35).
pub fn fall_special(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    fighter.special_fall_physics(assets);
    fighter.core.finish_air_update(assets, wind);
}

/// ftData_MotionStateList: ftCo_MS_JumpF (25), ftCo_MS_JumpB (26), ftCo_MS_JumpAerialF (27),
/// ftCo_MS_JumpAerialB (28), ftCo_MS_Fall (29), ftCo_MS_FallAerial (32).
pub fn fall(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase {
        assets,
        map: _,
        wind,
    } = phase;
    // Shield recoil decays in finish_air_update, as Fighter_procUpdate does
    // for every airborne state; the invisible-ceiling store can leave a
    // residual y recoil in the air.
    fighter.airborne_physics(assets);
    fighter.core.finish_air_update(assets, wind);
}

/// ftData_MotionStateList: ftCo_MS_Entry (322), ftCo_MS_EntryStart (323), ftCo_MS_EntryEnd
/// (324).
pub fn entry(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter.core.physics_entry(phase)
}

impl FighterCore {
    /// The capture and thrown states have no physics callback of their own;
    /// Fighter_procUpdate's tail still runs. A victim standing on a floor
    /// (fp->ground_or_air == GA_Ground, 0x8006BE48) rides it by mpGetSpeed
    /// before its accessory pins it to the captor again, which is the
    /// position the map proc and the next tick's pin start from.
    fn physics_capture(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            crate::physics::grounded::finish_ground_update(
                &mut self.physics,
                &self.collision.data,
                &GroundedParameters::from_attributes(&self.attributes, &assets.common),
                map,
                wind,
            );
        } else {
            self.finish_air_update(assets, wind);
        }
    }
    fn physics_revival(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase {
            assets,
            map: _,
            wind,
        } = phase;
        self.revival_physics(assets, wind)
    }
    fn physics_down(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        self.down_physics(assets, map, wind)
    }
    fn physics_catch(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        self.catch_physics(assets, map, wind)
    }
    fn physics_jab(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        self.jab_physics(assets, map, wind)
    }
    fn physics_wait(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        step_wait(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        )
    }
    fn physics_guard_on(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        crate::physics::grounded::friction_physics(
            &mut self.physics,
            &params,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        crate::physics::grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &params,
            map,
            wind,
        );
    }
    fn physics_escape(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        self.escape_physics(map);
        crate::physics::grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
    }
    fn physics_turn_run(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        self.turn_run_physics(assets);
        crate::physics::grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        crate::physics::grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
    }
    fn physics_running(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        self.running_physics(assets);
        crate::physics::grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        crate::physics::grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
    }
    fn physics_walk(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase { assets, map, wind } = phase;
        let MotionData::Walk(walk) = &mut self.state_data else {
            panic!("walk data missing")
        };
        walk.slippery_animation_velocity = crate::physics::grounded::walk_physics(
            &mut self.physics,
            &self.attributes,
            &assets.movement,
            self.input.current.stick.x,
            walk.acceleration_multiplier,
        );
        crate::physics::grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        crate::physics::grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
    }
    /// ft_80084DB0 (80084DB0): fast-fall check, gravity or fast fall, drift.
    fn physics_pass(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase {
            assets,
            map: _,
            wind,
        } = phase;
        // Unlike Jump's first physics frame, nothing is skipped here.
        self.apply_fall_gravity(assets);
        self.physics.animation_velocity.x = crate::physics::airborne::drift(
            self.physics.self_velocity.x,
            self.input.current.stick.x,
            &self.attributes.air,
        );
        self.finish_air_update(assets, wind);
    }
    fn physics_entry(&mut self, phase: PhysicsPhase<'_>) {
        let PhysicsPhase {
            assets,
            map: _,
            wind,
        } = phase;
        self.entry_physics(assets.entry);
        self.finish_air_update(assets, wind);
    }
}

/// ftCo_AttackDash_Phys: concrete root-motion/friction implementation.
pub fn dash_attack(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter
        .core
        .dash_attack_physics(phase.assets, phase.map, phase.wind);
}

/// ftData_MotionStateList: ftCo_MS_Ottotto (245), ftCo_MS_OttottoWait (246).
/// ftCo_Ottotto_Phys is empty; Fighter_procUpdate still runs its grounded tail
/// (ground knockback decay, velocity and overlap-nudge integration, moving
/// floor, wind).
pub fn ottotto(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, map, wind } = phase;
    crate::physics::grounded::finish_ground_update(
        &mut fighter.core.physics,
        &fighter.core.collision.data,
        &GroundedParameters::from_attributes(&fighter.core.attributes, &assets.common),
        map,
        wind,
    );
}

/// ftData_MotionStateList: ftCo_MS_LightThrowDash (98).
pub fn dash_throw(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    fighter
        .core
        .dash_throw_physics(phase.assets, phase.map, phase.wind)
}

/// ftData_MotionStateList: ftCo_MS_SwordSwing1..SwordSwingDash (120..123),
/// ftCo_SwordSwing_Phys -> ftCo_800CD278: ft_80084FA8 but for the dash
/// swing's ft_80085030 at PlCo +420 of the ground friction.
pub fn swing(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let MotionData::Swing(swing) = fighter.core.state_data else {
        panic!("swing scratch missing")
    };
    if swing.input == crate::fighter::item_swing::SwingInput::Dash {
        return fighter
            .core
            .dash_swing_physics(phase.assets, phase.map, phase.wind);
    }
    fighter.core.physics_jab(phase)
}
