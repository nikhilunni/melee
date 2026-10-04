//! Confusion, ftmewtwospecials.c (8014665C..80146CCC).
//!
//! One row on the ground (351) and one in the air (352). The special arms a
//! grab (type 0x40): a caught fighter hangs from Mewtwo in ThrownMewtwo
//! (melee-ft's capture_mewtwo) while Mewtwo's own motion plays on, and the
//! script's cmd_vars[0] lets it go into DamageFall with the throw's hit.
//! cmd_vars[1] raises (1) and lowers (2) a reflector that leaves a
//! reflected item its owner (x2218_b4). The first aerial use each airtime
//! lifts Mewtwo by attribute x18.
use crate::{
    common,
    init::{Accessory, Mewtwo},
};
use melee_coll::defense::ReflectDescriptor;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        capture_mewtwo,
        grab::GrabLink,
        ledge::GrabExclusions,
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};

/// ftMt_MS_SpecialS (351) / ftMt_MS_SpecialAirS (352).
pub const GROUND: ActionId = ActionId(351);
pub const AIR: ActionId = ActionId(352);

/// FTMEWTWO_SPECIALS_COLL_FLAG: ftCommon_GroundAirColl_MF with KeepGfx.
const TRANSITION_FLAGS: MotionEntryFlags =
    MotionEntryFlags(common::GROUND_AIR_COLL_BASE_FLAGS.0 | 1 << 1);

/// ftCommon_8007E2D0(fp, 0x40, ...): Confusion's grab type (x1A68).
pub const GRAB_TYPE: GrabExclusions = GrabExclusions(0x40);

/// cmd_vars[1]: CONFUSION_REFLECT_ON / CONFUSION_REFLECT_OFF.
const REFLECT_ON: u32 = 1;
const REFLECT_OFF: u32 = 2;

/// mv.mt.SpecialS (ftMewtwo/types.h) with the reflector
/// ftColl_CreateReflectHit installed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Confusion {
    /// isConfusionReflect: the reflector is up.
    pub reflecting: bool,
    pub reflector: Option<ReflectDescriptor>,
}

pub const fn rows() -> [MotionRow; 2] {
    [
        common::row(
            GROUND,
            305,
            ground_anim,
            common::no_input,
            ground_physics,
            ground_collision,
        ),
        common::row(
            AIR,
            306,
            air_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
    ]
}

fn scratch(f: &mut Fighter) -> &mut Confusion {
    &mut f.character.get_mut::<Mewtwo>().confusion
}

fn install_accessory(f: &mut Fighter) {
    f.character.get_mut::<Mewtwo>().accessory = Accessory::ConfusionReflect;
    f.core.arm_accessory4();
}

/// ftMewtwo_SpecialS_SetGrab / ftMewtwo_SpecialAirS_SetGrab: with no victim
/// the grab is armed (ftCommon_8007E2D0; the grabbed_cb is ftCo_800BCF18 on
/// the ground and ftCo_800BD000 in the air, chosen from Mewtwo's row when
/// the grab lands); with one, nothing else can be caught
/// (ftCommon_8007E2F4(fp, 0x1FF)).
fn arm_grab(f: &mut Fighter) {
    if f.combat.grab.is_none() {
        f.core.status.special_grab = Some(GRAB_TYPE);
    } else {
        f.core.status.grab_exclusions = GrabExclusions::ALL;
    }
}

/// ftMt_SpecialS_Enter (801466C4) / ftMt_SpecialAirS_Enter (8014677C): the
/// flags cleared, the aerial lift once per airtime (x223C), the grab armed
/// and the reflector's accessory installed.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    scratch(f).reflecting = false;
    if air && !f.character.get::<Mewtwo>().confusion_boost_used {
        let boost = f.character.get::<Mewtwo>().attributes.confusion.air_boost;
        f.physics.self_velocity.y = boost;
        f.character.get_mut::<Mewtwo>().confusion_boost_used = true;
    }
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Confusion assets");
    f.step_animation(a);
    arm_grab(f);
    install_accessory(f);
}

/// ftMt_SpecialS_SetFlags (8014665C), the grab_cb: nothing else can be
/// caught, every velocity stops (ftCommon_8007E2FC), the grab disarms
/// (x221E_b6) and a cape no longer turns Mewtwo (x2222_b2). Mewtwo's motion
/// plays on. Then the common grabbed_cb on the victim.
pub fn grab(
    mewtwo: &mut Fighter,
    victim: &mut Fighter,
    mewtwo_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    mewtwo.core.status.grab_exclusions = GrabExclusions::ALL;
    mewtwo.core.clear_movement();
    mewtwo.core.status.special_grab = None;
    mewtwo.character.get_mut::<Mewtwo>().cape_turn_blocked = true;
    mewtwo.core.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: 0.0,
    });
    let air = mewtwo.motion_state.action == AIR;
    capture_mewtwo::enter(victim, mewtwo, victim_assets, mewtwo_assets, air)
}

