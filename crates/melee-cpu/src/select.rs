//! ftCo_800ADE48: the CPU's choice of behaviour (CpuFighter.x18) from its
//! situation: recover, get up, climb, attack, pick up, or arrive.
use crate::{route, script, world::Scene};
use gekko_math::{
    fma::fmadds,
    msl::{fabsf, sqrtf},
    HsdRng,
};
use hsd_types::{Vec2, Vec3};
use melee_ft::fighter::Fighter;
use melee_types::{CommonMotionState as S, GroundOrAir};

/// A motion test for a behaviour the CPU is forced into.
type MotionTest = fn(&Fighter, u16) -> bool;

/// CpuFighter.x18 values ftCo_800ADE48 selects.
pub mod behavior {
    pub const STAND_BY: i32 = 0;
    pub const IDLE: i32 = 1;
    pub const ATTACK: i32 = 2;
    pub const RANGED: i32 = 3;
    pub const RECOVER: i32 = 4;
    pub const DOWN: i32 = 5;
    pub const LEDGE: i32 = 6;
    pub const DEFEND: i32 = 7;
    pub const EDGE_GUARD: i32 = 8;
    pub const HOLD: i32 = 9;
    pub const ARRIVED: i32 = 10;
    pub const PICK_UP: i32 = 13;
    pub const USE_ITEM: i32 = 14;
    pub const SHIELD_BROKEN: i32 = 15;
    pub const CAPTURED: i32 = 16;
    pub const BARREL: i32 = 17;
    pub const TUMBLE: i32 = 18;
    pub const WARP_STAR: i32 = 19;
}

/// ftCo_800ADE48 (0x800ADE48). True when it set (or kept) a behaviour.
pub fn choose_behavior(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) -> bool {
    validate_destination(fp, scene);
    note_destination_distance(fp);
    note_kills(fp, scene);
    roll_xf8_b5(fp, rng);
    if !crate::facts::holding_victim(fp) {
        fp.core.cpu.x94 = 0;
    }
    use behavior as B;
    let cpu = &fp.core.cpu;
    if cpu.behavior == B::TUMBLE {
        return true;
    }
    if tumble(fp, rng) {
        return switch(fp, B::TUMBLE);
    }
    let current = fp.core.cpu.behavior;
    let motion = fp.core.motion_state.action.0;
    let checks: [(i32, MotionTest); 4] = [
        (B::BARREL, |_, m| {
            m == S::BarrelWait as u16 || m == S::Barrel as u16
        }),
        (B::WARP_STAR, |_, m| {
            m == S::WarpStarJump as u16 || m == S::WarpStarFall as u16
        }),
        (B::DOWN, |_, m| matches!(m, 0xBF | 0xB7 | 0xC0 | 0xB8)),
        (B::LEDGE, |_, m| matches!(m, 0xFC | 0xFD)),
    ];
    for (target, test) in checks {
        if current == target {
            return true;
        }
        if test(fp, motion) {
            return switch(fp, target);
        }
    }
    if current == B::HOLD {
        return true;
    }
    if grabbing(motion) {
        return switch(fp, B::HOLD);
    }
    if fp.core.cpu.behavior == B::RECOVER {
        return true;
    }
    if recovery_needed(fp, scene) {
        return switch(fp, B::RECOVER);
    }
    let current = fp.core.cpu.behavior;
    if current == B::SHIELD_BROKEN {
        return true;
    }
    // Level 5 and up, in motion 0x26 (TODO(meaning): which state).
    if fp.core.cpu.level >= 5 && fp.core.motion_state.action.0 == 0x26 {
        return switch(fp, B::SHIELD_BROKEN);
    }
    if current == B::CAPTURED {
        return true;
    }
    let motion = fp.core.motion_state.action.0;
    if motion == 0xE0 || motion == 0xE3 || (0x10A..=0x10E).contains(&motion) {
        return switch(fp, B::CAPTURED);
    }
    if current == B::DEFEND {
        unimplemented!("ftCo_800BB9B4: the defence behaviour's check");
    }
    if ranged_ready(fp) {
        unimplemented!("ftCo_800BB9B4 (ftcpuattack.c:2891)");
    }
    if fp.core.cpu.behavior == B::RANGED {
        return true;
    }
    if crate::attack::ranged_attack(fp, scene, rng) {
        return switch(fp, B::RANGED);
    }
    if fp.core.cpu.behavior == B::ATTACK {
        return true;
    }
    if crate::attack::close_attack(fp, scene, rng) {
        return switch(fp, B::ATTACK);
    }
    if fp.core.cpu.behavior == B::EDGE_GUARD {
        return true;
    }
    let edge_guard = crate::attack::edge_guard_style(fp, scene, rng);
    fp.core.cpu.xf8_b34 = edge_guard;
    if edge_guard != 0 {
        fp.core.cpu.behavior = B::EDGE_GUARD;
        return true;
    }
    if fp.core.cpu.behavior == B::PICK_UP {
        return true;
    }
    if pick_up_ready(fp, scene) {
        fp.core.cpu.behavior = B::PICK_UP;
        return true;
    }
    if fp.core.cpu.behavior == B::USE_ITEM {
        return true;
    }
    if holding_usable_item(fp) {
        fp.core.cpu.behavior = B::USE_ITEM;
        return true;
    }
    if fp.core.cpu.behavior == B::ARRIVED {
        return true;
    }
    if fp.core.cpu.x20 != 0 && arrived(fp, scene, 0.0) {
        fp.core.cpu.behavior = fp.core.cpu.x20;
        return true;
    }
    false
}

