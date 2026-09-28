//! Rollout, ftpurinspecialn.c (8013D658..801422E8).
//!
//! B charges a roll on the spot (Start, Loop, Full; the aerial rows
//! mirror them), releasing it rolls (Release) until the charge runs down;
//! the stick against the roll turns it round (Turn); End stops it. A roll
//! that hits someone bounces off them (Hit). The motion scratch is
//! `mv.pr.specialn`; the per-motion callbacks it installs are tracked in
//! [`Callbacks`].
mod charge;
mod finish;
mod model;
mod roll;

use crate::init::Jigglypuff;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    state::{callbacks, InputPhase},
    ActionId, Fighter, MotionEntryFlags, MotionRow,
};
use melee_types::{mp::FtCollisionBox, CommonMotionState};

/// ftPr_MS_SpecialNStartR..SpecialNHit (346..362).
pub const START_RIGHT: ActionId = ActionId(346);
pub const START_LEFT: ActionId = ActionId(347);
pub const LOOP: ActionId = ActionId(348);
pub const FULL: ActionId = ActionId(349);
pub const RELEASE: ActionId = ActionId(350);
pub const TURN: ActionId = ActionId(351);
pub const END_RIGHT: ActionId = ActionId(352);
pub const END_LEFT: ActionId = ActionId(353);
pub const AIR_START_RIGHT: ActionId = ActionId(354);
pub const AIR_START_LEFT: ActionId = ActionId(355);
pub const AIR_LOOP: ActionId = ActionId(356);
pub const AIR_FULL: ActionId = ActionId(357);
pub const AIR_RELEASE: ActionId = ActionId(358);
pub const AIR_TURN: ActionId = ActionId(359);
pub const AIR_END_RIGHT: ActionId = ActionId(360);
pub const AIR_END_LEFT: ActionId = ActionId(361);
pub const HIT: ActionId = ActionId(362);

/// Fighter_ChangeMotionState flag words, as ftpurinspecialn.c spells them.
mod flags {
    use melee_ft::fighter::MotionEntryFlags as F;
    /// KeepGfx | SkipModel | SkipItemVis: Start's hand-off to the charge,
    /// and the end states (8013DA24's usual caller flags).
    pub const START: F = F(0x0004_0012);
    /// KeepGfx | SkipModel | SkipMatAnim | KeepSfx | SkipItemVis | Unk19 |
    /// SkipModelPartVis | SkipModelFlags | Unk27: the charge reaching full,
    /// the turn, the hit.
    pub const HELD: F = F(0x0C4C_0292);
    /// KeepGfx | SkipModel | SkipMatAnim | SkipColAnim | SkipItemVis: the
    /// release and the charge's ground/air changes.
    pub const CHARGE: F = F(0x0004_1092);
    /// ftCommon_GroundAirColl_MF | KeepGfx | SkipModel.
    pub const GROUND_AIR: F = F(0x0C4C_5092);
    /// GROUND_AIR | SkipHit: the rolling ground/air changes keep the hitbox.
    pub const ROLLING_GROUND_AIR: F = F(0x0C4C_509A);
    /// SkipModel | SkipItemVis | SkipAttackCount: a turn rolling on the
    /// right way again (ftPr_SpecialNTurn_Phys, from rightward).
    pub const UNTURN_FROM_RIGHT: F = F(0x0204_0010);
    /// KeepGfx | UNTURN_FROM_RIGHT (from leftward).
    pub const UNTURN_FROM_LEFT: F = F(0x0204_0012);
}

/// ftPr_Init_803D0610: the roll's fixed ECB.
const ROLL_BOX: FtCollisionBox = FtCollisionBox {
    top: 8.0,
    bottom: 0.0,
    left: hsd_types::Vec2 { x: -4.0, y: 4.0 },
    right: hsd_types::Vec2 { x: 4.0, y: 4.0 },
};

/// fp->mv.pr.specialn (Fighter +2340..+237C).
#[derive(Clone, Debug, Default)]
pub struct Rollout {
    /// x0: frames of rolling left.
    pub remaining: i32,
    /// x8: step through the landing squash, -1 when idle.
    pub squash_step: i32,
    /// xC: frames since the hitbox group last toggled.
    pub group_timer: i32,
    /// x10: ground speed when the turn began.
    pub turn_speed: f32,
    /// x14: the roll angle about part 3, kept in [0, 2pi].
    pub roll_angle: f32,
    /// x18: rolling speed.
    pub speed: f32,
    /// x1C: acceleration (the turn's deceleration while turning).
    pub acceleration: f32,
    /// facing_dir (x20): the facing a turn will leave, 0 when none.
    pub pending_facing: f32,
    /// x24: frames of turning, for the dust interval.
    pub dust_timer: i32,
    /// x2C: the charge, an integer.
    pub charge: i32,
    /// x30: whether the charge flash already played.
    pub charged: bool,
    /// x34.x: the rolling direction.
    pub direction: f32,
    /// x34.y: the roll angle the previous frame, for the rolling sound.
    pub previous_angle: f32,
}

