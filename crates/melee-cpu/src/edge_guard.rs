//! Behaviour 8, the edge guard (ftcpuattack.c): ftCo_800B732C chooses it
//! for a target on a ledge above the CPU, ftCo_800B7180 runs it.
use crate::{
    script::{self, command, command1, Command as C},
    world::Scene,
};
use melee_ft::fighter::Fighter;
use melee_types::GroundOrAir;

/// ftCo_800B7180 (0x800B7180), behaviour 8: jump at the target on the
/// ledge (xF8 bits 3-4 = 1) or toward it (2), then (3) drift toward it
/// while rising, jump again while it is higher, and return to the previous
/// behaviour once grounded or level with it.
pub fn run(fp: &mut Fighter, scene: &mut Scene) {
    let target_height = fp
        .core
        .cpu
        .target
        .map(|index| scene.fighter(index).core.physics.position.y);
    if target_height.is_none() {
        // 800B71A4..800B71CC: no return follows, so the switch below runs
        // on the cleared bits and ends the script a second time.
        leave_edge_guard(fp);
    }
    match fp.core.cpu.xf8_b34 {
        1 => {
            jump_at_target(fp, scene);
            fp.core.cpu.xf8_b34 = 3;
        }
        2 => {
            jump_toward_target(fp);
            fp.core.cpu.xf8_b34 = 3;
        }
        3 if fp.core.physics.ground_or_air == GroundOrAir::Air => {
            let target_height = target_height.expect("xF8 bits 3-4 set without a target");
            // 800B7240..800B724C: pos_delta.y against a double zero.
            if f64::from(fp.core.physics.position_delta.y) > 0.0 {
                let cpu = &mut fp.core.cpu;
                command1(cpu, C::SetLstickY, 0);
                command1(cpu, C::LstickXTowardFighter, 0x7F);
                command(cpu, C::Done);
            } else if target_height > fp.core.physics.position.y {
                // 800B7280..800B728C: fcmpo, ble.
                jump_at_target(fp, scene);
            } else {
                leave_edge_guard(fp);
            }
        }
        _ => leave_edge_guard(fp),
    }
}

/// x18 = x1C, xF8 bits 3-4 cleared, Done.
fn leave_edge_guard(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    cpu.behavior = cpu.home_behavior;
    cpu.xf8_b34 = 0;
    command(cpu, C::Done);
}

/// ftCo_800A05F4 (0x800A05F4): a full jump at the target, unless an aerial
/// jump's height (x558) from here passes the top blast line (800A060C..
/// 800A061C: fadds, fcmpo, ble). ftCo_800A1CA8's arm is the hammer
/// (x2168), which only its item sets.
fn jump_at_target(fp: &mut Fighter, scene: &Scene) {
    let reach = fp.core.physics.position.y + fp.core.cpu.jump_height;
    let cpu = &mut fp.core.cpu;
    if reach > scene.arena.top {
        script::finish_with_neutral_stick(cpu);
        return;
    }
    script::neutral_stick(cpu);
    command(cpu, C::ReleaseY);
    command1(cpu, C::WaitFor, 1);
    command1(cpu, C::LstickTowardFighter, 0x7F);
    command(cpu, C::PressY);
    command1(cpu, C::WaitFor, 10);
    command(cpu, C::ReleaseY);
    command1(cpu, C::WaitFor, 1);
    command(cpu, C::Done);
}

/// ftCo_800A0798 (0x800A0798): a short hop toward the target's x. The
/// hammer arm (ftCo_800A1CA8) needs its item.
fn jump_toward_target(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::neutral_stick(cpu);
    command(cpu, C::ReleaseY);
    command1(cpu, C::WaitFor, 1);
    command1(cpu, C::LstickXTowardFighter, 0x7F);
    command(cpu, C::PressY);
    command1(cpu, C::WaitFor, 1);
    command(cpu, C::ReleaseY);
    command1(cpu, C::WaitFor, 5);
    command(cpu, C::Done);
}
