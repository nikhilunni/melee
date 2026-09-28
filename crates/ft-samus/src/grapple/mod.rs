//! The grapple beam (It_Kind_Samus_GBeam): itsamusgrapple.c's article and
//! the grabs that throw it, ftCo_0D95.c (fn_800D9558, fn_800D9930).
//!
//! The grab counts frames (mv.ca.specials.grav): on the timeline's spawn
//! frame the beam appears at part 51 (it_802B7C18) and hangs from ThrowN;
//! on the throw frame the tip flies out; later it is reeled in and removed.
//! Its rope runs from the fighter's accessory2 (it_802BAC80), by the
//! article's state, and in hitlag from accessory3 (it_802BACC4). The tip's
//! model is fp->parts[0x8B], where the grab's script puts its catch
//! capsule. The article itself only mirrors the state; it never moves.
pub mod air;
pub mod chain;

use crate::init::Samus;
use chain::{Chain, RopeAttributes};
use hsd_types::Vec3;
use melee_ft::{
    fighter::{assets::FighterAssets, Fighter, GraftedPart},
    input::Buttons,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState as S, FtPart, GroundOrAir, ItemKind};

/// fp->parts[0x8B]: the tip's model takes this parts slot (it_802B7C18).
pub const TIP_PART: usize = 0x8B;
/// fp->parts[51]: where the beam appears (fn_800D9558).
const SPAWN_PART: usize = 51;
/// ftSs_MS_AirCatch / AirCatchHit (0x165 / 0x166).
const AIR_CATCH: u16 = 0x165;
const AIR_CATCH_HIT: u16 = 0x166;
/// fn_800D9558: the throw's sparks every third frame from frame 20.
const SPARK_FIRST: i32 = 0x14;
const SPARK_STEP: i32 = 3;
/// efSync_Spawn(0x3F3): a spark at the tip.
const SPARK: u16 = 0x3F3;

/// Item.xDD4_itemVar.samusgrapple's state index (Item_80268E5C).
pub mod state {
    /// Hanging from the hand (fn_802B7E34).
    pub const HELD: u16 = 0;
    /// Thrown (fn_802B805C).
    pub const THROWN: u16 = 1;
    /// Bounced off the map (fn_802B8384).
    pub const BOUNCED: u16 = 2;
    /// Paid out and sagging (fn_802B8524).
    pub const SAGGING: u16 = 3;
    /// Reeled in (fn_802B8684).
    pub const RETRACTING: u16 = 4;
    /// Reeling in a caught fighter (fn_802B8814).
    pub const REELING: u16 = 5;
}

/// Samus's live beam: u.ss.x223C and its item's scratch.
#[derive(Clone, Debug, Default)]
pub struct Grapple {
    pub chain: Box<Chain>,
    /// u.ss.x223C != NULL.
    pub live: bool,
    /// Item_80268E5C's state index (the article's motion).
    pub state: u16,
    /// xDD4 x14: the caught fighter is reeled in.
    pub reeled: bool,
    /// The tip model's world translation, as last posed.
    pub tip: Vec3,
    /// accessory2/death1/accessory3 = it_802BAC80/it_802BAC3C/it_802BACC4.
    pub callbacks: bool,
    /// u.ss.x2240: the hidden beam's button code (up, down, up, A).
    pub code: u8,
}

fn samus(f: &Fighter) -> &Samus {
    f.character.get::<Samus>()
}
fn grapple(f: &mut Fighter) -> &mut Grapple {
    &mut f.character.get_mut::<Samus>().grapple
}

/// samus_grapple_fighter_compare: the motions that keep a beam.
fn in_grapple_motion(f: &Fighter) -> bool {
    matches!(f.motion_state.id, S::Catch | S::CatchDash)
        || matches!(f.motion_state.action.0, AIR_CATCH | AIR_CATCH_HIT)
}

