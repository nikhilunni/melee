//! Raptor Boost, ftcaptainspecials.c (800E3278..800E3EAC).
//!
//! The startup (349 grounded, 351 aerial) carries an inert detection
//! hitbox; once the script opens cmd_vars[0], touching a fighter switches
//! to the lunge (350 / 352) through the hurtbox-detect callback. Leaving the
//! ground ends in FallSpecial, an aerial landing in LandingFallSpecial.
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::{
    mp::collide::{LEFT_WALL_MASK, RIGHT_WALL_MASK},
    CommonMotionState, FtPart,
};

use crate::init::CaptainFalcon;

/// ftCa_MS_SpecialSStart (349) .. ftCa_MS_SpecialAirS (352).
pub const GROUND_START: ActionId = ActionId(349);
pub const GROUND: ActionId = ActionId(350);
pub const AIR_START: ActionId = ActionId(351);
pub const AIR: ActionId = ActionId(352);

/// efSync_Spawn(1169, gobj, HeadN): the startup's attached model (efAlt
/// 0x491, model 0xFA4).
const START_EFFECT: u16 = 1169;
/// efSync_Spawn(1170 / 1171, gobj, TransN, &facing_dir): the lunge's scaled
/// models (efAlt 0x492 / 0x493, 0xFA3 / 0xFA5) turned to the facing.
const GROUND_LUNGE_EFFECT: u16 = 1170;
const AIR_LUNGE_EFFECT: u16 = 1171;

/// transition_flags (ftcaptainspecials.c:121-124): KeepGfx | SkipMatAnim |
/// UpdateCmd | SkipColAnim | SkipItemVis | Unk19 | SkipModelPartVis |
/// SkipModelFlags | Unk27, from frame zero.
const LUNGE_FLAGS: MotionEntryFlags = MotionEntryFlags(
    1 << 1 | 1 << 7 | 1 << 12 | 1 << 14 | 1 << 18 | 1 << 19 | 1 << 22 | 1 << 26 | 1 << 27,
);

/// mv.ca.specials (fp+2340) and the word this move leaves at +2344.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RaptorBoost {
    /// +2340: the aerial boost's own vertical velocity.
    pub vertical_velocity: f32,
    /// mv+4, never written here: the word the previous state left, which
    /// LandingFallSpecial inherits (`None` when the port does not model it).
    pub inherited_word: Option<f32>,
}

fn is_raptor_boost(action: ActionId) -> bool {
    (GROUND_START.0..=AIR.0).contains(&action.0)
}

fn attributes(f: &Fighter) -> &crate::attributes::RaptorBoostAttributes {
    &f.character.get::<CaptainFalcon>().attributes.raptor_boost
}

fn captain(f: &mut Fighter) -> &mut CaptainFalcon {
    f.character.get_mut::<CaptainFalcon>()
}

/// The retained second scratch word while a Raptor Boost state is current.
pub fn retained_scratch_word(falcon: &CaptainFalcon, action: ActionId) -> Option<f32> {
    if !is_raptor_boost(action) {
        return None;
    }
    Some(falcon.raptor_boost.inherited_word.unwrap_or_else(|| {
        unimplemented!("ftCa_SpecialS: mv+4 inherited from an unmodelled scratch word")
    }))
}

/// ftCa_SpecialS_Enter (800E3530) / ftCa_SpecialAirS_Enter (800E3688).
pub fn enter(f: &mut Fighter, airborne: bool, a: &FighterAssets) {
    let inherited_word = f.inherited_scratch_word();
    f.commands.variables[..4].fill(0);
    if !airborne {
        // resetCmdVarsGround: ftCommon_8007D7FC.
        f.land();
    }
    f.change_motion_state(if airborne { AIR_START } else { GROUND_START }, a)
        .expect("Raptor Boost assets");
    // setCallbacks: take_dmg_cb and death2_cb are `remove_effects`, keyed
    // on the motion; ftAnim_8006EBA4.
    f.step_animation(a);
    let head = usize::from(a.parts.joint(FtPart::HeadN).expect("HeadN"));
    f.effects.push(EffectRequest::SyncAttached {
        id: START_EFFECT,
        bone: head,
    });
    let falcon = captain(f);
    falcon.raptor_boost_start_effect_active = true;
    falcon.raptor_boost_lunge_effect_active = false;
    falcon.raptor_boost.inherited_word = inherited_word;
    // Fighter_SetEffectHitlagCallbacks; hurtbox_detect_cb is `detect`.
    f.effect_state.hitlag_callbacks = true;
    f.physics.self_velocity = hsd_types::Vec3::ZERO;
    if airborne {
        captain(f).raptor_boost.vertical_velocity = 0.0;
        f.leave_ground_with_spent_jumps();
    } else {
        f.physics.ground_velocity = 0.0;
    }
}

