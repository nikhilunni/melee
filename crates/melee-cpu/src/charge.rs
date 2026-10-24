//! Charging and releasing a neutral special: Donkey Kong's Giant Punch,
//! Samus's Charge Shot, Mewtwo's Shadow Ball and Kirby's copies of them
//! (ftCo_800ADC28 before every behaviour's script, ftCo_800ACD5C on
//! arrival).
use crate::script::{self, Command as C};
use melee_ft::fighter::cpu_profile::Charge;
use melee_ft::fighter::Fighter;

/// Charge semantics and loop rows come from the character's static profile.
fn charged(fp: &Fighter) -> Option<Charge> {
    if (fp.character.table().cpu_copied_charge)(&fp.character) {
        unimplemented!("ftCo_800ADC28: a CPU copied neutral charge still needs a retail witness");
    }
    fp.character.table().cpu_profile.charge
}

/// ftCo_CpuTapR: press R a tick.
fn tap_r(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::PressR);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseR);
}

/// ftCo_800ADC28 (0x800ADC28): in a charge's loop, R stores it (Donkey
/// Kong's own punch without the tick's wait after), unless the CPU just
/// arrived (behaviour 10).
pub fn release(fp: &mut Fighter) {
    if fp.core.cpu.behavior == crate::select::behavior::ARRIVED {
        return;
    }
    let Some(charged) = charged(fp) else {
        return;
    };
    if !charged.loops.contains(&fp.core.motion_state.action.0) {
        return;
    }
    tap_r(fp);
    if charged.release_wait {
        script::command1(&mut fp.core.cpu, C::WaitFor, 1);
    }
}

/// ftCo_800ACD5C's tail at level 5 and up: start charging the special
/// when nothing is stored and it is not already charging; Mewtwo stores a
/// full Shadow Ball he is still holding. True when it wrote the script.
pub fn charge_on_arrival(fp: &mut Fighter) -> bool {
    let Some(charged) = charged(fp) else {
        return false;
    };
    let motion = fp.core.motion_state.action.0;
    let charge = fp.character.neutral_charge();
    if charged.store_full {
        // 800AD148/800AD334: (float) stored == full, fcmpu.
        if charge.stored as f32 == charge.full {
            if charged.loops.contains(&motion) {
                tap_r(fp);
                script::command(&mut fp.core.cpu, C::Done);
                return true;
            }
            return false;
        }
    }
    // The first loop row only: a full Shadow Ball's loop does not count.
    if motion == charged.loops[0] || charge.stored != 0 {
        return false;
    }
    // ftCo_CpuStartCharging: neutral stick, tap B, Done.
    let cpu = &mut fp.core.cpu;
    script::neutral_stick(cpu);
    script::command(cpu, C::PressB);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseB);
    script::command(cpu, C::Done);
    true
}