/// fp->parts[ThrowN]'s world translation (the rope's hand end, every
/// link's jobj; HSD_JObjSetupMatrix then mtx[.][3]).
fn hand(f: &mut Fighter) -> Vec3 {
    let c = &mut f.core;
    melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        crate::common::part(FtPart::ThrowN),
        Vec3::ZERO,
    )
}

/// Item_80268E5C on the beam, from its owner's proc.
fn set_state(f: &mut Fighter, state: u16) {
    let player = f.player.id;
    grapple(f).state = state;
    f.core.item_requests.push(ItemRequest::Control {
        owner: player,
        kind: ItemKind::SamusGBeam,
        control: ItemControl::Motion(state),
    });
}

/// it_802BA9B8 / it_802BAA08 / it_802BAA94: states that also clear the
/// fighter's hitboxes (ftColl_8007AFF8).
fn set_state_clearing_hitboxes(f: &mut Fighter, state: u16) {
    set_state(f, state);
    f.core.commands.hitboxes.fill(None);
}

/// it_802B7C18 (802B7C18): the beam at `position`, facing `facing`, its
/// rope built for a grab (type 0) and hung from ThrowN. Only a grapple
/// motion makes one.
fn spawn(f: &mut Fighter, position: Vec3, map: &melee_mp::CollMap) -> bool {
    if !in_grapple_motion(f) {
        return false;
    }
    let player = f.player.id;
    let facing = f.physics.facing;
    let air = f.motion_state.action.0 == AIR_CATCH;
    // x16: the button code's longer beam. Entering a beam resets the code
    // unless it was complete.
    let g = grapple(f);
    if g.code >= 4 {
        unimplemented!("it_802B7C18: the button code's grapple beam (xDD4 x16)");
    }
    g.code = 0;
    let scale = f.player.scale;
    let article = samus(f).grapple_article.clone();
    let (attrs, count) = article.rope(scale, air);
    let g = grapple(f);
    g.chain.build(count, attrs, map);
    g.live = true;
    g.reeled = false;
    g.state = state::HELD;
    g.callbacks = true;
    f.core.tether_article = true;
    let spawn = SpawnItem::attached(ItemKind::SamusGBeam, player, position, facing);
    f.core.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: crate::common::part(FtPart::ThrowN) as u8,
        hold: false,
    });
    true
}

/// it_802B7B84 (802B7B84): the beam's owner lets go of it and it goes
/// (Item_8026A8EC). parts[0x8B] keeps pointing at the freed model.
pub fn remove(f: &mut Fighter) {
    if !grapple(f).live {
        return;
    }
    let player = f.player.id;
    let g = grapple(f);
    g.live = false;
    g.callbacks = false;
    f.core.tether_article = false;
    f.core.item_requests.push(ItemRequest::Control {
        owner: player,
        kind: ItemKind::SamusGBeam,
        control: ItemControl::Remove,
    });
}

/// it_802BAC3C (802BAC3C), death1_cb: the beam goes, or the callbacks do.
pub fn on_death(f: &mut Fighter) {
    if grapple(f).live {
        remove(f);
    } else {
        grapple(f).callbacks = false;
    }
}

/// The grab's timeline (Samus attributes x9C.., xAC..): spawn, throw,
/// retract and removal frames.
fn timeline(f: &Fighter) -> crate::attributes::GrappleTimeline {
    let a = &samus(f).attributes;
    if f.motion_state.id == S::CatchDash {
        a.dash_grab_beam
    } else {
        a.grab_beam
    }
}

