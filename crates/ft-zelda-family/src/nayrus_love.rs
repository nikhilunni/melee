//! Nayru's Love, ftzeldaspecialn.c (8013A830..8013ADB4), and Kirby's copy
//! of it (ftkirbyspecialzelda.c, 80105A34..80105FEC): the same states over
//! either user's rows, generic over [`NayrusLoveUser`].
//!
//! The script's cmd_vars[0] raises the reflector (1 -> 2,
//! ftColl_CreateReflectHit with the user's ReflectDesc and an empty hit
//! callback, ftZd_SpecialN_8013ADB0 / fn_80105FEC) and lowers it (0). The
//! crystal is a model at TransN (Zelda's efSync 0x4F4 on the ground, 0x4F5
//! in the air; Kirby's efAlt 0x4B6 / 0x4B7). In the air the user hangs for
//! the attribute's frames, then falls under the attribute's gravity.
use crate::common;
use hsd_archive::Reader;
use melee_coll::defense::ReflectDescriptor;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    desc::{fox_attributes::ReflectionAttributes, FighterDescError},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::friction,
};

/// ftZd_MF_SpecialN_Coll: ftCommon_GroundAirColl_MF with KeepGfx,
/// KeepColAnimHitStatus and SkipHit (Kirby's ground/air changes pass the
/// same constant).
const GROUND_AIR_FLAGS: MotionEntryFlags = common::GROUND_AIR_COLLISION_FLAGS;
/// efSync_Spawn(id, gobj, fp->parts[FtPart_TransN].joint): the crystal at
/// joint 1 (Kirby's spawners name `parts[1]` directly).
const TRANS_JOINT: usize = 1;

/// cmd_vars[0]: the script asks for the reflector (1), it is up (2), it
/// went down (0).
const RAISE: u32 = 1;
const RAISED: u32 = 2;
const LOWERED: u32 = 0;

/// Nayru's Love's attributes: ftZelda_DatAttrs +0..+C and ftKb_DatAttrs
/// +2A4..+2B0 (`specialn_zd_*`), the same four words in the same order.
#[derive(Clone, Debug, PartialEq)]
pub struct NayrusLoveAttributes {
    /// +0x00.
    // TODO(meaning): no consumer in the pinned C.
    pub unused: f32,
    /// +0x04: aerial frames before gravity applies.
    pub air_float_frames: i32,
    /// +0x08: aerial horizontal velocity divisor on entry.
    pub air_horizontal_velocity_divisor: f32,
    /// +0x0C: aerial gravity after the float.
    pub air_gravity: f32,
}
impl NayrusLoveAttributes {
    /// The four words at `base`.
    pub fn read(r: &Reader<'_>, base: u32) -> std::result::Result<Self, FighterDescError> {
        Ok(Self {
            unused: r.f32(base)?,
            air_float_frames: r.s32(base + 0x4)?,
            air_horizontal_velocity_divisor: r.f32(base + 0x8)?,
            air_gravity: r.f32(base + 0xC)?,
        })
    }
}

/// Nayru's Love's live state: mv.zd.specialn and the reflector.
#[derive(Clone, Copy, Debug, Default)]
pub struct NayrusLove {
    /// mv.zd.specialn.x0: aerial frames before gravity applies.
    pub float_frames: i32,
    /// ftColl_CreateReflectHit's descriptor while the reflector is up.
    pub reflector: Option<ReflectDescriptor>,
}

/// What Nayru's Love's states ask of their user.
pub trait NayrusLoveUser {
    /// The grounded and aerial rows.
    const GROUND: ActionId;
    const AIR: ActionId;
    /// The grounded row's animation; the aerial row plays the next.
    const GROUND_ANIMATION: i32;
    /// The crystal on the ground and in the air (the spawner's
    /// efSync/efAlt id).
    const CRYSTALS: [u16; 2];
    fn attributes(f: &Fighter) -> &NayrusLoveAttributes;
    /// The reflector's ReflectDesc.
    fn reflection(f: &Fighter) -> &ReflectionAttributes;
    fn scratch(f: &mut Fighter) -> &mut NayrusLove;
    fn scratch_ref(f: &Fighter) -> &NayrusLove;
    /// accessory4_cb = the crystal's spawner for the row.
    fn install_accessory(f: &mut Fighter, air: bool);
    /// accessory4_cb = NULL.
    fn uninstall_accessory(f: &mut Fighter);
    /// What the ground/air change does besides the reflector: Zelda's
    /// installs the new row's crystal spawner again (ftZd_SpecialN_8013AC88
    /// / 8013AD1C); Kirby's restores the effects' hitlag callbacks once the
    /// crystal is out (ftKb_SpecialNSk_80105E8C / 80105F3C).
    fn after_ground_air_change(f: &mut Fighter, air: bool);
}

/// The grounded and aerial rows: the users' callbacks.
pub const fn rows<C: NayrusLoveUser>() -> [MotionRow; 2] {
    [
        common::row(
            C::GROUND,
            C::GROUND_ANIMATION,
            anim::<C>,
            common::no_input,
            ground_physics,
            ground_collision::<C>,
        ),
        common::row(
            C::AIR,
            C::GROUND_ANIMATION + 1,
            anim::<C>,
            common::no_input,
            air_physics::<C>,
            air_collision::<C>,
        ),
    ]
}

