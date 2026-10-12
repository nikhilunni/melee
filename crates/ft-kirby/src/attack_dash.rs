//! Kirby's dash attack, ftkirbyattackdash.c (800F1F68..800F21C0).
//!
//! The common AttackDash on Kirby's own rows: the burning dash runs off a
//! ledge into AttackDashAir (352) at the same frame and lands back into
//! AttackDash (351). ftCo_AttackDash.c's decideFighter enters row 351
//! through `CharacterCallbacks::DASH_ATTACK_ROW`.
use crate::common;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase, COMMON},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::CommonMotionState as S;

/// ftKb_MS_AttackDash (351) / ftKb_MS_AttackDashAir (352).
pub const GROUND: ActionId = ActionId(351);
pub const AIR: ActionId = ActionId(352);

/// 0x0C4C5082: ftCommon_GroundAirColl_MF with KeepGfx.
const TRANSITION_FLAGS: MotionEntryFlags =
    MotionEntryFlags(common::GROUND_AIR_COLL_BASE_FLAGS.0 | 1 << 1);

/// Both rows play ftCo_SM_AttackDash (52).
const ANIMATION: i32 = 52;

pub const fn rows() -> [MotionRow; 2] {
    [
        // ftKb_AttackDash_Anim/IASA (800F203C, 800F205C) are the common
        // ftCo_AttackDash_Anim/IASA; ftKb_AttackDash_Phys (800F207C) is
        // ft_80084FA8.
        MotionRow {
            action: GROUND,
            id: S::None,
            physics: callbacks::physics::jab,
            collision: ground_collision,
            ..COMMON[S::AttackDash as usize]
        },
        common::row(
            AIR,
            ANIMATION,
            air_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
    ]
}

/// A counterpart change keeps mv.co.attackdash (the dash-grab window): the
/// motion change leaves the union alone.
fn change_keeping_scratch(f: &mut Fighter, state: ActionId, assets: &FighterAssets) -> Result<()> {
    let scratch = f.core.state_data.clone();
    common::continue_as(f, state, TRANSITION_FLAGS, assets)?;
    f.core.state_data = scratch;
    Ok(())
}

/// ftKb_AttackDash_Coll (800F209C) -> ft_8008403C: off the floor,
/// fn_800F20C4: ftCommon_8007D5D4, then the aerial row at the current frame
/// and rate.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("dash attack collision assets");
    f.leave_ground();
    change_keeping_scratch(f, AIR, assets)
}

/// ftKb_AttackDashAir_Anim (800F2124): Fall at the animation's end.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_AttackDashAir_Phys (800F2164): ft_80085204 (TransN's forward delta
/// along the facing, fmuls) and ftCommon_FallBasic.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("dash attack TransN")
        .primary_history
        .offset;
    f.physics.self_velocity.x = offset.z * f.physics.facing;
    common::fall_basic(f);
    common::finish_air(f, &p);
}

/// ftKb_AttackDashAir_Coll (800F2198) -> ft_80082C74: landing, fn_800F1FDC:
/// ftCommon_8007D7FC, then the grounded row at the current frame and rate.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("dash attack landing assets");
    f.land();
    change_keeping_scratch(f, GROUND, assets)
}
