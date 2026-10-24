//! Behaviour 18 (ftCo_800AC5A0): steer out of a hit's knockback during
//! hitlag, and tech with R when xFA_b1 rolled so.
use crate::partner::StepScratch;
use crate::script::{self, Command as C};
use crate::world::Scene;
use gekko_math::{
    fma::fmadds,
    msl::{fctiwz, sqrtf},
};
use melee_ft::fighter::Fighter;

/// ftCo_IsNearlyZero's bounds.
const NEARLY_ZERO: f32 = 0.00001;

/// ftCo_800AC5A0 (0x800AC5A0).
pub fn steer(fp: &mut Fighter, scene: &Scene) {
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
        let (x, y) = away_from_knockback(fp, scene);
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
/// fctiwz. With no knockback (a pummel) 800AC6D0 branches past both stores
/// to 800AC744 and retail passes two registers the function never set.
fn away_from_knockback(fp: &Fighter, scene: &Scene) -> (i8, i8) {
    let kb = fp.core.physics.knockback_velocity;
    // 800AC648: fmuls, fmadds, then the inlined sqrtf when positive.
    let squared = fmadds(kb.x, kb.x, kb.y * kb.y);
    let magnitude = if squared > 0.0 {
        sqrtf(squared)
    } else {
        squared
    };
    if magnitude < NEARLY_ZERO && magnitude > -NEARLY_ZERO {
        return unset_registers(fp, scene);
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

/// The low bytes of r5 and r30 as they reach 800AC744 unset.
///
/// - SetLstickY takes r30 (800AC754), which the function saved and did not
///   write: ftCo_800B2790 keeps the Fighter pointer there (800B27A4: mr
///   r30, r3), so the stick's y is the low byte of the fighter's address.
/// - SetLstickX takes r5 as it arrived. Neither ftCo_800B2790 nor
///   ftCo_800B462C writes it, ftCo_800ADC28 only for Donkey Kong, Samus,
///   Mewtwo and Kirby (li r5, 1), and ftCo_800B3900 calls nothing between
///   ftCo_800B2AFC and ftCo_800B2790: it is what the mode's think left.
///   The partner's (ftCo_800B101C) ends its seek in ftCo_800B0E98, whose
///   only write is ftCo_800B0CA8's scratch use (`partner::step_scratch`).
///
/// Retail, `corpus_v3_fd_iceclimbers_peach4_ecad2716f_p0` ticks 950-951:
/// Nana, pummelled, her Fighter at 0x80C9D2A0, steers y = -96 (0xA0) and
/// x = 0 (Popo's motion, 80, forbids nothing; her hands are empty).
fn unset_registers(fp: &Fighter, scene: &Scene) -> (i8, i8) {
    let cpu = &fp.core.cpu;
    if cpu.mode != melee_ft::fighter::cpu::PARTNER_MODE || scene.partner_of(fp).is_none() {
        // The Versus think (ftCo_800B24B8) ends in ftCo_800ADE48, which
        // leaves r5 as `StaleStack::r5` tracks it.
        let Some(r5) = scene.stale.r5.get() else {
            unimplemented!("ftCo_800AC5A0: r5 with no knockback, unknown after ftCo_800ADE48");
        };
        let Some(address) = cpu.fighter_address else {
            unimplemented!(
                "ftCo_800AC5A0: r30 with no knockback is the Fighter's address, unknown for this scene"
            );
        };
        return (r5 as u8 as i8, address as u8 as i8);
    }
    if cpu.following {
        unimplemented!(
            "ftCo_800AC5A0: r5 with no knockback after another think than the partner's seek"
        );
    }
    let (_, own) = scene.partner_of(fp).expect("the partner's player's fighter");
    let x = match crate::partner::step_scratch(fp, own) {
        StepScratch::Word(word) => word as u8,
        StepScratch::HeldItemAddress => unimplemented!(
            "ftCo_800AC5A0: r5 with no knockback is the held Item's address (800B0DB8)"
        ),
    };
    let Some(address) = cpu.fighter_address else {
        unimplemented!(
            "ftCo_800AC5A0: r30 with no knockback is the Fighter's address, unknown for this scene"
        );
    };
    (x as i8, address as u8 as i8)
}
