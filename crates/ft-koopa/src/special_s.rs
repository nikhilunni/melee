//! Koopa Klaw, ftkoopaspecials.c (80132E30..801344F0).
//!
//! The start swipes; a catch box that finds a fighter holds it (the hit
//! rows, then a frozen wait). While holding, B bites again, a stick flick
//! throws forward or back, and losing the floor drops the victim. The
//! victim's side is melee-ft's `capture_koopa`.
use crate::{
    common::{self, change},
    init::Koopa,
};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        capture_koopa::{self, CaptorRequest},
        grab::GrabLink,
        ledge::GrabExclusions,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
    physics::friction::friction_acceleration,
};

const START: ActionId = ActionId(347);
const HIT: ActionId = ActionId(348);
const BITE: ActionId = ActionId(349);
const WAIT: ActionId = ActionId(350);
const THROW_FORWARD: ActionId = ActionId(351);
const THROW_BACK: ActionId = ActionId(352);
const AIR_START: ActionId = ActionId(353);
const AIR_HIT: ActionId = ActionId(354);
const AIR_BITE: ActionId = ActionId(355);
const AIR_WAIT: ActionId = ActionId(356);
const AIR_THROW_FORWARD: ActionId = ActionId(357);
const AIR_THROW_BACK: ActionId = ActionId(358);

/// ftCommon_8007E2D0(fp, 8, ...): the Klaw's grab type (x1A68).
const GRAB_TYPE: GrabExclusions = GrabExclusions(8);
/// Ft_MF_SkipMatAnim: the bite row's entry (transition_flags0).
const BITE_FLAGS: MotionEntryFlags = MotionEntryFlags(0x80);
/// Ft_MF_SkipMatAnim | Ft_MF_Unk19: the hit rows' hand-off to the wait
/// (transition_flags3).
const WAIT_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0008_0080);
/// Ft_MF_Unk19: the throws' entry.
const THROW_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0008_0000);
/// The aerial wait's landing: the ground/air switch without UpdateCmd and
/// Unk27 (transition_flags2, 0x044C1080).
const WAIT_LANDING_FLAGS: MotionEntryFlags = MotionEntryFlags(0x044C_1080);
/// Ft_MF_Unk19, which keeps x2222_b2 across a motion change
/// (fighter.c:1021).
const KEEPS_CAPE_PROOF: u32 = 0x0008_0000;

/// mv.kp.unk1 for the Klaw rows (Fighter +2340..+2350), and what
/// Fighter_ChangeMotionState keeps beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Klaw {
    /// +0x0: B was pressed during a hit or the wait; a bite follows.
    pub bite_pressed: bool,
    /// +0x4: a bite has been taken; the hit rows play their bite variant and
    /// their hitbox takes the bite's damage.
    pub biting: bool,
    /// +0x8: the last stick flick, as seen from the facing (1 forward).
    pub throw_direction: i32,
    /// +0xC: the throw's script turned Bowser round (ftCheckThrowB4).
    pub turned: bool,
    /// facing_dir1: the facing at the last motion change, which the
    /// grounded root motion follows (ft_800850B4).
    pub entry_facing: f32,
    /// x2222_b2: a cape hit does not turn Bowser while he holds.
    pub cape_proof: bool,
}

