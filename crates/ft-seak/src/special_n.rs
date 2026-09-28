//! Needle Storm, ftseakspecialn.c (80111FBC..80112ED8).
//!
//! The start (341 / 345) puts the needle bundle in Sheik's hand; the loop
//! (342 / 346) adds a needle per cycle up to six while B is held. Releasing
//! B throws (344 / 348): accessory4 fires one needle at each of the
//! script's six throw frames while she has any. Shield cancels (343 /
//! 347), keeping the charge. Hit while charging, she drops the needles.
use crate::{
    common,
    init::{Accessory, Sheik},
};
use gekko_math::fma::fmadds;
use hsd_types::Vec3;
use it_seak::needle::Launch;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{FtPart, GroundOrAir, ItemKind};

/// ftSk_MS_SpecialNStart (341) .. ftSk_MS_SpecialAirNEnd (348).
pub const GROUND_START: ActionId = ActionId(341);
pub const GROUND_LOOP: ActionId = ActionId(342);
pub const GROUND_CANCEL: ActionId = ActionId(343);
pub const GROUND_END: ActionId = ActionId(344);
pub const AIR_START: ActionId = ActionId(345);
pub const AIR_LOOP: ActionId = ActionId(346);
pub const AIR_CANCEL: ActionId = ActionId(347);
pub const AIR_END: ActionId = ActionId(348);
/// Their submotions, ftSk_SM_SpecialNStart..SpecialAirNEnd in row order.
pub const ANIMATIONS: [i32; 8] = [295, 296, 297, 298, 299, 300, 301, 302];

/// A full charge (ftSk_SpecialNLoop_Anim).
const MAX_NEEDLES: i32 = 6;
/// ftCo_800BFFD0(fp, 86, 0): the full charge's flash.
const FULL_CHARGE_FLASH: u8 = 86;
/// The loop's frame counter parked past its sound once charged.
const CHARGED_LOOP_FRAMES: i32 = 100;
/// ft_PlaySFX(fp, 270134, 127, 64): a needle added.
const CHARGE_SOUND: u32 = 270_134;
/// ft_PlaySFX(fp, 270140, 127, 64): a needle thrown.
const THROW_SOUND: u32 = 270_140;
/// efSync_Spawn(1283, gobj, &pos): the throw's flash
/// (efLib_CreateGenerator 0x6A at the point).
const THROW_FLASH: u16 = 0x503;
/// it_802B19AC's part: the bundle rides the left hand (FtPart_L1stNb).
const BUNDLE_PART: FtPart = FtPart::L1stNb;
/// needleYPosScale: a throw's random height offsets.
const THROW_HEIGHTS: [f32; 9] = [-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0];
/// The aerial throw doubles the height offset (retail @504, fmadds).
const AIR_HEIGHT_SCALE: f32 = 2.0;
/// The six script frames a throw leaves on (ftSk_SpecialNEnd_Anim).
const THROW_FRAMES: [i32; 6] = [2, 5, 8, 11, 14, 17];

/// The ground/air counterparts' flags: ftCommon_GroundAirColl_MF.
const GROUND_AIR_FLAGS: MotionEntryFlags =
    MotionEntryFlags(1 << 7 | 1 << 12 | 1 << 14 | 1 << 18 | 1 << 19 | 1 << 22 | 1 << 26 | 1 << 27);

/// mv.sk.specialn (ftSeak/types.h).
#[derive(Clone, Copy, Debug, Default)]
pub struct NeedleCharge {
    /// x0: frames since the throw began.
    pub throw_frames: i32,
    /// x4: a throw frame arrived; accessory4 fires on it.
    pub throw_pending: bool,
    /// x8: frames into the current charge cycle.
    pub cycle_frames: i32,
}

fn sheik(f: &mut Fighter) -> &mut Sheik {
    f.character.get_mut::<Sheik>()
}
fn attributes(f: &Fighter) -> &crate::attributes::NeedleAttributes {
    &f.character.get::<Sheik>().attributes.needles
}

/// setDmgCallbacks: take_dmg_cb and death2_cb = ftSk_Init_80110198.
fn install_damage_callbacks(f: &mut Fighter) {
    sheik(f).damage_callbacks = true;
}
/// clearDmgCallbacks.
fn clear_damage_callbacks(f: &mut Fighter) {
    sheik(f).damage_callbacks = false;
}

/// doEnter: the start, the throw flag and command variables cleared, the
/// scratch reset, at least one needle, the damage callbacks, then
/// ftAnim_8006EBA4.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.change_motion_state(if air { AIR_START } else { GROUND_START }, a)
        .expect("Needle Storm assets");
    f.commands.clear_throw_flags();
    f.commands.variables = [0; 4];
    let s = sheik(f);
    s.needle_charge = NeedleCharge::default();
    if s.needles == 0 {
        s.needles = 1;
    }
    install_damage_callbacks(f);
    f.step_animation(a);
}

