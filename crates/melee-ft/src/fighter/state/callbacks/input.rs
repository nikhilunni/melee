//! Shared IASA callbacks, including the existing float-hook ordering.
use crate::fighter::state::InputPhase;
use crate::fighter::*;
use crate::input::{wait_iasa, WaitContext, WaitTransition};
/// ftData_MotionStateList: ftCo_MS_DeadDown (0), ftCo_MS_Rebirth (12), ftCo_MS_RebirthWait
/// (13), ftCo_MS_DownBoundD (191), ftCo_MS_DownWaitD (192), ftCo_MS_PassiveStandB (201),
/// ftCo_MS_Catch (212), ftCo_MS_CatchPull (213), ftCo_MS_CatchWait (216), ftCo_MS_ThrowB (220),
/// ftCo_MS_CapturePulledLw (226), ftCo_MS_CaptureWaitLw (227), ftCo_MS_ThrownB (240).
/// ftCo_AppealS_IASA: only the retail attack/defense prefix after script unlock.
pub fn appeal(fighter: &mut Fighter, phase: InputPhase<'_>) {
    use crate::input::WaitPredicate as P;
    if !fighter.commands.allow_interrupt {
        return;
    }
    let context = WaitContext {
        facing: fighter.physics.facing,
        specials_available: fighter.capabilities.specials,
        shield_health: fighter.status.shield_health,
        ..WaitContext::default()
    };
    let transition = fighter.first_ground_transition(
        phase.assets,
        &context,
        &[
            P::SpecialSide,
            P::SpecialUp,
            P::SpecialNeutral,
            P::SpecialDown,
            P::Grab,
            P::SmashSide,
            P::SmashUp,
            P::SmashDown,
            P::TiltSide,
            P::TiltUp,
            P::TiltDown,
            P::Jab,
            P::Escape,
            P::Shield,
        ],
    );
    fighter
        .apply_ground_transition(phase.assets, transition)
        .expect("taunt IASA");
}

