//! Shadow Ball, ftmewtwospecialn.c (80146CCC..80148C64).
//!
//! Five rows on the ground (341..345: start, loop, full loop, cancel, end)
//! and their aerial counterparts (346..350). The start forms the ball at
//! the right shoulder joint on the script's cmd_vars[3]; the loop gains a
//! charge every xC frames up to x0 (the charge Mewtwo keeps between uses,
//! u.mt.x2234) and holds it in the full loop; A or B release into the end,
//! whose script (cmd_vars[1]) lets the ball go with a recoil per charge,
//! and a shield press cancels, keeping the charge. A use that starts
//! without charge cannot be released for x10 frames.
use crate::{
    common::{self, GROUND_AIR_COLL_BASE_FLAGS},
    init::Mewtwo,
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        commands::RumbleRequest,
        state::{AnimationPhase, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_it::{Aim, ItemControl, ItemRequest, Launch, SpawnItem};
use melee_types::{GroundOrAir, ItemKind};

/// ftMt_MS_SpecialNStart (341) .. ftMt_MS_SpecialAirNEnd (350).
pub const START: ActionId = ActionId(341);
pub const LOOP: ActionId = ActionId(342);
pub const LOOP_FULL: ActionId = ActionId(343);
pub const CANCEL: ActionId = ActionId(344);
pub const END: ActionId = ActionId(345);
pub const AIR_START: ActionId = ActionId(346);
pub const AIR_LOOP: ActionId = ActionId(347);
pub const AIR_LOOP_FULL: ActionId = ActionId(348);
pub const AIR_CANCEL: ActionId = ActionId(349);
pub const AIR_END: ActionId = ActionId(350);

/// `fp->parts[FtPart_RShoulderN]` / `[FtPart_LHandNb]` / `[FtPart_TopN]`:
/// the parts array indexed by the part enum.
const SHOULDER_JOINT: usize = 35;
const LEFT_HAND_JOINT: usize = 32;
const ROOT_JOINT: usize = 0;
/// The ball forms and leaves 2 units along the shoulder joint's Z.
const SHOULDER_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 2.0);
/// ftMt_SpecialNLoop_Anim's shadowBallPos: the full charge's flash, above
/// the root.
const FULL_FLASH_OFFSET: Vec3 = Vec3::new(0.0, 7.0, 0.0);
/// efSync_Spawn(27, gobj, &pos).
const FULL_FLASH: u16 = 27;
/// ftCo_800BFFD0(fp, 92, 0): the full charge's colour animation.
const FULL_CHARGE_COLOR: u8 = 92;
/// ftCommon_8007EBAC(fp, 12, 0): the full charge's rumble.
const FULL_CHARGE_RUMBLE: u16 = 12;

/// ftMt_MF_SpecialN_Coll (ftCommon_GroundAirColl_MF), and with Ft_MF_KeepSfx
/// for the loops (ftMt_MF_SpecialNLoop_Coll).
const TRANSITION_FLAGS: MotionEntryFlags = GROUND_AIR_COLL_BASE_FLAGS;
const LOOP_TRANSITION_FLAGS: MotionEntryFlags =
    MotionEntryFlags(GROUND_AIR_COLL_BASE_FLAGS.0 | 1 << 9);
/// FTMEWTWO_SPECIALN_ACTION_FLAG: SkipMatAnim, KeepSfx, UpdateCmd,
/// SkipItemVis and Unk19.
const FULL_LOOP_FLAGS: MotionEntryFlags =
    MotionEntryFlags(1 << 7 | 1 << 9 | 1 << 14 | 1 << 18 | 1 << 19);

/// mv.mt.SpecialN (ftMewtwo/types.h). chargeLevel (x2350) only paces the
/// charge sounds (ftMt_SpecialN_PlayChargeSFX).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShadowBall {
    /// isFull: never set by retail's code, read at the start's end.
    pub full: bool,
    /// x2344: loop frames toward the next charge.
    pub frames: i32,
    /// x2348: the charge is full; the loop no longer counts.
    pub charged: bool,
    /// releaseLag: frames before the loop reacts.
    pub release_lag: i32,
}

