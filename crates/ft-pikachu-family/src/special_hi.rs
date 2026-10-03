//! Quick Attack, ftpikachuspecialhi.c (80125D80..80127368).
//!
//! The start (353 / 356) reads the stick at its end and zips (354 / 357)
//! with the animation frozen on frame 13. At the zip's end (355 / 358) the
//! script's cmd_vars[0] opens a window in which a stick turned far enough
//! from the first zip's direction zips once more.
use crate::{
    common::{self, change, no_input},
    flags::{GROUND_AIR, KEEP_COL_ANIM_HIT_STATUS, KEEP_GFX, SKIP_HIT},
    row, FamilyState as S, PikachuFamily,
};
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::{cosf, sinf, sqrtf},
};
use hsd_types::{Vec2, Vec3};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
    physics::friction,
};
use melee_lb::{shield::angle_xy, trigf::atan2f};
use melee_types::{mp::collide, FtPart};

/// ftPk_MF_SpecialHiStart_Coll: the start's counterparts.
const START_FLAGS: u32 = GROUND_AIR | KEEP_COL_ANIM_HIT_STATUS;
/// ftPk_MF_SpecialHiMove_Coll: the zip's and the ending's counterparts.
const MOVE_FLAGS: u32 = GROUND_AIR | KEEP_GFX | SKIP_HIT;
/// The stick magnitude cap (MAX_STICK_MAG, retail @470).
const MAXIMUM_STICK: f32 = 0.999;
/// `(float) M_PI_2` (retail @350).
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
/// ftPk_SpecialHi_80126E1C: the stick's x must pass this to turn Pikachu
/// (retail @505).
const TURN_STICK: f32 = 0.001;
/// The frozen zip frame, and the frame a second zip passes through first.
const ZIP_FRAME: f32 = 13.0;
const SECOND_ZIP_FRAME: f32 = 12.0;
/// efSync_Spawn(1012, gobj, &pos): the zip's trail (efLib_CreateGenerator
/// 0x48 at the point).
const TRAIL: u16 = 1012;
/// The trail's scatter while zipping: 6*r - 3 on the ground, 10*r - 5 in
/// the air (retail fmsubs).
const GROUND_SCATTER: (f32, f32) = (6.0, 3.0);
const AIR_SCATTER: (f32, f32) = (10.0, 5.0);
/// MTXDegToRad's factor (retail @421), and the right angle the teleport
/// thresholds add to their attribute (@422).
const DEGREES_TO_RADIANS: f32 = 0.017453292;
const RIGHT_ANGLE_DEGREES: f32 = 90.0;

/// mv.pk.specialhi (ftPikachu/types.h).
#[derive(Clone, Debug, Default)]
pub struct QuickAttack {
    /// x0: frames the aerial start hangs before falling.
    pub hang_frames: i32,
    /// x4: frames the current zip still travels; `None` before the first
    /// zip, when mv+4 is still the previous state's word.
    pub zip_frames: Option<i32>,
    /// x8: the second zip has begun.
    pub second_zip: bool,
    /// x10: the first zip's stick.
    pub first_stick: Vec2,
    /// x18: aerial zip collision frames.
    pub air_frames: i32,
    /// x1C / x24: the velocities the ending saved.
    pub saved_velocity: Vec2,
    pub saved_ground_velocity: f32,
}

pub const fn rows<C: PikachuFamily>() -> [MotionRow; 6] {
    [
        row(
            S::SpecialHiStart0,
            start_anim::<C, false>,
            no_input,
            callbacks::physics::guard_on,
            start_ground_collision,
        ),
        row(
            S::SpecialHiStart1,
            zip_anim::<C, false>,
            no_input,
            zip_ground_physics,
            zip_ground_collision::<C>,
        ),
        row(
            S::SpecialHiEnd,
            end_anim::<C, false>,
            no_input,
            end_ground_physics,
            end_ground_collision::<C>,
        ),
        row(
            S::SpecialAirHiStart0,
            start_anim::<C, true>,
            no_input,
            start_air_physics::<C>,
            start_air_collision,
        ),
        row(
            S::SpecialAirHiStart1,
            zip_anim::<C, true>,
            no_input,
            zip_air_physics::<C>,
            zip_air_collision::<C>,
        ),
        row(
            S::SpecialAirHiEnd,
            end_anim::<C, true>,
            no_input,
            end_air_physics::<C>,
            end_air_collision::<C>,
        ),
    ]
}

