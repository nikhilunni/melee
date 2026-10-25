//! The copied Needle Storm, ftkirbyspecialseak.c (80105FEC..80106F3C):
//! Kirby's own rows 467..474 (ftKb_Sk*), not Sheik's states. Beside the
//! names they differ from ftseakspecialn.c in what goes when: the charge's
//! callbacks are Kirby's (ftKb_SpecialN_set_cbs, the put-away arm
//! ftKb_SpecialNSk_8010603C), the grounded cancel and throw end in Wait
//! and leave the floor into Fall without touching them, the aerial cancel
//! lands without them, the full charge's colour is 87 (Sheik's 86), and
//! the bundle rides FtPart_RHandN (39). The articles are It_Kind_Kirby_
//! SeakNeedleThrow / SeakNeedleHeld (`it_seak::needle`, generic over the
//! owner).
use crate::{
    common,
    init::{Accessory, Kirby},
};
use gekko_math::fma::fmadds;
use hsd_types::Vec3;
use it_seak::needle::Launch;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        commands::{FootstepSound, SoundChannel},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{FighterKind, FtPart, GroundOrAir, ItemKind};

/// ftKb_MS_SkSpecialNStart (467) .. ftKb_MS_SkSpecialAirNEnd (474).
pub const GROUND_START: ActionId = ActionId(467);
pub const GROUND_LOOP: ActionId = ActionId(468);
pub const GROUND_CANCEL: ActionId = ActionId(469);
pub const GROUND_END: ActionId = ActionId(470);
pub const AIR_START: ActionId = ActionId(471);
pub const AIR_LOOP: ActionId = ActionId(472);
pub const AIR_CANCEL: ActionId = ActionId(473);
pub const AIR_END: ActionId = ActionId(474);
/// ftKb_SM_SkSpecialNStart .. SkSpecialAirNEnd, in row order.
const FIRST_ANIMATION: i32 = 402;

/// A full charge (ftKb_SkSpecialNLoop_Anim: `xB4 > 6`).
pub const MAX_NEEDLES: i32 = 6;
/// ftCo_800BFFD0(fp, 87, 0): the full charge's colour, which
/// ftKb_Init_UnkMotionStates4 also installs while `xB4 == 6`.
pub const FULL_CHARGE_COLOR: u8 = 87;
/// The loop's frame counter parked past its sound once charged.
const CHARGED_LOOP_FRAMES: i32 = 100;
/// ft_PlaySFX(fp, 270134, 127, 64): a needle added.
const CHARGE_SOUND: u32 = 270_134;
/// ft_PlaySFX(fp, 0x41F3C, 127, 64): a needle thrown.
const THROW_SOUND: u32 = 0x4_1F3C;
/// efSync_Spawn(0x503, gobj, &pos): the throw's flash (Sheik's).
const THROW_FLASH: u16 = 0x503;
/// it_802B19AC(gobj, &cur_pos, 39, ...): the bundle in the right hand.
const BUNDLE_PART: FtPart = FtPart::RHandN;
/// ftKb_Init_803CB770: a throw's random height offsets.
const THROW_HEIGHTS: [f32; 9] = [-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0];
/// The aerial throw doubles the height offset (retail 80106EB4, @480).
const AIR_HEIGHT_SCALE: f32 = 2.0;
/// The six script frames a throw leaves on (ftKb_SkSpecialNEnd_Anim).
const THROW_FRAMES: [i32; 6] = [2, 5, 8, 11, 14, 17];

/// ftCommon_GroundAirColl_MF.
const GROUND_AIR_FLAGS: MotionEntryFlags = common::GROUND_AIR_COLL_BASE_FLAGS;

/// ftKb_DatAttrs +290..+2A4 (`specialn_sk_*`).
#[derive(Clone, Debug, PartialEq)]
pub struct NeedleStormAttributes {
    /// +290: grounded throw offset ahead, times the scale.
    pub ground_offset_x: f32,
    /// +294: grounded throw (and drop) height.
    pub ground_offset_y: f32,
    /// +298: aerial throw offset ahead.
    pub air_offset_x: f32,
    /// +29C: aerial throw (and drop) height.
    pub air_offset_y: f32,
    /// +2A0 (`specialn_sk_freefall_toggle`): an aerial end's landing lag;
    /// zero ends in Fall.
    pub landing_lag: f32,
}

