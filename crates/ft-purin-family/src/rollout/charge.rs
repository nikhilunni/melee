//! The charge: Start (Jigglypuff's rows 346/347, 354/355), Loop (348, 356)
//! and Full (349, 357). The fighter winds up on the spot, spinning faster as
//! the charge grows; letting go of B releases the roll.
use super::{attributes, change, current, flags, model, scratch, RolloutKind, RolloutRow};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter,
    },
    input::Buttons,
    physics::grounded,
};

type AnimResult = Result<Option<WaitChoice>>;

/// `MTXDegToRad(1)` as MWCC rounds it (@230).
pub(super) const DEGREES_TO_RADIANS: f32 = 0.017453292;
/// @229: the charge's nominal speed, so the roll keeps its facing.
const CREEP: f32 = 0.0001;
/// ftCo_800BFFD0(fp, 5, 0): the full-charge flash.
const FULL_CHARGE_FLASH: u8 = 5;

/// Whether `f` is in one of the aerial charge rows.
fn in_air<C: RolloutKind>(f: &Fighter) -> bool {
    matches!(
        current::<C>(f),
        Some(
            RolloutRow::AirStartRight
                | RolloutRow::AirStartLeft
                | RolloutRow::AirLoop
                | RolloutRow::AirFull
        )
    )
}

/// ftPr_SpecialNStart_Anim (8013E014) / ftPr_SpecialAirNStart_Anim
/// (8013EDB0): at the end, the charge loop with a frozen animation.
pub(super) fn start_anim<C: RolloutKind>(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox::<C>(f);
    f.step_animation(p.assets);
    scratch::<C>(f).pending_facing = 0.0;
    if f.animation.frames_remaining(&f.skeleton) {
        return Ok(None);
    }
    let air = in_air::<C>(f);
    change::<C>(
        f,
        if air {
            RolloutRow::AirLoop
        } else {
            RolloutRow::Loop
        },
        flags::START,
        0.0,
        0.0,
        p.assets,
    )?;
    f.animation.frame = 0.0;
    // ftAnim_SetAnimRate: no FreezeState flag, so not deferred.
    f.core.animation.set_rate(&mut f.core.skeleton, 0.0, false);
    // 8013E0B8 fmuls.
    let creep = CREEP * f.physics.facing;
    f.physics.self_velocity.x = creep;
    f.physics.animation_velocity.x = 0.0;
    if !air {
        f.physics.ground_velocity = creep;
        f.physics.ground_acceleration = 0.0;
    }
    model::face_forward(f);
    Ok(None)
}

/// ftPr_SpecialNLoop_Anim (8013E0F0) / ftPr_SpecialAirNChargeLoop_Anim:
/// charge; once full, the Full row (flashing the first time).
pub(super) fn loop_anim<C: RolloutKind>(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox::<C>(f);
    f.step_animation(p.assets);
    model::roll_sound::<C>(f, false);
    scratch::<C>(f).pending_facing = 0.0;
    if charge_up::<C>(f) {
        if !std::mem::replace(&mut scratch::<C>(f).charged, true) {
            f.core
                .install_color_overlay_now(FULL_CHARGE_FLASH, p.assets);
        }
        let full = if in_air::<C>(f) {
            RolloutRow::AirFull
        } else {
            RolloutRow::Full
        };
        let frame = f.animation.frame;
        change::<C>(f, full, flags::HELD, frame, 0.0, p.assets)?;
    }
    spin::<C>(f);
    Ok(None)
}

/// ftPr_SpecialNFull_Anim (8013E2A0) / ftPr_SpecialAirNChargeFull_Anim.
pub(super) fn full_anim<C: RolloutKind>(f: &mut Fighter, p: AnimationPhase<'_>) -> AnimResult {
    model::remember_hitbox::<C>(f);
    f.step_animation(p.assets);
    model::roll_sound::<C>(f, false);
    scratch::<C>(f).pending_facing = 0.0;
    if charge_up::<C>(f) {
        scratch::<C>(f).charged = true;
    }
    spin::<C>(f);
    Ok(None)
}

/// The charge grows by `charge_per_tick` (an integer charge: 8013E148
/// fadds, 8013E14C fctiwz) up to its maximum. True once full.
fn charge_up<C: RolloutKind>(f: &mut Fighter) -> bool {
    let a = attributes::<C>(f);
    let (step, maximum) = (a.charge_per_tick, a.maximum_charge);
    let rollout = scratch::<C>(f);
    rollout.charge = gekko_math::msl::fctiwz(rollout.charge as f32 + step);
    if rollout.charge as f32 >= maximum {
        rollout.charge = gekko_math::msl::fctiwz(maximum);
        return true;
    }
    false
}

/// The spin in place: the roll angle advances with the charge (8013E1FC
/// fmuls, 8013E210 fmuls, 8013E214 fmadds), then the pose.
fn spin<C: RolloutKind>(f: &mut Fighter) {
    let multiplier = attributes::<C>(f).charge_rotation_multiplier;
    let rollout = scratch::<C>(f);
    let step = rollout.charge as f32 * (DEGREES_TO_RADIANS * multiplier);
    rollout.roll_angle = gekko_math::fma::fmadds(rollout.direction, step, rollout.roll_angle);
    model::set_roll::<C>(f);
    model::face_forward(f);
}

/// ftPr_SpecialNLoop_IASA / ftPr_SpecialNFull_IASA (8013FF04 / 80140064):
/// letting go of B releases the roll at the charge's speed.
pub(super) fn charge_input<C: RolloutKind>(f: &mut Fighter, p: InputPhase<'_>) {
    release::<C>(f, p.assets, false).expect("Rollout release");
}

