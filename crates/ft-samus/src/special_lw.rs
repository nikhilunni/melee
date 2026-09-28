//! Bomb and bomb jump: ftsamusspeciallw1.c (the Bomb rows 355/356,
//! 8012ADF0..8012B6E8) and ftsamusspeciallw0.c (the launched ball, rows
//! 341/342, and ftSs_Init_80128944's launch).
//!
//! Down special curls Samus into the ball; the script's throw flag drops a
//! bomb from TopN (ftSs_SpecialLw_8012ADF0). While the script holds
//! cmd_vars[0] she is the ball (one capsule on XRotN) and collides with its
//! box. A blast whose capsule meets her launches her up and away from it
//! (ftSs_Init_80128944) unless she is intangible or in a motion class that
//! keeps its ground.
use crate::{
    common::{self, change},
    escape::{become_ball, leave_ball},
    init::{Accessory, Samus},
};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{
            callbacks, class_nibble, AnimationPhase, CollisionPhase, InputPhase, MotionRow,
            PhysicsPhase, COMMON_MOTION_FLAGS,
        },
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemRequest, OwnerBlast, SpawnItem};
use melee_types::{CommonMotionState, FtPart, GroundOrAir, ItemKind};

/// ftSs_MS_SpecialLw (341) / SpecialAirLw (342): the launched ball.
pub const LAUNCHED: ActionId = ActionId(341);
pub const AIR_LAUNCHED: ActionId = ActionId(342);
/// ftSs_MS_SpecialLwBomb (355) / SpecialAirLwBomb (356).
pub const BOMB: ActionId = ActionId(355);
pub const AIR_BOMB: ActionId = ActionId(356);

/// ftSs_MF_SpecialLw_Coll (ftsamusspeciallw1.c:24): ftCommon_GroundAirColl_MF
/// | KeepColAnimHitStatus | SkipHit | SkipModel.
const BOMB_GROUND_AIR: MotionEntryFlags = MotionEntryFlags(common::GROUND_AIR.0 | 0x1C);
/// ftSs_SpecialLw_80129048 / 801290A4: Ft_MF_SkipModel alone.
const SKIP_MODEL: MotionEntryFlags = MotionEntryFlags::SKIP_MODEL;
/// ftSs_SpecialLw_Enter: a bomb from SquatWait (0x28) starts three frames in.
const SQUAT_START_FRAME: f32 = 3.0;
/// cmd_vars[1]: the script's hop cue (1), and hopped or no hop to come (2).
const HOP_CUED: u32 = 1;
const HOPPED: u32 = 2;
/// ftSs_Init_80128944: the ball-model selection that starts the launch late.
const BALL_MODEL: i32 = 2;
/// ftSs_Init_80128944: an x2073 attack id that always allows the launch.
const BOMB_ATTACK_ID: u32 = 0x14;
/// ft_800895E0: an x2073 of 0x62 with a held item becomes 0x44003D.
const ITEM_ATTACK_ID: u32 = 0x62;
/// x2071_b5: bit 18 of the flags word.
const X2071_B5: u32 = 1 << 18;
/// ftSs_Init_80128AC8: the launch angle's centre, (float) pi/2.
const VERTICAL: f32 = 1.570_796_4;

