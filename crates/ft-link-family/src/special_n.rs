//! The bow, ftlinkspecialn.c (800ED...800EE450), and Kirby's copy of it
//! (ftkirbyspeciallink.c, 800FB394..800FBF0C): the same states over either
//! user's rows, generic over [`BowUser`]; every `LinkFamily` gets it by a
//! blanket impl.
//!
//! Entering the special creates the bow in the right hand (it_802AF1A4);
//! the draw's script creates the arrow in the other (command variable 0,
//! it_802A83E0), which follows the hands (it_802A8398) while the charge
//! counts (once command variable 2 starts it) until B is let go. The
//! release's collision callback shoots it on command variable 1
//! (itLinkArrow_802A850C). The bow follows the motion itself and goes once
//! the release ends. Both articles go if the user is hit while drawing
//! (ftLk_800EAF58, Kirby's put-away 800FB800 / 800FB840).
use crate::{attributes::BowAttributes, common, LinkFamily};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionRow,
    },
};
use melee_it::{ItemControl, ItemRequest, Launch, Shot, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// Fighter_ChangeMotionState flags between the draw's rows
/// (Ft_MF_SkipModel | Ft_MF_SkipItemVis).
const ROW_FLAGS: u32 = 0x0004_0010;
/// ... and between the grounded and aerial counterparts (coll_mf).
const GROUND_AIR_FLAGS: u32 = 0x0C4C_5090;
/// MTXDegToRad(5): the arrow's flight angle.
const SHOT_ANGLE: f32 = 0.087_266_46;

/// The rows from the grounded start: start, loop, end, then the aerial
/// three.
mod row {
    pub const START: u16 = 0;
    pub const LOOP: u16 = 1;
    pub const END: u16 = 2;
    pub const AIR: u16 = 3;
    pub const COUNT: u16 = 6;
}

/// Command variables the draw's script sets.
mod var {
    /// 0: make the arrow.
    pub const NOCK: usize = 0;
    /// 1: shoot it.
    pub const SHOOT: usize = 1;
    /// 2: the charge starts counting.
    pub const DRAW: usize = 2;
}

/// mv.lk.specialn and the articles' pointers (u.lk.x14 / arrow_gobj,
/// Kirby's u.kb.x78 / x74).
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

/// What the bow's states ask of their user.
pub trait BowUser {
    /// The grounded start; the loop and end follow, then the aerial three.
    const START: ActionId;
    /// The grounded start's animation; the other rows play the next five.
    const START_ANIMATION: i32;
    /// The bow block of the attributes (its arrow and bow item kinds).
    fn attributes(f: &Fighter) -> BowAttributes;
    fn scratch(f: &mut Fighter) -> &mut Bow;
    fn state(f: &Fighter) -> &Bow;
    /// take_dmg_cb / death2_cb, which every row change installs
    /// (setCallbacks, ftKb_SpecialN_set_cbs).
    fn arm_callbacks(f: &mut Fighter);
    /// The parts entry (an index into fp->parts) the bow hangs from and
    /// appears at.
    fn bow_part(assets: &FighterAssets) -> u8;
    /// The parts entry the arrow is nocked at and shot from.
    fn arrow_part(assets: &FighterAssets) -> u8;
    /// The arrow's tip and tail while drawn (it_802A8398's arguments).
    fn aim_points(f: &mut Fighter, assets: &FighterAssets) -> (Vec3, Vec3);
    /// The shot's tip and tail before their z is cleared.
    fn shot_points(f: &mut Fighter, assets: &FighterAssets) -> (Vec3, Vec3);
}

/// Link's and Young Link's own: the rows from ftLk_MS_SpecialNStart, the
/// thumbs (through ftParts_GetBoneIndex).
impl<C: LinkFamily> BowUser for C {
    const START: ActionId = crate::FamilyState::SpecialNStart.action();
    const START_ANIMATION: i32 = crate::FamilyState::SpecialNStart.animation();
    fn attributes(f: &Fighter) -> BowAttributes {
        f.character.get::<C>().attributes().bow
    }
    fn scratch(f: &mut Fighter) -> &mut Bow {
        &mut f.character.get_mut::<C>().specials().bow
    }
    fn state(f: &Fighter) -> &Bow {
        &f.character.get::<C>().specials_ref().bow
    }
    /// setCallbacks: take_dmg_cb / death2_cb = ftLk_800EAF58.
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<C>().specials().removal_armed = true;
    }
    /// isDrawback: RThumbNb.
    fn bow_part(assets: &FighterAssets) -> u8 {
        mapped(assets, FtPart::RThumbNb)
    }
    /// isDrawn: LThumbNb.
    fn arrow_part(assets: &FighterAssets) -> u8 {
        mapped(assets, FtPart::LThumbNb)
    }
    /// animate: the arrow at the right thumb, its tail at the left. mv
    /// x8/x14 (the draw's angle) feed nothing else.
    fn aim_points(f: &mut Fighter, assets: &FighterAssets) -> (Vec3, Vec3) {
        let tail = joint(f, mapped(assets, FtPart::LThumbNb));
        let tip = joint(f, mapped(assets, FtPart::RThumbNb));
        (tip, tail)
    }
    /// doEndColl: the right thumb and the left.
    fn shot_points(f: &mut Fighter, assets: &FighterAssets) -> (Vec3, Vec3) {
        let tip = joint(f, mapped(assets, FtPart::RThumbNb));
        let tail = joint(f, mapped(assets, FtPart::LThumbNb));
        (tip, tail)
    }
}

