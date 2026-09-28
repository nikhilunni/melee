//! Behaviour 16 (ftCo_800AC30C): mash out of a grab.
use crate::script::{self, Command as C};
use melee_ft::fighter::Fighter;

/// Motions a captured fighter mashes in: CaptureWaitHi (0xE0),
/// CaptureWaitLw (0xE3) and 0x10A..=0x10E (TODO(meaning): the special
/// captures' waits).
fn held(motion: u16) -> bool {
    matches!(motion, 0xE0 | 0xE3 | 0x10A..=0x10E)
}

/// ftCo_800AC30C (0x800AC30C): every third tick, unless
/// 0.1 * level < Randf() (fsubs, fmuls), flip the stick's x to full.
pub fn mash(fp: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    let cpu = &mut fp.core.cpu;
    if !held(fp.core.motion_state.action.0) {
        script::return_to_previous(cpu);
        return;
    }
    if cpu.reaction_timer % 3 != 0 {
        return;
    }
    let threshold = 0.1f32 * cpu.level as f32;
    if threshold < rng.randf() {
        return;
    }
    let x: i8 = if cpu.stick[0] < 0 { 0x7F } else { -0x7F };
    script::command1(cpu, C::SetLstickX, x as u8);
    script::command(cpu, C::Done);
}
