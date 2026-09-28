//! Din's Fire, ftzeldaspecials.c (8013B638..8013C2F8).
//!
//! The start (343 / 346) and loop (344 / 347) spawn the fire from Zelda's
//! hand when the script raises cmd_vars[0] (once: it_802C3BAC, with
//! efSync 0x4FB at her joint 76). While the fire is hers she steers it in
//! the loop (itZeldadinfire reads her stick); releasing B after the
//! attribute's frames, or the loop's frames running out with no fire,
//! ends in 345 / 348, whose script detonates it (cmd_vars[1]).
use crate::{
    common,
    init::Zelda,
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
    physics::friction,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftZd_MS_SpecialSStart (343) .. ftZd_MS_SpecialAirSEnd (348).
pub const GROUND_START: ActionId = ActionId(343);
pub const GROUND_LOOP: ActionId = ActionId(344);
pub const GROUND_END: ActionId = ActionId(345);
pub const AIR_START: ActionId = ActionId(346);
pub const AIR_LOOP: ActionId = ActionId(347);
pub const AIR_END: ActionId = ActionId(348);
/// ftZd_SM_SpecialSStart..SpecialAirSEnd.
pub const ANIMATIONS: [i32; 6] = [297, 298, 299, 300, 301, 302];

/// ftZd_MF_SpecialSStart_Coll: ftCommon_GroundAirColl_MF with KeepGfx.
const START_FLAGS: MotionEntryFlags =
    MotionEntryFlags(common::GROUND_AIR_COLL_BASE_FLAGS.0 | MotionEntryFlags::KEEP_GFX.0);
/// ftCommon_GroundAirColl_MF alone, for the loop and the end.
const LOOP_FLAGS: MotionEntryFlags = common::GROUND_AIR_COLL_BASE_FLAGS;

/// `fp->parts[89].joint`: the hand the fire leaves from.
const HAND_JOINT: usize = 89;
/// efSync_Spawn(1275, gobj, fp->parts[76].joint): the hand's flash,
/// hsd_8039EFAC(0, 0, 0x71, jobj).
const HAND_FLASH: u16 = 0x4FB;
const FLASH_JOINT: usize = 76;
/// cmd_vars[0]: the script asks for the fire.
const SPAWN_FIRE: u32 = 1;
/// `fp->parts[0]`: the model root, whose X rotation the end resets.
const ROOT_PART: usize = 0;

/// Din's Fire's motion scratch (mv.zd.specials) and Zelda's hold on the
/// fire (u.zd.x222C).
#[derive(Clone, Copy, Debug, Default)]
pub struct DinsFire {
    /// x0 / x4: the loop's frames left while no fire is out.
    pub loop_frames: i32,
    pub second_loop_frames: i32,
    /// x8: aerial frames before gravity applies.
    pub hang_frames: i32,
    /// xC: frames before a released B ends the loop.
    pub release_lock: i32,
    /// u.zd.x222C: her fire is out.
    pub fire_out: bool,
    /// take_dmg_cb / death2_cb = ftZd_Init_801393AC, installed with the
    /// fire until the next motion change.
    pub damage_callbacks: bool,
}

fn attributes(f: &Fighter) -> &crate::attributes::DinsFireAttributes {
    &f.character.get::<Zelda>().attributes.dins_fire
}
fn scratch(f: &mut Fighter) -> &mut DinsFire {
    &mut f.character.get_mut::<Zelda>().dins_fire
}

/// The entry's and the end's reset (ftZd_SpecialS_Enter and
/// ftZd_SpecialSEnd_Anim): the counters from the attributes, no fire, no
/// callbacks.
fn reset(f: &mut Fighter) {
    let a = attributes(f).clone();
    let s = scratch(f);
    s.loop_frames = a.loop_frames;
    s.second_loop_frames = a.second_loop_frames;
    s.hang_frames = a.air_hang_frames;
    s.fire_out = false;
    s.release_lock = a.release_lock_frames;
    s.damage_callbacks = false;
}

/// ftZd_SpecialS_Enter (8013B638) / ftZd_SpecialAirS_Enter (8013B6D8).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.change_motion_state(if air { AIR_START } else { GROUND_START }, a)
        .expect("Din's Fire assets");
    f.commands.variables = [0; 4];
    if air {
        f.physics.self_velocity.y = 0.0;
    }
    reset(f);
    f.step_animation(a);
}

