//! The bow, ftlinkspecialn.c (800ED...800EE450).
//!
//! Entering the special creates the bow in the right hand (it_802AF1A4);
//! the draw's script creates the arrow in the left (command variable 0,
//! it_802A83E0), which follows both hands (it_802A8398) while the charge
//! counts (once command variable 2 starts it) until B is let go. The
//! release's collision callback shoots it on command variable 1
//! (itLinkArrow_802A850C). The bow follows the motion itself and goes once
//! the release ends. Both articles go if Link is hit while drawing
//! (ftLk_800EAF58).
use crate::{common, row, FamilyState, LinkFamily};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
};
use melee_it::{ItemControl, ItemRequest, Launch, Shot, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// Fighter_ChangeMotionState flags between the draw's rows
/// (Ft_MF_SkipModel | Ft_MF_SkipItemVis).
const ROW_FLAGS: u32 = 0x0004_0010;
/// ... and between the grounded and aerial counterparts (coll_mf).
const GROUND_AIR_FLAGS: u32 = 0x0C4C_5090;
/// MTXDegToRad(5): the arrow's flight angle.
const SHOT_ANGLE: f32 = 0.087_266_46;

/// Command variables the draw's script sets.
mod var {
    /// 0: make the arrow.
    pub const NOCK: usize = 0;
    /// 1: shoot it.
    pub const SHOOT: usize = 1;
    /// 2: the charge starts counting.
    pub const DRAW: usize = 2;
}

/// mv.lk.specialn and the articles' pointers (u.lk.x14, arrow_gobj).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bow {
    /// u.lk.x14: the bow is out.
    pub bow_out: bool,
    /// u.lk.arrow_gobj: an arrow is on the string.
    pub arrow_out: bool,
    /// mv x0.x: the draw has started counting.
    pub drawing: bool,
    /// mv x0.y: frames drawn.
    pub charge: f32,
}

fn bow_kind<C: LinkFamily>(f: &Fighter) -> ItemKind {
    ItemKind::try_from(f.character.get::<C>().attributes().bow.bow_item as i32).expect("bow kind")
}
fn arrow_kind<C: LinkFamily>(f: &Fighter) -> ItemKind {
    ItemKind::try_from(f.character.get::<C>().attributes().bow.arrow_item as i32)
        .expect("arrow kind")
}
fn bow<C: LinkFamily>(f: &mut Fighter) -> &mut Bow {
    &mut f.character.get_mut::<C>().specials().bow
}

fn part(assets: &FighterAssets, part: FtPart) -> u8 {
    assets.parts.joint(part).expect("Link thumb part")
}
fn thumb(f: &mut Fighter, assets: &FighterAssets, which: FtPart) -> hsd_types::Vec3 {
    let index = part(assets, which);
    let c = &mut f.core;
    melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(index),
        hsd_types::Vec3::ZERO,
    )
}

/// setCallbacks: take_dmg_cb / death2_cb = ftLk_800EAF58.
fn arm_removal<C: LinkFamily>(f: &mut Fighter) {
    f.character.get_mut::<C>().specials().removal_armed = true;
}

/// ftLk_SpecialN_Enter (800ED9E4) / ftLk_SpecialAirN_Enter (800EDAB8).
pub fn enter<C: LinkFamily>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    {
        let b = bow::<C>(f);
        b.drawing = false;
        b.charge = 0.0;
    }
    f.commands.variables[..4].fill(0);
    if !air {
        // ftCommon_8007D7FC, then no vertical speed.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    let state = if air {
        FamilyState::SpecialAirNStart
    } else {
        FamilyState::SpecialNStart
    };
    common::change(f, state.action(), 0, 0.0, a).expect("bow assets");
    arm_removal::<C>(f);
    set_draw_rate::<C>(f);
    // ftAnim_8006EBA4.
    f.step_animation(a);
    // isDrawback: a bow still out ends the special at once.
    if !take_bow::<C>(f, a) {
        forget_articles::<C>(f);
        if air {
            fall_after::<C>(f, a).expect("bow assets");
        } else {
            f.change_motion_state(melee_types::CommonMotionState::Wait.into(), a)
                .expect("bow assets");
        }
    }
}

/// ftAnim_SetAnimRate(specialn_anim_rate).
fn set_draw_rate<C: LinkFamily>(f: &mut Fighter) {
    let rate = f.character.get::<C>().attributes().bow.draw_animation_rate;
    let c = &mut f.core;
    c.animation.set_rate(&mut c.skeleton, rate, false);
}