pub fn catch(_fighter: &mut Fighter, _phase: InputPhase<'_>) {}

/// ftData_MotionStateList: ftCo_MS_AttackHi3 (56), ftCo_MS_AttackS4S (60).
/// ftData_MotionStateList: ftCo_MS_AttackDash (50).
pub fn dash_attack(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter
        .dash_attack_input(assets, &context)
        .expect("dash attack IASA");
}

/// ftData_MotionStateList: ftCo_MS_AttackS4 (60..64).
pub fn forward_smash(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter
        .forward_smash_input(assets, &context)
        .expect("forward smash IASA");
}

pub fn tilt(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter.tilt_input(assets, &context).expect("tilt IASA");
}

/// ftData_MotionStateList: ftCo_MS_DamageFall (38), ftCo_MS_DamageHi3 (77), ftCo_MS_DamageN1
/// (78), ftCo_MS_DamageN2 (79), ftCo_MS_DamageFlyN (88).
/// ftCo_DamageFall_IASA (80090828), independent of the retained union owner.
pub fn damage_fall(fighter: &mut Fighter, phase: InputPhase<'_>) {
    fighter
        .damage_fall_input(phase.assets)
        .expect("DamageFall IASA");
}
pub fn damage(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter.damage_input(assets, &context).expect("damage IASA");
}

/// ftData_MotionStateList: ftCo_MS_Attack11 (44).
pub fn jab(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter.jab_input(assets, &context).expect("jab IASA");
}

/// ftData_MotionStateList: ftCo_MS_GuardOn (178), ftCo_MS_Guard (179), ftCo_MS_GuardOff (180),
/// ftCo_MS_GuardSetOff (181), ftCo_MS_GuardReflect (182).
pub fn guard_on(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter.shield_input(assets, &context).expect("shield IASA");
}

/// ftData_MotionStateList: ftCo_MS_TurnRun (19).
pub fn turn_run(fighter: &mut Fighter, phase: InputPhase<'_>) {
    // ftCo_TurnRun_IASA (800C9ED8): bl fn_800CAF78 at 800C9EE4.
    fighter
        .try_running_jump(phase.assets)
        .expect("TurnRun jump cancel");
}

/// ftData_MotionStateList: ftCo_MS_CliffClimbQuick (255), ftCo_MS_CliffEscapeQuick (259).
pub fn cliff_climb(_fighter: &mut Fighter, _phase: InputPhase<'_>) {}

/// ftData_MotionStateList: ftCo_MS_CliffCatch (252), ftCo_MS_CliffJumpSlow1 (260),
/// ftCo_MS_CliffJumpSlow2 (261), ftCo_MS_CliffJumpQuick1 (262), ftCo_MS_CliffJumpQuick2 (263).
pub fn cliff_catch(_fighter: &mut Fighter, _phase: InputPhase<'_>) {}

/// ftData_MotionStateList: ftCo_MS_CliffWait (253).
pub fn cliff_wait(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    fighter.ledge_input(assets).expect("ledge input");
}

/// ftData_MotionStateList: ftCo_MS_EscapeAir (236).
pub fn escape_air(fighter: &mut Fighter, _phase: InputPhase<'_>) {
    fighter.air_dodge_input();
}

/// ftData_MotionStateList: ftCo_MS_EscapeF (233), ftCo_MS_EscapeB (234).
pub fn escape(fighter: &mut Fighter, _phase: InputPhase<'_>) {
    fighter.core.input_escape(_phase)
}

/// ftData_MotionStateList: ftCo_MS_EscapeN (235).
pub fn escape_n(_fighter: &mut Fighter, _phase: InputPhase<'_>) {}

/// ftData_MotionStateList: ftCo_MS_KneeBend (24).
pub fn knee_bend(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter
        .knee_bend_input(phase.assets, &context)
        .expect("KneeBend IASA");
}

/// ftData_MotionStateList: ftCo_MS_Squat (39), ftCo_MS_SquatWait (40), ftCo_MS_SquatRv (41).
pub fn squat(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter
        .squat_input(assets, &context)
        .expect("squat transition");
}

/// ftData_MotionStateList: ftCo_MS_Turn (18).
pub fn turn(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter
        .turn_input(assets, &context)
        .expect("turn transition");
}

/// ftData_MotionStateList: ftCo_MS_Dash (20).
pub fn dash(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter
        .dash_input(assets, &context)
        .expect("dash transition");
}

/// ftData_MotionStateList: ftCo_MS_Run (21).
pub fn run(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter.run_input(assets, &context).expect("run transition");
}

/// ftData_MotionStateList: ftCo_MS_RunBrake (23).
pub fn run_brake(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    fighter.run_brake_input(assets).expect("brake transition");
}

/// ftData_MotionStateList: ftCo_MS_WalkSlow (15), ftCo_MS_WalkMiddle (16), ftCo_MS_WalkFast
/// (17).
pub fn walk(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    fighter
        .walk_input(assets, &context)
        .expect("walk transition");
}

/// ftData_MotionStateList: ftCo_MS_JumpF (25), ftCo_MS_JumpB (26), ftCo_MS_JumpAerialF (27),
/// ftCo_MS_JumpAerialB (28), ftCo_MS_Fall (29), ftCo_MS_FallAerial (32), ftCo_MS_Pass (244).
pub fn aerial(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let transition = crate::fighter::fall::iasa_with_jump(
        &fighter.core.input,
        &assets.input,
        fighter.aerial_jump_requested(assets),
        true,
        |phase| {
            let enabled = match &fighter.core.state_data {
                MotionData::Jump(jump) => jump.physics_started,
                MotionData::JumpAerial { .. } => {
                    phase == crate::fighter::FloatInputPhase::BeforeAerialJump
                        || fighter.core.commands.variables[0] != 0
                }
                _ => true,
            };
            if enabled {
                fighter.character.check_float_input(
                    &fighter.core.input,
                    assets,
                    fighter.core.physics.self_velocity.y,
                    phase,
                );
            }
        },
    );
    match transition {
        WaitTransition::None => {}
        WaitTransition::Special => fighter.enter_buffered_special(assets, true),
        WaitTransition::Attack => {
            (fighter.character.table().enter_aerial)(fighter, assets).expect("aerial attack")
        }
        WaitTransition::Jump => fighter.enter_aerial_jump(assets).expect("aerial jump"),
        WaitTransition::Escape => fighter.enter_air_dodge(assets).expect("air dodge"),
        _ => unimplemented!("ftCo_Fall.c:132-149 / ftCo_Jump.c:173-189: aerial {transition:?}"),
    }
}

/// ftData_MotionStateList: ftCo_MS_RebirthWait (13).
pub fn revival(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    fighter
        .revival_input(assets)
        .expect("revival platform exit");
}

/// ftData_MotionStateList: ftCo_MS_FallSpecial (35).
pub fn fall_special(fighter: &mut Fighter, _phase: InputPhase<'_>) {
    fighter.core.input_fall_special(_phase)
}

/// ftData_MotionStateList: ftCo_MS_Entry (322), ftCo_MS_EntryStart (323), ftCo_MS_EntryEnd
/// (324).
pub fn entry(_fighter: &mut Fighter, _phase: InputPhase<'_>) {}

/// ftData_MotionStateList: ftCo_MS_Wait (14).
pub fn wait(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    let transition = wait_iasa(&fighter.core.input, &assets.input, &context);
    fighter
        .apply_ground_transition(assets, transition)
        .expect("grounded transition");
}

/// ftData_MotionStateList: ftCo_MS_Landing (42), ftCo_MS_LandingFallSpecial (43).
pub fn landing(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        specials_available: fighter.core.capabilities.specials,
        shield_health: fighter.core.status.shield_health,
        ..WaitContext::default()
    };
    let MotionData::Landing {
        allow_interrupt, ..
    } = fighter.core.state_data
    else {
        panic!("landing data missing")
    };
    let transition = crate::fighter::landing::iasa(
        &fighter.core.input,
        &assets.input,
        &context,
        fighter.core.animation.frame,
        fighter.core.animation.speed,
        fighter.core.attributes.landing.normal_landing_lag,
        allow_interrupt,
    );
    if transition == WaitTransition::Squat {
        fighter
            .enter_landing_squat(assets)
            .expect("landing squat hold");
    } else {
        fighter
            .apply_ground_transition(assets, transition)
            .expect("grounded transition");
    }
}

impl FighterCore {
    fn input_escape(&mut self, _phase: InputPhase<'_>) {
        // ftCo_8009563C (8009563C), ftCo_ItemThrow.c:261-263.
        if let MotionData::Escape(escape) = &mut self.state_data {
            if escape.interrupt_frames != 0 {
                escape.interrupt_frames -= 1;
            }
        }
    }
    fn input_fall_special(&mut self, _phase: InputPhase<'_>) {
        // ftCo_FallSpecial_IASA (80096AF4): item/parasol predicates are
        // excluded by require_supported; air-dodge entry consumed all jumps.
        if i32::from(self.physics.jumps_used) < self.attributes.jumping.max_jumps {
            unimplemented!("ftCo_FallSpecial.c:96-100: special fall with remaining aerial jumps");
        }
    }
}

/// ftCo_AttackLw3_IASA.
pub fn down_tilt(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let context = WaitContext {
        facing: fighter.core.physics.facing,
        ..WaitContext::default()
    };
    fighter
        .down_tilt_input(phase.assets, &context)
        .expect("down tilt IASA");
}

/// ftCo_Attack100Loop_IASA: both A edges keep the rapid attack alive.
pub fn rapid_loop(fighter: &mut Fighter, _phase: InputPhase<'_>) {
    let pressed = (fighter.core.input.pressed | fighter.core.input.released)
        .intersects(crate::input::Buttons::A);
    let MotionData::RapidJab(rapid) = &mut fighter.core.state_data else {
        panic!("rapid jab scratch")
    };
    rapid.edge_pressed |= pressed;
}

/// ftData_MotionStateList: ftCo_MS_Ottotto (245), ftCo_MS_OttottoWait (246).
pub fn ottotto(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let InputPhase { assets } = phase;
    fighter.teeter_input(assets).expect("teeter input");
}