/// The callbacks Rollout installs, until the next motion change clears
/// them (Fighter_ChangeMotionState, fighter.c:1376-1389).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Callbacks {
    /// death2_cb and take_dmg_cb = ftPr_SpecialS_8013D658.
    pub restore: bool,
    /// deal_dmg_cb = ftPr_SpecialS_8013D764.
    pub bounce: bool,
}

pub const fn rows() -> [MotionRow; 17] {
    use charge::*;
    use finish::*;
    use roll::*;
    [
        row(
            START_RIGHT,
            start_anim,
            no_input,
            hold_physics,
            start_collision,
        ),
        row(
            START_LEFT,
            start_anim,
            no_input,
            hold_physics,
            start_collision,
        ),
        row(LOOP, loop_anim, charge_input, hold_physics, loop_collision),
        row(FULL, full_anim, charge_input, hold_physics, full_collision),
        row(
            RELEASE,
            release_anim,
            release_input,
            release_physics,
            release_collision,
        ),
        row(TURN, turn_anim, no_input, turn_physics, turn_collision),
        row(END_RIGHT, end_anim, no_input, end_physics, end_collision),
        row(END_LEFT, end_anim, no_input, end_physics, end_collision),
        row(
            AIR_START_RIGHT,
            start_anim,
            no_input,
            air_fall_physics,
            air_start_collision,
        ),
        row(
            AIR_START_LEFT,
            start_anim,
            no_input,
            air_fall_physics,
            air_start_collision,
        ),
        row(
            AIR_LOOP,
            loop_anim,
            air_charge_input,
            air_fall_physics,
            air_loop_collision,
        ),
        row(
            AIR_FULL,
            full_anim,
            air_charge_input,
            air_fall_physics,
            air_full_collision,
        ),
        row(
            AIR_RELEASE,
            air_release_anim,
            no_input,
            air_release_physics,
            air_release_collision,
        ),
        row(
            AIR_TURN,
            air_turn_anim,
            no_input,
            air_turn_physics,
            air_turn_collision,
        ),
        row(
            AIR_END_RIGHT,
            air_end_anim,
            no_input,
            air_fall_physics,
            air_end_collision,
        ),
        row(
            AIR_END_LEFT,
            air_end_anim,
            no_input,
            air_fall_physics,
            air_end_collision,
        ),
        row(HIT, hit_anim, no_input, hit_physics, hit_collision),
    ]
}

/// ftPr_SM_SpecialNStartR = 300: submotions follow the action numbers.
const fn row(
    action: ActionId,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation: action.0 as i32 - 46,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The IASA callbacks the table leaves empty.
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

fn puff(f: &Fighter) -> &Jigglypuff {
    f.character.get::<Jigglypuff>()
}
fn scratch(f: &mut Fighter) -> &mut Rollout {
    &mut f.character.get_mut::<Jigglypuff>().rollout
}
fn attributes(f: &Fighter) -> &crate::attributes::RolloutAttributes {
    &puff(f).attributes.rollout
}

/// ftPr_SpecialN_Enter (8013DEA8) / ftPr_SpecialAirN_Enter (8013DF60).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let direction = f.physics.facing;
    scratch(f).direction = direction;
    let state = match (air, direction == 1.0) {
        (false, true) => START_RIGHT,
        (false, false) => START_LEFT,
        (true, true) => AIR_START_RIGHT,
        (true, false) => AIR_START_LEFT,
    };
    f.change_motion_state(state, assets)
        .expect("Rollout assets");
    f.commands.variables = [0; 4];
    f.step_animation(assets);
    setup(f);
    let a = attributes(f);
    let (gravity, ground_acceleration, air_acceleration) =
        (a.gravity, a.ground_acceleration, a.air_acceleration);
    if air {
        f.physics.animation_velocity.y = gravity;
        scratch(f).acceleration = air_acceleration;
    } else {
        f.physics.animation_velocity.y = 0.0;
        f.physics.self_velocity.y = 0.0;
        scratch(f).acceleration = ground_acceleration;
    }
}

/// ftPr_SpecialS_8013DC64 (8013DC64): remember the model scale, reset the
/// scratch and install deal_dmg_cb (and x21F8).
fn setup(f: &mut Fighter) {
    let root = f.animation.root;
    let scale = f.skeleton.get(root).scale;
    f.physics.ground_velocity = 0.0;
    let a = attributes(f);
    let (duration, initial_charge) = (a.duration, a.initial_charge);
    let direction = scratch(f).direction;
    let puff = f.character.get_mut::<Jigglypuff>();
    puff.rollout_scale = scale;
    puff.rollout = Rollout {
        remaining: duration,
        squash_step: -1,
        // 8013DD1C fctiwz.
        charge: gekko_math::msl::fctiwz(initial_charge),
        direction,
        ..Default::default()
    };
    puff.rollout_callbacks.bounce = true;
}

