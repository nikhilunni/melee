//! Nayru's Love, ftzeldaspecialn.c (8013A830..8013ADB4).
//!
//! The script's cmd_vars[0] raises the reflector (1 -> 2,
//! ftColl_CreateReflectHit with ZeldaAttributes.nayrus_love_reflection and
//! the empty ftZd_SpecialN_8013ADB0 hit callback) and lowers it (0). The
//! crystal is a model at TransN (efSync 0x4F4 on the ground, 0x4F5 in the
//! air). In the air Zelda hangs for the attribute's frames, then falls
//! under her own gravity.
use crate::{
    common,
    init::{Accessory, Zelda},
};
use melee_coll::defense::ReflectDescriptor;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::friction,
};

/// ftZd_MS_SpecialN (341) and ftZd_MS_SpecialAirN (342).
pub const GROUND: ActionId = ActionId(341);
pub const AIR: ActionId = ActionId(342);
/// ftZd_SM_SpecialN / SpecialAirN.
pub const ANIMATIONS: [i32; 2] = [295, 296];

/// ftZd_MF_SpecialN_Coll: ftCommon_GroundAirColl_MF with KeepGfx,
/// KeepColAnimHitStatus and SkipHit.
const GROUND_AIR_FLAGS: MotionEntryFlags = common::GROUND_AIR_COLLISION_FLAGS;
/// efSync_Spawn(1268 / 1269, gobj, fp->parts[FtPart_TransN].joint): the
/// crystal (efLib_Create_AttachChild 0x4268 / 0x4269), at joint 1.
const GROUND_CRYSTAL: u16 = 0x4F4;
const AIR_CRYSTAL: u16 = 0x4F5;
const TRANS_JOINT: usize = 1;

/// cmd_vars[0]: the script asks for the reflector (1), it is up (2), it
/// went down (0).
const RAISE: u32 = 1;
const RAISED: u32 = 2;
const LOWERED: u32 = 0;

/// Nayru's Love's live state: mv.zd.specialn and the reflector.
#[derive(Clone, Copy, Debug, Default)]
pub struct NayrusLove {
    /// mv.zd.specialn.x0: aerial frames before gravity applies.
    pub float_frames: i32,
    /// ftColl_CreateReflectHit's descriptor while the reflector is up.
    pub reflector: Option<ReflectDescriptor>,
}

fn attributes(f: &Fighter) -> &crate::attributes::NayrusLoveAttributes {
    &f.character.get::<Zelda>().attributes.nayrus_love
}
fn scratch(f: &mut Fighter) -> &mut NayrusLove {
    &mut f.character.get_mut::<Zelda>().nayrus_love
}
fn install_accessory(f: &mut Fighter, air: bool) {
    f.character.get_mut::<Zelda>().accessory = if air {
        Accessory::NayrusLoveAirCrystal
    } else {
        Accessory::NayrusLoveCrystal
    };
    f.core.arm_accessory4();
}

/// ftZd_SpecialN_Enter (8013A93C) / ftZd_SpecialAirN_Enter (8013A9B4):
/// in the air the fall stops and the drift is divided down (fdivs).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        f.physics.self_velocity.y = 0.0;
        f.physics.self_velocity.x /= attributes(f).air_horizontal_velocity_divisor;
    }
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Nayru's Love assets");
    f.step_animation(a);
    f.commands.variables[0] = LOWERED;
    let frames = attributes(f).air_float_frames;
    scratch(f).float_frames = frames;
    install_accessory(f, air);
}

