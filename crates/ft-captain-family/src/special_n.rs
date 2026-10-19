//! Falcon Punch, ftcaptainspecialn.c (800E2B80..800E3278), and Kirby's
//! copied Falcon and Warlock Punch, ftkirbyspecialcaptain.c
//! (800F99BC..800F9FDC): the same states over Kirby's rows. The two files
//! differ only in what [`FalconPunch`] names: the rows, the attributes, the
//! punch's effect and the wind.
//!
//! The motion script drives the move: its first `MoveCue` (throw_flags_b1)
//! spawns the punch's two attached models, the second removes them. The
//! aerial script sets cmd_vars[0] once to aim the lunge and walks
//! cmd_vars[1] through the three aerial physics phases.
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionPreservation,
    },
    physics::airborne,
};
use melee_types::CommonMotionState;

use crate::{attributes::FalconPunchAttributes, CaptainFamily};

/// ftCa_MS_SpecialN (347) and ftCa_MS_SpecialAirN (348).
pub const GROUND: ActionId = ActionId(347);
pub const AIR: ActionId = ActionId(348);

/// `MTXDegToRad(1)` as MWCC rounds it (retail @274, 800E2ED0; Kirby's
/// @261).
const DEGREES_TO_RADIANS: f32 = 0.017453292;

/// The GroundToAir/AirToGround changes' flags: KeepGfx | SkipMatAnim |
/// SkipRumble | UpdateCmd | SkipColAnim | SkipItemVis | Unk19 |
/// SkipModelPartVis | SkipModelFlags | Unk27 (ftcaptainspecialn.c:189-192;
/// ftKb_Ca_transition_flags). Without KeepColAnimHitStatus or SkipHit, only
/// the effects stay.
const GROUND_AIR_PRESERVATION: MotionPreservation = MotionPreservation {
    hit_status: false,
    hitboxes: false,
    effects: true,
    fast_fall: false,
};

/// The punch's attached pair of models: doPhys's efSync id and the two
/// fighter joints (`fp->parts[n]`) they hang from.
#[derive(Clone, Copy, Debug)]
pub struct PunchEffect {
    pub id: u16,
    pub bones: [usize; 2],
}

/// Who throws a Falcon Punch: Captain Falcon and Ganondorf (every
/// [`CaptainFamily`]) and Kirby with either one's copied ability, a marker
/// type whose methods read Kirby's own state.
pub trait FalconPunch: 'static {
    /// ftCaptain_SpecialN_CreateWindEffect: Ganondorf's own Anim callbacks
    /// push wind; Kirby's (ftKb_CaSpecialN_Anim, 800F9AEC) never do.
    const WIND: bool;
    /// The grounded and the aerial row, in that order.
    fn rows(f: &Fighter) -> [ActionId; 2];
    fn attributes(f: &Fighter) -> &FalconPunchAttributes;
    /// doPhys's `switch` on the kind; `None` for a kind without an arm.
    fn effect(f: &Fighter) -> Option<PunchEffect>;
}

/// Captain Falcon's and Ganondorf's own rows 347 and 348.
impl<C: CaptainFamily> FalconPunch for C {
    const WIND: bool = C::EFFECTS.punch_wind;
    fn rows(_: &Fighter) -> [ActionId; 2] {
        [GROUND, AIR]
    }
    fn attributes(f: &Fighter) -> &FalconPunchAttributes {
        &crate::attributes::<C>(f).falcon_punch
    }
    fn effect(_: &Fighter) -> Option<PunchEffect> {
        Some(PunchEffect {
            id: C::EFFECTS.punch,
            bones: C::EFFECTS.punch_bones,
        })
    }
}

/// ftCa_SpecialN_Enter (800E2B80) / ftCa_SpecialAirN_Enter (800E2C00), and
/// Kirby's ftKb_SpecialNCa_800F99BC / 800F9A54.
pub fn enter<P: FalconPunch>(f: &mut Fighter, airborne: bool, a: &FighterAssets) {
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    f.commands.clear_throw_flags();
    let [ground, air] = P::rows(f);
    f.change_motion_state(if airborne { air } else { ground }, a)
        .expect("Falcon Punch assets");
    // Fighter_SetEffectHitlagCallbacks.
    f.effect_state.hitlag_callbacks = true;
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// ftCa_SpecialN_Anim (800E2C80) and ftKb_CaSpecialN_Anim (800F9AEC): the
/// wind effect is Ganondorf's only; Wait at the animation's end
/// (ft_8008A2BC).
pub fn ground_anim<P: FalconPunch>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    wind::<P>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// ftCa_SpecialAirN_Anim (800E2D5C) and ftKb_CaSpecialAirN_Anim
/// (800F9B28): the same wind; Fall at the animation's end.
pub fn air_anim<P: FalconPunch>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    wind::<P>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
    }
    Ok(None)
}

/// ftCaptain_SpecialN_CreateWindEffect (inlined at 800E2C9C): Ganondorf's
/// wind-up pushes dynamics chains from his position on odd frames,
/// lb_800119DC(&cur_pos, 2, s, s, 0.0): strength 2.0 on frames 16..=50
/// and 4.0 on 51..=68 (@256, @258; @257 is the 0.0 phase step).
fn wind<P: FalconPunch>(f: &mut Fighter) {
    if !P::WIND {
        return;
    }
    // 800E2CA4: fctiwz of cur_anim_frame.
    let frame = gekko_math::msl::fctiwz(f.animation.frame);
    if frame & 1 == 0 {
        return;
    }
    let strength = match frame {
        16..=50 => 2.0,
        51..=68 => 4.0,
        _ => return,
    };
    let center = f.physics.position;
    f.commands
        .gusts
        .push(melee_lb::radial_force::RadialImpulse {
            center,
            frames: 2,
            strength,
            decay: strength,
            phase_step: 0.0,
        });
}

