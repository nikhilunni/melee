//! The roll: Release (350, 358) and Turn (351, 359). Puff rolls at a
//! speed the charge sets; the charge runs down, walls bounce her back, the
//! stick against the roll turns her round.
use super::{
    attributes, change,
    charge::{assets, fall, finish_ground, DEGREES_TO_RADIANS},
    end, flags, model, scratch, AIR_RELEASE, AIR_TURN, RELEASE, ROLL_BOX, TURN,
};
use gekko_math::{fma, msl::fabsf};
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter,
    },
    physics::grounded,
};

type AnimResult = Result<Option<WaitChoice>>;

/// @232: the rotation scale for a roll's speed attributes, in double.
const ROTATION_SCALE: f64 = 0.2;
/// @222 / @233 / @234: pi/2, 3pi/2 and pi in double precision.
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const THREE_HALF_PI: f64 = 4.71238898038469;
const PI: f64 = std::f64::consts::PI;
/// ftPr_SpecialNRelease_IASA: the turn's deceleration per unit of speed.
const TURN_DECELERATION: f32 = -0.05;

/// SIGNF: positive is right, anything else left.
fn sign(value: f32) -> f32 {
    if value > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// ftPr_SpecialNRelease_Anim (8013E410): squash, hitbox upkeep and roll;
/// once the roll time is spent, stop when the ball turns face-down.
pub(super) fn release_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox(f);
    f.step_animation(p.assets);
    model::roll_sound(f, false);
    scratch(f).pending_facing = 0.0;
    model::squash(f);
    model::toggle_group(f);
    model::update_hitbox(f, p.assets);
    let a = attributes(f);
    let rotation = a.release_rotation_speed;
    let rollout = scratch(f);
    let previous = rollout.roll_angle;
    // 8013E640 fmul, 8013E654 fmul (double), frsp; 8013E668 fmuls,
    // 8013E66C fmuls, 8013E670 fadds.
    let scale = (ROTATION_SCALE * f64::from(rotation) * f64::from(rollout.direction)) as f32;
    let delta = DEGREES_TO_RADIANS * rollout.charge as f32 * scale;
    rollout.roll_angle = previous + delta;
    model::set_roll(f);
    stop_face_down(f, previous, delta, false, p.assets)?;
    Ok(None)
}

/// The shared tail of both release anims: count the roll down; once
/// spent, when the angle crosses pi (the ball face-down) the roll ends.
fn stop_face_down(
    f: &mut Fighter,
    previous: f32,
    delta: f32,
    air: bool,
    assets: &FighterAssets,
) -> Result<()> {
    let rollout = scratch(f);
    rollout.remaining -= 1;
    if rollout.remaining <= 0 {
        let angle = f64::from(rollout.roll_angle);
        if HALF_PI < angle && angle < THREE_HALF_PI {
            let crossed = if delta > 0.0 {
                angle > PI && f64::from(previous) < PI
            } else {
                angle < PI && f64::from(previous) > PI
            };
            if crossed {
                rollout.remaining = 0;
                return end(f, air, flags::START, 0.0, assets);
            }
        }
    }
    model::face_forward(f);
    Ok(())
}

/// ftPr_SpecialAirNChargeRelease_Anim (8013F1A4): as on the ground, the
/// roll's angular speed scaled by the aerial multiplier.
pub(super) fn air_release_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox(f);
    f.step_animation(p.assets);
    model::roll_sound(f, false);
    scratch(f).pending_facing = 0.0;
    model::squash(f);
    model::toggle_group(f);
    model::update_hitbox(f, p.assets);
    let a = attributes(f);
    let (rotation, aerial) = (a.release_rotation_speed, a.air_rotation_multiplier);
    let rollout = scratch(f);
    let previous = rollout.roll_angle;
    // 8013F3C8 fmul, 8013F3E4 fmul (double), frsp; 8013F3F0 fmuls,
    // 8013F3F8 fmuls, 8013F3FC fmuls, 8013F400 fadds.
    let scale = (ROTATION_SCALE * f64::from(rotation) * f64::from(rollout.direction)) as f32;
    let delta = scale * (DEGREES_TO_RADIANS * rollout.charge as f32 * aerial);
    rollout.roll_angle = previous + delta;
    model::set_roll(f);
    stop_face_down(f, previous, delta, true, p.assets)?;
    Ok(None)
}

