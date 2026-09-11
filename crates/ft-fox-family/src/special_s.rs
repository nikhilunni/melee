//! Illusion / Phantasm, ftfoxspecials.c (800E9DF8..800EAD68).
use crate::{FamilyState as S, FoxFamily};
use hsd_types::Vec3;
use melee_ft::{
    anim::{MotionFlags, WaitChoice},
    fighter::{
        assets::{FighterAssets, Result},
        state::{self, callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
    physics::{airborne, friction, grounded, integrate},
};
use melee_types::{CommonMotionState, GroundOrAir};

#[derive(Clone, Debug, Default)]
pub struct SpecialSide {
    /// Fighter +2340: ticks until gravity resumes.
    pub gravity_delay: i32,
    /// Fighter +2344..2370: four preceding positions, newest first.
    pub ghost_positions: [Vec3; 4],
    /// Fighter +2374..2380: matching TopN rotation-X samples.
    pub ghost_rotations: [f32; 4],
    /// Fighter +2384: whether the ghost item has been requested.
    pub ghost_present: bool,
    /// accessory4_cb = ftFx_SpecialS_CreateGFX, cleared after one call.
    pub trail_pending: bool,
}

pub const fn rows<C: FoxFamily>() -> [MotionRow; 6] {
    [
        row(
            S::SpecialSStart,
            301,
            start::<C, false>,
            no_input,
            startup_physics::<C, false>,
            ground_startup_collision,
        ),
        row(
            S::SpecialS,
            302,
            travel::<C, false>,
            shorten::<C>,
            travel_physics::<C, false>,
            ground_travel_collision,
        ),
        row(
            S::SpecialSEnd,
            303,
            end::<C, false>,
            no_input,
            end_physics::<C, false>,
            callbacks::collision::escape,
        ),
        row(
            S::SpecialAirSStart,
            304,
            start::<C, true>,
            no_input,
            startup_physics::<C, true>,
            startup_collision,
        ),
        row(
            S::SpecialAirS,
            305,
            travel::<C, true>,
            shorten::<C>,
            travel_physics::<C, true>,
            startup_collision,
        ),
        row(
            S::SpecialAirSEnd,
            306,
            end::<C, true>,
            no_input,
            end_physics::<C, true>,
            end_collision::<C>,
        ),
    ]
}

pub(crate) const fn row(
    action: S,
    animation: i32,
    anim: state::AnimFn,
    iasa: state::InputFn,
    physics: state::PhysicsFn,
    collision: state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: melee_ft::fighter::ActionId(action as u16),
        id: CommonMotionState::None,
        animation,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// ftFx_SpecialSStart_Enter / SpecialAirSStart_Enter (800E9EE8 / 800E9F6C).
pub fn enter<C: FoxFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let attrs = &f.character.get::<C>().attributes().illusion;
    let delay = gekko_math::msl::fctiwz(attrs.gravity_delay);
    let divisor = attrs.startup_momentum_divisor;
    // Retail 800E9F38 / 800E9FCC: fdivs, no fused arithmetic.
    if air {
        f.physics.self_velocity.y = 0.0;
        f.physics.self_velocity.x /= divisor;
    } else {
        f.physics.ground_velocity /= divisor;
    }
    f.commands.variables[2] = 0;
    *f.character.get_mut::<C>().special_side() = SpecialSide {
        gravity_delay: delay,
        ..Default::default()
    };
    f.change_motion_state(
        (if air {
            S::SpecialAirSStart
        } else {
            S::SpecialSStart
        })
        .into(),
        assets,
    )
    .expect("Illusion startup assets");
    f.step_animation(assets);
    if air {
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    }
}

fn no_input(_f: &mut Fighter, _phase: InputPhase<'_>) {}

fn start<C: FoxFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(
            (if AIR { S::SpecialAirS } else { S::SpecialS }).into(),
            p.assets,
        )?;
        let position = f.physics.position;
        let rotation = f.skeleton.get(f.animation.root).rotate.x;
        let scratch = f.character.get_mut::<C>().special_side();
        scratch.ghost_positions.fill(position);
        scratch.ghost_rotations.fill(rotation);
        scratch.trail_pending = true;
    }
    Ok(None)
}

fn travel<C: FoxFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        enter_end::<C>(f, AIR, p.assets)?;
    }
    if f.commands.variables[2] == 1 {
        f.commands.variables[2] = 0;
        let mut spawn =
            melee_it::SpawnItem::held(C::GHOST, f.player.id, f.physics.position, f.physics.facing);
        spawn.ground_or_air = f.physics.ground_or_air;
        f.item_requests.push(melee_it::ItemRequest::Spawn(spawn));
        f.character.get_mut::<C>().special_side().ghost_present = true;
    }
    Ok(None)
}