pub const fn rows() -> [MotionRow; 4] {
    [
        common::row(
            LAUNCHED,
            launched_anim,
            launched_input,
            launched_physics,
            launched_collision,
        ),
        common::row(
            AIR_LAUNCHED,
            air_launched_anim,
            callbacks::input::aerial,
            air_launched_physics,
            air_launched_collision,
        ),
        common::row(BOMB, bomb_anim, bomb_input, bomb_physics, bomb_collision),
        common::row(
            AIR_BOMB,
            air_bomb_anim,
            common::no_input,
            air_bomb_physics,
            air_bomb_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::SamusAttributes {
    &f.character.get::<Samus>().attributes
}

/// ftSamus_SpecialLw_StartAction_inner: the script's words and the throw
/// flag clear, Samus is not yet the ball, a start on frame 3 is already
/// cued to hop, and accessory4 drops the bomb.
fn start(f: &mut Fighter) {
    f.commands.variables[2] = 0;
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    f.commands.throw_accessory = false;
    f.character.get_mut::<Samus>().ball = false;
    if f.animation.frame == SQUAT_START_FRAME {
        f.commands.variables[1] = HOP_CUED;
    }
    arm_drop(f);
}

/// accessory4_cb = ftSs_SpecialLw_8012ADF0.
fn arm_drop(f: &mut Fighter) {
    f.character.get_mut::<Samus>().accessory = Accessory::Bomb;
    f.core.arm_accessory4();
}

/// ftSs_SpecialLw_Enter (8012AFD4) / ftSs_SpecialAirLw_Enter (8012B0D0).
/// On the ground the speed scales by x6C; from SquatWait the bomb starts three
/// frames in and hops at once. In the air x speed scales by x70 and the y
/// speed becomes x58.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let bomb = attributes(f).bomb.clone();
    if air {
        f.physics.self_velocity.x *= bomb.air_velocity_scale;
        f.physics.self_velocity.y = bomb.air_launch_y;
        change(f, AIR_BOMB, MotionEntryFlags(0), 0.0, 1.0, a).expect("Bomb assets");
        // ftAnim_8006EBA4.
        f.step_animation(a);
        start(f);
        return;
    }
    f.physics.ground_velocity *= bomb.ground_velocity_scale;
    if f.motion_state.action == CommonMotionState::SquatWait.into() {
        change(f, BOMB, MotionEntryFlags(0), SQUAT_START_FRAME, 1.0, a).expect("Bomb assets");
        start(f);
        f.commands.variables[1] = HOPPED;
        hop(f, a).expect("Bomb hop assets");
        return;
    }
    change(f, BOMB, MotionEntryFlags(0), 0.0, 1.0, a).expect("Bomb assets");
    f.step_animation(a);
    start(f);
}

/// ftSs_SpecialLw_8012ADF0 (8012ADF0): on the script's throw flag a bomb
/// drops from TopN plus x74 (x facing-scaled: 8012AE68 fmadds), and the
/// accessory goes.
pub fn drop_bomb(f: &mut Fighter) {
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let offset = attributes(f).bomb.spawn_offset;
    let c = &mut f.core;
    let top = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        common::part(FtPart::TopN),
        Vec3::ZERO,
    );
    let facing = c.physics.facing;
    let position = Vec3::new(
        gekko_math::fma::fmadds(offset.x, facing, top.x),
        top.y + offset.y,
        top.z + offset.z,
    );
    // it_802B4AC8: prev_pos is the drop point on the stage plane; pos is
    // it_8026BB68's ECB midpoint.
    let mut spawn = SpawnItem::ray(ItemKind::SamusBomb, c.player.id, position, facing);
    spawn.previous_position.z = 0.0;
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    // fp->accessory4_cb = NULL.
    c.accessory4_armed = false;
}

/// ftSs_SpecialLw_8012B5F0 (8012B5F0): the hop, x54 up into the aerial Bomb
/// row, dropping the bomb still to come.
fn hop(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    f.physics.self_velocity.y = attributes(f).bomb.ground_hop;
    common::ground_to_air(f, AIR_BOMB, BOMB_GROUND_AIR, a)?;
    arm_drop(f);
    Ok(())
}

/// ftSamus_UnkSetStateAndCb: after a ground/air change, no hop to come, the
/// ball's capsule to reinstall, and the bomb still to drop.
fn after_ground_air(f: &mut Fighter) {
    f.commands.variables[1] = HOPPED;
    f.character.get_mut::<Samus>().ball = false;
    arm_drop(f);
}

/// checkStateVar1 / ftSs_SpecialLw_Anim's head: the ball's capsule follows
/// cmd_vars[0] (ftSs_SpecialLw_8012AEBC / ftColl_8007B0C0).
fn follow_ball(f: &mut Fighter) {
    let script = f.commands.variables[0] != 0;
    let ball = f.character.get::<Samus>().ball;
    if script && !ball {
        become_ball(f);
        f.character.get_mut::<Samus>().ball = true;
    }
    if !script && f.character.get::<Samus>().ball {
        leave_ball(f);
        f.character.get_mut::<Samus>().ball = false;
    }
}

/// ftSs_SpecialLwBomb_Anim (8012B294): a cued hop, else the ball, and Wait
/// at the end.
fn bomb_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[1] == HOP_CUED {
        f.commands.variables[1] = HOPPED;
        hop(f, p.assets)?;
        return Ok(None);
    }
    follow_ball(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialAirLwBomb_Anim (8012B33C): the ball, and Fall at the end.
fn air_bomb_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    follow_ball(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialLwBomb_IASA (8012B3D4): once the script allows it
/// (cmd_vars[2]), the stick below x80 crouches (ftCo_800D638C).
fn bomb_input(f: &mut Fighter, p: InputPhase<'_>) {
    let threshold = attributes(f).bomb.squat_stick_y;
    if f.commands.variables[2] != 0 && f.input.current.stick.y < threshold {
        f.commands.variables[2] = 0;
        f.enter_squat_wait(p.assets).expect("SquatWait assets");
    }
}

/// ftSs_SpecialLwBomb_Phys (8012B420): the ball rolls toward the stick
/// (ftCommon_8007CADC with x64 / x5C), otherwise ground friction.
fn bomb_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] == 0 {
        common::ground_friction(f, p);
        return;
    }
    let bomb = &attributes(f).bomb;
    let walking = &f.core.attributes.walking;
    let (acceleration, target) = (
        walking.walk_accel_mul * bomb.ground_accel,
        walking.walk_max_vel * bomb.ground_max,
    );
    common::walk_toward_stick(f, 0.0, acceleration, target);
    common::move_on_ground(f, &p);
}

/// ftSs_SpecialAirLwBomb_Phys (8012B488): gravity, then drift toward the
/// stick without friction (ftCommon_8007D3A8 with x68 / x60).
fn air_bomb_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::fall_basic(f);
    let bomb = &attributes(f).bomb;
    let air = &f.core.attributes.air;
    let (acceleration, target) = (
        air.air_drift_stick_mul * bomb.air_accel,
        air.air_drift_max * bomb.air_max,
    );
    common::drift_without_friction(f, 0.0, acceleration, target);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftSs_SpecialLwBomb_Coll (8012B4E8): the ball's box (ft_80082888) or the
/// edge-stopping pass (ft_800827A0); off the floor, the aerial Bomb
/// (ftSs_SpecialLw_8012B570).
fn bomb_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let supported = if f.commands.variables[0] != 0 {
        let ecb = attributes(f).morph_ball_box;
        common::stays_grounded_in_box(f, &mut p, ecb)
    } else {
        common::stays_on_edge(f, &mut p)
    };
    if !supported {
        let a = p.assets.expect("Bomb collision assets");
        common::ground_to_air(f, AIR_BOMB, BOMB_GROUND_AIR, a)?;
        after_ground_air(f);
    }
    Ok(())
}