/// The copied Needle Storm's words: u.kb.xB4, u.kb.xB8 and
/// mv.kb.specialhi (x0, x4, x8).
#[derive(Clone, Copy, Debug, Default)]
pub struct KirbyNeedles {
    /// Fighter +22E0 (u.kb.xB4): needles charged (0..=6).
    pub count: i32,
    /// Fighter +22E4 (u.kb.xB8): the bundle in Kirby's hand (its item).
    pub held: bool,
    /// mv x0: frames since the throw began.
    pub throw_frames: i32,
    /// mv x4: a throw frame arrived; accessory4 fires on it.
    pub throw_pending: bool,
    /// mv x8: frames into the current charge cycle.
    pub cycle_frames: i32,
}

fn needles(f: &mut Fighter) -> &mut KirbyNeedles {
    &mut f.character.get_mut::<Kirby>().needles
}
fn attributes(f: &Fighter) -> &NeedleStormAttributes {
    &f.character.get::<Kirby>().attributes.needle_storm
}

/// u.kb.xB4 written: the charge colour follows it.
fn set_count(f: &mut Fighter, count: i32) {
    needles(f).count = count;
    crate::charge_flash::refresh(f);
}

/// ftKb_SpecialN_set_cbs: take_dmg_cb and death2_cb.
fn set_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Kirby>().damage_callbacks = true;
}

fn play(f: &mut Fighter, id: u32) {
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ftKb_SpecialNSk_8010612C / 801061E4: the start, the throw flag and
/// command variables cleared, at least one needle, the scratch reset, the
/// callbacks, then ftAnim_8006EBA4.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.change_motion_state(if air { AIR_START } else { GROUND_START }, a)
        .expect("copied Needle Storm assets");
    f.commands.clear_throw_flags();
    f.commands.variables = [0; 4];
    needles(f).throw_frames = 0;
    if needles(f).count == 0 {
        set_count(f, 1);
    }
    let n = needles(f);
    n.throw_pending = false;
    n.cycle_frames = 0;
    set_callbacks(f);
    f.step_animation(a);
}

/// ftKb_SkSpecialNStart_Anim / ftKb_SkSpecialAirNStart_Anim: at the
/// start's end the bundle in the right hand (it_802B19AC), then the loop.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let part = p.assets.parts.joint(BUNDLE_PART).expect("RHandN part");
        // Item_InitSpawnOnPlane at Kirby's position.
        let spawn = SpawnItem::ray(
            ItemKind::KirbySeakNeedleHeld,
            f.player.id,
            f.physics.position,
            f.physics.facing,
        );
        f.core.item_requests.push(ItemRequest::SpawnInHand {
            spawn,
            part,
            hold: false,
            catch_item: false,
            scale_by_owner: false,
        });
        needles(f).held = true;
        let air = f.motion_state.action == AIR_START;
        f.change_motion_state(if air { AIR_LOOP } else { GROUND_LOOP }, p.assets)?;
        set_callbacks(f);
    }
    Ok(None)
}

/// ftKb_SkSpecialNLoop_Anim / ftKb_SkSpecialAirNLoop_Anim: the charge
/// sound at a cycle's first frame; each time the animation comes back to
/// frame 0 a needle more, up to six (then the colour).
fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if needles(f).cycle_frames == 0 {
        play(f, CHARGE_SOUND);
    }
    needles(f).cycle_frames += 1;
    if f.animation.frame == 0.0 {
        let count = needles(f).count + 1;
        needles(f).cycle_frames = 0;
        if count > MAX_NEEDLES {
            needles(f).cycle_frames = CHARGED_LOOP_FRAMES;
            set_count(f, MAX_NEEDLES);
            f.core.install_color_overlay_now(FULL_CHARGE_COLOR, p.assets);
        } else {
            set_count(f, count);
        }
    }
    Ok(None)
}

