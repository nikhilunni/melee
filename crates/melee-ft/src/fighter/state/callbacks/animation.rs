//! Shared animation callbacks, preserving the former dispatch operation order.
use crate::anim::WaitChoice;
use crate::fighter::assets::Result;
use crate::fighter::state::AnimationPhase;
use crate::fighter::*;
/// ftData_MotionStateList: ftCo_MS_DeadDown (0).
pub fn dead(fighter: &mut Fighter, _phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_dead(_phase)
}

/// ftData_MotionStateList: ftCo_MS_Rebirth (12), ftCo_MS_RebirthWait (13).
pub fn revival(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    let choice = fighter.update_idle_animation(assets, rng)?;
    fighter.revival_animation(assets)?;
    Ok(choice)
}

/// ftData_MotionStateList: ftCo_MS_PassiveStandB (201).
pub fn tech_roll(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.change_motion_state(melee_types::CommonMotionState::Wait.into(), assets)?;
    }
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_DownBoundD (191), ftCo_MS_DownWaitD (192).
pub fn down_bound(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.down_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_ThrownB (240).
pub fn thrown(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_thrown(phase)
}

/// ftData_MotionStateList: ftCo_MS_ThrowB (220).
pub fn throw(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    // The linked release runs immediately after this callback in scene order.
    fighter.jab_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_DamageFall (38), ftCo_MS_CatchWait (216),
/// ftCo_MS_CapturePulledLw (226), ftCo_MS_CaptureWaitLw (227).
pub fn capture(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_capture(phase)
}

/// ftData_MotionStateList: ftCo_MS_CatchPull (213).
pub fn catch_pull(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    if fighter.core.commands.grab_release
        || !fighter
            .core
            .animation
            .frames_remaining(&fighter.core.skeleton)
    {
        fighter.enter_catch_wait(assets)?;
    }
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Catch (212).
pub fn catch(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.catch_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_DamageHi3 (77), ftCo_MS_DamageN1 (78), ftCo_MS_DamageN2
/// (79), ftCo_MS_DamageFlyN (88).
pub fn damage(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.damage_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Attack11 (44), ftCo_MS_AttackHi3 (56), ftCo_MS_AttackS4S
/// (60).
pub fn jab(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.jab_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_GuardOn (178), ftCo_MS_Guard (179), ftCo_MS_GuardOff (180),
/// ftCo_MS_GuardSetOff (181), ftCo_MS_GuardReflect (182).
pub fn guard_on(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.shield_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_CliffCatch (252), ftCo_MS_CliffWait (253),
/// ftCo_MS_CliffJumpSlow1 (260), ftCo_MS_CliffJumpSlow2 (261), ftCo_MS_CliffJumpQuick1 (262),
/// ftCo_MS_CliffJumpQuick2 (263).
pub fn cliff_catch(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.ledge_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_EscapeAir (236).
pub fn escape_air(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.air_dodge_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_EscapeF (233), ftCo_MS_EscapeB (234), ftCo_MS_EscapeN (235).
pub fn escape(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.escape_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Squat (39), ftCo_MS_SquatRv (41).
pub fn squat(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.squat_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Turn (18).
pub fn turn(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.turn_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Dash (20).
pub fn dash(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.dash_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Run (21).
pub fn run(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_run(phase)
}

/// ftData_MotionStateList: ftCo_MS_TurnRun (19).
pub fn turn_run(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.turn_run_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_CliffClimbQuick (255), ftCo_MS_CliffEscapeQuick (259).
pub fn cliff_climb(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.cliff_climb_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_RunBrake (23).
pub fn run_brake(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.run_brake_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_WalkSlow (15), ftCo_MS_WalkMiddle (16), ftCo_MS_WalkFast
/// (17).
pub fn walk(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_walk(phase)
}

/// ftData_MotionStateList: ftCo_MS_SquatWait (40).
pub fn squat_wait(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_squat_wait(phase)
}

/// ftData_MotionStateList: ftCo_MS_Entry (322), ftCo_MS_EntryStart (323), ftCo_MS_EntryEnd
/// (324).
pub fn entry(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.entry_animation(assets)?;
    Ok(None)
}

/// ftCo_JumpAerialF1_Anim (800D7590), ftPr_Init_MotionStateTable[0..5]:
/// Jigglypuff actions 341..345 retain the shared animation and turn behavior.
pub fn multi_jump(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.multi_jump_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_KneeBend (24).
pub fn knee_bend(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.knee_bend_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_JumpF (25), ftCo_MS_JumpB (26), ftCo_MS_JumpAerialF (27),
/// ftCo_MS_JumpAerialB (28), ftCo_MS_Pass (244).
pub fn pass(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.jump_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Fall (29), ftCo_MS_FallAerial (32), ftCo_MS_FallSpecial
/// (35).
pub fn fall(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_fall(phase)
}

/// ftData_MotionStateList: ftCo_MS_Landing (42), ftCo_MS_LandingFallSpecial (43).
pub fn landing(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    fighter.landing_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Wait (14).
pub fn wait(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.core.animation_wait(phase)
}

impl FighterCore {
    fn animation_dead(&mut self, _phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        self.death_animation();
        Ok(None)
    }
    fn animation_thrown(&mut self, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        let AnimationPhase { assets, rng: _ } = phase;
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        self.thrown_animation(assets);
        Ok(None)
    }
    fn animation_capture(&mut self, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        let AnimationPhase { assets, rng: _ } = phase;
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        Ok(None)
    }
    fn animation_run(&mut self, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        let AnimationPhase { assets, rng: _ } = phase;
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        self.run_animation();
        Ok(None)
    }
    fn animation_walk(&mut self, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        let AnimationPhase { assets, rng: _ } = phase;
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        self.walk_animation(assets);
        Ok(None)
    }
    fn animation_squat_wait(&mut self, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        let AnimationPhase { assets, rng } = phase;
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        self.update_idle_animation(assets, rng)
    }
    fn animation_fall(&mut self, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        let AnimationPhase { assets, rng: _ } = phase;
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        self.fall_animation(assets)?;
        Ok(None)
    }
    fn animation_wait(&mut self, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
        let AnimationPhase { assets, rng } = phase;
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        self.update_idle_animation(assets, rng)
    }
}

/// ftCo_AttackLw3_Anim, after ordinary animation/charge processing.
pub fn down_tilt(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    fighter.down_tilt_animation(phase.assets)?;
    Ok(None)
}

pub fn rapid_start(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    fighter.rapid_start_animation(phase.assets)?;
    Ok(None)
}
pub fn rapid_loop(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    fighter.rapid_loop_animation(phase.assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_Ottotto (245).
pub fn ottotto(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.teeter_animation(assets)?;
    Ok(None)
}

/// ftData_MotionStateList: ftCo_MS_OttottoWait (246): ftCo_OttottoWait_Anim is empty.
pub fn ottotto_wait(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    let AnimationPhase { assets, rng: _ } = phase;
    fighter.step_animation(assets);
    Ok(None)
}
