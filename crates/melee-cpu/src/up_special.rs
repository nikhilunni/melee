//! ftCo_800A96B8: the kind's recovery special toward the stage, the last
//! resort of a CPU below its destination with no jump left.
use crate::script::{self, Command as C};
use gekko_math::HsdRng;
use melee_ft::fighter::cpu_profile::Recovery;
use melee_ft::fighter::Fighter;

/// A full stick and ftCo_CpuRecoverDiagonally's diagonal.
const FULL: u8 = 0x7F;
const DIAGONAL: u8 = 0x58;

/// ftCo_800A96B8 (0x800A96B8). Fox's and Falco's Firefox and the
/// diagonal default are witnessed; the other kinds' arms (Pikachu and
/// Pichu ftCo_800A92CC, Yoshi none, Ness ftCo_800A8EB0, Luigi's Green
/// Missile, Zelda ftCo_800A949C, Samus ftCo_800A963C) are not yet.
pub(crate) fn up_special(fp: &mut Fighter, _rng: &mut HsdRng) {
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
        Recovery::Unwitnessed => {
            unimplemented!(
                "ftCo_800A96B8: {:?}'s recovery (ftCo_0A01.c:4422)",
                fp.core.kind
            )
        }
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
