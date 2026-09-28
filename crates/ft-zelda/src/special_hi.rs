//! Farore's Wind, ftzeldaspecialhi.c (801396AC..8013A7F0).
//!
//! The start (349 / 352) raises a gust at the hip and a wind model at the
//! root (efSync 0x4F6 on the ground, 0x4F7 in the air); at its end it reads
//! the stick and Zelda travels invisible and intangible (350 / 353, the
//! animation frozen on frame 35) for the attribute's frames, then reappears
//! (351 / 354, efSync 0x505 at her hip). Unlike Sheik's Vanish no item is
//! left behind.
use crate::{
    common,
    init::{Accessory, Zelda},
};
use gekko_math::{
    fma::fmadds,
    msl::{cosf, sinf, sqrtf},
};
use hsd_types::{Vec2, Vec3};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::air,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_lb::{shield::angle_xy, trigf::atan2f};
use melee_types::{combat::HurtStatus, mp::collide, FtPart};

/// ftZd_MS_SpecialHiStart_0 (349) .. ftZd_MS_SpecialAirHi (354).
pub const GROUND_START: ActionId = ActionId(349);
pub const GROUND_TRAVEL: ActionId = ActionId(350);
pub const GROUND_END: ActionId = ActionId(351);
pub const AIR_START: ActionId = ActionId(352);
pub const AIR_TRAVEL: ActionId = ActionId(353);
pub const AIR_END: ActionId = ActionId(354);
/// Their submotions: the starts share ftZd_SM_SpecialHiStart /
/// SpecialAirHiStart with the travels.
pub const ANIMATIONS: [i32; 6] = [303, 303, 304, 305, 305, 306];

/// transition_flags1: ftCommon_GroundAirColl_MF with KeepGfx,
/// KeepColAnimHitStatus and SkipHit.
const TRANSITION_FLAGS: MotionEntryFlags = common::GROUND_AIR_COLLISION_FLAGS;
/// transition_flags0 (ftZd_SpecialHi_8013A648): the same without
/// KeepColAnimHitStatus.
const END_FLAGS: MotionEntryFlags = MotionEntryFlags(
    common::GROUND_AIR_COLLISION_FLAGS.0 & !MotionEntryFlags::KEEP_COL_ANIM_HIT_STATUS.0,
);

/// The travel's frozen frame.
const TRAVEL_FRAME: f32 = 35.0;
/// `(float) M_PI_2`: straight up, and the floor-angle limit.
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
/// ftZd_SpecialHi_8013A244: the stick's x must pass this to turn Zelda.
const TURN_STICK: f32 = 0.001;
/// MTXDegToRad's factor and the right angle the teleport thresholds add.
const DEGREES_TO_RADIANS: f32 = 0.017453292;
const RIGHT_ANGLE_DEGREES: f32 = 90.0;
/// The ending's rise decays by a tenth each frame (fdivs, fsubs).
const END_RISE_DIVISOR: f32 = 10.0;

/// lb_800119DC(&hip, 120, 1.5, 0.02, 60 * (float) M_PI / 180): the gust.
const GUST_FRAMES: i32 = 120;
const GUST_STRENGTH: f32 = 1.5;
const GUST_DECAY: f32 = 0.02;
const GUST_PHASE_STEP: f32 = 60.0 * std::f32::consts::PI / 180.0;

/// efSync_Spawn(1270 / 1271, gobj, fp->parts[0].joint): the wind on the
/// ground and in the air (efLib_Create_Attach_Scale 0x426A / 0x426B).
const GROUND_WIND: u16 = 0x4F6;
const AIR_WIND: u16 = 0x4F7;
/// `fp->parts->joint`: the model root.
const ROOT_JOINT: usize = 0;
/// efSync_Spawn(1285, gobj, &hip): the reappearance puff.
const REAPPEAR_PUFF: u16 = 0x505;
/// `fp->parts[FtPart_HipN]`: joint 4.
const HIP_JOINT: usize = 4;