/// ftCa_SpecialS_RemoveGFX (800E3278), the take_dmg_cb and death2_cb of
/// every Raptor Boost state (ftCa_Init_800E28C8).
pub fn remove_effects(f: &mut Fighter) {
    if !is_raptor_boost(f.motion_state.action) {
        return;
    }
    f.effects.push(EffectRequest::DestroyOwned);
    let falcon = captain(f);
    falcon.raptor_boost_lunge_effect_active = false;
    falcon.raptor_boost_start_effect_active = false;
}

/// ftCa_SpecialS_OnDetect (800E3780): with the script's window open, an
/// inert touch on a fighter starts the lunge from frame zero.
pub fn detect(f: &mut Fighter, a: &FighterAssets, _target: u32) {
    if f.commands.variables[0] == 0 {
        return;
    }
    let result = match f.motion_state.action {
        GROUND_START => detect_on_ground(f, a),
        AIR_START => detect_in_air(f, a),
        _ => return,
    };
    result.expect("Raptor Boost lunge assets");
}

/// onDetectGround: land, enter 350, keep only horizontal speed, scaled.
fn detect_on_ground(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let multiplier = attributes(f).ground_hit_speed_multiplier;
    f.land();
    f.change_motion_state_with_flags(GROUND, a, LUNGE_FLAGS, 0.0, 1.0)?;
    f.physics.self_velocity.y = 0.0;
    f.physics.self_velocity.z = 0.0;
    // 800E36F0: fmuls.
    f.physics.ground_velocity *= multiplier;
    Ok(())
}

/// onDetectAir: enter 352.
fn detect_in_air(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    f.change_motion_state_with_flags(AIR, a, LUNGE_FLAGS, 0.0, 1.0)?;
    f.physics.self_velocity.z = 0.0;
    Ok(())
}

/// ftCa_SpecialSStart_Anim (800E3900): Wait at the end (ft_8008A2BC).
pub fn ground_start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// The lunge's model, spawned on its first animation tick.
fn lunge_effect(f: &mut Fighter, a: &FighterAssets, id: u16) {
    if captain(f).raptor_boost_lunge_effect_active {
        return;
    }
    let trans = usize::from(a.parts.joint(FtPart::TransN).expect("TransN"));
    f.effects
        .push(EffectRequest::SyncAttached { id, bone: trans });
    captain(f).raptor_boost_lunge_effect_active = true;
    f.effect_state.hitlag_callbacks = true;
}

/// ftCa_SpecialS_Anim (800E393C).
pub fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    lunge_effect(f, p.assets, GROUND_LUNGE_EFFECT);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// Leaving the boost in the air: FallSpecial at the given landing lag, or
/// an ordinary Fall without one. `clamp` is ftCommon_ClampAirDrift, which
/// only the grounded collisions run before FallSpecial.
fn fall(f: &mut Fighter, a: &FighterAssets, lag: f32, clamp: bool) -> Result<()> {
    f.leave_ground_with_spent_jumps();
    if lag == 0.0 {
        return f.change_motion_state(CommonMotionState::Fall.into(), a);
    }
    if clamp {
        let maximum = f.attributes.air.air_drift_max;
        f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-maximum, maximum);
    }
    f.enter_special_fall(a, true, true, false, 1.0, lag)
}

/// ftCa_SpecialAirSStart_Anim (800E3A2C).
pub fn air_start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let lag = attributes(f).miss_landing_lag;
        fall(f, p.assets, lag, false)?;
    }
    Ok(None)
}

