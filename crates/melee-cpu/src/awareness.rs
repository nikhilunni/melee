//! ftCo_800B33B0: the CPU's per-tick upkeep before it decides anything.
use crate::world::Scene;
use gekko_math::{
    fma::{fmadd, fmadds},
    msl::{fctiwz, sqrtf},
    HsdRng,
};
use melee_ft::fighter::Fighter;
use melee_types::GrKind;

/// Ticks between the upkeep's recovery-style rolls (x7C % 300).
const RECOVERY_STYLE_PERIOD: i32 = 300;
/// Ticks between recovery-scale rolls (x7C % 30).
const RECOVERY_SCALE_PERIOD: i32 = 30;
/// Ticks between reach rolls (ftCo_800A0CB0, x7C % 600).
const REACH_PERIOD: i32 = 600;

/// ftCo_800B33B0 (0x800B33B0).
pub fn upkeep(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    roll_recovery_style(fp, rng);
    note_floor_over_stage(fp, scene);
    count_still_ticks(fp);
    hold_target_lock(fp, rng);
    advance_route(fp);
    ride_destination_floor(fp, scene);
    roll_recovery_scale(fp, rng);
    roll_reach(fp, rng);
    note_ceiling(fp, scene);
}

/// x7C % 300: xFA_b34 = Randf() < 0.04 * level + 0.3 (800B3428: fmadds).
fn roll_recovery_style(fp: &mut Fighter, rng: &mut HsdRng) {
    let cpu = &mut fp.core.cpu;
    if cpu.reaction_timer % RECOVERY_STYLE_PERIOD == 0 {
        let threshold = fmadds(0.04, cpu.level as f32, 0.3);
        cpu.xfa_b34 = u8::from(rng.randf() < threshold);
    }
}

/// xFA_b5: the floor from 10 above to 1000 below the fighter exists and
/// lies inside the blast zones.
fn note_floor_over_stage(fp: &mut Fighter, scene: &mut Scene) {
    let position = fp.core.physics.position;
    // 800B346C..800B3488: double arithmetic, each end rounded (frsp).
    let below = (f64::from(position.y) - 1000.0) as f32;
    let above = (10.0 + f64::from(position.y)) as f32;
    let floor = scene.check_usable_floor(position.x, above, position.x, below);
    fp.core.cpu.over_stage =
        floor.is_some_and(|hit| !scene.outside(hit.pos.x, hit.pos.y, fp.core.cpu.half_size));
}

/// x84: consecutive ticks the collision position moved less than 0.01
/// (800B35AC: fmadds, then an inlined sqrtf).
fn count_still_ticks(fp: &mut Fighter) {
    let data = &fp.core.collision.data;
    let dy = data.cur_pos.y - data.last_pos.y;
    let dx = data.cur_pos.x - data.last_pos.x;
    let moved = sqrtf(fmadds(dx, dx, dy * dy));
    let cpu = &mut fp.core.cpu;
    if f64::from(moved) < 0.01 {
        cpu.still_ticks += 1;
    } else {
        cpu.still_ticks = 0;
    }
}

/// x30 counts down a held target lock (xF9_b0); without one it rerolls
/// every tick: 120 * (0.5 * (0.5 * Randf())), three fmuls then fctiwz.
fn hold_target_lock(fp: &mut Fighter, rng: &mut HsdRng) {
    let cpu = &mut fp.core.cpu;
    if cpu.target_locked {
        if cpu.target_lock_timer != 0 {
            cpu.target_lock_timer -= 1;
        } else {
            cpu.target_locked = false;
        }
    } else {
        cpu.target_lock_timer = lock_duration(rng);
    }
}

/// 120 * (0.5 * (0.5 * Randf())), three fmuls, fctiwz (800B367C..800B3688;
/// ftCo_800A4BEC repeats it).
pub fn lock_duration(rng: &mut HsdRng) -> i32 {
    fctiwz(120.0 * (0.5 * (0.5 * rng.randf())))
}