/// setupPurinCallbacks: death2, take_dmg, deal_dmg and x21F8 (the
/// reversal hook, ftPr_SpecialN_8014222C, which only ftCo_800C37A0's
/// cape turnaround calls; the port does not model that turnaround).
fn install_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Jigglypuff>().rollout_callbacks = Callbacks {
        restore: true,
        bounce: true,
    };
}

/// A Rollout state change with a retail flag word, installing every
/// callback. Without SkipHit the change disables the hitboxes, whose
/// capsule slot 0 remembers its contents (see [`model::remember_hitbox`]).
fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    start: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    if !flags.contains(MotionEntryFlags::SKIP_HIT) {
        model::remember_hitbox(f);
    }
    f.change_motion_state_with_flags(state, assets, flags, start, rate)?;
    install_callbacks(f);
    Ok(())
}

/// ftPr_SpecialS_8013DA24 (8013DA24): stop rolling on the ground or in the
/// air, damping the speed.
fn end(
    f: &mut Fighter,
    air: bool,
    flags: MotionEntryFlags,
    start: f32,
    assets: &FighterAssets,
) -> Result<()> {
    let direction = scratch(f).direction;
    f.physics.facing = direction;
    model::clear_hitboxes(f);
    let a = attributes(f);
    let (horizontal, vertical) = (
        a.rebound_horizontal_multiplier,
        a.rebound_vertical_multiplier,
    );
    let right = f.physics.facing == 1.0;
    if air {
        let state = if right { AIR_END_RIGHT } else { AIR_END_LEFT };
        change(f, state, flags, start, 1.0, assets)?;
        // 8013DAFC / 8013DB0C: separate fmuls.
        f.physics.self_velocity.x *= horizontal;
        f.physics.self_velocity.y *= vertical;
        f.physics.ground_acceleration = 0.0;
        f.physics.ground_velocity = 0.0;
    } else {
        let state = if right { END_RIGHT } else { END_LEFT };
        change(f, state, flags, start, 1.0, assets)?;
        // 8013DAA4 fmuls.
        f.physics.ground_velocity *= horizontal;
        f.physics.animation_velocity.y = 0.0;
        f.physics.self_velocity.y = 0.0;
    }
    model::restore(f);
    scratch(f).pending_facing = 0.0;
    model::face(f);
    Ok(())
}

/// ftPr_SpecialS_8013D764 (8013D764), deal_dmg_cb: a rolling hit costs
/// roll time and bounces Puff off into SpecialNHit.
pub fn bounce(f: &mut Fighter, assets: &FighterAssets) {
    if !puff(f).rollout_callbacks.bounce {
        return;
    }
    scratch(f).group_timer = 0;
    let action = f.motion_state.action;
    if ![RELEASE, TURN, AIR_RELEASE, AIR_TURN].contains(&action) {
        return;
    }
    let a = attributes(f);
    let (cost, horizontal, vertical) = (
        a.hit_frame_cost,
        a.hit_horizontal_multiplier,
        a.hit_vertical_speed,
    );
    let rollout = scratch(f);
    rollout.remaining -= cost;
    let direction = rollout.direction;
    f.physics.facing = direction;
    model::clear_hitboxes(f);
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(HIT, assets, flags::HELD, frame, 0.0)
        .expect("Rollout hit assets");
    // death2/take_dmg return; deal_dmg_cb = NULL; x21F8 is not reinstalled.
    f.character.get_mut::<Jigglypuff>().rollout_callbacks = Callbacks {
        restore: true,
        bounce: false,
    };
    if f.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        // 8013D82C fmuls.
        f.physics.self_velocity.x = f.physics.ground_velocity * horizontal;
        f.leave_ground();
    } else {
        // 8013D844 fmuls.
        f.physics.self_velocity.x *= horizontal;
    }
    f.physics.self_velocity.y = vertical;
    f.physics.animation_velocity.y = 0.0;
    f.physics.animation_velocity.x = 0.0;
    f.physics.ground_acceleration = 0.0;
    f.physics.ground_velocity = 0.0;
    scratch(f).pending_facing = 0.0;
    model::face_forward(f);
    model::sound(f, model::BOUNCE_SOUND);
}

/// ftPr_SpecialS_8013D658 (8013D658), death2_cb and take_dmg_cb: the
/// model's scale and facing return.
pub fn restore_on_callback(f: &mut Fighter) {
    if puff(f).rollout_callbacks.restore {
        model::restore(f);
    }
}