pub const fn rows() -> [MotionRow; 10] {
    [
        common::row(
            START,
            295,
            start_anim,
            common::no_input,
            common::ground_friction,
            start_collision,
        ),
        common::row(
            LOOP,
            296,
            loop_anim,
            loop_input,
            common::ground_friction,
            loop_collision,
        ),
        common::row(
            LOOP_FULL,
            297,
            loop_full_anim,
            loop_full_input,
            common::ground_friction,
            loop_full_collision,
        ),
        common::row(
            CANCEL,
            298,
            cancel_anim,
            common::no_input,
            common::ground_friction,
            cancel_collision,
        ),
        common::row(
            END,
            299,
            end_anim,
            common::no_input,
            common::ground_friction,
            end_collision,
        ),
        common::row(
            AIR_START,
            300,
            air_start_anim,
            common::no_input,
            common::air_friction_fall,
            air_start_collision,
        ),
        common::row(
            AIR_LOOP,
            301,
            loop_anim,
            air_loop_input,
            common::air_friction_fall,
            air_loop_collision,
        ),
        common::row(
            AIR_LOOP_FULL,
            302,
            air_loop_full_anim,
            air_loop_full_input,
            common::air_friction_fall,
            air_loop_full_collision,
        ),
        common::row(
            AIR_CANCEL,
            303,
            air_cancel_anim,
            common::no_input,
            common::air_friction_fall,
            air_cancel_collision,
        ),
        common::row(
            AIR_END,
            304,
            air_end_anim,
            common::no_input,
            common::air_friction_fall,
            air_end_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::ShadowBallAttributes {
    &f.character.get::<Mewtwo>().attributes.shadow_ball
}
fn scratch(f: &mut Fighter) -> &mut ShadowBall {
    &mut f.character.get_mut::<Mewtwo>().shadow_ball
}
fn charge(f: &Fighter) -> i32 {
    f.character.get::<Mewtwo>().shadow_ball_charge
}
fn airborne_row(f: &Fighter) -> bool {
    f.motion_state.action.0 >= AIR_START.0
}

/// Mewtwo's stored charge (u.mt.x2234), and ftMt_Init_UnkMotionStates4's
/// glow (colour 92 whenever the secondary colour slot empties) while it is
/// the full charge x0.
pub fn set_charge(f: &mut Fighter, level: i32) {
    f.character.get_mut::<Mewtwo>().shadow_ball_charge = level;
    let full = level as f32 == attributes(f).full_charge;
    f.core.combat.secondary_color_fallback = full.then_some(FULL_CHARGE_COLOR);
}

/// The full charge as the ball reads it (ftMt_SpecialN_GetChargeLevel: the
/// float x0 stored to an s32).
pub fn full_charge(f: &Fighter) -> i32 {
    gekko_math::msl::fctiwz(attributes(f).full_charge)
}

/// ftMewtwo_SpecialN_SetCall: death2_cb and death3_cb are
/// ftMt_Init_OnDeath2 and take_dmg_cb is ftMt_Init_OnTakeDamage until the
/// next motion change.
fn install_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Mewtwo>().damage_callbacks = true;
}

/// ftMt_SpecialN_Enter (80147320) / ftMt_SpecialAirN_Enter (801473F4): the
/// start; a use without charge waits x10 frames in the loop; the aerial one
/// halves the vertical speed (fmuls).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.change_motion_state(if air { AIR_START } else { START }, a)
        .expect("Shadow Ball assets");
    f.commands.variables = [0; 4];
    if !air {
        // ftCommon_8007D7FC.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    install_callbacks(f);
    let release_lag = if charge(f) == 0 {
        attributes(f).release_lag
    } else {
        0
    };
    *scratch(f) = ShadowBall {
        full: false,
        frames: 0,
        charged: scratch(f).charged,
        release_lag,
    };
    if air {
        f.physics.self_velocity.y *= 0.5;
    }
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// The shoulder joint's point 2 units along its Z, on the stage plane
/// (ftMewtwo_SpecialN_GetPos).
fn shoulder_point(f: &mut Fighter) -> Vec3 {
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        SHOULDER_JOINT,
        SHOULDER_OFFSET,
    );
    position.z = 0.0;
    position
}

/// The start's and the loop's ball (ftMewtwo_SpecialN_CreateHeldShadow):
/// on the script's cmd_vars[3], with no ball held, it forms at the
/// shoulder (it_802C5000, then Item_8026AB54 into that joint) and the
/// callbacks go in. it_802C5000's pos is it_8026BB68's ECB midpoint
/// (ftLib_80086990: fadds, fmuls, fadds).
fn form_ball(f: &mut Fighter) {
    if f.commands.variables[3] != 1 || f.character.get::<Mewtwo>().held_ball {
        return;
    }
    let position = shoulder_point(f);
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(
        ItemKind::MewtwoShadowBall,
        c.player.id,
        position,
        c.physics.facing,
    );
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: SHOULDER_JOINT as u8,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    f.character.get_mut::<Mewtwo>().held_ball = true;
    install_callbacks(f);
}

/// ftMewtwo_SpecialN_RemoveShadowBall2: a held ball is destroyed
/// (it_802C573C). x2238, which the effects' removal hangs on, is never set.
pub fn remove_ball(f: &mut Fighter) {
    if std::mem::take(&mut f.character.get_mut::<Mewtwo>().held_ball) {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: ItemKind::MewtwoShadowBall,
            control: ItemControl::Remove,
        });
    }
}

