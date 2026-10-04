//! PSI Magnet, ftnessspeciallw.c (80119E14..8011B51C): an absorb volume
//! held while B is down. An absorbed hit heals (ftNs_AbsorbThink_
//! DecideAction) and plays the hit rows; in the air the move stalls, then
//! falls at its own rate.
//!
//! The turn rows (371, 376) have callbacks but no entry in retail: nothing
//! changes to them, so they stay unported.
use crate::{
    attributes::MagnetAttributes,
    common::{self, row},
    init::Ness,
};
use melee_coll::defense::AbsorbDescriptor;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        absorb::Absorbed,
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
    physics::friction,
};
use melee_types::{FtPart, GroundOrAir};

/// ftNs_MS_SpecialLwStart..SpecialLwEnd (367..370) and the aerial rows
/// (372..375), which follow in the same order five rows on.
pub const START: ActionId = ActionId(367);
pub const HOLD: ActionId = ActionId(368);
pub const HIT: ActionId = ActionId(369);
pub const END: ActionId = ActionId(370);
const AIR_OFFSET: u16 = 5;

/// efAsync_Spawn(gobj, &fp->x60C, 0, 1264, parts[FtPart_L1stNb]): the
/// magnet's model (efSync 0x4F0, model 0x2712).
const MAGNET_EFFECT: u16 = 0x4F0;
/// ft_80088478(fp, 210081, 127, 64): the magnet's hum, every 40 frames.
const HUM_SOUND: u32 = 210_081;
const HUM_FRAMES: i32 = 40;

/// FTNESS_SPECIALLW_COLL_FLAG / FTNESS_SPECIALLW_END_FLAG.
const COLL: MotionEntryFlags = MotionEntryFlags(common::GROUND_AIR.0 | common::KEEP_GFX);
const END_COLL: MotionEntryFlags = common::GROUND_AIR;
const KEEP_GFX: MotionEntryFlags = MotionEntryFlags(common::KEEP_GFX);

/// mv.ns.speciallw.
#[derive(Clone, Debug, Default)]
pub struct Magnet {
    /// +2340 releaseLag: frames the magnet stays out with B released.
    pub release_lag: i32,
    /// +2348 isRelease: B has been let go; the latch never re-arms.
    pub released: bool,
    /// +234C gravityDelay: frames before the aerial move falls.
    pub gravity_delay: i32,
    /// +2350 x10: frames until the next hum.
    pub hum_frames: i32,
}

pub const fn rows() -> [MotionRow; 8] {
    [
        row(
            START,
            0x13E,
            start_anim,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            HOLD,
            0x13F,
            hold_anim,
            common::no_input,
            ground_physics,
            ground_collision,
        ),
        row(
            HIT,
            0x140,
            hit_anim,
            common::no_input,
            ground_physics,
            ground_collision,
        ),
        row(
            END,
            0x141,
            end_anim,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            air(START),
            0x142,
            start_anim,
            common::no_input,
            air_physics::<false>,
            air_collision,
        ),
        row(
            air(HOLD),
            0x143,
            hold_anim,
            common::no_input,
            air_physics::<true>,
            air_collision,
        ),
        row(
            air(HIT),
            0x144,
            hit_anim,
            common::no_input,
            air_physics::<true>,
            air_collision,
        ),
        row(
            air(END),
            0x145,
            end_anim,
            common::no_input,
            air_physics::<false>,
            air_collision,
        ),
    ]
}

const fn air(ground: ActionId) -> ActionId {
    ActionId(ground.0 + AIR_OFFSET)
}

/// The row of `ground` for the fighter's ground_or_air.
fn placed(f: &Fighter, ground: ActionId) -> ActionId {
    if f.physics.ground_or_air == GroundOrAir::Air {
        air(ground)
    } else {
        ground
    }
}

fn attributes(f: &Fighter) -> &MagnetAttributes {
    &f.character.get::<Ness>().attributes.magnet
}

fn magnet(f: &mut Fighter) -> &mut Magnet {
    &mut f.character.get_mut::<Ness>().magnet
}

