//! ftcpuattack.c: whether and how the CPU attacks its target.
use crate::{desc::AttackEntry, world::Scene};
use gekko_math::{
    fma::{fmadd, fmadds, fnmsub},
    msl::{fctiwz, sqrtf},
    HsdRng,
};
use melee_ft::fighter::Fighter;
use melee_types::{CommonMotionState as S, FighterKind as K, GroundOrAir};

pub use crate::ranged::ranged_attack;

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

/// ftCo_800A1C44 (0x800A1C44): a fighter the CPU leaves alone: out of play
/// (x2219_b1) or disabled (x221F_b3). Its Starman (x2164) and Hammer
/// (x2168) arms need items the port does not have.
pub fn untouchable(fp: &Fighter) -> bool {
    fp.core.out_of_play() || fp.core.status.disabled
}

/// PlCo attack scripts the choice adjusts for (ftCo_AttackEntry.cmd).
mod scripts {
    /// Excluded against a grabbed-range target or a bigger one.
    pub const GRAB: i32 = 0x28;
    pub const GRAB_RUN: i32 = 0x29;
    /// Kirby's inhale, excluded against a bigger target.
    pub const INHALE: i32 = 0x11;
    /// The only ones allowed against a downed or ledge-hanging target.
    pub const DOWN_A: i32 = 0xA;
    pub const DOWN_B: i32 = 0xE;
}

/// ftCo_800B8A9C (0x800B8A9C): a close attack now (x18 2); xA4 is the
/// chosen script.
pub fn close_attack(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) -> bool {
    if !fp.core.cpu.xf9_b2 {
        return false;
    }
    fp.core.cpu.excluded_attacks.clear();
    fp.core.cpu.allowed_attacks.clear();
    let Some(target_index) = fp.core.cpu.target else {
        fp.core.cpu.xa4 = 0;
        return false;
    };
    let target = scene.fighter(target_index);
    // ftCo_800A1CA8 (the Hammer) needs items.
    if fp.core.motion_state.action.0 == S::Pass as u16 || untouchable(target) || holding_back(fp) {
        fp.core.cpu.xa4 = 0;
        return false;
    }
    let target_motion = target.core.motion_state.action.0;
    let bigger = target.player.scale > fp.player.scale;
    let cpu = &mut fp.core.cpu;
    if (S::Catch as u16..=S::EscapeAir as u16).contains(&target_motion) {
        cpu.excluded_attacks.push(scripts::GRAB);
        cpu.excluded_attacks.push(scripts::GRAB_RUN);
    } else if bigger {
        cpu.excluded_attacks.push(scripts::GRAB);
        cpu.excluded_attacks.push(scripts::GRAB_RUN);
        if fp.core.kind == K::Kirby {
            cpu.excluded_attacks.push(scripts::INHALE);
        }
    }
    crate::exclusions::exclude_for_kind(fp, scene);
    let kind = usize::from(fp.core.kind as u8);
    let data = scene.data;
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        if air_attack_ready(fp, scene) {
            if let Some(script) = choose(fp, scene, target_index, data.air[kind].as_deref(), rng) {
                fp.core.cpu.xa4 = script;
                return true;
            }
        }
        fp.core.cpu.xa4 = 0;
        return false;
    }
    let target = scene.fighter(target_index);
    if crate::select::downed(target) != 0 || ledge_state(target) != 0 {
        fp.core.cpu.allowed_attacks.push(scripts::DOWN_A);
        fp.core.cpu.allowed_attacks.push(scripts::DOWN_B);
        let chosen = choose(fp, scene, target_index, data.ground[kind].as_deref(), rng);
        fp.core.cpu.xa4 = chosen.unwrap_or(0);
        return chosen.is_some();
    }
    let holding = crate::facts::held_kind(fp);
    if holding.is_some_and(swing_weapon) {
        unimplemented!("ftCo_800B52AC: a swing weapon's attacks (ftcpuattack.c:1844)");
    }
    let target = scene.fighter(target_index);
    if fp.core.cpu.level > 5 && shielding(target) {
        if let Some(script) = choose(fp, scene, target_index, data.smash[kind].as_deref(), rng) {
            fp.core.cpu.xa4 = script;
            return true;
        }
    }
    // Holding anything (an article too), the CPU does not edge guard.
    if holding.is_none() && target_over_no_floor(scene, target_index) {
        if let Some(script) = choose(
            fp,
            scene,
            target_index,
            data.edge_guard[kind].as_deref(),
            rng,
        ) {
            fp.core.cpu.xa4 = script;
            fp.core.cpu.xf8_b7 = true;
            return true;
        }
    }
    if let Some(script) = choose(fp, scene, target_index, data.ground[kind].as_deref(), rng) {
        fp.core.cpu.xa4 = script;
        return true;
    }
    if fp.core.cpu.x50 != 0 {
        unimplemented!("ftCo_800B5AB0: an attack at a stage enemy");
    }
    fp.core.cpu.xa4 = 0;
    false
}

