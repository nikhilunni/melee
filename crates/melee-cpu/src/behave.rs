//! ftCo_800B2790: when no command script is running, the current
//! behaviour (CpuFighter.x18) writes the next one.
use crate::{
    movement,
    script::{self, Command as C},
    select::behavior as B,
    world::{distance, Scene},
};
use gekko_math::{fma::fmadds, msl::fctiwz, HsdRng};
use melee_ft::fighter::Fighter;
use melee_types::{CommonMotionState as S, GroundOrAir};

/// ftCo_800B2790 (0x800B2790).
pub fn write_script(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    if script::running(&fp.core.cpu.script) {
        return;
    }
    let cpu = &mut fp.core.cpu;
    cpu.x80 += 1;
    cpu.xf8_b7 = false;
    script::restart(cpu);
    character_script(fp);
    match fp.core.cpu.behavior {
        B::IDLE => idle(fp, scene, rng),
        B::STAND_BY => {
            script::clear(&mut fp.core.cpu);
            script::command(&mut fp.core.cpu, C::Done);
        }
        B::HOLD => crate::hold::hold(fp, scene, rng),
        B::ARRIVED => crate::arrived::arrived(fp, scene),
        B::CAPTURED => crate::captured::mash(fp, rng),
        behavior @ (2..=19) => {
            unimplemented!("ftCo_800B2790: behaviour {behavior}")
        }
        _ => script::command(&mut fp.core.cpu, C::Done),
    }
    script::start(&mut fp.core.cpu);
}

/// ftCo_800ADC28 (0x800ADC28): charge-release taps for Donkey Kong,
/// Samus, Mewtwo and Kirby's copies; nothing for the ported kinds.
fn character_script(fp: &mut Fighter) {
    use melee_types::FighterKind as K;
    if fp.core.cpu.behavior == B::ARRIVED {
        return;
    }
    if matches!(fp.core.kind, K::Donkey | K::Samus | K::Mewtwo | K::Kirby) {
        unimplemented!("ftCo_800ADC28: {:?}'s charge release", fp.core.kind);
    }
}

/// ftCo_800ABA34 (0x800ABA34): behaviour 1 for the ported CPU modes.
fn idle(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    let mode = fp.core.cpu.mode;
    if matches!(mode, 11 | 12 | 16) {
        unimplemented!("ftCo_800ABA34: CPU mode {mode}");
    }
    use melee_types::FighterKind as K;
    if matches!(fp.core.kind, K::Zelda | K::Seak) && fp.core.cpu.xf8_b5 {
        unimplemented!("ftCo_800ABA34: Zelda's transformation");
    }
    if fp.core.motion_state.action.0 == S::RebirthWait as u16 {
        // ftCo_800A5CE0: no one carries the star or the hammer.
        unimplemented!("ftCo_800A08F0: leaving the revival platform");
    }
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        movement::toward_destination_in_air(fp, scene);
    } else {
        toward_destination_on_ground(fp, scene, rng);
    }
}

/// ftCo_800AB224 (0x800AB224): grounded movement toward the destination.
fn toward_destination_on_ground(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    if fp.core.cpu.xfa_b6 {
        unimplemented!("ftCo_800AB224: under Corneria's ceiling");
    }
    if fp.core.cpu.xf9_b1 {
        attack_approach(fp, scene, rng);
        return;
    }
    walk_or_climb(fp, scene);
}

/// ftCo_800AB224's xF9_b1 arm: close in on the target (ftCo_800A0148 at
/// close range), else a level-scaled chance of ftCo_800BA2E8.
fn attack_approach(_fp: &mut Fighter, _scene: &mut Scene, _rng: &mut HsdRng) {
    unimplemented!("ftCo_800AB224: approaching the target (xF9_b1, ftCo_0A01.c:5202)");
}

/// ftCo_800AB224's block_49: walk when the destination is on this island,
/// otherwise climb or jump toward it by its angle.
fn walk_or_climb(fp: &mut Fighter, scene: &mut Scene) {
    if crate::route::destination_on_island(fp, scene) {
        walk(fp, scene);
        return;
    }
    unimplemented!("ftCo_800AB224: a destination off this island (ftCo_0A01.c:5244)");
}

