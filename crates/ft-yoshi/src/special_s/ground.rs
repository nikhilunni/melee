//! The grounded Egg Roll states: start (356), loop (357), turn (358) and
//! the break (359).
use super::{
    assets, attributes, b_breaks, change, count_down, end, model, part, roll, step,
    touchdown_sound, touched_wall, AnimResult, BREAK_FLAGS, EGG_BOX, GROUND_AIR_FLAGS, LOOP_AIR,
    LOOP_AIR_BOUNCE, LOOP_FLAGS, LOOP_GROUND, START_AIR, TURN_GROUND,
};
use gekko_math::{fma, msl::fabsf};
use melee_ft::{
    collision::{air::begin_map, ground},
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionEntryFlags,
    },
    physics::{friction, grounded},
};
use melee_types::CommonMotionState;

/// The loop's grounded-to-aerial change: GROUND_AIR_FLAGS | SkipHit.
const ROLL_OFF_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_509A);
/// The turn's grounded-to-aerial change: ROLL_OFF_FLAGS without SkipHit,
/// with SkipRumble.
const TURN_OFF_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5892);
/// The turn's entry: BREAK_FLAGS | SkipRumble.
const TURN_FLAGS: MotionEntryFlags = MotionEntryFlags(0x004C_4892);
/// The turn's return to the loop: LOOP_FLAGS | SkipRumble | SkipAttackCount.
const UNTURN_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0244_0812);
/// @421: 2pi / 25 in double precision, the roll angle per unit speed.
const ROLL_STEP: f64 = 0.25132741228718347;
/// @422: the turn's reversal point, as a fraction of its entry speed.
const TURN_FRACTION: f32 = 0.7;
/// @426: the turn's deceleration per unit of entry speed.
const TURN_DECELERATION: f32 = -0.05;
/// @423 / @310 / @424: pi, pi/2 and 3pi/2 in double precision.
const PI: f64 = std::f64::consts::PI;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const THREE_HALF_PI: f64 = 4.71238898038469;
/// efSync_Spawn id of the turn's rolling dust (model 5).
const ROLL_DUST: u16 = 0x3FF;

/// ftYs_SpecialAirSStart_0_Anim (8012F658): the grounded start hops into
/// the aerial loop at frame 0 with a frozen animation.
pub(super) fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    roll(f).pending_facing = 0.0;
    model::step_shell(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.leave_ground();
        change(f, LOOP_AIR, LOOP_FLAGS, 0.0, 0.0, p.assets)?;
        f.animation.frame = 0.0;
        // Retail 8012F704 fmuls.
        let velocity = roll(f).speed * f.physics.facing;
        f.physics.self_velocity.x = velocity;
        f.physics.ground_velocity = velocity;
        f.physics.animation_velocity.x = 0.0;
        f.physics.ground_acceleration = 0.0;
        let a = attributes(f);
        let (hop, gravity) = (a.hop_speed, a.start_gravity);
        f.physics.self_velocity.y = hop;
        f.physics.animation_velocity.y = gravity;
    }
    Ok(None)
}

/// ftYs_SpecialAirSStart_0_Phys (80130E38): held still.
pub(super) fn start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let physics = &mut f.physics;
    physics.ground_acceleration = 0.0;
    physics.ground_velocity = 0.0;
    physics.self_velocity.y = 0.0;
    physics.self_velocity.x = 0.0;
    physics.animation_velocity.y = 0.0;
    physics.animation_velocity.x = 0.0;
    finish_ground(f, &p);
}

/// Fighter_procUpdate's grounded tail after a physics callback that set
/// the velocities itself.
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

/// ftCommon_ApplyGroundMovement (8007CB74).
fn apply_ground_movement(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let core = &mut f.core;
    grounded::apply_ground_movement(
        &mut core.physics,
        core.collision.data.floor.normal,
        p.map.floor_speed_scale(&core.collision.data),
    );
}

/// ftCommon_ApplyGroundMovement, then the grounded tail.
fn move_on_ground(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    apply_ground_movement(f, p);
    finish_ground(f, p);
}

/// ft_80082708: the ordinary grounded pass. False when the floor is lost.
fn stays_grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ftYs_SpecialAirSStart_0_Coll (8013182C): off an edge, back to the
/// aerial start (ftCommon_GroundToAirStateChange).
pub(super) fn start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !stays_grounded(f, &mut p) {
        f.leave_ground();
        let frame = f.animation.frame;
        change(f, START_AIR, GROUND_AIR_FLAGS, frame, 1.0, assets(&p))?;
    }
    Ok(())
}

