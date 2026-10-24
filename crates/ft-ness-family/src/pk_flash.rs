//! PK Flash, ftnessspecialn.c (80116E68..80117B70), and Kirby's copy of it
//! (ftkirbyspecialness.c, 800FEC78..800FF8EC): the same states over either
//! user's rows, generic over [`PkFlashUser`].
//!
//! The start (row 0 / 4) ends by sending the flash up from the user's
//! joint and entering the hold (1 / 5), where the flash reads the stick.
//! Letting go of B after the release delay drops the hold on it (it bursts
//! on its own); with no flash the release row (2 / 6) plays out the loop
//! counters and the end row (3 / 7) follows. A flash that met a surface
//! moves the hold to the release row at the same frame.
use hsd_archive::Reader;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    desc::FighterDescError,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
    physics::{airborne, friction},
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, GroundOrAir, ItemKind};

/// The rows from the grounded start: start, hold, release, end; the aerial
/// rows follow, four on.
const HOLD_OFFSET: u16 = 1;
const RELEASE_OFFSET: u16 = 2;
const END_OFFSET: u16 = 3;
const AIR_OFFSET: u16 = 4;
const ROWS: usize = 8;

/// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12): SkipMatAnim |
/// SkipColAnim | UpdateCmd | SkipItemVis | Unk19 | SkipModelPartVis |
/// SkipModelFlags | Unk27 (both users' collision callbacks: 0x0C4C5080).
const GROUND_AIR: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);

/// The flash leaves three units (times the fighter's Y scale) above the
/// joint (retail 0x8011716C, Kirby's 0x800FEF00: fmadds).
const JOINT_RISE: f32 = 3.0;
/// `fp->parts[0]`: the model root, whose X rotation the end resets.
const ROOT_PART: usize = 0;

/// The PK Flash block of the user's attributes (ftNessAttributes +0..+1C,
/// ftKb_DatAttrs +1A0..+1BC `specialn_ns_*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PkFlashAttributes {
    /// +0x00 x0_PKFLASH_TIMER1_LOOPFRAMES: frames the user holds the loop
    /// on the ground once the flash is gone.
    pub ground_loop_frames: i32,
    /// +0x04 x4_PKFLASH_TIMER2_LOOPFRAMES: the same in the air.
    pub air_loop_frames: i32,
    /// +0x08 x8_PKFLASH_GRAVITY_DELAY: frames before the aerial move falls.
    pub gravity_delay: i32,
    /// +0x0C xC_PKFLASH_MINCHARGEFRAMES: frames before a released B
    /// detonates the flash.
    pub release_delay: i32,
    /// +0x10 x10_PKFLASH_UNK1.
    // TODO(meaning): no retail reader found.
    pub unknown_10: f32,
    /// +0x14 x14_PKFLASH_FALL_ACCEL.
    pub fall_acceleration: f32,
    /// +0x18 x18_PKFLASH_UNK2.
    // TODO(meaning): no retail reader found.
    pub unknown_18: f32,
    /// +0x1C x1C_PKFLASH_LANDING_LAG (0: plain fall).
    pub landing_lag: f32,
}

impl PkFlashAttributes {
    /// The block at `base`.
    pub fn read(r: &Reader<'_>, base: u32) -> std::result::Result<Self, FighterDescError> {
        Ok(Self {
            ground_loop_frames: r.s32(base)?,
            air_loop_frames: r.s32(base + 0x04)?,
            gravity_delay: r.s32(base + 0x08)?,
            release_delay: r.s32(base + 0x0C)?,
            unknown_10: r.f32(base + 0x10)?,
            fall_acceleration: r.f32(base + 0x14)?,
            unknown_18: r.f32(base + 0x18)?,
            landing_lag: r.f32(base + 0x1C)?,
        })
    }
}

/// mv.ns.specialn / mv.kb.specialn_ns and the user's hold on the flash
/// (u.ns.pkflash_gobj, u.kb.ns_flash_gobj).
#[derive(Clone, Copy, Debug, Default)]
pub struct PkFlash {
    /// +2340 frames_to_loop_charge_ground: counts down from the hold's start.
    pub loop_frames: i32,
    /// +2344 frames_to_loop_charge_air: counts down once the flash is gone.
    pub gone_frames: i32,
    /// +2348 falling_acceleration_delay: aerial frames before gravity.
    pub gravity_delay: i32,
    /// +234C charge_release_delay: frames before a released B drops the
    /// flash.
    pub release_delay: i32,
    /// The flash pointer: the flash is out and still the user's.
    pub flash_out: bool,
}

