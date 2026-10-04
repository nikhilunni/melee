//! A grounded CPU whose destination is on another island (ftCo_800AB224's
//! tail, ftCo_0A01.c:5240-5293): by the destination's angle it jumps up,
//! walks toward it, or comes down off its platform.
use crate::{
    behave::{leave_revival_platform, stick_limits, turn_around},
    script::{self, Command as C},
    world::Scene,
};
use melee_ft::fighter::Fighter;
use melee_lb::trigf::stick_angle;
use melee_types::{mp::collide, mp::line_flag, CommonMotionState as S, GrKind, GroundOrAir};

/// Steeper than this (35 degrees) the destination is above: a jump.
const ABOVE: f64 = 0.610_865_233_466_029_2;
/// Steeper still (75 degrees), a CPU facing away walks on instead of
/// turning.
const OVERHEAD: f64 = 1.308_996_928_855_776_8;
/// Below this (-45 degrees) the destination is underneath.
const BELOW: f64 = -0.785_398_157_313_466_1;

/// ftCo_800AB224 (0x800AB600..0x800ABA10) once ftCo_800A21FC has failed.
pub(crate) fn toward_other_island(fp: &mut Fighter, scene: &mut Scene) {
    let position = fp.core.physics.position;
    let destination = fp.core.cpu.destination;
    // 800AB60C..800AB62C: fsubs, negated below zero; fsubs.
    let mut across = destination.x - position.x;
    if across < 0.0 {
        across = -across;
    }
    let up = destination.y - position.y;
    let angle = if across < 0.00001 && across > -0.00001 {
        if f64::from(up) > 0.0 {
            core::f32::consts::FRAC_PI_2
        } else {
            -core::f32::consts::FRAC_PI_2
        }
    } else {
        stick_angle(up, across)
    };
    // 800AB678, 800AB7xx, 800AB928: the angle against double constants.
    let angle = f64::from(angle);
    if angle > ABOVE {
        let ecb = fp.core.collision.data.cur_pos;
        let ceiling = scene
            .map
            .check_ceiling(ecb.x, ecb.y, ecb.x, destination.y, -1, -1)
            .is_some();
        if ceiling {
            if near_island_end(fp, scene, 1.0) {
                turn_around(fp);
            } else if temple_waypoint(scene) {
                jump(fp, scene);
            } else {
                walk_forward(fp);
            }
        } else if scene.stage() == GrKind::Icemt {
            jump(fp, scene);
        } else if !facing_destination(fp) {
            if angle < OVERHEAD {
                turn_around(fp);
            } else {
                walk_forward(fp);
            }
        } else {
            jump(fp, scene);
        }
    } else if angle > BELOW {
        if facing_destination(fp) {
            approach(fp, scene);
        } else {
            turn_around(fp);
        }
    } else {
        descend(fp, scene);
    }
}

/// ftCo_800A2BD4 (0x800A2BD4): the destination is not behind the CPU
/// (fmuls, a double compare with >=).
fn facing_destination(fp: &Fighter) -> bool {
    let dx = fp.core.cpu.destination.x - fp.core.physics.position.x;
    f64::from(fp.core.physics.facing * dx) >= 0.0
}

/// ftCo_800A28D0 (0x800A28D0): the island's end ahead of the CPU is within
/// `5.0 * scale` (fmul in double, 800A2968). The ends are the island's
/// stored ones (mp_UnkStruct0 x8 / x14), not refreshed.
fn near_island_end(fp: &Fighter, scene: &Scene, scale: f32) -> bool {
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return false;
    }
    let Some(node) = scene.map.island_of_line(fp.core.collision.data.floor.index) else {
        return false;
    };
    let island = scene.map.island(node);
    let end = if f64::from(fp.core.physics.facing) > 0.0 {
        island.right.x
    } else {
        island.left.x
    };
    let mut distance = end - fp.core.physics.position.x;
    if distance < 0.0 {
        distance = -distance;
    }
    f64::from(distance) < 5.0 * f64::from(scale)
}

/// ftCo_800AAF48 (0x800AAF48): a waypoint up the Temple's ledges; no other
/// stage has one.
fn temple_waypoint(scene: &Scene) -> bool {
    if scene.stage() == GrKind::Shrine {
        unimplemented!("ftCo_800AAF48: the Temple's island waypoints");
    }
    false
}

/// Collide_WallMask on the CPU's last map pass (inlineC0).
fn against_wall(fp: &Fighter) -> bool {
    fp.core.collision.data.env_flags as u32 & collide::WALL_MASK != 0
}

