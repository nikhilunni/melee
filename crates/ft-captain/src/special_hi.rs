//! Falcon Dive, ftcaptainspecialhi.c (800E47B8..800E5534).
//!
//! The rise (353 grounded, 354 aerial) steers on its own drift while the
//! script's cmd_vars[0] opens the reversal and ledge window; the animation's
//! end falls special. A catch (355) throws the victim (356).
use melee_ft::{
    anim::WaitChoice,
    collision::{ecb::EcbPose, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter,
    },
    physics::{friction, grounded},
};
use melee_types::GroundOrAir;

use crate::init::CaptainFalcon;

/// ftCa_MS_SpecialHi (353) .. ftCa_MS_SpecialHiThrow (356).
pub const GROUND: ActionId = ActionId(353);
pub const AIR: ActionId = ActionId(354);
pub const CATCH: ActionId = ActionId(355);
pub const THROW: ActionId = ActionId(356);

/// mv.ca.specialhi (fp+2340).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FalconDive {
    /// +2340 (u16): specialhi_air_var, copied on entry.
    pub counter: u16,
    /// +2342 bit 0: the throw's drift phase has begun.
    pub throw_drift: bool,
    /// +2342 bit 1: the script opened the reversal/ledge window.
    pub window_open: bool,
    /// +2344 / +2348: the velocity the drift carries between ticks.
    pub velocity: hsd_types::Vec2,
}

fn dive(f: &mut Fighter) -> &mut FalconDive {
    &mut f.character.get_mut::<CaptainFalcon>().falcon_dive
}

fn attributes(f: &Fighter) -> &crate::attributes::FalconDiveAttributes {
    &f.character.get::<CaptainFalcon>().attributes.falcon_dive
}

/// The retained second scratch word (mv+4, velocity.x) in Falcon Dive.
pub fn retained_scratch_word(falcon: &CaptainFalcon, action: ActionId) -> Option<f32> {
    (GROUND.0..=THROW.0)
        .contains(&action.0)
        .then_some(falcon.falcon_dive.velocity.x)
}

/// ftCa_SpecialLw_800E49FC, installed as x21EC: Fighter_ChangeMotionState
/// runs it before the frame-zero commands. No motion change resets the
/// command variables, so running it just before the change is equivalent.
fn begin(f: &mut Fighter) {
    let (counter, command) = {
        let a = attributes(f);
        (a.initial_air_counter, a.initial_command_value)
    };
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    // 800E4A34: __cvt_fp2unsigned. Only its in-range branch (fctiwz) is
    // ported; the attribute is a small non-negative value.
    assert!(
        (0.0..2147483648.0).contains(&command),
        "__cvt_fp2unsigned outside the signed range: {command}"
    );
    f.commands.variables[0] = 0;
    f.commands.variables[1] = gekko_math::msl::fctiwz(command) as u32;
    *dive(f) = FalconDive {
        // 800E4A28: sth of the s32.
        counter: counter as u16,
        ..Default::default()
    };
}

/// ftCa_SpecialHi_Enter (800E4A78) / ftCa_SpecialAirHi_Enter (800E4D0C).
pub fn enter(f: &mut Fighter, airborne: bool, a: &FighterAssets) {
    begin(f);
    f.change_motion_state(if airborne { AIR } else { GROUND }, a)
        .expect("Falcon Dive assets");
    // ftCommon_8007E2D0(fp, 2, grab_cb, NULL, grabbed_cb): the catch.
    f.core.status.special_grab = Some(crate::special_hi_catch::GRAB_TYPE);
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// ftCa_SpecialHi_Anim / ftCa_SpecialAirHi_Anim: FallSpecial at the end.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let (mobility, lag) = {
            let a = attributes(f);
            (a.freefall_mobility, a.landing_lag)
        };
        f.enter_special_fall(p.assets, true, true, false, mobility, lag)?;
    }
    Ok(None)
}

/// ftCa_SpecialHi_IASA / ftCa_SpecialAirHi_IASA: when the script sets
/// cmd_vars[0], open the window and let a firm stick turn the dive.
pub fn input(f: &mut Fighter, _: InputPhase<'_>) {
    if f.commands.variables[0] == 0 {
        return;
    }
    f.commands.variables[0] = 0;
    dive(f).window_open = true;
    let stick = f.input.current.stick.x;
    let magnitude = if stick < 0.0 { -stick } else { stick };
    if magnitude > attributes(f).reverse_stick_threshold {
        // ftCommon_UpdateFacing, then ftPartSetRotY(fp, 0, M_PI_2 * facing).
        f.physics.facing = if stick >= 0.0 { 1.0 } else { -1.0 };
        let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
        let root = f.animation.root;
        f.skeleton.set_rotation_y(root, rotation);
    }
}