/// ftKp_Init_MotionStateTable[6..18]: submotions 301..310; the waits replay
/// the hit rows' animation.
pub const fn rows() -> [MotionRow; 12] {
    [
        common::row(
            START,
            301,
            start_ground_anim,
            common::no_input,
            root_motion_physics,
            start_ground_collision,
        ),
        common::row(
            HIT,
            302,
            hit_anim::<false>,
            hit_input::<false>,
            common::ground_friction,
            held_ground_collision,
        ),
        common::row(
            BITE,
            303,
            hit_anim::<false>,
            hit_input::<false>,
            common::ground_friction,
            held_ground_collision,
        ),
        common::row(
            WAIT,
            302,
            wait_anim,
            wait_input::<false>,
            common::ground_friction,
            held_ground_collision,
        ),
        common::row(
            THROW_FORWARD,
            304,
            throw_ground_anim,
            common::no_input,
            root_motion_physics,
            held_ground_collision,
        ),
        common::row(
            THROW_BACK,
            305,
            throw_ground_anim,
            common::no_input,
            root_motion_physics,
            held_ground_collision,
        ),
        common::row(
            AIR_START,
            306,
            start_air_anim,
            common::no_input,
            start_air_physics,
            start_air_collision,
        ),
        common::row(
            AIR_HIT,
            307,
            hit_anim::<true>,
            hit_input::<true>,
            callbacks::physics::air_friction,
            hit_air_collision,
        ),
        common::row(
            AIR_BITE,
            308,
            hit_anim::<true>,
            hit_input::<true>,
            callbacks::physics::air_friction,
            hit_air_collision,
        ),
        common::row(
            AIR_WAIT,
            307,
            wait_anim,
            wait_input::<true>,
            callbacks::physics::air_friction,
            wait_air_collision,
        ),
        common::row(
            AIR_THROW_FORWARD,
            309,
            throw_air_anim,
            common::no_input,
            callbacks::physics::air_friction,
            throw_air_collision::<false>,
        ),
        common::row(
            AIR_THROW_BACK,
            310,
            throw_air_anim,
            common::no_input,
            callbacks::physics::air_friction,
            throw_air_collision::<true>,
        ),
    ]
}

fn klaw(f: &mut Fighter) -> &mut Klaw {
    &mut f.character.get_mut::<Koopa>().klaw
}

/// Fighter_ChangeMotionState from a Klaw row: facing_dir1 takes the facing
/// (fighter.c:948) and x2222_b2 survives only Ft_MF_Unk19 (fighter.c:1021).
fn change_row(
    f: &mut Fighter,
    row: ActionId,
    flags: MotionEntryFlags,
    start: f32,
    assets: &FighterAssets,
) -> Result<()> {
    let cape_proof = klaw(f).cape_proof && flags.0 & KEEPS_CAPE_PROOF != 0;
    change(f, row, flags, start, assets)?;
    let facing = f.physics.facing;
    let scratch = klaw(f);
    scratch.entry_facing = facing;
    scratch.cape_proof = cape_proof;
    Ok(())
}

/// ftCommon_8007E2D0(fp, 8, grab_cb, NULL, grabbed_cb), then mv +0 and +8.
fn arm_catch(f: &mut Fighter) {
    f.core.status.special_grab = Some(GRAB_TYPE);
    let scratch = klaw(f);
    scratch.bite_pressed = false;
    scratch.throw_direction = 0;
}

/// ftKp_SpecialS_Enter (80132F2C) / ftKp_SpecialAirS_Enter (80132FAC).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    *klaw(f) = Klaw::default();
    change_row(
        f,
        if air { AIR_START } else { START },
        MotionEntryFlags(0),
        0.0,
        a,
    )
    .expect("Koopa Klaw assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    arm_catch(f);
}

/// x2222_b2 for ftCo_800C3538.
pub fn cape_turn_blocked(f: &mut Fighter) -> bool {
    klaw(f).cape_proof
}

/// The hold's hit row for the bites taken so far, grounded or aerial.
fn enter_hit_row(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let (row, flags) = match (air, klaw(f).biting) {
        (false, false) => (HIT, MotionEntryFlags(0)),
        (false, true) => (BITE, BITE_FLAGS),
        (true, false) => (AIR_HIT, MotionEntryFlags(0)),
        (true, true) => (AIR_BITE, BITE_FLAGS),
    };
    change_row(f, row, flags, 0.0, assets)?;
    klaw(f).cape_proof = true;
    // ftCommon_8007E2F4(fp, 511), ftCommon_8007E2FC.
    f.core.status.grab_exclusions = GrabExclusions::ALL;
    f.core.clear_movement();
    klaw(f).bite_pressed = false;
    f.commands.variables[0] = 0;
    Ok(())
}

