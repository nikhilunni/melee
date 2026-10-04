//! Oil Panic, ftgamewatchspeciallw.c (8014CBF4..8014DEEC).
//!
//! While B is held the bucket's absorbing bubble is up (the script's
//! cmd_vars[0], kept across the motion's own restarts and ground/air
//! changes); an absorbed hitbox fills one level of the bucket and adds its
//! damage (ftData_OnAbsorb -> the Catch rows). With three levels the next
//! down special spills: the Shoot rows, whose hitboxes take the absorbed
//! damage times an attribute plus another, and the oil article.
use crate::{
    articles::{self, Accessory},
    common::{self, flags, row},
    init::{GameWatch, PANIC_EMPTY, PANIC_FULL},
};
use hsd_types::Vec3;
use melee_coll::{defense::AbsorbDescriptor, hitbox::CapsulePhase};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        absorb::Absorbed,
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
    input::Buttons,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{GroundOrAir, ItemKind};

/// ftGw_MS_SpecialLw .. ftGw_MS_SpecialAirLwShoot.
pub const GROUND: ActionId = ActionId(375);
pub const GROUND_CATCH: ActionId = ActionId(376);
pub const GROUND_SHOOT: ActionId = ActionId(377);
pub const AIR: ActionId = ActionId(378);
pub const AIR_CATCH: ActionId = ActionId(379);
pub const AIR_SHOOT: ActionId = ActionId(380);

pub const fn rows() -> [MotionRow; 6] {
    [
        row(
            GROUND,
            hold_anim,
            hold_input,
            hold_ground_physics,
            hold_ground_collision,
        ),
        row(
            GROUND_CATCH,
            catch_anim,
            common::no_input,
            ground_physics,
            catch_ground_collision,
        ),
        row(
            GROUND_SHOOT,
            shoot_anim,
            common::no_input,
            ground_physics,
            shoot_ground_collision,
        ),
        row(
            AIR,
            hold_anim,
            hold_input,
            hold_air_physics,
            hold_air_collision,
        ),
        row(
            AIR_CATCH,
            catch_anim,
            common::no_input,
            common::fall_without_drift,
            catch_air_collision,
        ),
        row(
            AIR_SHOOT,
            shoot_anim,
            common::no_input,
            common::fall_without_drift,
            shoot_air_collision,
        ),
    ]
}

/// transition_flags0 (ftgamewatchspeciallw.c:370): the ground/air set with
/// KeepGfx, KeepColAnimHitStatus and SkipHit.
const GROUND_AIR_FLAGS: u32 =
    flags::GROUND_AIR | flags::KEEP_GFX | flags::KEEP_COL_ANIM_HIT_STATUS | flags::SKIP_HIT;
/// transition_flags1 (ftgamewatchspeciallw.c:409): the same without
/// KeepColAnimHitStatus, for the hold's restarts.
const RESTART_FLAGS: u32 = flags::GROUND_AIR | flags::KEEP_GFX | flags::SKIP_HIT;

/// anim_update_frame: the hold's animation restarts here while B is held.
const HOLD_LOOP_FRAME: f32 = 38.0;
/// The frame it restarts at (ftGw_SpecialLw_UpdateAction(gobj, 5): one
/// frame before), also where a Catch row returns to.
const HOLD_RESTART_FRAME: f32 = 4.0;
/// ftCo_800BFFD0(fp, 5, 0): the full bucket's colour animation.
pub const FULL_BUCKET_COLOR: u8 = 5;

/// The model groups ftGw_SpecialLw_UpdateBucketModel selects: the bucket
/// and its three oil levels.
const BUCKET_GROUP: i32 = 5;
const LEVEL_GROUPS: [i32; 3] = [6, 7, 8];
const BUCKET_VARIANT: i32 = 2;
const HIDDEN: i32 = -1;

/// fp->mv.gw.SpecialLw.
#[derive(Clone, Copy, Debug, Default)]
pub struct OilPanic {
    /// isRelease: B was let go.
    pub released: bool,
    /// turnFrames: frames until the stick may turn him again.
    pub turn_frames: i32,
}

fn gw(f: &mut Fighter) -> &mut GameWatch {
    f.character.get_mut::<GameWatch>()
}

fn is_air(f: &Fighter) -> bool {
    f.motion_state.action.0 >= AIR.0
}