/// ftSs_SpecialAirLwBomb_Coll (8012B52C): the ball's box (ft_800824A0) or
/// ft_80081D0C; a landing is the grounded Bomb (ftSs_SpecialLw_8012B668).
fn air_bomb_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let landed = if f.commands.variables[0] != 0 {
        let ecb = attributes(f).morph_ball_box;
        common::lands_in_box(f, &mut p, ecb)
    } else {
        common::lands(f, &mut p)
    };
    if landed {
        let a = p.assets.expect("Bomb landing assets");
        common::air_to_ground(f, BOMB, BOMB_GROUND_AIR, a)?;
        after_ground_air(f);
    }
    Ok(())
}

/// ftSs_Init_80128A1C (80128A1C), then ftSs_Init_80128944 (80128944): a
/// blast touching any of Samus's capsules launches her, unless she is
/// intangible or invincible (ftColl_8007B868) or her motion's class
/// (x2071_b0_3) is 1 or above 4, or it is x2071_b5 without the bomb's
/// attack id.
pub fn take_blast(f: &mut Fighter, blast: &OwnerBlast, a: &FighterAssets) {
    if !f
        .core
        .hurt_capsules_touched(blast.previous, blast.position, blast.contact_radius)
    {
        return;
    }
    let angle = launch_angle(f, blast.x, blast.range);
    if untouchable(f) {
        return;
    }
    let flags = motion_flags(f);
    if !matches!(class_nibble(flags), 0 | 2 | 3 | 4) {
        return;
    }
    if attack_id(flags) != BOMB_ATTACK_ID && flags & X2071_B5 != 0 {
        return;
    }
    let start = if f.commands.model_selections.get(&0) == Some(&BALL_MODEL) {
        attributes(f).bomb_jump.late_start_frame
    } else {
        0.0
    };
    launch(f, angle, start, a).expect("bomb jump assets");
}