/// mv.zd.specialhi (ftZelda/types.h).
#[derive(Clone, Copy, Debug, Default)]
pub struct FaroresWind {
    /// x0: travel frames left.
    pub travel_frames: i32,
    /// x4: the stick the travel took.
    pub stick: Vec2,
    /// xC: aerial travel collision frames.
    pub air_frames: i32,
    /// x10 / x18: the travel's velocities, saved at its end.
    pub saved_velocity: Vec2,
    pub saved_ground_velocity: f32,
}

fn attributes(f: &Fighter) -> &crate::attributes::FaroresWindAttributes {
    &f.character.get::<Zelda>().attributes.farores_wind
}
fn scratch(f: &mut Fighter) -> &mut FaroresWind {
    &mut f.character.get_mut::<Zelda>().farores_wind
}
fn install_accessory(f: &mut Fighter, accessory: Accessory) {
    f.character.get_mut::<Zelda>().accessory = accessory;
    f.core.arm_accessory4();
}
fn uninstall_accessory(f: &mut Fighter) {
    f.character.get_mut::<Zelda>().accessory = Accessory::None;
    f.core.accessory4_armed = false;
}

/// ftZd_SpecialHi_Enter (80139834) / ftZd_SpecialAirHi_Enter (801398E8):
/// the start (on the ground from rest; in the air with the momentum
/// divided down, fdivs), then the gust at the hip and the wind accessory.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        let (horizontal, vertical) = {
            let w = attributes(f);
            (
                w.air_horizontal_velocity_divisor,
                w.air_vertical_velocity_divisor,
            )
        };
        f.physics.self_velocity.x /= horizontal;
        f.physics.self_velocity.y /= vertical;
    } else {
        f.physics.ground_velocity = 0.0;
        f.physics.self_velocity.y = 0.0;
        f.physics.self_velocity.x = 0.0;
    }
    f.change_motion_state(if air { AIR_START } else { GROUND_START }, a)
        .expect("Farore's Wind assets");
    f.step_animation(a);
    f.commands.variables[0] = 0;
    scratch(f).air_frames = 0;
    // ftParts_GetBoneIndex(fp, 4): the HipN part's joint.
    let hip = usize::from(a.parts.joint(FtPart::HipN).expect("HipN part"));
    let center = common::joint_position(f, hip);
    f.core
        .commands
        .radial_impulses
        .push(melee_lb::radial_force::RadialImpulse {
            center,
            frames: GUST_FRAMES,
            strength: GUST_STRENGTH,
            decay: GUST_DECAY,
            phase_step: GUST_PHASE_STEP,
        });
    install_accessory(f, Accessory::FaroresWindStart);
}