/// ftParts_GetBoneIndex(fp, part).
fn mapped(assets: &FighterAssets, part: FtPart) -> u8 {
    assets.parts.joint(part).expect("Link thumb part")
}

/// lb_8000B1CC(fp->parts[index].joint, NULL, &pos).
pub fn joint(f: &mut Fighter, index: u8) -> Vec3 {
    let c = &mut f.core;
    melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(index),
        Vec3::ZERO,
    )
}

const fn action<C: BowUser>(offset: u16) -> ActionId {
    ActionId(C::START.0 + offset)
}
fn on_side<C: BowUser>(offset: u16, air: bool) -> ActionId {
    action::<C>(if air { row::AIR + offset } else { offset })
}

fn bow_kind<C: BowUser>(f: &Fighter) -> ItemKind {
    ItemKind::try_from(C::attributes(f).bow_item as i32).expect("bow kind")
}
fn arrow_kind<C: BowUser>(f: &Fighter) -> ItemKind {
    ItemKind::try_from(C::attributes(f).arrow_item as i32).expect("arrow kind")
}

/// ftLk_SpecialN_Enter (800ED9E4) / ftLk_SpecialAirN_Enter (800EDAB8);
/// Kirby's ftKb_SpecialNLk800FB880 / 800FBA00.
pub fn enter<C: BowUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    {
        let b = C::scratch(f);
        b.drawing = false;
        b.charge = 0.0;
    }
    f.commands.variables[..4].fill(0);
    if !air {
        // ftCommon_8007D7FC, then no vertical speed.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    common::change(f, on_side::<C>(row::START, air), 0, 0.0, a).expect("bow assets");
    C::arm_callbacks(f);
    set_draw_rate::<C>(f);
    // ftAnim_8006EBA4.
    f.step_animation(a);
    // isDrawback: a bow still out ends the special at once.
    if !take_bow::<C>(f, a) {
        forget_articles::<C>(f);
        if air {
            fall_after::<C>(f, a).expect("bow assets");
        } else {
            f.change_motion_state(CommonMotionState::Wait.into(), a)
                .expect("bow assets");
        }
    }
}

