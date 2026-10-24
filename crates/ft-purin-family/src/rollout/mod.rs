//! Rollout, ftpurinspecialn.c (8013D658..801422E8) and ftkirbyspecialpurin.c
//! (80100E0C..80105978).
//!
//! B charges a roll on the spot (Start, Loop, Full; the aerial rows
//! mirror them), releasing it rolls (Release) until the charge runs down;
//! the stick against the roll turns it round (Turn); End stops it. A roll
//! that hits someone bounces off them (Hit). The motion scratch is
//! `mv.pr.specialn`; deal_dmg_cb is tracked in [`RolloutState::bounce`],
//! death2_cb and take_dmg_cb by the kind ([`RolloutKind::arm_damage_callbacks`]).
//!
//! The functions cite Jigglypuff's symbols; Kirby's are the ftKb_Pr names of
//! the same rows (ftkirbyspecialpurin.h), whose asm has the same fused
//! operations at the same places.
mod charge;
mod finish;
mod model;
mod roll;

use crate::RolloutAttributes;
use hsd_types::Vec3;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    state::{callbacks, InputPhase},
    ActionId, Fighter, MotionEntryFlags, MotionRow,
};
use melee_types::{mp::FtCollisionBox, CommonMotionState};

/// The seventeen rows, in table order (ftPr_MS_SpecialNStartR..SpecialNHit,
/// ftKb_MS_PrSpecialNStartR..PrSpecialNHit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum RolloutRow {
    StartRight,
    StartLeft,
    Loop,
    Full,
    /// SpecialN1 in Kirby's names.
    Release,
    Turn,
    EndRight,
    EndLeft,
    AirStartRight,
    AirStartLeft,
    AirLoop,
    AirFull,
    /// SpecialAirN in Kirby's names.
    AirRelease,
    /// SpecialN0 in Kirby's names.
    AirTurn,
    AirEndRight,
    AirEndLeft,
    Hit,
}

impl RolloutRow {
    const ALL: [RolloutRow; 17] = [
        Self::StartRight,
        Self::StartLeft,
        Self::Loop,
        Self::Full,
        Self::Release,
        Self::Turn,
        Self::EndRight,
        Self::EndLeft,
        Self::AirStartRight,
        Self::AirStartLeft,
        Self::AirLoop,
        Self::AirFull,
        Self::AirRelease,
        Self::AirTurn,
        Self::AirEndRight,
        Self::AirEndLeft,
        Self::Hit,
    ];
}

/// Where a kind's seventeen rows start: their actions and submotions are
/// contiguous.
#[derive(Clone, Copy, Debug)]
pub struct RolloutRows {
    pub first_action: u16,
    pub first_animation: i32,
}

impl RolloutRows {
    pub const fn action(self, row: RolloutRow) -> ActionId {
        ActionId(self.first_action + row as u16)
    }
    /// The row `action` is, if it is one of the seventeen.
    pub fn row(self, action: ActionId) -> Option<RolloutRow> {
        let index = action.0.checked_sub(self.first_action)?;
        RolloutRow::ALL.get(index as usize).copied()
    }
}

/// Where the aerial roll's landing dust (efSync 0x3FF) takes its facing
/// and angle from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandingDust {
    /// ftPr_SpecialAirNChargeRelease_Coll: `&facing_dir` and the floor's
    /// angle (atan2f of its normal).
    FacingAndFloor,
    /// ftKb_PrSpecialAirN_Coll (retail 0x801054F8..80105514): the floor's
    /// atan2f is computed and dropped; the facing pointer is
    /// `&mv.pr.specialn.x34` (the direction) and the angle pointer
    /// `&mv.pr.specialn.x14` (the roll angle).
    DirectionAndRoll,
}

/// What differs between Jigglypuff's Rollout and Kirby's copy.
pub trait RolloutKind: 'static {
    const ROWS: RolloutRows;
    const LANDING_DUST: LandingDust;
    fn attributes(f: &Fighter) -> &RolloutAttributes;
    fn state(f: &mut Fighter) -> &mut RolloutState;
    fn state_ref(f: &Fighter) -> &RolloutState;
    /// death2_cb and take_dmg_cb: ftPr_SpecialS_8013D658 (setupPurinCallbacks)
    /// or ftKb_Init_800EE74C / 800EE7B8 (ftKb_PrSetCallbacks), until the next
    /// motion change.
    fn arm_damage_callbacks(f: &mut Fighter);
}

