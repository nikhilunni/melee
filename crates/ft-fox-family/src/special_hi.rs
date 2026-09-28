//! Fire Fox / Fire Bird, ftfoxspecialhi.c (800E71AC..800E83E0).
use crate::{
    special_s::{air_drift_friction, air_friction, finish_air, finish_ground, row},
    FamilyState as S, FoxFamily,
};
use gekko_math::{
    fma::fnmsubs,
    msl::{cosf, fctiwz, sinf},
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
    physics::{airborne, friction},
};
use melee_types::{CommonMotionState, FtPart, GroundOrAir};

#[derive(Clone, Debug, Default)]
pub struct SpecialHi {
    /// Fighter +2340, gravityDelay.
    pub gravity_delay: i32,
    /// Fighter +2344, rotateModel: launch direction relative to facing.
    pub angle: f32,
    /// Fighter +2348, travelFrames.
    pub travel_frames: i32,
    /// Fighter +234C: elapsed launch physics ticks.
    pub travel_ticks: i32,
    /// Fighter +2350: grounded launch collision ticks.
    pub collision_ticks: i32,
    /// accessory4_cb, one-shot charge or launch effect.
    pub pending_effect: Option<u16>,
}

pub const fn rows<C: FoxFamily>() -> [MotionRow; 7] {
    [
        row(
            S::SpecialHiHold,
            307,
            hold::<C>,
            no_input,
            callbacks::physics::guard_on,
            hold_ground_collision::<C>,
        ),
        row(
            S::SpecialHiHoldAir,
            308,
            hold::<C>,
            no_input,
            hold_air_physics::<C>,
            hold_air_collision::<C>,
        ),
        row(
            S::SpecialHi,
            309,
            travel::<C>,
            no_input,
            travel_ground_physics::<C>,
            travel_ground_collision::<C>,
        ),
        row(
            S::SpecialAirHi,
            309,
            travel::<C>,
            no_input,
            travel_air_physics::<C>,
            travel_air_collision::<C>,
        ),
        row(
            S::SpecialHiLanding,
            310,
            end::<C>,
            no_input,
            end_ground_physics::<C>,
            end_ground_collision::<C>,
        ),
        row(
            S::SpecialHiFall,
            311,
            end::<C>,
            no_input,
            callbacks::physics::fall,
            end_air_collision,
        ),
        row(
            S::SpecialHiBound,
            312,
            bound::<C>,
            no_input,
            bound_physics,
            bound_collision,
        ),
    ]
}
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftFx_SpecialHi_Enter / SpecialAirHiStart_Enter (800E7238 / 800E72C4).
pub fn enter<C: FoxFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let a = &f.character.get::<C>().attributes().fire_fox;
    let delay = fctiwz(a.gravity_delay);
    let divisor = a.vel_x;
    // Retail entry uses fdivs, with no fused arithmetic.
    if air {
        f.physics.self_velocity.x /= divisor;
        f.physics.self_velocity.y = 0.0;
    } else {
        f.physics.ground_velocity /= divisor;
    }
    *f.character.get_mut::<C>().special_hi() = SpecialHi {
        gravity_delay: delay,
        pending_effect: Some(0x48B),
        ..Default::default()
    };
    f.change_motion_state(
        (if air {
            S::SpecialHiHoldAir
        } else {
            S::SpecialHiHold
        })
        .into(),
        assets,
    )
    .expect("Fire Fox charge assets");
    f.arm_accessory4();
    f.step_animation(assets);
}
fn hold<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.physics.ground_or_air == GroundOrAir::Ground {
            let stick = f.input.current.stick;
            let a = &f.character.get::<C>().attributes().fire_fox;
            let direction = hsd_types::Vec3::new(stick.x, stick.y, 0.0);
            let normal = f.collision.data.floor.normal;
            let grounded = stick.x.abs() + stick.y.abs() >= a.direction_stick_min
                && melee_lb::shield::angle_xy(normal, direction) >= std::f32::consts::FRAC_PI_2;
            // ftFx_SpecialAirHi_AirToGround (800E7AE4): on a platform,
            // ftCo_8009A134 skips it and the launch goes airborne instead.
            if grounded && !f.skip_platform_floor() {
                return launch_ground::<C>(f, p.assets).map(|()| None);
            }
            f.leave_ground_with_spent_jumps();
        }
        launch::<C>(f, p.assets)?;
    }
    Ok(None)
}
/// ftFx_SpecialAirHi_Enter (800E7C98): separate fmuls for initial velocity.
fn launch<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let a = &f.character.get::<C>().attributes().fire_fox;
    let (minimum, facing_minimum, speed, duration) = (
        a.direction_stick_min,
        a.facing_stick_min,
        a.speed,
        fctiwz(a.duration),
    );
    let stick = f.input.current.stick;
    let angle = if stick.x.abs() + stick.y.abs() >= minimum {
        if stick.x.abs() > facing_minimum {
            f.physics.facing = if stick.x < 0.0 { -1.0 } else { 1.0 };
        }
        melee_lb::trigf::atan2f(stick.y, stick.x * f.physics.facing)
    } else {
        std::f32::consts::FRAC_PI_2
    };
    f.change_motion_state(S::SpecialAirHi.into(), assets)?;
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.angle = angle;
    scratch.travel_frames = duration;
    scratch.travel_ticks = 0;
    scratch.collision_ticks = 0;
    scratch.pending_effect = Some(0x48C);
    f.arm_accessory4();
    // ftfoxspecialhi.c:530: x21F8 = ftCommon_8007F76C.
    f.set_cape_turn_end(melee_ft::fighter::cape_turn::CapeTurnEnd::SpeedForward);
    f.physics.self_velocity.x = f.physics.facing * (speed * cosf(angle));
    f.physics.self_velocity.y = speed * sinf(angle);
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    let part = assets.parts.part_to_joint[FtPart::XRotN as usize].expect("XRotN");
    let joint = f.animation.parts[usize::from(part)].joint;
    // ftFox_SpecialHi_RotateModel: fsubs from single-precision 2*pi.
    f.skeleton
        .set_rotation_x(joint, std::f32::consts::TAU - angle);
    Ok(())
}
fn travel<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.travel_frames -= 1;
    if scratch.travel_frames <= 0 {
        // ftCommon_8007DB24 precedes the end-state transition.
        f.effects.push(EffectRequest::DestroyOwned);
        f.effect_state.destroy_on_state_change = false;
        f.change_motion_state(
            (if f.physics.ground_or_air == GroundOrAir::Air {
                S::SpecialHiFall
            } else {
                S::SpecialHiLanding
            })
            .into(),
            p.assets,
        )?;
        // ftfoxspecialhi.c:646, 660: x21F8 = ftCommon_8007F76C.
        f.set_cape_turn_end(melee_ft::fighter::cape_turn::CapeTurnEnd::SpeedForward);
    }
    Ok(None)
}
fn end<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.physics.ground_or_air == GroundOrAir::Air {
            let a = &f.character.get::<C>().attributes().fire_fox;
            let (mobility, lag) = (a.freefall_mobility, a.landing_lag);
            f.enter_special_fall(p.assets, true, false, true, mobility, lag)?;
        } else {
            f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
        }
    }
    Ok(None)
}
fn hold_air_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let scratch = f.character.get_mut::<C>().special_hi();
    let falling = scratch.gravity_delay == 0;
    if !falling {
        scratch.gravity_delay -= 1;
    }
    let a = &f.character.get::<C>().attributes().fire_fox;
    let (gravity, friction) = (a.fall_accel, a.air_momentum_preserve_x);
    if falling {
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            gravity,
            f.attributes.air.terminal_velocity,
        );
    }
    air_friction(f, friction);
    finish_air(f, p);
}
fn travel_air_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.travel_ticks += 1;
    let (ticks, angle) = (scratch.travel_ticks, scratch.angle);
    let a = &f.character.get::<C>().attributes().fire_fox;
    let (end, acceleration) = (a.duration_end, a.reverse_accel);
    if ticks as f32 >= end {
        // Retail 800E77C8 / 800E77E0 fnmsubs; inner horizontal fmuls is separate.
        f.physics.self_velocity.x = fnmsubs(
            f.physics.facing,
            acceleration * cosf(angle),
            f.physics.self_velocity.x,
        );
        f.physics.self_velocity.y = fnmsubs(acceleration, sinf(angle), f.physics.self_velocity.y);
    }
    finish_air(f, p);
}
fn travel_ground_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let scratch = f.character.get_mut::<C>().special_hi();
    scratch.travel_ticks += 1;
    let ticks = scratch.travel_ticks;
    let a = &f.character.get::<C>().attributes().fire_fox;
    if ticks as f32 >= a.duration_end {
        f.physics.ground_acceleration =
            friction::friction_acceleration(f.physics.ground_velocity, a.reverse_accel);
    }
    finish_ground(f, p);
}
fn end_ground_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let friction = f
        .character
        .get::<C>()
        .attributes()
        .fire_fox
        .ground_momentum_end;
    f.physics.ground_acceleration =
        friction::friction_acceleration(f.physics.ground_velocity, friction);
    finish_ground(f, p);
}
pub(crate) fn grounded_support(f: &mut Fighter, p: CollisionPhase<'_>) -> bool {
    use melee_ft::collision::ground::{map_ground_action, WaitGroundResult};
    let c = &mut f.core;
    matches!(
        map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        WaitGroundResult::Supported
    )
}
/// ftFx_SpecialHiHold_GroundToAir (800E7554): 0x0C4C5082 and spent jumps.
fn hold_ground_collision<C: FoxFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Fire Fox ground collision assets");
    if !grounded_support(f, p) {
        f.leave_ground_with_spent_jumps();
        f.change_ground_air_motion(
            S::SpecialHiHoldAir.into(),
            assets,
            melee_ft::fighter::MotionPreservation {
                effects: true,
                ..Default::default()
            },
        )?;
        f.character.get_mut::<C>().special_hi().pending_effect = Some(0x48B);
        f.arm_accessory4();
    }
    Ok(())
}
/// ftFx_SpecialHi_Coll / GroundToAir (800E77C8 / 800E7A74).
fn travel_ground_collision<C: FoxFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Fire Fox ground collision assets");
    f.character.get_mut::<C>().special_hi().collision_ticks += 1;
    if !grounded_support(f, p) {
        f.leave_ground_with_spent_jumps();
        f.change_ground_air_motion(
            S::SpecialAirHi.into(),
            assets,
            melee_ft::fighter::MotionPreservation {
                hitboxes: true,
                effects: true,
                ..Default::default()
            },
        )?;
        f.character.get_mut::<C>().special_hi().pending_effect = Some(0x48C);
        f.arm_accessory4();
    } else if f.collision.data.env_flags as u32 & melee_types::mp::collide::FLOOR_MASK != 0 {
        let n = f.collision.data.floor.normal;
        let angle = melee_lb::trigf::atan2f(-n.x * f.physics.facing, n.y);
        set_angle::<C>(f, assets, angle);
    }
    Ok(())
}

