//! ftCo_800A96B8: the kind's recovery special toward the stage, the last
//! resort of a CPU below its destination with no jump left.
use crate::script::{self, Command as C};
use gekko_math::{
    msl::{cosf, fabsf, fctiwz, sinf},
    HsdRng,
};
use melee_ft::fighter::cpu_profile::Recovery;
use melee_ft::fighter::Fighter;

/// A full stick and ftCo_CpuRecoverDiagonally's diagonal.
const FULL: u8 = 0x7F;
const DIAGONAL: u8 = 0x58;
/// Zelda holds B this long through Farore's Wind.
const FARORE_HOLD: u8 = 40;
/// Pikachu's and Pichu's Quick Attack legs.
const QUICK_ATTACK_LEG: u8 = 15;
/// Pikachu and Pichu aim the first leg diagonally beyond this x distance
/// (@340, a double).
const QUICK_ATTACK_FAR: f64 = 60.0;
/// Zelda teleports diagonally beyond this x distance (@283, a double).
const FARORE_FAR: f64 = 30.0;
/// Ness waits until this far below the destination (@302, a double).
const PK_THUNDER_DEPTH: f64 = 10.0;
/// Ness steers PK Thunder only from level 9 (800A8ED4: cmpwi, bge).
const PK_THUNDER_LEVEL: i32 = 9;

/// ftCo_800A96B8 (0x800A96B8): the arm the kind's profile names.
pub(crate) fn up_special(fp: &mut Fighter, rng: &mut HsdRng) {
    match fp.character.table().cpu_profile.recovery {
        Recovery::TimedSpecial(wait) => {
            let cpu = &mut fp.core.cpu;
            script::command1(cpu, C::SetLstickY, DIAGONAL);
            script::command1(cpu, C::LstickXTowardDestination, DIAGONAL);
            script::command1(cpu, C::PressBFor, 1);
            script::command(cpu, C::ReleaseB);
            script::command1(cpu, C::WaitFor, wait);
            script::command(cpu, C::Done);
        }
        // 800A9824..800A9878: the Green Missile toward the stage.
        Recovery::SideSpecial(wait) => {
            let cpu = &mut fp.core.cpu;
            script::command1(cpu, C::SetLstickY, 0);
            script::command1(cpu, C::LstickXTowardDestination, FULL);
            script::command1(cpu, C::PressBFor, 1);
            script::command(cpu, C::ReleaseB);
            script::command1(cpu, C::WaitFor, wait);
            script::command(cpu, C::Done);
        }
        Recovery::QuickAttack => quick_attack(fp, rng),
        Recovery::PkThunder => {
            // 800A97F0..800A9804: y > x54.y - 10.0 in double.
            let depth = f64::from(fp.core.cpu.destination.y) - PK_THUNDER_DEPTH;
            if f64::from(fp.core.physics.position.y) > depth {
                script::command(&mut fp.core.cpu, C::Done);
            } else {
                pk_thunder(fp);
            }
        }
        Recovery::FaroresWind => farores_wind(fp),
        Recovery::None => script::command(&mut fp.core.cpu, C::Done),
        Recovery::Diagonal => diagonal(fp),
    }
}

/// ftCo_CpuRecoverDiagonally (and ftCo_800A963C): up special on the
/// diagonal stick, then full drift toward the destination.
fn diagonal(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, DIAGONAL);
    script::command1(cpu, C::LstickXTowardDestination, DIAGONAL);
    script::command1(cpu, C::PressBFor, 1);
    script::command(cpu, C::ReleaseB);
    script::command1(cpu, C::LstickXTowardDestination, FULL);
    script::command(cpu, C::Done);
}

/// ftCo_CpuStartUpSpecial: up special with the stick straight up.
fn start_up_special(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, FULL);
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::PressBFor, 1);
    script::command(cpu, C::ReleaseB);
}