/// isDrawback: the bow in the right hand (it_802AF1A4, Item_InitSpawnOnPlane
/// at the thumb, taken at RThumbNb) unless one is out already.
fn take_bow<C: LinkFamily>(f: &mut Fighter, a: &FighterAssets) -> bool {
    if bow::<C>(f).bow_out {
        return false;
    }
    let position = thumb(f, a, FtPart::RThumbNb);
    let kind = bow_kind::<C>(f);
    let c = &mut f.core;
    let spawn = SpawnItem::held(kind, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: part(a, FtPart::RThumbNb),
        hold: false,
        catch_item: false,
    });
    bow::<C>(f).bow_out = true;
    arm_removal::<C>(f);
    true
}

/// isDrawn: on command variable 0, the arrow in the left hand
/// (it_802A83E0 at the thumb, z 0, taken at LThumbNb) unless one is on.
/// True when that fails.
fn nock<C: LinkFamily>(f: &mut Fighter, a: &FighterAssets) -> bool {
    if f.commands.variables[var::NOCK] != 1 || bow::<C>(f).arrow_out {
        return false;
    }
    f.commands.variables[var::NOCK] = 0;
    let mut position = thumb(f, a, FtPart::LThumbNb);
    position.z = 0.0;
    let kind = arrow_kind::<C>(f);
    let c = &mut f.core;
    let spawn = SpawnItem::attached(kind, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: part(a, FtPart::LThumbNb),
        hold: false,
        catch_item: false,
    });
    bow::<C>(f).arrow_out = true;
    arm_removal::<C>(f);
    false
}

/// animate (the inline in the draw's Anim callbacks): the arrow at the
/// right thumb, its tail at the left (it_802A8398). mv x8/x14 (the draw's
/// angle) feed nothing else.
fn aim<C: LinkFamily>(f: &mut Fighter, a: &FighterAssets) {
    let tail = thumb(f, a, FtPart::LThumbNb);
    let tip = thumb(f, a, FtPart::RThumbNb);
    if bow::<C>(f).arrow_out {
        let kind = arrow_kind::<C>(f);
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind,
            control: ItemControl::Aim { tip, tail },
        });
    }
}

/// ftLk_SpecialN_UnsetArrow / UnsetFv14: the fighter lets go of its
/// pointers; the articles carry on by themselves.
fn forget_articles<C: LinkFamily>(f: &mut Fighter) {
    let b = bow::<C>(f);
    b.arrow_out = false;
    b.bow_out = false;
}

/// ftLk_SpecialN_ProcessFv10 / ProcessFv14 (ftLk_800EAF58): a hit takes the
/// arrow (it_802A8A7C) and the bow (it_802AF304).
pub fn remove_articles<C: LinkFamily>(f: &mut Fighter) {
    let (arrow, bow_out) = {
        let b = bow::<C>(f);
        (std::mem::take(&mut b.arrow_out), std::mem::take(&mut b.bow_out))
    };
    for (out, kind) in [(arrow, arrow_kind::<C>(f)), (bow_out, bow_kind::<C>(f))] {
        if out {
            f.core.item_requests.push(ItemRequest::Control {
                owner: f.player.id,
                kind,
                control: ItemControl::Remove,
            });
        }
    }
}

/// An article went by itself (itLinkBow_Logic100_Destroyed, the arrow's
/// Logic98_Destroyed before its release).
pub fn article_destroyed<C: LinkFamily>(f: &mut Fighter, kind: ItemKind) {
    if kind == bow_kind::<C>(f) {
        bow::<C>(f).bow_out = false;
    } else if kind == arrow_kind::<C>(f) {
        bow::<C>(f).arrow_out = false;
    }
}

/// ftLk_SpecialN_GetIndex (800ED...): the row the bow follows, while it
/// is out.
pub fn stage<C: LinkFamily>(f: &Fighter) -> Option<u8> {
    if !f.character.get::<C>().specials_ref().bow.bow_out {
        return None;
    }
    let first = FamilyState::SpecialNStart as u16;
    let action = f.motion_state.action.0;
    (first..first + 6)
        .contains(&action)
        .then(|| (action - first) as u8)
}

/// Command variable 2 starts the charge (once).
fn start_charge<C: LinkFamily>(f: &mut Fighter) {
    if f.commands.variables[var::DRAW] != 0 && !bow::<C>(f).drawing {
        bow::<C>(f).drawing = true;
        f.commands.variables[var::DRAW] = 0;
    }
}

