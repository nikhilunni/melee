//! Behaviour 9 (ftCo_800B683C, ftcpuattack.c:1011): what to do with a
//! grabbed opponent: pummel, walk it toward the nearer edge, or throw.
use crate::{
    script::{self, Command as C},
    world::Scene,
};
use gekko_math::HsdRng;
use melee_ft::fighter::cpu_profile::Finisher;
use melee_ft::fighter::{grab::GrabLink, Fighter};

/// PlCo stored script 2: a pummel.
const SCRIPT_PUMMEL: usize = 2;
/// PlCo stored script 0x11: Bowser's Koopa Klaw pummel.
const SCRIPT_KLAW_PUMMEL: usize = 0x11;

/// ftCo_800B683C (0x800B683C).
pub fn hold(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    let Some(GrabLink::Holding { victim, .. }) = fp.core.combat.grab else {
        // ftCo_CpuClearTargetAndFinish.
        let cpu = &mut fp.core.cpu;
        cpu.target = None;
        cpu.behavior = cpu.home_behavior;
        script::command(cpu, C::Done);
        return;
    };
    let profile = fp.character.table().cpu_profile;
    let motion = fp.core.motion_state.action.0;
    if profile
        .cargo_motions
        .is_some_and(|rows| rows.contains(motion))
    {
        carry_to_edge(fp, scene);
        return;
    }
    if profile
        .mouthful_motions
        .is_some_and(|rows| rows.contains(motion))
    {
        unimplemented!("ftCo_800B683C: Kirby's mouthful (ftcpuattack.c:1073)");
    }
    let klaw = profile.klaw_motion == Some(motion);
    if !klaw && !fp.core.capabilities.cpu_partner {
        if let Some(finisher) = profile.finisher {
            if finish(fp, scene, rng, &finisher, victim) {
                return;
            }
        }
    }
    if !crate::select::grabbing(fp.core.motion_state.action.0) {
        script::command(&mut fp.core.cpu, C::Done);
        return;
    }
    // 800B6DE8..800B6E1C: Randf() * (1.0 + x94) in double, rounded.
    let roll = (f64::from(rng.randf()) * (1.0 + f64::from(fp.core.cpu.x94))) as f32;
    if roll < 0.5 {
        let id = if klaw {
            SCRIPT_KLAW_PUMMEL
        } else {
            SCRIPT_PUMMEL
        };
        script::stored(&mut fp.core.cpu, scene.data, id);
        fp.core.cpu.x94 += 1;
        return;
    }
    if walk_to_edge(fp, scene) {
        return;
    }
    throw(fp, rng);
}

/// The kind's scripted throw: 800B6C40 and siblings, 0.05 * level as
/// fsubs, fmuls against Randf().
fn finish(
    fp: &mut Fighter,
    scene: &mut Scene,
    rng: &mut HsdRng,
    finisher: &Finisher,
    victim: u32,
) -> bool {
    let cpu = &fp.core.cpu;
    let level_ok = if finisher.high_level {
        cpu.level > 4
    } else {
        cpu.level < 5
    };
    if cpu.x94 != 0 || !level_ok {
        return false;
    }
    let roll = rng.randf();
    if roll >= 0.05f32 * cpu.level as f32 {
        return false;
    }
    // The held fighter, by its spawn number (GrabLink's victim).
    let victim_percent = scene
        .others()
        .find(|(_, f)| f.core.spawn_number == victim)
        .map(|(_, f)| f.core.physics.percent)
        .expect("the held fighter");
    let script = match finisher.at_high_percent {
        Some(high) if victim_percent >= 100.0 => high,
        _ => finisher.script,
    };
    script::stored(&mut fp.core.cpu, scene.data, script);
    true
}

/// Within 30 of the island's nearer end (800B6E5C..800B6FF0): flick the
/// stick toward it. mpIsland_8005AC14 looks 10 below the fighter.
fn walk_to_edge(fp: &mut Fighter, scene: &mut Scene) -> bool {
    let position = fp.core.physics.position;
    let Some(island) = scene.map.island_below(position, -10.0) else {
        return false;
    };
    let (left, right) = scene.map.island_ends(island);
    let to_right = abs(position.x - right.x);
    let to_left = abs(position.x - left.x);
    let (half, full) = if to_right > to_left {
        if f64::from(to_left) >= 30.0 {
            return false;
        }
        (-0x50i8, -0x7Fi8)
    } else {
        if f64::from(to_right) >= 30.0 {
            return false;
        }
        (0x50, 0x7F)
    };
    // ftCo_CpuFlickLstickX.
    let cpu = &mut fp.core.cpu;
    script::neutral_stick(cpu);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickX, half as u8);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickX, full as u8);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickX, 0);
    script::command(cpu, C::Done);
    true
}

/// Donkey Kong's cargo walk (motions 0x15F..0x164): toward the nearer end
/// of the island 10 below him, throwing (A with the stick that way) within
/// 10 of it (double compares).
fn carry_to_edge(fp: &mut Fighter, scene: &mut Scene) {
    let position = fp.core.physics.position;
    let Some(island) = scene.map.island_below(position, -10.0) else {
        script::command(&mut fp.core.cpu, C::Done);
        return;
    };
    let (left, right) = scene.map.island_ends(island);
    let to_right = abs(position.x - right.x);
    let to_left = abs(position.x - left.x);
    let (distance, stick) = if to_right < to_left {
        (to_right, 0x7F_u8)
    } else {
        (to_left, 0x81)
    };
    let cpu = &mut fp.core.cpu;
    if f64::from(distance) > 10.0 {
        script::command1(cpu, C::SetLstickY, 0);
        script::command1(cpu, C::SetLstickX, stick);
        script::command(cpu, C::Done);
        return;
    }
    // ftCo_CpuPressAWithLstickX.
    script::command1(cpu, C::SetLstickY, 0);
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickY, 0);
    script::command1(cpu, C::SetLstickX, stick);
    script::command(cpu, C::PressA);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::SetLstickY, 0);
    script::command(cpu, C::ReleaseA);
    script::command(cpu, C::Done);
}

/// A throw in one of four directions by a Randf() quarter (double
/// compares): tilt the stick halfway for a tick, then fully.
fn throw(fp: &mut Fighter, rng: &mut HsdRng) {
    let roll = f64::from(rng.randf());
    let (command, half, full) = if roll < 0.25 {
        (C::SetLstickY, 0x50u8, 0x7Fu8)
    } else if roll < 0.5 {
        (C::SetLstickY, 0xB0, 0x81)
    } else if roll < 0.75 {
        (C::LstickXForward, 0x50, 0x7F)
    } else {
        (C::LstickXForward, 0xB0, 0x81)
    };
    let cpu = &mut fp.core.cpu;
    script::neutral_stick(cpu);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, command, half);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, command, full);
    script::command1(cpu, C::WaitFor, 1);
    script::neutral_stick(cpu);
    script::command(cpu, C::Done);
}

/// ABS as fsubs, fcmpo against 0, fneg.
fn abs(value: f32) -> f32 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}
