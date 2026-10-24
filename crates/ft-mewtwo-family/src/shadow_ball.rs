//! Shadow Ball: ftmewtwospecialn.c (80146CCC..80148C64) and Kirby's copy,
//! ftkirbyspecialmewtwo.c (80106F44..80108C60). The two files differ only
//! in what [`ShadowBallUser`] names: the rows, the ball's item kind and
//! joint, the full charge's colour and flash point, the attributes, the
//! words that keep the charge, and whether the ball's removal and release
//! also destroy the user's effects (Kirby's copy runs efLib_DestroyAll
//! there; Mewtwo's tests x2238, which nothing sets).
//!
//! Five rows on the ground (start, loop, full loop, cancel, end) and their
//! aerial counterparts. The start forms the ball at the joint on the
//! script's cmd_vars[3]; the loop gains a charge every xC frames up to x0
//! (the charge kept between uses) and holds it in the full loop; A or B
//! release into the end, whose script (cmd_vars[1]) lets the ball go with
//! a recoil per charge, and a shield press cancels, keeping the charge. A
//! use that starts without charge cannot be released for x10 frames.
use crate::motion::{self, GROUND_AIR_COLL_BASE_FLAGS};
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

/// Shadow Ball's attributes: Mewtwo +0..+18, Kirby +384..+39C
/// (`specialn_mt_*`), the same six words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowBallAttributes {
    /// x0 / specialn_mt_charge_time: the stored charge at which the ball is
    /// full (compared with the charge as a float).
    pub full_charge: f32,
    /// x4 / specialn_mt_ground_horizontal_momentum: recoil per charge.
    pub ground_recoil: f32,
    /// x8 / specialn_mt_air_horizontal_momentum.
    pub air_recoil: f32,
    /// xC / specialn_mt_loops_to_full_charge: loop frames per charge step.
    pub frames_per_charge: i32,
    /// x10 / specialn_mt_frames_to_transition: the release lag.
    pub release_lag: i32,
    /// x14 / specialn_mt_freefall_toggle: landing lag (0: plain fall).
    pub landing_lag: f32,
}

/// mv.mt.SpecialN (ftMewtwo/types.h) / mv.kb.specialhi as
/// ftkirbyspecialmewtwo.c uses it. chargeLevel (x2350 / x10) only paces
/// the charge sounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShadowBall {
    /// isFull / x0: set by no code, read at the start's end.
    pub full: bool,
    /// x2344 / x4: loop frames toward the next charge.
    pub frames: i32,
    /// x2348 / x8: the charge is full; the loop no longer counts.
    pub charged: bool,
    /// releaseLag / xC: frames before the loop reacts.
    pub release_lag: i32,
}

/// Who charges: Mewtwo, and Kirby with his copied ability.
pub trait ShadowBallUser: 'static {
    /// SpecialNStart; Loop, LoopFull, Cancel, End and the five aerial rows
    /// follow in that order.
    const START: ActionId;
    /// SpecialNStart's submotion; the other rows' follow.
    const START_ANIMATION: i32;
    /// It_Kind_Mewtwo_ShadowBall / It_Kind_Kirby_MewtwoShadowBall.
    const BALL: ItemKind;
    /// `fp->parts[...]` the ball forms at and leaves from, 2 along its Z.
    const BALL_JOINT: usize;
    /// The full charge's flash (efSync 27) above parts[TopN].
    const FLASH_OFFSET: Vec3;
    /// ftCo_800BFFD0's colour as the loop reaches the full charge.
    const FULL_CHARGE_COLOR: u8;
    /// The ball's removal and its release run efLib_DestroyAll on the
    /// user (Kirby's ftKb_SpecialNMt_80107040 and the inlined removals).
    const DESTROYS_EFFECTS: bool;
    fn attributes(f: &Fighter) -> ShadowBallAttributes;
    fn scratch(f: &mut Fighter) -> &mut ShadowBall;
    /// The stored charge (u.mt.x2234 / u.kb.x9C).
    fn charge(f: &Fighter) -> i32;
    /// Stores the charge; the kind's ftData_UnkMotionStates4 colour
    /// (`CombatState::secondary_color_fallback`) follows it.
    fn store_charge(f: &mut Fighter, charge: i32);
    /// The ball in hand (u.mt.x2230 / u.kb.x98).
    fn held(f: &Fighter) -> bool;
    fn set_held(f: &mut Fighter, held: bool);
    /// The take-damage and death callbacks (ftMewtwo_SpecialN_SetCall,
    /// ftKb_SpecialN_set_cbs).
    fn arm_callbacks(f: &mut Fighter);
}