/// What PK Flash's states ask of their user.
pub trait PkFlashUser {
    /// The grounded start; the hold, release and end rows follow, then the
    /// aerial four.
    const START: ActionId;
    /// The grounded start's animation; the other seven rows play the next
    /// ones in order.
    const START_ANIMATION: i32;
    /// The part whose joint the flash leaves from (`fp->parts[part]`, the
    /// enum indexing the parts array directly).
    const JOINT: FtPart;
    /// The flash's item kind.
    const FLASH: ItemKind;
    /// Whether dropping the flash and the counters' reset clear take_dmg_cb
    /// and death2_cb as well: Ness's ftNs_SpecialN_SetNULL and
    /// SetPKFlashAttr do; Kirby's 800FECBC and his inlined reset only clear
    /// the pointer (800FF444..48, 800FF0CC).
    const CLEARS_CALLBACKS: bool;
    /// The PK Flash attributes.
    fn attributes(f: &Fighter) -> PkFlashAttributes;
    /// The scratch.
    fn scratch(f: &mut Fighter) -> &mut PkFlash;
    /// The scratch, to read.
    fn state(f: &Fighter) -> &PkFlash;
    /// take_dmg_cb and death2_cb in (`true`, with a new flash) or out.
    fn set_damage_callbacks(f: &mut Fighter, installed: bool);
}

const fn action<C: PkFlashUser>(offset: u16) -> ActionId {
    ActionId(C::START.0 + offset)
}

/// The eight rows, grounded then aerial.
pub const fn rows<C: PkFlashUser>() -> [MotionRow; ROWS] {
    // The rows play the start's animation and the seven after it, in
    // order (`row`).
    [
        row::<C>(0, start_anim::<C>, no_input, start_ground_physics::<C>, ground_collision::<C>),
        row::<C>(HOLD_OFFSET, hold_anim::<C>, hold_input::<C>, ground_friction, ground_collision::<C>),
        row::<C>(RELEASE_OFFSET, hold_anim::<C>, hold_input::<C>, ground_friction, ground_collision::<C>),
        row::<C>(END_OFFSET, end_anim::<C>, no_input, ground_friction, ground_collision::<C>),
        row::<C>(AIR_OFFSET, start_anim::<C>, no_input, air_physics::<C>, air_collision::<C>),
        row::<C>(AIR_OFFSET + HOLD_OFFSET, hold_anim::<C>, hold_input::<C>, air_physics::<C>, air_collision::<C>),
        row::<C>(AIR_OFFSET + RELEASE_OFFSET, hold_anim::<C>, hold_input::<C>, air_physics::<C>, air_collision::<C>),
        row::<C>(AIR_OFFSET + END_OFFSET, end_anim::<C>, no_input, air_physics::<C>, air_collision::<C>),
    ]
}

const fn row<C: PkFlashUser>(
    offset: u16,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: action::<C>(offset),
        id: CommonMotionState::None,
        animation: C::START_ANIMATION + offset as i32,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// Whether the current row is one of the aerial ones.
fn airborne_row<C: PkFlashUser>(f: &Fighter) -> bool {
    f.motion_state.action.0 >= action::<C>(AIR_OFFSET).0
}

/// The row `offset` on the current row's side.
fn placed<C: PkFlashUser>(f: &Fighter, offset: u16) -> ActionId {
    if airborne_row::<C>(f) {
        action::<C>(AIR_OFFSET + offset)
    } else {
        action::<C>(offset)
    }
}

/// The entries' and the end's reset (SetPKFlashAttr and Kirby's copies of
/// it): the counters from the attributes and no flash.
fn reset<C: PkFlashUser>(f: &mut Fighter) {
    let a = C::attributes(f);
    *C::scratch(f) = PkFlash {
        loop_frames: a.ground_loop_frames,
        gone_frames: a.air_loop_frames,
        gravity_delay: a.gravity_delay,
        release_delay: a.release_delay,
        flash_out: false,
    };
    if C::CLEARS_CALLBACKS {
        C::set_damage_callbacks(f, false);
    }
}

/// ftNs_SpecialNStart_Enter (80116F94) / ftNs_SpecialAirNStart_Enter
/// (80117034); Kirby's ftKb_SpecialNNs_800FED38 / 800FEDD0: the aerial
/// entry stops the fall.
pub fn enter<C: PkFlashUser>(f: &mut Fighter, air_entry: bool, a: &FighterAssets) {
    let start = if air_entry {
        action::<C>(AIR_OFFSET)
    } else {
        C::START
    };
    f.change_motion_state(start, a).expect("PK Flash assets");
    f.commands.variables = [0; 4];
    if air_entry {
        f.physics.self_velocity.y = 0.0;
    }
    reset::<C>(f);
    f.step_animation(a);
}

/// The flash from `fp->parts[JOINT]`, three units up, on the stage plane
/// (it_802AA8C0: the ray from the user's ECB centre), with the user's
/// callbacks.
fn spawn_flash<C: PkFlashUser>(f: &mut Fighter) {
    if C::scratch(f).flash_out {
        return;
    }
    let c = &mut f.core;
    let mut joint = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        i32::from(C::JOINT) as usize,
        hsd_types::Vec3::ZERO,
    );
    joint.z = 0.0;
    // fp->x34_scale.y (lfs 0x38(fp)): the player's scale alone, without
    // the model scaling (Kirby's is not 1).
    joint.y = gekko_math::fma::fmadds(JOINT_RISE, c.player.scale, joint.y);
    let mut spawn = SpawnItem::ray(C::FLASH, c.player.id, joint, c.physics.facing);
    // it_8026BB68 -> ftLib_80086990: the ECB centre (fadds, fmuls, fadds).
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = hsd_types::Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    C::scratch(f).flash_out = true;
    C::set_damage_callbacks(f, true);
}