/// ftSs_Init_80128AC8 (80128AC8): pi/2 less x4 times Samus's offset from the
/// blast over its range, clamped to -1..1 (80128B14: fmadds).
fn launch_angle(f: &Fighter, x: f32, range: f32) -> f32 {
    // Both retail tests pass a NaN through, as clamp does.
    let offset = ((f.physics.position.x - x) / range).clamp(-1.0, 1.0);
    let spread = attributes(f).bomb_jump.angle_range;
    gekko_math::fma::fmadds(-spread, offset, VERTICAL)
}

/// ftColl_8007B868 (8007B868): the script's body state (x1988), timed
/// intangibility or invincibility (x198C). x221D_b6 (an item's
/// invincibility) has no ported source.
fn untouchable(f: &Fighter) -> bool {
    f.commands.hurt_status != melee_types::combat::HurtStatus::Normal
        || f.status.ledge_intangibility != 0
        || f.status.revival_invincibility != 0
}

/// fp->x2070: the current row's MotionState.x4_flags (ft_800895E0).
fn motion_flags(f: &Fighter) -> u32 {
    let action = usize::from(f.motion_state.action.0);
    let flags = if action < COMMON_MOTION_FLAGS.len() {
        COMMON_MOTION_FLAGS[action]
    } else {
        crate::MOTION_FLAGS[action - COMMON_MOTION_FLAGS.len()]
    };
    if attack_id(flags) == ITEM_ATTACK_ID && f.core.held_item.is_some() {
        unimplemented!("ft_800895E0: x2073 0x62 with a held item");
    }
    flags
}

/// x2073: the flags word's low byte.
fn attack_id(flags: u32) -> u32 {
    flags & 0xFF
}

/// ftSs_Init_80128B1C (80128B1C): the take-damage callbacks run
/// (ftCommon_8007DB58), the ball flies at x8 along `angle` with its x
/// clamped to x10 times the air drift maximum, the script's words and the
/// ball clear, and the aerial launch row starts at `start`.
fn launch(f: &mut Fighter, angle: f32, start: f32, a: &FighterAssets) -> Result<()> {
    f.interrupt_actions();
    let jump = attributes(f).bomb_jump.clone();
    f.physics.self_velocity.x = jump.speed * gekko_math::msl::cosf(angle);
    f.physics.self_velocity.y = jump.speed * gekko_math::msl::sinf(angle);
    let maximum = f.core.attributes.air.air_drift_max * jump.air_mobility;
    common::clamp_self_velocity_x(f, maximum);
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.character.get_mut::<Samus>().ball = false;
    if f.physics.ground_or_air == GroundOrAir::Ground {
        f.leave_ground();
    }
    change(f, AIR_LAUNCHED, MotionEntryFlags(0), start, 1.0, a)?;
    f.step_animation(a);
    Ok(())
}

