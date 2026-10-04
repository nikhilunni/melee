//! PK Flash, ftnessspecialn.c (80116E68..80117B70).
//!
//! The start (348 / 352) ends by sending the flash up from Ness's hand and
//! entering the hold (349 / 353), where the flash reads his stick. Letting
//! go of B after xC frames drops his hold on it (it bursts on its own);
//! with no flash the release row (350 / 354) plays out the loop counters
//! and the end row (351 / 355) follows. A flash that met a surface moves
//! the hold to the release row at the same frame.
use crate::{
    common::{self, row},
    init::Ness,
};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
    input::Buttons,
    physics::friction,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// ftNs_MS_SpecialNStart..SpecialNEnd (348..351) and the aerial rows
/// (352..355), four rows on.
pub const START: ActionId = ActionId(348);
pub const HOLD: ActionId = ActionId(349);
pub const RELEASE: ActionId = ActionId(350);
pub const END: ActionId = ActionId(351);
const AIR_OFFSET: u16 = 4;

/// The flash leaves three units (times the fighter's Y scale) above the
/// hand (retail 0x8011716C: fmadds).
const HAND_RISE: f32 = 3.0;
/// `fp->parts[0]`: the model root, whose X rotation the end resets.
const ROOT_PART: usize = 0;

/// mv.ns.specialn and Ness's hold on the flash (u.ns.pkflash_gobj).
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
    /// u.ns.pkflash_gobj: his flash is out and still his.
    pub flash_out: bool,
}

pub const fn rows() -> [MotionRow; 8] {
    [
        row(
            START,
            0x12B,
            start_anim,
            common::no_input,
            start_ground_physics,
            ground_collision,
        ),
        row(
            HOLD,
            0x12C,
            hold_anim,
            hold_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            RELEASE,
            0x12D,
            hold_anim,
            hold_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            END,
            0x12E,
            end_anim,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            air(START),
            0x12F,
            start_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
        row(
            air(HOLD),
            0x130,
            hold_anim,
            hold_input,
            air_physics,
            air_collision,
        ),
        row(
            air(RELEASE),
            0x131,
            hold_anim,
            hold_input,
            air_physics,
            air_collision,
        ),
        row(
            air(END),
            0x132,
            end_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
    ]
}

const fn air(ground: ActionId) -> ActionId {
    ActionId(ground.0 + AIR_OFFSET)
}

/// Whether the current row is one of the aerial ones.
fn airborne_row(f: &Fighter) -> bool {
    f.motion_state.action.0 >= air(START).0
}

/// The row of `ground` on the current row's side.
fn placed(f: &Fighter, ground: ActionId) -> ActionId {
    if airborne_row(f) {
        air(ground)
    } else {
        ground
    }
}

fn scratch(f: &mut Fighter) -> &mut PkFlash {
    &mut f.character.get_mut::<Ness>().pk_flash
}

/// The entry's and the end's reset (SetPKFlashAttr): the counters from the
/// attributes, no flash, no callbacks.
fn reset(f: &mut Fighter) {
    let ness = f.character.get_mut::<Ness>();
    let a = &ness.attributes.pk_flash;
    ness.pk_flash = PkFlash {
        loop_frames: a.ground_loop_frames,
        gone_frames: a.air_loop_frames,
        gravity_delay: a.gravity_delay,
        release_delay: a.release_delay,
        flash_out: false,
    };
    ness.damage_callbacks = false;
}

/// ftNs_SpecialNStart_Enter (80116F94) / ftNs_SpecialAirNStart_Enter
/// (80117034): the aerial entry stops the fall.
pub fn enter(f: &mut Fighter, air_entry: bool, a: &FighterAssets) {
    f.change_motion_state(if air_entry { air(START) } else { START }, a)
        .expect("PK Flash assets");
    f.commands.variables = [0; 4];
    if air_entry {
        f.physics.self_velocity.y = 0.0;
    }
    reset(f);
    f.step_animation(a);
}

/// The flash from fp->parts[FtPart_L2ndNa], three units up, on the stage
/// plane (it_802AA8C0: the ray from Ness's ECB centre), with his
/// callbacks.
fn spawn_flash(f: &mut Fighter) {
    if scratch(f).flash_out {
        return;
    }
    let c = &mut f.core;
    let mut hand = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        common::part(FtPart::L2ndNa),
        hsd_types::Vec3::ZERO,
    );
    hand.z = 0.0;
    let scale = c.player.scale * c.attributes.size.model_scaling;
    hand.y = gekko_math::fma::fmadds(HAND_RISE, scale, hand.y);
    let mut spawn = SpawnItem::ray(ItemKind::NessPKFlush, c.player.id, hand, c.physics.facing);
    // it_8026BB68 -> ftLib_80086990: the ECB centre (fadds, fmuls, fadds).
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = hsd_types::Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::Spawn(spawn));
    scratch(f).flash_out = true;
    common::install_damage_callbacks(f);
}

/// ftNs_SpecialNStart_Anim (801170DC) / ftNs_SpecialAirNStart_Anim
/// (80117378): at the end the hold, the flash, and every jump spent.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = placed(f, HOLD);
        f.change_motion_state(state, p.assets)?;
        spawn_flash(f);
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    }
    Ok(None)
}