/// ftNs_SpecialNStart_Anim (801170DC) / ftNs_SpecialAirNStart_Anim
/// (80117378); Kirby's 800FEE70 / 800FF10C: at the end the hold, the
/// flash, and every jump spent.
fn start_anim<C: PkFlashUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = placed::<C>(f, HOLD_OFFSET);
        f.change_motion_state(state, p.assets)?;
        spawn_flash::<C>(f);
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    }
    Ok(None)
}

/// ftNs_SpecialNRelease_Anim (801171BC) / ftNs_SpecialAirNRelease_Anim
/// (80117458); Kirby's ftKb_NsSpecial(Air)NHold_Anim (800FEF58, 800FF1F4),
/// the hold and release rows: the counters; with no flash the end once both
/// run out, else the release row at the same frame; a flash no longer the
/// user's is forgotten; one a surface burst (it_802AA7F0) moves the hold to
/// the release row.
fn hold_anim<C: PkFlashUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let s = C::scratch(f);
    if s.loop_frames != 0 {
        s.loop_frames -= 1;
    }
    if !s.flash_out && s.gone_frames != 0 {
        s.gone_frames -= 1;
    }
    let s = *s;
    let release = placed::<C>(f, RELEASE_OFFSET);
    let frame = f.animation.frame;
    if !s.flash_out {
        if s.loop_frames <= 0 && s.gone_frames <= 0 {
            let end = placed::<C>(f, END_OFFSET);
            f.change_motion_state(end, p.assets)?;
        } else if f.motion_state.action != release {
            f.change_motion_state_at(release, p.assets, frame)?;
        }
        return Ok(None);
    }
    match f.core.owned_article {
        // it_802AA7E4(flash) != gobj.
        None => C::scratch(f).flash_out = false,
        Some(report) => {
            if report.struck && f.motion_state.action != release {
                f.change_motion_state_at(release, p.assets, frame)?;
            }
        }
    }
    Ok(None)
}

/// ftNs_SpecialNEnd_Anim (801172F0) / ftNs_SpecialAirNEnd_Anim (8011758C);
/// Kirby's 800FF08C / 800FF328: the reset and the model root's X rotation
/// back to zero, every frame; at the end Wait, or Fall (a special fall
/// with x1C of landing lag when it is not zero: ftCo_80096900(gobj, 1, 0,
/// 1, 1.0, x1C)).
fn end_anim<C: PkFlashUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    reset::<C>(f);
    f.core.set_part_rotation(ROOT_PART, Axis::X, 0.0);
    if !f.animation.frames_remaining(&f.skeleton) {
        if airborne_row::<C>(f) {
            let lag = C::attributes(f).landing_lag;
            if lag == 0.0 {
                fall(f, p.assets)?;
            } else {
                f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
            }
        } else {
            f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
        }
    }
    Ok(None)
}

/// The empty IASA callbacks of the start and end rows.
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftNs_SpecialNRelease_IASA (8011764C) / ftNs_SpecialAirNRelease_IASA
/// (801176D0); Kirby's ftKb_NsSpecial(Air)NHold_IASA (800FF3E0,
/// 800FF458): once the release delay has run out, B up drops the hold on
/// the flash.
fn hold_input<C: PkFlashUser>(f: &mut Fighter, _: InputPhase<'_>) {
    let s = C::scratch(f);
    s.release_delay -= 1;
    if s.release_delay > 0 {
        return;
    }
    s.release_delay = 0;
    if !f.input.current.held.intersects(Buttons::B) {
        flash_gone::<C>(f);
    }
}