/// ftYs_SpecialAirSLoop_0_Anim (8012F79C).
pub(super) fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    roll(f).pending_facing = 0.0;
    model::step_shell(f);
    model::squash(f);
    model::update_hitbox(f);
    // Retail 8012FA0C fmuls, 8012FA14 fmadd (double), frsp.
    let amount = attributes(f).rolling_rotation_multiplier * fabsf(roll(f).speed);
    model::advance_roll(f, ROLL_STEP, amount);
    model::wrap_roll(f);
    count_down(f, false, p.assets)?;
    Ok(None)
}

/// ftYs_SpecialAirSLoop_0_IASA (801308F8): after a grounded tick, holding
/// the stick against the facing turns; after the minimum roll, B breaks.
pub(super) fn loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    if roll(f).ground_ticks != 0 {
        let stick = f.input.current.stick.x;
        if fabsf(stick) > attributes(f).steer_threshold {
            let direction = if stick > 0.0 { 1.0 } else { -1.0 };
            if f.physics.facing != direction {
                begin_turn(f, direction, p.assets).expect("Egg Roll turn");
            }
        }
    }
    if b_breaks(f) {
        roll(f).pending_facing = 0.0;
        end(f, false, LOOP_FLAGS, 0.0, p.assets).expect("Egg Roll break");
    }
}

/// The turn's entry from the loop IASA.
fn begin_turn(f: &mut Fighter, direction: f32, assets: &FighterAssets) -> Result<()> {
    // ftColl_8007AFF8: every hitbox off.
    f.commands.hitboxes.fill(None);
    roll(f).pending_facing = direction;
    let frame = f.animation.frame;
    change(f, TURN_GROUND, TURN_FLAGS, frame, 0.0, assets)?;
    let speed = f.physics.ground_velocity;
    let scratch = roll(f);
    scratch.turn_speed = speed;
    scratch.tilt = 0.0;
    // Retail fmuls.
    scratch.turn_acceleration = TURN_DECELERATION * speed;
    scratch.dust_timer = 0;
    model::wrap_roll(f);
    Ok(())
}

/// ftYs_SpecialAirSLoop_0_Phys (801310E8): accelerate toward the target
/// speed, scaled up the slope the roll climbs and down the one it descends.
pub(super) fn loop_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let normal = f.collision.data.floor.normal;
    let velocity = f.physics.ground_velocity;
    let a = attributes(f);
    let (slope_multiplier, max, target, accel, decel) = (
        a.slope_multiplier,
        a.ground_maximum_speed,
        a.ground_target_speed,
        a.ground_acceleration,
        a.ground_deceleration,
    );
    let (speed_scale, deceleration_scale) = slope_scales(normal, velocity, slope_multiplier);
    // Retail 801311DC..EC: four fmuls.
    let max = max * speed_scale;
    let target = target * speed_scale;
    let accel = accel * speed_scale;
    let decel = decel * deceleration_scale;
    let speed = fabsf(velocity);
    let mut delta = if speed >= target {
        let delta = -decel;
        // Retail 80131210 fadds; the clamp is retail's (a positive step).
        if speed + delta < target {
            speed - target
        } else {
            delta
        }
    } else if speed + accel > target {
        target - speed
    } else {
        accel
    };
    if velocity < 0.0 {
        delta = -delta;
    }
    let terrain = melee_mp::terrain_speed_scale(f.collision.data.floor.flags);
    // Retail 80131258 fmadds.
    let physics = &mut f.physics;
    physics.ground_velocity = fma::fmadds(delta, terrain, physics.ground_velocity);
    if fabsf(physics.ground_velocity) > max {
        physics.ground_velocity = if physics.ground_velocity < 0.0 {
            -max
        } else {
            max
        };
    }
    let speed = fabsf(physics.ground_velocity);
    physics.animation_velocity.y = 0.0;
    physics.self_velocity.y = 0.0;
    roll(f).speed = speed;
    move_on_ground(f, &p);
}

/// The loop's speed and deceleration factors from the floor's slope.
fn slope_scales(normal: hsd_types::Vec3, velocity: f32, slope_multiplier: f32) -> (f32, f32) {
    let mut direction: f32 = if normal.x > 0.0 { 1.0 } else { -1.0 };
    let climbing = if velocity > 0.0 {
        normal.x > 0.0
    } else {
        normal.x < 0.0
    };
    if velocity <= 0.0 && climbing {
        direction = -direction;
    }
    let slope = 1.0 - normal.y;
    if climbing {
        // Retail 80131150 fmuls, 80131158 fmadds, 8013115C fnmsubs.
        let tilted = direction * slope_multiplier;
        (
            fma::fmadds(slope, tilted, 1.0),
            fma::fnmsubs(slope, direction, 1.0),
        )
    } else {
        let tilt = if velocity > 0.0 {
            // Retail 80131170 fmuls.
            slope * direction
        } else {
            // Retail 801311BC fmuls by the negated direction.
            slope * -direction
        };
        (1.0 + tilt, 1.0 - tilt)
    }
}