fn attributes<C: PikachuFamily>(f: &Fighter) -> &crate::attributes::QuickAttackAttributes {
    &f.character.get::<C>().attributes().quick_attack
}
fn scratch<C: PikachuFamily>(f: &mut Fighter) -> &mut QuickAttack {
    &mut f.character.get_mut::<C>().specials().quick_attack
}

/// ftPk_SpecialHi_Enter (80125DE8) / ftPk_SpecialAirHi_Enter (80125E60).
pub fn enter<C: PikachuFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    f.commands.variables[0] = 0;
    let hang = attributes::<C>(f).air_start_hang_frames;
    let scratch = scratch::<C>(f);
    scratch.hang_frames = hang;
    scratch.zip_frames = None;
    scratch.second_zip = false;
    scratch.air_frames = 0;
    f.physics.ground_velocity = 0.0;
    f.physics.self_velocity.y = 0.0;
    f.physics.self_velocity.x = 0.0;
    let state = if air {
        S::SpecialAirHiStart0
    } else {
        S::SpecialHiStart0
    };
    change(f, state.action(), 0, 0.0, 1.0, assets).expect("Quick Attack assets");
    f.step_animation(assets);
}

/// ftPk_SpecialHiStart0_Anim / ftPk_SpecialAirHiStart0_Anim: at the
/// start's end, the zip.
fn start_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if AIR {
            zip_air::<C>(f, p.assets, p.rng)?;
        } else {
            zip_ground::<C>(f, p.assets, p.map, p.rng)?;
        }
    }
    Ok(None)
}

/// The stick's magnitude: fmuls, fmuls, fadds, then the inlined sqrtf.
fn stick_magnitude(f: &Fighter) -> f32 {
    let stick = f.input.current.stick;
    sqrtf(stick.x * stick.x + stick.y * stick.y)
}

/// The shared tail of both zips: its duration, every jump spent, and the
/// zip motion frozen on frame 13, after a pass through frame 12 for a
/// second zip (whose speed takes the decay first).
///
/// The pass through frame 12 (retail 0x80126FE4 in the air, 0x80126D8C on
/// the ground) runs the zip script up to it, which queues the cheek spark
/// (efAsync, 1042); the change to frame 13 that follows (0x8012700C,
/// 0x80126DB4) flushes that queue (Fighter_ChangeMotionState's
/// efAsync_QueueFlush, 0x800694A0), so the spark spawns here, in the
/// animation proc, and not at this fighter's link-9 flush behind an earlier
/// fighter's.
fn begin_zip<C: PikachuFamily>(
    f: &mut Fighter,
    state: S,
    decay: impl FnOnce(&mut Fighter, f32),
    assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let a = attributes::<C>(f);
    let (duration, multiplier) = (a.zip_frames, a.second_zip_multiplier);
    let second = {
        let scratch = scratch::<C>(f);
        scratch.zip_frames = Some(duration);
        scratch.second_zip
    };
    // stb of co_attrs.max_jumps into x1968.
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    if second {
        decay(f, multiplier);
        change(f, state.action(), KEEP_GFX, SECOND_ZIP_FRAME, 1.0, assets)?;
        f.step_animation(assets);
        f.seal_issued_graphics(assets, rng);
    }
    change(
        f,
        state.action(),
        KEEP_GFX | SKIP_HIT,
        ZIP_FRAME,
        1.0,
        assets,
    )?;
    f.step_animation(assets);
    // ftAnim_SetAnimRate(0): x2223_b0 was cleared by the change.
    f.core.animation.set_rate(&mut f.core.skeleton, 0.0, false);
    // x2223_b4 (ft_80081A00's item landing) and x21F8 (a cape turnaround's
    // ftPk_SpecialHi_UpdateVel) are not modelled.
    Ok(())
}