/// `ftCo_800B4A78(fp); data->x18 = behavior; return true`.
fn switch(fp: &mut Fighter, behavior: i32) -> bool {
    script::clear(&mut fp.core.cpu);
    fp.core.cpu.behavior = behavior;
    true
}

/// ftCo_800ADE48's head: a destination without a floor under it (5 above
/// to 5 below, in bounds) falls back to the floor under an airborne
/// fighter, or to the fighter itself. That also clears xFA_b2.
fn validate_destination(fp: &mut Fighter, scene: &mut Scene) {
    let destination = fp.core.cpu.destination;
    // 800ADE9C..800ADEB4: y -/+ 5.0 in double, rounded.
    let below = (f64::from(destination.y) - 5.0) as f32;
    let above = (5.0 + f64::from(destination.y)) as f32;
    let floor = scene.check_usable_floor(destination.x, above, destination.x, below);
    if floor.is_some() && !scene.outside(destination.x, destination.y, fp.core.cpu.half_size) {
        return;
    }
    fp.core.cpu.xfa_b2 = false;
    let position = fp.core.physics.position;
    fp.core.cpu.destination = if fp.core.physics.ground_or_air == GroundOrAir::Air {
        // 800ADF90: fsubs y - 1000.0f.
        match scene.check_usable_floor(position.x, position.y, position.x, position.y - 1000.0) {
            Some(hit) => Vec2::new(hit.pos.x, hit.pos.y),
            None => Vec2::new(position.x, position.y),
        }
    } else {
        Vec2::new(position.x, position.y)
    };
}

/// x5C: the distance to the destination (800AE040: fmadds, inline sqrtf;
/// ftCo_800A49B4 out of line the same way).
pub(crate) fn note_destination_distance(fp: &mut Fighter) {
    let position = fp.core.physics.position;
    let destination = fp.core.cpu.destination;
    let dy = position.y - destination.y;
    let dx = position.x - destination.x;
    fp.core.cpu.x5c = sqrtf(fmadds(dx, dx, dy * dy));
}

/// x88 counts down; x8C follows the player's KO total (gm_8016C75C: the
/// match standings' x20), a new KO restarting x88 at 300.
fn note_kills(fp: &mut Fighter, scene: &Scene) {
    let cpu = &mut fp.core.cpu;
    if cpu.x88 > 0 {
        cpu.x88 -= 1;
    }
    let kills = scene.player_kills;
    if cpu.x8c != kills {
        cpu.x88 = 300;
        cpu.x8c = kills;
    }
}

/// xF8_b5: rerolled every 120 ticks for CPU mode 14, every 400 otherwise
/// (Randf() > 0.5, a double compare).
fn roll_xf8_b5(fp: &mut Fighter, rng: &mut HsdRng) {
    let cpu = &mut fp.core.cpu;
    let period = if cpu.mode == 14 { 120 } else { 400 };
    if cpu.reaction_timer % period == 0 {
        cpu.xf8_b5 = f64::from(rng.randf()) > 0.5;
    }
}

/// The tumble branch (x18 0x12): only for a fighter hit this frame whose
/// mode is neither 0 nor 15 (Giga Bowser never). It clears xF9_b0 and
/// rolls xFA_b1 = 0.1 * level > Randf() (800AE240: fsubs, fmuls).
///
/// On this path retail never assigns the switch flag: 800AE270 tests r31,
/// which 800ADE48 saved but never set, so it holds the caller's r31. The
/// only caller the port reaches, ftCo_800B0760 (the partner's decision),
/// keeps `fp + 0x1A88` there (800B0778), so the switch always happens.
fn tumble(fp: &mut Fighter, rng: &mut HsdRng) -> bool {
    let cpu = &fp.core.cpu;
    if fp.core.kind == melee_types::FighterKind::GKoops
        || cpu.mode == 15
        || cpu.mode == 0
        || !crate::facts::in_damage_hitlag(fp)
    {
        return false;
    }
    let cpu = &mut fp.core.cpu;
    cpu.target_locked = false;
    let threshold = 0.1f32 * cpu.level as f32;
    cpu.xfa_b1 = threshold > rng.randf();
    true
}

/// ftCo_IsGrabbing (0x800A31A4) for the ported kinds: CatchWait.
pub(crate) fn grabbing(motion: u16) -> bool {
    motion == S::CatchWait as u16
}

