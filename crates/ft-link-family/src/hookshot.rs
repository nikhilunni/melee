//! The hookshot from its thrower's side: the standing and dash grabs'
//! frames (ftCo_0D8E.c fn_800D8EC8 / fn_800D9228), the aerial hookshot
//! (ftCo_AirCatch.c), the catch's reel-in (fn_800D9CE8's Link arm), the
//! article's per-state steps the thrower's accessory proc runs
//! (it_802A7AF0 -> fn_802A2E4C..it_802A3500, and it_802A7B34 in hitlag)
//! and its removal (it_802A2B10 / it_802A7AAC).
//!
//! The chain ([`Chain`]) lives here because retail runs every step of it
//! from the thrower's procs; the article (it-link) only hangs from the hand
//! and reports which step its physics proc installed.
use crate::{attributes::HookshotFrames, FamilyState, LinkFamily};
use hsd_types::{Mtx, Vec3};
use it_link::hookshot::{
    chain::{Chain, ChainAttributes, Extension, Throw},
    motion,
};
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    Fighter,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_mp::CollMap;
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// fp->parts[0x8B]: the claw's joint takes this parts slot, where the
/// grab's script puts its catch capsule (the Catch box names bone 139).
pub const CLAW_PART: usize = 0x8B;

/// it_804D6D48: the claw's distance along the thumb's z axis (it_802A2BA4
/// sets 6.0 for either kind).
const CLAW_OFFSET: f32 = 6.0;
/// The grabs' wall probe reaches 8 (in double) past the thumb, facing.
const WALL_PROBE: f64 = 8.0;

/// fp->u.lk.xC's article while it exists, and what the thrower keeps of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hookshot {
    /// linkhookshot.x10: the step the article's physics proc installed
    /// last (its state), none before the first (it_802A2418).
    pub step: Option<u8>,
    /// linkhookshot.x14: a caught fighter is reeled all the way in.
    pub reeled_in: bool,
}

/// The throwing motions (link_fighter_compare: Catch, CatchDash, AirCatch
/// and AirCatchHit), and which frames each counts.
fn throw_frames<C: LinkFamily>(f: &Fighter) -> Option<HookshotFrames> {
    let h = &f.character.get::<C>().attributes().hookshot;
    let action = f.motion_state.action;
    if action == CommonMotionState::Catch.into() {
        Some(h.grab)
    } else if action == CommonMotionState::CatchDash.into() {
        Some(h.dash_grab)
    } else if action == FamilyState::AirCatch.action() {
        Some(h.aerial)
    } else {
        None
    }
}

fn throwing(f: &Fighter) -> bool {
    let action = f.motion_state.action;
    action == CommonMotionState::Catch.into()
        || action == CommonMotionState::CatchDash.into()
        || action == FamilyState::AirCatch.action()
        || action == FamilyState::AirCatchHit.action()
}

/// The frame counter the grabs and the aerial hookshot keep in mv+0,
/// against the motion's launch frame (an int, converted).
fn throw_state<C: LinkFamily>(f: &Fighter) -> Throw {
    let timer = f.character.get::<C>().specials_ref().hookshot_timer;
    match throw_frames::<C>(f) {
        Some(frames) => Throw {
            launching: timer == frames.launch as f32,
            before_launch: timer < frames.launch as f32,
        },
        None => Throw {
            launching: false,
            before_launch: false,
        },
    }
}

fn kind<C: LinkFamily>(f: &Fighter) -> ItemKind {
    ItemKind::try_from(f.character.get::<C>().attributes().hookshot_item as i32)
        .expect("hookshot item kind")
}

fn thumb(assets: &FighterAssets) -> u8 {
    assets
        .parts
        .joint(FtPart::RThumbNb)
        .expect("Link thumb part")
}

/// fp->parts[RThumbNb].joint's world matrix (HSD_JObjSetupMatrix).
fn thumb_matrix(f: &mut Fighter, assets: &FighterAssets) -> Mtx {
    let c = &mut f.core;
    let joint = c.animation.parts[usize::from(thumb(assets))].joint;
    *c.skeleton.get_mtx(joint)
}

/// it_802A2EE4_inline: the thumb's matrix times a translation of
/// it_804D6D48 along z (PSMTXConcat), the hand end of the chain.
fn anchor(f: &mut Fighter, assets: &FighterAssets) -> Vec3 {
    let hand = thumb_matrix(f, assets);
    let mut offset = Mtx::default();
    hsd_anim::mtx::mtx_identity(&mut offset);
    offset.0[0][3] = 0.0;
    offset.0[1][3] = 0.0;
    offset.0[2][3] = CLAW_OFFSET;
    let mut out = Mtx::default();
    hsd_anim::mtx::mtx_concat(&hand, &offset, &mut out);
    Vec3::new(out.0[0][3], out.0[1][3], out.0[2][3])
}

