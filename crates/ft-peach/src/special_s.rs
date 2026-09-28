//! Peach Bomber, ftpeachspecials.c (8011C2F4..8011CE44): a wind-up (354 on
//! the ground, 357 in the air), the flying hip attack (360), whose inert
//! hitbox sets off the blast item on contact, and the recoil (355 on the
//! ground; 358 in the air, 359 after a blast).
use crate::init::Peach;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        damage::InertTouch,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::{airborne, grounded},
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{mp::collide, CommonMotionState, FighterKind, ItemKind};

/// ftPe_MS_SpecialSStart (354): the grounded wind-up.
pub const START: ActionId = ActionId(354);
/// ftPe_MS_SpecialSEnd (355): the grounded recoil.
pub const END: ActionId = ActionId(355);
/// ftPe_MS_SpecialAirSStart (357): the aerial wind-up.
pub const AIR_START: ActionId = ActionId(357);
/// ftPe_MS_SpecialAirSEnd_0 (358): the aerial recoil without a blast.
pub const AIR_END: ActionId = ActionId(358);
/// ftPe_MS_SpecialAirSEnd_1 (359): the aerial recoil after a blast.
pub const AIR_END_BLAST: ActionId = ActionId(359);
/// ftPe_MS_SpecialAirSJump (360): the flying hip attack.
pub const JUMP: ActionId = ActionId(360);

/// start_mf: KeepColAnimHitStatus | SkipMatAnim | SkipColAnim | UpdateCmd |
/// SkipItemVis | Unk19 | SkipModelPartVis | SkipModelFlags | Unk27.
const START_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5084);
/// end_mf: SkipColAnim | UpdateCmd.
const END_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0000_5000);
/// fp->parts[FtPart_HipN] (retail 8011CDE4: parts + 0x40): the blast's origin.
const HIP_PART: usize = 4;
/// SpecialSStart_Anim's lift into the jump, in player-scale units.
const LIFT_BACK: f32 = -4.0;
const LIFT_UP: f32 = 3.5;

/// Command variables the scripts and callbacks share.
mod var {
    /// A wall or ceiling stopped the wind-up: go straight to the recoil.
    pub const STOPPED: usize = 0;
    /// Raised by the jump's script: air friction and the later gravity.
    pub const SLOWING: usize = 1;
    /// The jump connected or hit a wall: the recoil leaves a blast.
    pub const BLAST: usize = 2;
    /// Raised by the jump's script to end it early.
    pub const DONE: usize = 3;
}

/// fp->mv.pe.specials.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bomber {
    /// x0: a smash input started the move; the jump flies farther and the
    /// blast takes its smash state.
    pub smash: bool,
}

fn attributes(f: &Fighter) -> &crate::attributes::BomberAttributes {
    &f.character.get::<Peach>().attributes.bomber
}

/// ftPe_SpecialS_Enter (8011C34C) / ftPe_SpecialAirS_Enter (8011C3C4).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let a = attributes(f).clone();
    let state = if air {
        f.physics.self_velocity.y = a.air_start_vertical_speed;
        AIR_START
    } else {
        f.physics.self_velocity.y = 0.0;
        // retail 8011C390: fmuls.
        f.physics.ground_velocity = a.ground_start_speed * f.physics.facing;
        START
    };
    // x21EC = reset, which Fighter_ChangeMotionState runs before the new
    // motion's first script frame; nothing it changes is read in between.
    reset(f);
    f.change_motion_state(state, assets)
        .expect("Peach Bomber assets");
    f.step_animation(assets);
}

/// reset (8011C2F4): the four command variables, the horizontal speed and
/// the smash check. x6A4 (the TransN offset) is zeroed again by the frame-0
/// motion entry; x2070's count_thrown_items is a statistic.
fn reset(f: &mut Fighter) {
    f.commands.variables[..4].fill(0);
    f.physics.self_velocity.x = 0.0;
    // retail 8011C31C..24: the u8 x673 against the int window, cmpw.
    let smash = i32::from(f.input.horizontal.held) < attributes(f).smash_input_window;
    f.character.get_mut::<Peach>().bomber = Bomber { smash };
}

