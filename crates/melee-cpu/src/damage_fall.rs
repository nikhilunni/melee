//! Behaviour 15 (ftCo_800ABBA8): tumbling (DamageFall) at level 5 and up.
//! Head for the island end farthest from the target, tech when the floor is
//! under ten frames away (level 8 and up), tech-jump over an opponent below
//! who is swinging upward, and otherwise drift toward the destination when
//! the fall would land short of it.
use crate::{
    movement::finish_with_neutral_y,
    script::{self, Command as C},
    world::Scene,
};
use gekko_math::{
    fma::{fmadds, fnmsub},
    msl::{fabsf, fctiwz, sqrtf},
    HsdRng,
};
use hsd_types::Vec3;
use melee_ft::fighter::Fighter;
use melee_types::CommonMotionState as S;

/// ftCo_IsNearlyZero.
fn nearly_zero(x: f32) -> bool {
    x < 0.00001 && x > -0.00001
}

/// Level 5 and up tumbles this way (the selection's own test).
const MINIMUM_LEVEL: i32 = 5;
/// Above this level the CPU techs the floor it is about to hit.
const TECH_LEVEL: i32 = 7;
/// Ticks to the floor below which the CPU presses R.
const TECH_TICKS: i32 = 10;
/// The ray down from the CPU for the floor it will hit.
const FLOOR_PROBE_DEPTH: f64 = 1000.0;
/// ftCo_800A1F3C's arrival radius at the chosen island end.
const ARRIVAL_RADIUS: f32 = 5.0;
/// ftCo_800A6700: island ends are taken 5 inside, probed 5 above and below.
const END_INSET: f64 = 5.0;
/// The horizontal slack added to both fighters' half widths (a double).
const BELOW_SLACK: f64 = 5.0;
/// With no speed or gravity, the time terms are this large.
const NO_TIME: f32 = 1000.0;
/// No speed when the gravity is zero: the floor is "far".
const FAR_TICKS: i32 = 100;
/// The wait before the tech-jump's stick flick.
const TECH_JUMP_WAIT: u8 = 0x32;
/// SetLstickY's byte for straight down (-127).
const STICK_DOWN: u8 = 0x81;

/// ftCo_800ABBA8 (0x800ABBA8).
pub fn steer(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    if fp.core.cpu.level < MINIMUM_LEVEL || fp.core.motion_state.action.0 != S::DamageFall as u16 {
        let cpu = &mut fp.core.cpu;
        cpu.behavior = cpu.home_behavior;
        script::command1(cpu, C::SetLstickX, 0);
        script::command(cpu, C::ReleaseR);
        script::command(cpu, C::Done);
        return;
    }
    if let Some(target) = fp.core.cpu.target {
        let target = scene.fighter(target).core.physics.position;
        if let Some(end) = farthest_island_end(fp, scene, target) {
            crate::route::set_destination(fp, scene, end.x, end.y, ARRIVAL_RADIUS);
        }
    }
    if fp.core.cpu.level > TECH_LEVEL && ticks_to_floor(fp, scene).is_some_and(|t| t < TECH_TICKS) {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickY, 0);
        script::command1(cpu, C::LstickXTowardDestination, 0x7F);
        script::command(cpu, C::PressR);
        script::command(cpu, C::Done);
        return;
    }
    let Some(below) = crate::targets::nearest_opponent(fp, scene, rng) else {
        script::return_to_previous(&mut fp.core.cpu);
        return;
    };
    if swinging_up_below(fp, scene.fighter(below)) {
        tech_jump(fp);
        return;
    }
    let short = lands_short(fp);
    let cpu = &mut fp.core.cpu;
    if short {
        script::command1(cpu, C::LstickXTowardDestination, 0x7F);
    } else {
        script::command1(cpu, C::SetLstickX, 0);
    }
    finish_with_neutral_y(fp);
}

/// ftCo_800A6700 (0x800A6700): of the safe floor islands' ends (5 inside,
/// with a usable floor from 5 above to 5 below, inside the blast zones),
/// the one farthest from `from`. Each probe coordinate is a double sum,
/// rounded; the squared distance fuses its x term (800A68AC: fmadds).
fn farthest_island_end(fp: &Fighter, scene: &mut Scene, from: Vec3) -> Option<Vec3> {
    let islands: melee_types::fixed::FixedVec<usize, 64> = {
        let mut list = melee_types::fixed::FixedVec::default();
        for island in scene.map.floor_islands() {
            list.push(island);
        }
        list
    };
    let half_size = fp.core.cpu.half_size;
    let mut best = -1.0f32;
    let mut chosen = None;
    for node in islands.iter().copied() {
        if crate::route::island_unsafe(scene, Some(node)) {
            continue;
        }
        let island = *scene.map.island(node);
        for (end, inset) in [(island.left, END_INSET), (island.right, -END_INSET)] {
            let x = (f64::from(end.x) + inset) as f32;
            let above = (END_INSET + f64::from(end.y)) as f32;
            let below = (f64::from(end.y) - END_INSET) as f32;
            if scene.check_usable_floor(x, above, x, below).is_none() {
                continue;
            }
            if scene.outside(x, end.y, half_size) {
                continue;
            }
            let dx = x - from.x;
            let dy = end.y - from.y;
            let distance = fmadds(dx, dx, dy * dy);
            if distance > best {
                best = distance;
                chosen = Some(Vec3::new(x, end.y, end.z));
            }
        }
    }
    chosen
}

