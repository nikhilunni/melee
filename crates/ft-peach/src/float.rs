//! Float, ftpeachfloat.c (8011BA54..8011BE7C), ftpeachfloatfall.c
//! (8011BD6C..8011BE7C).
use crate::init::Peach;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, FloatInputPhase,
    },
    input::{Buttons, FighterInput},
    physics::airborne,
};
use melee_types::{CommonMotionState, FtPart};

/// ftPe_MS_Float (341).
pub const FLOAT: ActionId = ActionId(341);
/// ftPe_MS_FloatFallF (342).
pub const FLOAT_FALL_FORWARD: ActionId = ActionId(342);
/// ftPe_MS_FloatFallB (343).
pub const FLOAT_FALL_BACKWARD: ActionId = ActionId(343);
/// efAsync_Spawn kind 0 effect 1236 on TransN (efsync.c:443-445).
const FLOAT_SPARKLE: u16 = 0x4D4;

/// ftPe_8011BA54 / ftPe_8011BAD8: the start and continue predicates. The
/// caller (Fall, Jump, JumpAerial and damage IASA) enters with timer reset.
pub fn float_input_selected(
    peach: &Peach,
    input: &FighterInput,
    assets: &FighterAssets,
    vertical_velocity: f32,
    phase: FloatInputPhase,
) -> bool {
    peach.has_float
        && match phase {
            // checkStartFloatInput: stick down past PlCo +88 while X/Y is held.
            FloatInputPhase::BeforeAerialJump => {
                input.current.stick.y <= -assets.jumping.fast_fall_threshold
                    && input.current.held.intersects(Buttons::XY)
            }
            FloatInputPhase::AfterAerialJump => {
                vertical_velocity <= 0.0 && continue_input(input, assets)
            }
        }
}

/// ftPe_Float_CheckContinueInput (8011B9F4): stick up past the tap-jump
/// threshold, or X/Y held.
pub(crate) fn continue_input(input: &FighterInput, assets: &FighterAssets) -> bool {
    input.current.stick.y >= assets.input.thresholds.tap_jump_threshold
        || input.current.held.intersects(Buttons::XY)
}

/// The IASA callers' entry: ftPe_8011BB6C(gobj, true).
pub fn enter_from_input(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    enter(f, assets, true)
}

/// ftPe_8011BB6C (8011BB6C): Float at frame zero. `reset_timer` is false
/// when a float aerial returns to Float (ftPe_FloatAttackAir_Anim).
pub fn enter(f: &mut Fighter, assets: &FighterAssets, reset_timer: bool) -> Result<()> {
    f.change_motion_state(FLOAT, assets)?;
    let peach = f.character.get_mut::<Peach>();
    peach.has_float = false;
    if reset_timer {
        peach.float_remaining = peach.attributes.float.duration;
    }
    f.physics.self_velocity.y = 0.0;
    // x2219_b0: the sparkle is destroyed by the next motion change that
    // does not keep effects.
    f.effect_state.destroy_on_state_change = true;
    let bone = usize::from(assets.parts.joint(FtPart::TransN).expect("float TransN"));
    f.effects.push(EffectRequest::Attached {
        id: FLOAT_SPARKLE,
        bone,
    });
    Ok(())
}

/// ftPe_UpdateFloatDir (8011BD6C): FloatFallF or FloatFallB with Ft_MF_KeepGfx,
/// started at the fall animation's recorded frame less the attribute offset.
fn enter_float_fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    // Retail 8011BD8C: separate fmuls, then fcmpo against -PlCo +78.
    let forward = f.input.current.stick.x * f.physics.facing > -assets.jumping.backward_threshold;
    let attributes = &f.character.get::<Peach>().attributes.float;
    let (state, start) = if forward {
        (FLOAT_FALL_FORWARD, attributes.forward_fall_start)
    } else {
        (FLOAT_FALL_BACKWARD, attributes.backward_fall_start)
    };
    // Retail 8011BDD4: fsubs.
    let start = start - attributes.fall_start_offset;
    f.change_motion_state_keeping_effects(state, assets, start)
}

/// ftPe_Float_Anim (8011BC38): count the float timer down, then fall.
pub fn float_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let peach = f.character.get_mut::<Peach>();
    if peach.float_remaining > 0.0 {
        peach.float_remaining -= 1.0;
    }
    if peach.float_remaining <= 0.0 {
        enter_float_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftPe_Float_IASA (8011BC8C): special, float aerial, then release.
pub fn float_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.try_air_special(p.assets)
        || crate::float_attack::try_enter(f, p.assets).expect("float aerial")
    {
        return;
    }
    if !continue_input(&f.input, p.assets) {
        enter_float_fall(f, p.assets).expect("float release");
    }
}

/// ftPe_Float_Phys (8011BD08) -> ftCommon_8007D268: drift only; the float
/// holds vertical velocity at zero.
pub fn float_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    f.physics.animation_velocity.x = airborne::drift(
        f.physics.self_velocity.x,
        f.input.current.stick.x,
        &f.attributes.air,
    );
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPe_FloatFall_Anim (8011BDF0): FallAerial once the animation ends.
pub fn float_fall_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::FallAerial.into(), p.assets)?;
    }
    Ok(None)
}

/// ftPe_FloatFall_IASA (8011BE34): no interrupts.
pub fn float_fall_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftPe_FloatFall_Phys (8011BE38) -> ftCo_JumpAerial_Phys_Cb.
pub fn float_fall_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    f.core.root_motion_aerial_physics(p.assets);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPe_Float_Coll / ftPe_FloatFall_Coll -> ft_800831CC(ftCo_80096CC8,
/// ft_80082B1C): Fall's collision, with walljump and ledge grab.
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    callbacks::collision::fall(f, p)
}