/// ftCo_800A59E4 (0x800A59E4): the fan, the Star Rod, the Lip's Stick, the
/// bat, the Beam Sword or the parasol.
fn swing_weapon(kind: melee_types::ItemKind) -> bool {
    use melee_types::ItemKind as I;
    matches!(
        kind,
        I::Harisen | I::StarRod | I::LipStick | I::Bat | I::Sword | I::Parasol
    )
}

/// ftCo_800A3200 (0x800A3200): 1 in CliffCatch, 2 in CliffWait.
pub(crate) fn ledge_state(fp: &Fighter) -> u8 {
    match fp.core.motion_state.action.0 {
        m if m == S::CliffCatch as u16 => 1,
        m if m == S::CliffWait as u16 => 2,
        _ => 0,
    }
}

/// ftCo_800B9F6C (0x800B9F6C): GuardOn or Guard.
fn shielding(fp: &Fighter) -> bool {
    let m = fp.core.motion_state.action.0;
    m == S::GuardOn as u16 || m == S::Guard as u16
}

/// The floor half of ftCo_800B8A9C's edge-guard test: a target neither on
/// the ledge nor over a usable floor from 5 above to 1000 below
/// (ftCo_800A0FB0; the y offsets in double, rounded).
fn target_over_no_floor(scene: &mut Scene, target_index: usize) -> bool {
    let target = scene.fighter(target_index);
    if ledge_state(target) != 0 {
        return false;
    }
    let p = target.core.physics.position;
    let above = (5.0 + f64::from(p.y)) as f32;
    let below = (f64::from(p.y) - 1000.0) as f32;
    scene.check_usable_floor(p.x, above, p.x, below).is_none()
}

/// ftCo_800B89CC (0x800B89CC): an aerial attack is on (the target on the
/// ledge, or xF9_b4 at level 4 or more; Bowser only over the stage).
fn air_attack_ready(fp: &Fighter, scene: &Scene) -> bool {
    if scene.stage() == melee_types::GrKind::Kongo {
        unimplemented!("ftCo_800B89CC: Kongo Jungle's barrel range");
    }
    let target = fp.core.cpu.target.map(|index| scene.fighter(index));
    if target.is_some_and(|t| ledge_state(t) != 0) {
        return true;
    }
    let cpu = &fp.core.cpu;
    if !cpu.xf9_b4 {
        return false;
    }
    if !cpu.over_stage && fp.core.kind == K::Koopa {
        return false;
    }
    cpu.level >= 4
}