/// ftPe_SpecialSStart_Anim (8011C4F0): a stopped wind-up recoils at once;
/// otherwise Peach lifts off into the jump.
pub fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.animation.frames_remaining(&f.skeleton) {
        return Ok(None);
    }
    if f.commands.variables[var::STOPPED] != 0 {
        enter_end(f, p.assets)?;
        return Ok(None);
    }
    f.leave_ground();
    let scale = f.player.scale;
    let position = &mut f.core.physics.position;
    // retail 8011C548 fmuls, 8011C550 / 8011C564 fmadds.
    position.x = gekko_math::fma::fmadds(LIFT_BACK * f.core.physics.facing, scale, position.x);
    position.y = gekko_math::fma::fmadds(LIFT_UP, scale, position.y);
    enter_jump(f, p.assets)?;
    Ok(None)
}

/// ftPe_SpecialAirSStart_Anim (8011C588).
pub fn air_start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.commands.variables[var::STOPPED] != 0 {
            enter_air_end(f, p.assets)?;
        } else {
            enter_jump(f, p.assets)?;
        }
    }
    Ok(None)
}

/// Every Peach Bomber IASA is empty.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftPe_SpecialSStart_Phys (8011C5F0): ftCommon_8007CA80 toward the
/// wind-up's top speed, then ftCommon_ApplyGroundMovement.
pub fn start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f);
    let facing = f.physics.facing;
    // retail 8011C60C / 8011C614: fmuls, fmuls.
    let acceleration = a.ground_start_acceleration * facing;
    let target = a.ground_start_max_speed * facing;
    let physics = &mut f.core.physics;
    physics.ground_acceleration = accelerate_toward(physics.ground_velocity, acceleration, target);
    let normal = f.core.collision.data.floor.normal;
    let terrain = p.map.floor_speed_scale(&f.core.collision.data);
    grounded::apply_ground_movement(&mut f.core.physics, normal, terrain);
    finish_ground(f, &p);
}

/// ftCommon_8007CA80 (8007CA80): the acceleration toward `target`, trimmed
/// so the speed does not pass it; friction plays no part.
fn accelerate_toward(velocity: f32, acceleration: f32, target: f32) -> f32 {
    if target == 0.0 {
        return -velocity;
    }
    // !(gr_vel * accel < 0), unordered included.
    if (velocity * acceleration).partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        if acceleration > 0.0 {
            if velocity + acceleration > target {
                return target - velocity;
            }
        } else if velocity + acceleration < target {
            return target - velocity;
        }
    }
    acceleration
}

/// Fighter_procUpdate's grounded tail after the callback set the velocities.
fn finish_ground(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let core = &mut f.core;
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}

/// ftPe_SpecialSStart_Coll (8011C664): ft_800827A0 stops at the edge;
/// losing the floor continues in the air. A wall ahead stops the wind-up.
pub fn start_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::ground::{map_escape, WaitGroundResult};
    let c = &mut f.core;
    if map_escape(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) != WaitGroundResult::Supported
    {
        // enterAirStart (8011C7B0).
        f.physics.self_velocity.x = 0.0;
        f.leave_ground();
        change_in_place(f, AIR_START, START_FLAGS, assets(&p))?;
    }
    if wall_ahead(f) {
        f.physics.ground_velocity = 0.0;
        f.commands.variables[var::STOPPED] = 1;
    }
    Ok(())
}