/// ftKp_SpecialS_8013302C (8013302C) / ftKp_SpecialS_801330E4 (801330E4),
/// the grab_cb, then the grabbed_cb on the victim (ftCo_800BC7E0 /
/// ftCo_800BC8D4).
pub fn grab(
    koopa: &mut Fighter,
    victim: &mut Fighter,
    koopa_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    let air = koopa.motion_state.action == AIR_START;
    enter_hit_row(koopa, air, koopa_assets)?;
    koopa.core.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: 0.0,
    });
    let hold = {
        let a = &koopa.character.get::<Koopa>().attributes.klaw;
        capture_koopa::Hold {
            time: a.hold_time,
            decay: a.hold_decay,
            mash_escape: a.mash_escape,
            mash_frames: a.mash_ticks,
            mash_rate: a.mash_rate,
            shake_scale: a.shake_scale,
            shake_limit: a.shake_limit,
        }
    };
    capture_koopa::capture(victim, koopa, victim_assets, koopa_assets, hold, air)
}

/// ftKp_SpecialSStart_Anim (80133654): Wait at the whiff's end.
fn start_ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialAirSStart_Anim (80133690): Fall at the whiff's end.
fn start_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftColl_8007ABD0 (8007ABD0): hitbox 0's damage becomes `damage`, staled.
fn set_bite_damage(f: &mut Fighter, damage: u32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    // ftCo_800DEEB8 scales only a released smash charge.
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: a charged Koopa Klaw"
    );
    let damage = damage as f32;
    let staled = f.commands.stale_damage(damage);
    if let Some(hit) = f.commands.hitboxes[0].as_mut() {
        hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
        hit.descriptor.damage = staled;
    }
}

/// ftKp_SpecialSHit_Anim (801336CC) / ftKp_SpecialAirSHit_Anim (8013383C):
/// once the script's cmd_vars[0] marks the bite's hitbox, a bite's takes the
/// attribute damage. At the animation's end a pressed B bites again (the
/// victim takes ftCo_800BC9C8 / ftCo_800BCAF4); otherwise the wait holds
/// the pose with the animation stopped.
fn hit_anim<const AIR: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 && klaw(f).biting {
        let damage = f.character.get::<Koopa>().attributes.klaw.bite_damage;
        set_bite_damage(f, damage);
        f.commands.variables[0] = 0;
    }
    if f.animation.frames_remaining(&f.skeleton) {
        return Ok(None);
    }
    if klaw(f).bite_pressed {
        klaw(f).biting = true;
        enter_hit_row(f, AIR, p.assets)?;
        f.combat.koopa_request = Some(CaptorRequest::Damage { air: AIR });
    } else {
        let frame = f.animation.frame;
        change_row(
            f,
            if AIR { AIR_WAIT } else { WAIT },
            WAIT_FLAGS,
            frame,
            p.assets,
        )?;
        stop_animation(f);
        klaw(f).bite_pressed = false;
        f.commands.variables[0] = 0;
        f.core.status.grab_exclusions = GrabExclusions::ALL;
    }
    Ok(None)
}

/// ftAnim_8006F0FC(gobj, 0.0).
fn stop_animation(f: &mut Fighter) {
    let c = &mut f.core;
    c.animation.set_rate(&mut c.skeleton, 0.0, false);
}

/// ftKp_SpecialSWait_Anim / ftKp_SpecialAirSWait_Anim are empty.
fn wait_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftKp_SpecialS_set_dir: a B press asks for a bite; a stick flick past the
/// attribute threshold (this frame beyond it, the last one not) is a throw
/// that way, kept as seen from the facing (80133C84: fmuls, fctiwz).
fn read_stick(f: &mut Fighter) -> i32 {
    let threshold = f.character.get::<Koopa>().attributes.klaw.throw_stick;
    if f.input.pressed.intersects(Buttons::B) {
        klaw(f).bite_pressed = true;
    }
    let (now, before) = (f.input.current.stick.x, f.input.previous.stick.x);
    let mut direction = 0;
    if before > -threshold && now < -threshold {
        direction = -1;
    }
    if before < threshold && now > threshold {
        direction = 1;
    }
    let facing = f.physics.facing;
    klaw(f).throw_direction = gekko_math::msl::fctiwz(facing * direction as f32);
    direction
}