fn set_article_state<C: LinkFamily>(f: &mut Fighter, state: u16) {
    let kind = kind::<C>(f);
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind,
        control: ItemControl::Motion(state),
    });
}

/// it_802A76EC / it_802A7764 / it_802A7840: the state, and the thrower's
/// hitboxes off (ftColl_8007AFF8).
fn set_article_state_clearing_hits<C: LinkFamily>(f: &mut Fighter, state: u16) {
    set_article_state::<C>(f, state);
    f.commands.hitboxes.fill(None);
}

/// it_802A2BA4 (802A2BA4): the article at the thumb (Item_80268B18 with
/// its initial pass, then Item_8026AB54 into the hand at RThumbNb) and its
/// chain (it_802A2568; the aerial hookshot's is longer).
fn create<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets, map: &CollMap) {
    assert!(throwing(f), "it_802A2BA4 outside a throwing motion");
    let hand = thumb(assets);
    let c = &mut f.core;
    let position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(hand),
        Vec3::ZERO,
    );
    let kind = kind::<C>(f);
    let c = &mut f.core;
    let spawn = SpawnItem::attached(kind, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: hand,
        hold: false,
    });
    let scale = f.core.player.scale;
    let aerial = f.motion_state.action == FamilyState::AirCatch.action();
    let words = f.character.get::<C>().attributes().hookshot_article;
    let specials = f.character.get_mut::<C>().specials();
    specials.chain.lay_out(
        ChainAttributes::for_throw(&words, aerial, scale),
        scale,
        map,
    );
    specials.hookshot = Some(Hookshot::default());
    f.core.tether_article = true;
}

/// it_802A2B10 (802A2B10): the article goes; the accessory callbacks with
/// it. fp->parts[139] keeps pointing at the claw's freed joint.
pub fn remove<C: LinkFamily>(f: &mut Fighter) {
    if f.character
        .get_mut::<C>()
        .specials()
        .hookshot
        .take()
        .is_none()
    {
        return;
    }
    let kind = kind::<C>(f);
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind,
        control: ItemControl::Remove,
    });
    f.core.tether_article = false;
}

/// The frames of fn_800D8EC8 / fn_800D9228 / ftCo_AirCatch_Anim, after
/// mv+0 counts this one (a double add). Returns true when it changed the
/// motion.
fn count_frame<C: LinkFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    map: &mut CollMap,
    probe_wall: bool,
    on_fail: fn(&mut Fighter, &FighterAssets) -> Result<()>,
) -> Result<bool> {
    let frames = throw_frames::<C>(f).expect("hookshot frames outside a throwing motion");
    let timer = {
        let s = f.character.get_mut::<C>().specials();
        s.hookshot_timer = (f64::from(s.hookshot_timer) + 1.0) as f32;
        s.hookshot_timer
    };
    if timer == frames.spawn as f32 {
        create::<C>(f, assets, map);
        return Ok(false);
    }
    if timer <= frames.spawn as f32 || timer > frames.remove as f32 {
        return Ok(false);
    }
    if timer == frames.launch as f32 {
        if probe_wall && wall_in_reach(f, assets, map) {
            remove::<C>(f);
            on_fail(f, assets)?;
            return Ok(true);
        }
        launch::<C>(f);
    } else if timer == frames.reel as f32 {
        // it_802A77DC; ft_PlaySFX.
        set_article_state::<C>(f, motion::REELING);
    } else if timer == frames.remove as f32 {
        remove::<C>(f);
    }
    Ok(false)
}

/// The launch frame's probe: mpCheckAllRemap across the thumb's height
/// from cur_pos.x to 8 past the thumb, facing (fmadd in double, then frsp).
fn wall_in_reach(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) -> bool {
    let hand = thumb_matrix(f, assets);
    let facing = f64::from(f.physics.facing);
    let scale = f64::from(f.core.player.scale);
    // retail 800D90E4 / 800C3F68: fmul, fmadd, frsp.
    let reach = gekko_math::fma::fmadd(WALL_PROBE * facing, scale, f64::from(hand.0[0][3])) as f32;
    let height = hand.0[1][3];
    map.check_all_remap(-1, -1, f.collision.data.cur_pos.x, height, reach, height)
        .is_some()
}

/// it_802A78B8: the claw's throw velocity x38 by facing (fmuls), plus the
/// fighter's own pos_delta.x in the air (fadds); state 1.
fn launch<C: LinkFamily>(f: &mut Fighter) {
    let facing = f.physics.facing;
    let aerial = f.motion_state.action == FamilyState::AirCatch.action();
    let drift = f.physics.position_delta.x;
    let specials = f.character.get_mut::<C>().specials();
    let mut x = specials.chain.attributes.throw_speed * facing;
    if aerial {
        x += drift;
    }
    specials.chain.throw(Vec3::new(x, 0.0, 0.0));
    // ft_PlaySFX(0x27149 / 0x111B9).
    set_article_state::<C>(f, motion::EXTENDING);
}

