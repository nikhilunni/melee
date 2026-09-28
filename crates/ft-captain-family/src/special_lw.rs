//! Falcon Kick, ftcaptainspeciallw.c (800E3EAC..800E47B8).
//!
//! The grounded kick (357) slides on root motion scaled by a friction factor
//! that each damaging hit shrinks; its end (358) brakes to Wait. Leaving the
//! ground turns it into the airborne end (362). The aerial kick (359) dives
//! on root motion and lands in 360, or finishes in the air in 361. A
//! grounded kick that runs into a wall rebounds (363).
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter,
    },
    physics::{airborne, friction::friction_acceleration, grounded},
};
use melee_types::{
    mp::collide::{LEFT_WALL_HUG, RIGHT_WALL_HUG},
    CommonMotionState, FtPart, GroundOrAir,
};

use crate::CaptainFamily;

/// ftCa_MS_SpecialLw (357) .. ftCa_MS_SpecialHiThrow1 (363).
pub const GROUND: ActionId = ActionId(357);
pub const GROUND_END: ActionId = ActionId(358);
pub const AIR: ActionId = ActionId(359);
pub const AIR_LANDING: ActionId = ActionId(360);
pub const AIR_END: ActionId = ActionId(361);
pub const GROUND_END_AIR: ActionId = ActionId(362);
pub const REBOUND: ActionId = ActionId(363);

/// `MTXDegToRad(1)` as MWCC rounds it (ftCa_SpecialHi_804D9224).
const DEGREES_TO_RADIANS: f32 = 0.017453292;

/// mv.ca.speciallw (fp+2340): the grounded kick's hit slowdown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FalconKick {
    /// +2340 (u16): damaging hits counted toward the slowdown limit.
    pub hits: u16,
    /// +2344: multiplies the kick's velocity each physics tick.
    pub friction: f32,
}
impl Default for FalconKick {
    fn default() -> Self {
        Self {
            hits: 0,
            friction: 1.0,
        }
    }
}

fn kick<C: CaptainFamily>(f: &mut Fighter) -> &mut FalconKick {
    &mut crate::family::<C>(f).specials().kick
}

fn attributes<C: CaptainFamily>(f: &Fighter) -> &crate::attributes::FalconKickAttributes {
    &crate::attributes::<C>(f).falcon_kick
}

/// cmd_vars[0..3] and the throw flags, cleared before every kick change.
fn reset_script_flags(f: &mut Fighter) {
    f.commands.variables[..3].fill(0);
    f.commands.clear_throw_flags();
}

/// ftCa_SpecialLw_Enter (800E4038) / ftCa_SpecialAirLw_Enter (800E40D4).
pub fn enter<C: CaptainFamily>(f: &mut Fighter, airborne: bool, a: &FighterAssets) {
    reset_script_flags(f);
    if !airborne {
        *kick::<C>(f) = FalconKick::default();
    }
    f.change_motion_state(if airborne { AIR } else { GROUND }, a)
        .expect("Falcon Kick assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    // The grounded kick's deal_dmg_cb (ftCa_SpecialHi_800E400C) is
    // `deal_damage`, keyed on the motion; then Fighter_SetEffectHitlagCallbacks.
    f.effect_state.hitlag_callbacks = true;
}

/// ftCa_SpecialHi_800E400C, the grounded kick's deal_dmg_cb: each damaging
/// hit up to the limit slows the kick further.
pub fn deal_damage<C: CaptainFamily>(
    f: &mut Fighter,
    _: &melee_ft::fighter::assets::FighterAssets,
) {
    if f.motion_state.action != GROUND {
        return;
    }
    let (limit, multiplier) = {
        let a = attributes::<C>(f);
        (a.hit_slowdown_counter_limit, a.on_hit_speed_multiplier)
    };
    let scratch = kick::<C>(f);
    if i32::from(scratch.hits) <= limit {
        scratch.hits += 1;
        scratch.friction *= multiplier;
    }
}

/// ftCa_SpecialLw_Anim (800E4174): at the end, the grounded ending (at the
/// attribute rate) or, off the ground, the airborne one.
pub fn ground_anim<C: CaptainFamily>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        reset_script_flags(f);
        if f.physics.ground_or_air == GroundOrAir::Ground {
            let rate = attributes::<C>(f).ground_ending_animation_rate;
            // ftCommon_8007D7FC on the grounded fighter.
            f.land();
            f.change_motion_state_with_rate(GROUND_END, p.assets, 0.0, rate)?;
        } else {
            f.leave_ground();
            f.change_motion_state(GROUND_END_AIR, p.assets)?;
        }
        f.effect_state.hitlag_callbacks = true;
    }
    Ok(None)
}

/// ftCa_SpecialLwEnd_Anim / ftCa_SpecialLwEndAir_Anim: ftCommon_8007D92C.
pub fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        wait_or_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftCommon_8007D92C (8007D92C): Fall in the air, Wait on the ground.
fn wait_or_fall(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let state = if f.physics.ground_or_air == GroundOrAir::Air {
        CommonMotionState::Fall
    } else {
        CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), a)
}