/// ftPk_SpecialHi_80126C0C (80126C0C): a grounded zip along the floor when
/// the stick is past the minimum (at most 0.999) and points off the
/// floor's plane side, unless the floor is a platform the zip drops
/// through (ftCo_8009A134); otherwise Pikachu leaves the floor with every
/// jump spent (ftCommon_8007D60C) and zips in the air.
fn zip_ground<C: PikachuFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    map: &melee_mp::CollMap,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let mut magnitude = stick_magnitude(f);
    if magnitude > MAXIMUM_STICK {
        magnitude = MAXIMUM_STICK;
    }
    let a = attributes::<C>(f);
    let (minimum, per_stick, base) = (a.minimum_stick, a.speed_per_stick, a.base_speed);
    if magnitude >= minimum {
        let stick = f.input.current.stick;
        let direction = Vec3::new(stick.x, stick.y, 0.0);
        if angle_xy(f.collision.data.floor.normal, direction) >= HALF_PI
            && !f.skip_platform_floor(map)
        {
            // ftCommon_UpdateFacing.
            f.physics.facing = if stick.x >= 0.0 { 1.0 } else { -1.0 };
            scratch::<C>(f).first_stick = Vec2::new(stick.x, stick.y);
            // Retail 80126D3C fmadds, then fmuls by the facing.
            f.physics.ground_velocity = fmadds(per_stick, magnitude, base);
            f.physics.ground_velocity *= f.physics.facing;
            return begin_zip::<C>(
                f,
                S::SpecialHiStart1,
                |f, multiplier| f.physics.ground_velocity *= multiplier,
                assets,
                rng,
            );
        }
    }
    f.leave_ground_with_spent_jumps();
    zip_air::<C>(f, assets, rng)
}

/// ftPk_SpecialHi_80126E1C (80126E1C): an aerial zip along the stick when
/// it is past the minimum, else straight up (turning only past the
/// deadzone, ftCommon_8007DA24). Speed per stick plus base (retail
/// fmadds), times the angle's cosine and sine (fmuls).
fn zip_air<C: PikachuFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let mut magnitude = stick_magnitude(f);
    if magnitude > MAXIMUM_STICK {
        magnitude = MAXIMUM_STICK;
    }
    let a = attributes::<C>(f);
    let (minimum, per_stick, base) = (a.minimum_stick, a.speed_per_stick, a.base_speed);
    let stick = f.input.current.stick;
    let angle = if magnitude > minimum {
        if stick.x.abs() > TURN_STICK {
            f.physics.facing = if stick.x >= 0.0 { 1.0 } else { -1.0 };
        }
        let angle = atan2f(stick.y, stick.x * f.physics.facing);
        scratch::<C>(f).first_stick = Vec2::new(stick.x, stick.y);
        angle
    } else {
        if stick.x.abs() > assets.input.thresholds.horizontal_stick_deadzone {
            f.physics.facing = if stick.x >= 0.0 { 1.0 } else { -1.0 };
        }
        magnitude = MAXIMUM_STICK;
        scratch::<C>(f).first_stick = Vec2::new(0.0, MAXIMUM_STICK);
        HALF_PI
    };
    let cosine = cosf(angle);
    let horizontal = fmadds(per_stick, magnitude, base) * cosine;
    f.physics.self_velocity.x = f.physics.facing * horizontal;
    let sine = sinf(angle);
    f.physics.self_velocity.y = fmadds(per_stick, magnitude, base) * sine;
    begin_zip::<C>(
        f,
        S::SpecialAirHiStart1,
        |f, multiplier| {
            f.physics.self_velocity.x *= multiplier;
            f.physics.self_velocity.y *= multiplier;
        },
        assets,
        rng,
    )
}