/// ftCo_800B4AB0 (0x800B4AB0): the attacks of `list` whose hit box, scaled
/// and grown by x570, meets where the target will be when they hit;
/// weighted at random among them. Picking one sets x6C/x74 to its box.
fn choose(
    fp: &mut Fighter,
    scene: &Scene,
    target_index: usize,
    list: Option<&[AttackEntry]>,
    rng: &mut HsdRng,
) -> Option<i32> {
    let cpu = &mut fp.core.cpu;
    cpu.x74 = hsd_types::Vec2::new(0.0, 0.0);
    cpu.x6c = hsd_types::Vec2::new(0.0, 0.0);
    // 800B4B34: fmadd 0.5 * x570 + 0.5 in double, frsp.
    let grow = fmadd(0.5, f64::from(cpu.x570), 0.5) as f32;
    let list = list?;
    let target = scene.fighter(target_index);
    let me = Mover::of(fp);
    let them = Mover::of(target);
    let (range_front, range_back) = if f64::from(target.core.physics.facing) > 0.0 {
        (
            target.core.cpu.hurtbox_extents[0],
            target.core.cpu.hurtbox_extents[1],
        )
    } else {
        (
            target.core.cpu.hurtbox_extents[1],
            target.core.cpu.hurtbox_extents[0],
        )
    };
    let target_height = target.core.cpu.hurtbox_extents[3];
    let target_airborne = target.core.physics.ground_or_air == GroundOrAir::Air;
    let scale = fp.player.scale;
    let facing = fp.core.physics.facing;
    let cpu = &fp.core.cpu;
    let mut candidates = [AttackEntry::NONE; 32];
    let mut count = 0;
    for entry in list {
        if entry.level > cpu.level || cpu.excluded_attacks.as_slice().contains(&entry.script) {
            continue;
        }
        // 800B4C58..68: the frames as single (fsubs), two fmadds, fsubs.
        let t = entry.frames as f32;
        let relative_x = fmadds(them.vx, t, them.x) - fmadds(me.vx, t, me.x);
        // Retail tests the target's air state for its own prediction too.
        let my_y = if target_airborne {
            me.predicted_y(t)
        } else {
            fmadds(me.vy, t, me.y)
        };
        let relative_y = if target_airborne {
            them.predicted_relative_y(t, my_y)
        } else {
            fmadds(them.vy, t, them.y) - my_y
        };
        let (mut front, mut back) = if facing > 0.0 {
            (entry.front * scale, entry.back * scale)
        } else {
            (-entry.back * scale, -entry.front * scale)
        };
        front *= grow;
        back *= grow;
        let high = entry.high * scale * grow;
        let low = entry.low * scale * grow;
        if !(high > relative_y
            && low < relative_y + target_height
            && front < relative_x + range_front
            && back > relative_x - range_back)
        {
            continue;
        }
        if cpu.allowed_attacks.len != 0 {
            for &allowed in cpu.allowed_attacks.as_slice() {
                if allowed == entry.script {
                    candidates[count] = *entry;
                    count += 1;
                }
            }
        } else {
            assert_ne!(
                entry.period, 0,
                "ftCo_800B4AB0: an attack entry with period 0"
            );
            if cpu.x80 % entry.period == 0 {
                candidates[count] = *entry;
                count += 1;
            }
        }
    }
    if count == 0 {
        return None;
    }
    let roll = rng.randf();
    let mut sum = 0.0f32;
    for entry in &candidates[..count] {
        sum += entry.weight;
    }
    if sum < 0.00001 && sum > -0.00001 {
        return None;
    }
    // 800B518C: 1.0 / sum in double, frsp.
    let inverse = (1.0 / f64::from(sum)) as f32;
    let mut accumulated = 0.0f32;
    for entry in &candidates[..count] {
        accumulated += entry.weight;
        if accumulated * inverse >= roll {
            // ftCo_CpuSelectAttack: fmuls each.
            let cpu = &mut fp.core.cpu;
            cpu.x6c = hsd_types::Vec2::new(scale * entry.front, scale * entry.low);
            cpu.x74 = hsd_types::Vec2::new(scale * entry.back, scale * entry.high);
            return Some(entry.script);
        }
    }
    panic!("ftcpuattack.c:250: no attack chosen");
}

