//! The aerial Egg Roll states: the entry hop (360), the loops (361, 362)
//! and the break (363).
use super::{
    assets, attributes, b_breaks, change, count_down, end, finish_air, model, roll, rumble,
    smash_input, step, touchdown_sound, touched_wall, AnimResult, BREAK_FLAGS, EGG_BOX,
    GROUND_AIR_FLAGS, LOOP_AIR, LOOP_FLAGS, LOOP_GROUND, START_GROUND,
};
use gekko_math::msl::fabsf;
use melee_ft::{
    collision::air,
    fighter::{
        assets::Result,
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionEntryFlags,
    },
};
use melee_types::CommonMotionState;

/// The bounce loop's landing: GROUND_AIR_FLAGS | SkipHit.
const LAND_ROLLING_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_509A);
/// @425: pi / 30 in double precision, the hop's roll per unit of x60.
const START_ROLL_STEP: f64 = 0.10471975511965977;
/// @421: 2pi / 25 in double precision.
const ROLL_STEP: f64 = 0.25132741228718347;
/// @427: the slowest a landing roll moves.
const MINIMUM_SPEED: f32 = 0.01;

/// ftCommon_Fall (8007D4B8): the roll's own gravity and terminal speed.
fn fall(f: &mut Fighter) {
    let a = attributes(f);
    let (gravity, terminal) = (a.start_gravity, a.start_terminal_velocity);
    let velocity = &mut f.physics.self_velocity.y;
    *velocity -= gravity;
    if *velocity < -terminal {
        *velocity = -terminal;
    }
}

/// ft_80081D0C: ordinary airborne collision; true on landing.
fn landed(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftYs_SpecialAirSStart_1_Anim (80130330): at the end, the aerial loop at
/// frame 0 with a frozen animation.
pub(super) fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    let scratch = roll(f);
    scratch.pending_facing = 0.0;
    scratch.ground_ticks = 0;
    model::step_shell(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        change(f, LOOP_AIR, LOOP_FLAGS, 0.0, 0.0, p.assets)?;
        f.animation.frame = 0.0;
    }
    Ok(None)
}

/// ftYs_SpecialAirSStart_1_IASA (80130D5C), and the other aerial states':
/// the grounded-tick count stays zero in the air.
pub(super) fn start_input(f: &mut Fighter, _: InputPhase<'_>) {
    roll(f).ground_ticks = 0;
}

/// ftYs_SpecialAirSStart_1_Phys (80130EB4).
pub(super) fn start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    roll(f).ground_ticks = 0;
    fall(f);
    finish_air(f, &p);
}

/// ftYs_SpecialAirSStart_1_Coll (80131E98): landing continues the start on
/// the ground (ftCommon_AirToGroundStateChange).
pub(super) fn start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    roll(f).ground_ticks = 0;
    if landed(f, &mut p) {
        f.land();
        let frame = f.animation.frame;
        change(f, START_GROUND, GROUND_AIR_FLAGS, frame, 1.0, assets(&p))?;
    }
    Ok(())
}

/// ftYs_SpecialAirSLoop_2_Anim (80130468).
pub(super) fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    let scratch = roll(f);
    scratch.pending_facing = 0.0;
    scratch.ground_ticks = 0;
    model::step_shell(f);
    model::squash(f);
    // Retail 801305F8 fmadd (double), frsp.
    let start_rotation = attributes(f).start_rotation_speed;
    model::advance_roll(f, START_ROLL_STEP, start_rotation);
    model::wrap_roll(f);
    count_down(f, true, p.assets)?;
    Ok(None)
}

/// ftYs_SpecialAirSLoop_3_Anim (801306A0): the aerial loop after rolling
/// off the ground keeps the ground loop's hitbox upkeep and roll.
pub(super) fn bounce_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    let scratch = roll(f);
    scratch.pending_facing = 0.0;
    scratch.ground_ticks = 0;
    model::step_shell(f);
    model::squash(f);
    model::update_hitbox(f);
    // Retail fmuls, 80130974 fmadd (double), frsp.
    let amount = attributes(f).rolling_rotation_multiplier * fabsf(roll(f).speed);
    model::advance_roll(f, ROLL_STEP, amount);
    model::wrap_roll(f);
    count_down(f, true, p.assets)?;
    Ok(None)
}

