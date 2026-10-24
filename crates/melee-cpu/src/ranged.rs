//! ftcpuattack.c: the ranged attack (CpuFighter.x18 = 3). ftCo_800B9CBC
//! decides whether to attack from afar and which PlCo script to run;
//! ftCo_800B9020, behaviour 3, runs it and steers the specials that need
//! steering (Ness's PK Flash and PK Thunder, Yoshi's Egg Roll).
use crate::{
    attack::facing_target,
    desc::AttackEntry,
    script::{self, Command as C},
    world::{distance, Scene},
};
use gekko_math::HsdRng;
use melee_ft::fighter::cpu_profile::Ranged;
use melee_ft::fighter::{cpu::CpuState, Fighter};
use melee_types::GroundOrAir;

/// lb_8000D008 angles (radians, the double literals of ftcpuattack.c).
mod angle {
    /// 5 degrees: "level" with the target.
    pub const LEVEL: f64 = 0.08726646192371845;
    /// 15 degrees: level enough to throw an item.
    pub const LEVEL_15: f64 = 0.26179938577115536;
    /// 40 and 50 degrees below (Ness's air PK Thunder).
    pub const NESS_AIR_HIGH: f64 = -0.6981316953897476;
    pub const NESS_AIR_LOW: f64 = -0.8726646192371845;
    /// 45, 60 and 75 degrees.
    pub const STEEP_45: f64 = 0.7853981573134661;
    pub const STEEP_60: f64 = 1.0471975430846214;
    pub const STEEP_75: f64 = 1.3089969288557768;
}

/// PlCo scripts the per-kind choices name (ftCo_AttackEntry.cmd).
mod scripts {
    /// The kind's neutral special, straight ahead.
    pub const SPECIAL_N: i32 = 0x1B;
    /// Ness's PK Thunder upward, and his PK Fire.
    pub const NESS_SPECIAL_HI: i32 = 0x14;
    pub const NESS_SPECIAL_LW: i32 = 0x1F;
    /// Yoshi's Egg Throw at 45-60, 60-75 and 75-90 degrees.
    pub const YOSHI_EGG_45: i32 = 0x31;
    pub const YOSHI_EGG_60: i32 = 0x32;
    pub const YOSHI_EGG_75: i32 = 0x33;
}

/// Below this distance the CPU does not attack from afar (a double).
const MIN_RANGE: f64 = 30.0;
/// The chance (Randf > 0.75, a double compare) of passing up the attack.
const SKIP_CHANCE: f64 = 0.75;

/// ftCo_800B9CBC (0x800B9CBC): whether to attack the target from afar; the
/// script goes in xA4 (zero with a true result only for Ness's ground
/// branch, which then does nothing).
pub fn ranged_attack(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) -> bool {
    let cpu = &mut fp.core.cpu;
    cpu.xa4 = 0;
    let Some(target_index) = cpu.target else {
        return false;
    };
    if !cpu.xf9_b5 {
        return false;
    }
    if cpu.attack_delay > 0 {
        cpu.attack_delay -= 1;
        return false;
    }
    // ftCo_800B9704, inlined (800B9D30..800B9D90).
    cpu.attack_delay = CpuState::roll_attack_delay(cpu.mode, cpu.level, rng);
    if wall_between(fp, scene, target_index) {
        return false;
    }
    let target = scene.fighter(target_index);
    let range = distance(fp.core.physics.position, target.core.physics.position);
    // 800B9DC8..800B9DE0: |dx| by fsubs and fneg, dy by fsubs.
    let mut across = target.core.physics.position.x - fp.core.physics.position.x;
    if across < 0.0 {
        across = -across;
    }
    let bearing = melee_lb::trigf::stick_angle(
        target.core.physics.position.y - fp.core.physics.position.y,
        across,
    );
    if crate::facts::has_item(fp) {
        return with_item(fp, scene, bearing, range);
    }
    if f64::from(rng.randf()) > SKIP_CHANCE {
        return false;
    }
    if f64::from(range) < MIN_RANGE {
        return false;
    }
    let chosen = match fp.character.table().cpu_profile.ranged {
        Ranged::RemoteProjectile => choose_remote_projectile(fp, scene, rng, bearing, range),
        Ranged::RollingAttack => choose_rolling_attack(fp, scene, bearing, range),
        Ranged::DownwardProjectile(script) => {
            if fp.core.physics.ground_or_air == GroundOrAir::Ground {
                false
            } else if f64::from(bearing) < -angle::STEEP_45 {
                fp.core.cpu.xa4 = script;
                true
            } else {
                false
            }
        }
        Ranged::None => false,
    };
    if chosen {
        return true;
    }
    if !facing_target(fp, scene) {
        return false;
    }
    if !level(bearing) {
        return false;
    }
    let kind = usize::from(fp.core.kind as u8);
    let Some(list) = scene.data.ranged[kind].as_deref() else {
        unimplemented!(
            "ftCo_800B6208: {:?} has no ranged table (a null list)",
            fp.core.kind
        );
    };
    fp.core.cpu.xa4 = weighted_pick(list, rng);
    fp.core.cpu.xa4 != 0
}

