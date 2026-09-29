//! Charge Shot, ftsamusspecialn.c (801291A8..8012A068).
//!
//! The start (343 / 347) creates the shot in Samus's right hand on the
//! script's cmd_vars[0]. On the ground an unfinished charge continues in
//! the hold (344), which gains a level every x20 frames up to x18 (the
//! level Samus keeps between charges, u.ss.x2230); B fires (346), a shield
//! press cancels (345) and a roll leaves. In the air, or at full charge,
//! the start fires at once (346 / 348): on the script's cmd_vars[1] the
//! shot leaves from ThrowN with the kept charge and the level resets.
use crate::{
    common::{self, change, GROUND_AIR},
    init::{install_damage_callbacks, Samus},
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_it::{Aim, ItemControl, ItemRequest, Launch, SpawnItem};
use melee_types::ItemKind;

/// ftSs_MS_SpecialNStart (343) .. SpecialAirN (348).
pub const START: ActionId = ActionId(343);
pub const HOLD: ActionId = ActionId(344);
pub const CANCEL: ActionId = ActionId(345);
pub const FIRE: ActionId = ActionId(346);
pub const AIR_START: ActionId = ActionId(347);
pub const AIR_FIRE: ActionId = ActionId(348);

/// `fp->parts[FtPart_RHandNb]` / `[FtPart_ThrowN]`: raw parts indices.
const HAND_PART: usize = 50;
const MUZZLE_PART: usize = 51;
/// 801292E4: the shot forms 4 units along the hand joint's Z.
const HAND_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.0);
/// ftCo_800BFFD0(fp, 53, 0): the full-charge colour animation.
const FULL_CHARGE_COLOR: u8 = 53;
/// efSync_Spawn(1158, gobj, &pos, &facing): efAlt 0x486, the muzzle flash
/// model 0x7D1 turned to the facing.
const MUZZLE_FLASH: u16 = 0x486;

/// mv.ss.unk3 (ftSamus/types.h).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChargeShot {
    /// x0: the charge began in the air (or went airborne): fire at once.
    pub airborne: bool,
    /// x4: frames toward the next level.
    pub frames: i32,
}

