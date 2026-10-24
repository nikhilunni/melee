//! Ice Shot, ftpopospecialn.c (8011F2A4..8011F6B4), which both climbers
//! run, and Kirby's copy of it (ftkirbyspecialiceclimber.c,
//! 80108CE8..80109238): the same states over either user's rows, generic
//! over [`IceShotUser`].
//!
//! The script sets cmd_vars[0] = 1 to make an ice block above the head
//! (it_802C1590) and cmd_vars[0] = 2 to launch it (it_802C16F8); accessory4
//! (ftPp_SpecialN_8011F500, Kirby's fn_801090D4) carries both out. Leaving
//! the ground or landing first breaks a block still held.
use crate::common;
use hsd_archive::Reader;
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    desc::FighterDescError,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, MotionRow},
        ActionId, Fighter,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// ft_PlaySFX(fp, 130021, 127, 64): the block forms
/// (retail 8011F594..9C: lis 2, subi 0x41B; Kirby's 0x1FBE5).
const MAKE_SOUND: u32 = 130_021;
/// ft_PlaySFX(fp, 130024, 127, 64): the block leaves (Kirby's 0x1FBE8).
const LAUNCH_SOUND: u32 = 130_024;
/// The aerial entries: a second aerial Ice Shot before landing makes its
/// block 10 lower (x2250 / u.kb.xC8 = -10.0).
const REPEAT_AIR_DROP: f32 = -10.0;

/// The script's cmd_vars[0] commands.
const MAKE: u32 = 1;
const LAUNCH: u32 = 2;

/// The Ice Shot's attributes: ftPopo_DatAttrs +0x04..+0x10 and
/// ftKb_DatAttrs +39C..+3A8 (`specialn_pp_*`), the same four words in the
/// same order.
#[derive(Clone, Debug, PartialEq)]
pub struct IceShotAttributes {
    /// +0x0: the lift of a first aerial Ice Shot.
    pub air_lift: f32,
    /// +0x4: an aerial Ice Shot's landing lag.
    pub landing_lag: f32,
    /// +0x8 / +0xC: where the ice block forms, along the facing and above
    /// TopN.
    pub ice_reach: f32,
    pub ice_height: f32,
}
impl IceShotAttributes {
    /// The four words at `base`.
    pub fn read(r: &Reader<'_>, base: u32) -> std::result::Result<Self, FighterDescError> {
        Ok(Self {
            air_lift: r.f32(base)?,
            landing_lag: r.f32(base + 0x4)?,
            ice_reach: r.f32(base + 0x8)?,
            ice_height: r.f32(base + 0xC)?,
        })
    }
}

/// What the Ice Shot's states ask of their user.
pub trait IceShotUser {
    /// The grounded and aerial rows.
    const GROUND: ActionId;
    const AIR: ActionId;
    /// The grounded row's animation; the aerial row plays the next.
    const GROUND_ANIMATION: i32;
    /// The block's item kind.
    const ITEM: ItemKind;
    fn attributes(f: &Fighter) -> &IceShotAttributes;
    /// The block the user holds (Popo's x222C, Kirby's u.kb.xC0).
    fn held(f: &mut Fighter) -> &mut bool;
    /// A first aerial Ice Shot lifted the user since it last landed
    /// (x224C, u.kb.xC4).
    fn lift_used(f: &mut Fighter) -> &mut bool;
    /// How far below the usual height the block forms (x2250, u.kb.xC8).
    fn drop(f: &mut Fighter) -> &mut f32;
    /// accessory4_cb = the script's commands.
    fn install_accessory(f: &mut Fighter);
    /// The block is made: held, with the user's take-damage and death
    /// callbacks (Popo's ftPp_Init_8011F060, Kirby's ftKb_SpecialN_set_cbs).
    fn hold(f: &mut Fighter);
    /// The user lets go of the block (Popo's ftPp_Init_8011F16C also drops
    /// its callbacks; Kirby's code clears u.kb.xC0 alone).
    fn let_go(f: &mut Fighter);
    /// ft_800881D8's voice at the launch.
    fn launch_voice(f: &Fighter) -> u32;
}

/// The grounded and aerial rows.
pub const fn rows<C: IceShotUser>() -> [MotionRow; 2] {
    [
        row(
            C::GROUND,
            C::GROUND_ANIMATION,
            anim::<false>,
            callbacks::physics::guard_on,
            ground_collision::<C>,
        ),
        row(
            C::AIR,
            C::GROUND_ANIMATION + 1,
            anim::<true>,
            callbacks::physics::air_friction,
            air_collision::<C>,
        ),
    ]
}