/// ftPk_SpecialAirHiStart0_Phys (80125FA4): the hang, then the start's
/// gravity; ftCommon_8007CF58's friction.
fn start_air_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let gravity = attributes::<C>(f).air_start_gravity;
    let scratch = scratch::<C>(f);
    if scratch.hang_frames != 0 {
        scratch.hang_frames -= 1;
    } else {
        let terminal = f.attributes.air.terminal_velocity;
        common::fall(f, gravity, terminal);
    }
    let air = &f.attributes.air;
    f.physics.animation_velocity.x = friction::air_drift_friction_acceleration(
        f.physics.self_velocity.x,
        air.aerial_friction,
        air.air_drift_max,
        p.assets.common.over_drift_air_friction,
    );
    common::finish_air(f, &p);
}

/// ftPk_SpecialHiStart0_Coll: off the floor, ftPk_SpecialHi_ChangeMotion_
/// Unk00 (every jump spent, ftCommon_8007D60C).
fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        f.leave_ground_with_spent_jumps();
        let frame = f.animation.frame;
        let assets = p.assets.expect("Quick Attack collision assets");
        change(
            f,
            S::SpecialAirHiStart0.action(),
            START_FLAGS,
            frame,
            1.0,
            assets,
        )?;
    }
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

/// ftPk_SpecialAirHiStart0_Coll: landing, ftPk_SpecialHi_ChangeMotion_
/// Unk01; otherwise a ledge.
fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Quick Attack collision assets");
    if lands_facing(f, &mut p) {
        common::air_to_ground(f, S::SpecialHiStart0.action(), START_FLAGS, assets)?;
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}

/// ftPk_SpecialHiStart1_Anim / ftPk_SpecialAirHiStart1_Anim (80126144 /
/// 801262B4): one zip frame; at the last, the ending; every frame the
/// trail at XRotN, scattered while the zip lasts (two HSD_Randf draws).
/// Pichu has no trail.
fn zip_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let frames = {
        let scratch = scratch::<C>(f);
        let frames = scratch.zip_frames.as_mut().expect("zip duration");
        *frames -= 1;
        *frames
    };
    let ended = frames <= 0;
    if ended {
        if AIR {
            enter_air_end::<C>(f, p.assets)?;
        } else {
            enter_ground_end::<C>(f, p.assets)?;
        }
    }
    if !C::QUICK_ATTACK_TRAIL {
        return Ok(None);
    }
    let part = usize::from(
        p.assets.parts.part_to_joint[FtPart::XRotN as usize].expect("Quick Attack XRotN"),
    );
    let mut position = melee_ft::fighter::caches::part_position(
        &mut f.core.skeleton,
        &f.core.animation,
        part,
        Vec3::ZERO,
    );
    if !ended {
        let (scale, offset) = if AIR { AIR_SCATTER } else { GROUND_SCATTER };
        position.x += fmsubs(scale, p.rng.randf(), offset);
        position.y += fmsubs(scale, p.rng.randf(), offset);
    }
    f.effects.push(EffectRequest::PositionalGenerator {
        id: TRAIL,
        position,
    });
    f.effect_state.destroy_on_state_change = true;
    // Fighter_SetEffectHitlagCallbacks.
    f.effect_state.hitlag_callbacks = true;
    Ok(None)
}

/// ftPk_SpecialHiStart1_Phys: ftCommon_ApplyGroundMovement alone.
fn zip_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::move_on_ground(f, &p);
}

/// ftPk_SpecialAirHiStart1_Phys -> ftPk_SpecialHi_8012642C.
fn zip_air_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    pose_along_velocity::<C>(f, p.assets);
    common::finish_air(f, &p);
}

/// The XRotN joint's part index.
fn rotation_part(assets: &FighterAssets) -> usize {
    usize::from(assets.parts.part_to_joint[FtPart::XRotN as usize].expect("Quick Attack XRotN"))
}

/// Pose XRotN: rotation X `angle` and the zip's squash `scale`
/// (HSD_JObjSetScale on the part's joint).
fn pose(f: &mut Fighter, assets: &FighterAssets, angle: f32, scale: Vec3) {
    let part = rotation_part(assets);
    f.core.set_part_rotation(part, Axis::X, angle);
    let joint = f.animation.parts[part].joint;
    f.skeleton.set_scale(joint, &scale);
}

