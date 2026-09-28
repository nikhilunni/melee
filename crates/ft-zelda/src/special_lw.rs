//! Transform, ftzeldaspeciallw.c (8013ADB4..8013B540).
//!
//! Zelda spins into Sheik: SpecialLw plays out (its start sparkle on
//! accessory4), then accessory4 hands the match to Sheik
//! (ftCommon_8007EFC8, performed by the scene). Sheik's arrival enters her
//! SpecialLw2 (ftSk_SpecialLw_80114758); Zelda's SpecialLw2 rows here are
//! her own arrival when Sheik transforms into her
//! ([`arrive`], ftZd_SpecialLw_8013B4D8).
use crate::{
    common,
    init::{Accessory, Zelda},
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};

/// ftZd_MS_SpecialLw (355), SpecialLw2 (356), SpecialAirLw (357) and
/// SpecialAirLw2 (358).
pub const GROUND: ActionId = ActionId(355);
pub const GROUND_ARRIVAL: ActionId = ActionId(356);
pub const AIR: ActionId = ActionId(357);
pub const AIR_ARRIVAL: ActionId = ActionId(358);
/// ftZd_SM_SpecialLw..SpecialAirLw2.
pub const ANIMATIONS: [i32; 4] = [307, 308, 309, 310];

/// efSync 0x4FC / 0x4FD: the transformation's start (at `fp->parts[104]`)
/// and arrival (at `fp->parts[FtPart_HipN]`, joint 4)
/// sparkles, hsd_8039EFAC(0, 0x11, 0x426D / 0x4271, jobj).
const START_SPARKLE: u16 = 0x4FC;
const ARRIVAL_SPARKLE: u16 = 0x4FD;
const START_SPARKLE_JOINT: usize = 104;
const ARRIVAL_SPARKLE_JOINT: usize = 4;
/// `fp->parts[FtPart_TopN]`: the model root, joint 0.
const TOP_JOINT: usize = 0;

/// lb_800119DC(&top, 120, 0.4, 0.003, MTXDegToRad(60)): the spin's gust.
const GUST_FRAMES: i32 = 120;
const GUST_STRENGTH: f32 = 0.4;
const GUST_DECAY: f32 = 0.003;
const GUST_PHASE_STEP: f32 = std::f32::consts::FRAC_PI_3;

/// ftZd_MF_SpecialLw_Coll: ftCommon_GroundAirColl_MF (SkipMatAnim,
/// SkipColAnim, UpdateCmd, SkipItemVis, Unk19, SkipModelPartVis,
/// SkipModelFlags, Unk27) with KeepGfx, KeepColAnimHitStatus and SkipHit.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(
    1 << 1
        | 1 << 2
        | 1 << 3
        | 1 << 7
        | 1 << 12
        | 1 << 14
        | 1 << 18
        | 1 << 19
        | 1 << 22
        | 1 << 26
        | 1 << 27,
);

fn attributes(f: &Fighter) -> &crate::attributes::TransformAttributes {
    &f.character.get::<Zelda>().attributes.transform
}

fn install_accessory(f: &mut Fighter, accessory: Accessory) {
    f.character.get_mut::<Zelda>().accessory = accessory;
    f.core.arm_accessory4();
}

/// ftZd_SpecialLw_Enter (8013AEE0) / ftZd_SpecialAirLw_Enter (8013AFA4):
/// the spin, the momentum divided down (fdivs), the gust at the model
/// root.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Transform assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    f.commands.variables[0] = 0;
    let (horizontal, vertical) = {
        let t = attributes(f);
        (t.horizontal_velocity_divisor, t.vertical_velocity_divisor)
    };
    let physics = &mut f.core.physics;
    physics.self_velocity.x /= horizontal;
    physics.self_velocity.y /= vertical;
    physics.ground_velocity /= horizontal;
    let center = common::joint_position(f, TOP_JOINT);
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
    install_accessory(f, Accessory::TransformStart);
}

/// ftZd_SpecialLw_8013ADB4 / 8013AE30: the sparkle once per motion (x2219_b0),
/// Fighter_SetEffectHitlagCallbacks, then accessory4 uninstalls.
pub fn sparkle(f: &mut Fighter, arrival: bool) {
    if !f.effect_state.destroy_on_state_change {
        let (id, bone) = if arrival {
            (ARRIVAL_SPARKLE, ARRIVAL_SPARKLE_JOINT)
        } else {
            (START_SPARKLE, START_SPARKLE_JOINT)
        };
        f.effects.push(EffectRequest::SyncAttached { id, bone });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    f.character.get_mut::<Zelda>().accessory = Accessory::None;
    f.core.accessory4_armed = false;
}

/// ftZd_SpecialLw_8013AEAC: accessory4 = NULL, then ftCommon_8007EFC8(gobj,
/// ftSk_SpecialLw_80114758): the scene swaps Sheik in.
pub fn hand_over(f: &mut Fighter) {
    f.character.get_mut::<Zelda>().accessory = Accessory::None;
    f.core.accessory4_armed = false;
    f.core.request_transformation();
}

/// ftZd_SpecialLw_Anim (8013B068) / ftZd_SpecialAirLw_Anim: at the spin's
/// end, accessory4 becomes the handover.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        install_accessory(f, Accessory::TransformHandOver);
    }
    Ok(None)
}

/// ftZd_SpecialLw2_Anim (8013B2A4) / ftZd_SpecialAirLw2_Anim: Wait or Fall.
pub fn arrival_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == AIR_ARRIVAL;
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ft_80084F3C.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftZd_SpecialAirLw_Phys / SpecialAirLw2_Phys: the transformation's own
/// gravity (ftCommon_Fall), then ftCommon_8007CEF4.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, terminal) = {
        let t = attributes(f);
        (t.gravity, t.terminal_velocity)
    };
    common::fall(f, gravity, terminal);
    common::aerial_friction(f);
    common::finish_air(f, &p);
}

/// ftZd_SpecialLw_Coll / SpecialLw2_Coll: off the floor, the aerial row
/// (ftZd_SpecialLw_8013B1CC / _8013B400) with its sparkle accessory.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Transform collision assets");
    let arrival = f.motion_state.action == GROUND_ARRIVAL;
    let state = if arrival { AIR_ARRIVAL } else { AIR };
    common::ground_to_air(f, state, GROUND_AIR_FLAGS, assets)?;
    install_sparkle(f, arrival);
    Ok(())
}

/// ftZd_SpecialAirLw_Coll / SpecialAirLw2_Coll: on landing, the grounded
/// row (ftZd_SpecialLw_8013B238 / _8013B46C).
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Transform landing assets");
    let arrival = f.motion_state.action == AIR_ARRIVAL;
    let state = if arrival { GROUND_ARRIVAL } else { GROUND };
    common::air_to_ground(f, state, GROUND_AIR_FLAGS, assets)?;
    install_sparkle(f, arrival);
    Ok(())
}

fn install_sparkle(f: &mut Fighter, arrival: bool) {
    install_accessory(
        f,
        if arrival {
            Accessory::TransformArrival
        } else {
            Accessory::TransformStart
        },
    );
}

/// ftZd_SpecialLw_8013B4D8 (8013B4D8): Zelda takes over from Sheik,
/// entering SpecialLw2 (or SpecialAirLw2) at the attribute's frame.
pub fn arrive(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let state = if f.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        GROUND_ARRIVAL
    } else {
        AIR_ARRIVAL
    };
    let start = attributes(f).finish_start_frame;
    f.change_motion_state_with_flags(state, a, MotionEntryFlags(0), start, 1.0)?;
    install_accessory(f, Accessory::TransformArrival);
    Ok(())
}
