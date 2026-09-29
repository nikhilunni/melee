//! The Ice Climbers' partner (Nana): CPU mode 6 records her player's
//! fighter's inputs into a ring and, once in step with it, replays them a
//! few ticks late (ftCo_800B101C, ftCo_800B0918, ftCo_800B0AF4).
use crate::{route, select, world::Scene};
use gekko_math::{
    fma::{fmadd, fmadds},
    msl::fctiwz,
    HsdRng,
};
use melee_ft::fighter::{cpu::FOLLOW_ENTRIES, state::partner_sync, Fighter};
use melee_types::{CommonMotionState as S, GroundOrAir, ItemKind};

/// ftPp_MS_SpecialHi_0..5 (361..366): the partner's own Belay rows.
const BELAY: std::ops::RangeInclusive<u16> = 361..=366;
/// ftPp_MS_SpecialLw and ftPp_MS_SpecialAirLw (357, 358).
const BLIZZARD: u16 = 357;
const BLIZZARD_AIR: u16 = 358;
/// Beyond 25 units (squared 625) the partner is out of step.
const STEP_RANGE_SQUARED: f64 = 625.0;

/// inlineM0 (ftCo_0A01.c:7294): a stick axis to CPU units, 127 above zero
/// and 128 below (fmuls, fctiwz, stb).
fn stick_units(axis: f32) -> i8 {
    let scaled = if axis >= 0.0 {
        127.0 * axis
    } else {
        128.0 * axis
    };
    fctiwz(scaled) as i8
}

/// ftCo_800B0918 (0x800B0918): advance both ring cursors (wrapping) and
/// record the player's fighter's current inputs, position and facing.
fn record(partner: &mut Fighter, own: &Fighter) {
    let ring = &mut partner.core.cpu.follow;
    ring.write = (ring.write + 1) % FOLLOW_ENTRIES;
    ring.read = (ring.read + 1) % FOLLOW_ENTRIES;
    let input = &own.core.input.current;
    // fctiwz of the 0..1 trigger: both bytes take 0 or 1.
    let trigger = fctiwz(input.trigger) as u8;
    ring.entries[ring.write] = melee_ft::fighter::cpu::FollowSample {
        buttons: input.held.0,
        triggers: [trigger, trigger],
        stick: [stick_units(input.stick.x), stick_units(input.stick.y)],
        cstick: [stick_units(input.cstick.x), stick_units(input.cstick.y)],
        position: own.core.physics.position,
        facing: own.core.physics.facing,
    };
}

/// ftCo_800B0CA8 (0x800B0CA8): whether the partner counts as in step with
/// the player's fighter: neither is in a state that forbids it, and the
/// partner's motion carries x2225_b3 or is a squat or landing.
fn in_step(partner: &Fighter, own: &Fighter) -> bool {
    let own_motion = own.core.motion_state.action.0;
    let is = |state: S| own_motion == state as u16;
    let between = |a: S, b: S| (a as u16..=b as u16).contains(&own_motion);
    if is(S::CliffCatch)
        || is(S::CliffWait)
        || is(S::CaptureWaitHi)
        || is(S::CaptureWaitLw)
        || between(S::ShoulderedWait, S::ShoulderedTurn)
        || is(S::Ottotto)
        || is(S::OttottoWait)
        || between(S::CaptureKirby, S::CaptureWaitKirby)
        || between(S::Rebirth, S::RebirthWait)
        || between(S::WarpStarJump, S::WarpStarFall)
        || between(S::ItemParasolFall, S::ItemParasolDamageFall)
    {
        return false;
    }
    if let Some(item) = &partner.core.held_item {
        if matches!(
            item.kind,
            ItemKind::Box | ItemKind::Taru | ItemKind::Kusudama | ItemKind::TaruCann
        ) {
            return false;
        }
    }
    let partner_position = partner.core.physics.position;
    let own_position = own.core.physics.position;
    if own.core.physics.ground_or_air == GroundOrAir::Ground
        && partner.core.physics.ground_or_air == GroundOrAir::Air
        && own_position.y > partner_position.y
        && partner.core.physics.position_delta.y < 0.0
    {
        return false;
    }
    let motion = partner.core.motion_state.action;
    if partner_sync(partner, motion) {
        return true;
    }
    [
        S::Squat,
        S::SquatWait,
        S::Landing,
        S::LandingFallSpecial,
        S::LandingAirN,
        S::LandingAirF,
        S::LandingAirB,
        S::LandingAirHi,
        S::LandingAirLw,
    ]
    .iter()
    .any(|&state| motion.0 == state as u16)
}

/// The partner's squared distance from the player's fighter, x from the
/// partner and y from the player's fighter (800B0FD4..800B0FEC: fmadds).
fn separation_squared(partner: &Fighter, own: &Fighter) -> f32 {
    let dx = partner.core.physics.position.x - own.core.physics.position.x;
    let dy = own.core.physics.position.y - partner.core.physics.position.y;
    fmadds(dx, dx, dy * dy)
}