/// ftAnim_SetAnimRate(specialn_anim_rate).
fn set_draw_rate<C: BowUser>(f: &mut Fighter) {
    let rate = C::attributes(f).draw_animation_rate;
    let c = &mut f.core;
    c.animation.set_rate(&mut c.skeleton, rate, false);
}

/// isDrawback (Kirby's 800FB5F4): the bow in the right hand (it_802AF1A4,
/// Item_InitSpawnOnPlane at the bow's joint, taken there) unless one is
/// out already.
fn take_bow<C: BowUser>(f: &mut Fighter, a: &FighterAssets) -> bool {
    if C::state(f).bow_out {
        return false;
    }
    let part = C::bow_part(a);
    let position = joint(f, part);
    let kind = bow_kind::<C>(f);
    let c = &mut f.core;
    let spawn = SpawnItem::held(kind, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    C::scratch(f).bow_out = true;
    C::arm_callbacks(f);
    true
}

/// isDrawn (Kirby's 800FB500): on command variable 0, the arrow at its
/// joint (it_802A83E0, z 0, taken there) unless one is on. True when that
/// fails.
fn nock<C: BowUser>(f: &mut Fighter, a: &FighterAssets) -> bool {
    if f.commands.variables[var::NOCK] != 1 || C::state(f).arrow_out {
        return false;
    }
    f.commands.variables[var::NOCK] = 0;
    let part = C::arrow_part(a);
    let mut position = joint(f, part);
    position.z = 0.0;
    let kind = arrow_kind::<C>(f);
    let c = &mut f.core;
    let spawn = SpawnItem::attached(kind, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    C::scratch(f).arrow_out = true;
    C::arm_callbacks(f);
    false
}

/// animate (the inline in the draw's Anim callbacks): the arrow follows
/// the user's aim (it_802A8398).
fn aim<C: BowUser>(f: &mut Fighter, a: &FighterAssets) {
    let (tip, tail) = C::aim_points(f, a);
    if C::state(f).arrow_out {
        let kind = arrow_kind::<C>(f);
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind,
            control: ItemControl::Aim { tip, tail },
        });
    }
}

/// ftLk_SpecialN_UnsetArrow / UnsetFv14 (Kirby's 800FB418 / 800FB444):
/// the user lets go of its pointers; the articles carry on by themselves.
fn forget_articles<C: BowUser>(f: &mut Fighter) {
    let b = C::scratch(f);
    b.arrow_out = false;
    b.bow_out = false;
}

/// ftLk_SpecialN_ProcessFv10 / ProcessFv14 (ftLk_800EAF58; Kirby's
/// 800FB800 / 800FB840): a hit takes the arrow (it_802A8A7C) and the bow
/// (it_802AF304).
pub fn remove_articles<C: BowUser>(f: &mut Fighter) {
    let (arrow, bow_out) = {
        let b = C::scratch(f);
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
pub fn article_destroyed<C: BowUser>(f: &mut Fighter, kind: ItemKind) {
    if kind == bow_kind::<C>(f) {
        C::scratch(f).bow_out = false;
    } else if kind == arrow_kind::<C>(f) {
        C::scratch(f).arrow_out = false;
    }
}

/// ftLk_SpecialN_GetIndex (Kirby's ftKb_SpecialNLk800FB394): the row the
/// bow follows, while it is out.
pub fn stage<C: BowUser>(f: &Fighter) -> Option<u8> {
    if !C::state(f).bow_out {
        return None;
    }
    let first = C::START.0;
    let action = f.motion_state.action.0;
    (first..first + row::COUNT)
        .contains(&action)
        .then(|| (action - first) as u8)
}

/// Command variable 2 starts the charge (once).
fn start_charge<C: BowUser>(f: &mut Fighter) {
    if f.commands.variables[var::DRAW] != 0 && !C::state(f).drawing {
        C::scratch(f).drawing = true;
        f.commands.variables[var::DRAW] = 0;
    }
}

/// ftLk_SpecialNStart_Anim (800EDB84) / ftLk_SpecialAirNStart_Anim; Kirby's
/// ftKb_LkSpecialNStart_Anim / LkSpecialAirNStart_Anim.
fn start_anim<C: BowUser, const AIR: bool>(
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
        common::change(f, on_side::<C>(row::LOOP, AIR), ROW_FLAGS, 0.0, p.assets)?;
        C::arm_callbacks(f);
        f.step_animation(p.assets);
    }
    Ok(None)
}

/// ftLk_SpecialNLoop_Anim / ftLk_SpecialAirNLoop_Anim (Kirby's
/// ftKb_LkSpecial(Air)NLoop_Anim).
fn loop_anim<C: BowUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    aim::<C>(f, p.assets);
    Ok(None)
}