/// ftKoopa_SpecialS_ChangeAction: the throw the flick chose, and the
/// victim's thrown row (ftCo_800BCDE0).
fn enter_throw(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    let back = klaw(f).throw_direction != 1;
    let row = match (air, back) {
        (false, false) => THROW_FORWARD,
        (false, true) => THROW_BACK,
        (true, false) => AIR_THROW_FORWARD,
        (true, true) => AIR_THROW_BACK,
    };
    change_row(f, row, THROW_FLAGS, 0.0, assets)?;
    f.combat.koopa_request = Some(CaptorRequest::Throw { back, air });
    Ok(())
}

/// ftKp_SpecialSHit_IASA (80133BF4) / ftKp_SpecialAirSHit_IASA (80133D20).
fn hit_input<const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if read_stick(f) != 0 {
        enter_throw(f, AIR, p.assets).expect("Koopa Klaw throw assets");
    }
}

/// ftKp_SpecialSWait_IASA (80133E4C) / ftKp_SpecialAirSWait_IASA
/// (80134038): a throw, else a bite on a pressed B.
fn wait_input<const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if read_stick(f) != 0 {
        enter_throw(f, AIR, p.assets).expect("Koopa Klaw throw assets");
    } else if klaw(f).bite_pressed {
        klaw(f).biting = true;
        enter_hit_row(f, AIR, p.assets).expect("Koopa Klaw bite assets");
        f.combat.koopa_request = Some(CaptorRequest::Damage { air: AIR });
    }
}

/// doEndFAnim / doEndBAnim (ftKp_SpecialS_80132E30): the script's turn
/// (ftCheckThrowB4) flips the facing; its cmd_vars[0] releases the victim
/// (ftCommon_8007E2F4(fp, 0), ftCo_800DE2A8, ftCo_800DE7C0(victim, 0, 0),
/// which the scene runs on the pair).
fn throw_frame(f: &mut Fighter) {
    if std::mem::take(&mut f.commands.throw_reverse) {
        f.physics.facing = -f.physics.facing;
        klaw(f).turned = true;
    }
    if f.commands.variables[0] != 0 && f.combat.grab.is_some() {
        f.core.status.grab_exclusions = GrabExclusions::NONE;
        f.combat.special_throw_release = true;
        f.commands.variables[0] = 0;
    }
}

/// ftKp_SpecialSEndF_Anim (801339B4) / ftKp_SpecialSEndB_Anim (80133A90).
fn throw_ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    throw_frame(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialAirSEndF_Anim (80133AD4) / ftKp_SpecialAirSEndB_Anim
/// (80133BB0).
fn throw_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    throw_frame(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::fall(f, p.assets)?;
    }
    Ok(None)
}

/// ft_800850B4 (800850B4) -> ft_800850E0 with the ground friction and
/// facing_dir1: TransN sets the ground speed along the facing the motion
/// was entered with (80085108: fmuls), otherwise friction; then
/// ftCommon_ApplyGroundMovement.
fn root_motion_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.animation
        .flags
        .contains(melee_ft::anim::MotionFlags::ROOT_MOTION)
    {
        let offset = f
            .animation
            .root_motion
            .as_ref()
            .expect("Koopa Klaw TransN")
            .primary_history
            .offset;
        let facing = klaw(f).entry_facing;
        f.physics.ground_velocity = offset.z * facing;
    } else {
        let friction = f.attributes.ground.ground_friction;
        f.physics.ground_acceleration = friction_acceleration(f.physics.ground_velocity, friction);
    }
    common::move_on_ground(f, &p);
}

/// ftKp_SpecialAirSStart_Phys (80134244): TransN drives the horizontal
/// speed (ft_80085134) while the vertical keeps falling (ftCommon_FallBasic).
fn start_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let vertical = f.physics.self_velocity.y;
    common::root_motion_velocity(f);
    f.physics.self_velocity.y = vertical;
    common::fall_basic(f);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftKp_SpecialSStart_Coll (80134388): off the floor (ft_80082708),
