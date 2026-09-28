//! Mario Tornado, ftmariospeciallw.c (800E207C..800E2B5C).
//!
//! Both entries start in the aerial row (349 is reached on landing); tapping
//! B once the script opens cmd_vars[2] rises. cmd_vars[0] starts the
//! horizontal slowdown, cmd_vars[1] spends the aerial rise
//! (x2234_tornadoCharge) and cmd_vars[3] tilts the model to the floor.
use crate::{common, init::Mario};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
    physics::{airborne, grounded},
};
use melee_types::{mp::FtCollisionBox, GroundOrAir};

/// ftMr_MS_SpecialLw (349) and ftMr_MS_SpecialAirLw (350).
pub const GROUND: ActionId = ActionId(349);
pub const AIR: ActionId = ActionId(350);

/// efSync_Spawn(0x47C, gobj, root): efAlt's efLib_Create_Attach_Scale(0x3E9)
/// with the efLib_Cb_ftMr_SpecialLw update.
const TORNADO_EFFECT: u16 = 0x47C;

/// ftmariospeciallw.c coll_box: top 12, bottom 0, left (-6, 6), right (6, 6).
const TORNADO_BOX: FtCollisionBox = FtCollisionBox {
    top: 12.0,
    bottom: 0.0,
    left: hsd_types::Vec2 { x: -6.0, y: 6.0 },
    right: hsd_types::Vec2 { x: 6.0, y: 6.0 },
};

/// transition_flags: KeepGfx | SkipHit | SkipMatAnim | UpdateCmd |
/// SkipColAnim | SkipItemVis | Unk19 | SkipModelPartVis | SkipModelFlags |
/// Unk27.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(
    MotionEntryFlags::KEEP_GFX.0
        | MotionEntryFlags::SKIP_HIT.0
        | MotionEntryFlags::SKIP_MAT_ANIM.0
        | MotionEntryFlags::UPDATE_CMD.0
        | MotionEntryFlags::SKIP_COL_ANIM.0
        | MotionEntryFlags::SKIP_ITEM_VIS.0
        | MotionEntryFlags::SKIP_MODEL_PART_VIS.0,
);

/// fp->mv.mr.SpecialLw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tornado {
    /// +2340 groundVelX: the slowdown accumulated since cmd_vars[0].
    pub slowdown: f32,
    /// +2344: speciallw.unk0 + 1, set and never read.
    pub unused_counter: i32,
    /// +234C isUnkColl: the last collision touched the floor.
    pub on_floor: bool,
    /// take_dmg_cb / death2_cb = updateRot until the next motion change.
    pub callbacks: bool,
}

fn attributes(f: &Fighter) -> &crate::attributes::TornadoAttributes {
    &f.character.get::<Mario>().attributes.tornado
}

fn tornado(f: &mut Fighter) -> &mut Tornado {
    &mut f.character.get_mut::<Mario>().tornado
}

/// Both hitlag callbacks pause and resume the fighter's effects.
fn install_hitlag_callbacks(f: &mut Fighter) {
    f.effect_state.hitlag_callbacks = true;
}

/// ftMr_SpecialLw_Enter (800E207C) / ftMr_SpecialAirLw_Enter (800E2194):
/// the aerial row either way, the start's fall speed (less the tap rise
/// unless an aerial rise was spent; 800E20E0: fsubs), the horizontal clamp,
/// doStartMotion.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.variables[2] = 0;
    f.change_motion_state(AIR, a).expect("Tornado assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    let (vel_y, tap, momentum, unknown) = {
        let t = attributes(f);
        (t.vel_y, t.tap_y_vel_max, t.air_momentum_x, t.unknown)
    };
    let charged = air && f.character.get::<Mario>().tornado_charged;
    f.physics.self_velocity.y = vel_y - if charged { 0.0 } else { tap };
    clamp_self_velocity_x(f, momentum);
    // doStartMotion.
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    *tornado(f) = Tornado {
        slowdown: 0.0,
        unused_counter: unknown.wrapping_add(1),
        on_floor: false,
        callbacks: true,
    };
    // setGfx: the model on the root, x2219_b0, and the hitlag callbacks.
    f.effects.push(EffectRequest::SyncAttached {
        id: TORNADO_EFFECT,
        bone: 0,
    });
    f.effect_state.destroy_on_state_change = true;
    install_hitlag_callbacks(f);
}