/// The aerial draw's end: Fall, or FallSpecial with the attribute's
/// landing lag (ftCo_80096900).
fn fall_after<C: BowUser>(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let lag = C::attributes(f).air_landing_lag;
    if lag == 0.0 {
        f.change_motion_state(CommonMotionState::Fall.into(), a)
    } else {
        f.enter_special_fall(a, true, false, true, 1.0, lag)
    }
}

/// ftLk_SpecialNEnd_Anim / ftLk_SpecialAirNEnd_Anim (Kirby's
/// ftKb_LkSpecial(Air)NEnd_Anim): at the animation's end the pointers go
/// (the bow ends itself) and Wait or the fall. The model part update
/// (updateParts) is drawing only.
fn end_anim<C: BowUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if AIR {
            C::scratch(f).arrow_out = false;
        }
        C::scratch(f).bow_out = false;
        if AIR {
            fall_after::<C>(f, p.assets)?;
        } else {
            f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
        }
    }
    Ok(None)
}

/// Releasing B ends the draw (the charge's cap and state switch shared by
/// ftLk_SpecialNStart_IASA and the loop's doLoopIASA).
fn release<C: BowUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if !f.input.current.held.intersects(melee_ft::input::Buttons::B) {
        common::change(f, on_side::<C>(row::END, air), ROW_FLAGS, 0.0, a).expect("bow assets");
        C::arm_callbacks(f);
    }
}

/// ftLk_SpecialNStart_IASA (800EDD1C) / ftLk_SpecialAirNStart_IASA: once
/// drawing, the charge counts up to the attribute's cap.
fn start_input<C: BowUser, const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if !C::state(f).drawing {
        return;
    }
    let max = C::attributes(f).max_charge;
    let b = C::scratch(f);
    b.charge += 1.0;
    if b.charge > max {
        b.charge = max;
    }
    release::<C>(f, AIR, p.assets);
}

/// ftLk_SpecialNLoop_IASA / ftLk_SpecialAirNLoop_IASA (doLoopIASA): full
/// charge.
fn loop_input<C: BowUser, const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    let max = C::attributes(f).max_charge;
    C::scratch(f).charge = max;
    release::<C>(f, AIR, p.assets);
}