/// fn_800D8EC8 (800D8EC8) / fn_800D9228 (800D9228): the standing grab
/// checks the wall at the launch frame (ft_8008A2BC on a hit), the dash
/// grab does not.
pub fn grab_animation<C: LinkFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    map: &mut CollMap,
) -> Result<bool> {
    let standing = f.motion_state.action == CommonMotionState::Catch.into();
    count_frame::<C>(f, assets, map, standing, |f, a| {
        f.change_motion_state(CommonMotionState::Wait.into(), a)
    })
}

/// ftCo_800D8C54 / ftCo_800C3BE8: mv+0 restarts.
pub fn restart_count<C: LinkFamily>(specials: &mut crate::Specials) {
    specials.hookshot_timer = 0.0;
}

/// ftCo_AirCatch_Anim (800C3E24), Link's arm, then the end check
/// (ftCo_800968C8 when no frames remain).
pub fn aerial_animation<C: LinkFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    map: &mut CollMap,
) -> Result<()> {
    if count_frame::<C>(f, assets, map, true, |f, a| {
        f.enter_ordinary_special_fall(a)
    })? {
        return Ok(());
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        f.enter_ordinary_special_fall(assets)?;
    }
    Ok(())
}

/// fn_800D949C (Catch_Coll's departure) and it_802A7AAC.
pub fn release<C: LinkFamily>(f: &mut Fighter) {
    remove::<C>(f);
}

/// fn_800D9CE8's Link arm: the caught fighter is reeled in (it_802A7840:
/// state 5, the thrower's hitboxes off) and pulled toward the claw
/// (mv.co.capturedamage.x18 = the claw's joint).
pub fn caught<C: LinkFamily>(f: &mut Fighter) {
    assert!(
        f.character.get::<C>().specials_ref().hookshot.is_some(),
        "fn_800D9CE8: a Link's catch without its hookshot"
    );
    set_article_state_clearing_hits::<C>(f, motion::REELING_CATCH);
    f.core.holds_by_graft = true;
}

/// ftCo_CatchPull_Anim's Link arm: the pull ends once the hookshot is gone
/// or has reeled the catch all the way in.
pub fn pull_done<C: LinkFamily>(f: &Fighter) -> bool {
    f.character
        .get::<C>()
        .specials_ref()
        .hookshot
        .is_none_or(|h| h.reeled_in)
}

/// The step the article's physics proc installed (OwnerRequest::ArticleStep).
pub fn install_step<C: LinkFamily>(f: &mut Fighter, step: u8) {
    if let Some(h) = &mut f.character.get_mut::<C>().specials().hookshot {
        h.step = Some(step);
    }
}

/// Fighter_CallAcessoryCallbacks_8006C624: accessory2 (it_802A7AF0, the
/// installed step) or, in hitlag, accessory3 (it_802A7B34).
pub fn accessory<C: LinkFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    map: &mut CollMap,
    in_hitlag: bool,
) {
    let Some(hookshot) = f.character.get::<C>().specials_ref().hookshot else {
        return;
    };
    if in_hitlag {
        hitlag_step::<C>(f, assets, map);
        return;
    }
    let Some(step) = hookshot.step else {
        return;
    };
    match u16::from(step) {
        motion::HELD => held_step::<C>(f, assets),
        motion::EXTENDING => extending_step::<C>(f, assets, map),
        motion::FALLING_BACK => falling_back_step::<C>(f, assets, map),
        motion::HANGING => hanging_step::<C>(f, assets, map),
        motion::REELING => reeling_step::<C>(f, assets, map),
        motion::REELING_CATCH => catch_reeling_step::<C>(f, assets, map),
        _ => unimplemented!("it_802A3630 / it_802A3828 / it_802A39FC: the hookshot's wall hang"),
    }
    // HSD_JObjSetTranslate(gobj->hsd_obj, &fp->cur_pos).
    let c = &mut f.core;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
}

/// it_802A7168: the chain's models; the claw joint the fighter's bone 139
/// reads ends at the claw.
fn place_models<C: LinkFamily>(f: &mut Fighter) {
    let claw = f
        .character
        .get::<C>()
        .specials_ref()
        .chain
        .claw_joint_position();
    f.core.grafted_part = Some(melee_ft::fighter::GraftedPart {
        part: CLAW_PART,
        position: claw,
    });
}

/// fn_802A2E4C (802A2E4C), state 0: the claw at the thumb (it_802A6944);
/// gone once the thrower stops throwing.
fn held_step<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let anchor = anchor(f, assets);
    f.core.grafted_part = Some(melee_ft::fighter::GraftedPart {
        part: CLAW_PART,
        position: anchor,
    });
    if !throwing(f) {
        remove::<C>(f);
    }
}