/// ftFx_SpecialHiLanding_Coll (800E7F40): normal special-fall entry.
fn end_ground_collision<C: FoxFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Fire Fox ground collision assets");
    if !grounded_support(f, p) {
        let a = &f.character.get::<C>().attributes().fire_fox;
        let (mobility, lag) = (a.freefall_mobility, a.landing_lag);
        f.enter_special_fall(assets, true, false, true, mobility, lag)?;
    }
    Ok(())
}
/// ftFx_SpecialHiHoldAir_AirToGround (800E75C0): 0x0C4C5082, then clamp.
fn hold_air_collision<C: FoxFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    // ft_CheckGroundAndLedge with the fighter-facing direction.
    if melee_ft::collision::air::collide_pass(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    ) {
        f.land();
        f.change_ground_air_motion(
            S::SpecialHiHold.into(),
            p.assets.expect("Fire Fox charge landing assets"),
            melee_ft::fighter::MotionPreservation {
                effects: true,
                ..Default::default()
            },
        )?;
        f.character.get_mut::<C>().special_hi().pending_effect = Some(0x48B);
        f.arm_accessory4();
        let maximum = f.attributes.air.air_drift_max;
        f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-maximum, maximum);
    } else {
        f.try_grab_ledge(p.assets.expect("Fire Fox charge ledge assets"), p.map)?;
    }
    Ok(())
}
/// ftFx_SpecialAirHi_Coll (800E78C8): floor incidence then cliff/wall checks.
fn travel_air_collision<C: FoxFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_types::mp::collide;
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    melee_mp::set_facing_dir(cd, 0); // CLIFFCATCH_BOTH
    let pose = melee_ft::collision::ecb::EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    let landed = if c.status.ledge_cooldown == 0 {
        p.map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
    } else {
        p.map.air_collide_pass(cd, Some(&|i| pose.position(i)))
    };
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    let a = &f.character.get::<C>().attributes().fire_fox;
    // 800E7958/5C: fadds then fmuls, not FMA.
    let threshold = 0.017453292 * (90.0 + a.bound_angle);
    let bounce_var = a.bounce_var;
    let bound_ready = f.character.get_mut::<C>().special_hi().collision_ticks >= bounce_var;
    let flags = f.collision.data.env_flags as u32;
    // ftFox_SpecialHi_IsBound: before bounce_var ticks, a platform landing
    // is skipped (ftCo_8009A134) and handled like no landing at all.
    if landed && (bound_ready || !f.skip_platform_floor()) {
        let shallow_contact =
            melee_lb::shield::angle_xy(f.collision.data.floor.normal, f.physics.self_velocity)
                < threshold;
        if flags & collide::FLOOR_MASK == 0 || !shallow_contact {
            return enter_bound::<C>(f, p.assets.expect("Fire Fox rebound"));
        }
        let direction = f.physics.self_velocity;
        f.physics.facing = if direction.x >= 0.0 { 1.0 } else { -1.0 };
        let angle = melee_lb::trigf::atan2f(direction.y, direction.x * f.physics.facing);
        set_angle::<C>(f, p.assets.expect("Fire Fox floor rotation"), angle);
    } else {
        f.try_grab_ledge(p.assets.expect("Fire Fox ledge"), p.map)?;
        if f.motion_state.action.0 != S::SpecialAirHi as u16 {
            return Ok(());
        }
        let normal = if flags & collide::CEILING_MASK != 0 {
            Some(f.collision.data.ceiling.normal)
        } else if flags & collide::LEFT_WALL_MASK != 0 {
            Some(f.collision.data.left_facing_wall.normal)
        } else if flags & collide::RIGHT_WALL_MASK != 0 {
            Some(f.collision.data.right_facing_wall.normal)
        } else {
            None
        };
        if normal
            .is_some_and(|n| melee_lb::shield::angle_xy(n, f.physics.self_velocity) < threshold)
        {
            let direction = f.physics.self_velocity;
            f.physics.facing = if direction.x >= 0.0 { 1.0 } else { -1.0 };
            let angle = melee_lb::trigf::atan2f(direction.y, direction.x * f.physics.facing);
            set_angle::<C>(f, p.assets.expect("Fire Fox wall rotation"), angle);
        }
    }
    Ok(())
}

