//! Final Cutter, ftkirbyspecialhi.c (800F21E8..800F3318).
//!
//! The start (385 on the ground, 389 in the air) draws the blade (efAlt
//! 0x494) and may turn Kirby round once. Its end is the rise (390): the
//! animation's TransN carries him up for at least 21 ticks before the
//! ledge-aware landing check starts. The fall (391) keeps the rise's last
//! velocity until a floor or a ledge. The landing (388, or 392 off the
//! floor's end) tips the model with the floor and, on the script's flag,
//! sends the beam out and lets the blade expire.
//!
//! Rows 386 and 387 (ftKb_MS_SpecialHi2 / SpecialHi3) are in the table but
//! nothing enters them: every motion change in this file goes to 385, 388,
//! 389, 390, 391 or 392. They stay unported rows.
use crate::{common, init::Kirby};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{GroundOrAir, ItemKind};

/// ftKb_MS_SpecialHi1 and SpecialHi4 (385, 388), SpecialAirHi1..4
/// (389..392).
pub const START: ActionId = ActionId(385);
pub const LANDING: ActionId = ActionId(388);
pub const AIR_START: ActionId = ActionId(389);
pub const RISE: ActionId = ActionId(390);
pub const FALL: ActionId = ActionId(391);
pub const AIR_LANDING: ActionId = ActionId(392);

/// The animation callbacks' changes (0xA): KeepGfx and SkipHit.
const NEXT_STAGE_FLAGS: MotionEntryFlags = MotionEntryFlags(1 << 1 | 1 << 3);
/// ftKb_MF_SpecialHi_Coll (0xC1082): KeepGfx, SkipMatAnim, SkipColAnim,
/// SkipItemVis and Unk19.
const COLLISION_FLAGS: MotionEntryFlags =
    MotionEntryFlags(1 << 1 | 1 << 7 | 1 << 12 | 1 << 18 | 1 << 19);

/// efAlt 0x494's joints: `fp->parts[44]` (the sword hand) and
/// `fp->parts[1]`, read as raw part indices (efalt.c:225-227).
const BLADE_EFFECT: u16 = 0x494;
const BLADE_JOINTS: [usize; 2] = [44, 1];
/// The blade's models that face with Kirby (efLib_Cb_SetRotYAndTransition
/// with the fighter as user data).
const FACING_BLADE_MODELS: [u32; 2] = [0x1389, 0x138A];
/// The rise's ticks before a floor or ledge can end it
/// (ftKb_SpecialAirHi2_Coll: `x0 > 0x14`).
const RISE_LOCK_TICKS: i32 = 0x14;
/// fn_800F21E8 lowers the beam's spawn point by one unit after the
/// attribute offsets.
const BEAM_DROP: f32 = 1.0;

/// fp->mv.kb.specialhi.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FinalCutter {
    /// x0: collision passes of the rise so far.
    pub rise_ticks: i32,
    /// x4: Kirby has turned round.
    pub turned: bool,
    /// xC: the beam is out.
    pub beam_spawned: bool,
}

pub const fn rows() -> [MotionRow; 6] {
    [
        common::row(
            START,
            324,
            start_anim,
            turn_input,
            start_physics,
            start_collision,
        ),
        // ftKb_SpecialHi4_Phys (800F2A48): ft_80084FA8.
        common::row(
            LANDING,
            327,
            landing_anim,
            common::no_input,
            callbacks::physics::jab,
            landing_collision,
        ),
        common::row(
            AIR_START,
            328,
            air_start_anim,
            turn_input,
            rise_physics,
            air_start_collision,
        ),
        common::row(
            RISE,
            329,
            rise_anim,
            common::no_input,
            rise_physics,
            rise_collision,
        ),
        common::row(
            FALL,
            330,
            fall_anim,
            common::no_input,
            fall_physics,
            fall_collision,
        ),
        common::row(
            AIR_LANDING,
            331,
            landing_anim,
            common::no_input,
            air_landing_physics,
            air_landing_collision,
        ),
    ]
}

fn scratch(f: &mut Fighter) -> &mut FinalCutter {
    &mut f.character.get_mut::<Kirby>().final_cutter
}

/// ftKb_SpecialN_set_cbs (ftKirby/inlines.h): death2_cb and take_dmg_cb.
fn install_damage_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Kirby>().damage_callbacks = true;
}