/// The ticks until the ECB's bottom reaches the floor under the CPU, by
/// the fall's quadratic (800ABD5C..800ABF10); `None` without a usable floor.
fn ticks_to_floor(fp: &Fighter, scene: &mut Scene) -> Option<i32> {
    let p = fp.core.physics.position;
    let depth = (f64::from(p.y) - FLOOR_PROBE_DEPTH) as f32;
    let floor = scene.check_usable_floor(p.x, p.y, p.x, depth)?;
    let g = -fp.core.attributes.air.gravity;
    let v = fp.core.physics.position_delta.y;
    let data = &fp.core.collision.data;
    // 800ABD70 fadds, 800ABD80 fsubs.
    let h = floor.pos.y - (data.cur_pos.y + data.ecb.bottom.y);
    if nearly_zero(g) {
        return Some(if nearly_zero(v) {
            FAR_TICKS
        } else {
            fctiwz(h / v)
        });
    }
    // 800ABDB4 fmuls, 800ABDB8 fmadds: 2g * h + v * v, then the inlined
    // sqrtf of its magnitude.
    let discriminant = fmadds(2.0 * g, h, v * v);
    let magnitude = if discriminant < 0.0 {
        -discriminant
    } else {
        discriminant
    };
    let root = if magnitude > 0.0 {
        sqrtf(magnitude)
    } else {
        magnitude
    };
    // 800ABEB4 fneg, fsubs, fdivs.
    Some(fctiwz((-root - v) / g))
}

/// 800ABF80..800AC014: the opponent is below, overlapping in height and
/// within 5 of touching across, in an up tilt or up smash.
fn swinging_up_below(fp: &Fighter, below: &Fighter) -> bool {
    let (mine, theirs) = (fp.core.physics.position, below.core.physics.position);
    if theirs.y >= mine.y {
        return false;
    }
    let (my_extents, their_extents) = (fp.core.cpu.hurtbox_extents, below.core.cpu.hurtbox_extents);
    // 800ABFAC fadds.
    if fabsf(theirs.y - mine.y) >= their_extents[3] + my_extents[3] {
        return false;
    }
    // 800ABFE0 fadds, 800ABFE4 fadd (double).
    let reach = BELOW_SLACK + f64::from(their_extents[2] + my_extents[2]);
    if f64::from(fabsf(theirs.x - mine.x)) >= reach {
        return false;
    }
    let motion = below.core.motion_state.action.0;
    motion == S::AttackHi3 as u16 || motion == S::AttackHi4 as u16
}

/// 800AC018..800AC0C4: tap R (a tech), let go, then flick the stick down
/// after 50 frames.
fn tech_jump(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::ReleaseR);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::PressR);
    script::command1(cpu, C::WaitFor, 1);
    script::neutral_stick(cpu);
    script::command(cpu, C::ReleaseR);
    script::command1(cpu, C::WaitFor, TECH_JUMP_WAIT);
    script::command1(cpu, C::SetLstickY, STICK_DOWN);
    script::command1(cpu, C::WaitFor, 1);
    finish_with_neutral_y(fp);
}

/// 800AC0CC..800AC2B4: where the fall reaches the destination's x (rising
/// to terminal velocity, then at it); true when that is behind the CPU or
/// below the destination.
fn lands_short(fp: &Fighter) -> bool {
    let p = fp.core.physics.position;
    let d = fp.core.physics.position_delta;
    let destination = fp.core.cpu.destination;
    let air = &fp.core.attributes.air;
    // 800AC0E0 fsubs, 800AC110 fdivs.
    let to_destination = if nearly_zero(d.x) {
        NO_TIME
    } else {
        (destination.x - p.x) / d.x
    };
    // 800AC158..800AC164: fneg, fsubs, fneg, fdivs.
    let to_terminal = if nearly_zero(air.gravity) {
        NO_TIME
    } else {
        -(-air.terminal_velocity - d.y) / air.gravity
    };
    let root = |t: f32| if t > 0.0 { sqrtf(t) } else { t };
    let land_y = if to_terminal <= 0.0 {
        // 800AC17C: fmadds.
        fmadds(d.y, to_destination, p.y)
    } else if to_destination < to_terminal {
        // 800AC1F0 / 800AC1F8 fmuls, 800AC200 fnmsub and 800AC204 fadd in
        // double, then frsp.
        let fall = air.gravity * root(to_destination);
        let rise = d.y * to_destination;
        (f64::from(p.y) + fnmsub(0.5, f64::from(fall), f64::from(rise))) as f32
    } else {
        // 800AC270..800AC298: fsubs, three fmuls, fnmsub, fsub and fadd in
        // double, then frsp.
        let fall = air.gravity * root(to_terminal);
        let rise = d.y * to_terminal;
        let at_terminal = (to_destination - to_terminal) * air.terminal_velocity;
        let height = fnmsub(0.5, f64::from(fall), f64::from(rise)) - f64::from(at_terminal);
        (f64::from(p.y) + height) as f32
    };
    f64::from(to_destination) < 0.0 || land_y < destination.y
}