/// ftPk_SpecialHi_8012642C (8012642C): the aerial zip leans along its
/// velocity (retail 80126478 fmadds of the facing and the angle with the
/// fsubs'd offset); every movement source but the velocity is cleared,
/// with the TransN offsets and the stick angle.
fn pose_along_velocity<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let a = attributes::<C>(f);
    let (offset, scale) = (a.air_angle_offset, a.air_scale);
    let velocity = f.physics.self_velocity;
    let angle = fmadds(
        f.physics.facing,
        atan2f(velocity.x, velocity.y),
        offset - HALF_PI,
    );
    pose(f, assets, angle, scale);
    // ftCommon_8007E2FC, keeping self_vel.
    f.core.clear_movement();
    f.physics.self_velocity = velocity;
    // x68C / x6A4 (TransN position and offset) and x6C0 / x6D8, the
    // secondary joint's; x6BC has no port counterpart.
    if let Some(root_motion) = f.animation.root_motion.as_mut() {
        for history in [
            &mut root_motion.primary_history,
            &mut root_motion.secondary_history,
        ] {
            history.position = Vec3::ZERO;
            history.offset = Vec3::ZERO;
        }
    }
}

/// The grounded zip's pose on the floor (ftPk_SpecialHiStart1_Coll and
/// ftPk_SpecialHi_ChangeMotion_Unk03; retail fmadds of the facing and the
/// floor angle with the offset).
fn pose_on_floor<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let a = attributes::<C>(f);
    let (offset, scale) = (a.ground_angle_offset, a.ground_scale);
    let part = rotation_part(assets);
    if f.collision.data.env_flags as u32 & collide::FLOOR_MASK != 0 {
        let normal = f.collision.data.floor.normal;
        let angle = fmadds(f.physics.facing, atan2f(normal.x, normal.y), offset);
        f.core.set_part_rotation(part, Axis::X, angle);
    }
    let joint = f.animation.parts[part].joint;
    f.skeleton.set_scale(joint, &scale);
}

/// ft_80082888 (80082888) with the zip's box, after Fighter_procMap's
/// prologue: false off the floor.
fn grounded_in_box<C: PikachuFamily>(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let ecb = f.character.get::<C>().attributes().quick_attack_box;
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
        ecb,
        false,
    )
}

fn touches_wall(f: &Fighter) -> bool {
    f.collision.data.env_flags as u32 & (collide::LEFT_WALL_MASK | collide::RIGHT_WALL_MASK) != 0
}

/// ftPk_SpecialHiStart1_Coll (80126598): off the floor, a wall ends the
/// zip in the air (ftPk_SpecialHi_MotionChangeUpdateVel_Unk1), anything
/// else continues it there (Unk02); on the floor the pose follows it and
/// a wall ends the zip on the ground.
fn zip_ground_collision<C: PikachuFamily>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    let assets = p.assets.expect("Quick Attack zip collision assets");
    if !grounded_in_box::<C>(f, &mut p) {
        f.leave_ground_with_spent_jumps();
        if touches_wall(f) {
            return enter_air_end::<C>(f, assets);
        }
        // ftPk_SpecialHi_ChangeMotion_Unk02: the aerial zip at rate 0.
        let frame = f.animation.frame;
        change(
            f,
            S::SpecialAirHiStart1.action(),
            MOVE_FLAGS,
            frame,
            0.0,
            assets,
        )?;
        pose_along_velocity::<C>(f, assets);
        return Ok(());
    }
    pose_on_floor::<C>(f, assets);
    if touches_wall(f) {
        enter_ground_end::<C>(f, assets)?;
    }
    Ok(())
}

/// ftPikachu_GetBool: a floor ends the zip once it has flown long enough
/// (the int count against the float attribute), otherwise unless it is a
/// platform the zip drops through (ftCo_8009A134).
fn floor_counts<C: PikachuFamily>(f: &mut Fighter, map: &melee_mp::CollMap) -> bool {
    let frames = scratch::<C>(f).air_frames as f32;
    if frames >= attributes::<C>(f).landing_frames {
        return true;
    }
    !f.skip_platform_floor(map)
}