/// ftCo_800A2C80 (0x800A2C80): off stage and falling toward no floor in
/// bounds, nor a wall in bounds.
pub fn recovery_needed(fp: &Fighter, scene: &mut Scene) -> bool {
    let cpu = &fp.core.cpu;
    if cpu.xfa_b6 {
        return false;
    }
    if fp.core.physics.ground_or_air == GroundOrAir::Ground {
        return scene.ignored_floor(fp.core.collision.data.floor.index);
    }
    if fp.core.motion_state.action.0 == 0xF4 {
        return false;
    }
    let delta = fp.core.physics.position_delta;
    // 800A2D68..800A2D70: fmuls, fmuls, fadds.
    let magnitude = delta.x * delta.x + delta.y * delta.y;
    if magnitude < 0.00001 && magnitude > -0.00001 {
        return false;
    }
    if f64::from(delta.y) > 0.0 {
        return false;
    }
    let angle = melee_lb::trigf::stick_angle(delta.y, fabsf(delta.x));
    if f64::from(angle) > -1.0471975430846214 {
        return false;
    }
    if cpu.over_stage {
        return false;
    }
    if matches!(
        scene.stage(),
        melee_types::GrKind::Inishie1 | melee_types::GrKind::Fourside
    ) {
        unimplemented!("ftCo_800A2C80: Mushroom Kingdom and Fourside heights");
    }
    let direction = melee_lb::vector::normalize(delta);
    let data = &fp.core.collision.data;
    let ax = data.cur_pos.x + data.ecb.bottom.x;
    let ay = data.cur_pos.y + data.ecb.bottom.y;
    // 800A2E84/800A2E88: fmadds.
    let ex = fmadds(1000.0, direction.x, ax);
    let ey = fmadds(1000.0, direction.y, ay);
    let inside = |scene: &Scene, p: Vec3| !scene.outside(p.x, p.y, cpu.half_size);
    if let Some(hit) = scene.check_usable_floor(ax, ay, ex, ey) {
        if inside(scene, hit.pos) {
            return false;
        }
    }
    if let Some(hit) = scene.map.check_left_wall(ax, ay, ex, ey, -1, -1) {
        if inside(scene, hit.pos) {
            return false;
        }
    }
    if let Some(hit) = scene.map.check_right_wall(ax, ay, ex, ey, -1, -1) {
        if inside(scene, hit.pos) {
            return false;
        }
    }
    true
}

/// ftCo_800A5ACC (0x800A5ACC): whether a ranged-defence check is due
/// (xF9_b6, by level and x7C % 120).
fn ranged_ready(fp: &Fighter) -> bool {
    let cpu = &fp.core.cpu;
    if !cpu.xf9_b6 {
        return false;
    }
    let phase = cpu.reaction_timer % 120;
    match cpu.level {
        2 => phase > 0x64,
        3 => phase > 0x50,
        4 => phase > 0x3C,
        5 => phase > 0x28,
        6 => phase > 0x1E,
        7 => phase > 0x14,
        8 => phase > 0xA,
        9 => true,
        _ => false,
    }
}

/// ftCo_800A3710 (0x800A3710): an item worth picking up is in reach.
fn pick_up_ready(fp: &mut Fighter, _scene: &Scene) -> bool {
    let cpu = &fp.core.cpu;
    if !cpu.xf9_b7 || cpu.item_target.is_none() {
        return false;
    }
    unimplemented!("ftCo_800A3710: picking up the targeted item");
}

/// ftCo_800ADE48's x18 = 0xE test: holding an item the CPU uses (a
/// container or Bob-omb, or an item swung like a weapon).
fn holding_usable_item(fp: &Fighter) -> bool {
    if !crate::facts::has_item(fp) {
        return false;
    }
    unimplemented!("ftCo_800ADE48: CPU item use");
}

/// ftCo_800A3554 (0x800A3554): on the destination's island and within
/// x38 + `margin` of it. Arriving with a route waypoint pending takes the
/// waypoint instead.
pub fn arrived(fp: &mut Fighter, scene: &mut Scene, margin: f32) -> bool {
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return false;
    }
    if !route::destination_on_island(fp, scene) {
        return false;
    }
    let position = fp.core.physics.position;
    let cpu = &mut fp.core.cpu;
    let dy = cpu.destination.y - position.y;
    let dx = cpu.destination.x - position.x;
    // 800A3628: fmadds, inline sqrtf.
    if sqrtf(fmadds(dx, dx, dy * dy)) < cpu.destination_radius + margin {
        if cpu.route_timer != 0 {
            cpu.route_timer = 0;
            cpu.destination = cpu.route_destination;
            route::stage_route(fp, scene);
            return false;
        }
        return true;
    }
    false
}

/// ftCo_800A3134 (0x800A3134): 1 bouncing off the floor (DownBoundD/U),
/// 2 lying on it (DownWaitD/U), else 0.
pub(crate) fn downed(fp: &Fighter) -> u8 {
    let m = fp.core.motion_state.action.0;
    if m == S::DownBoundD as u16 || m == S::DownBoundU as u16 {
        1
    } else if m == S::DownWaitD as u16 || m == S::DownWaitU as u16 {
        2
    } else {
        0
    }
}