/// ftCo_800B0E98 (0x800B0E98): ready to start replaying: in step, both
/// grounded, not in Belay, acting (or waiting), moving alike and within 25.
fn ready_to_follow(partner: &mut Fighter, own: &Fighter) -> bool {
    if !in_step(partner, own) {
        return false;
    }
    if own.core.physics.ground_or_air == GroundOrAir::Air
        || partner.core.physics.ground_or_air == GroundOrAir::Air
    {
        return false;
    }
    if partner.core.cpu.xfb_b0 || BELAY.contains(&partner.core.motion_state.action.0) {
        return false;
    }
    if !crate::should_act(&mut partner.core.cpu)
        && partner.core.motion_state.action.0 != S::Wait as u16
    {
        return false;
    }
    let a = partner.core.physics.position_delta;
    let b = own.core.physics.position_delta;
    let x = a.x - b.x;
    let y = a.y - b.y;
    let r = partner.core.attributes.walking.mid_walk_point;
    // 800B0FAC..800B0FB8: fmuls, fmuls, fmadds.
    if fmadds(x, x, y * y) > r * r {
        return false;
    }
    f64::from(separation_squared(partner, own)) < STEP_RANGE_SQUARED
}

/// ftCo_800B0760 (0x800B0760): out of step, the partner plays as a CPU:
/// pick a target and items, head back toward the player's fighter, and
/// choose a behaviour.
fn seek(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    let cpu = &mut fp.core.cpu;
    cpu.xf8_b0 = true;
    cpu.xf9_b2 = true;
    cpu.xf9_b4 = true;
    cpu.xf9_b3 = false;
    cpu.xf9_b5 = false;
    cpu.xf9_b6 = false;
    cpu.xf9_b7 = false;
    cpu.xf9_b1 = false;
    fp.core.cpu.target = crate::targets::nearest_opponent(fp, scene, rng);
    crate::targets::update_common_item(fp, scene);
    crate::targets::update_special_item(fp, scene);
    if crate::should_act(&mut fp.core.cpu) {
        if fp.core.cpu.xfb_b0 {
            crate::recover::belay_recovery(fp, scene);
        } else {
            route::toward_partner(fp, scene);
        }
    }
    select::choose_behavior(fp, scene, rng);
}

/// ftCo_800B101C (0x800B101C): CPU mode 6's decision each tick.
pub fn decide(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    fp.core.cpu.xf9_b2 = true;
    fp.core.cpu.xf9_b4 = true;
    let Some((_, own)) = scene.partner_of(fp) else {
        unimplemented!("ftCo_800AEFB8: a partner whose player's fighter is gone");
    };
    record(fp, own);
    // 800B10E0: fdivs, fctiwz, at most 9.
    let level = fctiwz(own.core.physics.percent / 20.0);
    fp.core.cpu.level = level.min(9);
    if fp.core.cpu.following {
        route::toward_partner(fp, scene);
        let own = scene.partner_of(fp).expect("the player's fighter").1;
        let fall_out = if BELAY.contains(&fp.core.motion_state.action.0) {
            fp.core.cpu.xfb_b0 = true;
            true
        } else if f64::from(separation_squared(fp, own)) > STEP_RANGE_SQUARED {
            true
        } else {
            !in_step(fp, own)
        };
        if fall_out {
            let cpu = &mut fp.core.cpu;
            cpu.following = false;
            cpu.behavior = cpu.home_behavior;
            cpu.stick = [0; 2];
            cpu.buttons = 0;
            cpu.triggers = [0; 2];
        }
    } else {
        seek(fp, scene, rng);
        let own = scene.partner_of(fp).expect("the player's fighter").1;
        if ready_to_follow(fp, own) {
            let cpu = &mut fp.core.cpu;
            cpu.behavior = select::behavior::STAND_BY;
            cpu.following = true;
            cpu.xf9_b2 = false;
            cpu.xf9_b4 = false;
        }
    }
    if fp.core.physics.ground_or_air == GroundOrAir::Ground {
        fp.core.cpu.xfb_b0 = false;
    }
}

/// ftCo_800B0AF4 (0x800B0AF4): while following, take the delayed sample's
/// pad; in a synced motion also drift 5% toward its position and take its
/// facing. The player's fighter in the Blizzard makes the partner join it.
pub fn replay(fp: &mut Fighter, scene: &Scene) {
    let Some((_, own)) = scene.partner_of(fp) else {
        return;
    };
    if !fp.core.cpu.following {
        return;
    }
    let motion = fp.core.motion_state.action.0;
    let joins_blizzard = !(motion == S::FireFlowerShoot as u16
        || motion == S::FireFlowerShootAir as u16)
        && own.core.motion_state.action.0 == BLIZZARD
        && usize::from(motion) < melee_ft::fighter::COMMON_COUNT;
    if joins_blizzard {
        unimplemented!("ftCo_800B0AF4: ftPp_SpecialLw_Enter for the partner");
    }
    let cpu = &mut fp.core.cpu;
    let sample = cpu.follow.entries[cpu.follow.read];
    cpu.stick = sample.stick;
    cpu.cstick = sample.cstick;
    cpu.buttons = sample.buttons;
    cpu.triggers = sample.triggers;
    if partner_sync(fp, fp.core.motion_state.action) {
        let position = &mut fp.core.physics.position;
        // 800B0C48..800B0C54: fmul, fmadd, frsp.
        position.x = fmadd(
            0.95,
            f64::from(position.x),
            0.05 * f64::from(sample.position.x),
        ) as f32;
        position.y = fmadd(
            0.95,
            f64::from(position.y),
            0.05 * f64::from(sample.position.y),
        ) as f32;
        if motion != BLIZZARD && motion != BLIZZARD_AIR {
            fp.core.physics.facing = sample.facing;
        }
    }
}
