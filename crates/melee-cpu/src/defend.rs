//! The CPU's defence (behaviour 7): ftCo_800BB9B4 looks for a hitbox about
//! to meet the CPU, ftCo_800BA9A0 shields, rolls, fires the blaster or
//! jumps out of it.
use crate::{
    roll,
    script::{self, Command as C},
    world::{distance, ItemView, Scene},
};
use gekko_math::{fma::fmadds, msl::fctiwz, HsdRng};
use hsd_types::Vec3;
use melee_coll::{
    geometry::{hitbox_pair_contact, Capsule},
    hitbox::CapsulePhase,
};
use melee_ft::fighter::Fighter;
use melee_types::{CommonMotionState as S, GroundOrAir, HitElement, ItemKind};

/// xF8_b12's threats.
pub mod threat {
    pub const NONE: u8 = 0;
    /// A fighter's hitbox (ftCo_800BB104) or move (ftCo_800BB768).
    pub const FIGHTER: u8 = 1;
    /// An item's hitbox (ftCo_800BB220).
    pub const ITEM: u8 = 2;
    /// An absorbable item hitbox Ness at level 4 and up meets far ahead.
    pub const ABSORBABLE: u8 = 3;
}

/// ftCo_800BB104: a fighter's hitbox is swept three of its last moves
/// ahead (800BB1A4..800BB1D0: fmadds).
const HITBOX_LOOKAHEAD: f32 = 3.0;
/// ftCo_800BA9A0: an opponent beyond this x distance is rolled away from,
/// a nearer one rolled through.
const ROLL_AWAY_DISTANCE: f64 = 30.0;

/// ftCo_800BB9B4 (0x800BB9B4): record in xF8_b12 the first threat about
/// to meet the CPU, and in xF0/xF4 its fighter or item: each opponent's
/// hitboxes, then its own moves, then each item with a hitbox. The CPU's
/// body is a point half its height up, with that radius.
pub fn check(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) -> u8 {
    let cpu = &mut fp.core.cpu;
    cpu.xf8_b12 = threat::NONE;
    cpu.threat_fighter = None;
    cpu.threat_item = None;
    if crate::attack::untouchable(fp) || fp.character.table().cpu_profile.ignores_threats {
        return fp.core.cpu.xf8_b12;
    }
    // 800BBA34: fmuls; 800BBA40: fadds.
    let radius = 0.5 * fp.core.cpu.hurtbox_extents[3];
    let p = fp.core.physics.position;
    let body = Vec3::new(p.x, p.y + radius, p.z);
    let fighters = scene.fighters;
    for (index, other) in fighters
        .iter()
        .enumerate()
        .filter_map(|(index, f)| f.map(|f| (index, f)))
    {
        if other.core.player.id == fp.core.player.id {
            continue;
        }
        if hitbox_ahead(fp, other, body, radius) {
            return found_fighter(fp, index, threat::FIGHTER);
        }
        if move_threatens(fp, other, scene) {
            return found_fighter(fp, index, threat::FIGHTER);
        }
    }
    let items = scene.items;
    for item in items {
        if !item.hitbox_active {
            continue;
        }
        let found = item_hitbox_ahead(fp, item, body, radius, rng);
        fp.core.cpu.xf8_b12 = found;
        if found != threat::NONE {
            fp.core.cpu.threat_item = Some(item.id);
            return found;
        }
    }
    fp.core.cpu.xf8_b12
}

fn found_fighter(fp: &mut Fighter, index: usize, found: u8) -> u8 {
    let cpu = &mut fp.core.cpu;
    cpu.xf8_b12 = found;
    cpu.threat_fighter = Some(index);
    found
}

/// The capsule from `position` to `position + frames * (position -
/// previous)`, each axis an fmadds.
fn swept(position: Vec3, previous: Vec3, frames: f32, radius: f32) -> Capsule {
    let ahead = |now: f32, before: f32| fmadds(now - before, frames, now);
    Capsule {
        start: position,
        end: Vec3::new(
            ahead(position.x, previous.x),
            ahead(position.y, previous.y),
            ahead(position.z, previous.z),
        ),
        radius,
    }
}

/// ftCo_800BB104 (0x800BB104): one of `other`'s placed hitboxes (not a
/// grab box, not Rest's sleep box, x43_b2, which the port never sets),
/// that has not already struck the CPU, meets the CPU's body three moves
/// ahead (lbColl_80006094).
fn hitbox_ahead(fp: &Fighter, other: &Fighter, body: Vec3, radius: f32) -> bool {
    other.core.commands.hitboxes.iter().flatten().any(|hit| {
        hit.phase != CapsulePhase::Enabled
            && hit.descriptor.element != HitElement::Catch
            && !hit.victims.contains(&fp.core.spawn_number)
            && hitbox_pair_contact(
                // 800BB1A4/800BB1C4/800BB1D0: fmadds 3.0 * d + x4C.
                swept(
                    hit.position,
                    hit.previous_position,
                    HITBOX_LOOKAHEAD,
                    hit.descriptor.radius,
                ),
                Capsule {
                    start: body,
                    end: body,
                    radius,
                },
            )
            .is_some()
    })
}

