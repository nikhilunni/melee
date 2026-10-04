//! The victim of Bowser's Koopa Klaw: ftCo_CaptureKoopa.c,
//! ftCo_CaptureDamageKoopa.c, ftCo_CaptureWaitKoopa.c and ftCo_ThrownKoopa.c.
//!
//! The victim hangs from its captor's TransN2 for the whole hold (the XRotN
//! constraint of ftCo_800DB368, placed each tick by accessory1
//! ftCo_800DB464), playing the captor's animations: the catch and each bite
//! (CaptureDamageKoopa), then frozen where it ended (CaptureWaitKoopa has
//! no animation), then the throw (ThrownKoopaF / B). A hold timer the
//! captor's attributes set runs down, faster for mashing; when it ends the
//! pair separates (CatchCut / CaptureCut).
use super::{
    assets::{FighterAssets, Result},
    grab::GrabLink,
    grab_throw::{self, ThrowSource},
    state::AnimationPhase,
    Fighter, MotionData,
};
use crate::anim::WaitChoice;
use melee_types::{CommonMotionState as S, FtPart, GroundOrAir};

/// ftCo_SM_CaptureDamageKoopa .. ftCo_SM_ThrownKoopaAirB: the captor's
/// animations its victim plays.
const DAMAGE_MOTION: i32 = 278;
const THROWN_FORWARD_MOTION: i32 = 279;
const THROWN_BACK_MOTION: i32 = 280;
const AIR_DAMAGE_MOTION: i32 = 281;
const AIR_THROWN_FORWARD_MOTION: i32 = 282;
const AIR_THROWN_BACK_MOTION: i32 = 283;
/// The victim motions a captor's data may author, for their skeleton remap.
pub const VICTIM_MOTIONS: [i32; 6] = [
    DAMAGE_MOTION,
    THROWN_FORWARD_MOTION,
    THROWN_BACK_MOTION,
    AIR_DAMAGE_MOTION,
    AIR_THROWN_FORWARD_MOTION,
    AIR_THROWN_BACK_MOTION,
];

/// What the victim reads from its captor through ftKp_SpecialS_80132DC0 ..
/// 80132E20 (ftKoopaAttributes x34..x4C): constant for the hold, so the
/// captor hands them over at the catch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hold {
    /// x4C: the hold timer's start (ftCommon_InitGrab).
    pub time: f32,
    /// x48: taken from the timer each frame.
    pub decay: f32,
    /// x44: taken per mash input (ftCommon_GrabMash).
    pub mash_escape: f32,
    /// x40: frames a mash keeps the struggle going.
    pub mash_frames: f32,
    /// x3C: the animation rate while struggling.
    pub mash_rate: f32,
    /// x34: the struggle's shake per unit of stick direction.
    pub shake_scale: f32,
    /// x38: the shake's limit either way.
    pub shake_limit: f32,
}

/// mv.co.capturekoopa and the grab fields it shares with the hold
/// (grab_timer, x1A50 / x1A51).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaptureKoopaState {
    pub hold: Hold,
    /// Fighter.grab_timer.
    pub timer: f32,
    /// x1A50 / x1A51: the last stick direction on each axis.
    pub stick_directions: [i8; 2],
    /// mv +0: this frame's ftCommon_GrabMash result.
    pub mashed: bool,
    /// mv +8: frames of struggle left.
    pub struggle: f32,
    /// The hold timer ran out in this frame's animation callback; the scene
    /// separates the pair (ftCo_800DA698, ftCo_CaptureCut_Enter).
    pub release_requested: bool,
}

/// What a captor's callback asks of the fighter it holds; the scene runs it
/// on the pair once the callback returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptorRequest {
    /// ftCo_800BC9C8 / ftCo_800BCAF4: the victim takes a bite.
    Damage { air: bool },
    /// ftCo_800BCDE0: the victim's thrown row from its first frame.
    Throw { back: bool, air: bool },
    /// ftCo_800BCE64: the grounded thrown row at the current frame, when an
    /// aerial throw lands.
    ThrowLanded { back: bool },
}

fn state(victim: &mut Fighter) -> &mut CaptureKoopaState {
    match &mut victim.core.state_data {
        MotionData::CaptureKoopa(state) => state,
        _ => panic!("Koopa capture scratch missing"),
    }
}