/// ftKp_SpecialS_8013319C continues in the aerial start with its hitboxes
/// and the catch armed again.
fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Koopa Klaw collision assets");
    f.leave_ground();
    let frame = f.animation.frame;
    change_row(f, AIR_START, common::GROUND_AIR_KEEP_HIT, frame, assets)?;
    arm_catch(f);
    Ok(())
}

/// ftKp_SpecialAirSStart_Coll (801343B0): on landing (ft_80081D0C),
/// ftKp_SpecialS_8013322C continues in the grounded start.
fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Koopa Klaw collision assets");
    f.land();
    let frame = f.animation.frame;
    change_row(f, START, common::GROUND_AIR_KEEP_HIT, frame, assets)?;
    arm_catch(f);
    f.commands.variables[0] = 0;
    Ok(())
}

/// The grounded hold rows' collision (ftKp_SpecialSHit_Coll 801343D8,
/// ftKp_SpecialSWait_Coll 80134428, ftKp_SpecialSEndF/B_Coll 80134478 /
/// 801344A0): ft_800827A0 stops at the floor's edge, so the floor is lost
/// only when it goes from under Bowser. Then he leaves the ground, the
/// victim is set down (ftCo_800DC920) and both fall
/// (ftKp_SpecialS_801332C4 / 80133398 / 80133484), which no stage in
/// scope reaches while he holds still.
fn held_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    unimplemented!(
        "ftKp_SpecialS_801332C4 (ftkoopaspecials.c:214): the Koopa Klaw's holder loses its floor"
    )
}

/// ftKp_SpecialAirSHit_Coll (80134400): landing (ft_80081D0C) continues in
/// the grounded hit row (ftKp_SpecialS_80133324; the bite row lands in it
/// too).
fn hit_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Koopa Klaw collision assets");
    f.land();
    let frame = f.animation.frame;
    change_row(f, HIT, common::GROUND_AIR_KEEP_HIT, frame, assets)?;
    f.core.status.grab_exclusions = GrabExclusions::ALL;
    f.core.clear_movement();
    Ok(())
}

/// ftKp_SpecialAirSWait_Coll (80134450): landing continues in the grounded
/// wait (ftKp_SpecialS_801333F8), the animation still stopped.
fn wait_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Koopa Klaw collision assets");
    f.land();
    let frame = f.animation.frame;
    change_row(f, WAIT, WAIT_LANDING_FLAGS, frame, assets)?;
    f.core.status.grab_exclusions = GrabExclusions::ALL;
    f.core.clear_movement();
    stop_animation(f);
    klaw(f).bite_pressed = false;
    f.commands.variables[0] = 0;
    Ok(())
}

/// ftKp_SpecialAirSEndF_Coll (801344C8) / ftKp_SpecialAirSEndB_Coll
/// (801344F0): landing continues in the grounded throw at the current
/// frame (ftKp_SpecialS_801334E4 / 8013359C). A throw that has turned
/// Bowser enters with the facing it started with, so facing_dir1 keeps the
/// root motion's direction. The victim takes the grounded thrown row
/// (ftCo_800BCE64).
fn throw_air_collision<const BACK: bool>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Koopa Klaw collision assets");
    f.land();
    let turned = klaw(f).turned;
    if turned {
        f.physics.facing = -f.physics.facing;
    }
    let frame = f.animation.frame;
    let row = if BACK { THROW_BACK } else { THROW_FORWARD };
    change_row(f, row, common::GROUND_AIR_KEEP_HIT, frame, assets)?;
    if turned {
        f.physics.facing = -f.physics.facing;
    }
    if f.combat.grab.is_some() {
        f.combat.koopa_request = Some(CaptorRequest::ThrowLanded { back: BACK });
    }
    f.core.status.grab_exclusions = GrabExclusions::ALL;
    f.core.clear_movement();
    Ok(())
}