/// ftPr_SpecialNRelease_IASA (801401C4): the stick held against the roll
/// turns it round.
pub(super) fn release_input(f: &mut Fighter, p: InputPhase<'_>) {
    let stick = f.input.current.stick.x;
    if fabsf(stick) <= attributes(f).reverse_stick_threshold {
        return;
    }
    let direction = sign(stick);
    if scratch(f).direction == direction {
        return;
    }
    model::clear_hitboxes(f);
    let rollout = scratch(f);
    rollout.pending_facing = direction;
    rollout.direction = -direction;
    let frame = f.animation.frame;
    change(f, TURN, flags::HELD, frame, 0.0, p.assets).expect("Rollout turn");
    let speed = f.physics.ground_velocity;
    let rollout = scratch(f);
    rollout.turn_speed = speed;
    // 801402A0 fmuls.
    rollout.acceleration = TURN_DECELERATION * speed;
    rollout.dust_timer = 0;
    model::set_roll(f);
    model::sound(f, model::RELEASE_SOUND);
}

/// ftPr_SpecialNTurn_Anim (8013E7E0): spin backward (8013E9A8 fmul,
/// 8013E9B4 fmadd, both double, frsp); the dust every few frames. If the
/// roll time runs out mid-turn, the roll ends facing the other way.
pub(super) fn turn_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox(f);
    f.step_animation(p.assets);
    model::roll_sound(f, true);
    model::squash(f);
    let rotation = attributes(f).turn_rotation_speed;
    let rollout = scratch(f);
    rollout.roll_angle = fma::fmadd(
        ROTATION_SCALE * f64::from(rotation),
        f64::from(-rollout.direction),
        f64::from(rollout.roll_angle),
    ) as f32;
    model::set_roll(f);
    if run_out(f) {
        end(f, false, flags::START, 0.0, p.assets)?;
        return Ok(None);
    }
    let interval = attributes(f).turn_effect_interval;
    if scratch(f).dust_timer % interval == 0 {
        model::dust(f);
    }
    scratch(f).dust_timer += 1;
    model::face_forward(f);
    Ok(None)
}

/// A turn whose roll time is spent ends reversed.
fn run_out(f: &mut Fighter) -> bool {
    let rollout = scratch(f);
    rollout.remaining -= 1;
    if rollout.remaining > 0 {
        return false;
    }
    rollout.remaining = 0;
    rollout.direction = -rollout.direction;
    true
}

/// ftPr_SpecialAirNStartTurn_Anim (8013F708): as on the ground, scaled by
/// the aerial multiplier (8013F8D0 fmul, 8013F8E0 fmul, frsp, 8013F8E8
/// fmadds), without the dust; it ends on the ground's row.
pub(super) fn air_turn_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox(f);
    f.step_animation(p.assets);
    model::roll_sound(f, true);
    model::squash(f);
    let a = attributes(f);
    let (rotation, aerial) = (a.turn_rotation_speed, a.air_rotation_multiplier);
    let rollout = scratch(f);
    let step = (ROTATION_SCALE * f64::from(rotation) * f64::from(-rollout.direction)) as f32;
    rollout.roll_angle = fma::fmadds(aerial, step, rollout.roll_angle);
    model::set_roll(f);
    if run_out(f) {
        end(f, false, flags::START, 0.0, p.assets)?;
        return Ok(None);
    }
    scratch(f).dust_timer += 1;
    model::face_forward(f);
    Ok(None)
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

/// Clamp `value`'s magnitude to `limit` (strictly above), keeping its sign.
fn clamp_speed(value: f32, limit: f32) -> f32 {
    if fabsf(value) > limit {
        if value < 0.0 {
            -limit
        } else {
            limit
        }
    } else {
        value
    }
}

