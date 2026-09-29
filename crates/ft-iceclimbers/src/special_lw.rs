//! Blizzard, ftpopospeciallw.c (80122898..80122ED8), which both climbers
//! run. The partner joins the player's fighter's Blizzard facing the other
//! way (ftCo_800B0AF4 calls the grounded entry).
//!
//! The script sets cmd_vars[0] = 1 to start blowing and cmd_vars[0] = 2 to
//! stop; while blowing, accessory4 (fn_80122D2C) spawns a puff
//! (itClimbersBlizzard_Spawn) every attribute xB8 frames. cmd_vars[3] tilts
//! the model with the floor.
use crate::climber::{self, attributes, vars, Accessory};
use crate::init::Climber;
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    part_rotation::Axis,
    state::{callbacks, AnimationPhase, CollisionPhase, MotionRow},
    ActionId, Fighter, MotionEntryFlags,
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftPp_MS_SpecialLw (357) and ftPp_MS_SpecialAirLw (358).
pub const GROUND: ActionId = ActionId(357);
pub const AIR: ActionId = ActionId(358);

/// ftPp_MF_SpecialLw_Coll (0x0C4C5282): ftCommon_GroundAirColl_MF with
/// KeepGfx and KeepSfx.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5282);

/// fp->parts[26]: the joint the puffs form in front of (retail 80122D78:
/// lwz 0x1A0 from the parts table).
const MOUTH_BONE: usize = 26;
/// fp->parts[29]: the joint the breath effect follows (retail 80122E00:
/// lwz 0x1D0).
const BREATH_BONE: usize = 29;
/// efSync_Spawn(0x4EC, gobj, joint): hsd_8039EFAC(0, 14, 0x36B7, joint).
const BREATH_EFFECT: u16 = 0x4EC;
/// The model root (ftPartSetRotX(fp, 0, ...)).
const ROOT_BONE: usize = 0;

/// ft_800881D8 / ft_80088510 at the first puff: each climber's voice and
/// breath (retail 80122E4C..80122EA4: lis 2 with subi).
const POPO_VOICE: u32 = 0x2_0000 - 0x3BE;
const POPO_BREATH: u32 = 0x2_0000 - 0x415;
const NANA_VOICE: u32 = 0x2_0000 - 0x3F1;
const NANA_BREATH: u32 = 0x2_0000 - 0x412;

/// The script's cmd_vars[0] commands.
const START: u32 = 1;
const STOP: u32 = 2;

/// fp->mv.pp.speciallw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Blizzard {
    /// +2340: frames until the next puff.
    pub countdown: i32,
    /// +2344 bit 0: the script's START came and no STOP since.
    pub blowing: bool,
}

/// ftPp_Init_MotionStateTable rows 357 and 358.
pub const ROWS: [MotionRow; 2] = [
    climber::row(
        GROUND.0,
        anim::<false>,
        climber::no_input,
        callbacks::physics::guard_on,
        ground_collision,
    ),
    climber::row(
        AIR.0,
        anim::<true>,
        climber::no_input,
        callbacks::physics::air_friction,
        air_collision,
    ),
];

/// ftPp_SpecialLw_Enter (80122904) / ftPp_SpecialAirLw_Enter (80122988).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    f.commands.variables[3] = 0;
    vars(f).blizzard = Blizzard::default();
    let state = if airborne { AIR } else { GROUND };
    f.change_motion_state(state, assets)
        .expect("Blizzard assets");
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    install_accessory(f);
}

/// accessory4_cb = fn_80122D2C.
fn install_accessory(f: &mut Fighter) {
    vars(f).accessory = Accessory::Blizzard;
    f.core.arm_accessory4();
}

/// death2_cb = take_dmg_cb = ftPp_Init_8011F060 (ftPp_set_cbs).
fn install_callbacks(f: &mut Fighter) {
    vars(f).ice_callbacks = true;
}

/// ftPp_SpecialLw_Anim (801229F4) / ftPp_SpecialAirLw_Anim (80122A74): at
/// the animation's end the breath stops (the inlined
/// ftPp_SpecialHi_80122898), then Wait or Fall.
fn anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<melee_ft::anim::WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        stop_breath(f);
        climber::finish(f, p.assets, AIR)?;
    }
    Ok(None)
}

/// ftPp_SpecialHi_80122898 (80122898), also a part of
/// ftPp_Init_8011F060: while the breath is out, its effects go
/// (efLib_DestroyAll), the callbacks are removed and the model stands
/// upright again.
pub fn stop_breath(f: &mut Fighter) {
    if !vars(f).breath {
        return;
    }
    f.effects.push(EffectRequest::DestroyOwned);
    let v = vars(f);
    v.breath = false;
    v.ice_callbacks = false;
    f.core.set_part_rotation(ROOT_BONE, Axis::X, 0.0);
}