/// ftCo_800AA320 (0x800AA320): the stick step and limit by level; Nana
/// steers harder so she keeps up.
fn stick_limits(fp: &Fighter) -> (i32, i32) {
    if fp.core.capabilities.cpu_partner {
        return (0x40, 0x7F);
    }
    match fp.core.cpu.level {
        0 => (1, 0x43),
        1 => (2, 0x48),
        2 => (2, 0x4D),
        3 => (2, 0x52),
        4 => (2, 0x57),
        5 => (2, 0x5C),
        6 => (4, 0x61),
        7 => (4, 0x6B),
        8 => (8, 0x75),
        9 => (8, 0x7F),
        level => unreachable!("CPU level {level}"),
    }
}

/// ftCo_800AA42C (0x800AA42C): walk (or run) toward the destination on
/// the same island, easing off near it.
fn walk(fp: &mut Fighter, scene: &Scene) {
    let (step, limit) = stick_limits(fp);
    let motion = fp.core.motion_state.action.0;
    if motion == S::Ottotto as u16 || motion == S::OttottoWait as u16 {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickY, 0);
        script::command1(cpu, C::SetLstickX, 0);
        script::command1(cpu, C::WaitFor, 1);
        script::command1(cpu, C::LstickXForward, 0x7F);
        script::command1(cpu, C::WaitFor, 1);
        script::command1(cpu, C::SetLstickX, 0);
        script::command(cpu, C::Done);
        return;
    }
    let position = fp.core.physics.position;
    let destination = fp.core.cpu.destination;
    // 800AA504: fmuls, a double compare.
    if f64::from(fp.core.physics.facing * (destination.x - position.x)) < 0.0 {
        turn_around(fp);
        return;
    }
    let dist = distance(
        hsd_types::Vec3::new(destination.x, destination.y, 0.0),
        position,
    );
    let cpu = &mut fp.core.cpu;
    let near = cpu.destination_radius;
    if dist - near > cpu.x3c {
        script::command1(cpu, C::SetLstickY, 0);
        if cpu.xf8_b0 {
            let running = [S::Dash, S::Run, S::TurnRun]
                .iter()
                .any(|&s| motion == s as u16);
            if running {
                script::command1(cpu, C::LstickXTowardDestination, 0x7F);
            } else {
                script::command1(cpu, C::LstickXTowardDestination, 0);
                script::command1(cpu, C::WaitFor, 2);
                script::command1(cpu, C::LstickXTowardDestination, 0x7F);
            }
        } else {
            script::command2(
                cpu,
                C::LstickXTowardDestinationClamped,
                step as u8,
                limit as u8,
            );
        }
    } else if dist > near {
        script::command1(cpu, C::SetLstickY, 0);
        let eased = ease(limit, dist, near, cpu.x3c, scene.horizontal_deadzone);
        script::command2(
            cpu,
            C::LstickXTowardDestinationClamped,
            step as u8,
            eased as u8,
        );
    } else {
        script::command1(cpu, C::SetLstickX, 0);
        script::command1(cpu, C::SetLstickY, 0);
    }
    script::command(cpu, C::Done);
}

/// ftCo_800AA42C_inline0: the stick limit easing from `limit` beyond `far`
/// to the dead zone's edge (at least 80) at `near`.
fn ease(limit: i32, dist: f32, near: f32, far: f32, deadzone: f32) -> i32 {
    let limit_f = limit as f32;
    if dist > far {
        return fctiwz(limit_f);
    }
    if dist < near {
        return 0;
    }
    // 800AA768: fmuls.
    let mut raw = 127.0 * deadzone;
    if raw > limit_f {
        raw = limit_f;
    }
    if raw < 80.0 {
        raw = 80.0;
    }
    // 800AA788..800AA7A4: fsubs, fsubs, fdivs, fmadds, fctiwz.
    let scale = (dist - near) / (far - near);
    let eased = fctiwz(fmadds(scale, limit_f - raw, raw));
    if eased as f32 > limit_f {
        fctiwz(limit_f)
    } else {
        eased
    }
}

/// ftCo_CpuTurnAround.
pub(crate) fn turn_around(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::SetLstickY, 0);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::LstickXForward, 0xB0);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickY, 0);
    script::command1(cpu, C::WaitFor, 10);
    script::command1(cpu, C::SetLstickX, 0);
    script::command(cpu, C::Done);
}