/// ftPr_SpecialNRelease_Phys (801406B0): the charge sets the speed, the
/// slope adds or takes some; the charge decays and, spent, ends the roll.
pub(super) fn release_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f);
    let (multiplier, minimum, slope_influence, ground_limit, slope_limit, decay) = (
        a.charge_speed_multiplier,
        a.minimum_charge,
        a.slope_influence,
        a.ground_speed_limit,
        a.slope_speed_limit,
        a.charge_decay,
    );
    let slope = f.collision.data.floor.normal.x;
    let rollout = scratch(f);
    // 8014070C fsubs, 80140710 fmuls, 80140714 fmuls.
    let base = rollout.direction * (multiplier * (rollout.charge as f32 - minimum));
    // 8014072C fmuls, 80140738 fmuls.
    let influence = slope_influence * (base * fabsf(slope));
    let mut velocity = if slope > 0.0 {
        base + influence
    } else {
        base - influence
    };
    velocity = clamp_speed(velocity, ground_limit);
    velocity = clamp_speed(velocity, slope_limit);
    rollout.speed = fabsf(velocity);
    f.physics.ground_velocity = velocity;
    f.physics.animation_velocity.y = 0.0;
    f.physics.self_velocity.y = 0.0;
    apply_ground_movement(f, &p);
    if decay_charge(f, decay, minimum) {
        end(f, false, flags::START, 0.0, p.assets).expect("Rollout end");
    }
    finish_ground(f, &p);
}

/// The charge runs down (80140848 fsubs, 80140850 fctiwz); true once it
/// falls below the minimum.
fn decay_charge(f: &mut Fighter, decay: f32, minimum: f32) -> bool {
    let rollout = scratch(f);
    rollout.charge = gekko_math::msl::fctiwz(rollout.charge as f32 - decay);
    (rollout.charge as f32) < minimum
}

/// ftPr_SpecialNTurn_Phys (801408B8): decelerate through zero, scaled by
/// the floor's friction and slope; once the speed reverses past a
/// fraction of the entry speed, roll on (x21EC starts a new attack
/// instance inside the state change).
pub(super) fn turn_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f);
    let (friction, slope_influence, ratio) = (
        a.ground_friction_multiplier,
        a.slope_influence,
        a.turn_speed_ratio,
    );
    let terrain = melee_mp::terrain_speed_scale(f.collision.data.floor.flags);
    let slope = f.collision.data.floor.normal.x;
    let acceleration = scratch(f).acceleration;
    // 801408F4 fmuls, 80140910 fmuls, 8014091C fmuls.
    let scale = friction * terrain;
    let influence = slope_influence * (acceleration * fabsf(slope));
    let step = if slope > 0.0 {
        acceleration + influence
    } else {
        acceleration - influence
    };
    // 80140934..78: fmadds.
    f.physics.ground_velocity = fma::fmadds(scale, step, f.physics.ground_velocity);
    f.physics.animation_velocity.y = 0.0;
    f.physics.self_velocity.y = 0.0;
    apply_ground_movement(f, &p);
    let turn = scratch(f).turn_speed;
    let velocity = f.physics.ground_velocity;
    let reversed = if turn > 0.0 {
        velocity < 0.0
    } else {
        velocity > 0.0
    };
    // 801409B4 fmuls, then |x10 * ratio| <= |gr_vel|.
    if reversed && fabsf(velocity) >= fabsf(turn * ratio) {
        let flags = if turn > 0.0 {
            flags::UNTURN_FROM_RIGHT
        } else {
            flags::UNTURN_FROM_LEFT
        };
        roll_on(f, flags, p.assets).expect("Rollout turn end");
    }
    model::face_forward(f);
    model::set_roll(f);
    finish_ground(f, &p);
}

/// The turn's return to Release, facing the new way.
fn roll_on(
    f: &mut Fighter,
    flags: melee_ft::fighter::MotionEntryFlags,
    assets: &FighterAssets,
) -> Result<()> {
    // ftPr_SpecialS_8013D8B0 (x21EC): ft_80089824's statistics, then
    // ft_800892A0.
    f.combat.stale.new_instance();
    let frame = f.animation.frame;
    change(f, RELEASE, flags, frame, 0.0, assets)?;
    let rollout = scratch(f);
    let pending = rollout.pending_facing;
    if pending != 0.0 {
        rollout.direction = pending;
        f.physics.facing = pending;
    }
    let rollout = scratch(f);
    rollout.pending_facing = 0.0;
    rollout.group_timer = 0;
    Ok(())
}