/// ftYs_SpecialAirSLoop_2_IASA / Loop_3_IASA (80130D6C / 80130DD0): B
/// after the minimum roll breaks the egg in the air.
pub(super) fn loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    roll(f).ground_ticks = 0;
    if b_breaks(f) {
        end(f, true, LOOP_FLAGS, 0.0, p.assets).expect("Egg Roll break");
    }
}

/// ftYs_SpecialAirSLoop_2_Phys / Loop_3_Phys (80130EF4 / 80130F84): steer
/// with the stick up to the aerial maximum, under the roll's gravity.
pub(super) fn loop_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    roll(f).ground_ticks = 0;
    let a = attributes(f);
    let (steer, maximum) = (a.air_steer_multiplier, a.air_maximum_speed);
    // Retail fmuls then fadds: no fusion.
    let steering = f.input.current.stick.x * steer;
    let velocity = &mut f.physics.self_velocity.x;
    *velocity += steering;
    if fabsf(*velocity) > maximum {
        *velocity = if *velocity < 0.0 { -maximum } else { maximum };
    }
    fall(f);
    finish_air(f, &p);
}

/// ft_800824A0 with the egg's box, after Fighter_procMap's prologue.
fn collide_box(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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
        EGG_BOX,
    )
}

/// The floor bounce shared by both aerial loops: the vertical speed kept
/// (always upward). Returns whether it is too slow to bounce again.
fn floor_bounce(f: &mut Fighter) -> bool {
    let a = attributes(f);
    let (kept, minimum) = (a.floor_bounce_multiplier, a.minimum_bounce_speed);
    // Retail 80132198 fmuls.
    let velocity = fabsf(f.physics.self_velocity.y * kept);
    f.physics.self_velocity.y = velocity;
    velocity < minimum
}

/// The stick past the steer threshold picks the new facing and speed.
fn steer_on_landing(f: &mut Fighter) -> bool {
    let stick = f.input.current.stick.x;
    let a = attributes(f);
    let (threshold, multiplier) = (a.steer_threshold, a.landing_stick_multiplier);
    if fabsf(stick) <= threshold {
        return false;
    }
    f.physics.facing = if stick > 0.0 { 1.0 } else { -1.0 };
    // Retail 801322F0 / 80132300 fmuls.
    let speed = multiplier * fabsf(stick);
    roll(f).speed = speed;
    let velocity = speed * f.physics.facing;
    f.physics.ground_velocity = velocity;
    f.physics.self_velocity.x = velocity;
    true
}

/// ftYs_SpecialAirSLoop_2_Coll (80131F80): a wall breaks the egg; a slow
/// floor hit lands rolling, a fast one bounces into the other aerial loop;
/// either way the roll restarts at the landing speed.
pub(super) fn loop_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    roll(f).ground_ticks = 0;
    let floor = collide_box(f, &mut p);
    let toward_right = f.physics.facing == 1.0;
    if touched_wall(f, toward_right) {
        return super::bounce_off_wall(f, assets(&p));
    }
    if !floor {
        return Ok(());
    }
    let frame = f.animation.frame;
    if floor_bounce(f) {
        f.land();
        change(f, LOOP_GROUND, LOOP_FLAGS, frame, 0.0, assets(&p))?;
        touchdown_sound(f);
        f.physics.self_velocity.z = 0.0;
        f.physics.self_velocity.y = 0.0;
        if fabsf(f.physics.self_velocity.x) < MINIMUM_SPEED {
            // Retail 8013225C fmuls.
            f.physics.self_velocity.x = MINIMUM_SPEED * f.physics.facing;
        }
        let velocity = f.physics.self_velocity.x;
        f.physics.ground_velocity = velocity;
        roll(f).speed = fabsf(velocity);
        steer_on_landing(f);
        rumble(f, 1, 0);
    } else {
        change(
            f,
            super::LOOP_AIR_BOUNCE,
            LOOP_FLAGS,
            frame,
            0.0,
            assets(&p),
        )?;
        model::wrap_roll(f);
    }
    let a = attributes(f);
    let (landing, smash) = (a.landing_speed, a.smash_speed_multiplier);
    roll(f).speed = landing;
    if smash_input(f) {
        // Retail 80132428 fmuls.
        roll(f).speed *= smash;
    }
    // Retail 8013243C fmuls.
    f.physics.self_velocity.x = roll(f).speed * f.physics.facing;
    roll(f).squash_step = 0;
    Ok(())
}