/// ftCa_SpecialN_IASA (800E2E38) and ftKb_CaSpecialN_IASA (800F9B64) are
/// empty.
pub fn ground_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftCa_SpecialAirN_IASA (800E2E3C) and ftKb_CaSpecialAirN_IASA (800F9B68):
/// once cmd_vars[0] is set, launch along the stick's vertical angle. No
/// fused sites in either (asm --fused is empty).
pub fn air_input<P: FalconPunch>(f: &mut Fighter, _: InputPhase<'_>) {
    if f.commands.variables[0] == 0 {
        return;
    }
    f.commands.variables[0] = 0;
    let punch = P::attributes(f);
    let angle = lunge_angle(f.input.current.stick.y, punch);
    let speed = punch.aerial_speed;
    f.physics.self_velocity.y = speed * gekko_math::msl::sinf(angle);
    f.physics.self_velocity.x = speed * (f.physics.facing * gekko_math::msl::cosf(angle));
}

/// ftCaptain_SpecialN_GetAngleVel: the stick's vertical magnitude, clamped
/// to the upward threshold, less the downward one, scaled to the maximum
/// angle and signed like the stick.
fn lunge_angle(stick_y: f32, punch: &FalconPunchAttributes) -> f32 {
    let maximum = punch.upward_stick_threshold;
    let minimum = punch.downward_stick_threshold;
    let mut magnitude = if stick_y < 0.0 { -stick_y } else { stick_y };
    if magnitude > maximum {
        magnitude = maximum;
    }
    magnitude -= minimum;
    if magnitude < 0.0 {
        magnitude = 0.0;
    }
    if stick_y < 0.0 {
        magnitude = -magnitude;
    }
    // 800E2ED4..EDC (Kirby's 800F9C00..C08): fmuls, fdivs, fmuls, each
    // rounded.
    DEGREES_TO_RADIANS * (magnitude * punch.maximum_angle / (maximum - minimum))
}

/// doPhys (inlined in both Phys callbacks): the script's cue spawns the
/// punch models the first time (x2219_b0 unset) and removes every owned
/// effect the second (ftCommon_8007DB24). A kind without an arm still sets
/// the bit.
fn punch_effect<P: FalconPunch>(f: &mut Fighter) {
    if !f.commands.take_move_cue() {
        return;
    }
    if !f.effect_state.destroy_on_state_change {
        if let Some(PunchEffect { id, bones }) = P::effect(f) {
            f.effects
                .push(EffectRequest::SyncAttachedPair { id, bones });
        }
        f.effect_state.destroy_on_state_change = true;
    } else {
        f.effects.push(EffectRequest::DestroyOwned);
        f.effect_state.destroy_on_state_change = false;
    }
}

/// ftCa_SpecialN_Phys (800E2F2C) and ftKb_CaSpecialN_Phys (800F9C58):
/// doPhys, then ft_80084FA8's root motion.
pub fn ground_physics<P: FalconPunch>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    punch_effect::<P>(f);
    callbacks::physics::jab(f, p);
}

/// ftCa_SpecialAirN_Phys (800E3018) and ftKb_CaSpecialAirN_Phys (800F9D40):
/// doPhys, then the phase cmd_vars[1] selects: ft_80084EEC before the
/// lunge, a velocity decay during it, ft_80084DB0 after it.
pub fn air_physics<P: FalconPunch>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    punch_effect::<P>(f);
    match f.commands.variables[1] {
        0 => {
            // ft_80084EEC: gravity and air friction, no stick input.
            let air = &f.core.attributes.air;
            let physics = &mut f.core.physics;
            physics.self_velocity.y =
                airborne::gravity(physics.self_velocity.y, air.gravity, air.terminal_velocity);
            physics.animation_velocity.x =
                airborne::drift_acceleration(physics.self_velocity.x, 0.0, 0.0, air);
        }
        1 => {
            // 800E3120..3C (Kirby's 800F9E44..60): separate fmuls, y before x.
            let multiplier = P::attributes(f).momentum_multiplier;
            f.physics.self_velocity.y *= multiplier;
            f.physics.self_velocity.x *= multiplier;
        }
        2 => return callbacks::physics::pass(f, p),
        _ => {}
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftCa_SpecialN_Coll (800E3168) and ftKb_CaSpecialN_Coll (800F9E8C):
/// ft_800827A0 stops at the floor's edge; losing the floor continues the
/// punch in the air.
pub fn ground_collision<P: FalconPunch>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    let result = ground::map_escape(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    );
    if matches!(result, ground::WaitGroundResult::Supported) {
        return Ok(());
    }
    // ftCommon_GroundToAirStateChange -> ftCommon_8007D5D4.
    f.leave_ground();
    let [_, air] = P::rows(f);
    f.change_ground_air_motion(
        air,
        p.assets.expect("Falcon Punch collision assets"),
        GROUND_AIR_PRESERVATION,
    )?;
    f.effect_state.hitlag_callbacks = true;
    // ftCommon_ClampAirDrift (8007D468).
    let maximum = f.attributes.air.air_drift_max;
    f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-maximum, maximum);
    Ok(())
}

/// ftCa_SpecialAirN_Coll (800E31F4) and ftKb_CaSpecialAirN_Coll (800F9F34):
/// ft_80081D0C; landing keeps punching.
pub fn air_collision<P: FalconPunch>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    ) {
        f.land();
        let [ground, _] = P::rows(f);
        f.change_ground_air_motion(
            ground,
            p.assets.expect("Falcon Punch landing assets"),
            GROUND_AIR_PRESERVATION,
        )?;
        f.effect_state.hitlag_callbacks = true;
    }
    Ok(())
}