/// ftCo_800BB768 (0x800BB768): a move whose threat the hitboxes do not
/// show yet: Bowser's and Yoshi's Bomb over the CPU with no floor between
/// them, Bowser's up smash just below it, Samus's Screw Attack below and
/// near.
fn move_threatens(fp: &Fighter, other: &Fighter, scene: &mut Scene) -> bool {
    let motion = other.core.motion_state.action.0;
    let profile = other.character.table().cpu_profile;
    // The falling threats precede the rising threats in retail's branch order.
    let bomb = profile.plunge_threat_motions.contains(&motion);
    if !bomb {
        if let Some((row, reach)) = profile.column_threat {
            if motion == row {
                let theirs = other.core.physics.position;
                let mine = fp.core.physics.position;
                return over_column(fp, other) && theirs.y < mine.y && mine.y - theirs.y < reach;
            }
        }
        if let Some((row, reach)) = profile.rising_threat {
            if motion == row {
                let theirs = other.core.physics.position;
                let mine = fp.core.physics.position;
                return theirs.y < mine.y && f64::from(distance(mine, theirs)) < reach;
            }
        }
    }
    if !bomb {
        return false;
    }
    let theirs = other.core.physics.position;
    let mine = fp.core.physics.position;
    // 800BB7E8: 5.0 + y in double, rounded; ftCo_800A0FB0.
    let above = (5.0 + f64::from(mine.y)) as f32;
    if scene
        .check_usable_floor(theirs.x, theirs.y, mine.x, above)
        .is_some()
    {
        return false;
    }
    over_column(fp, other)
}

/// The CPU stands within both fighters' x564 of `other`'s x
/// (800BB830..800BB850: fadds, fadds; fsubs, fsubs).
fn over_column(fp: &Fighter, other: &Fighter) -> bool {
    let x = other.core.physics.position.x;
    let theirs = other.core.cpu.hurtbox_extents[2];
    let mine = fp.core.cpu.hurtbox_extents[2];
    let own_x = fp.core.physics.position.x;
    mine + (x + theirs) > own_x && x - theirs - mine < own_x
}

/// ftCo_800BB220 (0x800BB220): an item's placed hitbox meets the CPU's
/// body over the next few ticks (a level-scaled random count; the CPU's
/// own motion sweeps the body too). Not its own or an ally's item, not
/// the Starman (0x0A) or kind 0x9E, nor a kind ftCo_800A5980 takes for a
/// fighter motion. Ness at level 4 and up answers absorbable hitboxes
/// further than 20 ticks ahead with 3.
fn item_hitbox_ahead(
    fp: &Fighter,
    item: &ItemView,
    body: Vec3,
    radius: f32,
    rng: &mut HsdRng,
) -> u8 {
    // ip->owner == fp->gobj, or a fighter owner allied with the CPU (one
    // player in a free-for-all).
    if item.owner == Some(fp.core.player.id) {
        return threat::NONE;
    }
    let kind = i32::from(item.kind);
    if item.kind == ItemKind::Star || kind == 0x9E || crate::select::read_as_motion(kind) {
        return threat::NONE;
    }
    // 800BB2E4..800BB348: fmuls, fctiwz.
    let level = fp.core.cpu.level;
    let ticks = if level < 3 {
        fctiwz(20.0 * rng.randf()) + 10
    } else if level < 6 {
        fctiwz(10.0 * rng.randf()) + 5
    } else if level < 9 {
        fctiwz(5.0 * rng.randf()) + 3
    } else {
        1
    };
    // 800BB378: the count as a single (the magic-double subtract).
    let frames = ticks as f32;
    let delta = fp.core.physics.position_delta;
    // 800BB380/800BB3A0/800BB3B8: fmadds.
    let ahead = Vec3::new(
        fmadds(delta.x, frames, body.x),
        fmadds(delta.y, frames, body.y),
        fmadds(delta.z, frames, body.z),
    );
    let ness = fp.character.table().cpu_profile.absorb_item_threats && level > 3;
    for hit in item.hitboxes.iter().flatten() {
        if !hit.placed || hit.element == HitElement::Inert || hit.struck_thinker {
            continue;
        }
        // 800BB460/800BB49C/800BB4B0: fmadds d * count + x4C.
        let meets = hitbox_pair_contact(
            swept(hit.position, hit.previous_position, frames, hit.radius),
            Capsule {
                start: body,
                end: ahead,
                radius,
            },
        )
        .is_some();
        if !meets {
            continue;
        }
        if ness && ticks >= 21 && hit.absorbable {
            return threat::ABSORBABLE;
        }
        return threat::ITEM;
    }
    threat::NONE
}