/// ftZd_SpecialN_Enter (8013A93C) / ftZd_SpecialAirN_Enter (8013A9B4),
/// Kirby's ftKb_SpecialNZd_80105B2C / 80105BA8: in the air the fall stops
/// and the drift is divided down (fdivs).
pub fn enter<C: NayrusLoveUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        f.physics.self_velocity.y = 0.0;
        f.physics.self_velocity.x /= C::attributes(f).air_horizontal_velocity_divisor;
    }
    f.change_motion_state(if air { C::AIR } else { C::GROUND }, a)
        .expect("Nayru's Love assets");
    f.step_animation(a);
    f.commands.variables[0] = LOWERED;
    let frames = C::attributes(f).air_float_frames;
    C::scratch(f).float_frames = frames;
    C::install_accessory(f, air);
}

/// ftZd_SpecialN_8013A830 / 8013A8AC (Kirby's fn_80105A34 / 80105AB0): the
/// crystal once per motion (x2219_b0), Fighter_SetEffectHitlagCallbacks;
/// accessory4 uninstalls.
pub fn crystal<C: NayrusLoveUser>(f: &mut Fighter, air: bool) {
    if !f.effect_state.destroy_on_state_change {
        let id = C::CRYSTALS[usize::from(air)];
        f.effects.push(EffectRequest::SyncAttached {
            id,
            bone: TRANS_JOINT,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    C::uninstall_accessory(f);
    f.core.accessory4_armed = false;
}

/// ftColl_CreateReflectHit (8007B240), literal descriptor transfer.
fn descriptor(a: &ReflectionAttributes) -> ReflectDescriptor {
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
fn raise_reflector<C: NayrusLoveUser>(f: &mut Fighter) {
    let descriptor = descriptor(C::reflection(f));
    C::scratch(f).reflector = Some(descriptor);
    f.combat.reflector_enabled = true;
}

/// ftZd_SpecialN_Anim (8013AA2C) / ftZd_SpecialAirN_Anim (8013AACC),
/// ftKb_ZdSpecialN_Anim / ftKb_ZdSpecialAirN_Anim: the script's request
/// raises the reflector, its lowering drops it (fp->reflecting = false);
/// Wait (ft_8008A2BC) or Fall at the end.
pub fn anim<C: NayrusLoveUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] == RAISE {
        f.commands.variables[0] = RAISED;
        raise_reflector::<C>(f);
    }
    if f.commands.variables[0] == LOWERED {
        f.combat.reflector_enabled = false;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == C::AIR;
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ftColl_8007AEF8: the reflector's position updates this frame.
fn refresh_reflector(f: &mut Fighter) {
    f.shield.reflect.volume.position_cached = false;
}

/// ftZd_SpecialN_Phys (8013AB68) / ftKb_ZdSpecialN_Phys: ft_80084F3C, then
/// ftColl_8007AEF8.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
    refresh_reflector(f);
}

/// ftZd_SpecialAirN_Phys (8013AB9C) / ftKb_ZdSpecialAirN_Phys: the hang
/// counts down, then the attribute's gravity with the ordinary terminal
/// speed; ftCommon_8007CF58's drift friction; ftColl_8007AEF8.
pub fn air_physics<C: NayrusLoveUser>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let hang = C::scratch_ref(f).float_frames;
    if hang != 0 {
        C::scratch(f).float_frames = hang - 1;
    } else {
        let gravity = C::attributes(f).air_gravity;
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

/// The counterpart change keeps a raised reflector (ftZd_SpecialN_8013AC88
/// / 8013AD1C, ftKb_SpecialNSk_80105E8C / 80105F3C re-create it).
fn after_ground_air_change<C: NayrusLoveUser>(f: &mut Fighter, air: bool) {
    C::after_ground_air_change(f, air);
    if f.commands.variables[0] == RAISED {
        raise_reflector::<C>(f);
    }
}

/// ftZd_SpecialN_Coll / ftKb_ZdSpecialN_Coll: off the floor, the aerial
/// row continues.
pub fn ground_collision<C: NayrusLoveUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Nayru's Love collision assets");
    common::ground_to_air(f, C::AIR, GROUND_AIR_FLAGS, assets)?;
    after_ground_air_change::<C>(f, true);
    Ok(())
}

/// ftZd_SpecialAirN_Coll / ftKb_ZdSpecialAirN_Coll: landing continues on
/// the ground.
pub fn air_collision<C: NayrusLoveUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Nayru's Love landing assets");
    common::air_to_ground(f, C::GROUND, GROUND_AIR_FLAGS, assets)?;
    after_ground_air_change::<C>(f, false);
    Ok(())
}

/// The live reflector's contact (ftColl_80077464's reflect volume).
pub fn reflector_contact<C: NayrusLoveUser>(
    f: &mut Fighter,
    hit: &melee_coll::hitbox::HitCapsule,
    scale: f32,
) -> Option<ReflectDescriptor> {
    if !f.combat.reflector_enabled {
        return None;
    }
    let descriptor = C::scratch_ref(f).reflector?;
    f.core
        .reflector_contact(hit, scale, &descriptor)
        .then_some(descriptor)
}

/// ftZd_SpecialN_8013ADB0 / fn_80105FEC: the reflector's hit callback is
/// empty.
pub fn reflect_hit(_: &mut Fighter, _: f32, _: &FighterAssets) -> Result<()> {
    Ok(())
}