/// ftLk_SpecialNStart_Anim (800EDB84) / ftLk_SpecialAirNStart_Anim.
fn start_anim<C: LinkFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    start_charge::<C>(f);
    if !AIR {
        set_draw_rate::<C>(f);
    }
    if nock::<C>(f, p.assets) {
        forget_articles::<C>(f);
        unimplemented!("ftLk_SpecialNStart_Anim: the arrow article could not be made");
    }
    aim::<C>(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let next = if AIR {
            FamilyState::SpecialAirNLoop
        } else {
            FamilyState::SpecialNLoop
        };
        common::change(f, next.action(), ROW_FLAGS, 0.0, p.assets)?;
        arm_removal::<C>(f);
        f.step_animation(p.assets);
    }
    Ok(None)
}

/// ftLk_SpecialNLoop_Anim / ftLk_SpecialAirNLoop_Anim.
fn loop_anim<C: LinkFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    aim::<C>(f, p.assets);
    Ok(None)
}

/// The aerial draw's end: Fall, or FallSpecial with the attribute's
/// landing lag (ftCo_80096900).
fn fall_after<C: LinkFamily>(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let lag = f.character.get::<C>().attributes().bow.air_landing_lag;
    if lag == 0.0 {
        f.change_motion_state(melee_types::CommonMotionState::Fall.into(), a)
    } else {
        f.enter_special_fall(a, true, false, true, 1.0, lag)
    }
}

/// ftLk_SpecialNEnd_Anim / ftLk_SpecialAirNEnd_Anim: at the animation's
/// end the pointers go (the bow ends itself) and Wait or the fall. The
/// model part update (updateParts) is drawing only.
fn end_anim<C: LinkFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if AIR {
            bow::<C>(f).arrow_out = false;
        }
        bow::<C>(f).bow_out = false;
        if AIR {
            fall_after::<C>(f, p.assets)?;
        } else {
            f.change_motion_state(melee_types::CommonMotionState::Wait.into(), p.assets)?;
        }
    }
    Ok(None)
}

/// Releasing B ends the draw (the charge's cap and state switch shared by
/// ftLk_SpecialNStart_IASA and the loop's doLoopIASA).
fn release<C: LinkFamily>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if !f.input.current.held.intersects(melee_ft::input::Buttons::B) {
        let end = if air {
            FamilyState::SpecialAirNEnd
        } else {
            FamilyState::SpecialNEnd
        };
        common::change(f, end.action(), ROW_FLAGS, 0.0, a).expect("bow assets");
        arm_removal::<C>(f);
    }
}

/// ftLk_SpecialNStart_IASA (800EDD1C) / ftLk_SpecialAirNStart_IASA: once
/// drawing, the charge counts up to the attribute's cap.
fn start_input<C: LinkFamily, const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if !bow::<C>(f).drawing {
        return;
    }
    let max = f.character.get::<C>().attributes().bow.max_charge;
    let b = bow::<C>(f);
    b.charge += 1.0;
    if b.charge > max {
        b.charge = max;
    }
    release::<C>(f, AIR, p.assets);
}

/// ftLk_SpecialNLoop_IASA / ftLk_SpecialAirNLoop_IASA (doLoopIASA): full
/// charge.
fn loop_input<C: LinkFamily, const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    let max = f.character.get::<C>().attributes().bow.max_charge;
    bow::<C>(f).charge = max;
    release::<C>(f, AIR, p.assets);
}