/// ftCommon_ClampSelfVelX.
fn clamp_self_velocity_x(f: &mut Fighter, maximum: f32) {
    let v = &mut f.physics.self_velocity.x;
    if *v < -maximum {
        *v = -maximum;
    } else if *v > maximum {
        *v = maximum;
    }
}

/// updateRot (the take-damage and death2 callback): the root's X rotation
/// back to zero.
pub fn clear_tilt(f: &mut Fighter) {
    if std::mem::take(&mut tornado(f).callbacks) {
        let root = f.animation.root;
        f.skeleton.set_rotation_x(root, 0.0);
    }
}

/// ftMr_SpecialLw_Anim (800E22BC): Wait at the end.
pub fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        tornado(f).callbacks = false;
        common::finish(f, p.assets, false)?;
    }
    Ok(None)
}

/// ftMr_SpecialAirLw_Anim (800E2308): cmd_vars[1] spends the aerial rise;
/// at the end, Fall with no landing lag, else FallSpecial.
pub fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[1] != 0 {
        f.commands.variables[1] = 0;
        f.character.get_mut::<Mario>().tornado_charged = true;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        tornado(f).callbacks = false;
        let lag = attributes(f).landing_lag;
        if lag == 0 {
            common::finish(f, p.assets, true)?;
        } else {
            // ftCo_80096900(gobj, 1, 0, true, 1, lag).
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag as f32)?;
        }
    }
    Ok(None)
}

/// ftMr_SpecialLw_IASA / ftMr_SpecialAirLw_IASA are empty.
pub fn input(_: &mut Fighter, _: InputPhase<'_>) {}

/// The horizontal speed cap, less the slowdown once cmd_vars[0] started it
/// (800E2420: fsubs, 800E2430: fadds), never below zero.
fn speed_cap(f: &mut Fighter, base: f32) -> f32 {
    let mut cap = base;
    if f.commands.variables[0] != 0 {
        let friction = attributes(f).friction_end;
        let t = tornado(f);
        t.slowdown -= friction;
        cap += t.slowdown;
        if cap < 0.0 {
            cap = 0.0;
        }
    }
    cap
}

/// Fighter_procUpdate's tail after the state's physics callback.
fn finish_update(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        grounded::finish_ground_update(
            &mut f.core.physics,
            &f.core.collision.data,
            &grounded::GroundedParameters::from_attributes(&f.core.attributes, &p.assets.common),
            p.map,
            p.wind,
        );
    } else {
        f.core.finish_air_update(p.assets, p.wind);
    }
}

/// doPhys (800E2464..): leave the ground into the aerial row at this
/// frame, clamped to the tap's fall and horizontal speeds.
fn rise(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[2] = 0;
    let (tap_gravity, momentum) = {
        let t = attributes(f);
        (t.tap_gravity, t.air_momentum_x)
    };
    common::ground_to_air(f, AIR, assets, GROUND_AIR_FLAGS)?;
    // ftCommon_ClampFallSpeed.
    if f.physics.self_velocity.y > tap_gravity {
        f.physics.self_velocity.y = tap_gravity;
    }
    clamp_self_velocity_x(f, momentum);
    install_hitlag_callbacks(f);
    Ok(())
}

/// ftMr_SpecialLw_Phys (800E23E8): the stick drives the ground speed
/// toward the cap (ftCommon_8007CADC), then ApplyGroundMovement; a B tap
/// once cmd_vars[2] opens lifts off (800E247C: fadds).
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (momentum, multiplier, tap) = {
        let t = attributes(f);
        (t.momentum_x, t.momentum_x_mul, t.tap_y_vel_max)
    };
    let cap = speed_cap(f, momentum);
    let stick = f.input.current.stick.x;
    let physics = &mut f.core.physics;
    physics.ground_acceleration =
        accelerate_toward(physics.ground_velocity, stick * multiplier, stick * cap);
    let normal = f.core.collision.data.floor.normal;
    let terrain = p.map.floor_speed_scale(&f.core.collision.data);
    grounded::apply_ground_movement(&mut f.core.physics, normal, terrain);
    if f.commands.variables[2] != 0 && f.input.pressed.intersects(Buttons::B) {
        f.physics.self_velocity.y += tap;
        rise(f, p.assets).expect("Tornado rise");
    }
    finish_update(f, &p);
}