const START: u16 = 0;
const LOOP: u16 = 1;
const LOOP_FULL: u16 = 2;
const CANCEL: u16 = 3;
const END: u16 = 4;
/// The aerial rows follow the grounded ones in the same order.
const AIR_OFFSET: u16 = 5;

fn action<C: ShadowBallUser>(row: u16, air: bool) -> ActionId {
    ActionId(C::START.0 + row + if air { AIR_OFFSET } else { 0 })
}

/// `fp->parts[FtPart_TopN]`: the flash's joint.
const TOP_JOINT: usize = 0;
/// The ball forms and leaves 2 units along its joint's Z.
const BALL_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 2.0);
/// efSync_Spawn(27, gobj, &pos).
const FULL_FLASH: u16 = 27;
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

const fn row<C: ShadowBallUser, const AIR: bool>(
    index: u16,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    let index = index + if AIR { AIR_OFFSET } else { 0 };
    motion::row(
        ActionId(C::START.0 + index),
        C::START_ANIMATION + index as i32,
        anim,
        iasa,
        if AIR {
            motion::air_friction_fall
        } else {
            motion::ground_friction
        },
        collision,
    )
}

/// The ten rows for `C`.
pub const fn rows<C: ShadowBallUser>() -> [MotionRow; 10] {
    [
        row::<C, false>(
            START,
            start_anim::<C, false>,
            motion::no_input,
            cross::<C, false, START>,
        ),
        row::<C, false>(
            LOOP,
            loop_anim::<C>,
            loop_input::<C>,
            cross::<C, false, LOOP>,
        ),
        row::<C, false>(
            LOOP_FULL,
            loop_full_anim::<C, false>,
            loop_full_input::<C>,
            cross::<C, false, LOOP_FULL>,
        ),
        row::<C, false>(
            CANCEL,
            cancel_anim::<C, false>,
            motion::no_input,
            cross::<C, false, CANCEL>,
        ),
        row::<C, false>(
            END,
            end_anim::<C, false>,
            motion::no_input,
            cross::<C, false, END>,
        ),
        row::<C, true>(
            START,
            start_anim::<C, true>,
            motion::no_input,
            cross::<C, true, START>,
        ),
        row::<C, true>(
            LOOP,
            loop_anim::<C>,
            air_loop_input::<C>,
            cross::<C, true, LOOP>,
        ),
        row::<C, true>(
            LOOP_FULL,
            loop_full_anim::<C, true>,
            air_loop_full_input::<C>,
            cross::<C, true, LOOP_FULL>,
        ),
        row::<C, true>(
            CANCEL,
            cancel_anim::<C, true>,
            motion::no_input,
            cross::<C, true, CANCEL>,
        ),
        row::<C, true>(
            END,
            end_anim::<C, true>,
            motion::no_input,
            cross::<C, true, END>,
        ),
    ]
}

fn airborne_row<C: ShadowBallUser>(f: &Fighter) -> bool {
    f.motion_state.action.0 >= C::START.0 + AIR_OFFSET
}

/// The charge's full level as the ball reads it (the float x0 stored to
/// an s32).
pub fn full_charge<C: ShadowBallUser>(f: &Fighter) -> i32 {
    gekko_math::msl::fctiwz(C::attributes(f).full_charge)
}

/// The stored charge is the full charge (the integer converted).
pub fn is_full<C: ShadowBallUser>(f: &Fighter) -> bool {
    C::charge(f) as f32 == C::attributes(f).full_charge
}

