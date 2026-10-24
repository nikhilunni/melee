//! Fireball and Megavitamin, ftmariospecialn.c (800E0DA8..800E1248), and
//! Kirby's copies of them (ftkirbyspecialmario.c, 800F9110..800F9614).
//!
//! The script raises throw_flags_b0 on the throw frame; accessory4
//! (ftMr_SpecialN_ItemFireSpawn, Kirby's fn_800F9260) then spawns the
//! kind's projectile at the left hand. cmd_vars[0] opens the interrupt
//! window. The states are generic over [`Fireball`]: every `MarioFamily`
//! gets it by a blanket impl, Kirby's copy implements it over his rows.
use crate::{common, specials, Accessory, MarioFamily};
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

/// ftMr_MF_SpecialN_Coll: SkipColAnim | UpdateCmd (0x5000; Kirby's
/// ftKb_SpecialNLg_800F951C / 800F9598 pass the same word).
const GROUND_AIR_FLAGS: MotionEntryFlags =
    MotionEntryFlags(MotionEntryFlags::SKIP_COL_ANIM.0 | MotionEntryFlags::UPDATE_CMD.0);

/// What the Fireball's states ask of their user.
pub trait Fireball {
    /// The grounded and aerial rows.
    fn rows(f: &Fighter) -> [ActionId; 2];
    /// The part whose joint the projectile leaves from.
    const HAND: FtPart;
    /// accessory4_cb = the projectile's spawner.
    fn install_accessory(f: &mut Fighter);
    /// Whether the spawner reads the throw flag at all (Kirby's
    /// fn_800F9260 returns first without a copied kind).
    fn armed(_f: &Fighter) -> bool {
        true
    }
    /// The projectile at `hand` (the joint of part bone `bone`).
    fn throw(
        f: &mut Fighter,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
        hand: Vec3,
        bone: usize,
    );
}

impl<C: MarioFamily> Fireball for C {
    fn rows(_: &Fighter) -> [ActionId; 2] {
        [GROUND, AIR]
    }
    /// ftMr_SpecialN_ItemFireSpawn: lb_8000B1CC(parts[L1stNb].joint).
    const HAND: FtPart = FtPart::L1stNb;
    /// accessory4_cb = ftMr_SpecialN_ItemFireSpawn.
    fn install_accessory(f: &mut Fighter) {
        specials::<C>(f).accessory = Accessory::NeutralProjectile;
        f.core.arm_accessory4();
    }
    fn throw(
        f: &mut Fighter,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
        hand: Vec3,
        bone: usize,
    ) {
        C::NEUTRAL_PROJECTILE(f, assets, rng, hand, bone);
    }
}

/// ftMr_SpecialN_Enter (800E0DA8) / ftMr_SpecialAirN_Enter (800E1040);
/// Kirby's ftKb_SpecialN_800F9110 / ftKb_SpecialNMr_800F93CC.
pub fn enter<C: Fireball>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.variables[0] = 0;
    f.commands.clear_throw_flags();
    let [ground, aerial] = C::rows(f);
    f.change_motion_state(if air { aerial } else { ground }, a)
        .expect("Fireball assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    C::install_accessory(f);
}

/// ftMr_SpecialN_ItemFireSpawn (800E0EE0) / fn_800F9260 (800F9260): on the
/// script's throw flag, the kind's projectile at the left hand
/// (lb_8000B1CC(parts[hand].joint)).
pub fn item_spawn<C: Fireball>(
    f: &mut Fighter,
    assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) {
    if !C::armed(f) || !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let bone = usize::from(assets.parts.joint(C::HAND).expect("hand part"));
    let c = &mut f.core;
    let hand =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, bone, Vec3::ZERO);
    C::throw(f, assets, rng, hand, bone);
}

/// it_8029B6F8 / itDrMarioPill_Spawn / it_802C01AC's SpawnItem: prev_pos is
/// the hand on the stage plane; pos is it_8026BB68's ECB midpoint
/// (ftLib_80086990, retail 800869AC..BC: fadds, fmuls, fadds).
pub fn spawn_projectile(f: &mut Fighter, kind: ItemKind, hand: Vec3, argument: i32) {
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(kind, c.player.id, hand, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_argument = argument;
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// A fireball of `kind` (it_8029B6F8), then its hand flash
/// (efSync_Spawn(flash, gobj, hand, &facing_dir)).
pub fn throw_fireball(f: &mut Fighter, kind: ItemKind, flash: u16, hand: Vec3, bone: usize) {
    spawn_projectile(f, kind, hand, 0);
    f.core
        .effects_after_items
        .push(EffectRequest::SyncAttached { id: flash, bone });
}

/// The FTKIND_MARIO branch: the fireball (it_8029B6F8), then its flash
/// (efSync_Spawn(1146, gobj, hand, &facing_dir)).
pub fn spawn_fireball(
    f: &mut Fighter,
    _assets: &FighterAssets,
    _rng: &mut gekko_math::HsdRng,
    hand: Vec3,
    bone: usize,
) {
    throw_fireball(f, ItemKind::MarioFire, FIRE_FLASH, hand, bone);
}

/// ftMr_SpecialN_Anim (800E0E18) / ftKb_MrSpecialN_Anim (800F9198): Wait
/// (ft_8008A2BC) at the animation's end.
pub fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, p.assets, false)?;
    }
    Ok(None)
}

/// ftMr_SpecialAirN_Anim (800E10B0) / ftKb_MrSpecialAirN_Anim (800F9454):
/// Fall (ftCo_Fall_Enter) at the animation's end.
pub fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, p.assets, true)?;
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
/// (800E1178; Kirby's ftKb_SpecialNLg_800F951C) continues in the air.
pub fn ground_collision<C: Fireball>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball collision assets");
    let aerial = C::rows(f)[1];
    common::ground_to_air(f, aerial, assets, GROUND_AIR_FLAGS)?;
    C::install_accessory(f);
    Ok(())
}

/// ftMr_SpecialAirN_Coll (800E113C): landing continues on the ground
/// (ftMr_SpecialAirN_AirToGround, 800E11E0; Kirby's
/// ftKb_SpecialNLg_800F9598).
pub fn air_collision<C: Fireball>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fireball landing assets");
    let ground = C::rows(f)[0];
    common::air_to_ground(f, ground, assets, GROUND_AIR_FLAGS)?;
    C::install_accessory(f);
    Ok(())
}

/// The grounded and aerial rows of another user's table (Kirby's 399/400
/// and 512/513): `actions` playing `first_animation` and the next, with
/// these callbacks.
pub const fn rows<C: Fireball>(
    actions: [ActionId; 2],
    first_animation: i32,
) -> [melee_ft::fighter::MotionRow; 2] {
    use melee_ft::fighter::MotionRow;
    use melee_types::CommonMotionState;
    [
        MotionRow {
            action: actions[0],
            id: CommonMotionState::None,
            animation: first_animation,
            anim: ground_anim,
            iasa: ground_input,
            physics: ground_physics,
            collision: ground_collision::<C>,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        },
        MotionRow {
            action: actions[1],
            id: CommonMotionState::None,
            animation: first_animation + 1,
            anim: air_anim,
            iasa: air_input,
            physics: air_physics,
            collision: air_collision::<C>,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        },
    ]
}