pub const fn rows() -> [MotionRow; 6] {
    [
        common::row(
            START,
            start_anim,
            common::no_input,
            common::ground_friction,
            start_collision,
        ),
        common::row(
            HOLD,
            hold_anim,
            hold_input,
            common::ground_friction,
            hold_collision,
        ),
        common::row(
            CANCEL,
            cancel_anim,
            common::no_input,
            common::ground_friction,
            cancel_collision,
        ),
        common::row(
            FIRE,
            fire_anim,
            common::no_input,
            common::ground_friction,
            fire_collision,
        ),
        common::row(
            AIR_START,
            air_start_anim,
            common::no_input,
            air_physics,
            air_start_collision,
        ),
        common::row(
            AIR_FIRE,
            air_fire_anim,
            common::no_input,
            air_physics,
            air_fire_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::ChargeShotAttributes {
    &f.character.get::<Samus>().attributes.charge_shot
}
fn scratch(f: &mut Fighter) -> &mut ChargeShot {
    &mut f.character.get_mut::<Samus>().charge_shot
}

/// ftSs_SpecialN_Enter (8012954C) / ftSs_SpecialAirN_Enter (801295F0).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    change(
        f,
        if air { AIR_START } else { START },
        MotionEntryFlags(0),
        0.0,
        1.0,
        a,
    )
    .expect("Charge Shot assets");
    f.commands.variables = [0; 4];
    if !air {
        // ftCommon_8007D7FC.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    install_damage_callbacks(f);
    *scratch(f) = ChargeShot {
        airborne: air,
        frames: 0,
    };
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// Samus's charge level (u.ss.x2230), and ftSs_Init_UnkMotionStates4's
/// glow (colour 53 whenever the secondary colour slot empties) while it is
/// the full level x18.
pub fn set_charge_level(f: &mut Fighter, level: i32) {
    f.character.get_mut::<Samus>().charge_level = level;
    let full = level as f32 == attributes(f).max_charge;
    f.core.combat.secondary_color_fallback = full.then_some(FULL_CHARGE_COLOR);
}

/// The shot's full level, fctiwz of the float x18.
pub fn full_charge(f: &Fighter) -> i32 {
    gekko_math::msl::fctiwz(attributes(f).max_charge)
}

/// ftSs_SpecialN_801292E4 (801292E4): on the script's cmd_vars[0], with no
/// shot yet, the shot forms at the right hand (it_802B55C8, then
/// Item_8026AB54 into the hand) and the callbacks go in.
fn form_shot(f: &mut Fighter) {
    if f.commands.variables[0] != 1 || f.character.get::<Samus>().charge_article {
        return;
    }
    f.commands.variables[0] = 0;
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        HAND_PART,
        HAND_OFFSET,
    );
    position.z = 0.0;
    // it_802B55C8: prev_pos on the stage plane, pos it_8026BB68's ECB
    // midpoint (ftLib_80086990: fadds, fmuls, fadds), x44_flag.b0.
    let mut spawn = SpawnItem::ray(
        ItemKind::SamusCharge,
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
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: HAND_PART as u8,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    f.character.get_mut::<Samus>().charge_article = true;
    install_damage_callbacks(f);
}

/// ftSs_SpecialN_801293BC (801293BC): on the script's cmd_vars[1], with a
/// shot, it leaves ThrowN (it_802B56E4) facing right (0) or left (M_PI)
/// with the kept charge; an aerial shot pushes Samus back by x1C per
/// level; the charge resets, the shot is let go (ftSs_SpecialN_801291F0)
/// and the muzzle flashes.
fn fire_shot(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[1] != 1 || !f.character.get::<Samus>().charge_article {
        return;
    }
    f.commands.variables[1] = 2;
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        MUZZLE_PART,
        Vec3::ZERO,
    );
    position.z = 0.0;
    let angle = if c.physics.facing == 1.0 {
        0.0
    } else {
        std::f64::consts::PI as f32
    };
    let level = f.character.get::<Samus>().charge_level as u32;
    let full = attributes(f).max_charge;
    let holder = f.core.item_holder(HAND_PART as u8, assets);
    // ftLib_80086630: the hand joint's world matrix, set up on demand.
    let hand = *holder.skeleton.get_mtx(holder.part);
    let launch = Launch {
        velocity: Vec3::ZERO,
        offset: Vec3::ZERO,
        spin_degrees: 0.0,
        hand,
        center: holder.center,
        attack: holder.attack,
        attack_stale: holder.attack_stale,
        aim: Some(Aim {
            position,
            angle,
            charge: level as f32,
            full_charge: full,
            facing: f.physics.facing,
        }),
        angle: 0.0,
        long_lifetime: false,
        shot: None,
    };
    f.core.item_requests.push(ItemRequest::Launch {
        owner: f.player.id,
        kind: ItemKind::SamusCharge,
        launch,
    });
    if f.motion_state.action == AIR_FIRE || f.physics.ground_or_air == melee_types::GroundOrAir::Air
    {
        // ftSamus_801293BC_inner: separate fmuls, level first.
        let recoil = attributes(f).recoil_per_level;
        f.physics.self_velocity.x = f.physics.facing * (recoil * level as f32);
    }
    set_charge_level(f, 0);
    let_go(f);
    f.core.effects.push(EffectRequest::PositionalModel {
        id: MUZZLE_FLASH,
        position,
    });
}

/// ftSs_SpecialN_801291F0 (801291F0): the shot is no longer Samus's
/// (x222C), and its charge effects go (ftSamus_destroyAllEF).
pub fn let_go(f: &mut Fighter) {
    f.character.get_mut::<Samus>().charge_article = false;
    destroy_charge_effects(f);
}

/// ftSamus_destroyAllEF: efLib_DestroyAll while x2234 is set.
fn destroy_charge_effects(f: &mut Fighter) {
    if std::mem::take(&mut f.character.get_mut::<Samus>().charge_effects) {
        f.core.effects.push(EffectRequest::DestroyOwned);
    }
}

/// ftSamus_UnkAndDestroyAllEF: the shot in hand is destroyed
/// (it_802B5974), then the charge effects.
pub fn drop_shot(f: &mut Fighter) {
    if std::mem::take(&mut f.character.get_mut::<Samus>().charge_article) {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: ItemKind::SamusCharge,
            control: ItemControl::Remove,
        });
    }
    destroy_charge_effects(f);
}

/// ftSs_SpecialNStart_Anim (80129684): the shot forms; at the end an
/// aerial or full charge fires, otherwise the hold (x2234 cleared).
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    form_shot(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        let full = f.character.get::<Samus>().charge_level as f32 == attributes(f).max_charge;
        if scratch(f).airborne || full {
            common::seal_graphics(f, p.assets, p.rng);
            change(f, FIRE, MotionEntryFlags(0), 0.0, 1.0, p.assets)?;
        } else {
            common::seal_graphics(f, p.assets, p.rng);
            change(f, HOLD, MotionEntryFlags(0), 0.0, 1.0, p.assets)?;
            f.character.get_mut::<Samus>().charge_effects = false;
        }
        install_damage_callbacks(f);
    }
    Ok(None)
}