/// ftMt_SpecialN_Enter (80147320) / ftMt_SpecialAirN_Enter (801473F4);
/// ftKb_SpecialNMt_80107568 / 80107638: the start; a use without charge
/// waits x10 frames in the loop; the aerial one halves the vertical speed
/// (fmuls).
pub fn enter<C: ShadowBallUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.change_motion_state(action::<C>(START, air), a)
        .expect("Shadow Ball assets");
    f.commands.variables = [0; 4];
    if !air {
        // ftCommon_8007D7FC.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    C::arm_callbacks(f);
    let release_lag = if C::charge(f) == 0 {
        C::attributes(f).release_lag
    } else {
        0
    };
    let charged = C::scratch(f).charged;
    *C::scratch(f) = ShadowBall {
        full: false,
        frames: 0,
        charged,
        release_lag,
    };
    if air {
        f.physics.self_velocity.y *= 0.5;
    }
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// The ball's joint's point 2 units along its Z, on the stage plane
/// (ftMewtwo_SpecialN_GetPos).
fn ball_point<C: ShadowBallUser>(f: &mut Fighter) -> Vec3 {
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        C::BALL_JOINT,
        BALL_OFFSET,
    );
    position.z = 0.0;
    position
}

/// The start's and the loop's ball (ftMewtwo_SpecialN_CreateHeldShadow,
/// ftKb_SpecialNMt_CreateHeldShadow): on the script's cmd_vars[3], with no
/// ball held, it forms at the joint (it_802C5000, then Item_8026AB54 into
/// that joint) and the callbacks go in. it_802C5000's pos is
/// it_8026BB68's ECB midpoint (ftLib_80086990: fadds, fmuls, fadds).
fn form_ball<C: ShadowBallUser>(f: &mut Fighter) {
    if f.commands.variables[3] != 1 || C::held(f) {
        return;
    }
    let position = ball_point::<C>(f);
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(C::BALL, c.player.id, position, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: C::BALL_JOINT as u8,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    C::set_held(f, true);
    C::arm_callbacks(f);
}

/// ftMewtwo_SpecialN_RemoveShadowBall2 / Kirby's inlined removal: a held
/// ball is destroyed (it_802C573C); Kirby's then destroys his effects
/// (efLib_DestroyAll, xA0 = NULL), Mewtwo's only when x2238, which is never
/// set.
pub fn remove_ball<C: ShadowBallUser>(f: &mut Fighter) {
    if C::held(f) {
        C::set_held(f, false);
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: C::BALL,
            control: ItemControl::Remove,
        });
    }
    if C::DESTROYS_EFFECTS {
        f.core.effects.push(EffectRequest::DestroyOwned);
    }
}

/// ftMewtwo_SpecialN_SetRecoil / ftKb_SpecialNMt_SetRecoil: per charge,
/// the aerial recoil on the aerial end or in the air and the grounded one
/// on the grounded end or on the ground, along the facing (separate fmuls,
/// the charge first; retail 0x80107300 / 0x8010734C).
fn recoil<C: ShadowBallUser>(f: &mut Fighter) {
    let level = C::charge(f) as f32;
    let a = C::attributes(f);
    let action = f.motion_state.action;
    if action == action_of::<C>(END, true) || f.physics.ground_or_air == GroundOrAir::Air {
        f.physics.self_velocity.x = f.physics.facing * (a.air_recoil * level);
    }
    if action == action_of::<C>(END, false) || f.physics.ground_or_air == GroundOrAir::Ground {
        f.physics.ground_velocity = f.physics.facing * (a.ground_recoil * level);
    }
}

fn action_of<C: ShadowBallUser>(row: u16, air: bool) -> ActionId {
    action::<C>(row, air)
}

/// ftMt_SpecialN_ReleaseShadowBall (80146FA8) / ftKb_SpecialNMt_801071FC:
/// on the script's cmd_vars[1], with a ball, it leaves the joint's point
/// (it_802C53F0) facing right (0) or left (M_PI) with the stored charge;
/// the user recoils, the charge resets and the ball is let go (Kirby's
/// ftKb_SpecialNMt_80107040 also destroys his effects).
fn release_ball<C: ShadowBallUser>(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[1] != 1 || !C::held(f) {
        return;
    }
    f.commands.variables[1] = 2;
    let position = ball_point::<C>(f);
    let angle = if f.physics.facing == 1.0 {
        0.0
    } else {
        std::f64::consts::PI as f32
    };
    let level = C::charge(f) as u32;
    let full = C::attributes(f).full_charge;
    let holder = f.core.item_holder(C::BALL_JOINT as u8, assets);
    // ftLib_80086630: the joint's world matrix, set up on demand.
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
        kind: C::BALL,
        launch,
    });
    recoil::<C>(f);
    C::store_charge(f, 0);
    C::set_held(f, false);
    if C::DESTROYS_EFFECTS {
        // After it_802C53F0's own work on the ball (retail 0x801072B8 then
        // 0x80107388): the destroy waits for the launch request.
        f.core.effects_after_items.push(EffectRequest::DestroyOwned);
    }
}