/// ftFx_SpecialHiFall_Coll / Enter (800E7FA0 / 800E7FF0).
fn end_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    melee_mp::set_facing_dir(cd, 0); // CLIFFCATCH_BOTH.
    let pose = melee_ft::collision::ecb::EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    let landed = if c.status.ledge_cooldown == 0 {
        p.map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
    } else {
        p.map.air_collide_pass(cd, Some(&|i| pose.position(i)))
    };
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    let assets = p.assets.expect("Fire Fox ending landing");
    if landed {
        f.land();
        // 800E8018..28: action357, flags0x5000, start13/rate1; no KeepGfx.
        f.change_motion_with_updated_commands(
            S::SpecialHiLanding.into(),
            assets,
            13.0,
            melee_ft::fighter::MotionColorPolicy::Preserve,
        )?;
        f.step_animation(assets);
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}
pub fn accessory<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let pending = f
        .character
        .get_mut::<C>()
        .special_hi()
        .pending_effect
        .take();
    if let Some(id) = pending.filter(|_| f.run_accessory4(true)) {
        // Fighter_SetEffectHitlagCallbacks, charge/launch accessory callback.
        f.effect_state.hitlag_callbacks = true;
        let part = if id == 0x48B {
            FtPart::TransN
        } else {
            FtPart::HipN
        };
        let bone =
            usize::from(assets.parts.part_to_joint[part as usize].expect("Fire Fox effect bone"));
        // ftFx_SpecialHi_CreateChargeGFX / CreateLaunchGFX retain owned graphics.
        if !f.effect_state.destroy_on_state_change {
            f.effects.push(EffectRequest::SyncAttached { id, bone });
            f.effect_state.destroy_on_state_change = true;
        }
    }
    if f.motion_state.action.0 == S::SpecialAirHi as u16
        || f.motion_state.action.0 == S::SpecialHi as u16
    {
        let angle = f.character.get_mut::<C>().special_hi().angle;
        // efLib_Cb_SetRotYZ_FromFighter: single-precision fsubs followed by fneg.
        let half_pi = std::f32::consts::FRAC_PI_2 - angle;
        let (y, z) = if f.physics.facing < 0.0 {
            (-std::f32::consts::FRAC_PI_2, half_pi)
        } else {
            (std::f32::consts::FRAC_PI_2, -half_pi)
        };
        f.effects.push(EffectRequest::OwnedRotation {
            model: 0xBBC,
            rotation: hsd_types::Vec3::new(0.0, y, z),
        });
    }
}

