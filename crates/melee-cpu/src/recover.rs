//! Recovery (ftCo_0A01.c): behaviour 4 (ftCo_800B2790's case 4) and the
//! island ends a falling CPU can drift to. ftCo_800A4038 looks right of
//! the fighter at each floor island's left end, ftCo_800A3908 left of it
//! at each right end; ftCo_800A8DE4 asks them once per recovery (it is
//! also the partner's Belay recovery); ftCo_800A9904 steers or jumps
//! toward the end chosen.
use crate::{
    script::{self, Command as C},
    world::Scene,
};
use gekko_math::{
    fma::{fmadds, fnmsub},
    msl::{fctiwz, sqrtf},
};
use melee_ft::fighter::Fighter;
use melee_types::{GrKind, GroundOrAir};

/// ftCo_800A8DE4: x5C before the search (retail @319).
const FAR: f32 = 10000.0;
/// The CPU heads 5 inside the island's end, arriving within 5 (retail
/// @282-area 5.0f and the double 5.0 of `5.0 + ex`).
const INSET: f64 = 5.0;
const ARRIVAL_RADIUS: f32 = 5.0;
/// The floor probe runs from 5 above the end to 5 below (retail @297).
const PROBE: f32 = 5.0;
/// Below this gravity the fall never reaches terminal speed (@305, @306):
/// 1000 frames.
const ZERO_GRAVITY: f32 = 0.00001;
const NO_TERMINAL_FRAMES: i32 = 1000;

/// Which island end, on which side of the fighter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    /// ftCo_800A4038: left ends to the right of the fighter.
    Right,
    /// ftCo_800A3908: right ends to the left of the fighter.
    Left,
}

/// ftCo_800A8DE4 (0x800A8DE4): once per recovery (xFA_b2), look for an
/// island end ahead that the fall reaches, then behind, then settle for
/// the nearest end ahead.
pub fn belay_recovery(fp: &mut Fighter, scene: &mut Scene) {
    if fp.core.cpu.xfa_b2 {
        return;
    }
    fp.core.cpu.xfa_b2 = true;
    fp.core.cpu.x5c = FAR;
    let (ahead, behind) = if f64::from(fp.core.physics.facing) > 0.0 {
        (Side::Right, Side::Left)
    } else {
        (Side::Left, Side::Right)
    };
    if !head_for_end(fp, scene, ahead, true) && !head_for_end(fp, scene, behind, true) {
        head_for_end(fp, scene, ahead, false);
    }
}

/// The frames until the fall reaches terminal speed:
/// `-(-terminal - pos_delta.y) / gravity` (fneg, fsubs, fneg, fdivs,
/// fctiwz), or 1000 without gravity.
fn frames_to_terminal(fp: &Fighter) -> i32 {
    let air = &fp.core.attributes.air;
    let gravity = air.gravity;
    if gravity < ZERO_GRAVITY && gravity > -ZERO_GRAVITY {
        return NO_TERMINAL_FRAMES;
    }
    let speed = -((-air.terminal_velocity) - fp.core.physics.position_delta.y);
    fctiwz(speed / gravity)
}

/// The height after `t` frames of the fall, with the quirk that the
/// gravity term takes sqrtf of the frame count (800A41E0..800A4418).
fn height_after(fp: &Fighter, t: i32, frames: i32) -> f32 {
    let air = &fp.core.attributes.air;
    let y = fp.core.physics.position.y;
    let rise_per_frame = fp.core.physics.position_delta.y;
    if frames <= 0 {
        // 800A41FC: fmadds.
        return fmadds(rise_per_frame, t as f32, y);
    }
    if t < frames {
        let fall = air.gravity * sqrtf(t as f32);
        let rise = rise_per_frame * t as f32;
        // 800A42F8: fnmsub, then fadd in double.
        return (f64::from(y) + fnmsub(0.5, f64::from(fall), f64::from(rise))) as f32;
    }
    let fall = air.gravity * sqrtf(frames as f32);
    let rise = rise_per_frame * frames as f32;
    let drop = (t - frames) as f32 * air.terminal_velocity;
    // 800A4408: fnmsub, fsub and fadd in double.
    (f64::from(y) + (fnmsub(0.5, f64::from(fall), f64::from(rise)) - f64::from(drop))) as f32
}