/// ftMewtwo_SetGrabVictim: on the script's cmd_vars[0], a held victim is
/// let go (ftCommon_8007E2F4(fp, 0), ftCo_800DE2A8, ftCo_80090780), which
/// the scene runs on the pair right after this callback.
fn release_victim(f: &mut Fighter) {
    if f.commands.variables[0] != 0 && matches!(f.combat.grab, Some(GrabLink::Holding { .. })) {
        f.core.status.grab_exclusions = GrabExclusions::NONE;
        f.combat.special_throw_release = true;
        f.commands.variables[0] = 0;
    }
}

/// ftMt_SpecialS_Anim (80146858): the release; Wait at the end.
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    release_victim(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftMt_SpecialAirS_Anim (801468EC): the release; Fall at the end.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    release_victim(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftColl_8007AEF8: the reflector's position updates this frame.
fn refresh_reflector(f: &mut Fighter) {
    f.shield.reflect.volume.position_cached = false;
}

/// ftMt_SpecialS_Phys (80146988): ft_80084F3C, then ftColl_8007AEF8.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::ground_friction(f, p);
    refresh_reflector(f);
}

/// ftMt_SpecialAirS_Phys (801469BC): ft_80084EEC, then ftColl_8007AEF8.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::air_friction_fall(f, p);
    refresh_reflector(f);
}

/// ftMewtwo_SpecialS_SetReflect: a raised reflector survives the ground/air
/// counterpart change (fp->reflecting, x2218_b4 and reflect_hit_cb again).
fn keep_reflector(f: &mut Fighter) {
    if scratch(f).reflecting {
        f.combat.reflector_enabled = true;
    }
}

/// ftMt_SpecialS_Coll (80146BB8) -> ft_8008403C: off the floor,
/// ftMt_SpecialS_GroundToAir (801469F0): the aerial row at the current
/// frame, the drift clamped (ftCommon_ClampAirDrift), the grab armed again
/// and the reflector kept.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Confusion collision assets");
    // The change keeps the victim, and with Ft_MF_Unk19 x2222_b2
    // (fighter.c:1021-1023).
    let held = (f.combat.grab, f.character.get::<Mewtwo>().cape_turn_blocked);
    common::ground_to_air(f, AIR, TRANSITION_FLAGS, assets)?;
    f.combat.grab = held.0;
    f.character.get_mut::<Mewtwo>().cape_turn_blocked = held.1;
    let drift = f.attributes.air.air_drift_max;
    common::clamp_self_velocity_x(f, drift);
    arm_grab(f);
    install_accessory(f);
    keep_reflector(f);
    Ok(())
}

/// ftMt_SpecialAirS_Coll (80146BE0) -> ft_80082C74: landing,
/// ftMt_SpecialAirS_AirToGround (80146AD4): the grounded row at the current
/// frame, the grab armed again, the lift restored and the reflector kept.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Confusion landing assets");
    let held = (f.combat.grab, f.character.get::<Mewtwo>().cape_turn_blocked);
    common::air_to_ground(f, GROUND, TRANSITION_FLAGS, assets)?;
    f.combat.grab = held.0;
    f.character.get_mut::<Mewtwo>().cape_turn_blocked = held.1;
    arm_grab(f);
    install_accessory(f);
    f.character.get_mut::<Mewtwo>().confusion_boost_used = false;
    keep_reflector(f);
    Ok(())
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

/// ftMt_SpecialS_ReflectThink (80146C08), accessory4 for the whole special:
/// cmd_vars[1] raises the reflector (ftColl_CreateReflectHit, then x2218_b4)
/// or lowers it.
pub fn reflect_think(f: &mut Fighter) {
    match f.commands.variables[1] {
        REFLECT_ON => {
            let reflector =
                descriptor(&f.character.get::<Mewtwo>().attributes.confusion.reflection);
            let confusion = scratch(f);
            confusion.reflector = Some(reflector);
            confusion.reflecting = true;
            f.combat.reflector_enabled = true;
            f.commands.variables[1] = 0;
        }
        REFLECT_OFF => {
            if scratch(f).reflecting {
                f.combat.reflector_enabled = false;
                scratch(f).reflecting = false;
            }
            f.commands.variables[1] = 0;
        }
        _ => {}
    }
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

/// ftMt_SpecialS_OnReflect (80146CC8): the reflector's hit callback is
/// empty.
pub fn reflect_hit(_: &mut Fighter, _: f32, _: &FighterAssets) -> Result<()> {
    Ok(())
}