/// x2238_panicCharge, and ftGw_Init_UnkMotionStates4's glow (colour 5
/// whenever the secondary colour slot empties) while the bucket is full.
pub fn set_charge(f: &mut Fighter, level: i32) {
    gw(f).panic_charge = level;
    f.core.combat.secondary_color_fallback = (level >= PANIC_FULL).then_some(FULL_BUCKET_COLOR);
}

/// ftGw_SpecialLw_UpdateBucketModel (8014CDC0): the bucket and as many oil
/// levels as the charge (ftParts_80074B0C).
fn update_bucket_model(f: &mut Fighter) {
    let charge = gw(f).panic_charge;
    f.commands
        .model_selections
        .insert(BUCKET_GROUP as _, BUCKET_VARIANT as _);
    for (level, group) in LEVEL_GROUPS.into_iter().enumerate() {
        let variant = if charge > level as i32 { 0 } else { HIDDEN };
        f.commands.model_selections.insert(group as _, variant as _);
    }
}

/// ftColl_CreateAbsorbHit(gobj, &sa->x80_GAMEWATCH_PANIC_ABSORPTION).
fn create_absorb_hit(f: &mut Fighter) {
    let bubble = gw(f).attributes.panic.absorb;
    f.core.create_absorb_hit(&AbsorbDescriptor {
        bone: bubble.bone as usize,
        offset: bubble.offset,
        radius: bubble.size,
    });
}

/// ftGw_SpecialLw_Enter (8014CEA4) / ftGw_SpecialAirLw_Enter (8014CF2C,
/// 8014CF68: fdivs): a full bucket spills instead.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if gw(f).panic_charge >= PANIC_FULL {
        return release_oil(f, air, a).expect("Oil Panic spill assets");
    }
    if air {
        f.physics.self_velocity.x /= gw(f).attributes.panic.momentum_preserve;
    }
    f.physics.self_velocity.y = 0.0;
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Oil Panic assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    // ftGameWatch_SpecialLw_SetVars.
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    gw(f).oil_panic = OilPanic::default();
}

/// ftGameWatch_SpecialLw_UpdateVars: the script's cmd_vars[0] raises the
/// bubble once (1 becomes 2) and takes it down at 0; the bucket shows from
/// then on.
fn update_vars(f: &mut Fighter) {
    match f.commands.variables[0] {
        1 => {
            f.commands.variables[0] = 2;
            create_absorb_hit(f);
        }
        0 => f.core.combat.absorb.active = false,
        _ => {}
    }
    if f.commands.variables[0] != 0 {
        update_bucket_model(f);
    }
}

/// ftGameWatch_SpecialLw_UpdateVarsColl: after a ground/air change (which
/// took the bubble down), it goes back up while cmd_vars[0] is 2.
fn update_vars_after_transition(f: &mut Fighter) {
    match f.commands.variables[0] {
        2 => create_absorb_hit(f),
        0 => f.core.combat.absorb.active = false,
        _ => {}
    }
    if f.commands.variables[0] != 0 {
        update_bucket_model(f);
    }
}

/// ftGameWatch_SpecialLw_UpdateVarsAction: after a restart, the bubble is
/// up whenever cmd_vars[0] was raised.
fn update_vars_after_restart(f: &mut Fighter) {
    if f.commands.variables[0] >= 1 {
        f.commands.variables[0] = 2;
        create_absorb_hit(f);
    }
    if f.commands.variables[0] != 0 {
        update_bucket_model(f);
    }
}

/// ftGw_SpecialLw_UpdateAction (8014D4EC) / ftGw_SpecialAirLw_UpdateAction
/// (8014D58C), and the Catch rows' return: the hold again at `frame`.
fn restart_hold(f: &mut Fighter, air: bool, frame: f32, assets: &FighterAssets) -> Result<()> {
    common::change(
        f,
        if air { AIR } else { GROUND },
        RESTART_FLAGS,
        frame,
        assets,
    )?;
    f.step_animation(assets);
    update_vars_after_restart(f);
    Ok(())
}