/// Within 5 degrees of level, both double compares strict.
fn level(bearing: f32) -> bool {
    f64::from(bearing) < angle::LEVEL && f64::from(bearing) > -angle::LEVEL
}

/// ftCo_800A1BA8 (0x800A1BA8): a wall between the CPU and its target, on
/// the side it faces (mpCheckRightWall facing right, a double compare).
fn wall_between(fp: &Fighter, scene: &mut Scene, target_index: usize) -> bool {
    let from = fp.core.physics.position;
    let to = scene.fighter(target_index).core.physics.position;
    if f64::from(fp.core.physics.facing) > 0.0 {
        scene
            .map
            .check_right_wall(from.x, from.y, to.x, to.y, -1, -1)
            .is_some()
    } else {
        scene
            .map
            .check_left_wall(from.x, from.y, to.x, to.y, -1, -1)
            .is_some()
    }
}

/// ftCo_800B6208 (0x800B6208): one of `list`'s scripts at random by weight
/// (Randf drawn first; 1.0 / sum in double, rounded; fadds and fmuls per
/// entry, taken at the first `>=`); 0 for an empty list or zero weight.
fn weighted_pick(list: &[AttackEntry], rng: &mut HsdRng) -> i32 {
    let roll = rng.randf();
    let mut sum = 0.0f32;
    for entry in list {
        sum += entry.weight;
    }
    if list.is_empty() {
        return 0;
    }
    if sum < 0.00001 && sum > -0.00001 {
        return 0;
    }
    let inverse = (1.0 / f64::from(sum)) as f32;
    let mut accumulated = 0.0f32;
    for entry in list {
        accumulated += entry.weight;
        if accumulated * inverse >= roll {
            return entry.script;
        }
    }
    panic!("ftcpuattack.c:650: no ranged attack chosen");
}

/// ftCo_800B9790 (0x800B9790): Ness. In the air, PK Thunder at a target 40
/// to 50 degrees below within 53; on the ground, the neutral special at a
/// level target within 53, else, at a higher target, PK Thunder above 60
/// degrees or PK Fire one time in ten; with neither it still answers yes,
/// with no script.
fn choose_remote_projectile(
    fp: &mut Fighter,
    scene: &Scene,
    rng: &mut HsdRng,
    bearing: f32,
    range: f32,
) -> bool {
    let bearing = f64::from(bearing);
    let near = f64::from(range) < 53.0;
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        if bearing < angle::NESS_AIR_HIGH
            && bearing > angle::NESS_AIR_LOW
            && near
            && facing_target(fp, scene)
        {
            fp.core.cpu.xa4 = scripts::SPECIAL_N;
            return true;
        }
        return false;
    }
    if bearing < angle::LEVEL && bearing > -angle::LEVEL && near && facing_target(fp, scene) {
        fp.core.cpu.xa4 = scripts::SPECIAL_N;
        return true;
    }
    let target = scene.fighter(fp.core.cpu.target.expect("the target"));
    // 800B9860: fcmpo, cror eq, gt: not below the target.
    if fp.core.physics.position.y >= target.core.physics.position.y {
        return false;
    }
    if bearing > angle::STEEP_60 {
        fp.core.cpu.xa4 = scripts::NESS_SPECIAL_HI;
    } else if f64::from(rng.randf()) < 0.1 {
        fp.core.cpu.xa4 = scripts::NESS_SPECIAL_LW;
    }
    true
}