/// ftKb_SkSpecialNCancel_Anim / ftKb_SkSpecialAirNCancel_Anim: the bundle
/// goes; at the end Wait, or (in the air) Fall or the special fall.
fn cancel_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    needles(f).held = false;
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_SkSpecialNEnd_Anim / ftKb_SkSpecialAirNEnd_Anim: each throw frame
/// arms a throw and lets go of the bundle.
fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let n = needles(f);
    if THROW_FRAMES.contains(&n.throw_frames) {
        n.throw_pending = true;
        n.held = false;
    }
    n.throw_frames += 1;
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, p.assets)?;
    }
    Ok(None)
}

/// ft_8008A2BC on the ground; in the air `fall`.
fn finish(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if matches!(f.motion_state.action, AIR_CANCEL | AIR_END) {
        fall(f, assets)
    } else {
        common::finish(f, false, assets)
    }
}

/// ftCo_Fall_Enter, or ftCo_80096900(gobj, 1, 0, 1, 1.0, +2A0) when the
/// attribute is set.
fn fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let lag = attributes(f).landing_lag;
    if lag == 0.0 {
        common::finish(f, true, assets)
    } else {
        f.enter_special_fall(assets, true, false, true, 1.0, lag)
    }
}

/// ftKb_SkSpecialNLoop_IASA / ftKb_SkSpecialAirNLoop_IASA: B let go
/// throws (accessory4 = fn_80106DB0); otherwise HSD_PAD_LR cancels.
fn loop_input(f: &mut Fighter, p: InputPhase<'_>) {
    let air = f.motion_state.action == AIR_LOOP;
    if !f.input.current.held.intersects(Buttons::B) {
        needles(f).throw_frames = 0;
        f.change_motion_state(if air { AIR_END } else { GROUND_END }, p.assets)
            .expect("copied Needle Storm throw");
        set_callbacks(f);
        f.character.get_mut::<Kirby>().accessory = Accessory::ThrowNeedles;
        f.core.arm_accessory4();
    } else if f.input.pressed.intersects(Buttons::SHIELD) {
        f.change_motion_state(if air { AIR_CANCEL } else { GROUND_CANCEL }, p.assets)
            .expect("copied Needle Storm cancel");
        set_callbacks(f);
    }
}

/// ft_80084F3C.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
/// ft_80084EEC.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

/// ftKb_SkSpecialNStart_Coll / ftKb_SkSpecialNLoop_Coll: off the floor,
/// ftCommon_GroundToAirStateChange to the aerial row, then the callbacks.
fn charge_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("copied Needle Storm collision assets");
    let state = if f.motion_state.action == GROUND_START {
        AIR_START
    } else {
        AIR_LOOP
    };
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    set_callbacks(f);
    Ok(())
}

/// ftKb_SkSpecialNCancel_Coll / ftKb_SkSpecialNEnd_Coll: off the floor
/// (the throw first drops the charge and its pending throw), Fall or the
/// special fall.
fn release_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("copied Needle Storm collision assets");
    if f.motion_state.action == GROUND_END {
        set_count(f, 0);
        needles(f).throw_pending = false;
    }
    fall(f, assets)
}

/// ftKb_SkSpecialAirNStart_Coll / ftKb_SkSpecialAirNLoop_Coll: on landing,
/// ftCommon_AirToGroundStateChange to the grounded row, then the callbacks.
fn charge_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("copied Needle Storm landing assets");
    let state = if f.motion_state.action == AIR_START {
        GROUND_START
    } else {
        GROUND_LOOP
    };
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, GROUND_AIR_FLAGS, frame, 1.0)?;
    set_callbacks(f);
    Ok(())
}

/// ftKb_SkSpecialAirNCancel_Coll / ftKb_SkSpecialAirNEnd_Coll: landing
/// (ftCo_Landing_Enter_Basic); the throw first drops the charge and its
/// pending throw.
fn release_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("copied Needle Storm landing assets");
    if f.motion_state.action == AIR_END {
        set_count(f, 0);
        needles(f).throw_pending = false;
    }
    f.enter_landing(assets)
}