/// ftMt_SpecialNStart_Anim (801474C0) / ftMt_SpecialAirNStart_Anim
/// (80147954): the ball forms; at the end a full charge goes straight to
/// the end row, anything else to the loop. The aerial start clears `full`.
fn start_anim<C: ShadowBallUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    form_ball::<C>(f);
    if AIR {
        C::scratch(f).full = false;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let full = C::scratch(f).full || is_full::<C>(f);
        motion::seal_graphics(f, p.assets, p.rng);
        if full {
            f.change_motion_state(action::<C>(END, AIR), p.assets)?;
        } else {
            f.change_motion_state(action::<C>(LOOP, AIR), p.assets)?;
            C::scratch(f).charged = false;
        }
        C::arm_callbacks(f);
    }
    Ok(None)
}

/// ftMt_SpecialNLoop_Anim (8014764C) / ftMt_SpecialAirNLoop_Anim
/// (80147AEC): once the release lag is spent the ball forms and the loop
/// counts; every xC frames the charge grows, and at the full charge x0 the
/// full loop takes over at the current frame, with its rumble, flash
/// (efSync 27 above the top joint) and colour.
fn loop_anim<C: ShadowBallUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let lag = {
        let s = C::scratch(f);
        s.release_lag -= 1;
        s.release_lag
    };
    if lag > 0 {
        return Ok(None);
    }
    C::scratch(f).release_lag = 0;
    form_ball::<C>(f);
    if C::scratch(f).charged {
        return Ok(None);
    }
    let per_charge = C::attributes(f).frames_per_charge;
    let s = C::scratch(f);
    s.frames += 1;
    if s.frames <= per_charge {
        return Ok(None);
    }
    s.frames = 0;
    let full = C::attributes(f).full_charge;
    let level = C::charge(f) + 1;
    C::store_charge(f, level);
    if level as f32 >= full {
        let state = action::<C>(LOOP_FULL, airborne_row::<C>(f));
        let frame = f.animation.frame;
        motion::seal_graphics(f, p.assets, p.rng);
        let scratch_kept = *C::scratch(f);
        let held = C::held(f);
        f.change_motion_state_with_flags(state, p.assets, FULL_LOOP_FLAGS, frame, 1.0)?;
        *C::scratch(f) = scratch_kept;
        C::set_held(f, held);
        C::store_charge(f, gekko_math::msl::fctiwz(full));
        C::scratch(f).charged = true;
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
                TOP_JOINT,
                C::FLASH_OFFSET,
            )
        };
        f.core.effects.push(EffectRequest::PositionalGenerator {
            id: FULL_FLASH,
            position,
        });
        f.core
            .install_color_overlay_now(C::FULL_CHARGE_COLOR, p.assets);
    }
    Ok(None)
}

/// ftMt_SpecialNLoopFull_Anim (80147850): the charge stays full and the
/// loop stays stopped. ftMt_SpecialAirNLoopFull_Anim (80147CF0): the same,
/// except that it clears x2348.
fn loop_full_anim<C: ShadowBallUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    C::scratch(f).charged = !AIR;
    let full = full_charge::<C>(f);
    C::store_charge(f, full);
    Ok(None)
}

/// ftMt_SpecialNCancel_Anim (80147880) / ftMt_SpecialAirNCancel_Anim
/// (80147D20): the ball goes; Wait or Fall at the end.
fn cancel_anim<C: ShadowBallUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    remove_ball::<C>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        motion::seal_graphics(f, p.assets, p.rng);
        if AIR {
            motion::enter_fall(f, p.assets)?;
        } else {
            motion::finish(f, false, p.assets)?;
        }
    }
    Ok(None)
}