/// fn_800D9558 (800D9558) / fn_800D9930 (800D9930), from ftCo_Catch_Anim /
/// ftCo_CatchDash_Anim: the counter rises (a double add); on the spawn
/// frame the beam appears at part 51 (none: Wait); while it lives, sparks
/// every third frame from 20 (six for a grab, four for a dash grab), then
/// the throw (a grab first checks for a wall in front of the hand: blocked,
/// the beam goes and Samus waits), the retraction and the removal. True
/// when Samus left the grab.
pub fn run_timeline(
    f: &mut Fighter,
    assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    rng: &mut gekko_math::HsdRng,
) -> melee_ft::fighter::assets::Result<bool> {
    let dash = f.motion_state.id == S::CatchDash;
    let times = timeline(f);
    let counter = {
        let samus = f.character.get_mut::<Samus>();
        samus.grab_frames = (f64::from(samus.grab_frames) + 1.0) as f32;
        samus.grab_frames
    };
    if counter == times.spawn as f32 {
        let position = {
            let c = &mut f.core;
            melee_ft::fighter::caches::part_position(
                &mut c.skeleton,
                &c.animation,
                SPAWN_PART,
                Vec3::ZERO,
            )
        };
        if !spawn(f, position, map) {
            crate::common::wait(f, assets)?;
            return Ok(true);
        }
        return Ok(false);
    }
    if counter <= times.spawn as f32 || counter > times.remove as f32 {
        return Ok(false);
    }
    assert!(grapple(f).live, "fn_800D9558: a beam that is gone");
    let sparks = if dash { 4 } else { 6 };
    for i in 0..sparks {
        if counter == (i * SPARK_STEP + SPARK_FIRST) as f32 {
            spark(f, rng);
        }
    }
    if counter == times.extend as f32 {
        if !dash && wall_in_front(f, map) {
            remove(f);
            crate::common::wait(f, assets)?;
            return Ok(true);
        }
        // x40 * facing (fmuls), then it_802BAAE4: the tip's velocity and
        // the thrown state.
        let speed = grapple(f).chain.attrs.throw_speed * f.physics.facing;
        let g = grapple(f);
        let tip = g.chain.tip();
        g.chain.links[tip].vel = Vec3::new(speed, 0.0, 0.0);
        set_state(f, state::THROWN);
    } else if counter == times.retract as f32 {
        set_state(f, state::RETRACTING);
    } else if counter == times.remove as f32 {
        remove(f);
    }
    Ok(false)
}

/// fn_800D9558's spark: the tip model's translation with three draws of
/// 4.0 * (r - 0.5f) added (fsubs, then double fmadd, rounded).
fn spark(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    let tip = grapple(f).tip;
    let mut jitter = |base: f32| {
        let r = rng.randf() - 0.5;
        gekko_math::fma::fmadd(4.0, f64::from(r), f64::from(base)) as f32
    };
    let x = jitter(tip.x);
    let y = jitter(tip.y);
    let z = jitter(tip.z);
    f.core
        .effects
        .push(melee_ef::request::EffectRequest::PositionalGenerator {
            id: SPARK,
            position: Vec3::new(x, y, z),
        });
}

/// fn_800D9558: mpCheckAllRemap from Samus's collision centre at the
/// hand's height to 2 * facing * scale past the hand (800D9848: double
/// fmadd, rounded).
fn wall_in_front(f: &mut Fighter, map: &mut melee_mp::CollMap) -> bool {
    let h = hand(f);
    let x0 = f.core.collision.data.cur_pos.x;
    let reach = 2.0 * f64::from(f.physics.facing);
    let x1 = gekko_math::fma::fmadd(reach, f64::from(f.player.scale), f64::from(h.x)) as f32;
    map.check_all_remap(-1, -1, x0, h.y, x1, h.y).is_some()
}

/// samus_grapple_state_sync: up, down, up, then A.
fn read_code(f: &mut Fighter) {
    let held = f.input.current.held;
    let pressed = f.input.pressed;
    let g = grapple(f);
    let step = match g.code {
        0 => held.intersects(Buttons::UP),
        1 => held.intersects(Buttons::DOWN),
        2 => held.intersects(Buttons::UP),
        3 => pressed.intersects(Buttons::A),
        _ => false,
    };
    if step {
        g.code += 1;
    }
}