/// fn_80106DB0 (80106DB0), accessory4 of the throw: on a pending throw,
/// while Kirby has a needle, one thrown ahead from a random height:
/// grounded +290 ahead (fmuls, then fmadds by the scale, 80106E30) at +294
/// plus the offset (fadds, fmadds 80106E60); in the air +298 ahead at
/// twice the offset plus +29C (fmadds 80106EB4, 80106EB8). Then the flash
/// and the sound.
pub fn throw_needle(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    if !std::mem::take(&mut needles(f).throw_pending) {
        return;
    }
    if needles(f).count <= 0 {
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
    let count = needles(f).count - 1;
    set_count(f, count);
    f.effects.push(EffectRequest::PositionalGenerator {
        id: THROW_FLASH,
        position,
    });
    play(f, THROW_SOUND);
}

/// it_802AFD8C(gobj, &pos, It_Kind_Kirby_SeakNeedleThrow, facing): a
/// needle at `point` leaving from Kirby's ECB centre (it_8026BB68), then
/// it_802AFEA8's launch.
fn spawn_needle(f: &mut Fighter, point: Vec3, launch: Launch) {
    let origin = common::ecb_centre(f);
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(
        ItemKind::KirbySeakNeedleThrow,
        c.player.id,
        point,
        c.physics.facing,
    );
    spawn.position = origin;
    spawn.spawn_argument = launch.argument();
    spawn.launch_draws = launch.draws();
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// ftKb_SpecialNSk_8010603C (8010603C), Sheik's arm of 800F19AC and
/// 800F1A8C: while Kirby holds the bundle it goes and every charged needle
/// drops from him (+294 on the ground, +29C in the air, times his scale:
/// 801060C4 fmadds); a charge he holds or not is then spent. Unlike
/// Sheik's ftSk_SpecialN_80111FBC it leaves the callbacks alone.
pub fn put_away(f: &mut Fighter) {
    if std::mem::take(&mut needles(f).held) {
        while needles(f).count != 0 {
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
            let count = needles(f).count - 1;
            set_count(f, count);
        }
    }
    set_count(f, 0);
}

/// ftKb_SpecialNSk_80105FF0: what the bundle reads of its owner (the
/// hand still holds it).
pub fn holds_bundle(f: &Fighter) -> bool {
    let kirby = f.character.get::<Kirby>();
    kirby.copied == FighterKind::Seak && kirby.needles.held
}

/// Rows 467..474 (ftKb_SkSpecial*N*).
pub const fn rows() -> [MotionRow; 8] {
    use melee_ft::fighter::state::{AnimFn, CollisionFn, InputFn};
    let table: [(ActionId, AnimFn, InputFn, bool, CollisionFn); 8] = [
        (GROUND_START, start_anim, common::no_input, false, charge_ground_collision),
        (GROUND_LOOP, loop_anim, loop_input, false, charge_ground_collision),
        (GROUND_CANCEL, cancel_anim, common::no_input, false, release_ground_collision),
        (GROUND_END, end_anim, common::no_input, false, release_ground_collision),
        (AIR_START, start_anim, common::no_input, true, charge_air_collision),
        (AIR_LOOP, loop_anim, loop_input, true, charge_air_collision),
        (AIR_CANCEL, cancel_anim, common::no_input, true, release_air_collision),
        (AIR_END, end_anim, common::no_input, true, release_air_collision),
    ];
    let mut rows = [common::row(
        GROUND_START,
        FIRST_ANIMATION,
        start_anim,
        common::no_input,
        ground_physics,
        charge_ground_collision,
    ); 8];
    let mut i = 0;
    while i < 8 {
        let (action, anim, iasa, air, collision) = table[i];
        let physics: melee_ft::fighter::state::PhysicsFn =
            if air { air_physics } else { ground_physics };
        rows[i] = common::row(
            action,
            FIRST_ANIMATION + i as i32,
            anim,
            iasa,
            physics,
            collision,
        );
        i += 1;
    }
    rows
}