/// ftMt_SpecialNEnd_Anim (80147910): the release; Wait at the end.
/// ftMt_SpecialAirNEnd_Anim (80147DB0): the release; at the end Fall, or
/// the special fall with x14 of landing lag (ftCo_80096900(1, 0, 1, 1,
/// x14)).
fn end_anim<C: ShadowBallUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    release_ball::<C>(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        motion::seal_graphics(f, p.assets, p.rng);
        if !AIR {
            motion::finish(f, false, p.assets)?;
            return Ok(None);
        }
        let lag = C::attributes(f).landing_lag;
        if lag == 0.0 {
            motion::enter_fall(f, p.assets)?;
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
fn loop_buttons<C: ShadowBallUser>(f: &mut Fighter, assets: &FighterAssets, lagged: bool) {
    let air = airborne_row::<C>(f);
    let pressed = f.input.pressed;
    let ready = !lagged || C::scratch(f).release_lag <= 0;
    if pressed.intersects(Buttons::A) && ready {
        f.change_motion_state(action::<C>(END, air), assets)
            .expect("Shadow Ball end");
        C::arm_callbacks(f);
        return;
    }
    if pressed.intersects(Buttons::B) {
        if ready {
            f.change_motion_state(action::<C>(END, air), assets)
                .expect("Shadow Ball end");
            C::arm_callbacks(f);
        }
    } else if pressed.intersects(Buttons::SHIELD) {
        f.change_motion_state(action::<C>(CANCEL, air), assets)
            .expect("Shadow Ball cancel");
        remove_ball::<C>(f);
        C::arm_callbacks(f);
    }
}

/// The grounded loops' roll first (ftCo_8009917C), which drops the ball.
fn grounded_loop_input<C: ShadowBallUser>(f: &mut Fighter, p: InputPhase<'_>, lagged: bool) {
    if let Some(roll) = f.core.roll_input(p.assets) {
        f.enter_escape(p.assets, roll)
            .expect("roll out of the charge");
        remove_ball::<C>(f);
        return;
    }
    loop_buttons::<C>(f, p.assets, lagged);
}

/// ftMt_SpecialNLoop_IASA (80147E34).
fn loop_input<C: ShadowBallUser>(f: &mut Fighter, p: InputPhase<'_>) {
    grounded_loop_input::<C>(f, p, true);
}
/// ftMt_SpecialNLoopFull_IASA (8014800C).
fn loop_full_input<C: ShadowBallUser>(f: &mut Fighter, p: InputPhase<'_>) {
    grounded_loop_input::<C>(f, p, false);
}
/// ftMt_SpecialAirNLoop_IASA (801481D8).
fn air_loop_input<C: ShadowBallUser>(f: &mut Fighter, p: InputPhase<'_>) {
    loop_buttons::<C>(f, p.assets, true);
}
/// ftMt_SpecialAirNLoopFull_IASA (80148354).
fn air_loop_full_input<C: ShadowBallUser>(f: &mut Fighter, p: InputPhase<'_>) {
    loop_buttons::<C>(f, p.assets, false);
}

/// A ground/air counterpart change keeps the special's scratch and ball.
fn change_counterpart<C: ShadowBallUser>(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
    to_air: bool,
) -> Result<()> {
    let kept = (*C::scratch(f), C::held(f));
    if to_air {
        motion::ground_to_air(f, state, flags, assets)?;
    } else {
        motion::air_to_ground(f, state, flags, assets)?;
    }
    *C::scratch(f) = kept.0;
    C::set_held(f, kept.1);
    C::arm_callbacks(f);
    Ok(())
}

/// The rows' Coll callbacks (ftMt_SpecialNStart_Coll, 80148600, through
/// ftMt_SpecialAirNEnd_Coll, 80148AEC): a grounded row's ft_80082708 off
/// the floor takes the aerial counterpart (ftCommon_GroundToAirStateChange),
/// an aerial row's ft_80081D0C landing the grounded one, with the loops'
/// KeepSfx and the callbacks.
fn cross<C: ShadowBallUser, const AIR: bool, const ROW: u16>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    let flags = if matches!(ROW, LOOP | LOOP_FULL) {
        LOOP_TRANSITION_FLAGS
    } else {
        TRANSITION_FLAGS
    };
    if AIR {
        if !motion::lands(f, &mut p) {
            return Ok(());
        }
        let assets = p.assets.expect("Shadow Ball landing assets");
        change_counterpart::<C>(f, action::<C>(ROW, false), flags, assets, false)
    } else {
        if motion::grounded(f, &mut p) {
            return Ok(());
        }
        let assets = p.assets.expect("Shadow Ball collision assets");
        change_counterpart::<C>(f, action::<C>(ROW, true), flags, assets, true)
    }
}