/// ftYs_SpecialAirSLoop_3_Coll (8013245C): as the other aerial loop, but a
/// slow floor hit keeps its rolling speed and a fast one may steer.
pub(super) fn bounce_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    roll(f).ground_ticks = 0;
    let floor = collide_box(f, &mut p);
    let toward_right = f.physics.facing == 1.0;
    if touched_wall(f, toward_right) {
        return super::bounce_off_wall(f, assets(&p));
    }
    if !floor {
        return Ok(());
    }
    if floor_bounce(f) {
        f.land();
        let frame = f.animation.frame;
        change(f, LOOP_GROUND, LAND_ROLLING_FLAGS, frame, 0.0, assets(&p))?;
        touchdown_sound(f);
        if fabsf(roll(f).speed) < MINIMUM_SPEED {
            roll(f).speed = MINIMUM_SPEED;
        }
        // Retail fmuls.
        let velocity = roll(f).speed * f.physics.facing;
        f.physics.ground_velocity = velocity;
        f.physics.self_velocity.x = velocity;
        f.physics.self_velocity.z = 0.0;
        f.physics.self_velocity.y = 0.0;
        // Retail fmuls, 80132770 fmadd (double), frsp.
        let amount = attributes(f).rolling_rotation_multiplier * fabsf(roll(f).speed);
        model::advance_roll(f, ROLL_STEP, amount);
        model::wrap_roll(f);
        rumble(f, 1, 0);
    } else {
        if steer_on_landing(f) {
            model::face(f);
        }
        model::wrap_roll(f);
    }
    roll(f).squash_step = 0;
    Ok(())
}

/// The end states' shell burst, once the script counts a restart
/// (cmd_vars[1] == 1).
pub(super) fn burst_on_script(f: &mut Fighter) {
    if f.commands.variables[1] == 1 {
        f.commands.variables[1] += 1;
        model::burst_shell(f);
    }
}

/// ftYs_SpecialAirSLanding_Anim (80130A9C): at the end, Fall, or a special
/// fall with the landing lag when the attribute sets one.
pub(super) fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    step(f, &p);
    let scratch = roll(f);
    scratch.pending_facing = 0.0;
    scratch.ground_ticks = 0;
    burst_on_script(f);
    model::step_shell(f);
    model::squash(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        model::restore(f);
        let lag = attributes(f).landing_lag;
        if lag == 0.0 {
            f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
        } else {
            // ftCo_80096900(gobj, 1, 0, 1, 1.0, xE8).
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// ftYs_SpecialAirSLanding_IASA (80130E30).
pub(super) fn end_input(f: &mut Fighter, _: InputPhase<'_>) {
    roll(f).ground_ticks = 0;
}

/// ftYs_SpecialAirSLanding_Phys (80131048).
pub(super) fn end_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    roll(f).ground_ticks = 0;
    fall(f);
    finish_air(f, &p);
}

/// ftYs_SpecialAirSLanding_Coll (801328EC): landing finishes the break on
/// the ground at the same frame.
pub(super) fn end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    roll(f).ground_ticks = 0;
    if landed(f, &mut p) {
        f.land();
        let frame = f.animation.frame;
        end(f, false, BREAK_FLAGS, frame, assets(&p))?;
    }
    Ok(())
}