/// ftFx_SpecialSEnd_Enter / SpecialAirSEnd_Enter (800EAC50 / 800EACD8).
fn enter_end<C: FoxFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let attrs = &f.character.get::<C>().attributes().illusion;
    let delay = gekko_math::msl::fctiwz(attrs.end_gravity_delay);
    let speed = if air {
        attrs.air_end_vel_x
    } else {
        attrs.ground_end_vel_x
    };
    // Retail 800EAC80 / 800EAD08: fmuls.
    if air {
        f.physics.self_velocity.x = speed * f.physics.facing;
        f.physics.self_velocity.y = 0.0;
    } else {
        f.physics.ground_velocity = speed * f.physics.facing;
    }
    f.change_motion_state(
        (if air {
            S::SpecialAirSEnd
        } else {
            S::SpecialSEnd
        })
        .into(),
        assets,
    )?;
    f.character.get_mut::<C>().special_side().gravity_delay = delay;
    Ok(())
}

fn shorten<C: FoxFamily>(f: &mut Fighter, p: InputPhase<'_>) {
    if f.input.pressed.intersects(melee_ft::input::Buttons::B) {
        enter_end::<C>(f, f.physics.ground_or_air == GroundOrAir::Air, p.assets)
            .expect("Illusion end assets");
    }
}

fn end<C: FoxFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if AIR {
            let attrs = &f.character.get::<C>().attributes().illusion;
            let (mobility, lag) = (attrs.freefall_mobility, attrs.landing_lag);
            f.enter_special_fall(p.assets, true, false, true, mobility, lag)?;
        } else {
            f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
        }
    }
    Ok(None)
}

fn delay<C: FoxFamily>(f: &mut Fighter) -> bool {
    let delay = &mut f.character.get_mut::<C>().special_side().gravity_delay;
    if *delay == 0 {
        true
    } else {
        *delay -= 1;
        false
    }
}

/// ftCommon_ApplyFrictionAir (8007CE94): acceleration applied by procUpdate.
pub(crate) fn air_friction(f: &mut Fighter, friction: f32) {
    let velocity = f.physics.self_velocity.x;
    f.physics.animation_velocity.x = if friction.abs() >= velocity.abs() {
        -velocity
    } else if velocity > 0.0 {
        -friction
    } else {
        friction
    };
}

pub(crate) fn finish_ground(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let core = &mut f.core;
    grounded::apply_ground_movement(
        &mut core.physics,
        core.collision.data.floor.normal,
        p.map.floor_speed_scale(&core.collision.data),
    );
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}
pub(crate) fn finish_air(f: &mut Fighter, p: PhysicsPhase<'_>) {
    integrate::integrate_velocity(&mut f.physics);
    integrate::integrate_environment(&mut f.physics, None, p.wind);
}

fn startup_physics<C: FoxFamily, const AIR: bool>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let falling = delay::<C>(f);
    if AIR {
        let attrs = &f.character.get::<C>().attributes().illusion;
        let (accel, friction) = (attrs.startup_fall_acceleration, attrs.startup_air_friction);
        if falling {
            f.physics.self_velocity.y = airborne::gravity(
                f.physics.self_velocity.y,
                accel,
                f.attributes.air.terminal_velocity,
            );
        }
        air_friction(f, friction);
        finish_air(f, p);
    } else {
        callbacks::physics::guard_on(f, p);
    }
}

fn record_position<C: FoxFamily>(f: &mut Fighter) {
    let position = f.physics.position;
    let rotation = f.skeleton.get(f.animation.root).rotate.x;
    let scratch = f.character.get_mut::<C>().special_side();
    scratch.ghost_positions.rotate_right(1);
    scratch.ghost_positions[0] = position;
    scratch.ghost_rotations.rotate_right(1);
    scratch.ghost_rotations[0] = rotation;
}

fn travel_physics<C: FoxFamily, const AIR: bool>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("Illusion TransN")
        .primary_history
        .offset;
    if AIR {
        // Retail 80085140 fmuls; vertical offset copied without arithmetic.
        f.physics.self_velocity.x = offset.z * f.physics.facing;
        f.physics.self_velocity.y = offset.y;
        record_position::<C>(f);
        finish_air(f, p);
    } else {
        if f.animation.flags.contains(MotionFlags::ROOT_MOTION) {
            // Retail 80085108 fmuls: unlike jab, replaces ground velocity.
            f.physics.ground_velocity = offset.z * f.physics.facing;
        } else {
            f.physics.ground_acceleration = friction::friction_acceleration(
                f.physics.ground_velocity,
                f.attributes.ground.ground_friction,
            );
        }
        record_position::<C>(f);
        finish_ground(f, p);
    }
}