/// `fp->accessory4_cb = fn_800F21E8`.
fn install_beam_accessory(f: &mut Fighter) {
    f.core.arm_accessory4();
    f.character.get_mut::<Kirby>().accessory = crate::init::Accessory::CutterBeam;
}

fn set_root_tilt(f: &mut Fighter, angle: f32) {
    f.core.set_part_rotation(0, Axis::X, angle);
}

/// ftPartSetRotX(fp, 0, facing * atan2f(floor.normal.x, floor.normal.y))
/// (800F2D70: fmuls).
fn tilt_with_floor(f: &mut Fighter) {
    let normal = f.collision.data.floor.normal;
    let angle = f.physics.facing * melee_lb::trigf::atan2f(normal.x, normal.y);
    set_root_tilt(f, angle);
}

/// ftKb_AttackDashAir_800F22D4 (800F22D4): the model root level again.
pub fn level_root(f: &mut Fighter) {
    set_root_tilt(f, 0.0);
}

/// ftKb_SpecialHi_Enter (800F2300) / ftKb_SpecialAirHi_Enter (800F23B8).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    f.commands.variables = [0; 4];
    *scratch(f) = FinalCutter::default();
    f.change_motion_state(if air { AIR_START } else { START }, assets)
        .expect("Final Cutter assets");
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    f.effects.push(EffectRequest::SyncAttachedPair {
        id: BLADE_EFFECT,
        bones: BLADE_JOINTS,
    });
    f.core.effect_state.destroy_on_state_change = true;
    f.core.effect_state.hitlag_callbacks = true;
}

/// The next stage from an animation callback: frame 0, rate 1, then
/// Fighter_SetEffectHitlagCallbacks.
fn next_stage(f: &mut Fighter, state: ActionId, p: &mut AnimationPhase<'_>) -> Result<()> {
    common::seal_graphics(f, p.assets, p.rng);
    f.change_motion_state_with_flags(state, p.assets, NEXT_STAGE_FLAGS, 0.0, 1.0)?;
    f.core.effect_state.hitlag_callbacks = true;
    Ok(())
}

/// ftKb_SpecialHi1_Anim (800F2470): at the end Kirby leaves the ground
/// (ftCommon_8007D5D4) and rises.
fn start_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.leave_ground();
        next_stage(f, RISE, &mut p)?;
        scratch(f).rise_ticks = 0;
    }
    Ok(None)
}

/// ftKb_SpecialAirHi1_Anim (800F25EC).
fn air_start_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        next_stage(f, RISE, &mut p)?;
        scratch(f).rise_ticks = 0;
    }
    Ok(None)
}

/// ftKb_SpecialAirHi2_Anim (800F266C): the fall at the rise's end.
fn rise_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        next_stage(f, FALL, &mut p)?;
    }
    Ok(None)
}

/// ftKb_SpecialAirHi3_Anim (800F26E4) is empty: the fall lasts until the
/// collision callback ends it.
fn fall_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftKb_SpecialHi4_Anim (800F257C) / ftKb_SpecialAirHiEnd_Anim (800F26E8):
/// at the end the root is level again, then Wait on the ground
/// (ft_8008A2BC) or Fall.
fn landing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        level_root(f);
        if f.physics.ground_or_air == GroundOrAir::Ground {
            common::finish(f, false, p.assets)?;
        } else {
            common::enter_fall(f, p.assets)?;
        }
    }
    Ok(None)
}

/// ftKb_SpecialHi1_IASA (800F2758) / ftKb_SpecialAirHi1_IASA (800F2834):
/// until the script's cmd_vars[3], a stick past the attribute range and
/// away from the facing turns Kirby round, once.
fn turn_input(f: &mut Fighter, _: InputPhase<'_>) {
    if f.commands.variables[3] != 0 || scratch(f).turned {
        return;
    }
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    let range = f
        .character
        .get::<Kirby>()
        .attributes
        .final_cutter
        .reverse_stick_range;
    if magnitude > range {
        let facing = f.physics.facing;
        if (facing == 1.0 && x < 0.0) || (facing == -1.0 && x > 0.0) {
            f.commands.variables[3] = 1;
            scratch(f).turned = true;
            // ftCommon_UpdateFacing.
            f.physics.facing = if x >= 0.0 { 1.0 } else { -1.0 };
            // 800F2800: fmul in double, frsp.
            let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
            f.core.set_part_rotation(0, Axis::Y, rotation);
            let facing = f.physics.facing;
            for model in FACING_BLADE_MODELS {
                f.effects.push(EffectRequest::OwnedFacing { model, facing });
            }
        }
    }
}