/// ftGw_SpecialLw_Anim (8014D014) / ftGw_SpecialAirLw_Anim (8014D0E0): at
/// frame 38 with B still held the hold restarts at frame 4; then the
/// bubble follows cmd_vars[0]; the animation's end is Wait or Fall.
fn hold_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let air = is_air(f);
    if f.animation.frame == HOLD_LOOP_FRAME && !gw(f).oil_panic.released {
        restart_hold(f, air, HOLD_RESTART_FRAME, p.assets)?;
    }
    update_vars(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ftGw_SpecialLw_IASA (8014D1AC) / ftGw_SpecialAirLw_IASA (8014D264): a
/// stick past the deadzone turns him, then not again for x7C frames
/// (8014D224: fctiwz); letting go of B ends the loop.
fn hold_input(f: &mut Fighter, p: InputPhase<'_>) {
    if gw(f).oil_panic.turn_frames > 0 {
        gw(f).oil_panic.turn_frames -= 1;
    } else {
        let x = f.input.current.stick.x;
        let magnitude = if x < 0.0 { -x } else { x };
        if magnitude > p.assets.input.thresholds.horizontal_stick_deadzone {
            let facing = f.physics.facing;
            // ftCommon_UpdateFacing.
            f.physics.facing = if x >= 0.0 { 1.0 } else { -1.0 };
            if facing != f.physics.facing {
                let frames = gekko_math::msl::fctiwz(gw(f).attributes.panic.turn_frames);
                gw(f).oil_panic.turn_frames = frames;
            }
        }
    }
    if !f.input.current.held.intersects(Buttons::B) {
        gw(f).oil_panic.released = true;
    }
}

/// ftGw_SpecialLw_Phys (8014D31C): ft_80084F3C, then the bubble's position
/// is read again (ftColl_8007AF10).
fn hold_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
    f.core.refresh_absorb_position();
}

/// ftGw_SpecialAirLw_Phys (8014D350): the move's own gravity, terminal
/// speed and air friction, then ftColl_8007AF10.
fn hold_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = gw(f).attributes.panic.clone();
    common::fall(f, a.fall_accel, a.vel_y_max);
    common::air_friction(f, a.momentum_mul);
    common::finish_update(f, &p);
    f.core.refresh_absorb_position();
}

/// ftGw_SpecialLwCatch_Phys / ftGw_SpecialLwShoot_Phys: ft_80084F3C.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftGw_SpecialLw_Coll (8014D3B0): off the floor (ft_800827A0),
/// ftGw_SpecialLw_GroundToAir (8014D426).
fn hold_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Oil Panic collision assets");
    common::ground_to_air(f, AIR, GROUND_AIR_FLAGS, assets)?;
    update_vars_after_transition(f);
    Ok(())
}

/// ftGw_SpecialAirLw_Coll (8014D3EC): landing (ft_80081D0C),
/// ftGw_SpecialAirLw_AirToGround.
fn hold_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Oil Panic landing assets");
    common::air_to_ground(f, GROUND, GROUND_AIR_FLAGS, assets)?;
    update_vars_after_transition(f);
    Ok(())
}

/// ftGw_SpecialLw_AbsorbThink_DecideAction (8014D9B8), ftData_OnAbsorb:
/// the bucket gains this frame's hits and damage; full, it glows; then the
/// Catch row of the fighter's ground state.
pub fn on_absorb(f: &mut Fighter, assets: &FighterAssets, absorbed: Absorbed) {
    let charge = gw(f).panic_charge + absorbed.hits;
    gw(f).panic_damage += absorbed.damage;
    set_charge(f, charge);
    if charge >= PANIC_FULL {
        f.core.install_color_overlay_now(FULL_BUCKET_COLOR, assets);
    }
    let state = if f.physics.ground_or_air == GroundOrAir::Ground {
        GROUND_CATCH
    } else {
        AIR_CATCH
    };
    f.change_motion_state(state, assets)
        .expect("Oil Panic catch assets");
    update_bucket_model(f);
}

/// ftGw_SpecialLwCatch_Anim (8014D62C) / ftGw_SpecialAirLwCatch_Anim
/// (8014D6F8): at the animation's end a full bucket is Wait or Fall;
/// otherwise the hold again at frame 4.
fn catch_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.animation.frames_remaining(&f.skeleton) {
        return Ok(None);
    }
    let air = is_air(f);
    if gw(f).panic_charge >= PANIC_FULL {
        common::finish(f, air, p.assets)?;
    } else {
        restart_hold(f, air, HOLD_RESTART_FRAME, p.assets)?;
    }
    Ok(None)
}

/// ftGw_SpecialLwCatch_Coll (8014D808): off the floor (ft_80082708),
/// ftGw_SpecialLwCatch_GroundToAir.
fn catch_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Oil Panic catch collision assets");
    common::ground_to_air(f, AIR_CATCH, GROUND_AIR_FLAGS, assets)
}