/// The fire on the script's request, once: from the hand, the attribute's
/// offset ahead (fmadds at 8013B7F4) and above, on the stage plane
/// (it_802C3BAC: the ray from Zelda's ECB centre); her callbacks; the
/// hand's flash.
fn spawn_fire(f: &mut Fighter) {
    if f.commands.variables[0] != SPAWN_FIRE || scratch(f).fire_out {
        return;
    }
    f.commands.variables[0] = 0;
    let mut hand = common::joint_position(f, HAND_JOINT);
    hand.z = 0.0;
    let (ahead, above) = {
        let a = attributes(f);
        (a.spawn_offset_x, a.spawn_offset_y)
    };
    hand.x = gekko_math::fma::fmadds(ahead, f.physics.facing, hand.x);
    hand.y += above;
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(ItemKind::ZeldaDinFire, c.player.id, hand, c.physics.facing);
    // it_8026BB68 -> ftLib_80086990: the ECB centre (fadds, fmuls, fadds).
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = hsd_types::Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    let s = scratch(f);
    s.fire_out = true;
    s.damage_callbacks = true;
    f.core.effects_after_items.push(EffectRequest::SyncAttached {
        id: HAND_FLASH,
        bone: FLASH_JOINT,
    });
}

/// ftZd_SpecialSStart_Anim (8013B780) / ftZd_SpecialAirSStart_Anim
/// (8013BA8C): the fire; at the end the loop, every jump spent.
pub fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    spawn_fire(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = if f.motion_state.action == AIR_START {
            AIR_LOOP
        } else {
            GROUND_LOOP
        };
        f.change_motion_state(state, p.assets)?;
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    }
    Ok(None)
}

/// ftZd_SpecialSLoop_Anim (8013B89C) / ftZd_SpecialAirSLoop_Anim
/// (8013BBA8): the fire; the counters; with no fire out the loop ends once
/// both run out; a fire no longer hers (reflected) is forgotten.
pub fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    spawn_fire(f);
    let s = scratch(f);
    if s.loop_frames != 0 {
        s.loop_frames -= 1;
    }
    if s.second_loop_frames != 0 {
        s.second_loop_frames -= 1;
    }
    if !s.fire_out {
        if s.loop_frames <= 0 && s.second_loop_frames <= 0 {
            end(f, p.assets)?;
        }
    } else if f.core.owned_article.is_none() {
        // itZeldaDinFire_GetOwner(x222C) != gobj.
        scratch(f).fire_out = false;
    }
    Ok(None)
}

/// Fighter_ChangeMotionState to the end row.
fn end(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let state = if f.motion_state.action == AIR_LOOP {
        AIR_END
    } else {
        GROUND_END
    };
    f.change_motion_state(state, assets)
}

/// ftZd_SpecialSEnd_Anim (8013BA04) / ftZd_SpecialAirSEnd_Anim (8013BD10):
/// the model root's X rotation back to zero (ftPartSetRotX); at the end the
/// reset, then Wait, or Fall (or a special fall with the attribute's
/// landing lag).
pub fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.core.set_part_rotation(ROOT_PART, Axis::X, 0.0);
    if !f.animation.frames_remaining(&f.skeleton) {
        reset(f);
        if f.motion_state.action == AIR_END {
            let lag = attributes(f).landing_lag;
            if lag == 0.0 {
                common::finish(f, true, p.assets)?;
            } else {
                f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
            }
        } else {
            common::finish(f, false, p.assets)?;
        }
    }
    Ok(None)
}

