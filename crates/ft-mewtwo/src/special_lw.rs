//! Disable, ftmewtwospeciallw.c (80146198..8014665C).
//!
//! One row on the ground (359) and one in the air (360). On the script's
//! flag (cmd_vars[0]) the accessory spawns the projectile ahead of the left
//! hand; Mewtwo keeps it (u.mt.x222C) until it ends, and removes it when
//! the animation ends or a hit or death interrupts the special.
use crate::{
    common,
    init::{Accessory, Mewtwo},
};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftMt_MS_SpecialLw (359) / ftMt_MS_SpecialAirLw (360).
pub const GROUND: ActionId = ActionId(359);
pub const AIR: ActionId = ActionId(360);

/// FTMEWTWO_SPECIALLW_COLL_FLAG: ftCommon_GroundAirColl_MF with KeepGfx.
const TRANSITION_FLAGS: MotionEntryFlags =
    MotionEntryFlags(common::GROUND_AIR_COLL_BASE_FLAGS.0 | 1 << 1);

/// `fp->parts[FtPart_L3rdNb]`: the parts array indexed by the part enum.
const HAND_JOINT: usize = 27;

pub const fn rows() -> [MotionRow; 2] {
    [
        common::row(
            GROUND,
            312,
            ground_anim,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        common::row(
            AIR,
            313,
            air_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
    ]
}

fn install_accessory(f: &mut Fighter) {
    f.character.get_mut::<Mewtwo>().accessory = Accessory::Disable;
    f.core.arm_accessory4();
}

/// ftMewtwo_SpecialLw_SetCall: while the projectile is Mewtwo's, death2_cb
/// and take_dmg_cb are ftMt_Init_OnDeath2 / ftMt_Init_OnTakeDamage until
/// the next motion change.
fn install_callbacks(f: &mut Fighter) {
    if f.character.get::<Mewtwo>().disable_article {
        f.character.get_mut::<Mewtwo>().damage_callbacks = true;
    }
}

/// ftMt_SpecialLw_Enter (801461F0) / ftMt_SpecialAirLw_Enter (80146264):
/// the flags cleared, no projectile, the aerial one without vertical speed.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    // fp->throw_flags = 0 is the script's throw latch.
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    f.character.get_mut::<Mewtwo>().disable_article = false;
    if air {
        f.physics.self_velocity.y = 0.0;
    }
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Disable assets");
    f.step_animation(a);
    install_accessory(f);
}

/// ftMt_SpecialLw_RemoveDisable (801461A8): the projectile Mewtwo still
/// keeps ends at once (itMewtwoDisable_Logic67_Destroy).
pub fn remove_projectile(f: &mut Fighter) {
    if std::mem::take(&mut f.character.get_mut::<Mewtwo>().disable_article) {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: ItemKind::MewtwoDisable,
            control: ItemControl::Remove,
        });
    }
}

/// ftMt_SpecialLw_CreateDisable (80146594), accessory4 for the whole
/// special: on cmd_vars[0], the projectile at the left hand's third finger,
/// attribute x80 along the facing (retail 801465EC: fmadds) and x84 above
/// it (fadds). itMewtwoDisable_Logic67_SpawnMewtwoDisable's pos is
/// it_8026BB68's ECB midpoint (ftLib_80086990: fadds, fmuls, fadds) and
/// prev_pos that point.
pub fn create_projectile(f: &mut Fighter) {
    if f.commands.variables[0] == 0 {
        return;
    }
    let (reach, height) = {
        let d = &f.character.get::<Mewtwo>().attributes.disable;
        (d.offset_x, d.offset_y)
    };
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        HAND_JOINT,
        Vec3::ZERO,
    );
    position.x = gekko_math::fma::fmadds(reach, c.physics.facing, position.x);
    position.y += height;
    let mut spawn = SpawnItem::ray(
        ItemKind::MewtwoDisable,
        c.player.id,
        position,
        c.physics.facing,
    );
    // prev_pos keeps the joint's depth.
    spawn.previous_position = position;
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    f.character.get_mut::<Mewtwo>().disable_article = true;
    install_callbacks(f);
    f.commands.variables[0] = 0;
}

/// ftMt_SpecialLw_Anim (801462DC): at the end the projectile goes; Wait.
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        remove_projectile(f);
        common::seal_graphics(f, p.assets, p.rng);
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftMt_SpecialAirLw_Anim (80146338): at the end the projectile goes; Fall.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        remove_projectile(f);
        common::seal_graphics(f, p.assets, p.rng);
        common::enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftMt_SpecialAirLw_Phys (801463BC): the attribute's gravity
/// (ftCommon_Fall), then the fighter's aerial friction
/// (ftCommon_ApplyFrictionAir).
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, terminal) = {
        let d = &f.character.get::<Mewtwo>().attributes.disable;
        (d.gravity, d.terminal_velocity)
    };
    common::fall(f, gravity, terminal);
    common::aerial_friction(f);
    common::finish_air(f, &p);
}

/// ftMt_SpecialLw_Coll (80146544) -> ft_8008403C: off the floor,
/// ftMt_SpecialLw_GroundToAir (80146410): no vertical speed, the aerial row
/// at the current frame, the accessory and callbacks again, and the drift
/// clamped (ftCommon_ClampAirDrift).
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Disable collision assets");
    f.leave_ground();
    f.physics.self_velocity.y = 0.0;
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(AIR, assets, TRANSITION_FLAGS, frame, 1.0)?;
    install_accessory(f);
    install_callbacks(f);
    let drift = f.attributes.air.air_drift_max;
    common::clamp_self_velocity_x(f, drift);
    Ok(())
}

/// ftMt_SpecialAirLw_Coll (8014656C) -> ft_80082C74: landing,
/// ftMt_SpecialAirLw_AirToGround (801464B0).
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Disable landing assets");
    common::air_to_ground(f, GROUND, TRANSITION_FLAGS, assets)?;
    install_accessory(f);
    install_callbacks(f);
    Ok(())
}