/// ft_80084F3C (80084F3C): ground friction and ftCommon_ApplyGroundMovement.
fn ground_friction(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftNs_SpecialNStart_Phys (80117750) / Kirby's 800FF4CC: the gravity
/// delay counts down, then ft_80084F3C.
fn start_ground_physics<C: PkFlashUser>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = C::scratch(f);
    if s.gravity_delay != 0 {
        s.gravity_delay -= 1;
    }
    ground_friction(f, p);
}

/// ftNs_SpecialAirN{Start,Release,End}_Phys (801177C8, 80117828,
/// 80117888); Kirby's 800FF544 and its two callers: the gravity delay,
/// then ftCommon_Fall at x14 to the fighter's terminal velocity;
/// ftCommon_ApplyFrictionAir with the aerial friction.
fn air_physics<C: PkFlashUser>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = C::scratch(f);
    if s.gravity_delay != 0 {
        s.gravity_delay -= 1;
    } else {
        let gravity = C::attributes(f).fall_acceleration;
        let terminal = f.attributes.air.terminal_velocity;
        f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
    }
    let aerial = f.attributes.air.aerial_friction;
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, aerial);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftNs_SpecialN{Start,Release,End}_Coll (801178E8, 80117954, 801179C0);
/// Kirby's 800FF664, 800FF6D0, 800FF73C: ft_80082708; off the floor the
/// aerial counterpart at the same frame (ftCommon_GroundToAirStateChange).
/// Both the hold and the release row continue in the aerial hold.
fn ground_collision<C: PkFlashUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
    let assets = p.assets.expect("PK Flash collision assets");
    let offset = match f.motion_state.action.0 - C::START.0 {
        0 => 0,
        HOLD_OFFSET | RELEASE_OFFSET => HOLD_OFFSET,
        _ => END_OFFSET,
    };
    // ftCommon_8007D5D4, then the aerial row at the current frame.
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(action::<C>(AIR_OFFSET + offset), assets, GROUND_AIR, frame, 1.0)
}

/// ftNs_SpecialAirN{Start,Release,End}_Coll (80117A2C, 80117A98,
/// 80117B04); Kirby's 800FF7A8, 800FF814, 800FF880: ft_80081D0C; a landing
/// continues in the grounded counterpart (ftCommon_AirToGroundStateChange),
/// the aerial hold and release rows both in the grounded hold.
fn air_collision<C: PkFlashUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
    let assets = p.assets.expect("PK Flash landing assets");
    let offset = match f.motion_state.action.0 - action::<C>(AIR_OFFSET).0 {
        0 => 0,
        HOLD_OFFSET | RELEASE_OFFSET => HOLD_OFFSET,
        _ => END_OFFSET,
    };
    // ftCommon_8007D7FC, then the grounded row at the current frame.
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(action::<C>(offset), assets, GROUND_AIR, frame, 1.0)
}

/// ftCo_Fall_Enter: Fall, leaving the ground first when grounded.
fn fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        f.leave_ground();
    }
    f.change_motion_state(CommonMotionState::Fall.into(), assets)
}

/// The user's take-damage or death callback with the flash out
/// (ftNs_SpecialN_ItemPKFlushSetNULL, 80116F00; Kirby's
/// ftKb_SpecialNNs_800FECE8): the flash flies on without its user
/// (it_802AAA50) and the pointer clears (Ness's callbacks too).
pub fn orphan_flash<C: PkFlashUser>(f: &mut Fighter) {
    if C::scratch(f).flash_out {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: C::FLASH,
            control: ItemControl::Orphan,
        });
    }
    flash_gone::<C>(f);
}

/// ftNs_SpecialN_SetNULL (80116EBC) / ftKb_SpecialNNs_800FECBC (800FECBC):
/// the flash is no longer the user's (the IASA's B up and the flash's
/// Destroyed callback); Ness's take_dmg_cb and death2_cb clear too.
pub fn flash_gone<C: PkFlashUser>(f: &mut Fighter) {
    C::scratch(f).flash_out = false;
    if C::CLEARS_CALLBACKS {
        C::set_damage_callbacks(f, false);
    }
}

/// ftNs_SpecialN_CheckSpecialNHold (80116E68) / ftKb_SpecialNNs_800FEC78
/// (800FEC78), which the flash reads: the user still holds it in the hold
/// row.
pub fn holding<C: PkFlashUser>(f: &Fighter) -> bool {
    let current = f.motion_state.action;
    C::state(f).flash_out
        && (current == action::<C>(HOLD_OFFSET) || current == action::<C>(AIR_OFFSET + HOLD_OFFSET))
}