/// doEndColl: on command variable 1 with the arrow on, it flies from the
/// right thumb (its tail at the left, both at z 0) at 5 degrees with the
/// charge (itLinkArrow_802A850C), and Link lets go of it.
fn shoot<C: LinkFamily>(f: &mut Fighter, a: &FighterAssets) {
    if f.commands.variables[var::SHOOT] != 1 || !bow::<C>(f).arrow_out {
        return;
    }
    f.commands.variables[var::SHOOT] = 0;
    // Fighter_procMap placed the model at this frame's position before the
    // collision callback (air::begin_map, which the port runs after).
    let c = &mut f.core;
    c.skeleton.set_translate(c.animation.root, &c.physics.position);
    let mut tip = thumb(f, a, FtPart::RThumbNb);
    let mut tail = thumb(f, a, FtPart::LThumbNb);
    tip.z = 0.0;
    tail.z = 0.0;
    let charge = bow::<C>(f).charge;
    let max_charge = f.character.get::<C>().attributes().bow.max_charge;
    let hand_part = part(a, FtPart::LThumbNb);
    let holder = f.core.item_holder(hand_part, a);
    // ftLib_80086630: the hand's world matrix, set up on demand.
    let hand = *holder.skeleton.get_mtx(holder.part);
    let (center, attack, attack_stale) = (holder.center, holder.attack, holder.attack_stale);
    let kind = arrow_kind::<C>(f);
    let launch = Launch {
        velocity: hsd_types::Vec3::ZERO,
        offset: hsd_types::Vec3::ZERO,
        spin_degrees: 0.0,
        hand,
        center,
        attack,
        attack_stale,
        angle: SHOT_ANGLE,
        long_lifetime: false,
        shot: Some(Shot {
            tip,
            tail,
            angle: SHOT_ANGLE,
            charge,
            max_charge,
            facing: f.physics.facing,
        }),
        aim: None,
    };
    f.core.item_requests.push(ItemRequest::Launch {
        owner: f.player.id,
        kind,
        launch,
    });
    bow::<C>(f).arrow_out = false;
    // doEndColl keeps fp->item_gobj across the shot, then
    // ftpickupitem_80094818(gobj, false) poses the hand for it again.
    f.pose_hand_for_held_item(a);
}

/// ftLk_SpecialNStart_Coll / Loop_Coll (doColl): off the floor, the
/// aerial row at the same frame.
fn ground_collision<C: LinkFamily, const NEXT: u16>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("bow assets");
        leave_ground::<C>(f, NEXT, assets)?;
    }
    Ok(())
}
fn leave_ground<C: LinkFamily>(f: &mut Fighter, next: u16, a: &FighterAssets) -> Result<()> {
    f.leave_ground();
    let frame = f.animation.frame;
    common::change(f, melee_ft::fighter::ActionId(next), GROUND_AIR_FLAGS, frame, a)?;
    arm_removal::<C>(f);
    f.step_animation(a);
    Ok(())
}
/// ftLk_SpecialAirNStart_Coll / AirNLoop_Coll (doAirColl): landing, the
/// grounded row at the same frame.
fn air_collision<C: LinkFamily, const NEXT: u16>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("bow assets");
        land::<C>(f, NEXT, assets)?;
    }
    Ok(())
}
fn land<C: LinkFamily>(f: &mut Fighter, next: u16, a: &FighterAssets) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    common::change(f, melee_ft::fighter::ActionId(next), GROUND_AIR_FLAGS, frame, a)?;
    arm_removal::<C>(f);
    f.step_animation(a);
    Ok(())
}
/// ftLk_SpecialNEnd_Coll (800EE1D8): the shot, then doColl.
fn end_collision<C: LinkFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    shoot::<C>(f, p.assets.expect("bow assets"));
    ground_collision::<C, { FamilyState::SpecialAirNEnd as u16 }>(f, p)
}
/// ftLk_SpecialAirNEnd_Coll: the shot, then doAirColl.
fn air_end_collision<C: LinkFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    shoot::<C>(f, p.assets.expect("bow assets"));
    air_collision::<C, { FamilyState::SpecialNEnd as u16 }>(f, p)
}

fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

pub(crate) const fn rows<C: LinkFamily>() -> [MotionRow; 6] {
    [
        row(
            FamilyState::SpecialNStart,
            start_anim::<C, false>,
            start_input::<C, false>,
            ground_physics,
            ground_collision::<C, { FamilyState::SpecialAirNStart as u16 }>,
        ),
        row(
            FamilyState::SpecialNLoop,
            loop_anim::<C>,
            loop_input::<C, false>,
            ground_physics,
            ground_collision::<C, { FamilyState::SpecialAirNLoop as u16 }>,
        ),
        row(
            FamilyState::SpecialNEnd,
            end_anim::<C, false>,
            no_input,
            ground_physics,
            end_collision::<C>,
        ),
        row(
            FamilyState::SpecialAirNStart,
            start_anim::<C, true>,
            start_input::<C, true>,
            air_physics,
            air_collision::<C, { FamilyState::SpecialNStart as u16 }>,
        ),
        row(
            FamilyState::SpecialAirNLoop,
            loop_anim::<C>,
            loop_input::<C, true>,
            air_physics,
            air_collision::<C, { FamilyState::SpecialNLoop as u16 }>,
        ),
        row(
            FamilyState::SpecialAirNEnd,
            end_anim::<C, true>,
            no_input,
            air_physics,
            air_end_collision::<C>,
        ),
    ]
}