/// ftCo_800A4038 (0x800A4038) / ftCo_800A3908 (0x800A3908): each floor
/// island's end on `side` inside the blast zones. With `reachable`, the
/// first whose height the fall (plus a jump) still reaches, with a floor
/// 5 inside it, becomes the destination; without, every such end nearer
/// than x5C does.
fn head_for_end(fp: &mut Fighter, scene: &mut Scene, side: Side, reachable: bool) -> bool {
    let frames = frames_to_terminal(fp);
    let islands: melee_types::fixed::FixedVec<usize, 64> = {
        let mut list = melee_types::fixed::FixedVec::default();
        for island in scene.map.floor_islands() {
            list.push(island);
        }
        list
    };
    for island in islands.iter().copied() {
        let island = *scene.map.island(island);
        let end = match side {
            Side::Right => island.left,
            Side::Left => island.right,
        };
        let (ex, ey) = (end.x, end.y);
        let half_size = fp.core.cpu.half_size;
        if scene.outside(ex, ey, half_size) {
            continue;
        }
        if scene.stage() == GrKind::Corneria {
            unimplemented!("grCorneria_801E2E50: Corneria's ceilinged floors");
        }
        let position = fp.core.physics.position;
        let dx = match side {
            Side::Right => ex - position.x,
            Side::Left => position.x - ex,
        };
        if dx <= 0.0 {
            continue;
        }
        let t = fctiwz(dx / fp.core.attributes.air.air_drift_max);
        let height = height_after(fp, t, frames);
        // 5 inside the end, in double, rounded.
        let x = match side {
            Side::Right => (INSET + f64::from(ex)) as f32,
            Side::Left => (f64::from(ex) - INSET) as f32,
        };
        if reachable {
            if height + fp.core.cpu.jump_height < ey {
                continue;
            }
            if landable(scene, x, ey, half_size) {
                crate::route::set_destination(fp, scene, x, ey, ARRIVAL_RADIUS);
                crate::select::note_destination_distance(fp);
                return true;
            }
        } else {
            let to_x = x - position.x;
            let to_y = ey - position.y;
            if landable(scene, x, ey, half_size) {
                // 800A4688: fmadds, inline sqrtf.
                let distance = sqrtf(fmadds(to_x, to_x, to_y * to_y));
                if fp.core.cpu.x5c > distance {
                    crate::route::set_destination(fp, scene, x, ey, ARRIVAL_RADIUS);
                    crate::select::note_destination_distance(fp);
                }
            }
        }
    }
    false
}

/// A usable floor from 5 above (x, y) to 5 below (fadds, fsubs), and the
/// point inside the blast zones.
fn landable(scene: &mut Scene, x: f32, y: f32, half_size: [f32; 2]) -> bool {
    scene
        .check_usable_floor(x, PROBE + y, x, y - PROBE)
        .is_some()
        && !scene.outside(x, y, half_size)
}

/// ftCo_800A9904: a zero horizontal speed or gravity counts as 1000
/// frames (retail @295).
const NEVER: f32 = 1000.0;
/// ftCo_800A9904's drift toward the destination over the stage: `4.7 *
/// (level + 1) + 80` (@341, @336; fmadds, fctiwz).
const DRIFT_PER_LEVEL: f32 = 4.700_000_3;
const DRIFT_BASE: f32 = 80.0;
/// A ceiling on the way: the stick goes away from the destination
/// (0x81 = -127 as the command's signed byte).
const AWAY: u8 = 0x81;
const FULL: u8 = 0x7F;
/// ftCo_CpuRecoverDiagonally: the up-special's stick.
const DIAGONAL: u8 = 0x58;

/// ftCo_800B2790's case 4 (recovery): pick an island end once
/// (ftCo_800A8DE4); on the ground, back to the previous behaviour, else
/// head for it (ftCo_800A9904).
pub fn recover(fp: &mut Fighter, scene: &mut Scene) {
    belay_recovery(fp, scene);
    if fp.core.physics.ground_or_air == GroundOrAir::Ground {
        if matches!(
            scene.stage(),
            GrKind::BigBlue | GrKind::Inishie1 | GrKind::Corneria | GrKind::Venom
        ) {
            unimplemented!("ftCo_800B2790 case 4: a stage's moving floor (ftCo_800A0148)");
        }
        script::return_to_previous(&mut fp.core.cpu);
        fp.core.cpu.xfa_b2 = false;
    } else {
        steer(fp, scene);
    }
}

/// ftCo_800A3498 (0x800A3498): off the stage, falling (or still), and
/// against a wall or below the destination (under the ECB's bottom plus
/// x568; Luigi whenever falling).
fn below_destination(fp: &Fighter) -> bool {
    let c = &fp.core;
    if c.cpu.over_stage
        || c.physics.ground_or_air != GroundOrAir::Air
        || f64::from(c.physics.position_delta.y) > 0.0
    {
        return false;
    }
    if c.collision.data.env_flags as u32 & melee_types::mp::collide::WALL_MASK != 0 {
        return true;
    }
    if c.kind == melee_types::FighterKind::Luigi {
        return f64::from(c.physics.position_delta.y) < 0.0;
    }
    let cd = &c.collision.data;
    // 800A3530..3538: two fadds.
    c.cpu.destination.y > c.cpu.hurtbox_extents[3] + (cd.cur_pos.y + cd.ecb.bottom.y)
}