/// ftYs_SpecialAirSLoop_0_Coll (80131870): a wall breaks the egg into the
/// air; rolling off the floor keeps rolling in the air.
pub(super) fn loop_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let supported = collide_box(f, &mut p, false);
    let toward_right = f.physics.facing == 1.0;
    if touched_wall(f, toward_right) {
        f.leave_ground();
        super::bounce_off_wall(f, assets(&p))?;
    } else if !supported {
        f.leave_ground();
        let frame = f.animation.frame;
        change(f, LOOP_AIR_BOUNCE, ROLL_OFF_FLAGS, frame, 0.0, assets(&p))?;
        model::wrap_roll(f);
    }
    roll(f).ground_ticks += 1;
    Ok(())
}

/// ft_80082888 / ft_80082978 with the egg's box, after Fighter_procMap's
/// prologue.
fn collide_box(f: &mut Fighter, p: &mut CollisionPhase<'_>, stop_at_edge: bool) -> bool {
    let c = &mut f.core;
    begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    ground::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        EGG_BOX,
        stop_at_edge,
    )
}

/// ftYs_SpecialAirSLoop_1_Anim (8012FAC4): spin, tilt and swing the model
/// round; every few frames the dust.
pub(super) fn turn_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    model::step_shell(f);
    model::squash(f);
    // Retail 8012FC88 fmadd (double), frsp.
    let rotation = attributes(f).rotation_speed;
    model::advance_roll(f, ROLL_STEP, rotation);
    model::wrap_roll(f);
    tilt(f);
    swing(f);
    if count_down(f, false, p.assets)? {
        return Ok(None);
    }
    let interval = attributes(f).effect_interval;
    if roll(f).dust_timer % interval == 0 {
        roll_dust(f);
    }
    roll(f).dust_timer += 1;
    Ok(None)
}

/// ftYs_SpecialS_UpdateLoop1Rotation: lean into the turn about part 2.
fn tilt(f: &mut Fighter) {
    let turn = roll(f).turn_speed;
    let velocity = f.physics.ground_velocity;
    let maximum = attributes(f).maximum_tilt;
    let (turn_abs, speed, maximum_abs) = (fabsf(turn), fabsf(velocity), fabsf(maximum));
    let same_direction = if turn > 0.0 {
        velocity > 0.0
    } else {
        velocity < 0.0
    };
    // Retail 8012FD54 fdivs / 8012FD70 fmuls then fdivs; fmadds.
    let ratio = if same_direction {
        speed / turn_abs
    } else {
        speed / (turn_abs * TURN_FRACTION)
    };
    let tilt = fma::fmadds(maximum_abs, ratio, -maximum);
    roll(f).tilt = tilt;
    f.core.set_part_rotation(part::TILT, Axis::Z, tilt);
}

/// ftYs_SpecialS_UpdateLoop1Rotation2: swing the facing about part 0 from
/// one side to the other as the speed reverses.
fn swing(f: &mut Fighter) {
    let turn = roll(f).turn_speed;
    let velocity = f.physics.ground_velocity;
    let turn_abs = fabsf(turn);
    // Retail 8012FE08 fmadds: 0.7 |x10| + |x10|.
    let total = fma::fmadds(TURN_FRACTION, turn_abs, turn_abs);
    let (same_direction, offset) = if turn > 0.0 {
        (velocity > 0.0, HALF_PI)
    } else {
        (velocity < 0.0, THREE_HALF_PI)
    };
    let travelled = if same_direction {
        turn_abs - fabsf(velocity)
    } else {
        turn_abs + fabsf(velocity)
    };
    // Retail 8012FE58 fdivs, 8012FE64 fmadd (double), frsp.
    let angle = fma::fmadd(PI, f64::from(travelled / total), offset) as f32;
    let angle = model::wrap(angle);
    f.core.set_part_rotation(part::FACING, Axis::Y, angle);
}

/// efSync_Spawn(0x3FF, gobj, &cur_pos, &direction, &floor_angle).
fn roll_dust(f: &mut Fighter) {
    let normal = f.collision.data.floor.normal;
    let facing = if f.physics.ground_velocity < 0.0 {
        -1.0
    } else {
        1.0
    };
    let angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
    let position = f.physics.position;
    f.effects
        .push(melee_ef::request::EffectRequest::PositionalGraphics {
            id: ROLL_DUST,
            position,
            facing,
            angle,
        });
}