/// ftMewtwo_SpecialN_SetRecoil: per charge, attribute x8 on the aerial end
/// or in the air and x4 on the grounded end or on the ground, along the
/// facing (separate fmuls, the charge first).
fn recoil(f: &mut Fighter) {
    let level = charge(f) as f32;
    let (ground, air) = {
        let a = attributes(f);
        (a.ground_recoil, a.air_recoil)
    };
    if f.motion_state.action == AIR_END || f.physics.ground_or_air == GroundOrAir::Air {
        f.physics.self_velocity.x = f.physics.facing * (air * level);
    }
    if f.motion_state.action == END || f.physics.ground_or_air == GroundOrAir::Ground {
        f.physics.ground_velocity = f.physics.facing * (ground * level);
    }
}

/// ftMt_SpecialN_ReleaseShadowBall (80146FA8): on the script's cmd_vars[1],
/// with a ball, it leaves the shoulder point (it_802C53F0) facing right (0)
/// or left (M_PI) with the stored charge; Mewtwo recoils, the charge
/// resets and the ball is let go.
fn release_ball(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[1] != 1 || !f.character.get::<Mewtwo>().held_ball {
        return;
    }
    f.commands.variables[1] = 2;
    let position = shoulder_point(f);
    let angle = if f.physics.facing == 1.0 {
        0.0
    } else {
        std::f64::consts::PI as f32
    };
    let level = charge(f) as u32;
    let full = attributes(f).full_charge;
    let holder = f.core.item_holder(SHOULDER_JOINT as u8, assets);
    // ftLib_80086630: the shoulder joint's world matrix, set up on demand.
    let hand = *holder.skeleton.get_mtx(holder.part);
    let launch = Launch {
        velocity: Vec3::ZERO,
        offset: Vec3::ZERO,
        spin_degrees: 0.0,
        hand,
        center: holder.center,
        attack: holder.attack,
        attack_stale: holder.attack_stale,
        aim: Some(Aim {
            position,
            angle,
            charge: level as f32,
            full_charge: full,
            facing: f.physics.facing,
        }),
        angle: 0.0,
        long_lifetime: false,
        shot: None,
    };
    f.core.item_requests.push(ItemRequest::Launch {
        owner: f.player.id,
        kind: ItemKind::MewtwoShadowBall,
        launch,
    });
    recoil(f);
    set_charge(f, 0);
    f.character.get_mut::<Mewtwo>().held_ball = false;
}