/// ftCo_800B98C8 (0x800B98C8): Yoshi, grounded and facing the target within
/// 80: the neutral special at a level target on the same island, else an
/// Egg Throw by the target's height (45 to 90 degrees).
fn choose_rolling_attack(fp: &mut Fighter, scene: &Scene, bearing: f32, range: f32) -> bool {
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return false;
    }
    if !facing_target(fp, scene) {
        return false;
    }
    if f64::from(range) > 80.0 {
        return false;
    }
    let bearing_f64 = f64::from(bearing);
    if level(bearing) {
        let target = scene.fighter(fp.core.cpu.target.expect("the target"));
        if target.core.physics.ground_or_air == GroundOrAir::Air {
            return false;
        }
        if !crate::route::same_island(scene, fp, target) {
            return false;
        }
        fp.core.cpu.xa4 = scripts::SPECIAL_N;
        return true;
    }
    if bearing_f64 < angle::STEEP_45 {
        return false;
    }
    fp.core.cpu.xa4 = if bearing_f64 < angle::STEEP_60 {
        scripts::YOSHI_EGG_45
    } else if bearing_f64 < angle::STEEP_75 {
        scripts::YOSHI_EGG_60
    } else {
        scripts::YOSHI_EGG_75
    };
    true
}

/// ftCo_800B9020 (0x800B9020): behaviour 3's script.
pub fn ranged(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    if fp.core.cpu.target.is_none() {
        clear_target_and_finish(&mut fp.core.cpu);
        return;
    }
    if crate::follow_up::ranged_follow_up(fp, scene) {
        fp.core.cpu.x90 += 1;
        return;
    }
    if crate::attack::attacking(fp) {
        let cpu = &mut fp.core.cpu;
        script::neutral_stick(cpu);
        // ftCo_800A0C8C.
        script::command(cpu, C::Done);
        cpu.xa4 = 0;
    } else if fp.core.cpu.xa4 != 0 {
        run_chosen(fp, scene);
    } else if ranged_attack(fp, scene, rng) {
        if fp.core.cpu.xa4 != 0 {
            run_chosen(fp, scene);
            fp.core.cpu.x90 += 1;
        } else {
            clear_target_and_finish(&mut fp.core.cpu);
        }
    } else {
        clear_target_and_finish(&mut fp.core.cpu);
    }
}

/// ftCo_800B4880 with xA4, then xA4 = 0.
fn run_chosen(fp: &mut Fighter, scene: &Scene) {
    let id = usize::try_from(fp.core.cpu.xa4).expect("a stored script");
    script::stored(&mut fp.core.cpu, scene.data, id);
    fp.core.cpu.xa4 = 0;
}

/// ftCo_CpuClearTargetAndFinish: x44 = NULL, x18 = x1C, Done.
fn clear_target_and_finish(cpu: &mut CpuState) {
    cpu.target = None;
    cpu.behavior = cpu.home_behavior;
    script::command(cpu, C::Done);
}

/// Held items ftCo_800B9A04 throws by the target's bearing: capsules,
/// Bob-ombs, Mr. Saturn, shells, Freezies, mines, the flipper and Poké
/// Balls (ftCo_800A5A28).
fn thrown_at_angle(kind: melee_types::ItemKind) -> bool {
    use melee_types::ItemKind as I;
    matches!(
        kind,
        I::Capsule
            | I::BombHei
            | I::Dosei
            | I::GShell
            | I::RShell
            | I::Freeze
            | I::MSBomb
            | I::Flipper
            | I::MBall
    )
}

