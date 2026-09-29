//! Popo's grounded Squall Hammer: SpecialS1 (343, alone) and SpecialS2
//! (344, with Nana), 8011FC78..80120A48.
use super::{
    attributes, clamp_ground_velocity, clamp_self_velocity_x, install_callbacks, press_due,
    read_steer, rebound, remove_callbacks, scratch, side, tilt, AIR_LINKED, AIR_SOLO,
    GROUND_AIR_FLAGS, SPIN_BOX,
};
use crate::{climber, partner as link};
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter,
    },
    physics::{friction, grounded},
};

/// ftPp_SpecialS1_Anim (8011FC78) / ftPp_SpecialS2_Anim (8011FCD0): Wait
/// at the end. Linked, the move also ends once Nana has left her rows
/// (ftNn_Init_80123B10), and both leave each other's hitlag (inlineC1).
pub fn anim<const LINKED: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let over = !f.animation.frames_remaining(&f.skeleton)
        || LINKED && !super::partner::in_rows(link::view(f).action);
    if over {
        if LINKED {
            link::separate(f);
        }
        remove_callbacks(f);
        climber::finish(f, p.assets, false)?;
    }
    Ok(None)
}

/// ftPp_SpecialS1_IASA (8011FF40) / ftPp_SpecialS2_IASA (8011FF90).
pub fn input(f: &mut Fighter, _: InputPhase<'_>) {
    let multiplier = attributes(f).ground_steer;
    read_steer(f, multiplier);
}

/// ftPp_SpecialS1_Phys (80120080) / ftPp_SpecialS2_Phys (80120230): the
/// stick drives the ground speed toward attribute x38 (ftCommon_8007CA80,
/// plus the slope's pull), else ground friction; a due B press lifts off
/// into the aerial row.
pub fn physics<const LINKED: bool>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let steer = scratch(f).steer;
    let (cap, slope) = {
        let a = attributes(f);
        (a.ground_speed_max, a.slope_pull)
    };
    if f.commands.variables[0] == 0 && steer != 0.0 {
        let target = if steer > 0.0 { cap } else { -cap };
        let physics = &mut f.core.physics;
        physics.ground_acceleration = accelerate_toward(physics.ground_velocity, steer, target);
        // retail 801200F8: fmadds.
        let normal_x = f.core.collision.data.floor.normal.x;
        f.physics.ground_acceleration =
            gekko_math::fma::fmadds(slope, normal_x, f.physics.ground_acceleration);
    } else {
        let friction = f.core.attributes.ground.ground_friction;
        f.physics.ground_acceleration =
            friction::friction_acceleration(f.physics.ground_velocity, friction);
    }
    clamp_ground_velocity(f, cap);
    // ftCommon_ApplyGroundMovementNoSlide: no terrain scaling.
    let normal = f.core.collision.data.floor.normal;
    grounded::apply_ground_movement(&mut f.core.physics, normal, 1.0);
    if press_due(f) {
        lift_off::<LINKED>(f, p.assets);
    }
    scratch(f).frames += 1;
    super::finish_update(f, &p);
}

/// ftCommon_8007CA80 (8007CA80): the acceleration toward `target`,
/// trimmed so the speed does not pass it.
fn accelerate_toward(velocity: f32, acceleration: f32, target: f32) -> f32 {
    if target == 0.0 {
        return -velocity;
    }
    // !(velocity * accel < 0), unordered included.
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

/// The physics' lift: leave the ground (ftCommon_8007D5D4) into the aerial
/// row at this frame, keeping the ground acceleration as x74, the speed
/// clamped to attribute x3C, and the press's rise added (fadds).
fn lift_off<const LINKED: bool>(f: &mut Fighter, assets: &FighterAssets) {
    to_air::<LINKED>(f, assets).expect("Squall Hammer lift assets");
    let rise = attributes(f).press_rise[side(LINKED)];
    f.physics.self_velocity.y += rise;
}

/// inline3 and the physics' lift: ftCommon_8007D5D4, x74.x = xE4, the
/// aerial row at the current frame, the callbacks, the clamp.
fn to_air<const LINKED: bool>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.core.leave_ground();
    f.physics.animation_velocity.x = f.physics.ground_acceleration;
    let frame = f.animation.frame;
    let state = if LINKED { AIR_LINKED } else { AIR_SOLO };
    f.change_motion_state_with_flags(state, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    install_callbacks(f);
    let cap = attributes(f).air_speed_max;
    clamp_self_velocity_x(f, cap);
    Ok(())
}

/// ftPp_SpecialS1_Coll (80120660) / ftPp_SpecialS2_Coll (80120854):
/// ft_80082888 with the spin's box, the wall rebound, and off the floor
/// the aerial row (inline3); then the tilt and the callbacks.
pub fn collision<const LINKED: bool>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let supported = ground::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        SPIN_BOX,
        false,
    );
    if let Some(speed) = rebound(f, f.physics.ground_velocity) {
        f.physics.ground_velocity = speed;
    }
    if supported {
        scratch(f).on_floor = true;
    } else {
        to_air::<LINKED>(f, p.assets.expect("Squall Hammer collision assets"))?;
        scratch(f).on_floor = false;
    }
    tilt(f);
    install_callbacks(f);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn steering_stops_at_the_cap() {
        assert_eq!(super::accelerate_toward(0.85, 0.09, 0.9), 0.9 - 0.85);
        assert_eq!(super::accelerate_toward(0.5, 0.09, 0.9), 0.09);
        assert_eq!(super::accelerate_toward(0.5, -0.09, -0.9), -0.09);
        assert_eq!(super::super::abs(-0.0), 0.0);
    }
}