/// ftCo_800A9904 (0x800A9904): below the destination, a jump toward it
/// while one is left, else the up special (ftCo_800A96B8); over the stage,
/// drift in while the fall would land short of it; otherwise full drift
/// toward it, or away under a ceiling.
fn steer(fp: &mut Fighter, scene: &mut Scene) {
    if below_destination(fp) {
        if fp.core.attributes.jumping.max_jumps > i32::from(fp.core.physics.jumps_used) {
            let cpu = &mut fp.core.cpu;
            script::command1(cpu, C::LstickTowardDestination, FULL);
            script::command(cpu, C::PressY);
            script::command1(cpu, C::WaitFor, 1);
            script::command(cpu, C::ReleaseY);
            script::command(cpu, C::Done);
        } else {
            up_special(fp);
        }
        return;
    }
    let position = fp.core.physics.position;
    let destination = fp.core.cpu.destination;
    if fp.core.cpu.over_stage {
        let stick = if lands_short(fp) {
            let level = (fp.core.cpu.level + 1) as f32;
            // 800A9BA8: fmadds, fctiwz; the command keeps the low byte.
            let stick = fctiwz(fmadds(DRIFT_PER_LEVEL, level, DRIFT_BASE));
            Some(stick as u8)
        } else {
            None
        };
        let cpu = &mut fp.core.cpu;
        match stick {
            Some(stick) => script::command1(cpu, C::LstickXTowardDestination, stick),
            None => script::command1(cpu, C::SetLstickX, 0),
        }
    } else {
        let ceiling = scene
            .map
            .check_ceiling(position.x, position.y, destination.x, destination.y, -1, -1)
            .is_some();
        let stick = if ceiling { AWAY } else { FULL };
        script::command1(&mut fp.core.cpu, C::LstickXTowardDestination, stick);
    }
    // ftCo_CpuFinishWithNeutralY.
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, 0);
    script::command(cpu, C::Done);
}

/// ftCo_800A9904's over-stage test: the frames to reach the destination's
/// x at the current speed, and the height then (the sqrtf quirk again,
/// 800A99E0..800A9B70); true when that is still moving away (negative)
/// or below the destination.
fn lands_short(fp: &Fighter) -> bool {
    let c = &fp.core;
    let air = &c.attributes.air;
    let delta = c.physics.position_delta;
    let near_zero = |x: f32| x < ZERO_GRAVITY && x > -ZERO_GRAVITY;
    let x_time = if near_zero(delta.x) {
        NEVER
    } else {
        (c.cpu.destination.x - c.physics.position.x) / delta.x
    };
    let terminal_time = if near_zero(air.gravity) {
        NEVER
    } else {
        -((-air.terminal_velocity) - delta.y) / air.gravity
    };
    let y = c.physics.position.y;
    let height = if terminal_time <= 0.0 {
        // 800A9A50: fmadds.
        fmadds(delta.y, x_time, y)
    } else if x_time < terminal_time {
        let fall = air.gravity * sqrtf(x_time);
        let rise = delta.y * x_time;
        // 800A9AD4: fnmsub, fadd in double.
        (f64::from(y) + fnmsub(0.5, f64::from(fall), f64::from(rise))) as f32
    } else {
        let beyond = x_time - terminal_time;
        let fall = air.gravity * sqrtf(terminal_time);
        let rise = delta.y * terminal_time;
        let drop = beyond * air.terminal_velocity;
        // 800A9B60: fnmsub, fsub and fadd in double.
        (f64::from(y) + (fnmsub(0.5, f64::from(fall), f64::from(rise)) - f64::from(drop))) as f32
    };
    f64::from(x_time) < 0.0 || height < c.cpu.destination.y
}

/// ftCo_800A96B8 (0x800A96B8): the kind's up special toward the stage.
/// The climbers take the default, diagonal (ftCo_CpuRecoverDiagonally);
/// the special-cased kinds are unported.
fn up_special(fp: &mut Fighter) {
    use melee_types::FighterKind as K;
    if matches!(
        fp.core.kind,
        K::Pikachu
            | K::Pichu
            | K::Fox
            | K::Falco
            | K::Yoshi
            | K::Ness
            | K::Luigi
            | K::Zelda
            | K::Samus
    ) {
        unimplemented!("ftCo_800A96B8: {:?}'s recovery", fp.core.kind);
    }
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, DIAGONAL);
    script::command1(cpu, C::LstickXTowardDestination, DIAGONAL);
    script::command1(cpu, C::PressBFor, 1);
    script::command(cpu, C::ReleaseB);
    script::command1(cpu, C::LstickXTowardDestination, FULL);
    script::command(cpu, C::Done);
}
