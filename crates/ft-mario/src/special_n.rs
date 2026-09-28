//! Fireball, ftmariospecialn.c (800E0DA8..800E1248).
//!
//! The script raises throw_flags_b0 on the throw frame; accessory4
//! (ftMr_SpecialN_ItemFireSpawn) then spawns the fireball at Mario's left
//! hand with its flash. cmd_vars[0] opens the interrupt window.
use crate::{
    common,
    init::{Accessory, Mario},
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// ftMr_MS_SpecialN (343) and ftMr_MS_SpecialAirN (344).
pub const GROUND: ActionId = ActionId(343);
pub const AIR: ActionId = ActionId(344);

/// efSync_Spawn(1146, gobj, hand, &facing_dir): efAlt's hand flash.
const FIRE_FLASH: u16 = 0x47A;

/// ftMr_MF_SpecialN_Coll: SkipColAnim | UpdateCmd.
const GROUND_AIR_FLAGS: MotionEntryFlags =
    MotionEntryFlags(MotionEntryFlags::SKIP_COL_ANIM.0 | MotionEntryFlags::UPDATE_CMD.0);

/// ftMr_SpecialN_Enter (800E0DA8) / ftMr_SpecialAirN_Enter (800E1040).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.variables[0] = 0;
    f.commands.clear_throw_flags();
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Fireball assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    install_accessory(f);
}

/// accessory4_cb = ftMr_SpecialN_ItemFireSpawn, after each motion change.
fn install_accessory(f: &mut Fighter) {
    f.character.get_mut::<Mario>().accessory = Accessory::Fireball;
    f.core.arm_accessory4();
}

/// ftMr_SpecialN_ItemFireSpawn (800E0EE0): on the script's throw flag, the
/// fireball (it_8029B6F8) at the left hand, then its flash.
pub fn spawn_fireball(f: &mut Fighter, assets: &FighterAssets) {
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let bone = usize::from(assets.parts.joint(FtPart::L1stNb).expect("L1stNb part"));
    let c = &mut f.core;
    // lb_8000B1CC(parts[L1stNb].joint, NULL, &coords).
    let hand =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, bone, Vec3::ZERO);
    // it_8029B6F8: prev_pos is the hand on the stage plane; pos is
    // it_8026BB68's ECB midpoint (ftLib_80086990, retail 800869AC..BC:
    // fadds, fmuls, fadds).
    let mut spawn = SpawnItem::ray(ItemKind::MarioFire, c.player.id, hand, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    c.effects_after_items.push(EffectRequest::SyncAttached {
        id: FIRE_FLASH,
        bone,
    });
}

/// ftMr_SpecialN_Anim (800E0E18) / ftMr_SpecialAirN_Anim (800E10B0): Wait
/// or Fall at the animation's end.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == AIR;
        common::finish(f, p.assets, air)?;
    }
    Ok(None)
}

/// ftMr_SpecialN_IASA (800E0E54): ftCo_Wait_IASA once the script opens it.
pub fn ground_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.commands.variables[0] != 0 {
        callbacks::input::wait(f, p);
    }
}

/// ftMr_SpecialAirN_IASA (800E10EC): ftCo_Fall_IASA_Inner once the script
/// opens it.
pub fn air_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.commands.variables[0] != 0 {
        callbacks::input::aerial(f, p);
    }
}

/// ftMr_SpecialN_Phys (800E0E84): ft_80084F3C's friction.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftMr_SpecialAirN_Phys (800E111C): ft_80084DB0.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::fall(f, p);
}

/// ftMr_SpecialN_Coll (800E0EA4): off the edge, ftMr_SpecialN_GroundToAir
/// (800E1178) continues in the air.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball collision assets");
    common::ground_to_air(f, AIR, assets, GROUND_AIR_FLAGS)?;
    install_accessory(f);
    Ok(())
}

/// ftMr_SpecialAirN_Coll (800E113C): landing continues on the ground
/// (ftMr_SpecialAirN_AirToGround, 800E11E0).
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball landing assets");
    common::air_to_ground(f, GROUND, assets, GROUND_AIR_FLAGS)?;
    install_accessory(f);
    Ok(())
}