/// it_8026B960 (0x8026B960): the fuse of a held Link bomb (its lifetime,
/// unless already exploding, state 5); -1 for anything else.
fn fuse(item: &crate::world::ItemView) -> f32 {
    use melee_types::ItemKind as I;
    match item.kind {
        I::BombHei => unimplemented!("it_8026B960: a Bob-omb's fuse (xDEC)"),
        I::LinkBomb if item.motion != 5 => item.life_timer,
        _ => -1.0,
    }
}

/// ftCo_800B9A04 (0x800B9A04): the ranged attack with an item in hand:
/// none with a healing item or mid-swing; a Link bomb about to blow is
/// thrown at once (script 2), a container too; an item thrown by the
/// target's bearing (0xD up, 0xE down, 0xF level, a Poké Ball 0xE); a
/// shooter fires (0x28 empty, the Star Rod 0xF or else 2 level and beyond
/// 30, else 0x2A); anything else but a swing weapon is thrown level
/// beyond 30 (0x2A). The bounds are double compares.
fn with_item(fp: &mut Fighter, scene: &Scene, bearing: f32, range: f32) -> bool {
    use melee_types::ItemKind as I;
    // fp->item_gobj: a held item, or an article in hand (Peach's parasol).
    struct Held {
        kind: I,
        use_kind: u8,
        item: Option<u32>,
    }
    let held = match (&fp.core.held_item, &fp.core.article_in_hand) {
        (Some(held), _) => Held {
            kind: held.kind,
            use_kind: held.use_kind,
            item: Some(held.item),
        },
        (None, Some(article)) => Held {
            kind: article.kind,
            use_kind: article.use_kind,
            item: None,
        },
        (None, None) => unreachable!("ftCo_800B9A04 without fp->item_gobj"),
    };
    if matches!(held.kind, I::Heart | I::Tomato | I::Foods)
        || crate::attack::item_attacking(fp.core.motion_state.action.0)
    {
        return false;
    }
    let fuse = match held.item {
        Some(id) => {
            let item = scene
                .items
                .iter()
                .find(|item| item.id == id)
                .copied()
                .expect("the held item");
            fuse(&item)
        }
        None if matches!(held.kind, I::LinkBomb | I::BombHei) => {
            unreachable!("a bomb is a held item")
        }
        None => -1.0,
    };
    let fuse = f64::from(fuse);
    let bearing = f64::from(bearing);
    let range = f64::from(range);
    let set = |fp: &mut Fighter, script: i32| {
        fp.core.cpu.xa4 = script;
        true
    };
    // 800B9A70..800B9A84.
    if fuse < 60.0 && fuse > 0.0 {
        return set(fp, 2);
    }
    // ftCo_800A59C0: the item attributes' use kind 3, a shooter.
    if held.use_kind == 3 {
        if matches!(held.kind, I::SScope | I::StarRod | I::LGun | I::FFlower) {
            unimplemented!("it_8026B924: a shooter's ammunition (xD4C)");
        }
        // it_8026B924's -1 for the other kinds is not an empty one.
        if !facing_target(fp, scene) {
            return false;
        }
        if bearing < angle::LEVEL && bearing > -angle::LEVEL && range > MIN_RANGE {
            let script = if held.kind == I::StarRod { 0xF } else { 2 };
            return set(fp, script);
        }
        return set(fp, 0x2A);
    }
    if matches!(held.kind, I::Box | I::Taru | I::Kusudama | I::TaruCann) {
        return set(fp, 2);
    }
    if thrown_at_angle(held.kind) {
        if bearing > angle::STEEP_75 {
            return set(fp, 0xD);
        }
        if bearing < -angle::STEEP_75 {
            return set(fp, 0xE);
        }
        if !facing_target(fp, scene) {
            return false;
        }
        if bearing < angle::LEVEL_15 && bearing > -angle::LEVEL_15 {
            return set(fp, 0xF);
        }
        if held.kind == I::MBall {
            return set(fp, 0xE);
        }
        return false;
    }
    if matches!(
        held.kind,
        I::Harisen | I::StarRod | I::LipStick | I::Bat | I::Sword | I::Parasol
    ) {
        return false;
    }
    if range > MIN_RANGE && bearing < angle::LEVEL_15 && bearing > -angle::LEVEL_15 {
        return set(fp, 0x2A);
    }
    false
}