/// ftYs_SpecialAirSLoop_1_IASA (80130C04): B after the minimum roll.
pub(super) fn turn_input(f: &mut Fighter, p: InputPhase<'_>) {
    if b_breaks(f) {
        end(f, false, LOOP_FLAGS, 0.0, p.assets).expect("Egg Roll break");
    }
}

/// ftYs_SpecialAirSLoop_1_Phys (801312EC): decelerate through zero; once
/// the speed reverses past 70% of the entry speed, roll on (x21EC starts a
/// new attack instance inside the state change).
pub(super) fn turn_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let terrain = melee_mp::terrain_speed_scale(f.collision.data.floor.flags);
    let acceleration = roll(f).turn_acceleration;
    // Retail 8013131C fmadds.
    f.physics.ground_velocity = fma::fmadds(acceleration, terrain, f.physics.ground_velocity);
    f.physics.animation_velocity.y = 0.0;
    f.physics.self_velocity.y = 0.0;
    apply_ground_movement(f, &p);
    let turn = roll(f).turn_speed;
    let velocity = f.physics.ground_velocity;
    let reversed = if turn > 0.0 {
        velocity < 0.0
    } else {
        velocity > 0.0
    };
    // Retail fmuls 0.7 * |x10|, fcmpo.
    if reversed && fabsf(velocity) > TURN_FRACTION * fabsf(turn) {
        finish_turn(f, p.assets).expect("Egg Roll turn end");
    }
    // Fighter_procUpdate integrates after the callback's state change.
    finish_ground(f, &p);
}

/// The turn's return to the loop, facing the new way.
fn finish_turn(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    // fn_8012EFC0 (x21EC): ft_80089824's statistics, then ft_800892A0.
    f.combat.stale.new_instance();
    let frame = f.animation.frame;
    change(f, LOOP_GROUND, UNTURN_FLAGS, frame, 0.0, assets)?;
    touchdown_sound(f);
    let pending = roll(f).pending_facing;
    if pending != 0.0 {
        f.physics.facing = pending;
    }
    let scratch = roll(f);
    scratch.pending_facing = 0.0;
    scratch.group_timer = 0;
    model::face(f);
    f.core.set_part_rotation(part::TILT, Axis::Z, 0.0);
    model::wrap_roll(f);
    Ok(())
}

/// ftYs_SpecialAirSLoop_1_Coll (80131B64): fast turns ignore floor edges.
pub(super) fn turn_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let fast = fabsf(f.physics.ground_velocity) > attributes(f).edge_ignore_speed;
    let supported = collide_box(f, &mut p, !fast);
    let toward_right = f.physics.self_velocity.x > 0.0;
    if touched_wall(f, toward_right) {
        f.leave_ground();
        super::bounce_off_wall(f, assets(&p))?;
    } else if !supported {
        f.leave_ground();
        let frame = f.animation.frame;
        change(f, LOOP_AIR_BOUNCE, TURN_OFF_FLAGS, frame, 0.0, assets(&p))?;
        let pending = roll(f).pending_facing;
        if pending != 0.0 {
            f.physics.facing = pending;
            let speed = fabsf(f.physics.self_velocity.x);
            roll(f).speed = speed;
        }
        roll(f).pending_facing = 0.0;
        model::face(f);
        f.core.set_part_rotation(part::TILT, Axis::Z, 0.0);
    }
    Ok(())
}

/// ftYs_SpecialAirSEnd_Anim (8012FFF4): the shell bursts once the script
/// counts a restart; at the end, Wait.
pub(super) fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    roll(f).pending_facing = 0.0;
    super::air::burst_on_script(f);
    model::step_shell(f);
    model::squash(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        model::restore(f);
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// ftYs_SpecialAirSEnd_Phys (80130E5C): ground friction.
pub(super) fn end_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let friction = f.attributes.ground.ground_friction;
    f.physics.ground_acceleration =
        friction::friction_acceleration(f.physics.ground_velocity, friction);
    move_on_ground(f, &p);
}

/// ftYs_SpecialAirSEnd_Coll (80131E40): off an edge, the aerial break at
/// the same frame.
pub(super) fn end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !stays_grounded(f, &mut p) {
        f.leave_ground();
        let frame = f.animation.frame;
        end(f, true, BREAK_FLAGS, frame, assets(&p))?;
    }
    Ok(())
}