fn end_physics<C: FoxFamily, const AIR: bool>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let falling = delay::<C>(f);
    let attrs = &f.character.get::<C>().attributes().illusion;
    let (accel, friction) = (
        attrs.end_fall_acceleration,
        if AIR {
            attrs.end_air_friction
        } else {
            attrs.ground_friction
        },
    );
    if AIR {
        if falling {
            f.physics.self_velocity.y = airborne::gravity(
                f.physics.self_velocity.y,
                accel,
                f.attributes.air.terminal_velocity,
            );
        }
        air_friction(f, friction);
        record_position::<C>(f);
        finish_air(f, p);
    } else {
        f.physics.ground_acceleration =
            friction::friction_acceleration(f.physics.ground_velocity, friction);
        record_position::<C>(f);
        finish_ground(f, p);
    }
}

/// ftFx_SpecialSStart_Coll / SpecialS_Coll use the non-teetering support probe.
fn ground_startup_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, S::SpecialAirSStart, false)
}

fn ground_travel_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, S::SpecialAirS, true)
}

/// ftFx_SpecialSStart_GroundToAir (800EA1D4), SpecialS_GroundToAir (800EA698).
fn ground_collision(
    f: &mut Fighter,
    p: CollisionPhase<'_>,
    air_state: S,
    travel: bool,
) -> Result<()> {
    use melee_ft::collision::ground::{map_ground_action, WaitGroundResult};
    let core = &mut f.core;
    match map_ground_action(
        &mut core.physics,
        &mut core.collision,
        p.map,
        &mut core.skeleton,
        core.animation.root,
        core.input.current.stick.x,
    ) {
        WaitGroundResult::Supported => Ok(()),
        WaitGroundResult::EnterFall => {
            f.leave_ground_with_spent_jumps();
            f.change_ground_air_motion(
                air_state.into(),
                p.assets.expect("Illusion collision assets"),
                melee_ft::fighter::MotionPreservation {
                    hit_status: travel,
                    ..Default::default()
                },
            )?;
            if travel {
                f.commands.variables[2] = 0;
            }
            Ok(())
        }
        WaitGroundResult::EnterTeeter => unreachable!("ft_80082708 does not teeter"),
    }
}

fn startup_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let core = &mut f.core;
    melee_ft::collision::air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    if melee_ft::collision::air::collide_fall(
        &mut core.physics,
        &mut core.collision,
        p.map,
        &mut core.skeleton,
        core.animation.root,
        core.status.ledge_cooldown == 0,
    ) {
        unimplemented!("ftFx_SpecialAirSStart_AirToGround / SpecialAirS_AirToGround: preserved motion transition");
    } else {
        f.try_grab_ledge(p.assets.expect("Illusion ledge assets"), p.map)?;
    }
    Ok(())
}
fn end_collision<C: FoxFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let core = &mut f.core;
    melee_ft::collision::air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    if melee_ft::collision::air::collide_fall(
        &mut core.physics,
        &mut core.collision,
        p.map,
        &mut core.skeleton,
        core.animation.root,
        core.status.ledge_cooldown == 0,
    ) {
        let lag = f.character.get::<C>().attributes().illusion.landing_lag;
        f.enter_special_landing(p.assets.expect("Illusion landing assets"), false, lag)?;
    } else {
        f.try_grab_ledge(p.assets.expect("Illusion ledge assets"), p.map)?;
    }
    Ok(())
}

pub fn accessory<C: FoxFamily>(f: &mut Fighter, _assets: &FighterAssets) {
    if std::mem::take(&mut f.character.get_mut::<C>().special_side().trail_pending) {
        // efAlt 0x48D -> Fox generator 0xBC0, attached to TopN.
        f.effects
            .push(melee_ef::request::EffectRequest::SyncAttached { id: 0x48D, bone: 0 });
        f.effect_state.destroy_on_state_change = true;
    }
}

pub fn item_owner<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> melee_it::ItemOwner {
    let mut owner = crate::special_n::item_owner::<C>(f, assets);
    let active =
        (S::SpecialSStart as u16..=S::SpecialAirSEnd as u16).contains(&f.motion_state.action.0);
    let create_secondary = f.commands.variables[2] == 2;
    let scratch = f.character.get_mut::<C>().special_side();
    owner.illusion = active.then_some(melee_it::IllusionOwner {
        create_secondary,
        positions: scratch.ghost_positions,
        rotations: scratch.ghost_rotations,
    });
    owner
}