/// ftNs_SpecialLwStart_Enter (80119E14) / ftNs_SpecialAirLwStart_Enter
/// (80119E90): the aerial entry stops the fall and divides the drift by x88.
pub fn enter(f: &mut Fighter, air_entry: bool, a: &FighterAssets) {
    let (lag, delay, divisor) = {
        let m = attributes(f);
        (
            gekko_math::msl::fctiwz(m.release_lag),
            m.gravity_delay,
            m.entry_velocity_divisor,
        )
    };
    // mv+4 (turnFrames) is never written: it stays the predecessor's.
    let retained_word = f.inherited_scratch_word();
    *magnet(f) = Magnet {
        release_lag: lag,
        released: false,
        gravity_delay: delay,
        hum_frames: 0,
    };
    if air_entry {
        f.physics.self_velocity.y = 0.0;
        f.physics.self_velocity.x /= divisor;
    }
    f.change_motion_state(if air_entry { air(START) } else { START }, a)
        .expect("PSI Magnet assets");
    f.character.get_mut::<Ness>().retained_word = retained_word;
    f.step_animation(a);
}

/// The B latch every row but the end keeps: `isRelease` once B is up.
fn latch_release(f: &mut Fighter) {
    if !f.input.current.held.intersects(Buttons::B) {
        magnet(f).released = true;
    }
}

/// ftColl_CreateAbsorbHit(gobj, &x98_PSI_MAGNET_ABSORPTION).
fn raise_volume(f: &mut Fighter) {
    let v = &attributes(f).volume;
    let descriptor = AbsorbDescriptor {
        bone: v.bone as usize,
        offset: v.offset,
        radius: v.radius,
    };
    f.core.create_absorb_hit(&descriptor);
}

/// The magnet's model once per hold (x2219_b0), then
/// Fighter_SetEffectHitlagCallbacks.
fn spawn_effect(f: &mut Fighter) {
    if !f.effect_state.destroy_on_state_change {
        f.effects.push(EffectRequest::Attached {
            id: MAGNET_EFFECT,
            bone: common::part(FtPart::L1stNb),
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
}

/// ftNs_SpecialLwHold_Enter (8011A650) / ftNs_SpecialAirLwHold_Enter
/// (8011A6A8): the hold row with Ft_MF_KeepGfx, then the absorb volume.
fn enter_hold(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let state = placed(f, HOLD);
    common::change(f, state, KEEP_GFX, 0.0, 1.0, a)?;
    raise_volume(f);
    Ok(())
}

/// ftNs_SpecialLwEnd_Enter (8011B4AC) / ftNs_SpecialAirLwEnd_Enter (8011B4E4).
fn enter_end(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let state = placed(f, END);
    f.change_motion_state(state, a)
}

/// The hum: x10 counts down and ft_80088478 plays it every 40 frames.
fn hum(f: &mut Fighter) {
    let m = magnet(f);
    m.hum_frames -= 1;
    if m.hum_frames <= 0 {
        m.hum_frames = HUM_FRAMES;
        common::play_loop_sound(f, HUM_SOUND);
    }
}

/// ftNs_SpecialLwStart_Anim (80119F20) / ftNs_SpecialAirLwStart_Anim
/// (8011A000): at the animation's end the model, then the hold.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    latch_release(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        spawn_effect(f);
        magnet(f).hum_frames = 0;
        enter_hold(f, p.assets)?;
    }
    Ok(None)
}

/// ftNs_SpecialLwHold_Anim (8011A2A8) / ftNs_SpecialAirLwHold_Anim
/// (8011A370): the release lag counts down; with it spent and B up, the end.
fn hold_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    latch_release(f);
    let m = magnet(f);
    if m.release_lag > 0 {
        m.release_lag -= 1;
    }
    if m.release_lag <= 0 && m.released {
        enter_end(f, p.assets)?;
    }
    hum(f);
    Ok(None)
}

/// ftNs_SpecialLwHit_Anim (8011ABF8) / ftNs_SpecialAirLwHit_Anim
/// (8011ADC8): at the animation's end ftNs_SpecialLwHold_GroundOrAir
/// (8011AB10) ends the move or holds again; a renewed hold drops the old
/// model (ftCommon_8007DB24) and spawns a new one.
fn hit_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    latch_release(f);
    let m = magnet(f);
    if m.release_lag > 0 {
        m.release_lag -= 1;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let m = magnet(f);
        if m.release_lag <= 0 && m.released {
            enter_end(f, p.assets)?;
        } else {
            enter_hold(f, p.assets)?;
            destroy_effect(f);
            spawn_effect(f);
        }
    }
    hum(f);
    Ok(None)
}

/// ftCommon_8007DB24: x2219_b0 clears and efLib_DestroyAll.
fn destroy_effect(f: &mut Fighter) {
    f.effect_state.destroy_on_state_change = false;
    f.effects.push(EffectRequest::DestroyOwned);
}

