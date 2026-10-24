//! Luigi's Fireball, ftluigispecialn.c (8014267C..80142A24), and Kirby's
//! copy of it (ftkirbyspecialluigi.c, 800F9614..800F99BC): the same states
//! over either user's rows, generic over [`GreenFireball`].
//!
//! The script raises throw_flags_b0 on the throw frame; accessory4
//! (ftLg_SpecialN_FireSpawn, Kirby's fn_800F98F4) then spawns the green
//! fireball at the left hand with its flash. cmd_vars[0] opens the
//! interrupt window.
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// ftLg_MF_SpecialN_Coll: SkipColAnim | UpdateCmd.
const GROUND_AIR_FLAGS: MotionEntryFlags =
    MotionEntryFlags(MotionEntryFlags::SKIP_COL_ANIM.0 | MotionEntryFlags::UPDATE_CMD.0);

/// What the Fireball's states ask of their user.
pub trait GreenFireball {
    /// The grounded and aerial rows.
    const GROUND: ActionId;
    const AIR: ActionId;
    /// The grounded row's animation; the aerial row plays the next.
    const GROUND_ANIMATION: i32;
    /// The fireball's item kind.
    const ITEM: ItemKind;
    /// The hand flash (efSync_Spawn's id).
    const FLASH: u16;
    /// The part whose joint the fireball leaves from.
    const HAND: FtPart;
    /// accessory4_cb = the fireball's spawner.
    fn install_accessory(f: &mut Fighter);
}

/// The grounded and aerial rows: the entries' callbacks.
pub const fn rows<C: GreenFireball>() -> [MotionRow; 2] {
    [
        row(
            C::GROUND,
            C::GROUND_ANIMATION,
            ground_anim,
            ground_input,
            ground_physics,
            ground_collision::<C>,
        ),
        row(
            C::AIR,
            C::GROUND_ANIMATION + 1,
            air_anim,
            air_input,
            air_physics,
            air_collision::<C>,
        ),
    ]
}

const fn row(
    action: ActionId,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// ftLg_SpecialN_Enter (8014267C) / ftLg_SpecialAirN_Enter (801426EC);
/// Kirby's ftKb_SpecialNLg_800F9614 / 800F9684.
pub fn enter<C: GreenFireball>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.variables[0] = 0;
    f.commands.clear_throw_flags();
    f.change_motion_state(if air { C::AIR } else { C::GROUND }, a)
        .expect("Fireball assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    C::install_accessory(f);
}

/// ftLg_SpecialN_FireSpawn (8014295C) / fn_800F98F4 (800F98F4): on the
/// script's throw flag, the fireball (it_802C01AC) at the left hand, then
/// its flash.
pub fn spawn<C: GreenFireball>(f: &mut Fighter, assets: &FighterAssets) {
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let bone = usize::from(assets.parts.joint(C::HAND).expect("hand part"));
    let c = &mut f.core;
    // lb_8000B1CC(parts[hand].joint, NULL, &coords).
    let hand =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, bone, Vec3::ZERO);
    // it_802C01AC: prev_pos is the hand on the stage plane; pos is
    // it_8026BB68's ECB midpoint (ftLib_80086990, retail 800869AC..BC:
    // fadds, fmuls, fadds).
    let mut spawn = SpawnItem::ray(C::ITEM, c.player.id, hand, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    c.effects_after_items.push(EffectRequest::SyncAttached {
        id: C::FLASH,
        bone,
    });
}

/// ft_8008A2BC on the ground (Wait), ftCo_Fall_Enter in the air.
fn finish(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let state = if air {
        CommonMotionState::Fall
    } else {
        CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
}

/// ftLg_SpecialN_Anim (8014275C) / ftKb_LgSpecialN_Anim (800F96F4): Wait
/// at the animation's end.
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftLg_SpecialAirN_Anim (80142798) / ftKb_LgSpecialAirN_Anim (800F9730):
/// Fall at the animation's end.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, true, p.assets)?;
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

/// ftLg_SpecialN_Coll (80142874): off the edge (ft_80082708), the aerial
/// row continues (ftCommon_GroundToAirStateChange) with the accessory
/// reinstalled.
fn ground_collision<C: GreenFireball>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    let supported = ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported;
    if supported {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball collision assets");
    // ftCommon_8007D5D4, then the aerial row at the current frame.
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(C::AIR, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    C::install_accessory(f);
    Ok(())
}

/// ftLg_SpecialAirN_Coll (801428E8): landing (ft_80081D0C) continues on
/// the ground (ftCommon_AirToGroundStateChange).
fn air_collision<C: GreenFireball>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    );
    if !landed {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball landing assets");
    // ftCommon_8007D7FC, then the grounded row at the current frame.
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(C::GROUND, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    C::install_accessory(f);
    Ok(())
}