/// ftPe_SpecialAirSStart_Coll (8011C6FC): landing continues on the ground;
/// a ceiling or a wall ahead stops the wind-up.
pub fn air_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if landed(f, &mut p) {
        // enterStart (8011C818).
        f.land();
        change_in_place(f, START, START_FLAGS, assets(&p))?;
    }
    if f.collision.data.env_flags as u32 & collide::CEILING_MASK != 0 {
        f.physics.self_velocity.y = 0.0;
        f.commands.variables[var::STOPPED] = 1;
    }
    if wall_ahead(f) {
        f.physics.self_velocity.x = 0.0;
        f.commands.variables[var::STOPPED] = 1;
    }
    Ok(())
}

/// Collide_RightWallMask while facing left, or Collide_LeftWallMask while
/// facing right.
fn wall_ahead(f: &Fighter) -> bool {
    let env = f.collision.data.env_flags as u32;
    (env & collide::RIGHT_WALL_MASK != 0 && f.physics.facing == -1.0)
        || (env & collide::LEFT_WALL_MASK != 0 && f.physics.facing == 1.0)
}

/// enterAirJump (8011C9E0): the flight's speed, then the jump, which arms
/// hurtbox_detect_cb (see [`inert_contact`]). x21F8 = ftCommon_8007F76C
/// only runs after a cape turnaround (ftCo_800C37A0), not modelled.
fn enter_jump(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let a = attributes(f).clone();
    let speed = if f.character.get::<Peach>().bomber.smash {
        a.smash_travel_speed
    } else {
        a.travel_speed
    };
    // retail 8011CA0C / 8011CA20: fmuls.
    f.physics.self_velocity.x = speed * f.physics.facing;
    f.physics.self_velocity.y = a.travel_vertical_speed;
    f.change_motion_state(JUMP, assets)
}

/// ftPe_SpecialAirSJump_Anim (8011C878): the script's end flag or the
/// animation's end recoils.
pub fn jump_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[var::DONE] != 0 || !f.animation.frames_remaining(&f.skeleton) {
        enter_air_end(f, p.assets)?;
    }
    Ok(None)
}

/// ftPe_SpecialAirSJump_Phys (8011C8CC): once the script slows the jump,
/// air friction and the second gravity.
pub fn jump_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f).clone();
    let slowing = f.commands.variables[var::SLOWING] != 0;
    if slowing {
        // ftCommon_ApplyFrictionAir (8007CE94).
        let x = f.physics.self_velocity.x;
        f.physics.animation_velocity.x = if a.air_friction.abs() >= x.abs() {
            -x
        } else if x > 0.0 {
            -a.air_friction
        } else {
            a.air_friction
        };
    }
    let gravity = if slowing {
        a.travel_gravity
    } else {
        a.start_gravity
    };
    f.physics.self_velocity.y =
        airborne::gravity(f.physics.self_velocity.y, gravity, a.terminal_velocity);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPe_SpecialAirSJump_Coll (8011C93C): landing recoils on the ground; a
/// wall ahead then recoils with a blast, even after that landing.
pub fn jump_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if landed(f, &mut p) {
        f.land();
        enter_end(f, assets(&p))?;
    }
    if wall_ahead(f) {
        f.commands.variables[var::BLAST] = 1;
        enter_air_end(f, assets(&p))?;
    }
    Ok(())
}

/// ftPe_SpecialSEnd_Anim (8011CA84) / ftPe_SpecialAirSEnd_Anim (8011CAC0).
pub fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}
pub fn air_end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
    }
    Ok(None)
}

/// ftPe_SpecialSEnd_Phys -> ft_80084F3C.
pub fn end_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
/// ftPe_SpecialAirSStart_Phys / ftPe_SpecialAirSEnd_Phys -> ft_80084EEC.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

/// ftPe_SpecialSEnd_Coll (8011CB44): ft_80082708; off the floor, the
/// aerial recoil (enterAirEnd).
pub fn end_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::ground::{map_ground_action, WaitGroundResult};
    let c = &mut f.core;
    if map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) != WaitGroundResult::Supported
    {
        f.leave_ground();
        change_in_place(f, AIR_END, END_FLAGS, assets(&p))?;
    }
    Ok(())
}