/// ftPr_SpecialAirNChargeRelease_Phys (80140C78): the charge sets the
/// speed, less the air deceleration, at least the aerial minimum.
pub(super) fn air_release_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f);
    let (multiplier, minimum, decay) =
        (a.charge_speed_multiplier, a.minimum_charge, a.charge_decay);
    let rollout = scratch(f);
    // 80140CC8 fsubs, 80140CCC fmuls, 80140CD0 fmuls.
    f.physics.self_velocity.x =
        rollout.direction * (multiplier * (rollout.charge as f32 - minimum));
    air_decelerate(f);
    let speed = fabsf(f.physics.self_velocity.x);
    scratch(f).speed = speed;
    fall(f);
    if decay_charge(f, decay, minimum) {
        end(f, true, flags::START, 0.0, p.assets).expect("Rollout end");
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// The aerial deceleration toward zero, never below the aerial minimum.
fn air_decelerate(f: &mut Fighter) {
    let a = attributes(f);
    let (deceleration, minimum) = (a.air_deceleration, a.air_minimum_speed);
    let velocity = &mut f.physics.self_velocity.x;
    // 80140CFC fsubs.
    *velocity -= if *velocity > 0.0 {
        deceleration
    } else {
        -deceleration
    };
    if fabsf(*velocity) < minimum {
        *velocity = if *velocity < 0.0 { -minimum } else { minimum };
    }
}

/// ftPr_SpecialAirNStartTurn_Phys (80140DF8).
pub(super) fn air_turn_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    air_decelerate(f);
    fall(f);
    model::face_forward(f);
    model::set_roll(f);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ft_80082888 / ft_80082978 with the roll's box, after Fighter_procMap's
/// prologue.
fn collide_ground(f: &mut Fighter, p: &mut CollisionPhase<'_>, stop_at_edge: bool) -> bool {
    let c = &mut f.core;
    air::begin_map(
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
        ROLL_BOX,
        stop_at_edge,
    )
}

/// The wall on the rolling side (env_flags 0x3F rolling right, 0xFC0
/// left), after its effect.
fn touched_wall(f: &mut Fighter) -> bool {
    use melee_types::mp::collide::{LEFT_WALL_MASK, RIGHT_WALL_MASK};
    let direction = scratch(f).direction;
    let env = f.collision.data.env_flags as u32;
    let wall = if direction == 1.0 {
        env & LEFT_WALL_MASK != 0
    } else {
        env & RIGHT_WALL_MASK != 0
    };
    if wall {
        model::wall_bounce_effect(f, if direction == 1.0 { 1.0 } else { -1.0 });
    }
    wall
}

/// A wall takes the charge and speed down by the rebound multiplier
/// (80141480 fmuls, fctiwz; separate fmuls for the rest).
fn damp_on_wall(f: &mut Fighter) {
    let rebound = attributes(f).wall_rebound_multiplier;
    let rollout = scratch(f);
    rollout.charge = gekko_math::msl::fctiwz(rollout.charge as f32 * rebound);
    if rollout.charge < 0 {
        rollout.charge = 0;
    }
    rollout.speed *= rebound;
}

/// ftPr_SpecialNRelease_Coll (80141254): walls bounce the roll back;
/// rolling off the floor continues in the air.
pub(super) fn release_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let supported = collide_ground(f, &mut p, false);
    if touched_wall(f) {
        damp_on_wall(f);
        let rebound = attributes(f).wall_rebound_multiplier;
        f.physics.ground_velocity = -f.physics.ground_velocity * rebound;
        f.physics.self_velocity.x = -f.physics.self_velocity.x * rebound;
        let direction = sign(f.physics.ground_velocity);
        scratch(f).direction = direction;
        model::face_forward(f);
    }
    if !supported {
        f.leave_ground();
        let frame = f.animation.frame;
        change(
            f,
            AIR_RELEASE,
            flags::ROLLING_GROUND_AIR,
            frame,
            0.0,
            assets(&p),
        )?;
        model::set_roll(f);
        model::face_forward(f);
        let acceleration = attributes(f).air_acceleration;
        scratch(f).acceleration = acceleration;
    }
    Ok(())
}

