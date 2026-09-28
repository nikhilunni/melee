//! Behaviour 18 (ftCo_800AC5A0): steer out of a hit's knockback during
//! hitlag, and tech with R when xFA_b1 rolled so.
use crate::script::{self, Command as C};
use gekko_math::{
    fma::fmadds,
    msl::{fctiwz, sqrtf},
};
use melee_ft::fighter::Fighter;

/// ftCo_IsNearlyZero's bounds.
const NEARLY_ZERO: f32 = 0.00001;

/// ftCo_800AC5A0 (0x800AC5A0).
pub fn steer(fp: &mut Fighter) {
    if !crate::facts::in_damage_hitlag(fp) {
        let cpu = &mut fp.core.cpu;
        cpu.behavior = cpu.home_behavior;
        script::command(cpu, C::ReleaseR);
        script::command(cpu, C::Done);
        return;
    }
    let cpu = &fp.core.cpu;
    let steering = cpu.reaction_timer % 120 > 120 - cpu.level * 12;
    if steering {
        let (x, y) = away_from_knockback(fp);
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickX, x as u8);
        script::command1(cpu, C::SetLstickY, y as u8);
    } else {
        script::neutral_stick(&mut fp.core.cpu);
    }
    let cpu = &mut fp.core.cpu;
    if cpu.xfa_b1 {
        script::command(cpu, C::PressR);
    }
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::Done);
}

/// The stick bytes 800AC640..800AC740 pass: the knockback direction
/// turned a quarter (ftcpu's own swap of x and y), 127 * the unit vector,
/// fctiwz. With no knockback retail passes two uninitialised registers.
fn away_from_knockback(fp: &Fighter) -> (i8, i8) {
    let kb = fp.core.physics.knockback_velocity;
    // 800AC648: fmuls, fmadds, then the inlined sqrtf when positive.
    let squared = fmadds(kb.x, kb.x, kb.y * kb.y);
    let magnitude = if squared > 0.0 {
        sqrtf(squared)
    } else {
        squared
    };
    if magnitude < NEARLY_ZERO && magnitude > -NEARLY_ZERO {
        unimplemented!("ftCo_800AC5A0: steering with no knockback (uninitialised r5/r30)");
    }
    let inverse = 1.0 / magnitude;
    let x = kb.x * inverse;
    let y = kb.y * inverse;
    if kb.x > 0.0 {
        (fctiwz(127.0 * -y) as i8, fctiwz(127.0 * x) as i8)
    } else {
        (fctiwz(127.0 * y) as i8, fctiwz(127.0 * -x) as i8)
    }
}