/// The teleport thresholds: 0.017453292 * (90 + degrees) (retail fadds of
/// the int as float, then fmuls).
fn surface_threshold(degrees: i32) -> f32 {
    DEGREES_TO_RADIANS * (RIGHT_ANGLE_DEGREES + degrees as f32)
}

/// ftPk_SpecialAirHiStart1_Coll (801267C8): a floor met steeply ends the
/// zip on the ground (ftCommon_8007D7FC, Unk0), shallowly the zip slides
/// on along it (Unk03); otherwise a ledge, or a ceiling or wall met
/// steeply ends it in the air (ftCommon_HandleTeleportCollisions, which
/// may end it once per surface).
fn zip_air_collision<C: PikachuFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Quick Attack zip collision assets");
    scratch::<C>(f).air_frames += 1;
    let threshold = surface_threshold(attributes::<C>(f).floor_angle_degrees);
    if lands_facing(f, &mut p) && floor_counts::<C>(f, p.map) {
        let angle = angle_xy(f.collision.data.floor.normal, f.physics.self_velocity);
        f.land();
        if angle > threshold {
            return enter_ground_end::<C>(f, assets);
        }
        // ftPk_SpecialHi_ChangeMotion_Unk03: the grounded zip at rate 0.
        let frame = f.animation.frame;
        change(
            f,
            S::SpecialHiStart1.action(),
            MOVE_FLAGS,
            frame,
            0.0,
            assets,
        )?;
        pose_on_floor::<C>(f, assets);
        return Ok(());
    }
    if f.try_grab_ledge(assets, p.map)? {
        return Ok(());
    }
    let data = &f.collision.data;
    let flags = data.env_flags as u32;
    let surfaces = [
        (collide::CEILING_MASK, data.ceiling.normal),
        (collide::LEFT_WALL_MASK, data.left_facing_wall.normal),
        (collide::RIGHT_WALL_MASK, data.right_facing_wall.normal),
    ];
    for (mask, normal) in surfaces {
        if flags & mask != 0 && angle_xy(normal, f.physics.self_velocity) > threshold {
            enter_air_end::<C>(f, assets)?;
        }
    }
    Ok(())
}

/// ftPk_SpecialHi_MotionChangeUpdateVel_Unk0 (801274AC): the grounded
/// ending keeps its ground speed times the attribute (fmuls).
fn enter_ground_end<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let multiplier = attributes::<C>(f).end_velocity_multiplier;
    save_velocity::<C>(f);
    let saved = scratch::<C>(f).saved_ground_velocity;
    f.physics.ground_velocity = saved * multiplier;
    change(f, S::SpecialHiEnd.action(), KEEP_GFX, 0.0, 1.0, assets)
}

/// ftPk_SpecialHi_MotionChangeUpdateVel_Unk1 (80127534): the aerial ending
/// keeps both components times the attribute (fmuls).
fn enter_air_end<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let multiplier = attributes::<C>(f).end_velocity_multiplier;
    save_velocity::<C>(f);
    let saved = scratch::<C>(f).saved_velocity;
    f.physics.self_velocity.x = saved.x * multiplier;
    f.physics.self_velocity.y = saved.y * multiplier;
    change(f, S::SpecialAirHiEnd.action(), KEEP_GFX, 0.0, 1.0, assets)
}

/// The shared head of both endings: every velocity saved, then zeroed.
fn save_velocity<C: PikachuFamily>(f: &mut Fighter) {
    let (velocity, ground) = (f.physics.self_velocity, f.physics.ground_velocity);
    let scratch = scratch::<C>(f);
    scratch.saved_velocity = Vec2::new(velocity.x, velocity.y);
    scratch.saved_ground_velocity = ground;
    f.physics.self_velocity.y = 0.0;
    f.physics.self_velocity.x = 0.0;
    f.physics.ground_velocity = 0.0;
}