/// it_802A2EE4 (802A2EE4), state 1: flying out (it_802A4BFC). Off a
/// wall it rebounds (state 2), fully out it hangs (state 3); either way
/// the thrower becomes grabbable again (ftCommon_8007E2F4(fp, 0)).
fn extending_step<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) {
    let below_out = f.character.get::<C>().specials_ref().chain.below_claw_out();
    if !throwing(f) && !below_out {
        remove::<C>(f);
        return;
    }
    let anchor = anchor(f, assets);
    let throw = throw_state::<C>(f);
    let result = f
        .character
        .get_mut::<C>()
        .specials()
        .chain
        .extend(anchor, throw, map);
    match result {
        Extension::Wall => {
            if f.motion_state.action == FamilyState::AirCatch.action() {
                unimplemented!("it_802A2EE4: the aerial hookshot's claw in a wall (ftCo_800C3CC0)");
            }
            f.character.get_mut::<C>().specials().chain.rebound();
            set_article_state_clearing_hits::<C>(f, motion::FALLING_BACK);
            f.core.status.grab_exclusions = melee_ft::fighter::ledge::GrabExclusions(0);
        }
        Extension::Extended => {
            set_article_state_clearing_hits::<C>(f, motion::HANGING);
            f.core.status.grab_exclusions = melee_ft::fighter::ledge::GrabExclusions(0);
        }
        Extension::Extending | Extension::Ceiling | Extension::Floor => {}
    }
    place_models::<C>(f);
}

/// fn_802A3110 (802A3110), state 2: falling back from the wall
/// (it_802A5320); it rebounds again off a wall and hangs once all out.
fn falling_back_step<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) {
    let anchor = anchor(f, assets);
    let result = f
        .character
        .get_mut::<C>()
        .specials()
        .chain
        .fall_back(anchor, map);
    match result {
        Extension::Wall => f.character.get_mut::<C>().specials().chain.rebound(),
        Extension::Extended => set_article_state_clearing_hits::<C>(f, motion::HANGING),
        _ => {}
    }
    place_models::<C>(f);
}

/// it_802A3254 (802A3254), state 3: hanging (it_802A5770); reeled in
/// once the thrower stops throwing.
fn hanging_step<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) {
    let anchor = anchor(f, assets);
    f.character
        .get_mut::<C>()
        .specials()
        .chain
        .hang(anchor, map);
    place_models::<C>(f);
    if !throwing(f) {
        set_article_state::<C>(f, motion::REELING);
    }
}

/// fn_802A33A0 (802A33A0), state 4: reeling in at x40 (it_802A5E28);
/// all in, back to state 0 (itLinkHookshot_Logic20_PickedUp), or gone if
/// the thrower stopped throwing.
fn reeling_step<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) {
    let anchor = anchor(f, assets);
    let chain = &mut f.character.get_mut::<C>().specials().chain;
    let speed = chain.attributes.reel_speed;
    if chain.reel(anchor, speed, map) {
        if !throwing(f) {
            remove::<C>(f);
            return;
        }
        set_article_state::<C>(f, motion::HELD);
    }
    place_models::<C>(f);
}

/// it_802A3500 (802A3500), state 5: reeling a catch in at x44
/// (it_802A678C); all in, x14 ends the pull, and the next step removes it.
fn catch_reeling_step<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) {
    if f.character
        .get::<C>()
        .specials_ref()
        .hookshot
        .is_some_and(|h| h.reeled_in)
    {
        remove::<C>(f);
        return;
    }
    let anchor = anchor(f, assets);
    let chain = &mut f.character.get_mut::<C>().specials().chain;
    let speed = chain.attributes.catch_reel_speed;
    if chain.reel(anchor, speed, map) {
        if let Some(h) = &mut f.character.get_mut::<C>().specials().hookshot {
            h.reeled_in = true;
        }
        return;
    }
    place_models::<C>(f);
}

/// it_802A7B34 (802A7B34), accessory3 in hitlag: before the launch the
/// claw stays at the thumb (it_802A6944); after, the chain pays out
/// behind it (it_802A6A78).
fn hitlag_step<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) {
    let anchor = anchor(f, assets);
    let throw = throw_state::<C>(f);
    let at_hand = f
        .character
        .get_mut::<C>()
        .specials()
        .chain
        .hold_in_hitlag(anchor, throw, map);
    if at_hand {
        f.core.grafted_part = Some(melee_ft::fighter::GraftedPart {
        part: CLAW_PART,
        position: anchor,
    });
    } else {
        place_models::<C>(f);
    }
}

/// The claw's chain for tests and tooling.
pub fn chain<C: LinkFamily>(f: &Fighter) -> &Chain {
    &f.character.get::<C>().specials_ref().chain
}
