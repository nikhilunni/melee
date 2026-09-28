//! The chain's four hitboxes. Retail toggles a capsule's state
//! (lbColl_80008434 on, lbColl_80008428 off) and keeps its data either
//! way; the port's fighter holds only live capsules, so an off capsule
//! waits in the chain's `parked` slot until it comes back on.
use super::{side, SpecialSide};
use hsd_types::Vec3;
use melee_coll::hitbox::{CapsulePhase, HitCapsule};
use melee_ft::fighter::Fighter;

/// Before the script runs (a ClearHitboxes turns every capsule off in
/// retail, data kept), remember each live capsule.
pub(super) fn park(f: &mut Fighter) {
    for i in 0..4 {
        if let Some(hit) = &f.commands.hitboxes[i] {
            let hit = hit.clone();
            side(f).chain.parked[i] = Some(hit);
        }
    }
}

/// The capsule `i`, on or off.
fn capsule<'a>(
    hitboxes: &'a mut [Option<HitCapsule>],
    s: &'a mut SpecialSide,
    i: usize,
) -> Option<&'a mut HitCapsule> {
    match &mut hitboxes[i] {
        Some(hit) => Some(hit),
        None => s.chain.parked[i].as_mut(),
    }
}

/// ftSk_SpecialS_ZeroHitboxPositions: x58 and x4C cleared.
fn zero(hit: &mut HitCapsule) {
    hit.position = Vec3::ZERO;
    hit.previous_position = Vec3::ZERO;
}

/// ftSk_SpecialS_80110AEC (80110AEC): every capsule on (lbColl_80008434)
/// with its positions cleared; its victims stay.
pub(super) fn enable_all(f: &mut Fighter) {
    for i in 0..4 {
        if f.commands.hitboxes[i].is_none() {
            f.commands.hitboxes[i] = side(f).chain.parked[i].take();
        }
        if let Some(hit) = &mut f.commands.hitboxes[i] {
            hit.phase = CapsulePhase::Enabled;
            zero(hit);
        }
    }
}

/// ftSeakSpecialS_LoopChainHitCollisions: every capsule off, its victims
/// forgotten (lbColl_80008440), its positions cleared.
pub(super) fn disable_all_forgetting(f: &mut Fighter) {
    for i in 0..4 {
        let (hitboxes, s) = split(f);
        if let Some(hit) = capsule(hitboxes, s, i) {
            hit.clear_victims();
            zero(hit);
        }
        park_live(f, i);
    }
}

/// ftColl_8007AFF8 (8007AFF8): every capsule off, its data kept.
pub(super) fn disable_all(f: &mut Fighter) {
    for i in 0..4 {
        park_live(f, i);
    }
}

fn park_live(f: &mut Fighter, i: usize) {
    if let Some(hit) = f.commands.hitboxes[i].take() {
        side(f).chain.parked[i] = Some(hit);
    }
}

fn split(f: &mut Fighter) -> (&mut [Option<HitCapsule>], &mut SpecialSide) {
    let hitboxes = &mut f.core.commands.hitboxes[..];
    let s = &mut f.character.get_mut::<crate::init::Sheik>().special_side;
    (hitboxes, s)
}

/// ftSk_SpecialS_UpdateHitboxes (80110A1C): once the script's var 0 is
/// set, the chain's point for capsule `i`; a point off the origin becomes
/// the capsule's world position (ftColl_8007B8A8, jobj NULL), its sweep
/// starting there if it had none.
pub(super) fn set_point(f: &mut Fighter, i: usize, point: Vec3) {
    if f.commands.variables[0] == 0 {
        return;
    }
    let (hitboxes, s) = split(f);
    s.points[i] = point;
    if point.x == 0.0 && point.y == 0.0 {
        return;
    }
    if let Some(hit) = capsule(hitboxes, s, i) {
        hit.world = true;
        hit.descriptor.offset = point;
        hit.position = point;
        let previous = hit.previous_position;
        if previous.x == 0.0 && previous.y == 0.0 && previous.z == 0.0 {
            hit.previous_position = point;
        }
    }
}