/// ftCa_SpecialAirS_Anim (800E3AA4).
pub fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    lunge_effect(f, p.assets, AIR_LUNGE_EFFECT);
    if !f.animation.frames_remaining(&f.skeleton) {
        let lag = attributes(f).hit_landing_lag;
        fall(f, p.assets, lag, false)?;
    }
    Ok(None)
}

/// ft_80085134 (80085134): airborne velocity straight from TransN.
fn root_motion_air(f: &mut Fighter) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("Raptor Boost TransN")
        .primary_history
        .offset;
    f.physics.self_velocity.x = offset.z * f.physics.facing;
    f.physics.self_velocity.y = offset.y;
}

/// The aerial boost's own gravity: separate fsubs, clamped to the terminal
/// velocity, replacing the vertical velocity.
fn boost_gravity(f: &mut Fighter) {
    let (gravity, terminal) = {
        let a = attributes(f);
        (a.gravity, a.terminal_velocity)
    };
    let boost = &mut captain(f).raptor_boost;
    boost.vertical_velocity -= gravity;
    if boost.vertical_velocity < -terminal {
        boost.vertical_velocity = -terminal;
    }
    let vertical = boost.vertical_velocity;
    f.physics.self_velocity.y = vertical;
}

/// ftCa_SpecialAirSStart_Phys (800E3B6C): gravity once the script sets
/// cmd_vars[1].
pub fn air_start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    root_motion_air(f);
    if f.commands.variables[1] == 1 {
        boost_gravity(f);
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftCa_SpecialAirS_Phys (800E3BD8).
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    root_motion_air(f);
    boost_gravity(f);
    f.core.finish_air_update(p.assets, p.wind);
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

/// ftCa_SpecialSStart_Coll (800E3C3C): before the script's cmd_vars[2] the
/// startup stops at the floor's edge (ft_80084104); after it, leaving the
/// floor falls, and once cmd_vars[0] is set a wall ahead ends the boost.
pub fn ground_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let a = p.assets.expect("Raptor Boost collision assets");
    if f.commands.variables[2] == 0 {
        let c = &mut f.core;
        if ground::map_escape(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ) != ground::WaitGroundResult::Supported
        {
            f.change_motion_state(CommonMotionState::Fall.into(), a)?;
        }
        return Ok(());
    }
    if !ground_supported(f, &mut p) {
        f.effects.push(EffectRequest::DestroyOwned);
        let lag = attributes(f).miss_landing_lag;
        return fall(f, a, lag, true);
    }
    if f.commands.variables[0] == 1 {
        let env = f.collision.data.env_flags as u32;
        let facing = f.physics.facing;
        if (facing == 1.0 && env & LEFT_WALL_MASK != 0)
            || (facing == -1.0 && env & RIGHT_WALL_MASK != 0)
        {
            f.effects.push(EffectRequest::DestroyOwned);
            f.change_motion_state(CommonMotionState::Wait.into(), a)?;
        }
    }
    Ok(())
}

/// ftCa_SpecialS_Coll (800E3D6C): leaving the floor mid-lunge falls.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !ground_supported(f, &mut p) {
        let a = p.assets.expect("Raptor Boost collision assets");
        f.effects.push(EffectRequest::DestroyOwned);
        let lag = attributes(f).hit_landing_lag;
        return fall(f, a, lag, true);
    }
    Ok(())
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

/// ftCa_SpecialAirSStart_Coll (800E3DF4).
pub fn air_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if air_landed(f, &mut p) {
        f.effects.push(EffectRequest::DestroyOwned);
        let lag = attributes(f).miss_landing_lag;
        f.enter_special_landing(p.assets.expect("Raptor Boost landing assets"), false, lag)?;
    }
    Ok(())
}

/// ftCa_SpecialAirS_Coll (800E3E50): the lunge keeps its speed on landing.
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if air_landed(f, &mut p) {
        f.physics.ground_velocity = f.physics.self_velocity.x;
        f.effects.push(EffectRequest::DestroyOwned);
        let lag = attributes(f).hit_landing_lag;
        f.enter_special_landing(p.assets.expect("Raptor Boost landing assets"), false, lag)?;
    }
    Ok(())
}

/// ftCa_SpecialSStart_Phys / ftCa_SpecialS_Phys: ft_80084FA8.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::jab(f, p);
}