/// ftGw_SpecialAirLwCatch_Coll (8014D844).
fn catch_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Oil Panic catch landing assets");
    common::air_to_ground(f, GROUND_CATCH, GROUND_AIR_FLAGS, assets)
}

/// ftGw_SpecialLwShoot_ReleaseOil (8014DD28) /
/// ftGw_SpecialAirLwShoot_ReleaseOil (8014DE0C): the Shoot row; its
/// hitboxes' damage, cmd_vars[1], is the absorbed damage times x78
/// (8014DD98: fmuls, __cvt_fp2unsigned) plus x74 (8014DDC0: fadds,
/// __cvt_fp2unsigned); the bucket empties.
fn release_oil(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state(if air { AIR_SHOOT } else { GROUND_SHOOT }, assets)?;
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    let (absorbed, multiplier, addend) = {
        let gw = gw(f);
        let a = &gw.attributes.panic;
        (gw.panic_damage, a.damage_mul, a.damage_add)
    };
    let scaled = (absorbed as f32 * multiplier) as u32;
    let damage = (scaled as f32 + addend) as u32;
    f.commands.variables[1] = damage;
    set_charge(f, PANIC_EMPTY);
    gw(f).panic_damage = 0;
    update_bucket_model(f);
    articles::install(f, Accessory::PanicSetup);
    Ok(())
}

/// ftGw_SpecialLwShoot_Anim (8014DAA4) / ftGw_SpecialAirLwShoot_Anim
/// (8014DB30): every enabled hitbox takes cmd_vars[1] as its damage
/// (ftColl_8007ABD0); the animation's end is Wait or Fall.
fn shoot_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    apply_spill_damage(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = is_air(f);
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ftColl_8007ABD0 (8007ABD0) on each enabled hitbox: the count is the
/// damage (ftCo_800DEEB8's smash charge and the y-scale branch do not
/// apply to an unscaled special), the hitbox damage its staled value
/// (ft_80089228).
fn apply_spill_damage(f: &mut Fighter) {
    assert!(
        f.player.scale == 1.0,
        "ftColl_8007ABD0: a scaled Mr. Game & Watch's spill"
    );
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: a charged spill"
    );
    let damage = f.commands.variables[1] as f32;
    let staled = f.commands.stale_damage(damage);
    for hit in f.commands.hitboxes.iter_mut().flatten() {
        if hit.phase == CapsulePhase::Enabled {
            hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
            hit.descriptor.damage = staled;
        }
    }
}

/// ftGw_SpecialLwShoot_Coll (8014DC00): off the floor (ft_80082708),
/// ftGw_SpecialLwShoot_GroundToAir.
fn shoot_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Oil Panic spill collision assets");
    common::ground_to_air(f, AIR_SHOOT, GROUND_AIR_FLAGS, assets)?;
    update_bucket_model(f);
    articles::install(f, Accessory::PanicSetup);
    Ok(())
}

/// ftGw_SpecialAirLwShoot_Coll (8014DC3C).
fn shoot_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Oil Panic spill landing assets");
    common::air_to_ground(f, GROUND_SHOOT, GROUND_AIR_FLAGS, assets)?;
    update_bucket_model(f);
    articles::install(f, Accessory::PanicSetup);
    Ok(())
}

/// ftGw_SpecialLw_ItemPanicSetup (8014CBF4): without oil out, the oil at
/// TopN (it_802C7D60, hanging from part 0); the callbacks; the accessory
/// then uninstalls.
pub fn panic_setup(f: &mut Fighter) {
    if !gw(f).articles.panic {
        let c = &mut f.core;
        let position =
            melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, 0, Vec3::ZERO);
        let spawn = SpawnItem::attached(
            ItemKind::GameWatchPanic,
            c.player.id,
            position,
            c.physics.facing,
        );
        c.item_requests.push(ItemRequest::SpawnInHand {
            spawn,
            part: 0,
            hold: false,
            catch_item: false,
            scale_by_owner: false,
        });
        gw(f).articles.panic = true;
    }
    articles::install_callbacks(f, ItemKind::GameWatchPanic);
    articles::uninstall(f);
}

/// ftGw_SpecialLw_ItemPanicRemove (8014CC98): it_802C7E94 destroys the
/// oil, which lets go of its owner.
pub fn remove_panic(f: &mut Fighter) {
    if gw(f).articles.panic {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::GameWatchPanic,
            control: ItemControl::Remove,
        });
        articles::destroyed(f, ItemKind::GameWatchPanic);
    }
}
