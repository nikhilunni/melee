//! CPU opponent AI: behaviour state machine, attack selection, command
//! script interpreter (decomp ftCo_0A01.c, ftcpuattack.c, ftcmdscript.c).
//!
//! The scene calls [`think`] from the CPU proc (Fighter_8006ABA0) for each
//! fighter whose input the CPU supplies; the input proc then samples the
//! pad the command script set (`CpuState::pad_sample`). The CPU state
//! itself (`CpuFighter`) lives in melee-ft, which initializes it.
//!
//! Ported so far: the Ice Climbers' partner (CPU mode 6) and the parts of
//! the shared AI it reaches; every other branch fails closed with its
//! retail name.
//!
//! See CLAUDE.md for the porting rules that apply to every crate.
mod arrived;
mod attack;
mod awareness;
mod behave;
mod captured;
pub mod desc;
mod facts;
mod hold;
mod movement;
mod partner;
mod route;
pub mod script;
mod select;
mod targets;
mod world;

pub use world::{distance, ItemView, Scene};

use gekko_math::HsdRng;
use melee_ft::fighter::{cpu::CpuState, Fighter};

/// ftCo_800B3900 (0x800B3900): one tick of the CPU.
pub fn think(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    awareness::upkeep(fp, scene, rng);
    decide(fp, scene, rng);
    behave::write_script(fp, scene, rng);
    run_script(fp, scene);
    partner::replay(fp, scene);
    fp.core.cpu.reaction_timer += 1;
}

/// ftCo_800B2AFC (0x800B2AFC): the CPU mode's decision.
fn decide(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    match fp.core.cpu.mode {
        melee_ft::fighter::cpu::PARTNER_MODE => partner::decide(fp, scene, rng),
        mode => unimplemented!("ftCo_800B2AFC: CPU mode {mode}"),
    }
}

/// ftCo_800B3E04 with the fighter's state and its target's position.
fn run_script(fp: &mut Fighter, scene: &Scene) {
    let subject = script::Subject {
        position: fp.core.physics.position,
        facing: fp.core.physics.facing,
        motion: u32::from(fp.core.motion_state.action.0),
        target: fp
            .core
            .cpu
            .target
            .map(|index| scene.fighter(index).core.physics.position),
    };
    script::run(&mut fp.core.cpu, &subject);
}

/// ftCo_CpuShouldAct / ftCo_CpuDataShouldAct: leaving the home behaviours
/// drops a pending route waypoint (x60); the recovery behaviour (4) never
/// acts, any other clears xFA_b2 and does.
pub(crate) fn should_act(cpu: &mut CpuState) -> bool {
    if cpu.behavior != cpu.x20 && cpu.behavior != cpu.home_behavior {
        cpu.route_timer = 0;
    }
    if cpu.behavior == select::behavior::RECOVER {
        false
    } else {
        cpu.xfa_b2 = false;
        true
    }
}