/// it_802BAC80 (802BAC80), accessory2: the beam's state step
/// (xDD4 unk_10, set by the article's physics).
pub fn accessory(f: &mut Fighter, map: &mut melee_mp::CollMap, rng: &mut gekko_math::HsdRng) {
    let g = grapple(f);
    if !g.callbacks || !g.live {
        return;
    }
    match g.state {
        state::HELD => held(f),
        state::THROWN => thrown(f, map, rng),
        state::BOUNCED => bounced(f, map, rng),
        state::SAGGING => sagging(f, rng),
        state::RETRACTING => retracting(f, rng),
        state::REELING => reeling(f, rng),
        other => unimplemented!("itsamusgrapple.c: the beam's state {other} (tethers)"),
    }
}

/// it_802BACC4 (802BACC4), accessory3 in hitlag: the rope holds (the tip
/// in the hand before the throw's frame), without moving.
pub fn hitlag_accessory(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    if !grapple(f).live {
        return;
    }
    let thrown = {
        let counter = samus(f).grab_frames;
        let t = timeline(f);
        match f.motion_state.id {
            S::Catch | S::CatchDash => counter >= t.extend as f32,
            _ if f.motion_state.action.0 == AIR_CATCH => {
                counter >= samus(f).attributes.air_beam.extend as f32
            }
            _ => true,
        }
    };
    let h = hand(f);
    let in_hand = grapple(f).chain.hold(h, thrown, rng);
    let tip = if in_hand {
        h
    } else {
        grapple(f).chain.tip_position()
    };
    pose(f, tip);
}

/// The tip model's translation, fp->parts[0x8B] for its capsules
/// (it_802A7168, or fn_802B7E34_inline's copy of ThrowN's matrix).
fn pose(f: &mut Fighter, tip: Vec3) {
    grapple(f).tip = tip;
    f.core.grafted_part = Some(GraftedPart {
        part: TIP_PART,
        position: tip,
    });
}

/// fn_802B7E34 (802B7E34): in the hand the tip is ThrowN; outside a
/// grapple motion the beam goes.
fn held(f: &mut Fighter) {
    read_code(f);
    let h = hand(f);
    pose(f, h);
    if !in_grapple_motion(f) {
        remove(f);
    }
}

/// fn_802B805C (802B805C): thrown. Outside a grapple motion a beam whose
/// second link has not joined goes. The throw's walls and floors bounce
/// the tip back (x0 of its speed) into the bounced state; a paid-out rope
/// sags. Each ends the grab's category exclusions (ftCommon_8007E2F4 0).
fn thrown(f: &mut Fighter, map: &mut melee_mp::CollMap, rng: &mut gekko_math::HsdRng) {
    read_code(f);
    let g = grapple(f);
    let tip = g.chain.tip();
    let second = g.chain.links[tip - 1].active;
    if !in_grapple_motion(f) && !second {
        remove(f);
        return;
    }
    let h = hand(f);
    let attach = attach_frame(f);
    let result = grapple(f).chain.throw(h, attach, map, rng);
    match result {
        1 => {
            if f.motion_state.action.0 == AIR_CATCH {
                unimplemented!("fn_802B805C: the aerial tether meeting a wall");
            }
            let g = grapple(f);
            let bounce = g.chain.attrs.bounce;
            g.chain.links[tip].vel.x *= -bounce;
            set_state_clearing_hitboxes(f, state::BOUNCED);
            f.status.grab_exclusions = melee_ft::fighter::ledge::GrabExclusions(0);
        }
        2 => {
            bounce_off_floor(f);
            set_state_clearing_hitboxes(f, state::BOUNCED);
            f.status.grab_exclusions = melee_ft::fighter::ledge::GrabExclusions(0);
        }
        3 => {
            set_state_clearing_hitboxes(f, state::SAGGING);
            f.status.grab_exclusions = melee_ft::fighter::ledge::GrabExclusions(0);
        }
        _ => {}
    }
    let tip_pos = grapple(f).chain.tip_position();
    pose(f, tip_pos);
}