/// ftPe_SpecialAirSEnd_Coll (8011CB80): ft_80081D0C; landing, the grounded
/// recoil (enterEnd).
pub fn air_end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if landed(f, &mut p) {
        f.land();
        change_in_place(f, END, END_FLAGS, assets(&p))?;
    }
    Ok(())
}

/// enterEndSmash (8011CC74): the grounded recoil.
fn enter_end(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(END, assets)?;
    leave_blast(f);
    Ok(())
}

/// enterAirEndSmash (8011CD30): the flight's speed divided down, then the
/// aerial recoil, with or without a blast.
fn enter_air_end(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let a = attributes(f).clone();
    // retail 8011CD5C / 8011CD6C: fdivs, fdivs.
    f.physics.self_velocity.x /= a.hit_horizontal_divisor;
    f.physics.self_velocity.y /= a.hit_vertical_divisor;
    let state = if f.commands.variables[var::BLAST] != 0 {
        AIR_END_BLAST
    } else {
        AIR_END
    };
    f.change_motion_state(state, assets)?;
    leave_blast(f);
    Ok(())
}

/// doPostEnd (inlined in both recoils): after a connection the blast
/// appears at the hip, on the stage plane, and Peach rebounds. x21F8 =
/// ftCommon_8007F7B4 is not modelled (see [`enter_jump`]).
fn leave_blast(f: &mut Fighter) {
    if f.commands.variables[var::BLAST] == 0 {
        return;
    }
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        HIP_PART,
        hsd_types::Vec3::ZERO,
    );
    position.z = 0.0;
    let smash = f.character.get::<Peach>().bomber.smash;
    // it_802BD158: Item_InitSpawn, then it_802BD248 with the smash flag.
    let spawn = SpawnItem {
        spawn_argument: i32::from(smash),
        ..SpawnItem::attached(
            ItemKind::PeachExplode,
            f.player.id,
            position,
            f.physics.facing,
        )
    };
    f.core.item_requests.push(ItemRequest::Spawn(spawn));
    let a = attributes(f).clone();
    // retail 8011CE10: fmuls.
    f.physics.self_velocity.x = a.rebound_horizontal_speed * f.physics.facing;
    f.physics.self_velocity.y = a.rebound_vertical_speed;
}

/// doAirEnd0 (8011C430), the jump's hurtbox_detect_cb: the inert hip
/// hitbox reached a fighter. After a shield touch (x221C_b5) it only counts
/// when the touched fighter is Marth or Roy in 369 / 371 or Peach in 365 /
/// 367 (retail 8011C440..94).
pub fn inert_contact(f: &mut Fighter, assets: &FighterAssets, touch: InertTouch) {
    if f.motion_state.action != JUMP {
        return;
    }
    if touch.shield {
        let counters: &[u16] = match touch.kind {
            FighterKind::Mars | FighterKind::Emblem => &[369, 371],
            FighterKind::Peach => &[365, 367],
            _ => return,
        };
        if !counters.contains(&touch.action.0) {
            return;
        }
    }
    f.physics.self_velocity.x = 0.0;
    if f.physics.self_velocity.y >= 0.0 {
        f.physics.self_velocity.y = 0.0;
    }
    f.commands.variables[var::BLAST] = 1;
    enter_air_end(f, assets).expect("Peach Bomber recoil assets");
}

/// Fighter_ChangeMotionState at the current frame with `flags`
/// (ftCommon_GroundToAirStateChange / AirToGroundStateChange).
fn change_in_place(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    assets: &FighterAssets,
) -> Result<()> {
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, flags, frame, 1.0)
}

/// ft_80081D0C: ordinary airborne collision; true on landing.
fn landed(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    )
}

fn assets<'a>(p: &CollisionPhase<'a>) -> &'a FighterAssets {
    p.assets.expect("Peach Bomber collision assets")
}
