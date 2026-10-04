//! Teleport, ftmewtwospecialhi.c (801450A0..80146198).
//!
//! The start (353 / 356) shows a dark flash at the waist (efSync 0x4E8);
//! at its end it reads the stick and Mewtwo travels invisible and
//! intangible (354 / 357, the animation frozen on frame 35) for the
//! attribute's frames, then reappears (355 / 358) and, in the air, falls
//! helpless. Farore's Wind (ftzeldaspecialhi.c) has the same shape.
use crate::{
    common,
    init::{Accessory, Mewtwo},
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
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_lb::{shield::angle_xy, trigf::atan2f};
use melee_types::{combat::HurtStatus, mp::collide};

/// ftMt_MS_SpecialHiStart (353) .. ftMt_MS_SpecialAirHi (358).
pub const GROUND_START: ActionId = ActionId(353);
pub const GROUND_TRAVEL: ActionId = ActionId(354);
pub const GROUND_END: ActionId = ActionId(355);
pub const AIR_START: ActionId = ActionId(356);
pub const AIR_TRAVEL: ActionId = ActionId(357);
pub const AIR_END: ActionId = ActionId(358);

/// transition_flags1: ftCommon_GroundAirColl_MF with KeepGfx,
/// KeepColAnimHitStatus and SkipHit.
const TRANSITION_FLAGS: MotionEntryFlags = common::GROUND_AIR_COLLISION_FLAGS;
/// transition_flags0 (ftMt_SpecialHiLost_GroundToAir): the same without
/// KeepColAnimHitStatus.
const END_FLAGS: MotionEntryFlags = MotionEntryFlags(
    common::GROUND_AIR_COLLISION_FLAGS.0 & !MotionEntryFlags::KEEP_COL_ANIM_HIT_STATUS.0,
);

/// The travel's frozen frame.
const TRAVEL_FRAME: f32 = 35.0;
/// `(float) M_PI_2`: straight up, and the floor-angle limit.
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
/// ftMt_SpecialAirHi_Enter's stick_epsilon: the stick's x must pass this
/// to turn Mewtwo.
const TURN_STICK: f32 = 0.001;
/// MTXDegToRad's factor and the right angle the teleport thresholds add.
const DEGREES_TO_RADIANS: f32 = 0.017453292;
const RIGHT_ANGLE_DEGREES: f32 = 90.0;
/// The ending's rise decays by a tenth each frame (fdivs, fsubs).
const END_RISE_DIVISOR: f32 = 10.0;

/// efSync_Spawn(0x4E8, gobj, &waist): efLib_Create_Attach_Pos(0x32CB), the
/// vanishing flash.
const VANISH_FLASH: u16 = 0x4E8;
/// `fp->parts[FtPart_WaistN]`: the parts array indexed by the part enum.
const WAIST_JOINT: usize = 5;

/// mv.mt.SpecialHi (ftMewtwo/types.h).
#[derive(Clone, Copy, Debug, Default)]
pub struct Teleport {
    /// travelFrames: travel frames left.
    pub travel_frames: i32,
    /// stickX / stickY: the stick the travel took.
    pub stick: Vec2,
    /// unk4: aerial travel collision frames.
    pub air_frames: i32,
    /// velX / velY / groundVelX: the travel's velocities, saved at its end.
    pub saved_velocity: Vec2,
    pub saved_ground_velocity: f32,
}

pub const fn rows() -> [MotionRow; 6] {
    [
        common::row(
            GROUND_START,
            307,
            start_anim,
            common::no_input,
            common::ground_friction,
            start_ground_collision,
        ),
        common::row(
            GROUND_TRAVEL,
            309,
            travel_anim,
            common::no_input,
            travel_ground_physics,
            travel_ground_collision,
        ),
        common::row(
            GROUND_END,
            308,
            end_anim,
            common::no_input,
            common::ground_friction,
            end_ground_collision,
        ),
        common::row(
            AIR_START,
            310,
            start_anim,
            common::no_input,
            start_air_physics,
            start_air_collision,
        ),
        common::row(
            AIR_TRAVEL,
            309,
            travel_anim,
            common::no_input,
            travel_air_physics,
            travel_air_collision,
        ),
        common::row(
            AIR_END,
            311,
            end_air_anim,
            common::no_input,
            end_air_physics,
            end_air_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::TeleportAttributes {
    &f.character.get::<Mewtwo>().attributes.teleport
}
fn scratch(f: &mut Fighter) -> &mut Teleport {
    &mut f.character.get_mut::<Mewtwo>().teleport
}
fn install_accessory(f: &mut Fighter, accessory: Accessory) {
    f.character.get_mut::<Mewtwo>().accessory = accessory;
    f.core.arm_accessory4();
}
fn uninstall_accessory(f: &mut Fighter) {
    f.character.get_mut::<Mewtwo>().accessory = Accessory::None;
    f.core.accessory4_armed = false;
}

/// ftMt_SpecialHiStart_Enter (801451DC) / ftMt_SpecialAirHiStart_Enter
/// (80145258): the start (on the ground from rest; in the air with the
/// momentum divided down, fdivs) and the flash accessory.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        let (horizontal, vertical) = {
            let t = attributes(f);
            (
                t.air_horizontal_velocity_divisor,
                t.air_vertical_velocity_divisor,
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
        .expect("Teleport assets");
    f.step_animation(a);
    f.commands.variables[0] = 0;
    scratch(f).air_frames = 0;
    install_accessory(f, Accessory::TeleportStart);
}

/// ftMt_SpecialHi_CreateGFX (801450A0) -> ftMt_SpecialHi_SetStartGFX
/// (801450D4): the flash at the waist once per motion (x2219_b0),
/// Fighter_SetEffectHitlagCallbacks; accessory4 uninstalls.
pub fn vanish_flash(f: &mut Fighter) {
    if !f.effect_state.destroy_on_state_change {
        let position = common::joint_position(f, WAIST_JOINT);
        f.effects.push(EffectRequest::PositionalModel {
            id: VANISH_FLASH,
            position,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    uninstall_accessory(f);
}

/// ftMt_SpecialHi_SetEndGFX (80145164): nothing spawns; x2219_b0 and the
/// hitlag callbacks only; accessory4 uninstalls.
pub fn reappear(f: &mut Fighter) {
    f.effect_state.destroy_on_state_change = true;
    f.effect_state.hitlag_callbacks = true;
    uninstall_accessory(f);
}

/// ftMt_SpecialHiStart_Anim (801452EC) / ftMt_SpecialAirHiStart_Anim
/// (80145328): at the start's end, the travel.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
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

/// The travel's speed for `magnitude` (retail 80145ACC / 80145CD4 /
/// 80145D00: fmadds).
fn speed(f: &Fighter, magnitude: f32) -> f32 {
    let a = attributes(f);
    fmadds(a.speed_per_stick, magnitude, a.base_speed)
}

/// Enter a travel row at frame 35 and freeze it (ftAnim_SetAnimRate 0);
/// then the frames, every jump spent, x2223_b4 (not modelled), the body
/// intangible (ftColl_8007B62C(gobj, 2)) and Mewtwo hidden.
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

/// ftMt_SpecialHi_Enter (80145990): a grounded travel along the floor when
/// the stick is past the threshold and points off the floor's plane side,
/// unless the floor is a platform the travel drops through
/// (ftCo_8009A134); otherwise Mewtwo leaves the floor with every jump spent
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

/// ftMt_SpecialAirHi_Enter (80145B94): an aerial travel along the stick
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

/// ftMt_SpecialAirHiStart_Phys (8014538C): the start's gravity
/// (ftCommon_Fall), then ftCommon_8007CEF4's aerial friction.
fn start_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, terminal) = {
        let a = attributes(f);
        (a.air_start_gravity, a.air_start_terminal_velocity)
    };
    common::fall(f, gravity, terminal);
    common::aerial_friction(f);
    common::finish_air(f, &p);
}

/// ftMt_SpecialHiStart_Coll (801453D0): off the floor,
/// ftMt_SpecialHiStart_GroundToAir (every jump spent) continues the start
/// in the air.
fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Teleport collision assets");
    f.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR_START, assets, TRANSITION_FLAGS, frame, 1.0)?;
    install_accessory(f, Accessory::TeleportStart);
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

/// ftMt_SpecialAirHiStart_Coll (8014540C): landing continues the start on
/// the ground (ftMt_SpecialAirHiStart_AirToGround); otherwise a ledge.
fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Teleport collision assets");
    if lands_facing(f, &mut p) {
        common::air_to_ground(f, GROUND_START, TRANSITION_FLAGS, assets)?;
        install_accessory(f, Accessory::TeleportStart);
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}

/// ftMt_SpecialHiLost_Anim (80145554) / ftMt_SpecialAirHiLost_Anim
/// (80145590): one travel frame; at the last, the ending.
fn travel_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let frames = {
        let scratch = scratch(f);
        scratch.travel_frames -= 1;
        scratch.travel_frames
    };
    if frames <= 0 {
        common::seal_graphics(f, p.assets, p.rng);
        if f.motion_state.action == AIR_TRAVEL {
            end_air(f, p.assets)?;
        } else {
            end_ground(f, p.assets)?;
        }
    }
    Ok(None)
}

/// ftMt_SpecialHiLost_Phys (801455D4): ftCommon_ApplyGroundMovement.
fn travel_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::move_on_ground(f, &p);
}

/// ftMt_SpecialAirHiLost_Phys (801455F4) is empty: the travel keeps its
/// velocity.
fn travel_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::finish_air(f, &p);
}

fn touches_wall(f: &Fighter) -> bool {
    f.collision.data.env_flags as u32 & (collide::LEFT_WALL_MASK | collide::RIGHT_WALL_MASK) != 0
}

/// ftMt_SpecialHiLost_Coll (801455F8): off the floor, a wall ends the
/// travel in the air (every jump spent), anything else continues it there
/// (ftMt_SpecialHi_GroundToAir); on the floor a wall ends it on the ground.
fn travel_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Teleport travel collision assets");
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

/// ftMt_SpecialAirHiLost_Coll (80145698): a floor ends the travel on the
/// ground (ftMt_SpecialAirHi_AirToGround) once it has flown long enough, or
/// unless it is a platform it drops through (ftCo_8009A134); otherwise a
/// ledge, or a ceiling or wall met steeply ends it in the air
/// (ftCommon_HandleTeleportCollisions, once per surface).
fn travel_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Teleport travel collision assets");
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

/// ftMewtwo_SpecialHiLost_SetVars: the velocities saved, then zeroed,
/// Mewtwo visible, ftMt_SpecialHi_SetEndGFX on accessory4.
fn save_velocity(f: &mut Fighter) {
    let (velocity, ground) = (f.physics.self_velocity, f.physics.ground_velocity);
    let scratch = scratch(f);
    scratch.saved_velocity = Vec2::new(velocity.x, velocity.y);
    scratch.saved_ground_velocity = ground;
    f.physics.self_velocity.y = 0.0;
    f.physics.self_velocity.x = 0.0;
    f.physics.ground_velocity = 0.0;
    f.effect_state.invisible = false;
    install_accessory(f, Accessory::TeleportReappear);
}

/// ftMt_SpecialHiLost_Enter (80146010): the grounded ending keeps its
/// ground speed times the attribute (fmuls).
fn end_ground(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(GROUND_END, assets)?;
    f.step_animation(assets);
    save_velocity(f);
    let multiplier = attributes(f).end_velocity_multiplier;
    f.physics.ground_velocity = scratch(f).saved_ground_velocity * multiplier;
    Ok(())
}

/// ftMt_SpecialAirHiLost_Enter (801460CC): the aerial ending keeps both
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

/// ftMt_SpecialHi_Anim (80145DB0): Wait at the end.
fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftMt_SpecialAirHi_Anim (80145DEC): at the end, a special fall with the
/// attribute's mobility and landing lag (ftCo_80096900(gobj, 1, 0, 1, x70,
/// x74)).
fn end_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let (mobility, lag) = {
            let a = attributes(f);
            (a.freefall_mobility, a.landing_lag)
        };
        common::seal_graphics(f, p.assets, p.rng);
        f.enter_special_fall(p.assets, true, false, true, mobility, lag)?;
    }
    Ok(None)
}

/// ftMt_SpecialAirHi_Phys (80145E74): once the script's window opened,
/// ordinary gravity (ftCommon_FallBasic) with the drift clamped to the
/// attribute's share (fmuls); before it, the rise decays by a tenth (fdivs,
/// fsubs) under aerial friction (ftCommon_8007CEF4).
fn end_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
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

/// ftMt_SpecialHi_Coll (80145EF0): off the edge,
/// ftMt_SpecialHiLost_GroundToAir (every jump spent) ends in the air, with
/// no accessory.
fn end_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Teleport collision assets");
    f.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR_END, assets, END_FLAGS, frame, 1.0)?;
    Ok(())
}

/// ftMt_SpecialAirHi_Coll (80145F2C): landing, the special landing with the
/// attribute's lag; otherwise a ledge.
fn end_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Teleport collision assets");
    if lands_facing(f, &mut p) {
        let lag = attributes(f).landing_lag;
        f.enter_special_landing(assets, false, lag)?;
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}