/// doEndColl (Kirby's 800FB6DC): on command variable 1 with the arrow on,
/// it flies from the tip (its tail behind, both at z 0) at 5 degrees with
/// the charge (itLinkArrow_802A850C), and the user lets go of it.
fn shoot<C: BowUser>(f: &mut Fighter, a: &FighterAssets) {
    if f.commands.variables[var::SHOOT] != 1 || !C::state(f).arrow_out {
        return;
    }
    f.commands.variables[var::SHOOT] = 0;
    // Fighter_procMap placed the model at this frame's position before the
    // collision callback (air::begin_map, which the port runs after).
    let c = &mut f.core;
    c.skeleton.set_translate(c.animation.root, &c.physics.position);
    let (mut tip, mut tail) = C::shot_points(f, a);
    tip.z = 0.0;
    tail.z = 0.0;
    let charge = C::state(f).charge;
    let max_charge = C::attributes(f).max_charge;
    let holder = f.core.item_holder(C::arrow_part(a), a);
    // ftLib_80086630: the hand's world matrix, set up on demand.
    let hand = *holder.skeleton.get_mtx(holder.part);
    let (center, attack, attack_stale) = (holder.center, holder.attack, holder.attack_stale);
    let kind = arrow_kind::<C>(f);
    let launch = Launch {
        velocity: Vec3::ZERO,
        offset: Vec3::ZERO,
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
    C::scratch(f).arrow_out = false;
    // doEndColl keeps fp->item_gobj across the shot, then
    // ftpickupitem_80094818(gobj, false) poses the hand for it again.
    f.pose_hand_for_held_item(a);
}

/// ftLk_SpecialNStart_Coll / Loop_Coll (doColl): off the floor, the
/// aerial row at the same frame.
fn ground_collision<C: BowUser, const ROW: u16>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("bow assets");
        switch_side::<C>(f, action::<C>(row::AIR + ROW), assets, false)?;
    }
    Ok(())
}
/// ftCommon_8007D5D4 / 8007D7FC, the counterpart at the current frame,
/// the callbacks and one animation step.
fn switch_side<C: BowUser>(f: &mut Fighter, next: ActionId, a: &FighterAssets, land: bool) -> Result<()> {
    if land {
        f.land();
    } else {
        f.leave_ground();
    }
    let frame = f.animation.frame;
    common::change(f, next, GROUND_AIR_FLAGS, frame, a)?;
    C::arm_callbacks(f);
    f.step_animation(a);
    Ok(())
}
/// ftLk_SpecialAirNStart_Coll / AirNLoop_Coll (doAirColl): landing, the
/// grounded row at the same frame.
fn air_collision<C: BowUser, const ROW: u16>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("bow assets");
        switch_side::<C>(f, action::<C>(ROW), assets, true)?;
    }
    Ok(())
}
/// ftLk_SpecialNEnd_Coll (800EE1D8): the shot, then doColl.
fn end_collision<C: BowUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    shoot::<C>(f, p.assets.expect("bow assets"));
    ground_collision::<C, { row::END }>(f, p)
}
/// ftLk_SpecialAirNEnd_Coll: the shot, then doAirColl.
fn air_end_collision<C: BowUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    shoot::<C>(f, p.assets.expect("bow assets"));
    air_collision::<C, { row::END }>(f, p)
}

fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

const fn bow_row<C: BowUser>(
    offset: u16,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: action::<C>(offset),
        id: CommonMotionState::None,
        animation: C::START_ANIMATION + offset as i32,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The six rows: start, loop, end, then the aerial three.
pub const fn rows<C: BowUser>() -> [MotionRow; 6] {
    [
        bow_row::<C>(
            row::START,
            start_anim::<C, false>,
            start_input::<C, false>,
            ground_physics,
            ground_collision::<C, { row::START }>,
        ),
        bow_row::<C>(
            row::LOOP,
            loop_anim::<C>,
            loop_input::<C, false>,
            ground_physics,
            ground_collision::<C, { row::LOOP }>,
        ),
        bow_row::<C>(
            row::END,
            end_anim::<C, false>,
            no_input,
            ground_physics,
            end_collision::<C>,
        ),
        bow_row::<C>(
            row::AIR + row::START,
            start_anim::<C, true>,
            start_input::<C, true>,
            air_physics,
            air_collision::<C, { row::START }>,
        ),
        bow_row::<C>(
            row::AIR + row::LOOP,
            loop_anim::<C>,
            loop_input::<C, true>,
            air_physics,
            air_collision::<C, { row::LOOP }>,
        ),
        bow_row::<C>(
            row::AIR + row::END,
            end_anim::<C, true>,
            no_input,
            air_physics,
            air_end_collision::<C>,
        ),
    ]
}