fn set_angle<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets, angle: f32) {
    f.character.get_mut::<C>().special_hi().angle = angle;
    let bone = assets.parts.part_to_joint[FtPart::XRotN as usize].expect("XRotN");
    let joint = f.animation.parts[usize::from(bone)].joint;
    f.skeleton
        .set_rotation_x(joint, std::f32::consts::TAU - angle);
}
/// ftFx_SpecialAirHi_AirToGround (800E7AF4): floor-directed ground launch.
fn launch_ground<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.physics.facing = if f.input.current.stick.x >= 0.0 {
        1.0
    } else {
        -1.0
    };
    f.change_motion_state(S::SpecialHi.into(), assets)?;
    let a = &f.character.get::<C>().attributes().fire_fox;
    let (duration, speed) = (fctiwz(a.duration), a.speed);
    let s = f.character.get_mut::<C>().special_hi();
    s.travel_frames = duration;
    s.travel_ticks = 0;
    s.collision_ticks = 0;
    s.pending_effect = Some(0x48C);
    f.arm_accessory4();
    // ftfoxspecialhi.c:472: x21F8 = ftCommon_8007F76C.
    f.set_cape_turn_end(melee_ft::fighter::cape_turn::CapeTurnEnd::SpeedForward);
    f.physics.ground_velocity = speed * f.physics.facing;
    let n = f.collision.data.floor.normal;
    let angle = melee_lb::trigf::atan2f(-n.x * f.physics.facing, n.y);
    set_angle::<C>(f, assets, angle);
    Ok(())
}
/// ftFx_SpecialHiBound_Enter (800E82E4): motion entry, animation, X damping.
fn enter_bound<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(S::SpecialHiBound.into(), assets)?;
    f.step_animation(assets);
    // ftfoxspecialhi.c:768: x21F8 = ftCommon_8007F76C.
    f.set_cape_turn_end(melee_ft::fighter::cape_turn::CapeTurnEnd::SpeedForward);
    f.physics.self_velocity.x *= f.character.get::<C>().attributes().fire_fox.bound_vel_x;
    f.commands.variables[0] = 0;
    let n = f.collision.data.floor.normal;
    let angle = if f.collision.data.env_flags as u32 & melee_types::mp::collide::FLOOR_MASK != 0 {
        -melee_lb::trigf::atan2f(n.x, n.y)
    } else {
        0.0
    };
    let position = f.physics.position;
    f.effects
        .push(EffectRequest::SurfaceRebound { position, angle });
    f.effect_state.destroy_on_state_change = true;
    f.effect_state.hitlag_callbacks = true;
    Ok(())
}
fn bound<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let air = f.physics.ground_or_air == GroundOrAir::Air;
    if (air && f.commands.variables[0] != 0) || !f.animation.frames_remaining(&f.skeleton) {
        if air {
            let a = &f.character.get::<C>().attributes().fire_fox;
            let (mobility, lag) = (a.freefall_mobility, a.landing_lag);
            f.enter_special_fall(p.assets, true, false, true, mobility, lag)?;
            f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
        } else {
            f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
        }
    }
    Ok(None)
}
/// 800E8200 -> 800851C0: rebound uses TransN Y, not gravity.
fn bound_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Air {
        f.physics.self_velocity.y = f
            .animation
            .root_motion
            .as_ref()
            .expect("Fire Fox rebound TransN")
            .primary_history
            .offset
            .y;
        air_drift_friction(f, p.assets);
        finish_air(f, p);
    } else {
        callbacks::physics::guard_on(f, p);
    }
}
fn bound_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Air {
        let c = &mut f.core;
        melee_ft::collision::air::begin_map(
            &c.physics,
            &mut c.collision,
            &mut c.skeleton,
            c.animation.root,
        );
        if melee_ft::collision::air::collide_pass(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.status.ledge_cooldown == 0,
        ) {
            f.land();
        } else {
            f.try_grab_ledge(p.assets.expect("Fire Fox rebound ledge"), p.map)?;
        }
    } else {
        callbacks::collision::escape(f, p)?;
    }
    Ok(())
}