/// The blaster's stick: down and back (-0x50).
const BLASTER_STICK_Y: u8 = (-0x50_i8) as u8;
/// ftCo_800A2998's scale when a rolling barrel or flipper comes.
const ISLAND_END_SCALE: f32 = 1.0;

/// ftCo_800BA9A0 (0x800BA9A0): behaviour 7's script.
pub fn defend(fp: &mut Fighter, scene: &Scene, rng: &mut HsdRng) {
    let motion = fp.core.motion_state.action.0;
    let dodging = [S::EscapeF, S::EscapeB, S::EscapeAir, S::EscapeN]
        .iter()
        .any(|&m| motion == m as u16);
    if dodging {
        // ftCo_800A0C8C: Done.
        script::command(&mut fp.core.cpu, C::Done);
        return;
    }
    if motion == S::GuardOn as u16 {
        roll::shield(fp, rng);
        return;
    }
    let found = fp.core.cpu.xf8_b12;
    if found == threat::NONE {
        let cpu = &mut fp.core.cpu;
        cpu.behavior = cpu.home_behavior;
        script::neutral_stick(cpu);
        script::command(cpu, C::ReleaseR);
        script::command(cpu, C::ReleaseB);
        script::command(cpu, C::Done);
        return;
    }
    let blaster = fp.character.table().cpu_profile.blaster_defence;
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        if blaster {
            if found == threat::ITEM {
                fire_blaster(fp, rng);
            } else {
                retap_r(fp);
            }
        } else if found == threat::ABSORBABLE {
            roll::down_special(fp);
        } else {
            retap_r(fp);
        }
        return;
    }
    if found == threat::ITEM {
        let item = fp
            .core
            .cpu
            .threat_item
            .and_then(|id| scene.items.iter().find(|item| item.id == id))
            .copied()
            .expect("ftCo_800BA9A0: xF4 names a live item");
        let ahead = f64::from(fp.core.physics.position.x - item.position.x) > 0.0;
        match item.kind {
            ItemKind::Flipper | ItemKind::TaruCann => {
                if roll::near_island_end(fp, scene, ISLAND_END_SCALE) {
                    roll::shield(fp, rng);
                } else if ahead {
                    roll::roll_right(fp);
                } else {
                    roll::roll_left(fp);
                }
            }
            ItemKind::DKinoko => {
                if ahead {
                    roll::roll_left(fp);
                } else {
                    roll::roll_right(fp);
                }
            }
            _ => {
                // 800BAD6C: Randf() > 0.5, a double compare.
                if blaster && f64::from(rng.randf()) > 0.5 {
                    fire_blaster(fp, rng);
                } else {
                    roll::shield(fp, rng);
                }
            }
        }
        return;
    }
    if found == threat::ABSORBABLE {
        roll::down_special(fp);
        return;
    }
    let Some(other) = fp.core.cpu.threat_fighter else {
        roll::shield(fp, rng);
        return;
    };
    // 800BB084..800BB0E4: fsubs, |d| > 30 (double): roll away, else
    // through.
    let d = fp.core.physics.position.x - scene.fighter(other).core.physics.position.x;
    let far = f64::from(gekko_math::msl::fabsf(d)) > ROLL_AWAY_DISTANCE;
    let right = f64::from(d) > 0.0;
    if far == right {
        roll::roll_right(fp);
    } else {
        roll::roll_left(fp);
    }
}

/// ftCo_CpuFireBlaster: the down-back B, then a level-scaled wait.
fn fire_blaster(fp: &mut Fighter, rng: &mut HsdRng) {
    let cpu = &mut fp.core.cpu;
    let scale = 9 - cpu.level;
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::SetLstickY, BLASTER_STICK_Y);
    script::command(cpu, C::PressB);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickY, 0);
    if scale != 0 {
        // 800BABC8: fmuls, fctiwz.
        let ticks = scale * (fctiwz(4.0 * rng.randf()) + 4);
        script::command1(cpu, C::WaitFor, ticks as u8);
    }
    script::command(cpu, C::Done);
}

/// ftCo_CpuRetapR: release R, a tick, press it, a tick, release it.
fn retap_r(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::ReleaseR);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::PressR);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseR);
    script::command(cpu, C::Done);
}