/// ftSk_SpecialNStart_Anim / ftSk_SpecialAirNStart_Anim (80112198 /
/// 801124D8): at the start's end the bundle in the left hand
/// (it_802B19AC, Item_InitSpawnOnPlane at Sheik), then the loop.
pub fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let part = p.assets.parts.joint(BUNDLE_PART).expect("L1stNb part");
        // Item_InitSpawnOnPlane: on the stage plane, with the initial
        // collision.
        let spawn = SpawnItem::ray(
            ItemKind::SeakNeedleHeld,
            f.player.id,
            f.physics.position,
            f.physics.facing,
        );
        f.core.item_requests.push(ItemRequest::SpawnInHand {
            spawn,
            part,
            hold: false,
        });
        sheik(f).holding_needles = true;
        let air = f.motion_state.action == AIR_START;
        f.change_motion_state(if air { AIR_LOOP } else { GROUND_LOOP }, p.assets)?;
        install_damage_callbacks(f);
    }
    Ok(None)
}

/// ftSk_SpecialNLoop_Anim / ftSk_SpecialAirNLoop_Anim (801122D8): the
/// charge sound at a cycle's first frame; each time the animation comes
/// back to frame 0 a needle more, up to six (then the flash).
pub fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if sheik(f).needle_charge.cycle_frames == 0 {
        common::play_sound(f, CHARGE_SOUND);
    }
    sheik(f).needle_charge.cycle_frames += 1;
    if f.animation.frame == 0.0 {
        let s = sheik(f);
        s.needles += 1;
        s.needle_charge.cycle_frames = 0;
        if s.needles > MAX_NEEDLES {
            s.needles = MAX_NEEDLES;
            s.needle_charge.cycle_frames = CHARGED_LOOP_FRAMES;
            f.core
                .commands
                .color_animations
                .push(melee_cmd::ColorAnimationRequest {
                    id: FULL_CHARGE_FLASH,
                    duration: 0,
                });
        }
    }
    Ok(None)
}

/// ftSk_SpecialNCancel_Anim / ftSk_SpecialAirNCancel_Anim: the bundle goes;
/// at the end Wait, or Fall (a special fall with the attribute's landing
/// lag when it is set).
pub fn cancel_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    sheik(f).holding_needles = false;
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, p.assets)?;
    }
    Ok(None)
}

/// ftSk_SpecialNEnd_Anim / ftSk_SpecialAirNEnd_Anim: each throw frame
/// arms a throw and lets go of the bundle; at the end Wait or Fall.
pub fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let s = sheik(f);
    if THROW_FRAMES.contains(&s.needle_charge.throw_frames) {
        s.needle_charge.throw_pending = true;
        s.holding_needles = false;
    }
    s.needle_charge.throw_frames += 1;
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, p.assets)?;
    }
    Ok(None)
}

/// The end of a cancel or throw: ft_8008A2BC on the ground; in the air
/// Fall, or ftCo_80096900(gobj, 1, 0, true, 1, x10) when x10 is set.
fn finish(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let air = matches!(f.motion_state.action, AIR_CANCEL | AIR_END);
    if !air {
        return common::finish(f, false, assets);
    }
    fall(f, assets)
}

/// Fall, or the special fall with the attribute's landing lag.
fn fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let lag = attributes(f).landing_lag;
    if lag == 0.0 {
        common::finish(f, true, assets)
    } else {
        f.enter_special_fall(assets, true, false, true, 1.0, lag)
    }
}

/// doIasa (ftSk_SpecialNLoop_IASA / ftSk_SpecialAirNLoop_IASA): B let go
/// throws (accessory4 = shootNeedles); L or R cancels.
pub fn loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    let air = f.motion_state.action == AIR_LOOP;
    if !f.input.current.held.intersects(Buttons::B) {
        sheik(f).needle_charge.throw_frames = 0;
        f.change_motion_state(if air { AIR_END } else { GROUND_END }, p.assets)
            .expect("Needle Storm throw");
        install_damage_callbacks(f);
        sheik(f).accessory = Accessory::ThrowNeedles;
        f.core.arm_accessory4();
    } else if f.input.pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
        f.change_motion_state(if air { AIR_CANCEL } else { GROUND_CANCEL }, p.assets)
            .expect("Needle Storm cancel");
        install_damage_callbacks(f);
    }
}

/// ft_80084F3C.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
/// ft_80084EEC.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

/// ftSk_SpecialNStart_Coll / ftSk_SpecialNLoop_Coll: off the floor, the
/// aerial row at the current frame with the damage callbacks.
pub fn charge_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Needle Storm collision assets");
    let state = if f.motion_state.action == GROUND_START {
        AIR_START
    } else {
        AIR_LOOP
    };
    common::ground_to_air(f, state, GROUND_AIR_FLAGS, assets)?;
    install_damage_callbacks(f);
    Ok(())
}