/// ftPr_SpecialAirNChargeLoop_IASA / ChargeFull_IASA (80140350 / 801404B0).
pub(super) fn air_charge_input<C: RolloutKind>(f: &mut Fighter, p: InputPhase<'_>) {
    release::<C>(f, p.assets, true).expect("Rollout release");
}

fn release<C: RolloutKind>(f: &mut Fighter, assets: &FighterAssets, air: bool) -> Result<()> {
    if f.input.current.held.intersects(Buttons::B) {
        return Ok(());
    }
    let frame = f.animation.frame;
    change::<C>(
        f,
        if air {
            RolloutRow::AirRelease
        } else {
            RolloutRow::Release
        },
        flags::CHARGE,
        frame,
        0.0,
        assets,
    )?;
    f.step_animation(assets);
    let a = attributes::<C>(f);
    let (multiplier, initial) = (a.charge_speed_multiplier, a.initial_charge);
    let rollout = scratch::<C>(f);
    // 8013FFBC..C8: fsubs, fsubs, fmuls, fmuls.
    let speed = rollout.direction * (multiplier * (rollout.charge as f32 - initial));
    if air {
        f.physics.self_velocity.x = speed;
    } else {
        f.physics.ground_velocity = speed;
    }
    model::face_forward(f);
    model::set_roll::<C>(f);
    model::sound(f, model::RELEASE_SOUND);
    Ok(())
}

/// ftPr_SpecialNStart_Phys (80140620), and Loop/Full's: held still with a
/// nominal speed in the facing.
pub(super) fn hold_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let physics = &mut f.core.physics;
    physics.ground_acceleration = 0.0;
    physics.ground_velocity = 0.0;
    // 80140638 fmuls.
    physics.self_velocity.x = physics.facing * CREEP;
    physics.self_velocity.y = 0.0;
    physics.animation_velocity.y = 0.0;
    physics.animation_velocity.x = 0.0;
    finish_ground(f, &p);
}

/// Fighter_procUpdate's grounded tail after a physics callback that set
/// the velocities itself.
pub(super) fn finish_ground(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let core = &mut f.core;
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}

/// ftCommon_Fall (8007D4B8) with the roll's gravity and terminal speed.
pub(super) fn fall<C: RolloutKind>(f: &mut Fighter) {
    let a = attributes::<C>(f);
    let (gravity, terminal) = (a.gravity, a.terminal_velocity);
    let velocity = &mut f.physics.self_velocity.y;
    *velocity -= gravity;
    if *velocity < -terminal {
        *velocity = -terminal;
    }
}

/// ftPr_SpecialAirNStart_Phys (80140BE8), and the aerial Loop, Full and
/// End: the roll's own gravity.
pub(super) fn air_fall_physics<C: RolloutKind>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    fall::<C>(f);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ft_80082708: the ordinary grounded pass. False when the floor is lost.
pub(super) fn stays_grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    use melee_ft::collision::ground;
    let c = &mut f.core;
    ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

pub(super) fn assets<'a>(p: &CollisionPhase<'a>) -> &'a FighterAssets {
    p.assets.expect("Rollout collision assets")
}

/// ftPr_SpecialNStart_Coll (80140FA4): off an edge, the aerial start.
pub(super) fn start_collision<C: RolloutKind>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if stays_grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    let row = if scratch::<C>(f).direction == 1.0 {
        RolloutRow::AirStartRight
    } else {
        RolloutRow::AirStartLeft
    };
    let frame = f.animation.frame;
    change::<C>(f, row, flags::GROUND_AIR, frame, 1.0, assets(&p))
}

/// ftPr_SpecialNLoop_Coll / ftPr_SpecialNFull_Coll: off an edge, the
/// aerial counterpart.
pub(super) fn loop_collision<C: RolloutKind>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    charge_off_edge::<C>(f, p, RolloutRow::AirLoop)
}
pub(super) fn full_collision<C: RolloutKind>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    charge_off_edge::<C>(f, p, RolloutRow::AirFull)
}

fn charge_off_edge<C: RolloutKind>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
    row: RolloutRow,
) -> Result<()> {
    if stays_grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    let frame = f.animation.frame;
    change::<C>(f, row, flags::CHARGE, frame, 0.0, assets(&p))?;
    model::face_forward(f);
    model::set_roll::<C>(f);
    Ok(())
}

/// ft_80081D0C: ordinary airborne collision; true on landing.
pub(super) fn lands(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    use melee_ft::collision::air;
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

/// ftPr_SpecialAirNStart_Coll (80141730): landing continues on the ground.
pub(super) fn air_start_collision<C: RolloutKind>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if !lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    let row = if scratch::<C>(f).direction == 1.0 {
        RolloutRow::StartRight
    } else {
        RolloutRow::StartLeft
    };
    let frame = f.animation.frame;
    change::<C>(f, row, flags::GROUND_AIR, frame, 1.0, assets(&p))
}

/// ftPr_SpecialAirNChargeLoop_Coll / ChargeFull_Coll: landing, the
/// grounded counterpart.
pub(super) fn air_loop_collision<C: RolloutKind>(
    f: &mut Fighter,
    p: CollisionPhase<'_>,
) -> Result<()> {
    charge_landing::<C>(f, p, RolloutRow::Loop)
}
pub(super) fn air_full_collision<C: RolloutKind>(
    f: &mut Fighter,
    p: CollisionPhase<'_>,
) -> Result<()> {
    charge_landing::<C>(f, p, RolloutRow::Full)
}

fn charge_landing<C: RolloutKind>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
    row: RolloutRow,
) -> Result<()> {
    if !lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    let frame = f.animation.frame;
    change::<C>(f, row, flags::CHARGE, frame, 0.0, assets(&p))?;
    model::face_forward(f);
    model::set_roll::<C>(f);
    Ok(())
}
