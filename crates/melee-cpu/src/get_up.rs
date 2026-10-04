//! The CPU lying on the floor or hanging from a ledge (ftCo_0A01.c):
//! behaviours 5 (ftCo_800AC7D4) and 6 (ftCo_800ACB44) choose how it gets
//! back up.
use crate::{
    attack::ledge_state,
    movement::finish_with_neutral_y,
    script::{self, Command as C},
    select::downed,
    targets::nearest_opponent,
    world::{distance, Scene},
};
use gekko_math::HsdRng;
use melee_ft::fighter::Fighter;

/// CPU modes 0 and 15 take the plain option without looking for a target.
fn plain_mode(fp: &Fighter) -> bool {
    matches!(fp.core.cpu.mode, 0 | 0xF)
}

/// The nearest opponent (ftCo_800A4BEC) and its distance (ftCo_800A1AB4).
fn opponent(fp: &mut Fighter, scene: &Scene, rng: &mut HsdRng) -> Option<(f32, f32)> {
    let index = nearest_opponent(fp, scene, rng)?;
    let theirs = scene.fighter(index).core.physics.position;
    Some((distance(fp.core.physics.position, theirs), theirs.x))
}

/// ftCo_800AC7D4 (0x800AC7D4): behaviour 5. Bouncing, the CPU waits;
/// lying down it stands (stick up for a frame), unless an opponent is
/// within 15: then a get-up attack at `0.1 * level` odds (800AC9B0: fmuls,
/// Randf below it), otherwise a roll away from the opponent.
pub fn down(fp: &mut Fighter, scene: &Scene, rng: &mut HsdRng) {
    match downed(fp) {
        0 => {
            script::return_to_previous(&mut fp.core.cpu);
            return;
        }
        1 => {
            script::finish_with_neutral_stick(&mut fp.core.cpu);
            return;
        }
        _ => {}
    }
    if plain_mode(fp) {
        stand_up(fp);
        return;
    }
    let Some((distance, their_x)) = opponent(fp, scene, rng) else {
        stand_up(fp);
        return;
    };
    // 800AC97C: the distance against a double 15.0.
    if f64::from(distance) >= 15.0 {
        stand_up(fp);
        return;
    }
    let odds = 0.1f32 * fp.core.cpu.level as f32;
    if rng.randf() < odds {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickY, 0);
        script::command1(cpu, C::SetLstickX, 0);
        retap_a(fp);
        return;
    }
    // 800ACA3C: fsubs, compared with zero.
    let away: u8 = if their_x - fp.core.physics.position.x < 0.0 {
        0x50
    } else {
        0xB0
    };
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickX, away);
    script::command1(cpu, C::SetLstickY, 0);
    script::command1(cpu, C::WaitFor, 0xA);
    script::command1(cpu, C::SetLstickX, 0);
    script::command(cpu, C::Done);
}

/// ftCo_800ACB44 (0x800ACB44): behaviour 6. Catching the ledge, the CPU
/// waits; hanging, with a close attack open (xF9_b2) on an opponent within
/// 30 it rolls up at `0.1 * level` odds (800ACC8C: the product above
/// Randf) or attacks; otherwise it takes a random option.
pub fn ledge(fp: &mut Fighter, scene: &Scene, rng: &mut HsdRng) {
    match ledge_state(fp) {
        1 => {
            script::finish_with_neutral_stick(&mut fp.core.cpu);
            return;
        }
        0 => {
            script::return_to_previous(&mut fp.core.cpu);
            return;
        }
        _ => {}
    }
    if plain_mode(fp) {
        random_ledge_option(fp, rng);
        return;
    }
    let Some((distance, _)) = opponent(fp, scene, rng) else {
        random_ledge_option(fp, rng);
        return;
    };
    // 800ACC54: the distance against a double 30.0.
    if fp.core.cpu.xf9_b2 && f64::from(distance) < 30.0 {
        let odds = 0.1f32 * fp.core.cpu.level as f32;
        if odds > rng.randf() {
            retap_r(fp);
        } else {
            retap_a(fp);
        }
    } else {
        random_ledge_option(fp, rng);
    }
}

/// ftCo_800A0AF4 (0x800A0AF4): from the ledge, by one Randf: below 0.6
/// climb (stick up 0x50), below 0.8 roll (R), below 0.9 attack (A), else
/// jump (Y).
fn random_ledge_option(fp: &mut Fighter, rng: &mut HsdRng) {
    let roll = rng.randf();
    if roll < 0.6 {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickX, 0);
        script::command1(cpu, C::SetLstickY, 0x50);
        script::command1(cpu, C::WaitFor, 1);
        finish_with_neutral_y(fp);
    } else if roll < 0.8 {
        retap_r(fp);
    } else if roll < 0.9 {
        retap_a(fp);
    } else {
        let cpu = &mut fp.core.cpu;
        script::command(cpu, C::ReleaseY);
        script::command1(cpu, C::WaitFor, 1);
        script::command(cpu, C::PressY);
        script::command1(cpu, C::WaitFor, 1);
        script::command(cpu, C::ReleaseY);
        script::command(cpu, C::Done);
    }
}

/// ftCo_CpuHoldUpForOneFrame.
fn stand_up(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::SetLstickY, 0x7F);
    script::command1(cpu, C::WaitFor, 1);
    finish_with_neutral_y(fp);
}

/// ftCo_CpuRetapA: A released, a tick, pressed for a tick.
fn retap_a(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::ReleaseA);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::PressA);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseA);
    script::command(cpu, C::Done);
}

/// ftCo_CpuRetapR: R released, a tick, pressed for a tick.
fn retap_r(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::ReleaseR);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::PressR);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseR);
    script::command(cpu, C::Done);
}
