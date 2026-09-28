//! ftcpuattack.c: whether and how the CPU attacks its target.
use crate::world::Scene;
use gekko_math::HsdRng;
use melee_ft::fighter::Fighter;
use melee_types::CommonMotionState as S;

/// ftCo_800B9CBC (0x800B9CBC): a ranged attack now (x18 3).
pub fn ranged_attack(fp: &mut Fighter, _scene: &mut Scene, _rng: &mut HsdRng) -> bool {
    let cpu = &mut fp.core.cpu;
    cpu.xa4 = 0;
    if cpu.target.is_none() || !cpu.xf9_b5 {
        return false;
    }
    unimplemented!("ftCo_800B9CBC: ranged attack choice (ftcpuattack.c:2286)");
}

/// ftCo_800B885C (0x800B885C): by level, whether the CPU holds back from
/// attacking during this part of its x80 cycle.
fn holding_back(fp: &Fighter) -> bool {
    let cpu = &fp.core.cpu;
    match cpu.level {
        0 => cpu.x80 % 300 <= 240,
        1 => cpu.x80 % 300 <= 180,
        2 => cpu.x80 % 300 <= 120,
        3 => cpu.x80 % 240 <= 120,
        4 => cpu.x80 % 240 <= 60,
        _ => false,
    }
}

/// ftCo_800B8A9C (0x800B8A9C): a close attack now (x18 2).
pub fn close_attack(fp: &mut Fighter, scene: &mut Scene, _rng: &mut HsdRng) -> bool {
    if !fp.core.cpu.xf9_b2 {
        return false;
    }
    fp.core.cpu.attack_queue_len = 0;
    fp.core.cpu.defend_queue_len = 0;
    let target = fp.core.cpu.target.map(|index| scene.fighter(index));
    let unavailable =
        target.is_none_or(|target| target.core.out_of_play() || target.core.status.disabled);
    if unavailable || fp.core.motion_state.action.0 == S::Pass as u16 || holding_back(fp) {
        fp.core.cpu.xa4 = 0;
        return false;
    }
    unimplemented!("ftCo_800B8A9C: close attack choice (ftcpuattack.c:1770)");
}

/// ftCo_800B732C (0x800B732C): whether to guard the ledge the target
/// hangs from (1) or jump at it (2); 0 otherwise.
pub fn edge_guard_style(fp: &mut Fighter, scene: &mut Scene, _rng: &mut HsdRng) -> u8 {
    let Some(target) = fp.core.cpu.target.map(|index| scene.fighter(index)) else {
        return 0;
    };
    if fp.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
        return 0;
    }
    let target_motion = target.core.motion_state.action.0;
    if target_motion == S::CliffCatch as u16 || target_motion == S::CliffWait as u16 {
        unimplemented!("ftCo_800B732C: a target on the ledge");
    }
    if !fp.core.cpu.xf9_b3 {
        return 0;
    }
    unimplemented!("ftCo_800B732C: edge guarding (ftcpuattack.c:1278)");
}