/// A fighter's motion as ftCo_800B4AB0 extrapolates it.
struct Mover {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    gravity: f32,
    /// -terminal_velocity.
    terminal: f32,
}

impl Mover {
    fn of(fp: &Fighter) -> Self {
        let p = fp.core.physics.position;
        let d = fp.core.physics.position_delta;
        let air = &fp.core.attributes.air;
        Self {
            x: p.x,
            y: p.y,
            vx: d.x,
            vy: d.y,
            gravity: air.gravity,
            terminal: -air.terminal_velocity,
        }
    }

    /// The frames until the fall speed caps (1000 without gravity).
    fn cap_frames(&self, rest: f32) -> f32 {
        if self.gravity < 0.00001 && self.gravity > -0.00001 {
            1000.0
        } else {
            // 800B4BA4..BB8: fsubs, fneg, then fdivs.
            -(self.terminal - rest) / self.gravity
        }
    }

    /// The CPU's own height in `t` frames (800B4C70..800B4D98). Retail
    /// folds gravity in as sqrtf(t), not t squared: fmuls, fmuls, fnmsub,
    /// fadd in double, frsp.
    fn predicted_y(&self, t: f32) -> f32 {
        let v = self.cap_frames(self.y);
        if v <= 0.0 {
            return fmadds(self.vy, t, self.y);
        }
        if t < v {
            let fall = self.gravity * sqrtf(t);
            let rise = self.vy * t;
            let sum = fnmsub(0.5, f64::from(fall), f64::from(rise));
            return (f64::from(self.y) + sum) as f32;
        }
        let fall = self.gravity * sqrtf(v);
        let rise = self.vy * t;
        let capped = t - v;
        let sum = fnmsub(0.5, f64::from(fall), f64::from(rise));
        let sum = f64::from(self.terminal * capped) + sum;
        (f64::from(self.y) + sum) as f32
    }

    /// The target's height in `t` frames less the CPU's (800B4DB0..800B4EE8):
    /// the cap counts from the vertical speed, not the height.
    fn predicted_relative_y(&self, t: f32, mine: f32) -> f32 {
        let v = self.cap_frames(self.vy);
        if v <= 0.0 {
            return fmadds(self.vy, t, self.y) - mine;
        }
        if t < v {
            let fall = self.gravity * sqrtf(t);
            let line = fmadds(self.vy, t, self.y);
            let sum = fnmsub(0.5, f64::from(fall), f64::from(line));
            return (sum - f64::from(mine)) as f32;
        }
        let fall = self.gravity * sqrtf(v);
        let line = fmadds(self.vy, t, self.y);
        let capped = t - v;
        let sum = fnmsub(0.5, f64::from(fall), f64::from(line));
        let sum = f64::from(self.terminal * capped) + sum;
        (sum - f64::from(mine)) as f32
    }
}

/// ftCo_800B63D8 (0x800B63D8): a level-scaled pause before the attack
/// (`(int)(n * Randf())` then + 5 for the lowest levels).
fn attack_delay(fp: &mut Fighter, rng: &mut HsdRng) {
    let roll = rng.randf();
    let frames = match fp.core.cpu.level {
        0 => fctiwz(40.0 * roll) + 5,
        1 => fctiwz(35.0 * roll) + 5,
        2 => fctiwz(30.0 * roll) + 5,
        3 => fctiwz(25.0 * roll) + 5,
        4 => fctiwz(25.0 * roll),
        5 => fctiwz(20.0 * roll),
        6 => fctiwz(15.0 * roll),
        7 => fctiwz(10.0 * roll),
        8 => fctiwz(5.0 * roll),
        _ => return,
    };
    crate::script::command1(
        &mut fp.core.cpu,
        crate::script::Command::WaitFor,
        frames as u8,
    );
}