/// ftPr_SpecialNTurn_Coll (801415F4): a fast turn ignores floor edges;
/// rolling off the floor continues the turn in the air.
pub(super) fn turn_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let fast = fabsf(f.physics.ground_velocity) > attributes(f).landing_speed_threshold;
    if collide_ground(f, &mut p, !fast) {
        return Ok(());
    }
    f.leave_ground();
    let frame = f.animation.frame;
    change(f, AIR_TURN, flags::GROUND_AIR, frame, 0.0, assets(&p))
}

/// ft_8008239C (8008239C): an airborne pass with the roll's box, catching
/// ledges on the rolling side while the ledge cooldown allows.
fn collide_air(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let direction = if scratch(f).direction == 1.0 { 1 } else { -1 };
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_box_catching_ledges(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        ROLL_BOX,
        direction,
        c.status.ledge_cooldown == 0,
    )
}

/// ftPr_SpecialAirNChargeRelease_Coll (801419E0): walls bounce the roll
/// back; a slow floor bounce lands rolling, a fast one bounces (the stick
/// may steer); a ledge is caught.
pub(super) fn air_release_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let floor = collide_air(f, &mut p);
    if touched_wall(f) {
        damp_on_wall(f);
        let rebound = attributes(f).wall_rebound_multiplier;
        f.physics.self_velocity.x = -f.physics.self_velocity.x * rebound;
        let rollout = scratch(f);
        rollout.direction = -rollout.direction;
    }
    if floor {
        bounce_or_land(f, assets(&p))?;
        scratch(f).squash_step = 0;
        return Ok(());
    }
    let assets = assets(&p);
    if f.try_grab_ledge(assets, p.map)? {
        // ftPr_SpecialAirNChargeRelease_Coll_inline: the pose returns, then
        // ftCliffCommon_80081370 runs a second time.
        model::restore(f);
        f.enter_cliff_catch(assets, p.map)?;
    }
    Ok(())
}

/// The floor under an aerial roll: the bounce keeps the vertical speed
/// (80141CA8 fmuls, always upward); too slow a bounce lands rolling.
fn bounce_or_land(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let a = attributes(f);
    let (kept, minimum, threshold, ground_acceleration) = (
        a.bounce_vertical_multiplier,
        a.minimum_bounce_speed,
        a.reverse_stick_threshold,
        a.ground_acceleration,
    );
    let vertical = fabsf(f.physics.self_velocity.y * kept);
    f.physics.self_velocity.y = vertical;
    if vertical < minimum {
        f.land();
        let frame = f.animation.frame;
        change(f, RELEASE, flags::ROLLING_GROUND_AIR, frame, 0.0, assets)?;
        let rollout = scratch(f);
        // 80141D28 fmuls.
        let velocity = rollout.speed * rollout.direction;
        f.physics.ground_velocity = velocity;
        f.physics.self_velocity.x = velocity;
        f.physics.self_velocity.z = 0.0;
        f.physics.self_velocity.y = 0.0;
        model::set_roll(f);
        model::face_forward(f);
        model::dust(f);
        scratch(f).acceleration = ground_acceleration;
        return Ok(());
    }
    let stick = f.input.current.stick.x;
    if fabsf(stick) > threshold {
        let rollout = scratch(f);
        rollout.direction = sign(stick);
        // 80141E34 fmuls.
        let velocity = rollout.speed * rollout.direction;
        f.physics.ground_velocity = velocity;
        f.physics.self_velocity.x = velocity;
        model::face_forward(f);
    }
    model::set_roll(f);
    Ok(())
}

/// ftPr_SpecialAirNStartTurn_Coll (80141FB8): landing turns on the ground.
pub(super) fn air_turn_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !collide_air_box(f, &mut p) {
        return Ok(());
    }
    f.land();
    let frame = f.animation.frame;
    change(f, TURN, flags::GROUND_AIR, frame, 0.0, assets(&p))?;
    f.physics.ground_velocity = f.physics.self_velocity.x;
    f.physics.self_velocity.z = 0.0;
    f.physics.self_velocity.y = 0.0;
    Ok(())
}

/// ft_800824A0 with the roll's box, after Fighter_procMap's prologue.
pub(super) fn collide_air_box(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        ROLL_BOX,
    )
}