/// The stick's drift for the whole move: PlCo's aerial acceleration times
/// the attribute (800F2940: fmuls), up to the aerial drift speed.
fn drift_limits(f: &Fighter) -> (f32, f32) {
    let air = &f.core.attributes.air;
    let scale = f
        .character
        .get::<Kirby>()
        .attributes
        .final_cutter
        .horizontal_momentum;
    (air.air_drift_stick_mul * scale, air.air_drift_max)
}

/// ftKb_SpecialHi1_Phys (800F2910): ft_80084FA8, whose ground acceleration
/// ftCommon_8007CADC then replaces with the stick's (no friction; a level
/// stick stops Kirby outright), and ftCommon_ApplyGroundMovement again.
fn start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (acceleration, limit) = drift_limits(f);
    common::walk_toward_stick(f, 0.0, acceleration, limit);
    common::move_on_ground(f, &p);
}

/// ftKb_SpecialAirHi1_Phys (800F2A68) / ftKb_SpecialAirHi2_Phys
/// (800F2AE4): ft_80085134's TransN velocity, a rise scaled by the
/// attribute (fmuls), and the stick's drift (ftCommon_8007D3A8).
fn rise_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::root_motion_air(f);
    if f.physics.self_velocity.y > 0.0 {
        let scale = f
            .character
            .get::<Kirby>()
            .attributes
            .final_cutter
            .vertical_momentum;
        f.physics.self_velocity.y *= scale;
    }
    let (acceleration, limit) = drift_limits(f);
    common::drift_without_friction(f, 0.0, acceleration, limit);
    common::finish_air(f, &p);
}

/// ftKb_SpecialAirHi3_Phys (800F2B60): the stick's drift alone; the fall
/// keeps the velocity the rise ended with.
fn fall_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (acceleration, limit) = drift_limits(f);
    common::drift_without_friction(f, 0.0, acceleration, limit);
    common::finish_air(f, &p);
}

/// ftKb_SpecialAirHiEnd_Phys (800F2B9C): ft_80085134.
fn air_landing_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::root_motion_air(f);
    common::finish_air(f, &p);
}

/// ftKb_SpecialHi1_Coll (800F2BBC): off the floor (ft_80082708), the
/// aerial start at the current frame with no vertical speed.
fn start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Final Cutter collision assets");
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR_START, assets, COLLISION_FLAGS, frame, 1.0)?;
    install_damage_callbacks(f);
    f.physics.self_velocity.z = 0.0;
    f.physics.self_velocity.y = 0.0;
    level_root(f);
    Ok(())
}

/// ftKb_SpecialAirHi1_Coll (800F2F70): the ordinary airborne pass
/// (ft_80081D0C). The aerial start has no speed of its own (its TransN
/// does not move), so on a still floor it cannot land; the landing into
/// the grounded start is not ported.
fn air_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        unimplemented!(
            "ftKb_SpecialAirHi1_Coll (ftkirbyspecialhi.c:447-453): the aerial start landing \
             into ftKb_MS_SpecialHi1 at the current frame"
        );
    }
    Ok(())
}

/// The rise's and the fall's landing: ftCommon_8007D7FC, the landing row
/// from its start, the beam's accessory, no speed left, and the root
/// tipped with the floor.
fn land(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.land();
    f.change_motion_state_with_flags(LANDING, assets, COLLISION_FLAGS, 0.0, 1.0)?;
    install_damage_callbacks(f);
    install_beam_accessory(f);
    f.physics.self_velocity = hsd_types::Vec3::ZERO;
    f.physics.ground_velocity = 0.0;
    tilt_with_floor(f);
    Ok(())
}

/// ft_CheckGroundAndLedge toward the facing; without a landing,
/// ftCliffCommon_80081298 and, on a catch, ftCliffCommon_80081370 again.
fn land_or_catch_ledge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> Result<bool> {
    let assets = p.assets.expect("Final Cutter collision assets");
    if common::lands_facing(f, p) {
        return Ok(true);
    }
    if f.try_grab_ledge(assets, p.map)? {
        // ftCliffCommon_80081298 enters CliffCatch itself; the callback
        // then calls ftCliffCommon_80081370 a second time
        // (ftkirbyspecialhi.c:484-485).
        f.enter_cliff_catch(assets, p.map)?;
    }
    Ok(false)
}