/// ftCo_800B630C (0x800B630C): mid-attack (Attack11..AttackAirLw, 0x2C..
/// 0x45), swinging or shooting an item (ftCo_800A5A90), or in one of the
/// kind's own motions: past Donkey Kong's cargo throws (ftDk_MS_ThrowAirFLw,
/// 0x170), Kirby's aerial dash attack (ftKb_MS_AttackDashAir, 0x160),
/// Peach's float falls (ftPe_MS_FloatFallB, 0x157), or anyone else's
/// common motions (ftCo_MS_Count, 0x155).
pub(crate) fn attacking(fp: &Fighter) -> bool {
    let m = fp.core.motion_state.action.0;
    if (S::Attack11 as u16..=S::AttackAirLw as u16).contains(&m) || item_attacking(m) {
        return true;
    }
    m >= fp.character.table().cpu_profile.attacking_from
}

/// ftCo_800A5A90 (0x800A5A90): a motion of swinging an item (0x78..=0x8B)
/// or shooting one (0x94..=0xAD); only an item puts a fighter there.
pub(crate) fn item_attacking(motion: u16) -> bool {
    (0x78..=0x8B).contains(&motion) || (0x94..=0xAD).contains(&motion)
}

/// ftCo_800BA2E8 (0x800BA2E8): roll away from the fighter `from` when the
/// island's end that way (ftCo_800A2A70) is farther than the kind's reach
/// (Fighter_804D64FC x20, a single compare), else toward it when that end
/// is; true when it rolled. The side is an fsubs against 0 in double.
pub(crate) fn roll_away(fp: &mut Fighter, scene: &Scene, from: usize) -> bool {
    let reach = scene.data.reach[usize::from(fp.core.kind as u8)];
    let dx = fp.core.physics.position.x - scene.fighter(from).core.physics.position.x;
    // Right of `from`: the right end (x14, ftCo_800BA160) first.
    let order = if f64::from(dx) > 0.0 {
        [false, true]
    } else {
        [true, false]
    };
    for left in order {
        if crate::roll::island_end_distance(fp, scene, left) > reach {
            if left {
                crate::roll::roll_left(fp);
            } else {
                crate::roll::roll_right(fp);
            }
            return true;
        }
    }
    false
}

/// ftCo_800B658C (0x800B658C): behaviour 2, the close attack's script.
pub fn attack(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    let Some(target) = fp.core.cpu.target else {
        clear_target_and_finish(fp);
        return;
    };
    if fp.core.cpu.xf8_b6 && fp.core.physics.ground_or_air == GroundOrAir::Ground {
        // 800B6614/800B6618: the level as single (fsubs), fmuls; the
        // Randf is drawn only here.
        let threshold = 0.05f32 * fp.core.cpu.level as f32;
        if rng.randf() < threshold {
            fp.core.cpu.xf8_b6 = false;
            if roll_away(fp, scene, target) {
                return;
            }
        }
    }
    if let crate::follow_up::FollowUp::Done = crate::follow_up::follow_up(fp, scene) {
        return;
    }
    if attacking(fp) {
        crate::script::command(&mut fp.core.cpu, crate::script::Command::Done);
        fp.core.cpu.xa4 = 0;
        return;
    }
    if fp.core.cpu.xa4 == 0 {
        if fp.core.physics.ground_or_air == GroundOrAir::Ground && !facing_target(fp, scene) {
            turn_toward(fp);
            return;
        }
        if !close_attack(fp, scene, rng) {
            clear_target_and_finish(fp);
            return;
        }
    }
    attack_delay(fp, rng);
    let script = usize::try_from(fp.core.cpu.xa4).expect("attack script");
    crate::script::stored(&mut fp.core.cpu, scene.data, script);
    fp.core.cpu.xa4 = 0;
}

/// ftCo_CpuClearTargetAndFinish: x44 = NULL, x18 = x1C, Done.
fn clear_target_and_finish(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    cpu.target = None;
    cpu.behavior = cpu.home_behavior;
    crate::script::command(cpu, crate::script::Command::Done);
}