const fn row(
    action: ActionId,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim,
        iasa: common::no_input,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// ftPp_SpecialN_Enter (8011F2A4) / ftPp_SpecialAirN_Enter (8011F318),
/// Kirby's ftKb_SpecialNIc_80108D64 / 80108E14. A first aerial Ice Shot
/// lifts the user (attribute +0) until it lands. (Kirby's entries also
/// hang the copy's model on his left thumb, ftCommon_SetAccessory: drawn
/// only.)
pub fn enter<C: IceShotUser>(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    *C::held(f) = false;
    if airborne {
        let lift = C::attributes(f).air_lift;
        if !*C::lift_used(f) {
            *C::lift_used(f) = true;
            *C::drop(f) = 0.0;
            f.physics.self_velocity.y = lift;
        } else {
            *C::drop(f) = REPEAT_AIR_DROP;
        }
    }
    let state = if airborne { C::AIR } else { C::GROUND };
    f.change_motion_state(state, assets)
        .expect("Ice Shot assets");
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    C::install_accessory(f);
}

/// ftPp_SpecialN_Anim (8011F3CC) / ftPp_SpecialAirN_Anim (8011F408),
/// ftKb_PpSpecial(Air)N_Anim: Wait or Fall at the animation's end.
fn anim<const AIR: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, p.assets, AIR)?;
    }
    Ok(None)
}

/// ftPp_SpecialN_Coll (8011F48C) / ftKb_PpSpecialN_Coll: off the floor, a
/// held block breaks and the user falls.
fn ground_collision<C: IceShotUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    break_held_ice::<C>(f);
    let assets = p.assets.expect("Ice Shot collision assets");
    f.change_motion_state(CommonMotionState::Fall.into(), assets)
}

/// ftPp_SpecialAirN_Coll (8011F4E4) / ftKb_PpSpecialAirN_Coll: on landing
/// a held block breaks, the aerial lift is spent again and
/// LandingFallSpecial runs attribute +4's lag.
fn air_collision<C: IceShotUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    break_held_ice::<C>(f);
    *C::lift_used(f) = false;
    *C::drop(f) = 0.0;
    let lag = C::attributes(f).landing_lag;
    let assets = p.assets.expect("Ice Shot landing assets");
    f.enter_special_landing(assets, false, lag)
}

/// ftPp_Init_8011F190 (8011F190) / ftKb_SpecialNIc_80108D04 (80108D04): a
/// held block breaks (it_802C17DC), and the user lets go of it.
pub fn break_held_ice<C: IceShotUser>(f: &mut Fighter) {
    if !*C::held(f) {
        return;
    }
    request_ice::<C>(f, ItemControl::Remove);
    C::let_go(f);
}

fn request_ice<C: IceShotUser>(f: &mut Fighter, control: ItemControl) {
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind: C::ITEM,
        control,
    });
}

/// ftPp_SpecialN_8011F500 (8011F500) / fn_801090D4 (801090D4): the
/// script's two commands.
pub fn accessory<C: IceShotUser>(f: &mut Fighter, assets: &FighterAssets) {
    let command = f.commands.variables[0];
    match command {
        MAKE => make_ice::<C>(f, assets),
        LAUNCH if *C::held(f) => launch_ice::<C>(f),
        _ => {}
    }
}

/// The block at TopN, attribute +8 along the facing (retail 8011F568 /
/// 80109140: fmadds) and +C plus the drop above it (two fadds), from
/// it_802C1590; its sound; the user holds it.
fn make_ice<C: IceShotUser>(f: &mut Fighter, assets: &FighterAssets) {
    let bone = usize::from(assets.parts.joint(FtPart::TopN).expect("TopN part"));
    let (reach, height) = {
        let a = C::attributes(f);
        (a.ice_reach, a.ice_height)
    };
    let extra_height = *C::drop(f);
    let c = &mut f.core;
    // lb_8000B1CC(parts[0].joint, NULL, &pos).
    let mut position =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, bone, Vec3::ZERO);
    position.x = gekko_math::fma::fmadds(reach, c.physics.facing, position.x);
    position.y += height + extra_height;
    // it_802C1590: prev_pos on the stage plane; pos is it_8026BB68's ECB
    // midpoint (ftLib_80086990, retail 800869AC..BC: fadds, fmuls, fadds).
    let mut spawn = SpawnItem::ray(C::ITEM, c.player.id, position, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    common::play_sound(f, MAKE_SOUND);
    C::hold(f);
    f.commands.variables[0] = 0;
}

/// cmd_vars[0] == 2 with a block: it_802C16F8, the launch voice and sound,
/// and the user lets go of it.
fn launch_ice<C: IceShotUser>(f: &mut Fighter) {
    request_ice::<C>(f, ItemControl::Fire);
    f.commands.variables[0] = 0;
    let voice = C::launch_voice(f);
    common::play_voice(f, voice);
    common::play_sound(f, LAUNCH_SOUND);
    C::let_go(f);
}