/// ftCommon_8007CA80 / ftCommon_8007D2E8: the acceleration toward
/// `target`, trimmed so the speed does not pass it; a zero target stops.
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

/// ftMr_SpecialAirLw_Phys (800E2510): a B tap rises (ftCommon_Ascend)
/// while the aerial rise is unspent, then ordinary gravity and the stick's
/// drift toward the cap (ftCommon_8007D3A8, threshold 0).
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (tap, tap_gravity, momentum, multiplier) = {
        let t = attributes(f);
        (
            t.tap_y_vel_max,
            t.tap_gravity,
            t.air_momentum_x,
            t.air_momentum_x_mul,
        )
    };
    let charged = f.character.get::<Mario>().tornado_charged;
    if !charged && f.commands.variables[2] != 0 && f.input.pressed.intersects(Buttons::B) {
        // ftCommon_Ascend(fp, tap_y_vel_max, tap_grav).
        let v = &mut f.physics.self_velocity.y;
        *v += tap;
        if *v > tap_gravity {
            *v = tap_gravity;
        }
    }
    let air = &f.attributes.air;
    f.physics.self_velocity.y = airborne::gravity(
        f.physics.self_velocity.y,
        air.gravity,
        air.terminal_velocity,
    );
    let cap = speed_cap(f, momentum);
    let stick = f.input.current.stick.x;
    // ftCommon_8007D3A8(fp, 0, mul, cap): |stick| >= 0 always holds.
    f.physics.animation_velocity.x =
        accelerate_toward(f.physics.self_velocity.x, stick * multiplier, stick * cap);
    finish_update(f, &p);
}

/// doColl: with cmd_vars[3] set and the floor touched, the root tilts to
/// the floor (facing * atan2f(n.x, n.y)); otherwise it stands upright.
/// efLib_Cb_ftMr_SpecialLw tilts the model to the same floor.
fn tilt(f: &mut Fighter) {
    let tilted = f.commands.variables[3] != 0 && tornado(f).on_floor;
    let normal = f.collision.data.floor.normal;
    let angle = if tilted {
        f.physics.facing * melee_lb::trigf::atan2f(normal.x, normal.y)
    } else {
        0.0
    };
    let root = f.animation.root;
    f.skeleton.set_rotation_x(root, angle);
    let model = if tilted {
        -melee_lb::trigf::atan2f(normal.x, normal.y)
    } else {
        0.0
    };
    f.effects.push(EffectRequest::OwnedRotationZ {
        model: 0x3E9,
        rotation: model,
    });
}

/// ftMr_SpecialLw_Coll (800E27D0): ft_80082888 with the Tornado box; off
/// the floor, doPhys.
pub fn ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
        TORNADO_BOX,
        false,
    );
    if supported {
        tornado(f).on_floor = true;
    } else {
        rise(f, p.assets.expect("Tornado collision assets"))?;
        tornado(f).on_floor = false;
    }
    tilt(f);
    Ok(())
}

/// ftMr_SpecialAirLw_Coll (800E2A38): ft_800824A0 with the Tornado box; a
/// landing continues on the ground (doAirCollIfUnk).
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = air::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        TORNADO_BOX,
    );
    if landed {
        land(f, p.assets.expect("Tornado landing assets"))?;
        tornado(f).on_floor = true;
    } else {
        tornado(f).on_floor = false;
    }
    tilt(f);
    Ok(())
}

/// doAirCollIfUnk: land into the grounded row at this frame, the rise
/// spent flag cleared and the ground speed clamped.
fn land(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[2] = 0;
    let momentum = attributes(f).momentum_x;
    f.land();
    f.physics.self_velocity.y = 0.0;
    f.character.get_mut::<Mario>().tornado_charged = false;
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(GROUND, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    // ftCommon_ClampGrVel.
    let v = &mut f.physics.ground_velocity;
    if *v < -momentum {
        *v = -momentum;
    } else if *v > momentum {
        *v = momentum;
    }
    install_hitlag_callbacks(f);
    Ok(())
}