/// The captor's `motion` as the victim's animation source.
fn source<'a>(
    captor_assets: &'a FighterAssets,
    victim_assets: &'a FighterAssets,
    motion: i32,
) -> ThrowSource<'a> {
    let motion = &captor_assets.motions[&motion];
    let remap = motion.remap.as_ref().expect("prepared capture skeleton");
    ThrowSource {
        assets: captor_assets,
        animation: Some((
            motion,
            crate::anim::attach::MotionRemapView {
                source: &remap.source,
                destination: &victim_assets.parts,
                source_masks: &remap.source_masks,
            },
        )),
        flags: motion.flags,
        blend_frames: motion.blend_frames,
    }
}

/// ftCo_800BC7E0 (800BC7E0) / ftCo_800BC8D4 (800BC8D4), the Klaw's
/// grabbed_cb: the victim faces as its captor does, takes the hold's timer,
/// is pinned to the captor's TransN2 and takes the first bite's motion;
/// then it leaves the ground (ftCommon_8007D5D4) and stops.
pub fn capture(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    hold: Hold,
    air: bool,
) -> Result<()> {
    // ftCommon_8007DB58.
    victim.interrupt_actions(victim_assets);
    // ftCo_8009750C drops a heavy item; ftCo_800DD168 releases the victim's
    // own victim.
    assert!(
        victim.core.held_item.is_none(),
        "ftCo_8009750C: a captured fighter holding an item"
    );
    assert!(
        victim.core.combat.grab.is_none(),
        "ftCo_800DD168: a captured fighter holding another"
    );
    victim.core.combat.grab = Some(GrabLink::Captured {
        captor: captor.core.spawn_number,
    });
    victim.core.physics.facing = captor.core.physics.facing;
    // ftCommon_InitGrab(fp, 0, x4C), and mv +8.
    let scratch = CaptureKoopaState {
        hold,
        timer: hold.time,
        ..Default::default()
    };
    // ftCo_800DB368(captor, victim).
    grab_throw::constrain_to_captor(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    enter_damage(victim, captor, victim_assets, captor_assets, air, scratch)?;
    victim.core.leave_ground();
    // ftCommon_8007E2FC.
    victim.core.clear_movement();
    Ok(())
}

/// ftCo_800BC9C8 (800BC9C8) / ftCo_800BCAF4 (800BCAF4): CaptureDamageKoopa
/// with the captor's animation, accessory1 = ftCo_800DB464 (the scene's, for
/// a pinned victim), ungrabbable, one animation step. x2222_b6
/// (Ft_MF_FreezeState) has no setter in scope.
fn enter_damage(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    air: bool,
    scratch: CaptureKoopaState,
) -> Result<()> {
    let (row, motion) = if air {
        (S::CaptureDamageKoopaAir, AIR_DAMAGE_MOTION)
    } else {
        (S::CaptureDamageKoopa, DAMAGE_MOTION)
    };
    victim.change_motion_state_with_source(
        row.into(),
        victim_assets,
        0.0,
        1.0,
        Some(source(captor_assets, victim_assets, motion)),
    )?;
    victim.core.state_data = MotionData::CaptureKoopa(scratch);
    // ftCommon_8007E2F4(fp, 0x1FF).
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    victim.step_animation(victim_assets);
    grab_throw::update_constraint(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    Ok(())
}

/// A bite on a victim already held ([`CaptorRequest::Damage`]).
pub fn bite(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    air: bool,
) -> Result<()> {
    let scratch = *state(victim);
    enter_damage(victim, captor, victim_assets, captor_assets, air, scratch)
}

/// ftCo_800BCC20 (800BCC20) / ftCo_800BCD00 (800BCD00): CaptureWaitKoopa,
/// which has no animation.
fn enter_wait(victim: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let scratch = *state(victim);
    let row = if victim.motion_state.id == S::CaptureDamageKoopaAir {
        S::CaptureWaitKoopaAir
    } else {
        S::CaptureWaitKoopa
    };
    victim.change_motion_state(row.into(), assets)?;
    victim.core.state_data = MotionData::CaptureKoopa(scratch);
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    Ok(())
}

/// ftCo_800BC458 (800BC458): the hold timer's decay and the mash.
fn run_timer(victim: &mut Fighter, assets: &FighterAssets) {
    let input = victim.core.input.clone();
    let scratch = state(victim);
    scratch.timer -= scratch.hold.decay;
    scratch.mashed = super::capture_yoshi::grab_mash(
        &mut scratch.timer,
        &mut scratch.stick_directions,
        &input,
        assets.grab_escape.stick_threshold,
        scratch.hold.mash_escape,
    );
}

/// ftCo_800BC4A8 (800BC4A8): while the struggle lasts, YRotN shakes by the
/// stick's last directions (x along Z, y along Y; 800BC544 / 800BC548
/// fmuls, the sums fadds) within the limit, and its count runs down; a
/// struggle that has run out with no new mash returns the animation to its
/// own rate, and a mash starts one at the hold's rate.
fn struggle(victim: &mut Fighter, assets: &FighterAssets) {
    let scratch = *state(victim);
    let mut remaining = scratch.struggle;
    if remaining != 0.0 {
        let joint = victim.core.animation.parts
            [usize::from(assets.parts.joint(FtPart::YRotN).expect("YRotN"))]
        .joint;
        let scale = scratch.hold.shake_scale;
        let shake_z = f32::from(scratch.stick_directions[0]) * scale;
        let shake_y = f32::from(scratch.stick_directions[1]) * scale;
        let limit = scratch.hold.shake_limit;
        let mut translation = victim.core.skeleton.translation(joint);
        let mut moved = false;
        if gekko_math::msl::fabsf(shake_z + translation.z) <= limit {
            translation.z += shake_z;
            moved = true;
        }
        if gekko_math::msl::fabsf(shake_y + translation.y) <= limit {
            translation.y += shake_y;
            moved = true;
        }
        if moved {
            victim.core.skeleton.set_translate(joint, &translation);
        }
        remaining -= 1.0;
        if remaining <= 0.0 && !scratch.mashed {
            victim
                .core
                .animation
                .set_rate(&mut victim.core.skeleton, 1.0, false);
            remaining = 0.0;
        }
    }
    if remaining <= 0.0 && scratch.mashed {
        remaining = scratch.hold.mash_frames;
        victim
            .core
            .animation
            .set_rate(&mut victim.core.skeleton, scratch.hold.mash_rate, false);
    }
    state(victim).struggle = remaining;
}

/// The hold's frame once the animation has stepped: the timer, and either
/// the release request or the struggle.
fn hold_frame(victim: &mut Fighter, assets: &FighterAssets) {
    run_timer(victim, assets);
    if state(victim).timer <= 0.0 {
        state(victim).release_requested = true;
    } else {
        struggle(victim, assets);
    }
}

/// ftCo_CaptureDamageKoopa_Anim (800BCA44) / ftCo_CaptureDamageKoopaAir_Anim
/// (800BCB70): at the animation's end the wait, with nothing else that
/// frame; otherwise the hold's frame.
pub fn damage_animation(
    victim: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    victim.step_animation(phase.assets);
    if !victim.animation.frames_remaining(&victim.skeleton) {
        enter_wait(victim, phase.assets)?;
    } else {
        hold_frame(victim, phase.assets);
    }
    Ok(None)
}

/// ftCo_CaptureWaitKoopa_Anim (800BCC70) and its aerial twin.
pub fn wait_animation(
    victim: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    victim.step_animation(phase.assets);
    hold_frame(victim, phase.assets);
    Ok(None)
}

/// ftCo_ThrownKoopaF_Anim and its three twins are empty: the borrowed
/// animation only advances.
pub fn thrown_animation(
    victim: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    victim.step_animation(phase.assets);
    Ok(None)
}

/// The Koopa rows' collision callbacks are empty, but Fighter_procMap's
/// head still runs for them (fighter.c:2480-2489): the ECB lock that the
/// catch's ftCommon_8007D5D4 set counts down and opens during the hold, so
/// a victim set down later meets the floor with its own ECB.
pub fn collision(victim: &mut Fighter, _phase: super::state::CollisionPhase<'_>) -> Result<()> {
    let c = &mut victim.core;
    crate::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    Ok(())
}

/// Whether the victim's hold timer ran out this frame (taken).
pub fn take_release_request(victim: &mut Fighter) -> bool {
    match &mut victim.core.state_data {
        MotionData::CaptureKoopa(state) => std::mem::take(&mut state.release_requested),
        _ => false,
    }
}

/// The hold timer ran out: ftCo_800DA698(captor, false) (the captor's
/// CatchCut with its escape speed), then the victim turns round and
/// ftCo_CaptureCut_Enter sets it down (ftCo_800DC920) with its own.
pub fn escape(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let p = &captor_assets.grab_escape;
    if captor.physics.ground_or_air == GroundOrAir::Air {
        captor.physics.self_velocity.x = -captor.physics.facing * p.air_escape_speed;
        captor.physics.self_velocity.y = p.air_escape_vertical_speed;
    } else {
        captor.physics.ground_velocity = -captor.physics.facing * p.escape_speed;
    }
    captor.change_motion_state(S::CatchCut.into(), captor_assets)?;
    victim.physics.facing = -victim.physics.facing;
    set_down(victim, captor, victim_assets, map);
    let velocity = -victim.physics.facing * victim_assets.grab_escape.escape_speed;
    if victim.physics.ground_or_air == GroundOrAir::Ground {
        victim.physics.ground_velocity = velocity;
    } else {
        victim.physics.self_velocity.x = velocity;
    }
    victim.change_motion_state(S::CaptureCut.into(), victim_assets)
}

/// ftCo_800DC920 (800DC920) for a pinned victim: both links go and the
/// victim is set down where its XRotN points. The constraint still holds
/// the captor's TransN2 of the hold: the captor's CatchCut entry just
/// before has not posed its joints yet (retail sets Fox down at the held
/// point, bowser_klaw_hold_fd_fox4 tick 494).
fn set_down(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) {
    captor.combat.grab = None;
    super::grab_damage::release_thrown(captor, victim, victim_assets, map);
    victim.combat.grab = None;
    let root = victim.animation.root;
    let position = victim.physics.position;
    victim.skeleton.set_translate(root, &position);
}

/// ftCo_800BCDE0 (800BCDE0): ftCo_Thrown_Enter into the thrown row from its
/// first frame (the victim faces as its captor does, accessory1 =
/// ftCo_800DE508, ungrabbable), then one animation step.
pub fn throw(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    back: bool,
    air: bool,
) -> Result<()> {
    let (row, motion) = match (air, back) {
        (false, false) => (S::ThrownKoopaF, THROWN_FORWARD_MOTION),
        (false, true) => (S::ThrownKoopaB, THROWN_BACK_MOTION),
        (true, false) => (S::ThrownKoopaAirF, AIR_THROWN_FORWARD_MOTION),
        (true, true) => (S::ThrownKoopaAirB, AIR_THROWN_BACK_MOTION),
    };
    victim.core.physics.facing = captor.core.physics.facing;
    let scratch = *state(victim);
    victim.change_motion_state_with_source(
        row.into(),
        victim_assets,
        0.0,
        1.0,
        Some(source(captor_assets, victim_assets, motion)),
    )?;
    victim.core.state_data = MotionData::CaptureKoopa(CaptureKoopaState {
        mashed: false,
        ..scratch
    });
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    victim.step_animation(victim_assets);
    grab_throw::update_constraint(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    Ok(())
}

/// ftCo_800BCE64 (800BCE64): the grounded thrown row at the current frame
/// with the ground/air switch's flags (SkipMatAnim | SkipColAnim |
/// UpdateCmd | SkipItemVis | Unk19 | SkipModelPartVis | SkipModelFlags |
/// Unk27), when the captor's aerial throw lands.
pub fn throw_landed(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    back: bool,
) -> Result<()> {
    let (row, motion) = if back {
        (S::ThrownKoopaB, THROWN_BACK_MOTION)
    } else {
        (S::ThrownKoopaF, THROWN_FORWARD_MOTION)
    };
    victim.core.physics.facing = captor.core.physics.facing;
    let scratch = *state(victim);
    victim.change_ground_air_motion_with_source(
        row.into(),
        victim_assets,
        source(captor_assets, victim_assets, motion),
    )?;
    victim.core.state_data = MotionData::CaptureKoopa(CaptureKoopaState {
        mashed: false,
        ..scratch
    });
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    grab_throw::update_constraint(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    Ok(())
}