/// ftZd_SpecialSLoop_IASA (8013BDD4) / ftZd_SpecialAirSLoop_IASA
/// (8013BE58): once the lock has run out, releasing B ends the loop.
pub fn loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    let s = scratch(f);
    s.release_lock -= 1;
    let unlocked = s.release_lock <= 0;
    if unlocked {
        s.release_lock = 0;
    }
    if unlocked && !f.input.current.held.intersects(Buttons::B) {
        end(f, p.assets).expect("Din's Fire end");
    }
}

/// ftZd_SpecialSStart_Phys (8013BED8): the hang counts down, then
/// ft_80084F3C.
pub fn start_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = scratch(f);
    if s.hang_frames != 0 {
        s.hang_frames -= 1;
    }
    callbacks::physics::guard_on(f, p);
}

/// ftZd_SpecialAirS*_Phys: the hang, then the attribute's gravity with
/// the ordinary terminal speed; ftCommon_ApplyFrictionAir with the
/// aerial friction.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let hang = scratch(f).hang_frames;
    if hang != 0 {
        scratch(f).hang_frames = hang - 1;
    } else {
        let gravity = attributes(f).air_gravity;
        let terminal = f.attributes.air.terminal_velocity;
        common::fall(f, gravity, terminal);
    }
    let aerial = f.attributes.air.aerial_friction;
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, aerial);
    common::finish_air(f, &p);
}

/// ftZd_SpecialSStart_Coll / Loop / End: off the floor, the aerial
/// counterpart.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Din's Fire collision assets");
    let (state, flags) = match f.motion_state.action {
        GROUND_START => (AIR_START, START_FLAGS),
        GROUND_LOOP => (AIR_LOOP, LOOP_FLAGS),
        _ => (AIR_END, LOOP_FLAGS),
    };
    common::ground_to_air(f, state, flags, assets)
}

/// ftZd_SpecialAirS*_Coll: landing, the grounded counterpart.
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Din's Fire landing assets");
    let (state, flags) = match f.motion_state.action {
        AIR_START => (GROUND_START, START_FLAGS),
        AIR_LOOP => (GROUND_LOOP, LOOP_FLAGS),
        _ => (GROUND_END, LOOP_FLAGS),
    };
    common::air_to_ground(f, state, flags, assets)
}

/// ftZd_Init_801393AC -> ftZd_SpecialLw_8013B5EC (8013B5EC), Zelda's
/// take_dmg_cb and death2_cb while they are installed: her fire flies on
/// without her (it_802C3D44).
pub fn damage_callback(f: &mut Fighter) {
    if !scratch(f).damage_callbacks {
        return;
    }
    if scratch(f).fire_out {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::ZeldaDinFire,
            control: ItemControl::Orphan,
        });
    }
    let s = scratch(f);
    s.fire_out = false;
    s.damage_callbacks = false;
}

/// ftZd_SpecialLw_8013B5C4 (8013B5C4): the fire ended while hers.
pub fn fire_gone(f: &mut Fighter) {
    let s = scratch(f);
    s.fire_out = false;
    s.damage_callbacks = false;
}

/// What the fire reads of Zelda: ftZd_SpecialLw_8013B540 (steering in
/// the loop, 344 / 347, with her fire out) and ftZd_SpecialLw_8013B574
/// (her end's script raised cmd_vars[1]). The latter also clears
/// cmd_vars[1]; nothing else reads it in the end rows, so the clear is
/// not modelled.
pub fn item_owner(f: &Fighter, owner: &mut melee_it::ItemOwner) {
    let action = f.motion_state.action;
    let fire_out = f.character.get::<Zelda>().dins_fire.fire_out;
    owner.steering_article = fire_out && (action == GROUND_LOOP || action == AIR_LOOP);
    owner.detonating_article =
        fire_out && (action == GROUND_END || action == AIR_END) && f.commands.variables[1] == 1;
}