/// ftSs_SpecialLw_Anim (80128C04): the ball, and Wait at the end.
fn launched_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    follow_ball(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialAirLw_Anim (80128CA0): the ball, and Fall at the end.
fn air_launched_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    follow_ball(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialLw_IASA (80128D3C): once the script allows it
/// (cmd_vars[1]), the stick below x14 crouches (ftCo_800D638C); otherwise
/// ftCo_Wait_IASA's specials, grab, attacks and roll.
fn launched_input(f: &mut Fighter, p: InputPhase<'_>) {
    let threshold = attributes(f).bomb_jump.squat_stick_y;
    if f.commands.variables[1] != 0 && f.input.current.stick.y < threshold {
        f.commands.variables[1] = 0;
        f.enter_squat_wait(p.assets).expect("SquatWait assets");
        return;
    }
    callbacks::input::standing_attacks(f, p);
}

/// ftSs_SpecialLw_Phys (80128E68): the ball rolls toward the stick
/// (ftCommon_8007CADC with xC), otherwise ground friction.
fn launched_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] == 0 {
        common::ground_friction(f, p);
        return;
    }
    let mobility = attributes(f).bomb_jump.ground_mobility;
    let walking = &f.core.attributes.walking;
    let (acceleration, target) = (
        walking.walk_accel_mul * mobility,
        walking.walk_max_vel * mobility,
    );
    common::walk_toward_stick(f, 0.0, acceleration, target);
    common::move_on_ground(f, &p);
}

/// ftSs_SpecialAirLw_Phys (80128EE0): gravity, then drift toward the stick
/// with the aerial friction (ftCommon_8007D344 with x10 on the jump's
/// momentum multiplier and maximum).
fn air_launched_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::fall_basic(f);
    let mobility = attributes(f).bomb_jump.air_mobility;
    let jumping = &f.core.attributes.jumping;
    let (acceleration, target) = (
        jumping.ground_to_air_jump_momentum_multiplier * mobility,
        jumping.jump_h_max_velocity * mobility,
    );
    common::drift_with_friction(f, 0.0, acceleration, target);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftSs_SpecialLw_Coll (80128F48): the ball's box or the edge-stopping
/// pass; off the floor, the aerial row at the same frame and rate
/// (ftSs_SpecialLw_80129048, Ft_MF_SkipModel).
fn launched_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let supported = if f.commands.variables[0] != 0 {
        let ecb = attributes(f).morph_ball_box;
        common::stays_grounded_in_box(f, &mut p, ecb)
    } else {
        common::stays_on_edge(f, &mut p)
    };
    if !supported {
        let a = p.assets.expect("bomb jump collision assets");
        f.leave_ground();
        let (frame, rate) = (f.animation.frame, f.animation.speed);
        change(f, AIR_LAUNCHED, SKIP_MODEL, frame, rate, a)?;
    }
    Ok(())
}

/// ftSs_SpecialAirLw_Coll (80128FC8): the ball's box or ft_80081D0C; a
/// landing is the grounded row at the same frame and rate
/// (ftSs_SpecialLw_801290A4, Ft_MF_SkipModel).
fn air_launched_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let landed = if f.commands.variables[0] != 0 {
        let ecb = attributes(f).morph_ball_box;
        common::lands_in_box(f, &mut p, ecb)
    } else {
        common::lands(f, &mut p)
    };
    if landed {
        let a = p.assets.expect("bomb jump landing assets");
        f.land();
        let (frame, rate) = (f.animation.frame, f.animation.speed);
        change(f, LAUNCHED, SKIP_MODEL, frame, rate, a)?;
    }
    Ok(())
}