/// ftCo_800A2C08 (0x800A2C08): no target, the target within 1.0 in x, or
/// the fighter faces it (fsubs; fmuls, a double compare with >=).
pub fn facing_target(fp: &Fighter, scene: &Scene) -> bool {
    let Some(target) = fp.core.cpu.target.map(|index| scene.fighter(index)) else {
        return true;
    };
    let dx = target.core.physics.position.x - fp.core.physics.position.x;
    let distance = if dx < 0.0 { -dx } else { dx };
    if distance < 1.0 {
        return true;
    }
    f64::from(dx * fp.core.physics.facing) >= 0.0
}

/// ftCo_800A0098 (0x800A0098): a turn: neutral, a tick, the stick back
/// (0xB0 against the facing), a tick, then held neutral for ten.
fn turn_toward(fp: &mut Fighter) {
    use crate::script::{command, command1, Command as C};
    let cpu = &mut fp.core.cpu;
    command1(cpu, C::SetLstickX, 0);
    command1(cpu, C::SetLstickY, 0);
    command1(cpu, C::WaitFor, 1);
    command1(cpu, C::LstickXForward, 0xB0);
    command1(cpu, C::WaitFor, 1);
    command1(cpu, C::SetLstickY, 0);
    command1(cpu, C::WaitFor, 0xA);
    command1(cpu, C::SetLstickX, 0);
    command(cpu, C::Done);
}

/// ftCo_800B732C's "@294" (0x800B7410), 1.3089969 rad: a ledge target is
/// guarded when it hangs more than 75 degrees above the CPU.
const LEDGE_GUARD_BEARING: f32 = 1.308_996_9;

/// ftCo_800B732C (0x800B732C): whether to guard the ledge the target
/// hangs from (1) or jump at it (2); 0 otherwise.
pub fn edge_guard_style(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) -> u8 {
    let Some(target) = fp.core.cpu.target.map(|index| scene.fighter(index)) else {
        return 0;
    };
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return 0;
    }
    let (on_ledge, theirs) = (ledge_state(target) != 0, target.core.physics.position);
    if on_ledge {
        let own = fp.core.physics.position;
        // 800B738C..800B7398: a ledge target below the CPU (fcmpo, bge) is
        // left alone.
        if theirs.y < own.y {
            return 0;
        }
        // 800B73A4..800B73C4: mpCheckCeiling from the CPU to the target; a
        // ceiling between them ends it.
        if scene
            .map
            .check_ceiling(own.x, own.y, theirs.x, theirs.y, -1, -1)
            .is_some()
        {
            return 0;
        }
        // 800B73E0: ftCo_800A1AB4's distance is computed and dropped.
        // 800B73E4..800B7418: the target's bearing, lb_8000D008(dy, |dx|)
        // (two fsubs, fneg below zero), steeper than 75 degrees (fcmpo, ble).
        let mut across = theirs.x - own.x;
        if across < 0.0 {
            across = -across;
        }
        let bearing = melee_lb::trigf::stick_angle(theirs.y - own.y, across);
        return u8::from(bearing > LEDGE_GUARD_BEARING);
    }
    if !fp.core.cpu.xf9_b3 {
        return 0;
    }
    if fp.core.cpu.mode == EDGE_GUARD_MODE {
        return jump_at_target(fp, scene);
    }
    if !crate::island_route::facing_destination(fp) || fp.core.cpu.level < 4 {
        return 0;
    }
    // 800B7488: ftCo_800A28D0 with 4.0.
    if crate::island_route::near_island_end(fp, scene, 4.0) {
        return 0;
    }
    let target = scene.fighter(fp.core.cpu.target.expect("checked above"));
    let (own, theirs) = (fp.core.physics.position, target.core.physics.position);
    if target.core.physics.ground_or_air == GroundOrAir::Air {
        // 800B74B4: fcmpo, bge.
        if theirs.y < own.y {
            return 0;
        }
        if scene
            .map
            .check_ceiling(own.x, own.y, theirs.x, theirs.y, -1, -1)
            .is_some()
        {
            return 0;
        }
        let (distance, bearing) = distance_and_bearing(own, theirs);
        return u8::from(bearing > DIAGONAL_BEARING && distance < JUMP_RANGE_NEAR);
    }
    // 800B7554: every 60th tick of x7C, at one chance in ten (Randf > 0.1
    // in double gives up).
    if fp.core.cpu.reaction_timer % 60 != 0 {
        return 0;
    }
    if f64::from(rng.randf()) > 0.1 {
        return 0;
    }
    level_with_target(own, theirs)
}

