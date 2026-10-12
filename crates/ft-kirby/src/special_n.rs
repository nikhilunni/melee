//! Inhale, ftkirbyspecialn.c (800F5F68..800F9090).
//!
//! Here: the Inhale until it catches. The start rows (353 ground, 371 air)
//! open the mouth and, on the script's flag, start the wind (efAlt 0x49A);
//! the loop rows (354, 372) hold it for at least the attribute's frames and
//! then while B is held; the end rows (355, 373) close the mouth. Each row
//! has its ground/air counterpart at the same frame. The grab is armed
//! throughout the start and loop rows (ftCommon_8007E2D0 with category
//! 0x10). A caught fighter continues in `mouthful.rs`
//! (`CharacterCallbacks::SPECIAL_GRAB`); catching an item is not ported.
use crate::{common, init::Kirby};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        ledge::GrabExclusions,
        state::{AnimationPhase, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};

/// ftKb_MS_SpecialN, SpecialNLoop, SpecialNEnd (353..355) and their aerial
/// rows (371..373).
pub const START: ActionId = ActionId(353);
pub const LOOP: ActionId = ActionId(354);
pub const END: ActionId = ActionId(355);
pub const AIR_START: ActionId = ActionId(371);
pub const AIR_LOOP: ActionId = ActionId(372);
pub const AIR_END: ActionId = ActionId(373);

/// ftCommon_8007E2D0's category argument for the Inhale.
const INHALE_GRAB: GrabExclusions = GrabExclusions(0x10);

/// The start row's change into its loop (ftKb_SpecialN_Anim): KeepGfx,
/// SkipModel and KeepSfx.
const LOOP_ENTRY_FLAGS: MotionEntryFlags = MotionEntryFlags(0x212);
/// ftKb_MF_SpecialN_Coll: ftCommon_GroundAirColl_MF with KeepGfx.
const END_COUNTERPART_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5082);
/// ftKb_MF_SpecialN_Loop_Coll: that with SkipModel, SkipHit and KeepSfx.
const COUNTERPART_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_529A);
/// ftKb_MF_SpecialN_LoopRumble_Coll: that with SkipRumble.
const LOOP_COUNTERPART_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5A9A);

/// `fp->parts[FtPart_YRotN]`: the parts array indexed by the part enum.
const WIND_JOINT: usize = 3;
/// efAlt 0x49A: efLib_CreateGenerator_AppSRT_SetFacingDir(0x138B).
const WIND_EFFECT: u16 = 0x49A;

pub const fn rows() -> [MotionRow; 6] {
    use melee_ft::fighter::state::callbacks::physics;
    [
        // ft_80084F3C on the ground, ft_80084EEC in the air, for all six.
        common::row(
            START,
            305,
            start_anim,
            common::no_input,
            physics::guard_on,
            start_collision,
        ),
        common::row(
            LOOP,
            306,
            loop_anim,
            loop_input,
            physics::guard_on,
            loop_collision,
        ),
        common::row(
            END,
            307,
            end_anim,
            common::no_input,
            physics::guard_on,
            end_collision,
        ),
        common::row(
            AIR_START,
            320,
            start_anim,
            common::no_input,
            physics::air_friction,
            air_start_collision,
        ),
        common::row(
            AIR_LOOP,
            321,
            loop_anim,
            loop_input,
            physics::air_friction,
            air_loop_collision,
        ),
        common::row(
            AIR_END,
            307,
            air_end_anim,
            common::no_input,
            physics::air_friction,
            air_end_collision,
        ),
    ]
}

/// ftCommon_8007E2D0(fp, 0x10, ...) with x2225_b1 (the grab makes no
/// sound), and x2222_b2: a cape does not turn Kirby round while he inhales.
fn arm(f: &mut Fighter) {
    f.core.status.special_grab = Some(INHALE_GRAB);
    f.character.get_mut::<Kirby>().cape_turn_blocked = true;
}