/// ftNs_SpecialNRelease_Anim (801171BC) / ftNs_SpecialAirNRelease_Anim
/// (80117458), the hold and release rows: the counters; with no flash the
/// end once both run out, else the release row at the same frame; a flash
/// no longer his is forgotten; one a surface burst (it_802AA7F0) moves the
/// hold to the release row.
fn hold_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let s = scratch(f);
    if s.loop_frames != 0 {
        s.loop_frames -= 1;
    }
    if !s.flash_out && s.gone_frames != 0 {
        s.gone_frames -= 1;
    }
    let s = *s;
    let release = placed(f, RELEASE);
    let frame = f.animation.frame;
    if !s.flash_out {
        if s.loop_frames <= 0 && s.gone_frames <= 0 {
            let end = placed(f, END);
            f.change_motion_state(end, p.assets)?;
        } else if f.motion_state.action != release {
            f.change_motion_state_at(release, p.assets, frame)?;
        }
        return Ok(None);
    }
    match f.core.owned_article {
        // it_802AA7E4(pkflash_gobj) != gobj.
        None => scratch(f).flash_out = false,
        Some(report) => {
            if report.struck && f.motion_state.action != release {
                f.change_motion_state_at(release, p.assets, frame)?;
            }
        }
    }
    Ok(None)
}

/// ftNs_SpecialNEnd_Anim (801172F0) / ftNs_SpecialAirNEnd_Anim (8011758C):
/// the reset and the model root's X rotation back to zero, every frame; at
/// the end Wait, or Fall (a special fall with x1C of landing lag when it is
/// not zero: ftCo_80096900(gobj, 1, 0, 1, 1.0, x1C)).
fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    reset(f);
    f.core.set_part_rotation(ROOT_PART, Axis::X, 0.0);
    if !f.animation.frames_remaining(&f.skeleton) {
        if airborne_row(f) {
            let lag = f.character.get::<Ness>().attributes.pk_flash.landing_lag;
            if lag == 0.0 {
                common::fall(f, p.assets)?;
            } else {
                f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
            }
        } else {
            common::wait(f, p.assets)?;
        }
    }
    Ok(None)
}

/// ftNs_SpecialNRelease_IASA (8011764C) / ftNs_SpecialAirNRelease_IASA
/// (801176D0): once the release delay has run out, B up drops Ness's hold
/// on the flash (ftNs_SpecialN_SetNULL: the pointer and the callbacks).
fn hold_input(f: &mut Fighter, _: InputPhase<'_>) {
    let s = scratch(f);
    s.release_delay -= 1;
    if s.release_delay > 0 {
        return;
    }
    s.release_delay = 0;
    if !f.input.current.held.intersects(Buttons::B) {
        flash_gone(f);
    }
}

/// ftNs_SpecialNStart_Phys (80117750): the gravity delay counts down, then
/// ft_80084F3C.
fn start_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = scratch(f);
    if s.gravity_delay != 0 {
        s.gravity_delay -= 1;
    }
    common::ground_friction(f, p);
}

/// ftNs_SpecialAirN{Start,Release,End}_Phys (801177C8, 80117828,
/// 80117888): the gravity delay, then ftCommon_Fall at x14 to the
/// fighter's terminal velocity; ftCommon_ApplyFrictionAir with the aerial
/// friction.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = scratch(f);
    if s.gravity_delay != 0 {
        s.gravity_delay -= 1;
    } else {
        let gravity = f
            .character
            .get::<Ness>()
            .attributes
            .pk_flash
            .fall_acceleration;
        let terminal = f.attributes.air.terminal_velocity;
        common::fall_at(f, gravity, terminal);
    }
    let aerial = f.attributes.air.aerial_friction;
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, aerial);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftNs_SpecialN{Start,Release,End}_Coll (801178E8, 80117954, 801179C0):
/// ft_80082708; off the floor the aerial counterpart at the same frame
/// (ftCommon_GroundToAirStateChange with FTNESS_SPECIALN_COLL_FLAG). Both
/// the hold and the release row continue in the aerial hold.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("PK Flash collision assets");
    let state = match f.motion_state.action {
        START => air(START),
        HOLD | RELEASE => air(HOLD),
        _ => air(END),
    };
    common::ground_to_air(f, state, common::GROUND_AIR, assets)
}

/// ftNs_SpecialAirN{Start,Release,End}_Coll (80117A2C, 80117A98,
/// 80117B04): ft_80081D0C; a landing continues in the grounded
/// counterpart (ftCommon_AirToGroundStateChange), the aerial hold and
/// release rows both in the grounded hold.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("PK Flash landing assets");
    let action = f.motion_state.action;
    let state = if action == air(START) {
        START
    } else if action == air(HOLD) || action == air(RELEASE) {
        HOLD
    } else {
        END
    };
    common::air_to_ground(f, state, common::GROUND_AIR, assets)
}

/// ftNs_SpecialN_ItemPKFlushSetNULL (80116F00), from ftNs_Init_OnDamage:
/// the flash flies on without Ness (it_802AAA50), and his callbacks go.
pub fn orphan_flash(f: &mut Fighter) {
    if scratch(f).flash_out {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::NessPKFlush,
            control: ItemControl::Orphan,
        });
    }
    flash_gone(f);
}

/// ftNs_SpecialN_SetNULL (80116EBC): the flash is no longer Ness's, and
/// take_dmg_cb / death2_cb clear.
pub fn flash_gone(f: &mut Fighter) {
    scratch(f).flash_out = false;
    f.character.get_mut::<Ness>().damage_callbacks = false;
}

/// ftNs_SpecialN_CheckSpecialNHold (80116E68), which the flash reads: Ness
/// still holds it in the hold row (349 / 353).
pub fn holding(f: &Fighter) -> bool {
    let action = f.motion_state.action;
    f.character.get::<Ness>().pk_flash.flash_out && (action == HOLD || action == air(HOLD))
}