/// ftMt_SpecialNStart_Anim (801474C0) / ftMt_SpecialAirNStart_Anim
/// (80147954): the ball forms; at the end a full charge goes straight to
/// the end row, anything else to the loop.
fn start_to_loop(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
    looping: ActionId,
    end: ActionId,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    form_ball(f);
    if looping == AIR_LOOP {
        scratch(f).full = false;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let full = scratch(f).full || charge(f) as f32 == attributes(f).full_charge;
        common::seal_graphics(f, p.assets, p.rng);
        if full {
            f.change_motion_state(end, p.assets)?;
        } else {
            f.change_motion_state(looping, p.assets)?;
            scratch(f).charged = false;
        }
        install_callbacks(f);
    }
    Ok(None)
}
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    start_to_loop(f, p, LOOP, END)
}
fn air_start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    start_to_loop(f, p, AIR_LOOP, AIR_END)
}

/// ftMt_SpecialNLoop_Anim (8014764C) / ftMt_SpecialAirNLoop_Anim
/// (80147AEC): once the release lag is spent the ball forms and the loop
/// counts; every xC frames the charge grows, and at the full charge x0 the
/// full loop takes over at the current frame, with its rumble, flash
/// (efSync 27 seven units above the root) and colour 92.
fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let lag = {
        let s = scratch(f);
        s.release_lag -= 1;
        s.release_lag
    };
    if lag > 0 {
        return Ok(None);
    }
    scratch(f).release_lag = 0;
    form_ball(f);
    if scratch(f).charged {
        return Ok(None);
    }
    let per_charge = attributes(f).frames_per_charge;
    let s = scratch(f);
    s.frames += 1;
    if s.frames <= per_charge {
        return Ok(None);
    }
    s.frames = 0;
    let full = attributes(f).full_charge;
    let level = charge(f) + 1;
    set_charge(f, level);
    if level as f32 >= full {
        let state = if airborne_row(f) {
            AIR_LOOP_FULL
        } else {
            LOOP_FULL
        };
        let frame = f.animation.frame;
        common::seal_graphics(f, p.assets, p.rng);
        let scratch_kept = *scratch(f);
        let held = f.character.get::<Mewtwo>().held_ball;
        f.change_motion_state_with_flags(state, p.assets, FULL_LOOP_FLAGS, frame, 1.0)?;
        *scratch(f) = scratch_kept;
        f.character.get_mut::<Mewtwo>().held_ball = held;
        set_charge(f, gekko_math::msl::fctiwz(full));
        scratch(f).charged = true;
        f.commands.rumble_requests.push(RumbleRequest {
            all_players: false,
            id: FULL_CHARGE_RUMBLE,
            duration: 0,
        });
        let position = {
            let c = &mut f.core;
            melee_ft::fighter::caches::part_position(
                &mut c.skeleton,
                &c.animation,
                ROOT_JOINT,
                FULL_FLASH_OFFSET,
            )
        };
        f.core.effects.push(EffectRequest::PositionalGenerator {
            id: FULL_FLASH,
            position,
        });
        f.core
            .install_color_overlay_now(FULL_CHARGE_COLOR, p.assets);
    }
    Ok(None)
}

/// ftMt_SpecialNLoopFull_Anim (80147850): the charge stays full and the
/// loop stays stopped.
fn loop_full_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    scratch(f).charged = true;
    let full = full_charge(f);
    set_charge(f, full);
    Ok(None)
}

/// ftMt_SpecialAirNLoopFull_Anim (80147CF0): as on the ground, except that
/// it clears x2348.
fn air_loop_full_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    scratch(f).charged = false;
    let full = full_charge(f);
    set_charge(f, full);
    Ok(None)
}

