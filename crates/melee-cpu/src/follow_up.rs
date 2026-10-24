//! ftCo_800B658C's per-kind follow-ups: a special already under way is
//! steered or released before the CPU picks another attack.
use crate::{
    script::{self, Command as C},
    world::Scene,
};
use hsd_types::Vec3;
use melee_ft::fighter::cpu_profile::FollowUpAction;
use melee_ft::fighter::Fighter;

/// PK Flash is released within this height of the target (a double).
const FLASH_RELEASE_HEIGHT: f64 = 25.0;
/// The Egg Roll turns back within this of its island's end (a double).
const EGG_ROLL_EDGE: f64 = 10.0;
/// mpIsland_8005AC14's probe depth for the Egg Roll's islands.
const ISLAND_PROBE_DEPTH: f32 = -100.0;
/// SetLstickX's byte for straight left (-0x7F).
const STICK_LEFT: u8 = 0x81;

/// What a follow-up leaves for ftCo_800B658C.
pub enum FollowUp {
    /// The follow-up wrote the script; the attack choice is done.
    Done,
    /// Go on choosing an attack (Samus's and Donkey Kong's charge release
    /// writes its taps and falls through).
    Continue,
}

/// ftCo_800B658C's kind switch (800B6648..800B677C).
pub fn follow_up(fp: &mut Fighter, scene: &mut Scene) -> FollowUp {
    apply(fp, scene, false)
}

/// Shared strategy selection; ranged continuations exclude the shield taps.
pub(crate) fn ranged_follow_up(fp: &mut Fighter, scene: &mut Scene) -> bool {
    matches!(apply(fp, scene, true), FollowUp::Done)
}
fn apply(fp: &mut Fighter, scene: &mut Scene, ranged: bool) -> FollowUp {
    let motion = fp.core.motion_state.action.0;
    let profile = fp.character.table().cpu_profile;
    let Some(rule) = profile
        .follow_ups
        .iter()
        .find(|r| (!ranged || r.ranged) && r.motions.contains(motion))
    else {
        return FollowUp::Continue;
    };
    match rule.action {
        FollowUpAction::ReleaseAtTargetHeight => release_pk_flash(fp, scene),
        FollowUpAction::SteerStickTowardTarget => steer_pk_thunder(fp),
        FollowUpAction::RollAcrossSharedIsland => steer_egg_roll(fp, scene),
        FollowUpAction::SteerOwnedProjectile => steer_dins_fire(fp, scene),
        FollowUpAction::TapShield => {
            tap_r_and_wait(fp);
            return FollowUp::Continue;
        }
    }
    FollowUp::Done
}

/// Din's Fire releases within this of its target, x and y (doubles).
const FIRE_REACH: f64 = 15.0;
/// ...aiming this far above the target's position.
const FIRE_AIM_HEIGHT: f32 = 15.0;

/// ftCo_800B9504 (0x800B9504): steer Din's Fire (u.zd.x222C, its JObj's
/// translation) at the opponent nearest it (ftCo_800A4E8C): release B once
/// it passes the opponent or is within 15 of 15 above it; else up or down
/// toward that point.
pub(crate) fn steer_dins_fire(fp: &mut Fighter, scene: &Scene) {
    let fire = scene.items.iter().find(|item| {
        item.kind == melee_types::ItemKind::ZeldaDinFire && item.owner == Some(fp.core.player.id)
    });
    let Some(fire) = fire.map(|item| item.position) else {
        // ftCo_800A0C8C.
        script::command(&mut fp.core.cpu, C::Done);
        return;
    };
    let Some(target) = crate::targets::nearest_opponent_to(fp, scene, fire) else {
        let cpu = &mut fp.core.cpu;
        cpu.target = None;
        cpu.behavior = cpu.home_behavior;
        script::command(cpu, C::Done);
        return;
    };
    let theirs = scene.fighter(target).core.physics.position;
    let x = fp.core.physics.position.x;
    let to_target = gekko_math::msl::fabsf(theirs.x - x);
    let to_fire = gekko_math::msl::fabsf(fire.x - x);
    let release = to_fire > to_target || {
        // 800B962C..800B965C: |fire.x - x| (fsubs) and |15.0 + y - fire.y|
        // (fadd, fsub in double) against 15.0.
        let across = f64::from(gekko_math::msl::fabsf(fire.x - theirs.x));
        let height = FIRE_REACH + f64::from(theirs.y) - f64::from(fire.y);
        let height = if height < 0.0 { -height } else { height };
        across < FIRE_REACH && height < FIRE_REACH
    };
    let cpu = &mut fp.core.cpu;
    if release {
        script::command(cpu, C::ReleaseB);
        script::neutral_stick(cpu);
        script::command1(cpu, C::WaitFor, 1);
    } else if FIRE_AIM_HEIGHT + theirs.y > fire.y {
        // 800B96B0: fadds.
        script::command1(cpu, C::SetLstickY, 0x7F);
    } else {
        script::command1(cpu, C::SetLstickY, 0x81);
    }
    script::command(cpu, C::Done);
}

