//! Fireball, ftluigispecialn.c (8014267C..80142A24).
//!
//! The script raises throw_flags_b0 on the throw frame; accessory4
//! (ftLg_SpecialN_FireSpawn) then spawns the green fireball at Luigi's left
//! hand with its flash. cmd_vars[0] opens the interrupt window.
use crate::{
    common::{self, row},
    init::{Accessory, Luigi},
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// ftLg_MS_SpecialN (341) and ftLg_MS_SpecialAirN (342).
pub const GROUND: ActionId = ActionId(341);
pub const AIR: ActionId = ActionId(342);

/// efSync_Spawn(1287, gobj, hand, &facing_dir): the hand flash (model
/// 0x4650 turned to the facing, then generator 0x4650).
const FIRE_FLASH: u16 = 0x507;

/// ftLg_MF_SpecialN_Coll: SkipColAnim | UpdateCmd.
const GROUND_AIR_FLAGS: u32 = MotionEntryFlags::SKIP_COL_ANIM.0 | MotionEntryFlags::UPDATE_CMD.0;

pub const fn rows() -> [MotionRow; 2] {
    [
        row(
            GROUND,
            0,
            anim,
            ground_input,
            ground_physics,
            ground_collision,
        ),
        row(AIR, 1, anim, air_input, air_physics, air_collision),
    ]
}

/// ftLg_SpecialN_Enter (8014267C) / ftLg_SpecialAirN_Enter (801426EC).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.variables[0] = 0;
    f.commands.clear_throw_flags();
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Fireball assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    install_accessory(f);
}

/// accessory4_cb = ftLg_SpecialN_FireSpawn, after each motion change.
fn install_accessory(f: &mut Fighter) {
    f.character.get_mut::<Luigi>().accessory = Accessory::Fireball;
    f.core.arm_accessory4();
}

/// ftLg_SpecialN_FireSpawn (8014295C): on the script's throw flag, the
/// fireball (it_802C01AC) at the left hand, then its flash.
pub fn spawn_fireball(f: &mut Fighter, assets: &FighterAssets) {
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let bone = usize::from(assets.parts.joint(FtPart::L1stNb).expect("L1stNb part"));
    let c = &mut f.core;
    // lb_8000B1CC(parts[L1stNb].joint, NULL, &coords).
    let hand =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, bone, Vec3::ZERO);
    // it_802C01AC: prev_pos is the hand on the stage plane; pos is
    // it_8026BB68's ECB midpoint (ftLib_80086990, retail 800869AC..BC:
    // fadds, fmuls, fadds).
    let mut spawn = SpawnItem::ray(ItemKind::LuigiFire, c.player.id, hand, c.physics.facing);
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

/// ftLg_SpecialN_Anim (8014275C) / ftLg_SpecialAirN_Anim (80142798): Wait
/// or Fall at the animation's end.
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == AIR;
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ftLg_SpecialN_IASA (801427D4): ftCo_Wait_IASA once the script opens it.
fn ground_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.commands.variables[0] != 0 {
        callbacks::input::wait(f, p);
    }
}

/// ftLg_SpecialAirN_IASA (80142804): ftCo_Fall_IASA_Inner once the script
/// opens it.
fn air_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.commands.variables[0] != 0 {
        callbacks::input::aerial(f, p);
    }
}

/// ftLg_SpecialN_Phys (80142834): ft_80084F3C's friction.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftLg_SpecialAirN_Phys (80142854): ft_80084DB0.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::fall(f, p);
}

/// ftLg_SpecialN_Coll (80142874): off the edge, the aerial row continues
/// with the accessory reinstalled.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball collision assets");
    common::ground_to_air(f, AIR, GROUND_AIR_FLAGS, assets)?;
    install_accessory(f);
    Ok(())
}

/// ftLg_SpecialAirN_Coll (801428E8): landing continues on the ground.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball landing assets");
    common::air_to_ground(f, GROUND, GROUND_AIR_FLAGS, assets)?;
    install_accessory(f);
    Ok(())
}