/// ftPr_Init_803D0610 / ftKb_Init_803CB758: the roll's fixed ECB.
const ROLL_BOX: FtCollisionBox = FtCollisionBox {
    top: 8.0,
    bottom: 0.0,
    left: hsd_types::Vec2 { x: -4.0, y: 4.0 },
    right: hsd_types::Vec2 { x: 4.0, y: 4.0 },
};

/// Fighter_ChangeMotionState flag words, as both files spell them.
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

/// A kind's Rollout state beyond the motion scratch.
#[derive(Clone, Debug, Default)]
pub struct RolloutState {
    /// fp->mv.pr.specialn.
    pub scratch: Rollout,
    /// The model scale Rollout squashes: Jigglypuff's Fighter +2230
    /// (u.pr.x2230), Kirby's u.kb.x8C.
    pub scale: Vec3,
    /// deal_dmg_cb = ftPr_SpecialS_8013D764 / fn_80100E0C, until the next
    /// motion change (Fighter_ChangeMotionState, fighter.c:1376-1389).
    pub bounce: bool,
    /// Hitbox 0's contents when it was last disabled (retail keeps them).
    pub hitbox: Option<melee_coll::hitbox::HitCapsule>,
}

pub const fn rows<C: RolloutKind>() -> [MotionRow; 17] {
    use charge::*;
    use finish::*;
    use roll::*;
    use RolloutRow as R;
    [
        row::<C>(
            R::StartRight,
            start_anim::<C>,
            no_input,
            hold_physics,
            start_collision::<C>,
        ),
        row::<C>(
            R::StartLeft,
            start_anim::<C>,
            no_input,
            hold_physics,
            start_collision::<C>,
        ),
        row::<C>(
            R::Loop,
            loop_anim::<C>,
            charge_input::<C>,
            hold_physics,
            loop_collision::<C>,
        ),
        row::<C>(
            R::Full,
            full_anim::<C>,
            charge_input::<C>,
            hold_physics,
            full_collision::<C>,
        ),
        row::<C>(
            R::Release,
            release_anim::<C>,
            release_input::<C>,
            release_physics::<C>,
            release_collision::<C>,
        ),
        row::<C>(
            R::Turn,
            turn_anim::<C>,
            no_input,
            turn_physics::<C>,
            turn_collision::<C>,
        ),
        row::<C>(
            R::EndRight,
            end_anim::<C>,
            no_input,
            end_physics,
            end_collision::<C>,
        ),
        row::<C>(
            R::EndLeft,
            end_anim::<C>,
            no_input,
            end_physics,
            end_collision::<C>,
        ),
        row::<C>(
            R::AirStartRight,
            start_anim::<C>,
            no_input,
            air_fall_physics::<C>,
            air_start_collision::<C>,
        ),
        row::<C>(
            R::AirStartLeft,
            start_anim::<C>,
            no_input,
            air_fall_physics::<C>,
            air_start_collision::<C>,
        ),
        row::<C>(
            R::AirLoop,
            loop_anim::<C>,
            air_charge_input::<C>,
            air_fall_physics::<C>,
            air_loop_collision::<C>,
        ),
        row::<C>(
            R::AirFull,
            full_anim::<C>,
            air_charge_input::<C>,
            air_fall_physics::<C>,
            air_full_collision::<C>,
        ),
        row::<C>(
            R::AirRelease,
            air_release_anim::<C>,
            no_input,
            air_release_physics::<C>,
            air_release_collision::<C>,
        ),
        row::<C>(
            R::AirTurn,
            air_turn_anim::<C>,
            no_input,
            air_turn_physics::<C>,
            air_turn_collision::<C>,
        ),
        row::<C>(
            R::AirEndRight,
            air_end_anim::<C>,
            no_input,
            air_fall_physics::<C>,
            air_end_collision::<C>,
        ),
        row::<C>(
            R::AirEndLeft,
            air_end_anim::<C>,
            no_input,
            air_fall_physics::<C>,
            air_end_collision::<C>,
        ),
        row::<C>(
            R::Hit,
            hit_anim::<C>,
            no_input,
            hit_physics::<C>,
            hit_collision::<C>,
        ),
    ]
}