/// ftCo_CpuTapRAndWaitFiveFrames: R for a tick, then five ticks released.
fn tap_r_and_wait(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::PressR);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseR);
    script::command1(cpu, C::WaitFor, 5);
}

/// ftCo_CpuClearTargetAndFinish: x44 = NULL, x18 = x1C, Done.
fn clear_target_and_finish(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    cpu.target = None;
    cpu.behavior = cpu.home_behavior;
    script::command(cpu, C::Done);
}

/// ftCo_800B920C (0x800B920C): release PK Flash once level with the target
/// (|dy| an fsubs, compared in double), else steer it toward the target.
pub(crate) fn release_pk_flash(fp: &mut Fighter, scene: &Scene) {
    let Some(target) = fp.core.cpu.target else {
        clear_target_and_finish(fp);
        return;
    };
    let mut dy = scene.fighter(target).core.physics.position.y - fp.core.physics.position.y;
    if dy < 0.0 {
        dy = -dy;
    }
    if f64::from(dy) < FLASH_RELEASE_HEIGHT {
        script::command(&mut fp.core.cpu, C::ReleaseB);
        script::command(&mut fp.core.cpu, C::Done);
        clear_target_and_finish(fp);
    } else {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::LstickTowardFighter, 0x7F);
        script::command(cpu, C::Done);
    }
}

/// ftCo_800B92D4 (0x800B92D4): steer PK Thunder at the target.
pub(crate) fn steer_pk_thunder(fp: &mut Fighter) {
    if fp.core.cpu.target.is_none() {
        clear_target_and_finish(fp);
        return;
    }
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::LstickTowardFighter, 0x7F);
    script::command(cpu, C::Done);
}

/// ftCo_800B9340 (0x800B9340): roll the egg at the target while both are
/// over one island (mpIsland_8005AC14 100 below), turning back near its
/// ends; otherwise drop the target and hop out of it.
pub(crate) fn steer_egg_roll(fp: &mut Fighter, scene: &mut Scene) {
    let own = scene
        .map
        .island_below(fp.core.physics.position, ISLAND_PROBE_DEPTH);
    let target_island = fp.core.cpu.target.map(|index| {
        let position: Vec3 = scene.fighter(index).core.physics.position;
        scene.map.island_below(position, ISLAND_PROBE_DEPTH)
    });
    let shared = match (own, target_island) {
        (Some(own), Some(Some(theirs))) if own == theirs => Some(own),
        _ => None,
    };
    let Some(node) = shared else {
        let cpu = &mut fp.core.cpu;
        cpu.target = None;
        cpu.behavior = cpu.home_behavior;
        script::command(cpu, C::ReleaseB);
        script::command1(cpu, C::WaitFor, 1);
        script::command(cpu, C::PressB);
        script::command1(cpu, C::WaitFor, 5);
        script::command(cpu, C::ReleaseB);
        script::command1(cpu, C::WaitFor, 1);
        script::command(cpu, C::Done);
        return;
    };
    let island = scene.map.island(node);
    let x = fp.core.physics.position.x;
    let near = |end: f32| {
        // 800B9418: fsubs, its magnitude compared in double.
        let mut d = end - x;
        if d < 0.0 {
            d = -d;
        }
        f64::from(d) < EGG_ROLL_EDGE
    };
    let (right, left) = (island.right.x, island.left.x);
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, 0);
    if near(right) {
        script::command1(cpu, C::SetLstickX, STICK_LEFT);
    } else if near(left) {
        script::command1(cpu, C::SetLstickX, 0x7F);
    } else {
        script::command1(cpu, C::LstickXTowardFighter, 0x7F);
    }
    script::command(cpu, C::Done);
}