/// ftSs_SpecialNHold_Anim (80129774): a level every x20 frames; the full
/// level flashes (colour 53) and ends the hold in the cancel, the shot
/// kept as a charge.
fn hold_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    // cmd_vars[2]'s charge sound is presentation.
    f.commands.variables[2] = 0;
    let per_level = attributes(f).frames_per_level;
    let s = scratch(f);
    s.frames += 1;
    if s.frames > per_level {
        s.frames = 0;
        let full = attributes(f).max_charge;
        let level = f.character.get::<Samus>().charge_level + 1;
        set_charge_level(f, level);
        if level as f32 >= full {
            f.core
                .install_color_overlay_now(FULL_CHARGE_COLOR, p.assets);
            set_charge_level(f, gekko_math::msl::fctiwz(full));
            common::seal_graphics(f, p.assets, p.rng);
            change(f, CANCEL, MotionEntryFlags(0), 0.0, 1.0, p.assets)?;
            drop_shot(f);
            install_damage_callbacks(f);
        }
    }
    Ok(None)
}

/// ftSs_SpecialNCancel_Anim (80129940): the shot goes; Wait at the end.
fn cancel_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    drop_shot(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialN_Anim (801299D0): the shot fires; Wait at the end.
fn fire_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    fire_shot(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialAirNStart_Anim (80129A14): the shot forms; the aerial fire
/// follows at the end.
fn air_start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    form_shot(f);
    scratch(f).airborne = true;
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        change(f, AIR_FIRE, MotionEntryFlags(0), 0.0, 1.0, p.assets)?;
        install_damage_callbacks(f);
    }
    Ok(None)
}

/// ftSs_SpecialAirN_Anim (80129A98): the shot fires; Fall at the end, or
/// special fall with x24 of landing lag (ftCo_80096900(1, 0, 1, 1, x24)).
fn air_fire_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    fire_shot(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let lag = attributes(f).landing_lag;
        if lag == 0.0 {
            common::seal_graphics(f, p.assets, p.rng);
            common::fall(f, p.assets)?;
        } else {
            common::seal_graphics(f, p.assets, p.rng);
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// ftSs_SpecialNHold_IASA (80129B1C): a roll (ftCo_8009917C) drops the
/// shot; otherwise B fires and a shield press cancels.
fn hold_input(f: &mut Fighter, p: InputPhase<'_>) {
    if let Some(roll) = f.core.roll_input(p.assets) {
        f.enter_escape(p.assets, roll)
            .expect("roll out of the charge");
        drop_shot(f);
        return;
    }
    let pressed = f.input.pressed;
    if pressed.intersects(Buttons::B) {
        change(f, FIRE, MotionEntryFlags(0), 0.0, 1.0, p.assets).expect("Charge Shot fire");
        install_damage_callbacks(f);
        return;
    }
    if pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
        change(f, CANCEL, MotionEntryFlags(0), 0.0, 1.0, p.assets).expect("Charge Shot cancel");
        drop_shot(f);
        install_damage_callbacks(f);
    }
}

/// ft_80084EEC: gravity and air friction.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    melee_ft::fighter::state::callbacks::physics::air_friction(f, p);
}

/// The grounded rows' ft_80082708: off the floor, the aerial counterpart
/// (ftCommon_GroundToAirStateChange) and the callbacks.
fn leave_floor(f: &mut Fighter, p: &mut CollisionPhase<'_>, state: ActionId) -> Result<bool> {
    if common::stays_grounded(f, p) {
        return Ok(false);
    }
    let assets = p.assets.expect("Charge Shot collision assets");
    common::ground_to_air(f, state, GROUND_AIR, assets)?;
    install_damage_callbacks(f);
    Ok(true)
}

/// ftSs_SpecialNStart_Coll (80129D48).
fn start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_START).map(|_| ())
}

/// ftSs_SpecialNHold_Coll (80129DC8): off the floor the shot fires at once
/// in the air (cmd_vars[1] = 1).
fn hold_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if leave_floor(f, &mut p, AIR_FIRE)? {
        f.commands.variables[1] = 1;
    }
    Ok(())
}

/// ftSs_SpecialNCancel_Coll (80129E68): off the floor, the aerial fire.
fn cancel_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_FIRE).map(|_| ())
}

/// ftSs_SpecialN_Coll (80129EE8).
fn fire_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, &mut p, AIR_FIRE).map(|_| ())
}

/// The aerial rows' ft_80081D0C: landing continues on the ground
/// (ftCommon_AirToGroundStateChange).
fn land_as(f: &mut Fighter, p: &mut CollisionPhase<'_>, state: ActionId) -> Result<()> {
    if !common::lands(f, p) {
        return Ok(());
    }
    let assets = p.assets.expect("Charge Shot landing assets");
    common::air_to_ground(f, state, GROUND_AIR, assets)?;
    install_damage_callbacks(f);
    Ok(())
}

/// ftSs_SpecialAirNStart_Coll (80129F68).
fn air_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as(f, &mut p, START)
}

/// ftSs_SpecialAirN_Coll (80129FE8).
fn air_fire_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as(f, &mut p, FIRE)
}