/// ftMt_SpecialNCancel_Anim (80147880): the ball goes; Wait at the end.
fn cancel_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    remove_ball(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftMt_SpecialAirNCancel_Anim (80147D20): the ball goes; Fall at the end.
fn air_cancel_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    remove_ball(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftMt_SpecialNEnd_Anim (80147910): the release; Wait at the end.
fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    release_ball(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftMt_SpecialAirNEnd_Anim (80147DB0): the release; at the end Fall, or
/// the special fall with x14 of landing lag (ftCo_80096900(1, 0, 1, 1,
/// x14)).
fn air_end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    release_ball(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let lag = attributes(f).landing_lag;
        common::seal_graphics(f, p.assets, p.rng);
        if lag == 0.0 {
            common::enter_fall(f, p.assets)?;
        } else {
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// The loops' buttons (ftMt_SpecialNLoop_IASA, 80147E34, and its three
/// siblings): A releases once the release lag is spent; B does too, and
/// while it is pressed nothing else is tested; otherwise a shield press
/// cancels. The cancel tests the input proc's shield bit (retail
/// 0x80147F60: clrrwi. r0, r3, 31 on the pressed word), which a digital
/// shoulder, an analog trigger past the deadzone or Z all set. The full
/// loops have no lag (`lagged` false).
fn loop_buttons(f: &mut Fighter, assets: &FighterAssets, lagged: bool) {
    let (end, cancel) = if airborne_row(f) {
        (AIR_END, AIR_CANCEL)
    } else {
        (END, CANCEL)
    };
    let pressed = f.input.pressed;
    let ready = !lagged || scratch(f).release_lag <= 0;
    if pressed.intersects(Buttons::A) && ready {
        f.change_motion_state(end, assets).expect("Shadow Ball end");
        install_callbacks(f);
        return;
    }
    if pressed.intersects(Buttons::B) {
        if ready {
            f.change_motion_state(end, assets).expect("Shadow Ball end");
            install_callbacks(f);
        }
    } else if pressed.intersects(Buttons::SHIELD) {
        f.change_motion_state(cancel, assets)
            .expect("Shadow Ball cancel");
        remove_ball(f);
        install_callbacks(f);
    }
}

/// The grounded loops' roll first (ftCo_8009917C), which drops the ball.
fn grounded_loop_input(f: &mut Fighter, p: InputPhase<'_>, lagged: bool) {
    if let Some(roll) = f.core.roll_input(p.assets) {
        f.enter_escape(p.assets, roll)
            .expect("roll out of the charge");
        remove_ball(f);
        return;
    }
    loop_buttons(f, p.assets, lagged);
}

/// ftMt_SpecialNLoop_IASA (80147E34).
fn loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    grounded_loop_input(f, p, true);
}
/// ftMt_SpecialNLoopFull_IASA (8014800C).
fn loop_full_input(f: &mut Fighter, p: InputPhase<'_>) {
    grounded_loop_input(f, p, false);
}
/// ftMt_SpecialAirNLoop_IASA (801481D8).
fn air_loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    loop_buttons(f, p.assets, true);
}
/// ftMt_SpecialAirNLoopFull_IASA (80148354).
fn air_loop_full_input(f: &mut Fighter, p: InputPhase<'_>) {
    loop_buttons(f, p.assets, false);
}

/// A ground/air counterpart change keeps the special's scratch and ball.
fn change_counterpart(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
    to_air: bool,
) -> Result<()> {
    let kept = (*scratch(f), f.character.get::<Mewtwo>().held_ball);
    if to_air {
        common::ground_to_air(f, state, flags, assets)?;
    } else {
        common::air_to_ground(f, state, flags, assets)?;
    }
    *scratch(f) = kept.0;
    f.character.get_mut::<Mewtwo>().held_ball = kept.1;
    install_callbacks(f);
    Ok(())
}

/// The grounded rows' ft_80082708: off the floor, the aerial counterpart
/// (ftCommon_GroundToAirStateChange) and the callbacks.
fn leave_floor(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    state: ActionId,
    flags: MotionEntryFlags,
) -> Result<()> {
    if common::grounded(f, p) {
        return Ok(());
    }
    let assets = p.assets.expect("Shadow Ball collision assets");
    change_counterpart(f, state, flags, assets, true)
}

/// The aerial rows' ft_80081D0C: landing continues on the ground
/// (ftCommon_AirToGroundStateChange) with the callbacks.
fn land_as(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    state: ActionId,
    flags: MotionEntryFlags,
) -> Result<()> {
    if !common::lands(f, p) {
        return Ok(());
    }
    let assets = p.assets.expect("Shadow Ball landing assets");
    change_counterpart(f, state, flags, assets, false)
}

/// ftMt_SpecialNStart_Coll (80148600).
fn start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_START, TRANSITION_FLAGS)
}
/// ftMt_SpecialNLoop_Coll (8014868C).
fn loop_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_LOOP, LOOP_TRANSITION_FLAGS)
}
/// ftMt_SpecialNLoopFull_Coll (80148718).
fn loop_full_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_LOOP_FULL, LOOP_TRANSITION_FLAGS)
}
/// ftMt_SpecialNCancel_Coll (801487A4).
fn cancel_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_CANCEL, TRANSITION_FLAGS)
}
/// ftMt_SpecialNEnd_Coll (80148830).
fn end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_END, TRANSITION_FLAGS)
}
/// ftMt_SpecialAirNStart_Coll (801488BC).
fn air_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as(f, &mut p, START, TRANSITION_FLAGS)
}
/// ftMt_SpecialAirNLoop_Coll (80148948).
fn air_loop_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as(f, &mut p, LOOP, LOOP_TRANSITION_FLAGS)
}
/// ftMt_SpecialAirNLoopFull_Coll (801489D4).
fn air_loop_full_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as(f, &mut p, LOOP_FULL, LOOP_TRANSITION_FLAGS)
}
/// ftMt_SpecialAirNCancel_Coll (80148A60).
fn air_cancel_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as(f, &mut p, CANCEL, TRANSITION_FLAGS)
}
/// ftMt_SpecialAirNEnd_Coll (80148AEC).
fn air_end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as(f, &mut p, END, TRANSITION_FLAGS)
}