/// ftPk_SpecialHi_80127064 (80127064): a second zip needs the stick past
/// the minimum (uncapped here) and, before one has happened, turned more
/// than the attribute's degrees from the first zip's stick.
fn wants_second_zip<C: PikachuFamily>(f: &mut Fighter) -> bool {
    let magnitude = stick_magnitude(f);
    let a = attributes::<C>(f);
    let (minimum, degrees) = (a.minimum_stick, a.second_zip_angle_degrees);
    if magnitude < minimum {
        return false;
    }
    let scratch = scratch::<C>(f).clone();
    if scratch.second_zip {
        return false;
    }
    let stick = f.input.current.stick;
    let current = Vec3::new(stick.x, stick.y, 0.0);
    let first = Vec3::new(scratch.first_stick.x, scratch.first_stick.y, 0.0);
    angle_xy(first, current) > DEGREES_TO_RADIANS * degrees as f32
}

/// ftPk_SpecialHiEnd_Anim / ftPk_SpecialAirHiEnd_Anim: while the script's
/// window is open (cmd_vars[0] == 1) the second zip, else the window
/// closes (2); otherwise at the end Wait, or a special fall with the
/// attribute's drift and landing lag.
fn end_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] == 1 {
        if wants_second_zip::<C>(f) {
            f.commands.variables[0] = 0;
            scratch::<C>(f).second_zip = true;
            if AIR {
                zip_air::<C>(f, p.assets, p.rng)?;
            } else {
                zip_ground::<C>(f, p.assets, p.map, p.rng)?;
            }
        } else {
            f.commands.variables[0] = 2;
        }
        return Ok(None);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        if AIR {
            let a = attributes::<C>(f);
            let (drift, lag) = (a.fall_special_drift, a.landing_lag);
            f.enter_special_fall(p.assets, true, false, true, drift, lag)?;
        } else {
            common::finish(f, false, p.assets)?;
        }
    }
    Ok(None)
}

/// ftPk_SpecialHiEnd_Phys: ft_80084F3C once the script's window opened;
/// before it, only Fighter_procUpdate's tail.
fn end_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        callbacks::physics::guard_on(f, p);
    } else {
        common::finish_ground(f, &p);
    }
}

/// ftPk_SpecialAirHiEnd_Phys (80127284): once the window opened, ordinary
/// gravity with the drift clamped to the attribute's share (fmuls);
/// before it, the rise decays by a ninth (fdivs, fsubs) under aerial
/// friction (ftCommon_8007CEF4).
fn end_air_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        let air = &f.attributes.air;
        let (gravity, terminal, drift) = (air.gravity, air.terminal_velocity, air.air_drift_max);
        common::fall(f, gravity, terminal);
        let multiplier = attributes::<C>(f).end_drift_multiplier;
        common::clamp_self_velocity_x(f, multiplier * drift);
    } else {
        f.physics.self_velocity.y -= f.physics.self_velocity.y / 9.0;
        let aerial = f.attributes.air.aerial_friction;
        common::air_friction(f, aerial);
    }
    common::finish_air(f, &p);
}

/// ftPk_SpecialHiEnd_Coll: off the zip box's floor, the aerial ending
/// (ftPk_SpecialHi_ChangeMotion_Unk04, every jump spent).
fn end_ground_collision<C: PikachuFamily>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if !grounded_in_box::<C>(f, &mut p) {
        f.leave_ground_with_spent_jumps();
        let frame = f.animation.frame;
        let assets = p.assets.expect("Quick Attack collision assets");
        change(
            f,
            S::SpecialAirHiEnd.action(),
            MOVE_FLAGS,
            frame,
            1.0,
            assets,
        )?;
    }
    Ok(())
}

/// ftPk_SpecialAirHiEnd_Coll: ft_8008239C with the zip's box; a landing
/// is LandingFallSpecial with the attribute's lag, else a ledge.
fn end_air_collision<C: PikachuFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Quick Attack collision assets");
    let ecb = f.character.get::<C>().attributes().quick_attack_box;
    let lag = attributes::<C>(f).landing_lag;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let direction = if c.physics.facing < 0.0 { -1 } else { 1 };
    let landed = air::collide_box_catching_ledges(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        ecb,
        direction,
        c.status.ledge_cooldown == 0,
    );
    if landed {
        f.enter_special_landing(assets, false, lag)?;
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}