/// The submotions follow the actions (ftPr_SM_SpecialNStartR = 300,
/// ftKb_SM_PrSpecialNStartR = 410).
const fn row<C: RolloutKind>(
    row: RolloutRow,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: C::ROWS.action(row),
        id: CommonMotionState::None,
        animation: C::ROWS.first_animation + row as i32,
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

fn scratch<C: RolloutKind>(f: &mut Fighter) -> &mut Rollout {
    &mut C::state(f).scratch
}
fn attributes<C: RolloutKind>(f: &Fighter) -> &RolloutAttributes {
    C::attributes(f)
}
/// The row `f` is in, if one of `C`'s.
fn current<C: RolloutKind>(f: &Fighter) -> Option<RolloutRow> {
    C::ROWS.row(f.motion_state.action)
}

/// ftPr_SpecialN_Enter (8013DEA8) / ftPr_SpecialAirN_Enter (8013DF60);
/// ftKb_SpecialNPr_80101560 / 80101618.
pub fn enter<C: RolloutKind>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let direction = f.physics.facing;
    scratch::<C>(f).direction = direction;
    let row = match (air, direction == 1.0) {
        (false, true) => RolloutRow::StartRight,
        (false, false) => RolloutRow::StartLeft,
        (true, true) => RolloutRow::AirStartRight,
        (true, false) => RolloutRow::AirStartLeft,
    };
    f.change_motion_state(C::ROWS.action(row), assets)
        .expect("Rollout assets");
    f.commands.variables = [0; 4];
    f.step_animation(assets);
    setup::<C>(f);
    let a = attributes::<C>(f);
    let (gravity, ground_acceleration, air_acceleration) =
        (a.gravity, a.ground_acceleration, a.air_acceleration);
    if air {
        f.physics.animation_velocity.y = gravity;
        scratch::<C>(f).acceleration = air_acceleration;
    } else {
        f.physics.animation_velocity.y = 0.0;
        f.physics.self_velocity.y = 0.0;
        scratch::<C>(f).acceleration = ground_acceleration;
    }
}

/// ftPr_SpecialS_8013DC64 (8013DC64) / ftKb_SpecialNPr_8010131C: remember
/// the model scale, reset the scratch and install deal_dmg_cb (and x21F8).
fn setup<C: RolloutKind>(f: &mut Fighter) {
    let root = f.animation.root;
    let scale = f.skeleton.get(root).scale;
    f.physics.ground_velocity = 0.0;
    let a = attributes::<C>(f);
    let (duration, initial_charge) = (a.duration, a.initial_charge);
    let state = C::state(f);
    let direction = state.scratch.direction;
    state.scale = scale;
    state.scratch = Rollout {
        remaining: duration,
        squash_step: -1,
        // 8013DD1C fctiwz.
        charge: gekko_math::msl::fctiwz(initial_charge),
        direction,
        ..Default::default()
    };
    state.bounce = true;
    f.set_cape_turn_end(melee_ft::fighter::cape_turn::CapeTurnEnd::Character);
}

/// setupPurinCallbacks / ftKb_PrSetCallbacks: death2, take_dmg, deal_dmg
/// and x21F8 (the reversal hook, ftPr_SpecialN_8014222C / fn_80105978,
/// which only ftCo_800C37A0's cape turnaround calls).
fn install_callbacks<C: RolloutKind>(f: &mut Fighter) {
    C::arm_damage_callbacks(f);
    C::state(f).bounce = true;
    f.set_cape_turn_end(melee_ft::fighter::cape_turn::CapeTurnEnd::Character);
}

/// ftPr_SpecialN_8014222C (8014222C) / fn_80105978 (80105978), x21F8 when a
/// cape turnaround ends in a Rollout state: TopN faces the new way (pi/2 *
/// facing in double, rounded once), and every signed speed and direction of
/// the roll is negated.
pub fn cape_turn_end<C: RolloutKind>(f: &mut Fighter) {
    // 80142244..54: lfd pi/2, fmul by facing_dir, frsp.
    let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
    f.core
        .set_part_rotation(0, melee_ft::fighter::part_rotation::Axis::Y, rotation);
    f.physics.self_velocity.x = -f.physics.self_velocity.x;
    f.physics.ground_velocity = -f.physics.ground_velocity;
    f.physics.ground_acceleration = -f.physics.ground_acceleration;
    let roll = scratch::<C>(f);
    roll.turn_speed = -roll.turn_speed;
    roll.roll_angle = -roll.roll_angle;
    roll.speed = -roll.speed;
    roll.acceleration = -roll.acceleration;
    roll.pending_facing = -roll.pending_facing;
    roll.direction = -roll.direction;
    roll.previous_angle = -roll.previous_angle;
}