/// The inlined ftPp_SpecialLw_Coll_inline: with cmd_vars[3] the model
/// leans with the floor (atan2f of its normal, fmuls by the facing),
/// otherwise it stands upright.
fn lean_with_floor(f: &mut Fighter) {
    let angle = if f.commands.variables[3] != 0 {
        let normal = f.collision.data.floor.normal;
        f.physics.facing * melee_lb::trigf::atan2f(normal.x, normal.y)
    } else {
        0.0
    };
    f.core.set_part_rotation(ROOT_BONE, Axis::X, angle);
}

/// ftPp_SpecialLw_Coll (80122C18): off the floor, the model stands upright
/// and the aerial row continues (ftCommon_8007D5D4, the current frame,
/// both callbacks, the accessory, ftCommon_ClampAirDrift); on it, the
/// lean.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if climber::stays_grounded(f, &mut p) {
        lean_with_floor(f);
        return Ok(());
    }
    f.core.set_part_rotation(ROOT_BONE, Axis::X, 0.0);
    let assets = p.assets.expect("Blizzard collision assets");
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    install_callbacks(f);
    install_accessory(f);
    // ftCommon_ClampAirDrift (8007D468).
    let maximum = f.attributes.air.air_drift_max;
    f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-maximum, maximum);
    Ok(())
}

/// ftPp_SpecialAirLw_Coll (80122D04) -> ft_80082C74(fn_80122B54): on
/// landing, ftCommon_8007D7FC, the grounded row at the current frame, both
/// callbacks, the lean and the accessory.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !climber::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Blizzard landing assets");
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(GROUND, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    install_callbacks(f);
    lean_with_floor(f);
    install_accessory(f);
    Ok(())
}

/// fn_80122D2C (80122D2C): while blowing, a puff every attribute xB8
/// frames; then the script's command.
pub fn accessory(f: &mut Fighter) {
    if vars(f).blizzard.blowing {
        if vars(f).blizzard.countdown == 0 {
            spawn_puff(f);
            let interval = gekko_math::msl::fctiwz(attributes(f).blizzard_interval);
            vars(f).blizzard.countdown = interval;
        }
        vars(f).blizzard.countdown -= 1;
    }
    match f.commands.variables[0] {
        START => start_blowing(f),
        STOP => {
            vars(f).blizzard.blowing = false;
            f.commands.variables[0] = 0;
        }
        _ => {}
    }
}

/// A puff at the mouth joint, attribute xBC along the facing (retail
/// 80122D94: fmadds) and xC0 above it (fadds): itClimbersBlizzard_Spawn,
/// whose pos is it_8026BB68's ECB midpoint (ftLib_80086990: fadds, fmuls,
/// fadds) and prev_pos the puff's point on the stage plane.
fn spawn_puff(f: &mut Fighter) {
    let (reach, height) = {
        let a = attributes(f);
        (a.blizzard_reach, a.blizzard_height)
    };
    let c = &mut f.core;
    // lb_8000B1CC(parts[26].joint, NULL, &pos).
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        MOUTH_BONE,
        Vec3::ZERO,
    );
    position.x = gekko_math::fma::fmadds(reach, c.physics.facing, position.x);
    position.y += height;
    let mut spawn = SpawnItem::ray(
        ItemKind::IceClimberBlizzard,
        c.player.id,
        position,
        c.physics.facing,
    );
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// cmd_vars[0] == 1: the breath effect on joint 29, blowing, the breath
/// flag (x2230_b0), both callbacks, and the climber's voice and breath
/// sound.
fn start_blowing(f: &mut Fighter) {
    f.effects.push(EffectRequest::SyncAttached {
        id: BREATH_EFFECT,
        bone: BREATH_BONE,
    });
    let v = vars(f);
    v.blizzard.blowing = true;
    v.breath = true;
    install_callbacks(f);
    f.commands.variables[0] = 0;
    let (voice, breath) = match climber::climber(f) {
        Climber::Popo => (POPO_VOICE, POPO_BREATH),
        Climber::Nana => (NANA_VOICE, NANA_BREATH),
    };
    climber::play_voice(f, voice);
    play_effect_sound(f, breath);
}

/// ft_80088510(fp, id, 127, 64): the fighter's effect channel.
fn play_effect_sound(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Effect,
        id,
        volume: 127,
        pan: 64,
    });
}