/// ftZd_SpecialHi_801396AC -> 801396E0: the wind model once per motion
/// (x2219_b0), Fighter_SetEffectHitlagCallbacks; accessory4 uninstalls.
pub fn wind(f: &mut Fighter) {
    if !f.effect_state.destroy_on_state_change {
        let id = if f.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            GROUND_WIND
        } else {
            AIR_WIND
        };
        f.effects.push(EffectRequest::SyncAttached {
            id,
            bone: ROOT_JOINT,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    uninstall_accessory(f);
}

/// ftZd_SpecialHi_8013979C: the reappearance puff at the hip once per
/// motion (x2219_b0); accessory4 uninstalls.
pub fn reappear(f: &mut Fighter) {
    let position = common::joint_position(f, HIP_JOINT);
    if !f.effect_state.destroy_on_state_change {
        f.effects.push(EffectRequest::PositionalGenerator {
            id: REAPPEAR_PUFF,
            position,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    uninstall_accessory(f);
}

/// ftZd_SpecialHiStart_0_Anim / ftZd_SpecialAirHiStart_0_Anim: at the
/// start's end, the travel.
pub fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.motion_state.action == AIR_START {
            travel_air(f, p.assets)?;
        } else {
            travel_ground(f, p.assets, p.map)?;
        }
    }
    Ok(None)
}

/// The stick's magnitude (fmuls, fmuls, fadds, the inlined square root),
/// at most 1.
fn stick_magnitude(f: &Fighter) -> f32 {
    let stick = f.input.current.stick;
    let magnitude = sqrtf(stick.x * stick.x + stick.y * stick.y);
    if magnitude > 1.0 {
        1.0
    } else {
        magnitude
    }
}

/// The travel's speed for `magnitude` (retail 8013A194 / 8013A384 /
/// 8013A3B0: fmadds).
fn speed(f: &Fighter, magnitude: f32) -> f32 {
    let a = attributes(f);
    fmadds(a.speed_per_stick, magnitude, a.base_speed)
}

/// Enter a travel row at frame 35 and freeze it (ftAnim_SetAnimRate 0);
/// then the frames, every jump spent, x2223_b4 (not modelled), the body
/// intangible (ftColl_8007B62C(gobj, 2)) and Zelda hidden.
fn enter_travel(f: &mut Fighter, state: ActionId, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, MotionEntryFlags(0), TRAVEL_FRAME, 1.0)?;
    f.step_animation(assets);
    f.core.animation.set_rate(&mut f.core.skeleton, 0.0, false);
    let frames = attributes(f).travel_frames;
    scratch(f).travel_frames = frames;
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    f.core.set_body_hurt_status(HurtStatus::Intangible);
    f.effect_state.invisible = true;
    Ok(())
}

/// ftZd_SpecialHi_8013A058 (8013A058): a grounded travel along the floor
/// when the stick is past the threshold and points off the floor's plane
/// side, unless the floor is a platform the travel drops through
/// (ftCo_8009A134); otherwise Zelda leaves the floor with every jump spent
/// (ftCommon_8007D60C) and travels in the air.
fn travel_ground(f: &mut Fighter, assets: &FighterAssets, map: &melee_mp::CollMap) -> Result<()> {
    let magnitude = stick_magnitude(f);
    if magnitude >= attributes(f).stick_threshold {
        let stick = f.input.current.stick;
        let direction = Vec3::new(stick.x, stick.y, 0.0);
        if angle_xy(f.collision.data.floor.normal, direction) >= HALF_PI
            && !f.skip_platform_floor(map)
        {
            // ftCommon_UpdateFacing.
            f.physics.facing = if stick.x >= 0.0 { 1.0 } else { -1.0 };
            let angle = atan2f(stick.y, stick.x * f.physics.facing);
            scratch(f).stick = Vec2::new(stick.x, stick.y);
            let horizontal = speed(f, magnitude) * cosf(angle);
            f.physics.ground_velocity = f.physics.facing * horizontal;
            return enter_travel(f, GROUND_TRAVEL, assets);
        }
    }
    f.leave_ground_with_spent_jumps();
    travel_air(f, assets)
}

/// ftZd_SpecialHi_8013A244 (8013A244): an aerial travel along the stick
/// when it is past the threshold, else straight up at full speed (turning
/// only past the deadzone, ftCommon_8007DA24).
fn travel_air(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let mut magnitude = stick_magnitude(f);
    let stick = f.input.current.stick;
    let angle = if magnitude > attributes(f).stick_threshold {
        if stick.x.abs() > TURN_STICK {
            f.physics.facing = if stick.x >= 0.0 { 1.0 } else { -1.0 };
        }
        let angle = atan2f(stick.y, stick.x * f.physics.facing);
        scratch(f).stick = Vec2::new(stick.x, stick.y);
        angle
    } else {
        if stick.x.abs() > assets.input.thresholds.horizontal_stick_deadzone {
            f.physics.facing = if stick.x >= 0.0 { 1.0 } else { -1.0 };
        }
        scratch(f).stick = Vec2::new(0.0, 1.0);
        magnitude = 1.0;
        HALF_PI
    };
    let horizontal = speed(f, magnitude) * cosf(angle);
    f.physics.self_velocity.x = f.physics.facing * horizontal;
    f.physics.self_velocity.y = speed(f, magnitude) * sinf(angle);
    enter_travel(f, AIR_TRAVEL, assets)
}

/// ftZd_SpecialAirHiStart_0_Phys: the start's gravity, then
/// ftCommon_8007CEF4's aerial friction.
pub fn start_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, terminal) = {
        let a = attributes(f);
        (a.air_start_gravity, a.air_start_terminal_velocity)
    };
    common::fall(f, gravity, terminal);
    common::aerial_friction(f);
    common::finish_air(f, &p);
}