/// The counter's throw frame for the current grab (it_802B9328_attach).
fn attach_frame(f: &Fighter) -> bool {
    let counter = samus(f).grab_frames;
    let a = &samus(f).attributes;
    match f.motion_state.id {
        S::Catch => counter == a.grab_beam.extend as f32,
        S::CatchDash => counter == a.dash_grab_beam.extend as f32,
        _ if f.motion_state.action.0 == AIR_CATCH => counter == a.air_beam.extend as f32,
        _ => false,
    }
}

/// A floor's bounce: the tip's velocity mirrored about the normalized
/// floor normal (lbVector_Normalize, lbVector_Mirror), its y times x0.
fn bounce_off_floor(f: &mut Fighter) {
    let g = grapple(f);
    let normal = melee_lb::vector::normalize(g.chain.tip_collision.floor.normal);
    let tip = g.chain.tip();
    let link = &mut g.chain.links[tip];
    link.vel = melee_lb::vector::mirror(link.vel, normal);
    link.vel.y *= g.chain.attrs.bounce;
}

/// fn_802B8384 (802B8384): the bounced tip flies loose.
fn bounced(f: &mut Fighter, map: &mut melee_mp::CollMap, rng: &mut gekko_math::HsdRng) {
    let h = hand(f);
    let result = grapple(f).chain.bounce(h, map, rng);
    match result {
        1 => {
            let g = grapple(f);
            let tip = g.chain.tip();
            let bounce = g.chain.attrs.bounce;
            g.chain.links[tip].vel.x *= -bounce;
        }
        2 => bounce_off_floor(f),
        3 => set_state_clearing_hitboxes(f, state::SAGGING),
        _ => {}
    }
    let tip_pos = grapple(f).chain.tip_position();
    pose(f, tip_pos);
}

/// fn_802B8524 (802B8524): the rope sags from the hand; outside a grapple
/// motion it is reeled in.
fn sagging(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    let h = hand(f);
    grapple(f).chain.sag(h, rng);
    let tip_pos = grapple(f).chain.tip_position();
    pose(f, tip_pos);
    if !in_grapple_motion(f) {
        set_state(f, state::RETRACTING);
    }
}

/// fn_802B8684 (802B8684): reeled in at x4C (twice that in the air, a
/// double product); once in, outside a grapple motion it goes, otherwise
/// it hangs from the hand again (itSamusGrapple_Logic53_PickedUp).
fn retracting(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    let h = hand(f);
    let speed = grapple(f).chain.attrs.retract_speed;
    let speed = if f.physics.ground_or_air == GroundOrAir::Ground {
        speed
    } else {
        (2.0 * f64::from(speed)) as f32
    };
    if grapple(f).chain.retract(h, speed, rng) {
        if !in_grapple_motion(f) {
            remove(f);
            return;
        }
        set_state(f, state::HELD);
    }
    let tip_pos = grapple(f).chain.tip_position();
    pose(f, tip_pos);
}

/// fn_802B8814 (802B8814): reeling a caught fighter in at x54; once in,
/// x14 (the pull ends), and the next step removes the beam.
fn reeling(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    if grapple(f).reeled {
        remove(f);
        return;
    }
    let h = hand(f);
    let speed = grapple(f).chain.attrs.reel_speed;
    if grapple(f).chain.reel(h, speed, rng) {
        grapple(f).reeled = true;
        return;
    }
    let tip_pos = grapple(f).chain.tip_position();
    pose(f, tip_pos);
}

