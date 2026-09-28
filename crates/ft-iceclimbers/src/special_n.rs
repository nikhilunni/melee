//! Ice Shot, ftpopospecialn.c (8011F2A4..8011F6B4), which both climbers run.
//!
//! The script sets cmd_vars[0] = 1 to make an ice block above the head
//! (it_802C1590) and cmd_vars[0] = 2 to launch it (it_802C16F8); accessory4
//! (ftPp_SpecialN_8011F500) carries both out. Leaving the ground or landing
//! first breaks a block still held.
use crate::climber::{self, attributes, vars, Accessory, Climber};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, MotionRow},
        ActionId, Fighter,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// ftPp_MS_SpecialN (341) and ftPp_MS_SpecialAirN (342).
pub const GROUND: ActionId = ActionId(341);
pub const AIR: ActionId = ActionId(342);

/// ft_PlaySFX(fp, 130021, 127, 64): the block forms
/// (retail 8011F594..9C: lis 2, subi 0x41B).
const MAKE_SOUND: u32 = 130_021;
/// ft_PlaySFX(fp, 130024, 127, 64): the block leaves.
const LAUNCH_SOUND: u32 = 130_024;
/// ft_800881D8's voice at the launch: Popo's and Nana's.
const LEADER_VOICE: u32 = 130_141;
const PARTNER_VOICE: u32 = 130_090;
/// ftPp_SpecialAirN_Enter: a second aerial Ice Shot before landing makes
/// its block 10 lower (x2250 = -10.0).
const REPEAT_AIR_DROP: f32 = -10.0;

/// The script's cmd_vars[0] commands.
const MAKE: u32 = 1;
const LAUNCH: u32 = 2;

/// ftPp_Init_MotionStateTable rows 341 and 342.
pub const fn rows<C: Climber>() -> [MotionRow; 2] {
    [
        climber::row(
            GROUND.0,
            anim::<false>,
            climber::no_input,
            callbacks::physics::guard_on,
            ground_collision::<C>,
        ),
        climber::row(
            AIR.0,
            anim::<true>,
            climber::no_input,
            callbacks::physics::air_friction,
            air_collision::<C>,
        ),
    ]
}

/// ftPp_SpecialN_Enter (8011F2A4) / ftPp_SpecialAirN_Enter (8011F318). A
/// first aerial Ice Shot lifts the climber (attribute x4) until it lands.
pub fn enter<C: Climber>(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    vars::<C>(f).ice = false;
    if airborne {
        let lift = attributes::<C>(f).air_lift;
        let v = vars::<C>(f);
        if !v.air_ice_shot_used {
            v.air_ice_shot_used = true;
            v.ice_drop = 0.0;
            f.physics.self_velocity.y = lift;
        } else {
            v.ice_drop = REPEAT_AIR_DROP;
        }
    }
    let state = if airborne { AIR } else { GROUND };
    f.change_motion_state(state, assets)
        .expect("Ice Shot assets");
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    install_accessory::<C>(f);
}

/// accessory4_cb = ftPp_SpecialN_8011F500.
fn install_accessory<C: Climber>(f: &mut Fighter) {
    vars::<C>(f).accessory = Accessory::IceShot;
    f.core.arm_accessory4();
}

/// ftPp_SpecialN_Anim (8011F3CC) / ftPp_SpecialAirN_Anim (8011F408): Wait
/// or Fall at the animation's end.
fn anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        climber::finish(f, p.assets, AIR)?;
    }
    Ok(None)
}

/// ftPp_SpecialN_Coll (8011F48C): off the floor, a held block breaks and the
/// climber falls.
fn ground_collision<C: Climber>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if climber::stays_grounded(f, &mut p) {
        return Ok(());
    }
    break_held_ice::<C>(f);
    let assets = p.assets.expect("Ice Shot collision assets");
    f.change_motion_state(melee_types::CommonMotionState::Fall.into(), assets)
}

/// ftPp_SpecialAirN_Coll (8011F4E4): on landing a held block breaks, the
/// aerial lift is spent again and LandingFallSpecial runs attribute x8's
/// lag.
fn air_collision<C: Climber>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !climber::lands(f, &mut p) {
        return Ok(());
    }
    break_held_ice::<C>(f);
    let v = vars::<C>(f);
    v.air_ice_shot_used = false;
    v.ice_drop = 0.0;
    let lag = attributes::<C>(f).ice_shot_landing_lag;
    let assets = p.assets.expect("Ice Shot landing assets");
    f.enter_special_landing(assets, false, lag)
}

/// ftPp_Init_8011F190 (8011F190): a held block breaks (it_802C17DC), and
/// the climber lets go of it (ftPp_Init_8011F16C: x222C, death2_cb and
/// take_dmg_cb cleared).
pub fn break_held_ice<C: Climber>(f: &mut Fighter) {
    if !vars::<C>(f).ice {
        return;
    }
    request_ice(f, ItemControl::Remove);
    release_ice::<C>(f);
}

/// ftPp_Init_8011F16C (8011F16C) for the block the climber holds.
pub fn release_ice<C: Climber>(f: &mut Fighter) {
    let v = vars::<C>(f);
    v.ice = false;
    v.ice_callbacks = false;
}

fn request_ice(f: &mut Fighter, control: ItemControl) {
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind: ItemKind::IceClimberIce,
        control,
    });
}

/// ftPp_SpecialN_8011F500 (8011F500): the script's two commands.
pub fn accessory<C: Climber>(f: &mut Fighter, assets: &FighterAssets) {
    let command = f.commands.variables[0];
    match command {
        MAKE => make_ice::<C>(f, assets),
        LAUNCH if vars::<C>(f).ice => launch_ice::<C>(f),
        _ => {}
    }
}

/// The block at TopN, attribute xC along the facing (retail 8011F568:
/// fmadds) and x10 plus x2250 above it (two fadds), from it_802C1590;
/// its sound; death2_cb and take_dmg_cb become ftPp_Init_8011F060.
fn make_ice<C: Climber>(f: &mut Fighter, assets: &FighterAssets) {
    let bone = usize::from(assets.parts.joint(FtPart::TopN).expect("TopN part"));
    let (reach, height) = {
        let a = attributes::<C>(f);
        (a.ice_reach, a.ice_height)
    };
    let extra_height = vars::<C>(f).ice_drop;
    let c = &mut f.core;
    // lb_8000B1CC(parts[0].joint, NULL, &pos).
    let mut position =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, bone, Vec3::ZERO);
    position.x = gekko_math::fma::fmadds(reach, c.physics.facing, position.x);
    position.y += height + extra_height;
    // it_802C1590: prev_pos on the stage plane; pos is it_8026BB68's ECB
    // midpoint (ftLib_80086990, retail 800869AC..BC: fadds, fmuls, fadds).
    let mut spawn = SpawnItem::ray(
        ItemKind::IceClimberIce,
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
    climber::play_sound(f, MAKE_SOUND);
    let v = vars::<C>(f);
    v.ice = true;
    v.ice_callbacks = true;
    f.commands.variables[0] = 0;
}

/// cmd_vars[0] == 2 with a block: it_802C16F8, the launch voice and sound,
/// and the climber lets go of it (inlineA0).
fn launch_ice<C: Climber>(f: &mut Fighter) {
    request_ice(f, ItemControl::Fire);
    f.commands.variables[0] = 0;
    climber::play_voice(
        f,
        if C::LEADER {
            LEADER_VOICE
        } else {
            PARTNER_VOICE
        },
    );
    climber::play_sound(f, LAUNCH_SOUND);
    release_ice::<C>(f);
}