/// ftCa_SpecialAirLw_Anim (800E42D8): finishing in the air continues as 361.
pub fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        reset_script_flags(f);
        f.leave_ground();
        f.change_motion_state(AIR_END, p.assets)?;
    }
    Ok(None)
}

/// ftCa_SpecialAirLwEnd_Anim (800E4340): Wait (ft_8008A2BC).
pub fn landing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// ftCa_SpecialAirLwEndAir_Anim / ftCa_SpecialHiThrow1_Anim: Fall.
pub fn fall_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
    }
    Ok(None)
}

/// The TransN offset this tick's animation extracted.
fn root_offset(f: &Fighter) -> hsd_types::Vec3 {
    f.animation
        .root_motion
        .as_ref()
        .expect("Falcon Kick TransN")
        .primary_history
        .offset
}

fn has_root_motion(f: &Fighter) -> bool {
    f.animation
        .flags
        .contains(melee_ft::anim::MotionFlags::ROOT_MOTION)
}

/// ftCommon_8007E5AC (8007E5AC): TopN leans with the floor; the airborne
/// branches reset the lean (ftPartSetRotZ(fp, 0, 0)).
fn lean_with_floor(f: &mut Fighter) {
    let normal = f.collision.data.floor.normal;
    let angle = -melee_lb::trigf::atan2f(normal.x, normal.y);
    let root = f.animation.root;
    f.skeleton.set_rotation_z(root, angle);
}

fn stand_upright(f: &mut Fighter) {
    let root = f.animation.root;
    f.skeleton.set_rotation_z(root, 0.0);
}

/// ft_800850E0 (800850E0) with the ground friction and facing: root motion
/// sets the ground speed, otherwise friction; then ApplyGroundMovement.
fn root_motion_ground(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    if has_root_motion(f) {
        // 80085108: fmuls.
        f.physics.ground_velocity = root_offset(f).z * f.physics.facing;
    } else {
        let friction = f.attributes.ground.ground_friction;
        f.physics.ground_acceleration = friction_acceleration(f.physics.ground_velocity, friction);
    }
    apply_ground_movement(f, p);
}

fn apply_ground_movement(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let normal = f.collision.data.floor.normal;
    let terrain = p.map.floor_speed_scale(&f.collision.data);
    grounded::apply_ground_movement(&mut f.core.physics, normal, terrain);
}

/// ft_80085134 (80085134): airborne velocity straight from TransN.
fn root_motion_air(f: &mut Fighter) {
    let offset = root_offset(f);
    f.physics.self_velocity.x = offset.z * f.physics.facing;
    f.physics.self_velocity.y = offset.y;
}

/// ft_80084EEC (80084EEC): gravity and air friction, no stick input.
fn fall_without_drift(f: &mut Fighter) {
    let air = &f.core.attributes.air;
    let physics = &mut f.core.physics;
    physics.self_velocity.y =
        airborne::gravity(physics.self_velocity.y, air.gravity, air.terminal_velocity);
    physics.animation_velocity.x =
        airborne::drift_acceleration(physics.self_velocity.x, 0.0, 0.0, air);
}

/// ftCa_Special_Inline_Friction: separate fmuls, x then y.
fn apply_kick_friction<C: CaptainFamily>(f: &mut Fighter) {
    let friction = kick::<C>(f).friction;
    f.physics.self_velocity.x *= friction;
    f.physics.self_velocity.y *= friction;
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

/// ftCa_SpecialHi_800E3EAC (800E3EAC): the script's cue lights the flame on
/// the kicking foot the first time and removes owned effects the second.
fn flame<C: CaptainFamily>(f: &mut Fighter, a: &FighterAssets) {
    if !f.commands.take_move_cue() {
        return;
    }
    if f.effect_state.destroy_on_state_change {
        // ftCommon_8007DB24.
        f.effect_state.destroy_on_state_change = false;
        f.effects.push(EffectRequest::DestroyOwned);
        return;
    }
    let (part, angle) = match f.motion_state.action {
        GROUND => (FtPart::RFootJA, 0.0),
        // 800E3F14: fmuls.
        AIR => (
            FtPart::LFootJA,
            DEGREES_TO_RADIANS * attributes::<C>(f).flame_angle_degrees,
        ),
        other => unreachable!("Falcon Kick flame in {other:?}"),
    };
    let bone = usize::from(a.parts.joint(part).expect("Falcon Kick foot"));
    f.effects.push(EffectRequest::AttachedParameter {
        id: C::EFFECTS.kick_flame,
        bone,
        parameter: angle,
    });
    f.effect_state.destroy_on_state_change = true;
}

/// ftCa_SpecialLw_Phys (800E43C0).
pub fn ground_physics<C: CaptainFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        lean_with_floor(f);
        root_motion_ground(f, &p);
    } else {
        stand_upright(f);
        root_motion_air(f);
    }
    apply_kick_friction::<C>(f);
    flame::<C>(f, p.assets);
    finish_update(f, &p);
}