/// x60: when the stage route's timer runs out, x64 becomes the destination.
fn advance_route(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    if cpu.route_timer != 0 {
        cpu.route_timer -= 1;
        if cpu.route_timer == 0 {
            cpu.destination = cpu.route_destination;
        }
    }
}

/// The destination rides the floor under it (mpGetSpeed), or the stage's
/// own drift (grLib_801C9E60).
fn ride_destination_floor(fp: &mut Fighter, scene: &mut Scene) {
    let destination = fp.core.cpu.destination;
    // 800B36D0..800B36F8: y -/+ 2.0 in double, rounded.
    let below = (f64::from(destination.y) - 2.0) as f32;
    let above = (2.0 + f64::from(destination.y)) as f32;
    let floor = scene.check_usable_floor(destination.x, above, destination.x, below);
    let speed = match floor {
        Some(hit) => scene
            .map
            .line_speed(hit.line_id, &hit.pos)
            .unwrap_or_default(),
        None => stage_drift(scene.stage()),
    };
    let cpu = &mut fp.core.cpu;
    // 800B379C/800B37AC: fadds.
    cpu.destination.x += speed.x;
    cpu.destination.y += speed.y;
}

/// grLib_801C9E60 (0x801C9E60): Rainbow Cruise, Big Blue and Icicle
/// Mountain move the whole stage; everywhere else nothing moves.
fn stage_drift(stage: GrKind) -> hsd_types::Vec3 {
    match stage {
        GrKind::RCruise | GrKind::BigBlue | GrKind::Icemt => {
            unimplemented!("grLib_801C9E60: stage drift of {stage:?}")
        }
        _ => hsd_types::Vec3::ZERO,
    }
}

/// x7C % 30: x570 = 0.05 * (level + 1) + 0.05 * Randf(), in double
/// (800B3834: fmul, 800B3848: fmadd, frsp).
fn roll_recovery_scale(fp: &mut Fighter, rng: &mut HsdRng) {
    let cpu = &mut fp.core.cpu;
    if cpu.reaction_timer % RECOVERY_SCALE_PERIOD == 0 {
        let random = 0.05 * f64::from(rng.randf());
        cpu.x570 = fmadd(0.05, f64::from(cpu.level + 1), random) as f32;
    }
}

/// ftCo_800A0CB0 (0x800A0CB0): x7C % 600 rerolls x56C, the extra reach the
/// CPU allows, as scale * (1 - r^3) (fmuls, fmuls, fsubs; double fmul).
fn roll_reach(fp: &mut Fighter, rng: &mut HsdRng) {
    let partner = fp.core.capabilities.cpu_partner;
    let cpu = &mut fp.core.cpu;
    if cpu.reaction_timer % REACH_PERIOD == 0 {
        let r = rng.randf();
        let rand = 1.0 - r * (r * r);
        // Donkey Kong and Bowser 9.0, Giga Bowser 18.0: unported kinds.
        cpu.x56c = if partner {
            rand
        } else {
            (4.0 * f64::from(rand)) as f32
        };
    }
}

/// xFA_b6: set on Corneria's ceilinged floor (grCorneria_801E2E50), kept
/// while a ceiling stays overhead.
fn note_ceiling(fp: &mut Fighter, scene: &mut Scene) {
    if fp.core.cpu.xfa_b6 {
        let position = fp.core.physics.position;
        if scene
            .map
            .check_ceiling(
                position.x,
                position.y,
                position.x,
                1000.0 + position.y,
                -1,
                -1,
            )
            .is_none()
        {
            fp.core.cpu.xfa_b6 = false;
        }
    } else if fp.core.physics.ground_or_air == melee_types::GroundOrAir::Ground
        && scene.stage() == GrKind::Corneria
    {
        unimplemented!("grCorneria_801E2E50: Corneria's ceilinged floors");
    }
}