/// ftMt_SpecialN_Shoot (80148B78), from ftCo_ThrowF_Anim: on the forward
/// throw's cmd_vars[3] a ball leaves the right shoulder joint
/// (it_802C519C) along the line from the left hand to it (atan2f of
/// separate fsubs), both taken on the stage plane.
pub fn throw_shot(f: &mut Fighter, _assets: &FighterAssets) {
    if f.commands.variables[3] != 1 {
        return;
    }
    f.commands.variables[3] = 0;
    let (mut shoulder, mut hand) = {
        let c = &mut f.core;
        let mut at = |joint| {
            melee_ft::fighter::caches::part_position(
                &mut c.skeleton,
                &c.animation,
                joint,
                Vec3::ZERO,
            )
        };
        (at(SHOULDER_JOINT), at(LEFT_HAND_JOINT))
    };
    hand.z = 0.0;
    shoulder.z = 0.0;
    let angle = melee_lb::trigf::atan2f(shoulder.y - hand.y, shoulder.x - hand.x);
    let full = full_charge(f);
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(
        ItemKind::MewtwoShadowBall,
        c.player.id,
        shoulder,
        c.physics.facing,
    );
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_variant = it_mewtwo::shadow_ball::THROWN_VARIANT;
    spawn.spawn_argument = angle.to_bits() as i32;
    spawn.auxiliary_damage = full as i16;
    c.item_requests.push(ItemRequest::Spawn(spawn));
}