/// ftZd_SpecialHiStart_0_Coll: off the floor, ftZd_SpecialHi_80139B44
/// (every jump spent) continues the start in the air.
pub fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Farore's Wind collision assets");
    f.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR_START, assets, TRANSITION_FLAGS, frame, 1.0)?;
    install_accessory(f, Accessory::FaroresWindStart);
    Ok(())
}

/// ft_CheckGroundAndLedge (800822A4) toward the fighter's facing.
fn lands_facing(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_pass(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    )
}

/// ftZd_SpecialAirHiStart_0_Coll: landing continues the start on the
/// ground (ftZd_SpecialHi_80139BB0); otherwise a ledge.
pub fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Farore's Wind collision assets");
    if lands_facing(f, &mut p) {
        common::air_to_ground(f, GROUND_START, TRANSITION_FLAGS, assets)?;
        install_accessory(f, Accessory::FaroresWindStart);
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}

/// ftZd_SpecialHiStart_1_Anim / ftZd_SpecialAirHiStart_1_Anim: one travel
/// frame; at the last, the ending.
pub fn travel_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let frames = {
        let scratch = scratch(f);
        scratch.travel_frames -= 1;
        scratch.travel_frames
    };
    if frames <= 0 {
        if f.motion_state.action == AIR_TRAVEL {
            end_air(f, p.assets)?;
        } else {
            end_ground(f, p.assets)?;
        }
    }
    Ok(None)
}

/// ftZd_SpecialHiStart_1_Phys: ftCommon_ApplyGroundMovement.
pub fn travel_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::move_on_ground(f, &p);
}

/// ftZd_SpecialAirHiStart_1_Phys is empty: the travel keeps its velocity.
pub fn travel_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::finish_air(f, &p);
}

fn touches_wall(f: &Fighter) -> bool {
    f.collision.data.env_flags as u32 & (collide::LEFT_WALL_MASK | collide::RIGHT_WALL_MASK) != 0
}

/// ftZd_SpecialHiStart_1_Coll: off the floor, a wall ends the travel in
/// the air (every jump spent), anything else continues it there
/// (ftZd_SpecialHi_80139F6C); on the floor a wall ends it on the ground.
pub fn travel_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Farore's Wind travel collision assets");
    if !common::grounded(f, &mut p) {
        f.leave_ground_with_spent_jumps();
        if touches_wall(f) {
            return end_air(f, assets);
        }
        let frame = f.animation.frame;
        f.change_motion_state_with_flags(AIR_TRAVEL, assets, TRANSITION_FLAGS, frame, 0.0)?;
        f.effect_state.invisible = true;
        return Ok(());
    }
    if touches_wall(f) {
        end_ground(f, assets)?;
    }
    Ok(())
}

/// The teleport threshold: 0.017453292 * (90 + degrees) (fadds of the int
/// as float, then fmuls).
fn surface_threshold(degrees: i32) -> f32 {
    DEGREES_TO_RADIANS * (RIGHT_ANGLE_DEGREES + degrees as f32)
}

/// ftZd_SpecialAirHiStart_1_Coll (80139DAC): a floor ends the travel on
/// the ground (ftZd_SpecialHi_80139FE8) once it has flown long enough, or
/// unless it is a platform it drops through (ftCo_8009A134); otherwise a
/// ledge, or a ceiling or wall met steeply ends it in the air
/// (ftCommon_HandleTeleportCollisions, once per surface).
pub fn travel_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Farore's Wind travel collision assets");
    scratch(f).air_frames += 1;
    if lands_facing(f, &mut p) {
        let flown = scratch(f).air_frames as f32 >= attributes(f).platform_frames;
        if flown || !f.skip_platform_floor(p.map) {
            f.land();
            let frame = f.animation.frame;
            f.change_motion_state_with_flags(GROUND_TRAVEL, assets, TRANSITION_FLAGS, frame, 0.0)?;
            f.effect_state.invisible = true;
            return Ok(());
        }
    }
    if f.try_grab_ledge(assets, p.map)? {
        return Ok(());
    }
    let threshold = surface_threshold(attributes(f).surface_angle_degrees);
    let data = &f.collision.data;
    let flags = data.env_flags as u32;
    let surfaces = [
        (collide::CEILING_MASK, data.ceiling.normal),
        (collide::LEFT_WALL_MASK, data.left_facing_wall.normal),
        (collide::RIGHT_WALL_MASK, data.right_facing_wall.normal),
    ];
    for (mask, normal) in surfaces {
        if flags & mask != 0 && angle_xy(normal, f.physics.self_velocity) > threshold {
            end_air(f, assets)?;
        }
    }
    Ok(())
}