/// ftZd_SpecialN_8013A830 / 8013A8AC: the crystal once per motion
/// (x2219_b0), Fighter_SetEffectHitlagCallbacks; accessory4 uninstalls.
pub fn crystal(f: &mut Fighter, air: bool) {
    if !f.effect_state.destroy_on_state_change {
        let id = if air { AIR_CRYSTAL } else { GROUND_CRYSTAL };
        f.effects.push(EffectRequest::SyncAttached {
            id,
            bone: TRANS_JOINT,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    f.character.get_mut::<Zelda>().accessory = Accessory::None;
    f.core.accessory4_armed = false;
}

/// ftColl_CreateReflectHit (8007B240), literal descriptor transfer.
fn descriptor(a: &melee_ft::desc::fox_attributes::ReflectionAttributes) -> ReflectDescriptor {
    ReflectDescriptor {
        bone: a.joint as usize,
        maximum_damage: a.max_damage,
        offset: a.offset,
        radius: a.size,
        damage_multiplier: a.damage_multiplier,
        speed_multiplier: a.speed_multiplier,
        exclude_master_ball_ownership: a.skip_ownership_change != 0,
    }
}

/// ftColl_CreateReflectHit with the Nayru's Love reflector.
fn raise_reflector(f: &mut Fighter) {
    let descriptor = descriptor(&f.character.get::<Zelda>().attributes.nayrus_love_reflection);
    scratch(f).reflector = Some(descriptor);
    f.combat.reflector_enabled = true;
}

/// ftZd_SpecialN_Anim (8013AA2C) / ftZd_SpecialAirN_Anim (8013AACC): the
/// script's request raises the reflector, its lowering drops it
/// (fp->reflecting = false); Wait or Fall at the end.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] == RAISE {
        f.commands.variables[0] = RAISED;
        raise_reflector(f);
    }
    if f.commands.variables[0] == LOWERED {
        f.combat.reflector_enabled = false;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == AIR;
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ftColl_8007AEF8: the reflector's position updates this frame.
fn refresh_reflector(f: &mut Fighter) {
    f.shield.reflect.volume.position_cached = false;
}

/// ftZd_SpecialN_Phys (8013AB68): ft_80084F3C, then ftColl_8007AEF8.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
    refresh_reflector(f);
}

/// ftZd_SpecialAirN_Phys (8013AB9C): the hang counts down, then the
/// attribute's gravity with the ordinary terminal speed; ftCommon_8007CF58's
/// drift friction; ftColl_8007AEF8.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let hang = scratch(f).float_frames;
    if hang != 0 {
        scratch(f).float_frames = hang - 1;
    } else {
        let gravity = attributes(f).air_gravity;
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
    refresh_reflector(f);
}

/// The counterpart change keeps a raised reflector
/// (ftZd_SpecialN_8013AC88 / 8013AD1C re-create it).
fn after_ground_air_change(f: &mut Fighter, air: bool) {
    if f.commands.variables[0] == RAISED {
        raise_reflector(f);
    }
    install_accessory(f, air);
}

/// ftZd_SpecialN_Coll: off the floor, ftZd_SpecialN_8013AC88 continues in
/// the air.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Nayru's Love collision assets");
    common::ground_to_air(f, AIR, GROUND_AIR_FLAGS, assets)?;
    after_ground_air_change(f, true);
    Ok(())
}

/// ftZd_SpecialAirN_Coll: landing continues on the ground
/// (ftZd_SpecialN_8013AD1C).
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Nayru's Love landing assets");
    common::air_to_ground(f, GROUND, GROUND_AIR_FLAGS, assets)?;
    after_ground_air_change(f, false);
    Ok(())
}

/// The live reflector's contact (ftColl_80077464's reflect volume).
pub fn reflector_contact(
    f: &mut Fighter,
    hit: &melee_coll::hitbox::HitCapsule,
    scale: f32,
) -> Option<ReflectDescriptor> {
    if !f.combat.reflector_enabled {
        return None;
    }
    let descriptor = scratch(f).reflector?;
    f.core
        .reflector_contact(hit, scale, &descriptor)
        .then_some(descriptor)
}

/// ftZd_SpecialN_8013ADB0: the reflector's hit callback is empty.
pub fn reflect_hit(_: &mut Fighter, _: f32, _: &FighterAssets) -> Result<()> {
    Ok(())
}