/// ftSk_SpecialNCancel_Coll / ftSk_SpecialNEnd_Coll: off the floor the
/// callbacks go (the throw also drops the charge and its pending throw),
/// then Fall or the special fall.
pub fn release_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Needle Storm collision assets");
    clear_damage_callbacks(f);
    if f.motion_state.action == GROUND_END {
        let s = sheik(f);
        s.needles = 0;
        s.needle_charge.throw_pending = false;
    }
    fall(f, assets)
}

/// doColl (ftSk_SpecialAirNStart_Coll / ftSk_SpecialAirNLoop_Coll): on
/// landing, the grounded row at the current frame with the callbacks.
pub fn charge_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Needle Storm landing assets");
    let state = if f.motion_state.action == AIR_START {
        GROUND_START
    } else {
        GROUND_LOOP
    };
    common::air_to_ground(f, state, GROUND_AIR_FLAGS, assets)?;
    install_damage_callbacks(f);
    Ok(())
}

/// ftSk_SpecialAirNCancel_Coll / ftSk_SpecialAirNEnd_Coll: landing (the
/// throw first drops the charge and its pending throw); the cancel lands
/// before its callbacks go, the throw after.
pub fn release_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Needle Storm landing assets");
    if f.motion_state.action == AIR_END {
        clear_damage_callbacks(f);
        let s = sheik(f);
        s.needles = 0;
        s.needle_charge.throw_pending = false;
        f.enter_landing(assets)?;
    } else {
        f.enter_landing(assets)?;
        clear_damage_callbacks(f);
    }
    Ok(())
}

/// shootNeedles (80112D44): on a pending throw, while Sheik has a needle,
/// one thrown ahead of her from a random height: grounded x0 ahead (fmuls,
/// then fmadds by her scale) at x4 plus the offset (fadds, fmadds); in the
/// air x8 ahead at twice the offset plus xC (fmadds, fmadds). Then the
/// flash and the sound.
pub fn throw_needle(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    if !std::mem::take(&mut sheik(f).needle_charge.throw_pending) {
        return;
    }
    if sheik(f).needles <= 0 {
        return;
    }
    let a = attributes(f).clone();
    let scale = f.player.scale;
    let facing = f.physics.facing;
    let mut position = f.physics.position;
    let airborne = f.physics.ground_or_air != GroundOrAir::Ground;
    if !airborne {
        position.x = fmadds(scale, a.ground_offset_x * facing, position.x);
        let height = a.ground_offset_y + THROW_HEIGHTS[rng.randi(9) as usize];
        position.y = fmadds(scale, height, position.y);
    } else {
        position.x = fmadds(scale, a.air_offset_x * facing, position.x);
        let height = fmadds(
            AIR_HEIGHT_SCALE,
            THROW_HEIGHTS[rng.randi(9) as usize],
            a.air_offset_y,
        );
        position.y = fmadds(scale, height, position.y);
    }
    position.z = 0.0;
    spawn_needle(f, position, Launch::Thrown { airborne });
    sheik(f).needles -= 1;
    f.effects.push(EffectRequest::PositionalGenerator {
        id: THROW_FLASH,
        position,
    });
    common::play_sound(f, THROW_SOUND);
}

/// it_802AFD8C: a needle at `point` leaving from Sheik's ECB centre
/// (it_8026BB68 -> ftLib_80086990), then it_802AFEA8's launch.
fn spawn_needle(f: &mut Fighter, point: Vec3, launch: Launch) {
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(
        ItemKind::SeakNeedleThrow,
        c.player.id,
        point,
        c.physics.facing,
    );
    // ftLib_80086990 (retail 800869AC..BC: fadds, fmuls, fadds).
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_argument = launch.argument();
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// ftSk_SpecialN_80111FBC (80111FBC): while she holds the bundle, it goes
/// and every charged needle drops from her (x4 on the ground, xC in the
/// air, times her scale: 80112044 fmadds); a charge she holds or not is
/// then spent.
pub fn drop_needles(f: &mut Fighter) {
    if std::mem::take(&mut sheik(f).holding_needles) {
        while sheik(f).needles != 0 {
            let a = attributes(f);
            let height = if f.physics.ground_or_air == GroundOrAir::Ground {
                a.ground_offset_y
            } else {
                a.air_offset_y
            };
            let mut position = f.physics.position;
            position.y = fmadds(f.player.scale, height, position.y);
            position.z = 0.0;
            spawn_needle(f, position, Launch::Dropped);
            sheik(f).needles -= 1;
        }
        // ftSk_SpecialN_801120D4 when the callbacks are Sheik's own.
        clear_damage_callbacks(f);
    }
    sheik(f).needles = 0;
}
