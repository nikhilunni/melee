//! ftcpuattack.c: whether and how the CPU attacks its target.
use crate::{desc::AttackEntry, world::Scene};
use gekko_math::{
    fma::{fmadd, fmadds, fnmsub},
    msl::{fctiwz, sqrtf},
    HsdRng,
};
use melee_ft::fighter::Fighter;
use melee_types::{CommonMotionState as S, FighterKind as K, GroundOrAir};

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
    /// Excluded for CPU mode 7 or an untouchable CPU.
    pub const MODE_7: i32 = 0x34;
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
    exclude_for_kind(fp, scene);
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
    if crate::facts::has_item(fp) {
        unimplemented!("ftCo_800B8A9C: a CPU holding an item (ftCo_800A59E4, ftCo_800B52AC)");
    }
    let target = scene.fighter(target_index);
    if fp.core.cpu.level > 5 && shielding(target) {
        if let Some(script) = choose(fp, scene, target_index, data.smash[kind].as_deref(), rng) {
            fp.core.cpu.xa4 = script;
            return true;
        }
    }
    if target_over_no_floor(scene, target_index) {
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

/// ftCo_800B77E8 (0x800B77E8): scripts CPU mode 7 and an untouchable CPU
/// skip, then the kind's own (ranged specials out of range, Peach's turnip
/// without a floor ahead). The climbers have none.
fn exclude_for_kind(fp: &mut Fighter, _scene: &Scene) {
    if fp.core.cpu.mode == 7 || untouchable(fp) {
        fp.core.cpu.excluded_attacks.push(scripts::MODE_7);
    }
    match fp.core.kind {
        K::Popo | K::Nana => {}
        kind @ (K::Mario
        | K::Fox
        | K::Falco
        | K::Captain
        | K::Ganon
        | K::Pikachu
        | K::Pichu
        | K::Koopa
        | K::GKoops
        | K::Yoshi
        | K::Luigi
        | K::Peach
        | K::Kirby) => unimplemented!("ftCo_800B77E8: {kind:?}'s exclusions"),
        _ => {}
    }
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

/// ftCo_800B630C (0x800B630C): mid-attack (Attack11..AttackAirLw), or in
/// a character's own motion.
fn attacking(fp: &Fighter) -> bool {
    let m = fp.core.motion_state.action.0;
    if (S::Attack11 as u16..=S::AttackAirLw as u16).contains(&m) {
        return true;
    }
    if crate::facts::has_item(fp) {
        unimplemented!("ftCo_800A5A90: swinging or shooting an item");
    }
    if matches!(fp.core.kind, K::Donkey | K::Kirby | K::Peach) {
        unimplemented!("ftCo_800B630C: {:?}'s motion ranges", fp.core.kind);
    }
    // ftCo_MS_Count: the first character-owned motion.
    m >= 341
}

/// ftCo_800B658C (0x800B658C): behaviour 2, the close attack's script.
pub fn attack(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    if fp.core.cpu.target.is_none() {
        clear_target_and_finish(fp);
        return;
    }
    let cpu = &fp.core.cpu;
    if cpu.xf8_b6 && fp.core.physics.ground_or_air == GroundOrAir::Ground {
        let threshold = 0.05f32 * cpu.level as f32;
        if rng.randf() < threshold {
            unimplemented!("ftCo_800BA2E8: a roll before attacking");
        }
    }
    if matches!(
        fp.core.kind,
        K::Ness | K::Yoshi | K::Samus | K::Donkey | K::Zelda
    ) {
        unimplemented!("ftCo_800B658C: {:?}'s special follow-ups", fp.core.kind);
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

/// ftCo_800B732C (0x800B732C): whether to guard the ledge the target
/// hangs from (1) or jump at it (2); 0 otherwise.
pub fn edge_guard_style(fp: &mut Fighter, scene: &mut Scene, _rng: &mut HsdRng) -> u8 {
    let Some(target) = fp.core.cpu.target.map(|index| scene.fighter(index)) else {
        return 0;
    };
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return 0;
    }
    if ledge_state(target) != 0 {
        // 800B738C..800B7398: a ledge target below the CPU (fcmpo, bge) is
        // left alone.
        if target.core.physics.position.y < fp.core.physics.position.y {
            return 0;
        }
        unimplemented!(
            "ftCo_800B732C: a target on a ledge above the CPU (ftcpuattack.c:1258, mpCheckCeiling and the 75 degree test)"
        );
    }
    if !fp.core.cpu.xf9_b3 {
        return 0;
    }
    unimplemented!("ftCo_800B732C: edge guarding (ftcpuattack.c:1278)");
}