/// ftCa_SpecialHi_Phys (800E4BF8), the drift both rises share; no fused
/// sites. Retains its own velocity across ticks and adds TransN's.
pub fn drift(f: &mut Fighter, a: &FighterAssets) {
    let (air_multiplier, speed_multiplier) = {
        let d = attributes(f);
        (d.air_acceleration_multiplier, d.horizontal_speed_multiplier)
    };
    let carried = dive(f).velocity;
    f.physics.self_velocity = hsd_types::Vec3::new(carried.x, carried.y, 0.0);
    let air = &f.core.attributes.air;
    // 800E4C48: fmuls.
    let maximum = speed_multiplier * air.air_drift_max;
    let velocity = f.core.physics.self_velocity.x;
    // ftCommon_8007D050: PlCo +1FC above the maximum, aerial friction below.
    f.core.physics.animation_velocity.x = friction::air_drift_friction_acceleration(
        velocity,
        air.aerial_friction,
        maximum,
        a.common.over_drift_air_friction,
    );
    let over_maximum = velocity.abs() > maximum;
    if !over_maximum {
        // 800E4C6C / 800E4C78: fmuls.
        let acceleration = air.air_drift_stick_mul * air_multiplier;
        let target = air.air_drift_max * speed_multiplier;
        let threshold = a.jumping.multi_jump_drift_threshold;
        f.core.physics.animation_velocity.x = stick_drift(&f.core, threshold, acceleration, target);
    }
    let physics = &f.core.physics;
    // 800E4C8C / 800E4C9C: fadds.
    let velocity = hsd_types::Vec2::new(
        physics.animation_velocity.x + physics.self_velocity.x,
        physics.animation_velocity.y + physics.self_velocity.y,
    );
    dive(f).velocity = velocity;
    root_motion_air(f);
    f.physics.animation_velocity.x = 0.0;
    f.physics.animation_velocity.y = 0.0;
    // 800E4CBC / 800E4CCC: fadds.
    f.physics.self_velocity.x += velocity.x;
    f.physics.self_velocity.y += velocity.y;
}

/// ftCommon_8007D3A8 -> ftCommon_8007D2E8 (8007D2E8): stick drift with no
/// friction; a zero target stops the drift outright.
fn stick_drift(
    c: &melee_ft::fighter::FighterCore,
    threshold: f32,
    maximum_acceleration: f32,
    maximum_target: f32,
) -> f32 {
    let stick = c.input.current.stick.x;
    let (mut acceleration, target) = if stick.abs() >= threshold {
        (stick * maximum_acceleration, stick * maximum_target)
    } else {
        (0.0, 0.0)
    };
    let velocity = c.physics.self_velocity.x;
    if target == 0.0 {
        return -velocity;
    }
    // Retail branches on !(velocity * acceleration < 0), including unordered.
    if (velocity * acceleration).partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        if acceleration > 0.0 {
            if velocity + acceleration > target {
                acceleration = target - velocity;
            }
        } else if velocity + acceleration < target {
            acceleration = target - velocity;
        }
    }
    acceleration
}

/// ft_80085134 (80085134): airborne velocity straight from TransN.
pub(crate) fn root_motion_air(f: &mut Fighter) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("Falcon Dive TransN")
        .primary_history
        .offset;
    f.physics.self_velocity.x = offset.z * f.physics.facing;
    f.physics.self_velocity.y = offset.y;
}

/// Fighter_procUpdate's tail after the state's physics callback.
pub(crate) fn finish_update(f: &mut Fighter, p: &PhysicsPhase<'_>) {
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

/// ftCa_SpecialHi_Phys / ftCa_SpecialAirHi_Phys.
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    drift(f, p.assets);
    finish_update(f, &p);
}

/// ft_80083B68 -> ft_80082578 -> mpColl_800477E0: airborne collision that
/// never lands.
pub(crate) fn stay_airborne(f: &mut Fighter, p: &mut CollisionPhase<'_>) {
    let c = &mut f.core;
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let pose = EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    p.map.air_collide_stay(cd, Some(&|i| pose.position(i)));
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
}

/// ft_CheckGroundAndLedge(gobj, 0) (800822A4): landing, with ledge flags
/// for a facing of zero unless the ledge cooldown blocks them.
fn land_or_ledge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let pose = EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    let landed = if c.status.ledge_cooldown != 0 {
        p.map.air_collide_pass(cd, Some(&|i| pose.position(i)))
    } else {
        melee_mp::set_facing_dir(cd, 0);
        p.map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
    };
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    landed
}

/// ftCa_SpecialHi_Coll / ftCa_SpecialAirHi_Coll (800E4B60).
pub fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        let c = &mut f.core;
        if ground::map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ) != ground::WaitGroundResult::Supported
        {
            f.leave_ground();
        }
        return Ok(());
    }
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let window_open = dive(f).window_open;
    if land_or_ledge(f, &mut p) {
        if window_open {
            let lag = attributes(f).landing_lag;
            f.enter_special_landing(p.assets.expect("Falcon Dive landing assets"), false, lag)?;
        } else {
            stay_airborne(f, &mut p);
        }
    } else if window_open && f.try_grab_ledge(p.assets.expect("Falcon Dive ledge assets"), p.map)? {
        unimplemented!(
            "ftcaptainspecialhi.c:137-138: Falcon Dive ledge grab (ftCliffCommon_80081370 runs again)"
        );
    }
    Ok(())
}