impl crate::attributes::GrappleArticle {
    /// it_802B75FC (802B75FC): the rope's derived words for a fighter of
    /// y `scale` (the aerial tether's coefficient x60, a grab's x5C), and
    /// its link count: fctiwz(xC * x38 * coefficient / x38) (fmuls, fmuls,
    /// fdivs), the scaled blend above scale 1.
    pub fn rope(&self, scale: f32, air: bool) -> (RopeAttributes, usize) {
        let coeff = if air {
            self.air_coefficient
        } else {
            self.grab_coefficient
        };
        let span = self.span * scale;
        let attrs = RopeAttributes {
            bounce: self.bounce,
            span,
            min_span: self.min_span * scale,
            throw_speed: coeff * (self.throw_speed * scale),
            gravity: self.gravity * scale,
            x48: coeff * (self.x20 * scale),
            retract_speed: coeff * (self.retract_speed * scale),
            x50: coeff * (self.x28 * scale),
            reel_speed: coeff * (self.reel_speed * scale),
            friction: self.friction * scale,
        };
        if scale > 1.0 {
            unimplemented!("it_802B75FC: a scaled fighter's grapple beam");
        }
        let links = self.links as f32 * span;
        let count = gekko_math::msl::fctiwz((links * coeff) / span);
        (attrs, count as usize)
    }
}

/// ftCo_800D8C54's grab entry: the counter (mv.co.catch.x0) restarts, and
/// the grab's rows run the beam (ftCo_Catch_Anim's fn_800D9558,
/// ftCo_Catch_Coll's fn_800D9C64).
pub fn catch_entered(f: &mut Fighter) {
    assert!(
        !grapple(f).live,
        "fn_800D952C: a grab with the beam still out"
    );
    f.character.get_mut::<Samus>().grab_frames = 0.0;
    f.motion_row.anim = catch_animation;
    f.motion_row.collision = catch_collision;
}

/// ftCo_Catch_Anim (800D8CC8) / ftCo_CatchDash_Anim (800D8D14) for Samus:
/// the beam's timeline, then Wait at the end.
fn catch_animation(
    f: &mut Fighter,
    p: melee_ft::fighter::state::AnimationPhase<'_>,
) -> melee_ft::fighter::assets::Result<Option<melee_ft::anim::WaitChoice>> {
    f.step_animation(p.assets);
    f.advance_smash_charge(p.assets);
    if run_timeline(f, p.assets, p.map, p.rng)? {
        return Ok(None);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        crate::common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftCo_Catch_Coll (800D8E08) / ftCo_CatchDash_Coll: ft_800841B8; off the
/// floor fn_800D8E30 removes the beam (fn_800D9C64) and falls.
fn catch_collision(
    f: &mut Fighter,
    mut p: melee_ft::fighter::state::CollisionPhase<'_>,
) -> melee_ft::fighter::assets::Result<()> {
    if !crate::common::stays_grounded(f, &mut p) {
        let assets = p.assets.expect("grab collision assets");
        remove(f);
        crate::common::fall(f, assets)?;
    }
    Ok(())
}

/// fn_800D9CE8's FTKIND_SAMUS arm: the beam reels the catch in
/// (it_802BAA94: state 5, the fighter's hitboxes cleared), the hold point
/// is the tip's model (x18), and the pull ends with the beam
/// (ftCo_CatchPull_Anim's Samus arm).
pub fn catch_pulled(f: &mut Fighter) {
    assert!(grapple(f).live, "fn_800D9CE8: a pull without the beam");
    set_state_clearing_hitboxes(f, state::REELING);
    f.core.holds_by_graft = true;
    f.motion_row.anim = pull_animation;
}

/// ftCo_CatchPull_Anim (800D9F54), FTKIND_SAMUS: CatchWait (fn_800DA1D8)
/// once the beam is gone or has reeled its catch in (xDD4 x14).
fn pull_animation(
    f: &mut Fighter,
    p: melee_ft::fighter::state::AnimationPhase<'_>,
) -> melee_ft::fighter::assets::Result<Option<melee_ft::anim::WaitChoice>> {
    f.step_animation(p.assets);
    f.advance_smash_charge(p.assets);
    let g = &samus(f).grapple;
    if !g.live || g.reeled {
        f.enter_catch_wait(p.assets)?;
    }
    Ok(None)
}
