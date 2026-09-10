//! Dancing Blade, ftmarsspecials.c (8013741C..80138208).
use crate::init::Marth;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter,
    },
    input::Buttons,
};
#[derive(Clone, Debug, Default)]
pub struct SpecialSide {
    /// Fighter +2340, mv.ms.specials.x0, reset by the first hit's entry.
    pub reserved: i32,
}
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.physics.self_velocity.y = 0.0;
    if air {
        let marth = f.character.get_mut::<Marth>();
        let attrs = &marth.attributes.dancing_blade;
        let divisor = attrs.momentum_divisor;
        let boost = if !marth.side_special_boost_used {
            attrs.first_air_vertical_speed
        } else {
            0.0
        };
        marth.side_special_boost_used = true;
        f.physics.self_velocity.x /= divisor;
        f.physics.self_velocity.y = boost;
    }
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.character.get_mut::<Marth>().special_side = Default::default();
    f.change_motion_state(ActionId(if air { 358 } else { 349 }), a)
        .expect("Dancing Blade assets");
    f.step_animation(a);
}
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(
            if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
                melee_types::CommonMotionState::Fall
            } else {
                melee_types::CommonMotionState::Wait
            }
            .into(),
            p.assets,
        )?;
    }
    Ok(None)
}
pub fn input(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.pressed.intersects(Buttons::A | Buttons::B) {
        return;
    }
    if f.commands.variables[0] == 0 {
        f.commands.variables[1] = 1;
        return;
    }
    if f.commands.variables[1] != 0 {
        return;
    }
    let y = f.input.current.stick.y;
    let threshold = p.assets.input.special_vertical_threshold;
    let air_offset = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        9
    } else {
        0
    };
    let action = match f.motion_state.action.0 - air_offset {
        349 => {
            if y > threshold {
                350
            } else {
                351
            }
        }
        350 | 351 => {
            if y > threshold {
                352
            } else if y < -threshold {
                354
            } else {
                353
            }
        }
        352..=354 => {
            if y > threshold {
                355
            } else if y < -threshold {
                357
            } else {
                356
            }
        }
        _ => unreachable!("Dancing Blade continuation row"),
    };
    // ftMs_SpecialS_80137A68 resets the per-instance hit count and stale latch.
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.combat.stale.new_instance();
    f.change_motion_state(ActionId(action + air_offset), p.assets)
        .expect("Dancing Blade continuation");
}
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    if !matches!(
        melee_ft::collision::ground::map_escape(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        melee_ft::collision::ground::WaitGroundResult::Supported
    ) {
        let state = f.motion_state.action.0;
        assert!((349..=357).contains(&state), "non-Dancing Blade transition");
        f.leave_ground_with_spent_jumps();
        f.change_ground_air_motion(
            ActionId(state + 9),
            p.assets.expect("Dancing Blade collision assets"),
            preservation(),
        )?;
    }
    Ok(())
}

/// ftMs_SpecialAirS1/S2_Phys; S3/S4 use TransN horizontal root motion.
/// No fused sites in these callbacks or ft_80085204.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    use melee_ft::physics::{airborne, integrate};
    let attrs = f.character.get::<Marth>().attributes.dancing_blade.clone();
    f.physics.self_velocity.y = airborne::gravity(
        f.physics.self_velocity.y,
        attrs.fall_acceleration,
        attrs.terminal_velocity,
    );
    if f.motion_state.action.0 <= 360 {
        let x = f.physics.self_velocity.x;
        let friction = attrs.air_friction;
        f.physics.animation_velocity.x = if friction.abs() >= x.abs() {
            -x
        } else if x > 0.0 {
            -friction
        } else {
            friction
        };
    } else {
        f.physics.self_velocity.x = f
            .animation
            .root_motion
            .as_ref()
            .expect("Dancing Blade TransN")
            .primary_history
            .offset
            .z
            * f.physics.facing;
    }
    f.decay_air_knockback(p.assets);
    integrate::integrate_velocity(&mut f.physics);
    integrate::integrate_environment(&mut f.physics, None, p.wind);
}
/// ftMs_SpecialS*_Coll -> ft_80081D0C and the corresponding ground row.
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    ) {
        let state = f.motion_state.action.0;
        f.character.get_mut::<Marth>().side_special_boost_used = false;
        f.land();
        f.change_ground_air_motion(
            ActionId(state - 9),
            p.assets.expect("Dancing Blade landing assets"),
            preservation(),
        )?;
    }
    Ok(())
}
fn preservation() -> melee_ft::fighter::MotionPreservation {
    melee_ft::fighter::MotionPreservation {
        hit_status: true,
        hitboxes: true,
        effects: false,
    }
}