/// ftNs_SpecialLwEnd_Anim (8011B25C) / ftNs_SpecialAirLwEnd_Anim
/// (8011B2A0): ftCommon_8007DB24, then ftCommon_8007D92C.
fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        destroy_effect(f);
        if f.physics.ground_or_air == GroundOrAir::Air {
            common::fall(f, p.assets)?;
        } else {
            common::wait(f, p.assets)?;
        }
    }
    Ok(None)
}

/// ftNs_SpecialLwHold_Phys (8011A440) / ftNs_SpecialLwHit_Phys (8011AEE8):
/// ft_80084F3C, then ftColl_8007AF10 moves the absorb volume.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::ground_friction(f, p);
    f.core.refresh_absorb_position();
}

/// ftNs_SpecialAirLw{Start,Hold,Hit,End}_Phys (8011A108, 8011A474,
/// 8011AF1C, 8011B30C): the gravity delay, then ftCommon_Fall at x8C to the
/// fighter's terminal velocity; ftCommon_8007CF58's friction; the hold and
/// hit rows move the volume (ftColl_8007AF10).
fn air_physics<const VOLUME: bool>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let m = magnet(f);
    if m.gravity_delay != 0 {
        m.gravity_delay -= 1;
    } else {
        let gravity = attributes(f).fall_acceleration;
        let terminal = f.attributes.air.terminal_velocity;
        common::fall_at(f, gravity, terminal);
    }
    let air = &f.attributes.air;
    f.physics.animation_velocity.x = friction::air_drift_friction_acceleration(
        f.physics.self_velocity.x,
        air.aerial_friction,
        air.air_drift_max,
        p.assets.common.over_drift_air_friction,
    );
    f.core.finish_air_update(p.assets, p.wind);
    if VOLUME {
        f.core.refresh_absorb_position();
    }
}

/// ftNs_SpecialLw{Start,Hold,Hit,End}_Coll (8011A168, 8011A4E8, 8011AF90,
/// 8011B36C): ft_80082708; off the floor the aerial row continues at the
/// same frame (ftCommon_GroundToAirStateChange), the hold and hit rows
/// with their volume again.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("PSI Magnet collision assets");
    let ground = f.motion_state.action;
    let flags = if ground == END { END_COLL } else { COLL };
    common::ground_to_air(f, air(ground), flags, assets)?;
    if ground == HOLD || ground == HIT {
        raise_volume(f);
    }
    Ok(())
}

/// ftNs_SpecialAirLw{Start,Hold,Hit,End}_Coll (8011A1A4, 8011A524,
/// 8011AFCC, 8011B3A8): ft_80081D0C; a landing continues on the ground at
/// the same frame (ftCommon_AirToGroundStateChange), with
/// ftCommon_ClampAirDrift.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("PSI Magnet landing assets");
    let ground = ActionId(f.motion_state.action.0 - AIR_OFFSET);
    let flags = if ground == END { END_COLL } else { COLL };
    common::air_to_ground(f, ground, flags, assets)?;
    let maximum = f.attributes.air.air_drift_max;
    common::clamp_self_velocity_x(f, maximum);
    if ground == HOLD || ground == HIT {
        raise_volume(f);
    }
    Ok(())
}

/// ftNs_AbsorbThink_DecideAction (8011B0F8), ftData_OnAbsorb[FTKIND_NESS]:
/// the absorbed damage times x94, truncated, comes off the percent (not
/// below zero); Ness faces the absorbed item; the hit row starts unless it
/// is already playing at or before frame x7C.
pub fn on_absorb(f: &mut Fighter, a: &FighterAssets, absorbed: Absorbed) {
    let (multiplier, restart_frame) = {
        let m = attributes(f);
        (m.heal_multiplier, m.hit_restart_frame)
    };
    // retail 8011B130..40: the int damage as f32, fmuls, fctiwz.
    let heal = gekko_math::msl::fctiwz(absorbed.damage as f32 * multiplier) as f32;
    f.physics.percent -= heal;
    if f.physics.percent < 0.0 {
        f.physics.percent = 0.0;
    }
    f.physics.facing = absorbed.direction;
    let action = f.motion_state.action;
    let hitting = action == HIT || action == air(HIT);
    let early = f.animation.frame <= restart_frame;
    if !hitting || !early {
        let state = placed(f, HIT);
        common::change(f, state, KEEP_GFX, 0.0, 1.0, a).expect("PSI Magnet hit row assets");
        raise_volume(f);
    }
}