/// ftKb_SpecialAirHi2_Coll (800F3004): for the first 21 passes the rise
/// spends every jump (ftCommon_8007D60C) and cannot land (ft_80082578);
/// after them a floor is the landing and a ledge is caught.
fn rise_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if scratch(f).rise_ticks > RISE_LOCK_TICKS {
        if land_or_catch_ledge(f, &mut p)? {
            scratch(f).rise_ticks += 1;
            land(f, p.assets.expect("Final Cutter collision assets"))?;
        }
    } else {
        scratch(f).rise_ticks += 1;
        f.leave_ground_with_spent_jumps();
        common::stay_airborne(f, &mut p);
    }
    Ok(())
}

/// ftKb_SpecialAirHi3_Coll (800F3140).
fn fall_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if land_or_catch_ledge(f, &mut p)? {
        land(f, p.assets.expect("Final Cutter collision assets"))?;
    }
    Ok(())
}

/// ftKb_SpecialHi4_Coll (800F2EA0): on the floor the root tips with it;
/// off it (ft_80082708) the aerial landing continues at the current frame,
/// level, with the beam's accessory again.
fn landing_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        tilt_with_floor(f);
        return Ok(());
    }
    let assets = p.assets.expect("Final Cutter collision assets");
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR_LANDING, assets, COLLISION_FLAGS, frame, 1.0)?;
    install_damage_callbacks(f);
    install_beam_accessory(f);
    f.physics.self_velocity.z = 0.0;
    f.physics.self_velocity.y = 0.0;
    level_root(f);
    Ok(())
}

/// ftKb_SpecialAirHiEnd_Coll (800F3248): in the air the root is level. The
/// row moves on its TransN alone, which does not descend, so on a still
/// floor it cannot land; the landing back into the grounded row is not
/// ported.
fn air_landing_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        unimplemented!(
            "ftKb_SpecialAirHiEnd_Coll (ftkirbyspecialhi.c:526-536): the aerial landing row \
             landing into ftKb_MS_SpecialHi4 at the current frame"
        );
    }
    level_root(f);
    Ok(())
}

/// fn_800F21E8 (800F21E8), the landing rows' accessory4: on the script's
/// cmd_vars[2], once per move, the blade is marked to expire
/// (efLib_SetFlags) and the beam leaves from TopN, the attribute offsets
/// ahead (800F2268..800F2290: fadds, fsubs) and one unit lower
/// (it_8029BAB8).
pub fn beam_accessory(f: &mut Fighter) {
    if scratch(f).beam_spawned || f.commands.variables[2] == 0 {
        return;
    }
    f.commands.variables[2] = 0;
    scratch(f).beam_spawned = true;
    let (ahead, height) = {
        let a = &f.character.get::<Kirby>().attributes.final_cutter;
        (a.projectile_spawn_x, a.projectile_spawn_y)
    };
    let facing = f.physics.facing;
    let mut point = common::joint_position(f, 0);
    point.x += if facing == 1.0 { ahead } else { -ahead };
    point.y += height;
    point.z = 0.0;
    f.effects.push(EffectRequest::ExpireOwned);
    point.y -= BEAM_DROP;
    // it_8029BAB8: Item_InitRaySpawnPosition from Kirby's ECB centre
    // (it_8026BB68 -> ftLib_80086990, retail 800869AC..BC: fadds, fmuls,
    // fadds); it_8029BB90 reads his root's X rotation.
    let angle = f.core.part_rotation_x(0);
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(ItemKind::KirbyCBeam, c.player.id, point, facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = hsd_types::Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_argument = angle.to_bits() as i32;
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// ftKb_Init_800EE7B8 / ftKb_Init_800EE74C, the take-damage and death
/// callbacks the collision changes install. Without a hat, a hammer, a
/// mouthful or Stone's armour, what is left is
/// ftKb_AttackDashAir_800F22D4 and ftKb_SpecialAirLw_800F5318: the held
/// item and the articles show again (ftCommon_8007F5CC, x221E_b4), the
/// root is level, and Kirby's effects go (efLib_DestroyAll).
pub fn damage_callback(f: &mut Fighter) {
    if !std::mem::take(&mut f.character.get_mut::<Kirby>().damage_callbacks) {
        return;
    }
    level_root(f);
    assert!(
        f.held_item.is_none(),
        "ftCommon_8007F5CC: Final Cutter holding an item"
    );
    f.commands.held_item_hidden = false;
    f.commands.articles_visible = true;
    f.effects.push(EffectRequest::DestroyOwned);
}