/// The CPU mode whose edge guard is ftCo_800B7638 alone (800B7444).
const EDGE_GUARD_MODE: i32 = 0x10;
/// 0.7853981573134661 rad (45 degrees, a double literal).
const DIAGONAL_BEARING: f64 = 0.785_398_157_313_466_1;
/// Edge-guard jump distances (doubles): within 30 airborne, 30..40 level.
const JUMP_RANGE_NEAR: f64 = 30.0;
const JUMP_RANGE_FAR: f64 = 40.0;
/// 0.08726646192371845 rad (5 degrees, a double literal).
const LEVEL_BEARING: f64 = 0.087_266_461_923_718_45;

/// ftCo_800A1AB4's distance (as a double) and lb_8000D008's bearing of
/// `theirs` from `own` (dy, |dx|: two fsubs, fneg below zero).
fn distance_and_bearing(own: hsd_types::Vec3, theirs: hsd_types::Vec3) -> (f64, f64) {
    let distance = crate::world::distance(own, theirs);
    let mut across = theirs.x - own.x;
    if across < 0.0 {
        across = -across;
    }
    let bearing = melee_lb::trigf::stick_angle(theirs.y - own.y, across);
    (f64::from(distance), f64::from(bearing))
}

/// 2 when a grounded target stands 30 to 40 away within 5 degrees of
/// level (800B75DC..800B7614); else 0.
fn level_with_target(own: hsd_types::Vec3, theirs: hsd_types::Vec3) -> u8 {
    let (distance, bearing) = distance_and_bearing(own, theirs);
    u8::from(
        distance > JUMP_RANGE_NEAR
            && distance < JUMP_RANGE_FAR
            && bearing > -LEVEL_BEARING
            && bearing < LEVEL_BEARING,
    ) * 2
}

/// ftCo_800B7638 (0x800B7638): CPU mode 16's edge guard. Airborne the CPU
/// needs a jump left and to be falling (pos_delta.y > 0 in double gives
/// up); the target must be no lower (fsubs against 0 in double). An
/// airborne target steeper than 45 degrees with no ceiling between them
/// is jumped at (1); a grounded one 30 to 40 away and level (2).
fn jump_at_target(fp: &Fighter, scene: &mut Scene) -> u8 {
    let Some(target) = fp.core.cpu.target else {
        return 0;
    };
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        if fp.core.attributes.jumping.max_jumps <= i32::from(fp.core.physics.jumps_used) {
            return 0;
        }
        if f64::from(fp.core.physics.position_delta.y) > 0.0 {
            return 0;
        }
    }
    let target = scene.fighter(target);
    let (own, theirs) = (fp.core.physics.position, target.core.physics.position);
    if f64::from(theirs.y - own.y) < 0.0 {
        return 0;
    }
    if target.core.physics.ground_or_air == GroundOrAir::Air {
        if scene
            .map
            .check_ceiling(own.x, own.y, theirs.x, theirs.y, -1, -1)
            .is_some()
        {
            return 0;
        }
        let (_, bearing) = distance_and_bearing(own, theirs);
        return u8::from(bearing > DIAGONAL_BEARING);
    }
    level_with_target(own, theirs)
}