/// ftCa_SpecialLwEnd_Phys (800E4460): after the script's cmd_vars[2] the
/// ending brakes on its own traction, before it on ft_80084F3C.
pub fn ground_end_physics<C: CaptainFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        lean_with_floor(f);
        if f.commands.variables[2] != 0 {
            // 800E44A8: fmuls.
            let friction =
                attributes::<C>(f).ground_traction_multiplier * f.attributes.ground.ground_friction;
            f.physics.ground_acceleration =
                friction_acceleration(f.physics.ground_velocity, friction);
            apply_ground_movement(f, &p);
        } else {
            wait_friction(f, &p);
        }
    } else {
        stand_upright(f);
        fall_without_drift(f);
    }
    apply_kick_friction::<C>(f);
    finish_update(f, &p);
}

/// ft_80084F3C (80084F3C): Wait's friction and ground movement.
fn wait_friction(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    let params =
        grounded::GroundedParameters::from_attributes(&f.core.attributes, &p.assets.common);
    let normal = f.collision.data.floor.normal;
    let terrain = p.map.floor_speed_scale(&f.collision.data);
    grounded::friction_physics(&mut f.core.physics, &params, normal, terrain);
}

/// ftCa_SpecialLwEndAir_Phys (800E4530).
pub fn ground_end_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        lean_with_floor(f);
        root_motion_ground(f, &p);
    } else {
        stand_upright(f);
        if f.commands.variables[0] != 0 {
            fall_without_drift(f);
        } else {
            root_motion_air(f);
        }
    }
    finish_update(f, &p);
}

/// ftCa_SpecialAirLw_Phys (800E45B8).
pub fn air_physics<C: CaptainFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    root_motion_air(f);
    flame::<C>(f, p.assets);
    finish_update(f, &p);
}

/// ftCa_SpecialAirLwEnd_Phys (800E45F4): the landing brakes on its own
/// traction after the script's cmd_vars[2], on ft_80084F3C before it.
pub fn landing_physics<C: CaptainFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[2] != 0 {
        // 800E4624: fmuls.
        let friction = attributes::<C>(f).air_landing_traction_multiplier
            * f.attributes.ground.ground_friction;
        f.physics.ground_acceleration = friction_acceleration(f.physics.ground_velocity, friction);
        apply_ground_movement(f, &p);
    } else {
        wait_friction(f, &p);
    }
    finish_update(f, &p);
}

/// ftCa_SpecialAirLwEndAir_Phys (800E4674): ft_80084EEC.
pub fn air_end_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    fall_without_drift(f);
    finish_update(f, &p);
}

/// ftCa_SpecialHiThrow1_Phys (800E4698): ft_80085134.
pub fn rebound_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    root_motion_air(f);
    finish_update(f, &p);
}

/// ft_80082708 (80082708): ordinary ground collision; false off the floor.
fn ground_supported(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ft_800827A0 (800827A0): ground collision stopping at the floor's edge.
fn ground_supported_at_edge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    ground::map_escape(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ft_80081D0C (80081D0C): ordinary airborne collision; true on landing.
fn air_landed(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftCa_SpecialLw_Coll (800E46B8): change ground state without changing
/// motion, then rebound off a wall the kick faces once the script allows.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        if !ground_supported(f, &mut p) {
            f.leave_ground();
        }
    } else if air_landed(f, &mut p) {
        f.land();
    }
    let env = f.collision.data.env_flags as u32;
    let facing_wall = (f.physics.facing == -1.0 && env & RIGHT_WALL_HUG != 0)
        || (f.physics.facing == 1.0 && env & LEFT_WALL_HUG != 0);
    if f.commands.variables[0] != 0 && facing_wall {
        reset_script_flags(f);
        f.leave_ground();
        f.change_motion_state(REBOUND, p.assets.expect("Falcon Kick rebound assets"))?;
    }
    Ok(())
}

/// ftCa_SpecialLwEnd_Coll / ftCa_SpecialLwEndAir_Coll (800E47D8): after the
/// script's cmd_vars[1] the ground ending stops at the floor's edge.
pub fn end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        let supported = if f.commands.variables[1] != 0 {
            ground_supported_at_edge(f, &mut p)
        } else {
            ground_supported(f, &mut p)
        };
        if !supported {
            f.leave_ground();
        }
    } else if air_landed(f, &mut p) {
        f.land();
    }
    Ok(())
}

/// doColl (ftCa_SpecialAirLw_Coll / ftCa_SpecialAirLwEndAir_Coll): landing
/// enters 360 at the landing rate.
pub fn air_collision<C: CaptainFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if air_landed(f, &mut p) {
        reset_script_flags(f);
        let rate = attributes::<C>(f).landing_animation_rate;
        f.land();
        f.change_motion_state_with_rate(
            AIR_LANDING,
            p.assets.expect("Falcon Kick landing assets"),
            0.0,
            rate,
        )?;
    }
    Ok(())
}

/// ftCa_SpecialAirLwEnd_Coll: ft_80084104, Fall off the floor's edge.
pub fn landing_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !ground_supported_at_edge(f, &mut p) {
        f.change_motion_state(
            CommonMotionState::Fall.into(),
            p.assets.expect("Falcon Kick fall assets"),
        )?;
    }
    Ok(())
}