/// ftKb_SpecialN_800F5F68 (800F5F68) / ftKb_SpecialN_800F6070 (800F6070).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    let kirby = f.character.get_mut::<Kirby>();
    kirby.inhale_hold = kirby.attributes.inhale_minimum_frames;
    f.change_motion_state(if air { AIR_START } else { START }, assets)
        .expect("Inhale assets");
    f.step_animation(assets);
    arm(f);
}

/// ftKb_SpecialN_Anim (800F70A0) / ftKb_SpecialAirN_Anim (800F71CC): the
/// wind on the script's flag, then the loop when the animation ends.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 {
        let facing = f.physics.facing;
        f.effects.push(EffectRequest::AttachedParameter {
            id: WIND_EFFECT,
            bone: WIND_JOINT,
            parameter: facing,
        });
        f.core.effect_state.destroy_on_state_change = true;
        f.core.effect_state.hitlag_callbacks = true;
        f.commands.variables[0] = 0;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == AIR_START;
        common::seal_graphics(f, p.assets, p.rng);
        f.change_motion_state_with_flags(
            if air { AIR_LOOP } else { LOOP },
            p.assets,
            LOOP_ENTRY_FLAGS,
            0.0,
            1.0,
        )?;
        f.core.effect_state.hitlag_callbacks = true;
        arm(f);
    }
    Ok(None)
}

/// ftKb_SpecialNLoop_Anim / ftKb_SpecialAirNLoop_Anim are empty.
fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftKb_SpecialNLoop_IASA (800F7FA0) / ftKb_SpecialAirNLoop_IASA
/// (800F8000): the minimum hold counts down, then the end row once B is up.
fn loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    let kirby = f.character.get_mut::<Kirby>();
    if kirby.inhale_hold != 0 {
        kirby.inhale_hold -= 1;
        return;
    }
    if !f.input.current.held.intersects(Buttons::B) {
        let end = if f.motion_state.action == AIR_LOOP {
            AIR_END
        } else {
            END
        };
        f.change_motion_state(end, p.assets)
            .expect("Inhale end assets");
    }
}

/// ftKb_SpecialNEnd_Anim (800F7300): ft_8008A2BC.
fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftKb_SpecialAirNEnd_Anim (800F733C): ftCo_Fall_Enter.
fn air_end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// A start or loop row's counterpart at the current frame, rate 1: the
/// effect pause callbacks and the grab again.
fn counterpart(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
) -> Result<()> {
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, flags, frame, 1.0)?;
    f.core.effect_state.hitlag_callbacks = true;
    arm(f);
    Ok(())
}

/// ftKb_SpecialN_Coll (800F8C10) -> ft_8008403C: off the floor,
/// fn_800F6588.
fn start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    counterpart(
        f,
        AIR_START,
        COUNTERPART_FLAGS,
        p.assets.expect("Inhale assets"),
    )
}

/// ftKb_SpecialAirN_Coll (800F8C38) -> ft_80082C74: landing, fn_800F6638.
fn air_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    counterpart(
        f,
        START,
        COUNTERPART_FLAGS,
        p.assets.expect("Inhale assets"),
    )
}

/// ftKb_SpecialNLoop_Coll (800F8C60): fn_800F66E8.
fn loop_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    counterpart(
        f,
        AIR_LOOP,
        LOOP_COUNTERPART_FLAGS,
        p.assets.expect("Inhale assets"),
    )
}

/// ftKb_SpecialAirNLoop_Coll (800F8C88): fn_800F6798.
fn air_loop_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    counterpart(
        f,
        LOOP,
        LOOP_COUNTERPART_FLAGS,
        p.assets.expect("Inhale assets"),
    )
}

/// ftKb_SpecialNEnd_Coll (800F8CB0): fn_800F6848.
fn end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(
        AIR_END,
        p.assets.expect("Inhale assets"),
        END_COUNTERPART_FLAGS,
        frame,
        1.0,
    )
}

/// ftKb_SpecialAirNEnd_Coll (800F8CD8): fn_800F68A8.
fn air_end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(
        END,
        p.assets.expect("Inhale assets"),
        END_COUNTERPART_FLAGS,
        frame,
        1.0,
    )
}