/// A Rollout state change with a retail flag word, installing every
/// callback. Without SkipHit the change disables the hitboxes, whose
/// capsule slot 0 remembers its contents (see [`model::remember_hitbox`]).
fn change<C: RolloutKind>(
    f: &mut Fighter,
    row: RolloutRow,
    flags: MotionEntryFlags,
    start: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    if !flags.contains(MotionEntryFlags::SKIP_HIT) {
        model::remember_hitbox::<C>(f);
    }
    f.change_motion_state_with_flags(C::ROWS.action(row), assets, flags, start, rate)?;
    install_callbacks::<C>(f);
    Ok(())
}

/// ftPr_SpecialS_8013DA24 (8013DA24) / ftKb_SpecialNPr_801010D4: stop
/// rolling on the ground or in the air, damping the speed.
fn end<C: RolloutKind>(
    f: &mut Fighter,
    air: bool,
    flags: MotionEntryFlags,
    start: f32,
    assets: &FighterAssets,
) -> Result<()> {
    let direction = scratch::<C>(f).direction;
    f.physics.facing = direction;
    model::clear_hitboxes::<C>(f);
    let a = attributes::<C>(f);
    let (horizontal, vertical) = (
        a.rebound_horizontal_multiplier,
        a.rebound_vertical_multiplier,
    );
    let right = f.physics.facing == 1.0;
    if air {
        let row = if right {
            RolloutRow::AirEndRight
        } else {
            RolloutRow::AirEndLeft
        };
        change::<C>(f, row, flags, start, 1.0, assets)?;
        // 8013DAFC / 8013DB0C: separate fmuls.
        f.physics.self_velocity.x *= horizontal;
        f.physics.self_velocity.y *= vertical;
        f.physics.ground_acceleration = 0.0;
        f.physics.ground_velocity = 0.0;
    } else {
        let row = if right {
            RolloutRow::EndRight
        } else {
            RolloutRow::EndLeft
        };
        change::<C>(f, row, flags, start, 1.0, assets)?;
        // 8013DAA4 fmuls.
        f.physics.ground_velocity *= horizontal;
        f.physics.animation_velocity.y = 0.0;
        f.physics.self_velocity.y = 0.0;
    }
    model::restore::<C>(f);
    scratch::<C>(f).pending_facing = 0.0;
    model::face(f);
    Ok(())
}

/// ftPr_SpecialS_8013D764 (8013D764) / fn_80100E0C, deal_dmg_cb: a rolling
/// hit costs roll time and bounces off into SpecialNHit.
pub fn bounce<C: RolloutKind>(f: &mut Fighter, assets: &FighterAssets) {
    if !C::state_ref(f).bounce {
        return;
    }
    scratch::<C>(f).group_timer = 0;
    if !matches!(
        current::<C>(f),
        Some(RolloutRow::Release | RolloutRow::Turn | RolloutRow::AirRelease | RolloutRow::AirTurn)
    ) {
        return;
    }
    let a = attributes::<C>(f);
    let (cost, horizontal, vertical) = (
        a.hit_frame_cost,
        a.hit_horizontal_multiplier,
        a.hit_vertical_speed,
    );
    let rollout = scratch::<C>(f);
    rollout.remaining -= cost;
    let direction = rollout.direction;
    f.physics.facing = direction;
    model::clear_hitboxes::<C>(f);
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(
        C::ROWS.action(RolloutRow::Hit),
        assets,
        flags::HELD,
        frame,
        0.0,
    )
    .expect("Rollout hit assets");
    // death2/take_dmg return; deal_dmg_cb = NULL; x21F8 is not reinstalled.
    C::arm_damage_callbacks(f);
    C::state(f).bounce = false;
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
    scratch::<C>(f).pending_facing = 0.0;
    model::face_forward(f);
    model::sound(f, model::BOUNCE_SOUND);
}

/// ftPr_SpecialS_8013D658 (8013D658): the model's scale and facing return
/// (Jigglypuff's death2_cb and take_dmg_cb).
pub fn restore<C: RolloutKind>(f: &mut Fighter) {
    model::restore::<C>(f);
}