/// The shared head of both endings: the velocities saved, then zeroed,
/// Zelda visible, the reappearance on accessory4.
fn save_velocity(f: &mut Fighter) {
    let (velocity, ground) = (f.physics.self_velocity, f.physics.ground_velocity);
    let scratch = scratch(f);
    scratch.saved_velocity = Vec2::new(velocity.x, velocity.y);
    scratch.saved_ground_velocity = ground;
    f.physics.self_velocity.y = 0.0;
    f.physics.self_velocity.x = 0.0;
    f.physics.ground_velocity = 0.0;
    f.effect_state.invisible = false;
    install_accessory(f, Accessory::FaroresWindReappear);
}

/// ftZd_SpecialHi_8013A6A8 (8013A6A8): the grounded ending keeps its ground
/// speed times the attribute (fmuls).
fn end_ground(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(GROUND_END, assets)?;
    f.step_animation(assets);
    save_velocity(f);
    let multiplier = attributes(f).end_velocity_multiplier;
    f.physics.ground_velocity = scratch(f).saved_ground_velocity * multiplier;
    Ok(())
}

/// ftZd_SpecialHi_8013A764 (8013A764): the aerial ending keeps both
/// components times the attribute (fmuls).
fn end_air(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(AIR_END, assets)?;
    f.step_animation(assets);
    save_velocity(f);
    let multiplier = attributes(f).end_velocity_multiplier;
    let saved = scratch(f).saved_velocity;
    f.physics.self_velocity.x = saved.x * multiplier;
    f.physics.self_velocity.y = saved.y * multiplier;
    Ok(())
}

/// ftZd_SpecialHi_Anim: Wait at the end.
pub fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftZd_SpecialAirHi_Anim: at the end, a special fall with the attribute's
/// mobility and landing lag (ftCo_80096900(gobj, 1, 0, 1, x68, x6C)).
pub fn end_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let (mobility, lag) = {
            let a = attributes(f);
            (a.freefall_mobility, a.landing_lag)
        };
        f.enter_special_fall(p.assets, true, false, true, mobility, lag)?;
    }
    Ok(None)
}

/// ftZd_SpecialHi_Phys: ft_80084F3C.
pub fn end_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftZd_SpecialAirHi_Phys: once the script's window opened, ordinary
/// gravity with the drift clamped to the attribute's share (fmuls);
/// before it, the rise decays by a tenth (fdivs, fsubs) under aerial
/// friction (ftCommon_8007CEF4).
pub fn end_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        let air = &f.attributes.air;
        let (gravity, terminal, drift) = (air.gravity, air.terminal_velocity, air.air_drift_max);
        common::fall(f, gravity, terminal);
        let multiplier = attributes(f).freefall_speed_multiplier;
        common::clamp_self_velocity_x(f, multiplier * drift);
    } else {
        f.physics.self_velocity.y -= f.physics.self_velocity.y / END_RISE_DIVISOR;
        common::aerial_friction(f);
    }
    common::finish_air(f, &p);
}

/// ftZd_SpecialHi_Coll: off the edge, ftZd_SpecialHi_8013A648 (every jump
/// spent) ends in the air, with no accessory.
pub fn end_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Farore's Wind collision assets");
    f.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR_END, assets, END_FLAGS, frame, 1.0)?;
    Ok(())
}

/// ftZd_SpecialAirHi_Coll: landing, the special landing with the
/// attribute's lag; otherwise a ledge.
pub fn end_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Farore's Wind collision assets");
    if lands_facing(f, &mut p) {
        let lag = attributes(f).landing_lag;
        f.enter_special_landing(assets, false, lag)?;
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}