/// ftCo_800A92CC (0x800A92CC): Quick Attack. Far out, a diagonal leg then
/// a level one; near, a straight one, then diagonal or level at random
/// (Randf() > 0.5, a double compare).
fn quick_attack(fp: &mut Fighter, rng: &mut HsdRng) {
    // 800A92F0..800A9318: fsubs, negated below zero; a double compare.
    let far = f64::from(fabsf(fp.core.physics.position.x - fp.core.cpu.destination.x))
        > QUICK_ATTACK_FAR;
    start_up_special(fp);
    if far {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickY, DIAGONAL);
        script::command1(cpu, C::LstickXTowardDestination, DIAGONAL);
        script::command1(cpu, C::WaitFor, QUICK_ATTACK_LEG);
        script::command1(cpu, C::LstickXTowardDestination, FULL);
        script::command1(cpu, C::SetLstickY, 0);
    } else {
        script::command1(&mut fp.core.cpu, C::WaitFor, QUICK_ATTACK_LEG);
        let up = f64::from(rng.randf()) > 0.5;
        let cpu = &mut fp.core.cpu;
        if up {
            script::command1(cpu, C::LstickXTowardDestination, DIAGONAL);
            script::command1(cpu, C::SetLstickY, DIAGONAL);
        } else {
            script::command1(cpu, C::LstickXTowardDestination, FULL);
            script::command1(cpu, C::SetLstickY, 0);
        }
    }
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::WaitFor, QUICK_ATTACK_LEG);
    script::command(cpu, C::Done);
}

/// ftCo_800A949C (0x800A949C): Farore's Wind, diagonal toward the
/// destination when it is more than 30 away in x, else straight up.
fn farores_wind(fp: &mut Fighter) {
    let far = f64::from(fabsf(fp.core.cpu.destination.x - fp.core.physics.position.x))
        > FARORE_FAR;
    let cpu = &mut fp.core.cpu;
    script::neutral_stick(cpu);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickY, FULL);
    if far {
        script::command1(cpu, C::LstickXTowardDestination, FULL);
        script::command1(cpu, C::PressBFor, FARORE_HOLD);
        script::command1(cpu, C::SetLstickY, 0);
        script::command1(cpu, C::SetLstickX, 0);
    } else {
        script::command1(cpu, C::SetLstickX, 0);
        script::command1(cpu, C::PressBFor, FARORE_HOLD);
        script::command1(cpu, C::SetLstickY, 0);
    }
    script::command(cpu, C::ReleaseB);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::Done);
}

/// ftCo_CpuAimPkThunder: the stick at `angle` (127 * cosf/sinf: fmuls,
/// fctiwz), held `ticks`.
fn aim_pk_thunder(fp: &mut Fighter, angle: f32, ticks: u8) {
    let x = fctiwz(127.0 * cosf(angle));
    let y = fctiwz(127.0 * sinf(angle));
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickX, x as u8);
    script::command1(cpu, C::SetLstickY, y as u8);
    script::command1(cpu, C::WaitFor, ticks);
}

/// ftCo_800A8EB0 (0x800A8EB0): at level 9, PK Thunder steered around
/// under Ness and back up into him: away, down, toward, up.
fn pk_thunder(fp: &mut Fighter) {
    use core::f32::consts::{FRAC_PI_2, PI};
    if fp.core.cpu.level < PK_THUNDER_LEVEL {
        script::command(&mut fp.core.cpu, C::Done);
        return;
    }
    start_up_special(fp);
    // 800A8F34..800A8F3C: x54.x - x > 0.0, a double compare.
    let right = f64::from(fp.core.cpu.destination.x - fp.core.physics.position.x) > 0.0;
    let (first, third) = if right { (PI, 0.0) } else { (0.0, PI) };
    aim_pk_thunder(fp, first, 20);
    aim_pk_thunder(fp, -FRAC_PI_2, 35);
    aim_pk_thunder(fp, third, 20);
    aim_pk_thunder(fp, FRAC_PI_2, 20);
    script::command(&mut fp.core.cpu, C::Done);
}