/// isInTeeter.
fn teetering(fp: &Fighter) -> bool {
    let motion = fp.core.motion_state.action.0;
    motion == S::Ottotto as u16 || motion == S::OttottoWait as u16
}

/// ftCo_CpuTurnAwayFromLedge: neutral, a tick, the stick forward, a tick,
/// neutral.
fn turn_away_from_ledge(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, 0);
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::LstickXForward, 0x7F);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickX, 0);
    script::command(cpu, C::Done);
}

/// ftCo_800AABC8 (0x800AABC8) and the shared tail of its siblings: the
/// stick eased forward at the level's step and limit (ftCo_800AA320).
fn walk_forward(fp: &mut Fighter) {
    if teetering(fp) {
        turn_away_from_ledge(fp);
        return;
    }
    ease_forward(fp);
}

fn ease_forward(fp: &mut Fighter) {
    let (step, limit) = stick_limits(fp);
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, 0);
    script::command2(cpu, C::LstickForwardClamped, step as u8, limit as u8);
    script::command(cpu, C::Done);
}

/// ftCo_800A0148 (0x800A0148): a jump with Y held ten frames, the stick at
/// the destination when it is more than 30 away across, neutral otherwise;
/// nothing when the jump would pass the top blast zone (800A0170: fadds).
/// ftCo_800A1CA8's arm is the hammer (x2168), which only its item sets.
fn jump(fp: &mut Fighter, scene: &Scene) {
    let position = fp.core.physics.position;
    let cpu = &mut fp.core.cpu;
    if position.y + cpu.jump_height > scene.arena.top {
        script::finish_with_neutral_stick(cpu);
        return;
    }
    let mut across = cpu.destination.x - position.x;
    if across < 0.0 {
        across = -across;
    }
    if f64::from(across) > 30.0 {
        script::command(cpu, C::ReleaseY);
        script::command1(cpu, C::WaitFor, 1);
        script::command1(cpu, C::LstickTowardDestination, 0x7F);
    } else {
        script::command(cpu, C::ReleaseY);
        script::neutral_stick(cpu);
        script::command1(cpu, C::WaitFor, 1);
    }
    script::command(cpu, C::PressY);
    script::command1(cpu, C::WaitFor, 10);
    script::command(cpu, C::ReleaseY);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::Done);
}

/// ftCo_800AA844 (0x800AA844): a destination about level with the CPU on
/// another island. At the island's end it jumps when the destination is
/// higher, more than 40 away across, or a wall is in the way; otherwise,
/// and short of the end without a wall, it walks on.
fn approach(fp: &mut Fighter, scene: &Scene) {
    if teetering(fp) {
        turn_away_from_ledge(fp);
        return;
    }
    let position = fp.core.physics.position;
    let destination = fp.core.cpu.destination;
    if near_island_end(fp, scene, 1.0) {
        // ftCo_800A1CA8 (the hammer) guards the platform-drop arm.
        let mut across = destination.x - position.x;
        if across < 0.0 {
            across = -across;
        }
        if destination.y > position.y || f64::from(across) > 40.0 || against_wall(fp) {
            jump(fp, scene);
        } else {
            ease_forward(fp);
        }
    } else if against_wall(fp) {
        jump(fp, scene);
    } else {
        ease_forward(fp);
    }
}

/// ftCo_800AACD0 (0x800AACD0): a destination underneath. From a platform
/// the CPU drops through (ftCo_800A08F0); from solid ground it walks off
/// the island toward it, turning first when the destination lies beyond
/// the island's ends behind it, and jumps a wall.
fn descend(fp: &mut Fighter, scene: &Scene) {
    if teetering(fp) {
        turn_away_from_ledge(fp);
        return;
    }
    let floor = fp.core.collision.data.floor.index;
    if floor != -1 && scene.map.line_get_flags(floor) & line_flag::PLATFORM != 0 {
        leave_revival_platform(fp);
        return;
    }
    if let Some(node) = scene.map.island_of_line(floor) {
        let (left, right) = scene.map.island_ends(node);
        let x = fp.core.cpu.destination.x;
        if (x < left.x || x > right.x) && !facing_destination(fp) {
            turn_around(fp);
            return;
        }
    }
    if scene.stage() != GrKind::Icemt && against_wall(fp) {
        jump(fp, scene);
        return;
    }
    ease_forward(fp);
}
